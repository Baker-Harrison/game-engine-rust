use engine::camera::Camera;
use engine::math::Vec3;
use engine::model::material::{Material, Plaster, Surface, SurfaceSample, WoodGrain};
use engine::model::mesher::mesh_sdf_surface;
use engine::model::sdf::Sdf;
use engine::scene::Scene;
use engine::trace::{render, TraceSettings};

fn sample_at(x: f32, y: f32, z: f32) -> SurfaceSample {
    SurfaceSample { position: Vec3::new(x, y, z), normal: Vec3::new(0.0, 1.0, 0.0), curvature: 0.0 }
}

fn sphere_scene(material: Material) -> Scene {
    let surface = move |_: &SurfaceSample| material;
    let mesh = mesh_sdf_surface(&Sdf::sphere(0.5), 0.02, &surface);
    Scene::from_mesh(&mesh, Vec3::new(0.6, 0.75, 0.95))
}

fn center_luminance(scene: &Scene) -> f32 {
    let camera = Camera::look_at(Vec3::new(0.0, 0.0, 2.5), Vec3::ZERO, 30.0);
    let settings = TraceSettings { samples_per_pixel: 2, ..TraceSettings::default() };
    let frame = render(scene, &camera, 48, 48, settings);
    let pixel = frame.pixels[24 * 48 + 24];
    let (r, g, b) = ((pixel >> 16) & 255, (pixel >> 8) & 255, pixel & 255);
    (r + g + b) as f32 / 3.0
}

#[test]
fn surfaces_are_deterministic() {
    let wood = WoodGrain { light: Vec3::new(0.6, 0.4, 0.2), dark: Vec3::new(0.3, 0.2, 0.1), grain_axis: Vec3::new(0.0, 1.0, 0.0), scale: 20.0, roughness: 0.5, seed: 1 };
    let a = wood.evaluate(&sample_at(0.3, 0.2, 0.1));
    let b = wood.evaluate(&sample_at(0.3, 0.2, 0.1));
    assert_eq!(a.albedo, b.albedo);
    assert_eq!(a.roughness, b.roughness);
}

#[test]
fn plaster_varies_across_space_but_stays_near_base() {
    let plaster = Plaster { base: Vec3::new(0.8, 0.8, 0.8), variation: 0.1, seed: 2 };
    let a = plaster.evaluate(&sample_at(0.0, 0.0, 0.0)).albedo;
    let b = plaster.evaluate(&sample_at(1.3, 0.7, 2.1)).albedo;
    assert_ne!(a, b);
    assert!((a.x - 0.8).abs() < 0.2 && (b.x - 0.8).abs() < 0.2);
}

#[test]
fn tracer_distinguishes_metal_from_diffuse() {
    let albedo = Vec3::new(0.7, 0.7, 0.7);
    let diffuse = center_luminance(&sphere_scene(Material::new(albedo, 0.9, 0.0)));
    let metal = center_luminance(&sphere_scene(Material::new(albedo, 0.15, 1.0)));
    assert!((diffuse - metal).abs() > 4.0, "diffuse {} metal {}", diffuse, metal);
}

#[test]
fn tracer_shadows_darken_occluded_surface() {
    let mut lit = sphere_scene(Material::new(Vec3::new(0.7, 0.7, 0.7), 0.9, 0.0));
    let open = center_luminance(&lit);
    let cover = mesh_sdf_surface(
        &Sdf::round_box(Vec3::new(6.0, 0.1, 6.0), 0.01).translate(Vec3::new(0.0, 1.2, 0.0)),
        0.1,
        &|_: &SurfaceSample| Material::new(Vec3::new(0.5, 0.5, 0.5), 0.9, 0.0),
    );
    lit.add_mesh(&cover);
    let shaded = center_luminance(&lit);
    assert!(shaded < open, "open {} shaded {}", open, shaded);
}
