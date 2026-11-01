use super::mesh::Mesh;
use super::part::{Member, Part, PartKind};
use super::sdf::Sdf;
use super::stats::MeshStats;
use super::style::Style;
use super::mesher::mesh_sdf_surface;

const MIN_CELLS_PER_FEATURE: f32 = 2.0;

#[derive(Clone, Copy)]
pub struct DetailSpec {
    pub smallest_feature: f32,
    pub cells_per_feature: f32,
    pub max_triangles: usize,
}

impl DetailSpec {
    pub fn new(smallest_feature: f32) -> DetailSpec {
        DetailSpec { smallest_feature, cells_per_feature: 3.0, max_triangles: 400_000 }
    }

    pub fn with_budget(mut self, max_triangles: usize) -> DetailSpec {
        self.max_triangles = max_triangles;
        self
    }

    pub fn cell(&self) -> f32 {
        self.smallest_feature / self.cells_per_feature
    }
}

pub fn build_styled_part(
    name: &str,
    kind: PartKind,
    sdf: &Sdf,
    style: &Style,
    material: &str,
    detail: DetailSpec,
    members: Vec<Member>,
) -> Result<Part, String> {
    if detail.cells_per_feature < MIN_CELLS_PER_FEATURE {
        return Err(format!("{}: cells_per_feature {} is below {}, thin features would tear", name, detail.cells_per_feature, MIN_CELLS_PER_FEATURE));
    }
    let surface = style.material(material)?;
    let mut mesh = mesh_sdf_surface(sdf, detail.cell(), &*surface);
    mesh.generate_uvs(style.meters_per_tile);
    check_mesh(name, &mesh, style, detail.max_triangles)?;
    Ok(Part { name: name.to_string(), kind, mesh, members })
}

pub fn build_styled_mesh_part(name: &str, kind: PartKind, mut mesh: Mesh, style: &Style, material: &str, members: Vec<Member>) -> Result<Part, String> {
    let surface = style.material(material)?;
    mesh.apply_surface(&*surface);
    mesh.generate_uvs(style.meters_per_tile);
    check_mesh(name, &mesh, style, usize::MAX)?;
    Ok(Part { name: name.to_string(), kind, mesh, members })
}

pub fn check_mesh(name: &str, mesh: &Mesh, style: &Style, max_triangles: usize) -> Result<(), String> {
    let stats = MeshStats::measure(mesh);
    if stats.triangles > max_triangles {
        return Err(format!(
            "{}: {} triangles exceeds budget {}; raise smallest_feature or the budget",
            name, stats.triangles, max_triangles
        ));
    }
    if stats.bevel_coverage < style.targets.min_bevel_coverage {
        return Err(format!(
            "{}: bevel coverage {:.2} is below the style minimum {:.2}; sharp 90 degree edges found, build with Sdf::round_box radius >= {} m",
            name, stats.bevel_coverage, style.targets.min_bevel_coverage, style.targets.min_bevel_radius
        ));
    }
    Ok(())
}
