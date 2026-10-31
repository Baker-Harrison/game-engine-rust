use crate::camera::Camera;
use crate::math::Vec3;
use crate::model::bake::BakeSettings;
use crate::model::mesh::Mesh;
use crate::model::part::{Asset, PartKind, View};
use crate::model::validate::{summarize, validate};
use crate::overlay::{blit, draw_line, draw_rect, draw_text};
use crate::png;
use crate::render::{render, Framebuffer};
use crate::scene::Scene;
use std::fmt::Write;

const VIEW_WIDTH: usize = 480;
const VIEW_HEIGHT: usize = 360;
const SHEET_COLUMNS: usize = 3;
const STUDIO_SKY: Vec3 = Vec3::new(0.80, 0.83, 0.87);
const OUTDOOR_SKY: Vec3 = Vec3::new(0.62, 0.78, 0.95);
const LONG_LENS_DISTANCE: f32 = 400.0;
const TEXT_COLOR: u32 = 0xF0F0F0;
const GUIDE_COLOR: u32 = 0xFF5A36;

pub struct InspectionResult {
    pub report: String,
    pub files: Vec<String>,
}

pub fn inspect(asset: &Asset, output_directory: &str) -> Result<InspectionResult, String> {
    std::fs::create_dir_all(output_directory).map_err(|e| format!("{}: {}", output_directory, e))?;
    let settings = BakeSettings::default();
    let full_mesh = asset.baked_mesh(&[], &settings);
    let has_roof = !asset.roof_part_names.is_empty();
    let open_mesh = if has_roof { asset.baked_mesh(&asset.roof_part_names, &settings) } else { full_mesh.clone() };
    let sky = if has_roof { OUTDOOR_SKY } else { STUDIO_SKY };
    let full_scene = Scene::from_mesh(&full_mesh, sky);
    let open_scene = Scene::from_mesh(&open_mesh, sky);

    let views = if asset.views.is_empty() { automatic_views(asset) } else { asset.views.clone() };
    let mut files = Vec::new();

    let rows = views.len().div_ceil(SHEET_COLUMNS);
    let mut sheet = Framebuffer::new(VIEW_WIDTH * SHEET_COLUMNS, VIEW_HEIGHT * rows);
    for (index, view) in views.iter().enumerate() {
        let scene = if view.name == "dollhouse" || view.name == "plan" { &open_scene } else { &full_scene };
        let mut frame = Framebuffer::new(VIEW_WIDTH, VIEW_HEIGHT);
        render(scene, &view.camera, &mut frame);
        draw_text(&mut frame, 8, 8, &view.name, TEXT_COLOR, 2);
        blit(&mut sheet, &frame, (index % SHEET_COLUMNS) * VIEW_WIDTH, (index / SHEET_COLUMNS) * VIEW_HEIGHT);
    }
    let sheet_path = format!("{}/contact_sheet.png", output_directory);
    png::write(&sheet, &sheet_path)?;
    files.push(sheet_path);

    let hero = &views[0];
    let hero_scene = if hero.name == "dollhouse" { &open_scene } else { &full_scene };
    let mut hero_frame = Framebuffer::new(VIEW_WIDTH * 2, VIEW_HEIGHT * 2);
    render(hero_scene, &hero.camera, &mut hero_frame);
    let hero_path = format!("{}/hero.png", output_directory);
    png::write(&hero_frame, &hero_path)?;
    files.push(hero_path);

    let (low, high) = asset.object_bounds();
    let dimension_sheet = dimension_views(&open_scene, low, high);
    let dimension_path = format!("{}/dimensions.png", output_directory);
    png::write(&dimension_sheet, &dimension_path)?;
    files.push(dimension_path);

    let report = build_report(asset, &full_mesh);
    let report_path = format!("{}/report.txt", output_directory);
    std::fs::write(&report_path, &report).map_err(|e| format!("{}: {}", report_path, e))?;
    files.push(report_path);
    Ok(InspectionResult { report, files })
}

fn automatic_views(asset: &Asset) -> Vec<View> {
    let (low, high) = asset.object_bounds();
    let center = (low + high) * 0.5;
    let size = high - low;
    let aspect = VIEW_WIDTH as f32 / VIEW_HEIGHT as f32;
    let wide = size.x.max(size.z) > size.y * 1.5;
    let radius = (size.length() * 0.5) / if wide { aspect } else { 1.0 };
    let distance = radius / (17.5f32.to_radians()).tan() * 1.15;
    let around = |direction: Vec3, scale: f32, target: Vec3, fov: f32| {
        Camera::look_at(target + direction.normalized() * (distance * scale), target, fov)
    };
    let upper_target = center + Vec3::new(0.0, (high.y - low.y) * 0.15, 0.0);
    vec![
        View { name: "three-quarter".into(), camera: around(Vec3::new(1.0, 0.55, 1.0), 1.0, center, 35.0) },
        View { name: "front".into(), camera: around(Vec3::new(0.0, 0.15, 1.0), 1.0, center, 35.0) },
        View { name: "side".into(), camera: around(Vec3::new(1.0, 0.15, 0.0), 1.0, center, 35.0) },
        View { name: "back".into(), camera: around(Vec3::new(-0.6, 0.35, -1.0), 1.0, center, 35.0) },
        View { name: "top".into(), camera: around(Vec3::new(0.0, 1.0, 0.12), 1.0, center, 35.0) },
        View { name: "close-up".into(), camera: around(Vec3::new(1.0, 0.45, 1.0), 0.5, upper_target, 35.0) },
    ]
}

fn dimension_views(scene: &Scene, low: Vec3, high: Vec3) -> Framebuffer {
    let size = high - low;
    let center = (low + high) * 0.5;
    let specs = [
        ("front", Vec3::new(0.0, 0.0, 1.0), size.x, size.y, "w", "h", size.x, size.y),
        ("side", Vec3::new(1.0, 0.0, 0.0), size.z, size.y, "d", "h", size.z, size.y),
        ("top", Vec3::new(0.0, 1.0, 0.0), size.x, size.z, "w", "d", size.x, size.z),
    ];
    let mut sheet = Framebuffer::new(VIEW_WIDTH * specs.len(), VIEW_HEIGHT);
    for (index, (name, direction, horizontal_extent, vertical_extent, horizontal_label, vertical_label, horizontal_value, vertical_value)) in
        specs.iter().enumerate()
    {
        let aspect = VIEW_WIDTH as f32 / VIEW_HEIGHT as f32;
        let half = (vertical_extent * 0.5).max(horizontal_extent * 0.5 / aspect) * 1.35;
        let fov = 2.0 * (half / LONG_LENS_DISTANCE).atan().to_degrees();
        let mut camera = Camera::look_at(center + *direction * LONG_LENS_DISTANCE, center, fov);
        if *name == "top" {
            camera.pitch = -std::f32::consts::FRAC_PI_2;
            camera.yaw = 0.0;
        }
        camera.near = 1.0;
        let mut frame = Framebuffer::new(VIEW_WIDTH, VIEW_HEIGHT);
        render(scene, &camera, &mut frame);
        annotate_extents(&mut frame, &camera, center, *name, size);
        let width_label = format!("{} {:.3} m", horizontal_label, horizontal_value);
        let height_label = format!("{} {:.3} m", vertical_label, vertical_value);
        draw_text(&mut frame, 8, 8, name, TEXT_COLOR, 2);
        draw_text(&mut frame, 8, 30, &width_label, TEXT_COLOR, 2);
        draw_text(&mut frame, 8, 52, &height_label, TEXT_COLOR, 2);
        blit(&mut sheet, &frame, index * VIEW_WIDTH, 0);
    }
    sheet
}

fn annotate_extents(frame: &mut Framebuffer, camera: &Camera, center: Vec3, view: &str, size: Vec3) {
    let half = size * 0.5;
    let corner = |sx: f32, sy: f32, sz: f32| center + Vec3::new(sx * half.x, sy * half.y, sz * half.z);
    let (a, b) = match view {
        "front" => (corner(-1.0, -1.0, 1.0), corner(1.0, 1.0, 1.0)),
        "side" => (corner(1.0, -1.0, -1.0), corner(1.0, 1.0, 1.0)),
        _ => (corner(-1.0, 1.0, -1.0), corner(1.0, 1.0, 1.0)),
    };
    let project = |p: Vec3| camera.project_to_screen(p, frame.width, frame.height);
    if let (Some(first), Some(second)) = (project(a), project(b)) {
        let (min_x, max_x) = (first.0.min(second.0), first.0.max(second.0));
        let (min_y, max_y) = (first.1.min(second.1), first.1.max(second.1));
        draw_rect(frame, (min_x, min_y), (max_x, max_y), GUIDE_COLOR);
        let tick = 6.0;
        draw_line(frame, (min_x, max_y + 14.0), (max_x, max_y + 14.0), GUIDE_COLOR);
        draw_line(frame, (min_x, max_y + 14.0 - tick), (min_x, max_y + 14.0 + tick), GUIDE_COLOR);
        draw_line(frame, (max_x, max_y + 14.0 - tick), (max_x, max_y + 14.0 + tick), GUIDE_COLOR);
        draw_line(frame, (max_x + 14.0, min_y), (max_x + 14.0, max_y), GUIDE_COLOR);
        draw_line(frame, (max_x + 14.0 - tick, min_y), (max_x + 14.0 + tick, min_y), GUIDE_COLOR);
        draw_line(frame, (max_x + 14.0 - tick, max_y), (max_x + 14.0 + tick, max_y), GUIDE_COLOR);
    }
}

fn build_report(asset: &Asset, mesh: &Mesh) -> String {
    let mut text = String::new();
    let (low, high) = asset.object_bounds();
    let size = high - low;
    writeln!(text, "asset {}", asset.name).unwrap();
    writeln!(text, "triangles {}  vertices {}", mesh.triangle_count(), mesh.vertex_count()).unwrap();
    writeln!(text, "bounds  x {:.3} to {:.3} ({:.3} m)", low.x, high.x, size.x).unwrap();
    writeln!(text, "        y {:.3} to {:.3} ({:.3} m)", low.y, high.y, size.y).unwrap();
    writeln!(text, "        z {:.3} to {:.3} ({:.3} m)", low.z, high.z, size.z).unwrap();
    writeln!(text, "\nparts").unwrap();
    for part in asset.parts.iter().filter(|part| part.kind != PartKind::Stage) {
        let topology = part.mesh.topology_report();
        let (part_low, part_high) = part.mesh.bounds();
        let extent = part_high - part_low;
        writeln!(
            text,
            "  {:<32} tris {:>7}  size {:.3} x {:.3} x {:.3}  watertight {}  sharpest edge {:.0} deg",
            part.name,
            part.mesh.triangle_count(),
            extent.x,
            extent.y,
            extent.z,
            if topology.watertight { "yes" } else { "no" },
            topology.max_edge_angle_degrees
        )
        .unwrap();
    }
    let findings = validate(asset);
    writeln!(text, "\nvalidation").unwrap();
    for finding in &findings {
        writeln!(text, "  {}", finding.format()).unwrap();
    }
    let (ok, warnings, errors) = summarize(&findings);
    writeln!(text, "\nsummary: {} ok, {} warnings, {} errors", ok, warnings, errors).unwrap();
    text
}
