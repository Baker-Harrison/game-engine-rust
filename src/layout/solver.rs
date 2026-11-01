use super::footprint::Footprint;
use super::{Item, Layout, Request, FLUSH_TOLERANCE, OVERLAP_TOLERANCE, WALK_CLEARANCE};
use crate::probe::{Finding, Severity};
use crate::random::Random;

const CANDIDATES_PER_ITEM: usize = 600;

pub fn placement_problems(layout: &Layout, against: &[Item], candidate: &Item) -> Vec<String> {
    let mut problems = Vec::new();
    match layout.zone_named(&candidate.zone) {
        Some(zone) if !candidate.footprint.inside(&zone.bounds) => problems.push(format!("outside zone {}", candidate.zone)),
        None => problems.push(format!("unknown zone {}", candidate.zone)),
        _ => {}
    }
    if !candidate.footprint.inside(&layout.bounds) {
        problems.push("outside arena bounds".to_string());
    }
    for other in against {
        if candidate.is_attached_to(other) {
            continue;
        }
        let height = candidate.footprint.height_overlap(&other.footprint);
        if height <= OVERLAP_TOLERANCE {
            continue;
        }
        let gap = candidate.footprint.separation(&other.footprint);
        if gap < -OVERLAP_TOLERANCE {
            problems.push(format!("overlaps {} by {:.2} m", other.name, -gap));
        } else if gap > FLUSH_TOLERANCE && gap < WALK_CLEARANCE && narrow_gap_matters(candidate, other) {
            problems.push(format!("{:.2} m sliver gap to {}: too narrow to walk, too wide to be flush", gap, other.name));
        }
    }
    problems
}

fn narrow_gap_matters(a: &Item, b: &Item) -> bool {
    a.kind.blocks_ground(&a.footprint) && b.kind.blocks_ground(&b.footprint)
}

fn violates_request(layout: &Layout, request: &Request, candidate: &Item, placed: &[usize]) -> Vec<String> {
    let mut problems = placement_problems(layout, &layout.items, candidate);
    for index in placed {
        let other = &layout.items[*index];
        let gap = candidate.footprint.separation(&other.footprint);
        if gap < request.min_spacing {
            problems.push(format!("{:.2} m from {}, spacing needs {:.2}", gap, other.name, request.min_spacing));
        }
    }
    for ((x, z), radius) in request.keep_out.iter().chain(layout.keep_clear.iter()) {
        if candidate.footprint.distance_to_point(*x, *z) < *radius {
            problems.push(format!("inside keep-clear area at ({:.1}, {:.1})", x, z));
        }
    }
    problems
}

pub fn solve(layout: &mut Layout, requests: &[Request], seed: u64) -> Vec<Finding> {
    let mut failures = Vec::new();
    for request in requests {
        let Some(zone) = layout.zone_named(&request.zone).cloned() else {
            failures.push(unsatisfied(&request.name, "unknown zone"));
            continue;
        };
        let mut placed: Vec<usize> = Vec::new();
        for number in 0..request.count {
            let name = if request.count == 1 { request.name.clone() } else { format!("{}_{}", request.name, number + 1) };
            let mut random = Random::new(seed ^ fnv(&name));
            let mut best: Option<(Item, Vec<String>)> = None;
            for _ in 0..CANDIDATES_PER_ITEM {
                let yaw = request.yaw_choices[(random.next_u32() as usize) % request.yaw_choices.len()];
                let x = random.range(zone.bounds.min_x, zone.bounds.max_x);
                let z = random.range(zone.bounds.min_z, zone.bounds.max_z);
                let footprint = Footprint::new(x, z, request.size.0, request.size.1, yaw, request.base_height, request.base_height + request.size.2);
                let candidate = Item::new(&name, request.kind, &request.reason, &request.zone, footprint);
                let problems = violates_request(layout, request, &candidate, &placed);
                let better = best.as_ref().map(|(_, p)| problems.len() < p.len()).unwrap_or(true);
                if better {
                    let done = problems.is_empty();
                    best = Some((candidate, problems));
                    if done {
                        break;
                    }
                }
            }
            let (item, problems) = best.expect("at least one candidate");
            if problems.is_empty() {
                layout.items.push(item);
                placed.push(layout.items.len() - 1);
            } else {
                failures.push(unsatisfied(&name, &problems.join("; ")));
            }
        }
    }
    failures
}

fn unsatisfied(name: &str, reason: &str) -> Finding {
    Finding::new(Severity::Error, name, "unplaceable", 0.0, "a valid position".to_string(), &format!("no position satisfies constraints: {}", reason))
}

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325, |hash, byte| (hash ^ byte as u64).wrapping_mul(0x100000001b3))
}
