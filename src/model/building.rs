use super::detail::{build_styled_part, DetailSpec};
use super::ornament::{cornice, Ornament};
use super::part::{Asset, Part, PartKind};
use super::sdf::Sdf;
use super::style::Style;
use crate::math::Vec3;

pub const MAX_RISE: f32 = 0.19;
pub const MIN_RUN: f32 = 0.25;
const WALL_THICKNESS: f32 = 0.45;
const STOREY_HEIGHT: f32 = 3.2;
const SLAB_THICKNESS: f32 = 0.25;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub struct StairSpec {
    pub total_rise: f32,
    pub width: f32,
    pub run: f32,
}

impl StairSpec {
    pub fn step_count(&self) -> usize {
        (self.total_rise / MAX_RISE).ceil() as usize
    }

    pub fn rise(&self) -> f32 {
        self.total_rise / self.step_count() as f32
    }

    pub fn check(&self) -> Result<(), String> {
        if self.run < MIN_RUN {
            return Err(format!("stair run {:.2} m is below the {:.2} m minimum", self.run, MIN_RUN));
        }
        let comfort = 2.0 * self.rise() + self.run;
        if !(0.55..=0.70).contains(&comfort) {
            return Err(format!("stair comfort 2*rise+run = {:.2} m is outside 0.55 to 0.70", comfort));
        }
        Ok(())
    }

    pub fn length(&self) -> f32 {
        self.step_count() as f32 * self.run
    }
}

pub fn stairs(spec: &StairSpec) -> Result<Ornament, String> {
    spec.check()?;
    let rise = spec.rise();
    let steps: Vec<Sdf> = (0..spec.step_count())
        .map(|index| {
            let top = rise * (index + 1) as f32;
            Sdf::round_box(v(spec.width, top, spec.run + 0.04), 0.012).translate(v(0.0, top * 0.5, -(index as f32 + 0.5) * spec.run))
        })
        .collect();
    Ok(Ornament { sdf: Sdf::smooth_union_all(steps, 0.004), smallest_feature: 0.06 })
}

pub fn gable_roof(width: f32, depth: f32, pitch_degrees: f32, overhang: f32, thickness: f32) -> Ornament {
    let half = width * 0.5 + overhang;
    let rise = half * pitch_degrees.to_radians().tan();
    let outer = [[-half, 0.0], [half, 0.0], [0.0, rise]];
    let inner_scale = 1.0 - thickness / rise.max(0.1);
    let inner = [[-half * inner_scale, 0.0], [half * inner_scale, 0.0], [0.0, rise * inner_scale]];
    let shell = Sdf::extrude(&outer, depth + overhang * 2.0, 0.03).subtract(Sdf::extrude(&inner, depth + overhang * 2.0 + 0.2, 0.0).translate(v(0.0, -0.01, 0.0)));
    Ornament { sdf: shell, smallest_feature: thickness }
}

pub fn gable_fill(width: f32, pitch_degrees: f32) -> Ornament {
    let half = width * 0.5;
    let rise = half * pitch_degrees.to_radians().tan();
    let triangle = [[-half, 0.0], [half, 0.0], [0.0, rise]];
    Ornament { sdf: Sdf::extrude(&triangle, WALL_THICKNESS, 0.02).translate(v(half, 0.0, 0.0)), smallest_feature: 0.09 }
}

pub struct Opening {
    pub x: f32,
    pub sill: f32,
    pub width: f32,
    pub height: f32,
    pub arched: bool,
}

pub fn wall_with_openings(length: f32, height: f32, openings: &[Opening]) -> Ornament {
    let wall = Sdf::round_box(v(length, height, WALL_THICKNESS), 0.02).translate(v(length * 0.5, height * 0.5, 0.0));
    let mut result = wall;
    for opening in openings {
        let straight = if opening.arched { opening.height - opening.width * 0.5 } else { opening.height };
        let mut hole = Sdf::round_box(v(opening.width, straight, WALL_THICKNESS * 2.0), 0.0).translate(v(opening.x, opening.sill + straight * 0.5, 0.0));
        if opening.arched {
            hole = hole.union(Sdf::cylinder(opening.width * 0.5, WALL_THICKNESS, 0.0).rotate_x(90.0).translate(v(opening.x, opening.sill + straight, 0.0)));
        }
        let reveal = Sdf::round_box(v(opening.width + 0.2, opening.height + 0.2, WALL_THICKNESS * 0.35), 0.02)
            .translate(v(opening.x, opening.sill + opening.height * 0.5 + 0.05, WALL_THICKNESS * 0.5));
        result = result.union(reveal.subtract(hole.clone())).subtract(hole);
    }
    Ornament { sdf: result, smallest_feature: 0.06 }
}

fn opening_shape(opening: &Opening, inset: f32, thickness: f32) -> Sdf {
    let width = opening.width - inset * 2.0;
    let height = opening.height - inset * 2.0;
    let straight = if opening.arched { height - width * 0.5 } else { height };
    let mut shape = Sdf::round_box(v(width, straight, thickness), 0.0).translate(v(0.0, inset + straight * 0.5, 0.0));
    if opening.arched {
        shape = shape.union(Sdf::cylinder(width * 0.5, thickness * 0.5, 0.0).rotate_x(90.0).translate(v(0.0, inset + straight, 0.0)));
    }
    shape
}

pub fn window_frame(opening: &Opening) -> Ornament {
    let bar = 0.05;
    let outer = opening_shape(opening, 0.0, 0.1);
    let inner = opening_shape(opening, bar, 0.2);
    let mullion_vertical = Sdf::round_box(v(0.03, opening.height - 0.1, 0.07), 0.008).translate(v(0.0, opening.height * 0.5, 0.0));
    let mullion_horizontal = Sdf::round_box(v(opening.width - 0.1, 0.03, 0.07), 0.008).translate(v(0.0, opening.height * 0.55, 0.0));
    let sill = Sdf::round_box(v(opening.width + 0.14, 0.05, 0.2), 0.012).translate(v(0.0, -0.02, 0.06));
    Ornament { sdf: outer.subtract(inner).union(mullion_vertical).union(mullion_horizontal).union(sill), smallest_feature: 0.06 }
}

pub fn window_glass(opening: &Opening) -> Ornament {
    Ornament { sdf: opening_shape(opening, 0.03, 0.012), smallest_feature: 0.036 }
}

pub struct BuildingSpec {
    pub width: f32,
    pub depth: f32,
    pub storeys: usize,
    pub windows_per_storey: usize,
    pub roof_pitch_degrees: f32,
}

fn styled(name: &str, ornament: &Ornament, style: &Style, material: &str) -> Result<Part, String> {
    let flat = material == "plaster_wall";
    let base = DetailSpec::new(ornament.smallest_feature).with_budget(900_000);
    let detail = if flat { base.simplified_flat(3.0) } else { base };
    build_styled_part(name, PartKind::Architecture, &ornament.sdf, style, material, detail, Vec::new())
}

fn placed(part: &Part, name: &str, position: Vec3, yaw_degrees: f32) -> Part {
    let mut copy = part.clone();
    copy.name = name.to_string();
    copy.mesh.rotate_y(yaw_degrees);
    copy.mesh.translate(position);
    copy
}

pub fn build_building(spec: &BuildingSpec, style: &Style) -> Result<Asset, String> {
    let mut asset = Asset::new("building");
    let height = spec.storeys as f32 * STOREY_HEIGHT;
    let window = |x: f32, storey: usize| Opening { x, sill: storey as f32 * STOREY_HEIGHT + 0.9, width: 0.9, height: 1.5, arched: storey == 0 };

    let mut front_openings = vec![Opening { x: spec.width * 0.5, sill: 0.0, width: 1.2, height: 2.4, arched: true }];
    for storey in 0..spec.storeys {
        for slot in 0..spec.windows_per_storey {
            let x = spec.width * (slot as f32 + 0.5) / spec.windows_per_storey as f32;
            if storey == 0 && (x - spec.width * 0.5).abs() < 1.2 {
                continue;
            }
            front_openings.push(window(x, storey));
        }
    }
    let front = styled("front", &wall_with_openings(spec.width, height, &front_openings), style, "plaster_wall")?;
    let back_openings: Vec<Opening> = (0..spec.storeys).flat_map(|s| (0..spec.windows_per_storey).map(move |slot| (s, slot))).map(|(s, slot)| window(spec.width * (slot as f32 + 0.5) / spec.windows_per_storey as f32, s)).collect();
    let back = styled("back", &wall_with_openings(spec.width, height, &back_openings), style, "plaster_wall")?;
    let side_openings: Vec<Opening> = (0..spec.storeys).map(|s| window(spec.depth * 0.5, s)).collect();
    let side = styled("side", &wall_with_openings(spec.depth, height, &side_openings), style, "plaster_wall")?;

    asset.parts.push(placed(&front, "wall_front", v(0.0, 0.0, spec.depth * 0.5), 0.0));
    asset.parts.push(placed(&back, "wall_back", v(spec.width, 0.0, -spec.depth * 0.5), 180.0));
    asset.parts.push(placed(&side, "wall_left", v(-spec.width * 0.5 + WALL_THICKNESS * 0.5 + spec.width * 0.5, 0.0, spec.depth * 0.5), 90.0));
    asset.parts.push(placed(&side, "wall_right", v(spec.width, 0.0, -spec.depth * 0.5), -90.0));

    let glass_material = "glass";
    let mut window_cache: std::collections::HashMap<(u32, u32, bool), (Part, Part)> = std::collections::HashMap::new();
    let mut openings_by_wall: Vec<(&str, Vec3, f32, f32, &[Opening])> = Vec::new();
    openings_by_wall.push(("front", v(0.0, 0.0, spec.depth * 0.5), 0.0, spec.width, &front_openings));
    openings_by_wall.push(("back", v(spec.width, 0.0, -spec.depth * 0.5), 180.0, spec.width, &back_openings));
    openings_by_wall.push(("left", v(WALL_THICKNESS * 0.5, 0.0, spec.depth * 0.5), 90.0, spec.depth, &side_openings));
    openings_by_wall.push(("right", v(spec.width, 0.0, -spec.depth * 0.5), -90.0, spec.depth, &side_openings));
    for (wall_name, origin, yaw, _length, openings) in openings_by_wall {
        for (index, opening) in openings.iter().enumerate() {
            let at = Vec3::new(opening.x, opening.sill, 0.0);
            let key = (opening.width.to_bits(), opening.height.to_bits(), opening.arched);
            if !window_cache.contains_key(&key) {
                let frame_part = styled("frame", &window_frame(opening), style, "timber")?;
                let glass_part = styled("glass", &window_glass(opening), style, glass_material)?;
                window_cache.insert(key, (frame_part, glass_part));
            }
            let (frame_part, glass_part) = window_cache[&key].clone();
            for (kind, part) in [("frame", frame_part), ("glass", glass_part)] {
                let mut copy = part.clone();
                copy.name = format!("window_{}_{}_{}", wall_name, index, kind);
                copy.mesh.translate(at);
                copy.mesh.rotate_y(yaw);
                copy.mesh.translate(origin);
                asset.parts.push(copy);
            }
        }
    }

    let fill = styled("gable", &gable_fill(spec.width, spec.roof_pitch_degrees), style, "plaster_wall")?;
    asset.parts.push(placed(&fill, "gable_front", v(0.0, height, spec.depth * 0.5), 0.0));
    asset.parts.push(placed(&fill, "gable_back", v(spec.width, height, -spec.depth * 0.5), 180.0));

    for storey in 0..=spec.storeys {
        let slab = Sdf::round_box(v(spec.width, SLAB_THICKNESS, spec.depth), 0.03).translate(v(spec.width * 0.5, storey as f32 * STOREY_HEIGHT - SLAB_THICKNESS * 0.5, 0.0));
        asset.parts.push(build_styled_part(&format!("slab_{}", storey), PartKind::Architecture, &slab, style, "cobble", DetailSpec::new(0.09).simplified_flat(3.0), Vec::new())?);
    }

    let belt = styled("cornice", &cornice(spec.width + 0.4, 0.3, 0.35), style, "stone_wall")?;
    asset.parts.push(placed(&belt, "eaves_cornice", v(-0.2, height - 0.2, spec.depth * 0.5 - 0.1), 0.0));

    let roof = styled("roof", &gable_roof(spec.width, spec.depth, spec.roof_pitch_degrees, 0.35, 0.18), style, "slate_roof")?;
    asset.parts.push(placed(&roof, "roof", v(spec.width * 0.5, height, 0.0), 0.0));
    asset.roof_part_names = vec!["roof".to_string()];
    Ok(asset)
}
