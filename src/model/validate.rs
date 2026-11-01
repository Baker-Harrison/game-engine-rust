use super::part::{Asset, Footprint, Member, Part, PartKind};
use crate::math::Vec3;
use std::collections::{BTreeMap, HashMap, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Ok,
    Warn,
    Error,
}

pub struct Finding {
    pub severity: Severity,
    pub subject: String,
    pub message: String,
}

impl Finding {
    fn new(severity: Severity, subject: &str, message: String) -> Finding {
        Finding { severity, subject: subject.to_string(), message }
    }

    pub fn format(&self) -> String {
        let label = match self.severity {
            Severity::Ok => "ok",
            Severity::Warn => "WARN",
            Severity::Error => "ERROR",
        };
        format!("{:<34} {:<5} {}", self.subject, label, self.message)
    }
}

const MEMBER_CONTACT_GAP: f32 = 0.004;
const FLOOR_CONTACT_HEIGHT: f32 = 0.004;
const SUPPORT_SAMPLE_HEIGHT: f32 = 0.006;
const STABILITY_WARNING_MARGIN: f32 = 0.02;
const SHARP_EDGE_WARNING_FRACTION: f32 = 0.15;

pub fn validate(asset: &Asset) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_dimensions(asset, &mut findings);
    check_surfaces(asset, &mut findings);
    let groups = object_groups(asset);
    for (group, parts) in &groups {
        let subject = if group.is_empty() { asset.name.as_str() } else { group.as_str() };
        check_member_contact(subject, parts, &mut findings);
        check_floor_contact(subject, parts, &mut findings);
        check_stability(subject, parts, &mut findings);
    }
    check_layout(asset, &mut findings);
    check_reachability(asset, &mut findings);
    findings
}

fn group_of(part_name: &str) -> String {
    part_name.split_once('.').map(|(prefix, _)| prefix.to_string()).unwrap_or_default()
}

fn object_groups(asset: &Asset) -> BTreeMap<String, Vec<&Part>> {
    let mut groups: BTreeMap<String, Vec<&Part>> = BTreeMap::new();
    for part in asset.parts.iter().filter(|part| part.kind == PartKind::Object) {
        groups.entry(group_of(&part.name)).or_default().push(part);
    }
    groups
}

fn check_dimensions(asset: &Asset, findings: &mut Vec<Finding>) {
    for dimension in &asset.dimensions {
        let subject = format!("{} {}", dimension.subject, dimension.name);
        let text = format!("{:.3} m (usual {:.2} to {:.2})", dimension.value, dimension.min, dimension.max);
        if dimension.value < dimension.min || dimension.value > dimension.max {
            findings.push(Finding::new(Severity::Warn, &subject, format!("{} is outside the usual range", text)));
        } else {
            findings.push(Finding::new(Severity::Ok, &subject, text));
        }
    }
}

fn check_surfaces(asset: &Asset, findings: &mut Vec<Finding>) {
    for part in asset.parts.iter().filter(|part| part.kind == PartKind::Object) {
        let report = part.mesh.topology_report();
        if !report.watertight {
            findings.push(Finding::new(
                Severity::Error,
                &format!("{} surface", part.name),
                format!("not watertight: {} open edges, {} non-manifold edges", report.open_edges, report.nonmanifold_edges),
            ));
        }
        if report.sharp_edge_fraction > SHARP_EDGE_WARNING_FRACTION {
            findings.push(Finding::new(
                Severity::Warn,
                &format!("{} edges", part.name),
                format!(
                    "{:.0}% of edges are sharper than 35 degrees (largest {:.0}); raise the edge radius or mesh detail",
                    report.sharp_edge_fraction * 100.0,
                    report.max_edge_angle_degrees
                ),
            ));
        }
    }
}

fn members_of<'a>(parts: &[&'a Part]) -> Vec<&'a Member> {
    parts.iter().flat_map(|part| part.members.iter()).collect()
}

fn members_touch(a: &Member, b: &Member) -> bool {
    let gap = |low_a: f32, high_a: f32, low_b: f32, high_b: f32| (low_a - high_b).max(low_b - high_a).max(0.0);
    let dx = gap(a.min.x, a.max.x, b.min.x, b.max.x);
    let dy = gap(a.min.y, a.max.y, b.min.y, b.max.y);
    let dz = gap(a.min.z, a.max.z, b.min.z, b.max.z);
    (dx * dx + dy * dy + dz * dz).sqrt() <= MEMBER_CONTACT_GAP
}

fn check_member_contact(subject: &str, parts: &[&Part], findings: &mut Vec<Finding>) {
    let members = members_of(parts);
    if members.len() < 2 {
        return;
    }
    let mut reached = vec![false; members.len()];
    let mut queue = VecDeque::from([0usize]);
    reached[0] = true;
    while let Some(current) = queue.pop_front() {
        for other in 0..members.len() {
            if !reached[other] && members_touch(members[current], members[other]) {
                reached[other] = true;
                queue.push_back(other);
            }
        }
    }
    let detached: Vec<&str> = members.iter().zip(&reached).filter(|(_, r)| !**r).map(|(m, _)| m.name.as_str()).collect();
    if detached.is_empty() {
        findings.push(Finding::new(Severity::Ok, &format!("{} parts connected", subject), format!("{} members form one body", members.len())));
    } else {
        findings.push(Finding::new(
            Severity::Error,
            &format!("{} parts connected", subject),
            format!("floating: {} not touching the rest (gap over {:.0} mm)", detached.join(", "), MEMBER_CONTACT_GAP * 1000.0),
        ));
    }
}

fn lowest_height_in_column(parts: &[&Part], member: &Member) -> f32 {
    let mut lowest = f32::MAX;
    for part in parts {
        for p in &part.mesh.positions {
            if p.x >= member.min.x && p.x <= member.max.x && p.z >= member.min.z && p.z <= member.max.z {
                lowest = lowest.min(p.y);
            }
        }
    }
    lowest
}

fn check_floor_contact(subject: &str, parts: &[&Part], findings: &mut Vec<Finding>) {
    let mut checked = 0;
    for member in members_of(parts).into_iter().filter(|m| m.touches_floor) {
        checked += 1;
        let lowest = lowest_height_in_column(parts, member);
        if lowest > FLOOR_CONTACT_HEIGHT {
            findings.push(Finding::new(
                Severity::Error,
                &format!("{} {}", subject, member.name),
                format!("bottom is {:.0} mm above the floor", lowest * 1000.0),
            ));
        }
    }
    if checked > 0 && !findings.iter().any(|f| f.severity == Severity::Error && f.subject.starts_with(subject) && f.message.contains("above the floor")) {
        findings.push(Finding::new(Severity::Ok, &format!("{} floor contact", subject), format!("{} supports reach the floor", checked)));
    }
}

fn check_stability(subject: &str, parts: &[&Part], findings: &mut Vec<Finding>) {
    if !parts.iter().any(|part| part.members.iter().any(|m| m.touches_floor)) {
        return;
    }
    let support: Vec<(f32, f32)> = parts
        .iter()
        .flat_map(|part| part.mesh.positions.iter())
        .filter(|p| p.y < SUPPORT_SAMPLE_HEIGHT)
        .map(|p| (p.x, p.z))
        .collect();
    let hull = convex_hull(support);
    if hull.len() < 3 {
        findings.push(Finding::new(Severity::Error, &format!("{} stability", subject), "fewer than three ground contact points".to_string()));
        return;
    }
    let mut weighted = Vec3::ZERO;
    let mut total = 0.0;
    for part in parts {
        let volume = part.mesh.volume();
        weighted += part.mesh.centroid_of_volume() * volume;
        total += volume;
    }
    let center = weighted / total;
    let margin = hull_margin(&hull, (center.x, center.z));
    let message = format!("center of mass at height {:.2} m, margin to tipping edge {:.0} mm", center.y, margin * 1000.0);
    let severity = if margin < 0.0 {
        Severity::Error
    } else if margin < STABILITY_WARNING_MARGIN {
        Severity::Warn
    } else {
        Severity::Ok
    };
    findings.push(Finding::new(severity, &format!("{} stability", subject), message));
}

fn convex_hull(mut points: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut lower: Vec<(f32, f32)> = Vec::new();
    for p in &points {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<(f32, f32)> = Vec::new();
    for p in points.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn hull_margin(hull: &[(f32, f32)], point: (f32, f32)) -> f32 {
    let mut margin = f32::MAX;
    for i in 0..hull.len() {
        let (a, b) = (hull[i], hull[(i + 1) % hull.len()]);
        let edge = (b.0 - a.0, b.1 - a.1);
        let length = (edge.0 * edge.0 + edge.1 * edge.1).sqrt().max(1e-9);
        let signed = (edge.0 * (point.1 - a.1) - edge.1 * (point.0 - a.0)) / length;
        margin = margin.min(signed);
    }
    margin
}

fn inside(outer: &Footprint, inner: &Footprint) -> bool {
    inner.min_x >= outer.min_x - 0.002 && inner.max_x <= outer.max_x + 0.002 && inner.min_z >= outer.min_z - 0.002 && inner.max_z <= outer.max_z + 0.002
}

fn check_layout(asset: &Asset, findings: &mut Vec<Finding>) {
    for footprint in &asset.footprints {
        let room = asset.rooms.iter().find(|room| inside(&room.area, footprint));
        match room {
            Some(room) => findings.push(Finding::new(Severity::Ok, &format!("{} placement", footprint.subject), format!("inside {}", room.name))),
            None => findings.push(Finding::new(Severity::Error, &format!("{} placement", footprint.subject), "not fully inside a room".to_string())),
        }
    }
    for (index, first) in asset.footprints.iter().enumerate() {
        for second in &asset.footprints[index + 1..] {
            if first.overlaps(second) {
                findings.push(Finding::new(Severity::Error, &format!("{} overlap", first.subject), format!("intersects {}", second.subject)));
            }
        }
    }
    for zone in &asset.clearance_zones {
        let blocker = asset.footprints.iter().find(|f| f.subject != zone.owner && f.overlaps(&zone.area));
        let in_room = asset.rooms.iter().any(|room| inside(&room.area, &zone.area));
        let subject = format!("{} clearance", zone.area.subject);
        match (blocker, in_room) {
            (Some(blocker), _) => findings.push(Finding::new(Severity::Error, &subject, format!("0.6 m clear space blocked by {}", blocker.subject))),
            (None, false) => findings.push(Finding::new(Severity::Warn, &subject, "clear space runs into a wall".to_string())),
            (None, true) => findings.push(Finding::new(Severity::Ok, &subject, "0.6 m clear".to_string())),
        }
    }
    for door in &asset.doors {
        let subject = format!("{} swing", door.name);
        match asset.footprints.iter().find(|f| f.overlaps(&door.swing_area)) {
            Some(blocker) => findings.push(Finding::new(Severity::Error, &subject, format!("door swing blocked by {}", blocker.subject))),
            None => findings.push(Finding::new(Severity::Ok, &subject, "swing area clear".to_string())),
        }
    }
}

fn check_reachability(asset: &Asset, findings: &mut Vec<Finding>) {
    if asset.rooms.is_empty() {
        return;
    }
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for door in &asset.doors {
        graph.entry(door.from.as_str()).or_default().push(door.to.as_str());
        graph.entry(door.to.as_str()).or_default().push(door.from.as_str());
    }
    let mut seen = vec!["outside"];
    let mut queue = VecDeque::from(["outside"]);
    while let Some(node) = queue.pop_front() {
        for next in graph.get(node).into_iter().flatten() {
            if !seen.contains(next) {
                seen.push(next);
                queue.push_back(next);
            }
        }
    }
    for room in &asset.rooms {
        let (severity, message) = if seen.contains(&room.name.as_str()) {
            (Severity::Ok, "reachable from outside through doors".to_string())
        } else {
            (Severity::Error, "no door path from outside".to_string())
        };
        findings.push(Finding::new(severity, &format!("{} access", room.name), message));
    }
}

pub fn summarize(findings: &[Finding]) -> (usize, usize, usize) {
    let count = |severity| findings.iter().filter(|f| f.severity == severity).count();
    (count(Severity::Ok), count(Severity::Warn), count(Severity::Error))
}
