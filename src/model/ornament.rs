use super::sdf::Sdf;
use crate::math::Vec3;
use std::f32::consts::{PI, TAU};

pub struct Ornament {
    pub sdf: Sdf,
    pub smallest_feature: f32,
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn smooth_profile(controls: &[[f32; 2]], samples_per_span: usize) -> Vec<[f32; 2]> {
    let last = controls.len() - 1;
    let at = |i: isize| controls[i.clamp(0, last as isize) as usize];
    let mut points = Vec::new();
    for span in 0..last {
        for step in 0..samples_per_span {
            let t = step as f32 / samples_per_span as f32;
            let (p0, p1, p2, p3) = (at(span as isize - 1), at(span as isize), at(span as isize + 1), at(span as isize + 2));
            let blend = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t + (3.0 * b - a - 3.0 * c + d) * t * t * t)
            };
            points.push([blend(p0[0], p1[0], p2[0], p3[0]).max(0.0), blend(p0[1], p1[1], p2[1], p3[1])]);
        }
    }
    points.push(controls[last]);
    points
}

pub fn lathe(controls: &[[f32; 2]]) -> Sdf {
    let mut outline = smooth_profile(controls, 8);
    let top = outline.last().map(|p| p[1]).unwrap_or(0.0);
    let bottom = outline.first().map(|p| p[1]).unwrap_or(0.0);
    outline.push([0.0, top]);
    outline.push([0.0, bottom]);
    Sdf::revolve(&outline)
}

pub fn baluster(height: f32, radius: f32) -> Ornament {
    let h = height;
    let r = radius;
    let controls = [
        [r * 0.80, 0.0],
        [r * 0.80, h * 0.05],
        [r * 0.55, h * 0.09],
        [r * 0.45, h * 0.16],
        [r * 0.95, h * 0.30],
        [r * 1.00, h * 0.38],
        [r * 0.60, h * 0.55],
        [r * 0.38, h * 0.70],
        [r * 0.55, h * 0.82],
        [r * 0.62, h * 0.88],
        [r * 0.80, h * 0.92],
        [r * 0.80, h],
    ];
    Ornament { sdf: lathe(&controls), smallest_feature: r * 0.8 }
}

pub fn finial(height: f32, radius: f32) -> Ornament {
    let (h, r) = (height, radius);
    let controls = [[r * 0.9, 0.0], [r * 0.9, h * 0.10], [r * 0.5, h * 0.18], [r * 0.4, h * 0.28], [r * 1.0, h * 0.50], [r * 0.85, h * 0.72], [r * 0.4, h * 0.90], [0.02 * r, h]];
    Ornament { sdf: lathe(&controls), smallest_feature: r * 0.3 }
}

pub struct BalustradeSpec {
    pub length: f32,
    pub height: f32,
    pub baluster_radius: f32,
    pub baluster_spacing: f32,
    pub rail_width: f32,
    pub post_width: f32,
}

impl BalustradeSpec {
    pub fn terrace(length: f32) -> BalustradeSpec {
        BalustradeSpec { length, height: 0.95, baluster_radius: 0.035, baluster_spacing: 0.15, rail_width: 0.18, post_width: 0.3 }
    }
}

pub fn balustrade(spec: &BalustradeSpec) -> Ornament {
    let plinth_height = 0.12;
    let top_height = 0.1;
    let shaft_height = spec.height - plinth_height - top_height;
    let usable = spec.length - spec.post_width;
    let count = ((usable / spec.baluster_spacing).floor() as usize).max(1);
    let step = usable / count as f32;
    let single = baluster(shaft_height, spec.baluster_radius).sdf;
    let balusters = single.linear_array_x(count, step).translate(v(spec.post_width * 0.5 + step * 0.5, plinth_height, 0.0));
    let base_rail = Sdf::round_box(v(spec.length, plinth_height, spec.rail_width), 0.02).translate(v(spec.length * 0.5, plinth_height * 0.5, 0.0));
    let coping_profile = [[-0.5, 0.0], [0.5, 0.0], [0.5, 0.035], [0.42, 0.06], [0.30, 0.1], [-0.30, 0.1], [-0.42, 0.06], [-0.5, 0.035]];
    let coping = Sdf::extrude(&coping_profile.map(|p| [p[0] * spec.rail_width * 1.25, p[1]]), spec.length, 0.012)
        .rotate_y(90.0)
        .translate(v(spec.length * 0.5, spec.height - top_height, 0.0));
    let mut parts = vec![base_rail, coping, balusters];
    for x in [spec.post_width * 0.5, spec.length - spec.post_width * 0.5] {
        let post = Sdf::round_box(v(spec.post_width, spec.height - plinth_height, spec.post_width), 0.025)
            .translate(v(x, plinth_height + (spec.height - plinth_height) * 0.5, 0.0));
        let cap = Sdf::round_box(v(spec.post_width * 1.2, 0.07, spec.post_width * 1.2), 0.02).translate(v(x, spec.height + 0.035, 0.0));
        let ball = finial(0.28, spec.post_width * 0.42).sdf.translate(v(x, spec.height + 0.07, 0.0));
        parts.extend([post, cap, ball]);
    }
    Ornament { sdf: Sdf::smooth_union_all(parts, 0.008), smallest_feature: spec.baluster_radius * 0.8 }
}

pub fn column(height: f32, radius: f32, flutes: usize) -> Ornament {
    let base_h = radius * 0.7;
    let cap_h = radius * 1.1;
    let shaft_h = height - base_h - cap_h;
    let shaft_controls = [[radius * 0.95, base_h], [radius, base_h + shaft_h * 0.35], [radius * 0.92, base_h + shaft_h * 0.8], [radius * 0.82, base_h + shaft_h]];
    let shaft = lathe(&shaft_controls);
    let groove = Sdf::cylinder(radius * 0.09, shaft_h * 0.86, 0.0).translate(v(radius * 0.97, base_h + shaft_h * 0.5, 0.0));
    let fluted = shaft.subtract(groove.radial_array(flutes.max(4)));
    let base_controls = [[radius * 1.35, 0.0], [radius * 1.35, base_h * 0.3], [radius * 1.15, base_h * 0.55], [radius * 1.2, base_h * 0.8], [radius * 0.95, base_h]];
    let cap_controls = [
        [radius * 0.82, base_h + shaft_h],
        [radius * 0.98, base_h + shaft_h + cap_h * 0.15],
        [radius * 1.1, base_h + shaft_h + cap_h * 0.45],
        [radius * 1.5, base_h + shaft_h + cap_h * 0.8],
        [radius * 1.5, base_h + shaft_h + cap_h],
    ];
    let parts = vec![lathe(&base_controls), fluted, lathe(&cap_controls)];
    Ornament { sdf: Sdf::smooth_union_all(parts, 0.006), smallest_feature: radius * 0.2 }
}

pub fn arch_opening_wall(width: f32, height: f32, thickness: f32, opening_width: f32, opening_height: f32) -> Ornament {
    let wall = Sdf::round_box(v(width, height, thickness), 0.02).translate(v(0.0, height * 0.5, 0.0));
    let radius = opening_width * 0.5;
    let straight = opening_height - radius;
    let door = Sdf::round_box(v(opening_width, straight, thickness * 2.0), 0.0).translate(v(0.0, straight * 0.5, 0.0));
    let round = Sdf::cylinder(radius, thickness, 0.0).rotate_x(90.0).translate(v(0.0, straight, 0.0));
    let opening = door.union(round);
    let surround = Sdf::extrude(&arch_surround_profile(radius), thickness * 1.2, 0.01).translate(v(0.0, straight, 0.0));
    let carved = wall.subtract(opening.clone());
    Ornament { sdf: carved.union(surround.subtract(opening)), smallest_feature: 0.06 }
}

fn arch_surround_profile(radius: f32) -> Vec<[f32; 2]> {
    let inner = radius;
    let outer = radius + 0.14;
    let mut points = Vec::new();
    for i in 0..=16 {
        let a = PI * i as f32 / 16.0;
        points.push([outer * a.cos(), outer * a.sin()]);
    }
    for i in (0..=16).rev() {
        let a = PI * i as f32 / 16.0;
        points.push([inner * a.cos(), inner * a.sin()]);
    }
    points
}

pub fn cornice(length: f32, projection: f32, height: f32) -> Ornament {
    let (p, h) = (projection, height);
    let profile = [[0.0, 0.0], [p * 0.15, 0.0], [p * 0.15, h * 0.18], [p * 0.45, h * 0.22], [p * 0.45, h * 0.45], [p * 0.25, h * 0.6], [p * 0.7, h * 0.72], [p, h * 0.82], [p, h], [0.0, h]];
    let band = Sdf::extrude(&profile, length, 0.006).rotate_y(90.0).translate(v(length * 0.5, 0.0, 0.0));
    let dentil_step = h * 0.28;
    let count = (length / dentil_step) as usize;
    let dentil = Sdf::round_box(v(dentil_step * 0.5, h * 0.18, p * 0.2), 0.004).linear_array_x(count, dentil_step).translate(v(dentil_step * 0.5, h * 0.08, p * 0.28));
    Ornament { sdf: band.union(dentil), smallest_feature: h * 0.12 }
}

pub fn iron_scroll(radius: f32, turns: f32, bar: f32) -> Ornament {
    let steps = 14;
    let path: Vec<Vec3> = (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let r = radius * (1.0 - 0.85 * t);
            let a = t * turns * TAU;
            v(r * a.cos(), r * a.sin(), 0.0)
        })
        .collect();
    let sizes = vec![[bar, bar]; path.len()];
    let tail = Sdf::limb(v(radius, 0.0, 0.0), v(radius, -radius * 1.4, 0.0), [bar, bar], [bar, bar], bar * 0.5);
    Ornament { sdf: Sdf::sweep(&path, &sizes, bar * 0.4, 6, v(0.0, 0.0, 1.0)).union(tail), smallest_feature: bar * 0.8 }
}

pub fn iron_railing(length: f32, height: f32, bar_radius: f32, bar_spacing: f32) -> Ornament {
    let rail = |y: f32| Sdf::round_box(v(length, bar_radius * 2.4, bar_radius * 2.4), bar_radius * 0.8).translate(v(length * 0.5, y, 0.0));
    let count = ((length / bar_spacing).floor() as usize).max(1);
    let step = length / count as f32;
    let bar = Sdf::cylinder(bar_radius, (height - bar_radius * 4.0) * 0.5, bar_radius * 0.4).linear_array_x(count, step).translate(v(step * 0.5, height * 0.5, 0.0));
    let spike = Sdf::sphere(bar_radius * 1.6).linear_array_x(count, step).translate(v(step * 0.5, height - bar_radius * 1.5, 0.0));
    let scroll_radius = step * 0.42;
    let scroll = iron_scroll(scroll_radius, 1.3, bar_radius * 0.8).sdf.linear_array_x(count, step).translate(v(step * 0.5, height * 0.45, 0.0));
    let parts = vec![rail(bar_radius * 1.2), rail(height - bar_radius * 1.2), bar, spike, scroll];
    Ornament { sdf: Sdf::smooth_union_all(parts, bar_radius * 0.6), smallest_feature: bar_radius * 1.2 }
}
