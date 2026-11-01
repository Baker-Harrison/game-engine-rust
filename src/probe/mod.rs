pub mod aov;
pub mod geometry;
pub mod lighting;
pub mod placement;
pub mod targets;

use crate::camera::Camera;
use crate::model::mesh::Mesh;
use crate::model::part::{Asset, PartKind, View};
use std::collections::BTreeMap;
use std::fmt::Write;
use targets::Targets;

const PROBE_WIDTH: usize = 640;
const PROBE_HEIGHT: usize = 480;
const SKIPPED_VIEWS: [&str; 2] = ["dollhouse", "plan"];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warn,
    Error,
}

pub struct Finding {
    pub severity: Severity,
    pub subject: String,
    pub metric: String,
    pub value: f32,
    pub target: String,
    pub hint: String,
}

impl Finding {
    pub fn new(severity: Severity, subject: &str, metric: &str, value: f32, target: String, hint: &str) -> Finding {
        Finding { severity, subject: subject.to_string(), metric: metric.to_string(), value, target, hint: hint.to_string() }
    }

    fn label(&self) -> &'static str {
        match self.severity {
            Severity::Info => "INFO ",
            Severity::Warn => "WARN ",
            Severity::Error => "ERROR",
        }
    }

    fn format(&self) -> String {
        format!("{} {}: {} = {:.4} (target {}) -> {}", self.label(), self.subject, self.metric, self.value, self.target, self.hint)
    }
}

pub struct ProbeObject {
    pub name: String,
    pub first_triangle: usize,
    pub triangle_count: usize,
}

pub struct ProbeScene {
    pub mesh: Mesh,
    pub objects: Vec<ProbeObject>,
    pub object_of_triangle: Vec<u32>,
    pub views: Vec<View>,
}

impl ProbeScene {
    pub fn from_asset(asset: &Asset, views: Vec<View>) -> ProbeScene {
        let mut groups: BTreeMap<String, Vec<&Mesh>> = BTreeMap::new();
        for part in asset.parts.iter().filter(|part| part.kind != PartKind::Stage) {
            let name = part.name.split_once('.').map(|(prefix, _)| prefix.to_string()).unwrap_or_else(|| part.name.clone());
            groups.entry(name).or_default().push(&part.mesh);
        }
        let mut mesh = Mesh::new();
        let mut objects = Vec::new();
        let mut object_of_triangle = Vec::new();
        for (name, meshes) in groups {
            let first_triangle = mesh.triangle_count();
            for part_mesh in meshes {
                mesh.append(part_mesh);
            }
            let triangle_count = mesh.triangle_count() - first_triangle;
            object_of_triangle.extend(std::iter::repeat(objects.len() as u32).take(triangle_count));
            objects.push(ProbeObject { name, first_triangle, triangle_count });
        }
        let settings = asset.default_bake_settings();
        let occluders = crate::model::bvh::Bvh::from_meshes(&[&asset.merged_mesh(&[])]);
        crate::model::bake::bake(&mut mesh, &occluders, &settings);
        ProbeScene { mesh, objects, object_of_triangle, views }
    }
}

pub struct ProbeReport {
    pub findings: Vec<Finding>,
    pub summary: String,
}

impl ProbeReport {
    pub fn error_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Error).count()
    }

    pub fn text(&self) -> String {
        let mut out = self.summary.clone();
        writeln!(out, "\nfindings ({} errors, {} warnings)", self.error_count(), self.findings.iter().filter(|f| f.severity == Severity::Warn).count()).unwrap();
        for finding in &self.findings {
            writeln!(out, "  {}", finding.format()).unwrap();
        }
        out
    }

    pub fn json(&self) -> String {
        let items: Vec<String> = self
            .findings
            .iter()
            .map(|f| {
                format!(
                    "{{\"severity\":\"{}\",\"subject\":{},\"metric\":{},\"value\":{:.5},\"target\":{},\"hint\":{}}}",
                    f.label().trim().to_lowercase(),
                    quote(&f.subject),
                    quote(&f.metric),
                    f.value,
                    quote(&f.target),
                    quote(&f.hint)
                )
            })
            .collect();
        format!("[{}]", items.join(",\n "))
    }
}

fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn run(scene: &ProbeScene, targets: &Targets) -> ProbeReport {
    let mut findings = Vec::new();
    let mut summary = String::new();
    writeln!(summary, "probe: {} triangles, {} objects, {} views", scene.mesh.triangle_count(), scene.objects.len(), scene.views.len()).unwrap();

    let mut best: Vec<geometry::ObjectView> = vec![geometry::ObjectView::default(); scene.objects.len()];
    for view in scene.views.iter().filter(|v| !SKIPPED_VIEWS.contains(&v.name.as_str())) {
        let aov = aov::rasterize(&scene.mesh, &camera_of(view), PROBE_WIDTH, PROBE_HEIGHT);
        let stats = lighting::analyze(&aov);
        writeln!(
            summary,
            "view {:<14} coverage {:.2}  mean luminance {:.2}  p5 {:.2}  p95 {:.2}  histogram {}",
            view.name,
            stats.covered_fraction,
            stats.mean,
            stats.low_percentile,
            stats.high_percentile,
            lighting::histogram_line(&stats)
        )
        .unwrap();
        findings.extend(lighting::findings(&view.name, &stats, targets));
        for (slot, current) in best.iter_mut().zip(geometry::measure_view(scene, &aov)) {
            if current.pixels > slot.pixels {
                *slot = current;
            }
        }
    }

    let reports: Vec<geometry::ObjectReport> = scene
        .objects
        .iter()
        .zip(&best)
        .map(|(object, best_view)| geometry::ObjectReport { geometry: geometry::measure_object(&scene.mesh, object), best_view: *best_view })
        .collect();
    writeln!(summary, "\nobjects").unwrap();
    for (object, report) in scene.objects.iter().zip(&reports) {
        writeln!(
            summary,
            "  {:<24} tris {:>7}  size {:.2} x {:.2} x {:.2}  planar {:.2}  unbeveled edge m {:.2}  silhouette solidity {:.2}  px/tri {:.1}",
            object.name,
            object.triangle_count,
            report.geometry.extent.x,
            report.geometry.extent.y,
            report.geometry.extent.z,
            report.geometry.planar_fraction,
            report.geometry.raw_hard_edge_length,
            report.best_view.solidity,
            report.best_view.pixels as f32 / report.best_view.visible_triangles.max(1) as f32
        )
        .unwrap();
    }
    findings.extend(geometry::findings(scene, &reports, targets));
    findings.extend(placement::findings(scene, targets));
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    ProbeReport { findings, summary }
}

fn camera_of(view: &View) -> Camera {
    view.camera
}
