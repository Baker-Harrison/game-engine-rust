use crate::math::Vec3;
use crate::model::arch::{build_house, HouseSpec};
use crate::model::furniture::*;
use crate::model::params::Params;
use crate::model::props::build_carbine;
use crate::model::part::Asset;

pub struct AssetDefinition {
    pub name: &'static str,
    pub summary: &'static str,
    pub parameters: &'static [(&'static str, f32)],
    pub build: fn(&Params) -> Asset,
}

pub fn definitions() -> Vec<AssetDefinition> {
    vec![
        AssetDefinition {
            name: "manor",
            summary: "two-storey gabled manor with arched door, reveals, eaves cornice and slate roof",
            parameters: &[("width", 9.0), ("depth", 6.0), ("storeys", 2.0), ("windows", 3.0), ("pitch", 38.0)],
            build: |p| {
                let spec = crate::model::building::BuildingSpec {
                    width: p.get("width"),
                    depth: p.get("depth"),
                    storeys: p.get("storeys") as usize,
                    windows_per_storey: p.get("windows") as usize,
                    roof_pitch_degrees: p.get("pitch"),
                };
                crate::model::building::build_building(&spec, &crate::model::style::Style::overgrown_manor()).unwrap_or_else(|error| panic!("{}", error))
            },
        },
        AssetDefinition {
            name: "terrace",
            summary: "stone terrace with balustrade, columns, arched wall, cornice and iron gate",
            parameters: &[],
            build: |_| {
                crate::model::terrace::build_terrace(&crate::model::style::Style::overgrown_manor()).unwrap_or_else(|error| panic!("{}", error))
            },
        },
        AssetDefinition {
            name: "swatches",
            summary: "one block per material in the overgrown_manor style",
            parameters: &[("cell", 0.012)],
            build: |p| crate::model::style::build_swatches(&crate::model::style::Style::overgrown_manor(), p.get("cell")),
        },
        AssetDefinition {
            name: "chair",
            summary: "dining chair with raked back, slats and tapered legs",
            parameters: &[
                ("seat_height", 0.46),
                ("seat_width", 0.44),
                ("seat_depth", 0.42),
                ("back_height", 0.44),
                ("back_rake_degrees", 8.0),
                ("slats", 4.0),
                ("leg_splay", 0.015),
                ("edge_radius", 0.012),
                ("stretchers", 1.0),
                ("raise_front_left_leg", 0.0),
                ("cell", 0.005),
            ],
            build: |p| {
                let spec = ChairSpec {
                    seat_height: p.get("seat_height"),
                    seat_width: p.get("seat_width"),
                    seat_depth: p.get("seat_depth"),
                    seat_thickness: 0.04,
                    seat_dish: 0.008,
                    back_height: p.get("back_height"),
                    back_rake_degrees: p.get("back_rake_degrees"),
                    slat_count: p.get("slats") as usize,
                    leg_splay: p.get("leg_splay"),
                    edge_radius: p.get("edge_radius"),
                    stretchers: p.flag("stretchers"),
                    raised_leg_height: p.get("raise_front_left_leg"),
                    cell: p.get("cell"),
                };
                on_stage(build_chair(&spec))
            },
        },
        AssetDefinition {
            name: "stool",
            summary: "bar stool with splayed legs and a ring brace",
            parameters: &[("seat_height", 0.62), ("seat_radius", 0.19), ("leg_splay", 0.07), ("cell", 0.005)],
            build: |p| {
                on_stage(build_stool(&StoolSpec {
                    seat_height: p.get("seat_height"),
                    seat_radius: p.get("seat_radius"),
                    leg_splay: p.get("leg_splay"),
                    cell: p.get("cell"),
                }))
            },
        },
        AssetDefinition {
            name: "table",
            summary: "dining table with aprons",
            parameters: &[("width", 1.4), ("depth", 0.8), ("height", 0.74), ("top_thickness", 0.03), ("cell", 0.006)],
            build: |p| {
                on_stage(build_table(&TableSpec {
                    width: p.get("width"),
                    depth: p.get("depth"),
                    height: p.get("height"),
                    top_thickness: p.get("top_thickness"),
                    cell: p.get("cell"),
                }))
            },
        },
        AssetDefinition {
            name: "bed",
            summary: "double bed with headboard, mattress, duvet and pillows",
            parameters: &[("width", 1.6), ("length", 2.0), ("mattress_top", 0.55), ("cell", 0.008)],
            build: |p| {
                on_stage(build_bed(&BedSpec {
                    width: p.get("width"),
                    length: p.get("length"),
                    mattress_top: p.get("mattress_top"),
                    cell: p.get("cell"),
                }))
            },
        },
        AssetDefinition {
            name: "shelf",
            summary: "bookcase with optional books",
            parameters: &[("width", 0.9), ("depth", 0.3), ("height", 1.8), ("shelves", 5.0), ("books", 1.0), ("cell", 0.006)],
            build: |p| {
                on_stage(build_shelf(&ShelfSpec {
                    width: p.get("width"),
                    depth: p.get("depth"),
                    height: p.get("height"),
                    shelf_count: p.get("shelves") as usize,
                    books: p.flag("books"),
                    cell: p.get("cell"),
                }))
            },
        },
        AssetDefinition {
            name: "carbine",
            summary: "hard-surface carbine prop built from extrusions, revolves, arrays and boolean cuts",
            parameters: &[("cell", 0.0016)],
            build: |p| build_carbine(p.get("cell")),
        },
        AssetDefinition {
            name: "house",
            summary: "two-room single-story house with doors, windows, trim and furniture",
            parameters: &[("ceiling_height", 2.5), ("furnished", 1.0), ("furniture_cell", 0.008)],
            build: |p| {
                build_house(&HouseSpec {
                    ceiling_height: p.get("ceiling_height"),
                    furniture_cell: p.get("furniture_cell"),
                    furnished: p.flag("furnished"),
                })
            },
        },
    ]
}

fn on_stage(mut asset: Asset) -> Asset {
    asset.add_stage_floor(0.0, 0.0, 3.0, 0.1, 0.0, Vec3::new(0.62, 0.62, 0.60));
    asset
}

pub fn find(name: &str) -> Result<AssetDefinition, String> {
    let all = definitions();
    let known: Vec<&str> = all.iter().map(|d| d.name).collect();
    all.into_iter().find(|d| d.name == name).ok_or_else(|| format!("unknown asset '{}' (known: {})", name, known.join(", ")))
}

pub fn build(name: &str, overrides: &[(String, f32)]) -> Result<Asset, String> {
    let definition = find(name)?;
    let mut params = Params::with_defaults(definition.parameters);
    for (key, value) in overrides {
        params.set(key, *value)?;
    }
    Ok((definition.build)(&params))
}

pub fn parse_override(text: &str) -> Result<(String, f32), String> {
    let (key, value) = text.split_once('=').ok_or_else(|| format!("expected key=value, got '{}'", text))?;
    let number = value.parse::<f32>().map_err(|_| format!("'{}' is not a number", value))?;
    Ok((key.to_string(), number))
}
