use engine::layout::footprint::{Footprint, Rect};
use engine::layout::{Item, ItemKind, Layout, Purpose, Request};
use engine::probe::Severity;

fn yard() -> Layout {
    let mut layout = Layout::new("yard", Rect::new(-30.0, -20.0, 30.0, 20.0));
    layout.zone("west", Purpose::Spawn, Rect::new(-28.0, -8.0, -18.0, 8.0));
    layout.zone("east", Purpose::Objective, Rect::new(18.0, -8.0, 28.0, 8.0));
    layout.zone("middle", Purpose::Cover, Rect::new(-14.0, -16.0, 14.0, 16.0));
    layout.link("west", "east", 1.2, 2);
    layout
}

fn errors(layout: &Layout) -> Vec<String> {
    layout.check().into_iter().filter(|f| f.severity == Severity::Error).map(|f| format!("{} {} {}", f.subject, f.metric, f.hint)).collect()
}

#[test]
fn solved_cover_is_valid_and_deterministic() {
    let requests = [Request::new("crate", ItemKind::Cover, "cover for the middle lane", "middle", (2.0, 1.2, 1.2)).count(8).spaced(2.5)];
    let mut first = yard();
    assert!(first.solve(&requests, 7).is_empty());
    let mut second = yard();
    second.solve(&requests, 7);
    let positions = |l: &Layout| l.items.iter().map(|i| (i.footprint.x, i.footprint.z)).collect::<Vec<_>>();
    assert_eq!(positions(&first), positions(&second));
    assert!(errors(&first).is_empty(), "{:?}", errors(&first));
}

#[test]
fn overlap_sliver_and_purposeless_placement_are_reported() {
    let mut layout = yard();
    layout.place(Item::new("tank", ItemKind::Solid, "landmark in the yard centre", "middle", Footprint::new(0.0, 0.0, 6.0, 6.0, 0.0, 0.0, 6.0)));
    layout.place(Item::new("shed", ItemKind::Solid, "storage for the yard", "middle", Footprint::new(2.0, 0.0, 6.0, 6.0, 0.0, 0.0, 3.0)));
    layout.place(Item::new("crate", ItemKind::Cover, "cover for the lane", "middle", Footprint::new(-4.5, 0.0, 2.0, 2.0, 0.0, 0.0, 1.2)));
    layout.place(Item::new("junk", ItemKind::Cover, "", "middle", Footprint::new(10.0, 10.0, 2.0, 2.0, 0.0, 0.0, 1.0)));
    let found = errors(&layout).join("\n");
    assert!(found.contains("shed placement") && found.contains("overlaps tank"), "{}", found);
    assert!(found.contains("crate placement") && found.contains("sliver gap"), "{}", found);
    assert!(found.contains("junk placement_reason_length"), "{}", found);
}

#[test]
fn unsupported_deck_is_reported_and_supported_deck_passes() {
    let mut layout = yard();
    layout.place(Item::new("deck", ItemKind::Deck, "vantage over the middle", "middle", Footprint::new(0.0, 0.0, 8.0, 4.0, 0.0, 4.0, 4.3)));
    assert!(errors(&layout).iter().any(|e| e.contains("deck support_count")));
    for (name, x) in [("post_a", -3.0), ("post_b", 3.0)] {
        layout.place(Item::new(name, ItemKind::Support, "holds up the vantage deck", "middle", Footprint::new(x, 0.0, 0.4, 0.4, 0.0, 0.0, 4.0)).attached("deck"));
    }
    assert!(!errors(&layout).iter().any(|e| e.contains("support_count")), "{:?}", errors(&layout));
}

#[test]
fn walled_off_route_and_single_route_are_reported() {
    let mut layout = yard();
    layout.place(Item::new("wall", ItemKind::Solid, "blocks the whole lane", "middle", Footprint::new(0.0, 0.0, 1.0, 40.0, 0.0, 0.0, 4.0)));
    layout.bounds = Rect::new(-30.0, -20.0, 30.0, 20.0);
    assert!(errors(&layout).iter().any(|e| e.contains("west -> east disjoint_routes")));

    let mut open = yard();
    open.place(Item::new("wall", ItemKind::Solid, "forces a detour in the lane", "middle", Footprint::new(0.0, -1.0, 1.0, 38.0, 0.0, 0.0, 4.0)));
    assert!(errors(&open).iter().any(|e| e.contains("disjoint_routes")), "single gap gives one route");
}

#[test]
fn two_open_lanes_pass_the_route_requirement() {
    let mut layout = yard();
    layout.place(Item::new("block", ItemKind::Solid, "central landmark between lanes", "middle", Footprint::new(0.0, 0.0, 6.0, 6.0, 0.0, 0.0, 5.0)));
    assert!(!errors(&layout).iter().any(|e| e.contains("disjoint_routes")), "{:?}", errors(&layout));
}

#[test]
fn unplaceable_request_names_the_reason() {
    let mut layout = yard();
    layout.place(Item::new("slab", ItemKind::Solid, "fills the middle zone", "middle", Footprint::new(0.0, 0.0, 27.9, 31.9, 0.0, 0.0, 5.0)));
    let requests = [Request::new("crate", ItemKind::Cover, "cover for the middle lane", "middle", (2.0, 1.2, 1.2))];
    let failures = layout.solve(&requests, 1);
    assert_eq!(failures.len(), 1);
    assert!(failures[0].hint.contains("overlaps slab") || failures[0].hint.contains("sliver"));
}
