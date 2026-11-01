use super::material::{Concrete, Material, Plain, Rusted, Surface, WoodGrain};
use super::part::{Asset, Part, PartKind};
use super::sdf::Sdf;
use super::stylized::{PaintedMetal, Sandbag, Skin, Bark, Cobblestone, PaintedPlaster, Slate, StoneBlocks, Weathering, WroughtIron};
use crate::math::Vec3;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct StyleTargets {
    pub min_bevel_radius: f32,
    pub min_bevel_coverage: f32,
    pub max_silhouette_convexity: f32,
    pub texel_density_range: (f32, f32),
    pub max_triangles_per_square_meter: f32,
}

#[derive(Clone, Copy)]
pub enum AlphaKind {
    LeafCutout,
    GlassBlend(f32),
}

#[derive(Clone)]
pub struct Style {
    pub alpha: BTreeMap<String, AlphaKind>,
    pub name: String,
    pub palette: BTreeMap<String, Vec3>,
    pub materials: BTreeMap<String, Arc<dyn Surface>>,
    pub weathering: Weathering,
    pub meters_per_tile: f32,
    pub targets: StyleTargets,
}

impl Style {
    pub fn color(&self, name: &str) -> Result<Vec3, String> {
        self.palette.get(name).copied().ok_or_else(|| format!("style '{}' has no color '{}'; available: {}", self.name, name, join_keys(&self.palette)))
    }

    pub fn material(&self, name: &str) -> Result<Arc<dyn Surface>, String> {
        self.materials.get(name).cloned().ok_or_else(|| format!("style '{}' has no material '{}'; available: {}", self.name, name, join_keys(&self.materials)))
    }

    pub fn with_material(mut self, name: &str, surface: impl Surface + 'static) -> Style {
        self.materials.insert(name.to_string(), Arc::new(surface));
        self
    }

    pub fn overgrown_manor() -> Style {
        let weathering = Weathering {
            moss: 0.4,
            grime: 0.85,
            edge_wear: 0.8,
            moss_color: Vec3::new(0.07, 0.12, 0.04),
            grime_color: Vec3::new(0.07, 0.065, 0.06),
            seed: 21,
        };
        let stone = Vec3::new(0.40, 0.38, 0.34);
        let mortar = Vec3::new(0.36, 0.34, 0.31);
        let mut palette = BTreeMap::new();
        palette.insert("stone".to_string(), stone);
        palette.insert("mortar".to_string(), mortar);
        palette.insert("plaster".to_string(), Vec3::new(0.62, 0.55, 0.42));
        palette.insert("slate".to_string(), Vec3::new(0.22, 0.25, 0.28));
        palette.insert("iron".to_string(), Vec3::new(0.08, 0.08, 0.09));
        palette.insert("timber".to_string(), Vec3::new(0.26, 0.17, 0.10));
        palette.insert("moss".to_string(), weathering.moss_color);
        let style = Style {
            name: "overgrown_manor".to_string(),
            palette,
            materials: BTreeMap::new(),
            alpha: BTreeMap::from([("leaf".to_string(), AlphaKind::LeafCutout), ("glass".to_string(), AlphaKind::GlassBlend(0.25))]),
            weathering,
            meters_per_tile: 1.0,
            targets: StyleTargets {
                min_bevel_radius: 0.015,
                min_bevel_coverage: 0.6,
                max_silhouette_convexity: 0.9,
                texel_density_range: (256.0, 1024.0),
                max_triangles_per_square_meter: 40000.0,
            },
        };
        style
            .with_material("stone_wall", StoneBlocks { stone, mortar, block_size: (0.45, 0.22), joint_width: 0.012, weathering, seed: 31 })
            .with_material("cobble", Cobblestone { stone: Vec3::new(0.34, 0.32, 0.30), gap: Vec3::new(0.15, 0.14, 0.12), cell: 0.22, weathering, seed: 32 })
            .with_material("slate_roof", Slate { slate: Vec3::new(0.22, 0.25, 0.28), tile_size: (0.3, 0.2), weathering, seed: 33 })
            .with_material("wrought_iron", WroughtIron { iron: Vec3::new(0.08, 0.08, 0.09), rust: Vec3::new(0.36, 0.17, 0.08), rust_amount: 0.25, seed: 34 })
            .with_material(
                "plaster_wall",
                PaintedPlaster { paint: Vec3::new(0.66, 0.60, 0.46), plaster: Vec3::new(0.55, 0.50, 0.42), peel: 0.18, weathering, seed: 35 },
            )
            .with_material("bark", Bark { bark: Vec3::new(0.24, 0.18, 0.13), groove: Vec3::new(0.08, 0.06, 0.05), ridge_frequency: 9.0, weathering, seed: 36 })
            .with_material("skin", Skin { base: Vec3::new(0.62, 0.44, 0.34), flush: Vec3::new(0.70, 0.40, 0.32), seed: 37 })
            .with_material("glass", Plain { material: Material::new(Vec3::new(0.35, 0.45, 0.50), 0.05, 0.0) })
            .with_material("leaf", Plain { material: Material::new(Vec3::new(0.13, 0.24, 0.06), 0.6, 0.0) })
            .with_material("timber", Plain { material: Material::new(Vec3::new(0.26, 0.17, 0.10), 0.7, 0.0) })
    }
}

impl Style {
    pub fn desert_industrial() -> Style {
        let weathering = Weathering {
            moss: 0.0,
            grime: 0.7,
            edge_wear: 0.9,
            moss_color: Vec3::new(0.2, 0.25, 0.1),
            grime_color: Vec3::new(0.10, 0.08, 0.06),
            seed: 51,
        };
        let mut palette = BTreeMap::new();
        palette.insert("sand".to_string(), Vec3::new(0.62, 0.50, 0.34));
        palette.insert("rust_red".to_string(), Vec3::new(0.46, 0.17, 0.10));
        palette.insert("faded_blue".to_string(), Vec3::new(0.18, 0.30, 0.44));
        palette.insert("olive".to_string(), Vec3::new(0.26, 0.30, 0.18));
        let paint = |color: Vec3, seed: u32| PaintedMetal { paint: color, rust: Vec3::new(0.34, 0.16, 0.08), chipping: 0.35, streaks: 0.8, weathering, seed };
        let style = Style {
            name: "desert_industrial".to_string(),
            palette,
            materials: BTreeMap::new(),
            alpha: BTreeMap::from([("glass".to_string(), AlphaKind::GlassBlend(0.25))]),
            weathering,
            meters_per_tile: 2.0,
            targets: StyleTargets {
                min_bevel_radius: 0.015,
                min_bevel_coverage: 0.5,
                max_silhouette_convexity: 0.95,
                texel_density_range: (128.0, 1024.0),
                max_triangles_per_square_meter: 40000.0,
            },
        };
        style
            .with_material("container_red", paint(Vec3::new(0.46, 0.17, 0.10), 61))
            .with_material("container_blue", paint(Vec3::new(0.18, 0.30, 0.44), 62))
            .with_material("container_sand", paint(Vec3::new(0.62, 0.50, 0.34), 63))
            .with_material("olive_metal", paint(Vec3::new(0.26, 0.30, 0.18), 64))
            .with_material("concrete", Concrete { base: Vec3::new(0.55, 0.53, 0.49), stain: 0.25, seed: 65 })
            .with_material("sandbag", Sandbag { burlap: Vec3::new(0.60, 0.50, 0.34), seed: 66 })
            .with_material("pallet_wood", WoodGrain { light: Vec3::new(0.55, 0.40, 0.24), dark: Vec3::new(0.30, 0.20, 0.12), grain_axis: Vec3::new(1.0, 0.0, 0.0), scale: 14.0, roughness: 0.8, seed: 67 })
            .with_material("steel", PaintedMetal { paint: Vec3::new(0.30, 0.31, 0.32), rust: Vec3::new(0.30, 0.15, 0.08), chipping: 0.5, streaks: 0.6, weathering, seed: 68 })
            .with_material("rusted_steel", Rusted { metal: Material::new(Vec3::new(0.25, 0.25, 0.26), 0.6, 0.8), rust: Vec3::new(0.38, 0.17, 0.08), amount: 0.55, seed: 69 })
            .with_material("office_wall", PaintedPlaster { paint: Vec3::new(0.70, 0.64, 0.52), plaster: Vec3::new(0.55, 0.50, 0.42), peel: 0.12, weathering, seed: 70 })
            .with_material("timber", Plain { material: Material::new(Vec3::new(0.30, 0.20, 0.12), 0.8, 0.0) })
            .with_material("glass", Plain { material: Material::new(Vec3::new(0.35, 0.45, 0.50), 0.05, 0.0) })
    }
}

fn join_keys<V>(map: &BTreeMap<String, V>) -> String {
    map.keys().cloned().collect::<Vec<_>>().join(", ")
}

pub fn build_swatches(style: &Style, cell: f32) -> Asset {
    let mut asset = Asset::new("swatches");
    let spacing = 1.0;
    for (index, (name, surface)) in style.materials.iter().enumerate() {
        let block = Sdf::round_box(Vec3::new(0.8, 0.8, 0.8), 0.04).translate(Vec3::new((index % 3) as f32 * spacing, 0.4, (index / 3) as f32 * spacing));
        asset.parts.push(Part::from_sdf_surface(name, PartKind::Object, &block, cell, &**surface, Vec::new()));
    }
    asset
}
