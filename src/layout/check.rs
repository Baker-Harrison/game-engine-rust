use super::nav::{anchor_node, NavGrid};
use super::solver::placement_problems;
use super::{ItemKind, Layout, Purpose, MIN_REASON_LENGTH};
use crate::probe::{Finding, Severity};

const SPAWN_CLEAR_RADIUS: f32 = 3.0;

pub fn check(layout: &Layout) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_reasons(layout, &mut findings);
    check_placement(layout, &mut findings);
    check_support(layout, &mut findings);
    check_spawns(layout, &mut findings);
    check_circulation(layout, &mut findings);
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    findings
}

fn error(subject: &str, metric: &str, value: f32, target: &str, hint: &str) -> Finding {
    Finding::new(Severity::Error, subject, metric, value, target.to_string(), hint)
}

fn check_reasons(layout: &Layout, findings: &mut Vec<Finding>) {
    for item in &layout.items {
        if item.kind != ItemKind::Dressing && item.reason.trim().len() < MIN_REASON_LENGTH {
            findings.push(error(&item.name, "placement_reason_length", item.reason.trim().len() as f32, ">= 8 characters", "state why this exists, for example 'cover for the north lane'"));
        }
    }
}

fn check_placement(layout: &Layout, findings: &mut Vec<Finding>) {
    for (index, item) in layout.items.iter().enumerate() {
        for problem in placement_problems(layout, &layout.items[..index], item) {
            findings.push(error(&item.name, "placement", 0.0, "valid", &format!("{}; move or resize it", problem)));
        }
    }
}

fn check_support(layout: &Layout, findings: &mut Vec<Finding>) {
    for item in layout.items.iter().filter(|i| i.footprint.y_min > super::STEP_HEIGHT && i.kind != ItemKind::Dressing) {
        let supports: Vec<_> = layout
            .items
            .iter()
            .filter(|s| s.name != item.name && s.footprint.y_max >= item.footprint.y_min - 0.1 && s.footprint.y_max <= item.footprint.y_min + 0.1 && s.footprint.separation(&item.footprint) <= 0.05)
            .collect();
        let center_supported = supports.iter().any(|s| s.footprint.contains_point(item.footprint.x, item.footprint.z));
        let spanned = supports.len() >= 2 && {
            let low_x = supports.iter().map(|s| s.footprint.x).fold(f32::MAX, f32::min);
            let high_x = supports.iter().map(|s| s.footprint.x).fold(f32::MIN, f32::max);
            let low_z = supports.iter().map(|s| s.footprint.z).fold(f32::MAX, f32::min);
            let high_z = supports.iter().map(|s| s.footprint.z).fold(f32::MIN, f32::max);
            item.footprint.x >= low_x && item.footprint.x <= high_x && item.footprint.z >= low_z && item.footprint.z <= high_z
        };
        if !center_supported && !spanned {
            findings.push(error(&item.name, "support_count", supports.len() as f32, "centre carried or spanned by 2+ supports", "add columns or walls under it that reach the ground"));
        }
    }
}

fn check_spawns(layout: &Layout, findings: &mut Vec<Finding>) {
    for zone in layout.zones.iter().filter(|z| z.purpose == Purpose::Spawn) {
        let (x, z) = zone.bounds.center();
        for item in layout.items.iter().filter(|i| i.kind.blocks_ground(&i.footprint) && i.zone != zone.name) {
            let distance = item.footprint.distance_to_point(x, z);
            if distance < SPAWN_CLEAR_RADIUS {
                findings.push(error(&zone.name, "spawn_clearance_m", distance, ">= 3", &format!("{} crowds the spawn; move it away", item.name)));
            }
        }
    }
}

fn check_circulation(layout: &Layout, findings: &mut Vec<Finding>) {
    for link in &layout.links {
        let grid = NavGrid::build(layout, link.width);
        let (Some(start), Some(goal)) = (anchor_node(&grid, layout, &link.from), anchor_node(&grid, layout, &link.to)) else {
            findings.push(error(&format!("{} -> {}", link.from, link.to), "route_anchor", 0.0, "reachable zone anchor", "zone centre is walled in; clear space or add a portal"));
            continue;
        };
        let routes = grid.count_routes(start, goal, link.min_routes);
        if routes < link.min_routes {
            findings.push(error(
                &format!("{} -> {}", link.from, link.to),
                "disjoint_routes",
                routes as f32,
                &format!(">= {} at width {:.1} m", link.min_routes, link.width),
                "open a gap or add stairs so a second separate route exists",
            ));
        }
    }
}

pub fn report(findings: &[Finding]) -> String {
    let errors = findings.iter().filter(|f| f.severity == Severity::Error).count();
    let mut out = format!("layout check: {} errors\n", errors);
    for finding in findings {
        out.push_str(&format!("  {} {}: {} = {:.2} (target {}) -> {}\n", if finding.severity == Severity::Error { "ERROR" } else { "WARN " }, finding.subject, finding.metric, finding.value, finding.target, finding.hint));
    }
    out
}
