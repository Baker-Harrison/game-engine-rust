use crate::math::Vec3;
use crate::font::{font, CAP_HEIGHT};
use crate::scene::{OverlayItem, OverlayRect, OverlayTri};

pub const CANVAS_WIDTH: f32 = 1280.0;
pub const CANVAS_HEIGHT: f32 = 720.0;
const BASE_CAP_HEIGHT: f32 = 7.0;
const STROKE_WEIGHT: f32 = 1.55;
const MITER_LIMIT: f32 = 2.2;
const SHADOW_ALPHA: f32 = 0.5;

pub const WHITE: Vec3 = Vec3::new(1.0, 1.0, 1.0);
pub const BLACK: Vec3 = Vec3::new(0.0, 0.0, 0.0);
pub const RED: Vec3 = Vec3::new(0.95, 0.2, 0.15);
pub const GREEN: Vec3 = Vec3::new(0.3, 0.9, 0.35);
pub const YELLOW: Vec3 = Vec3::new(1.0, 0.85, 0.2);
pub const ORANGE: Vec3 = Vec3::new(1.0, 0.55, 0.12);

#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct Ui {
    items: Vec<OverlayItem>,
    pixels_per_unit: f32,
}

impl Ui {
    pub fn new() -> Ui {
        Ui { items: Vec::new(), pixels_per_unit: 1.0 }
    }

    pub fn set_target(&mut self, width_pixels: usize, height_pixels: usize) {
        let across = width_pixels as f32 / CANVAS_WIDTH;
        let down = height_pixels as f32 / CANVAS_HEIGHT;
        self.pixels_per_unit = ((across + down) * 0.5).max(0.1);
    }

    pub fn pixels_per_unit(&self) -> f32 {
        self.pixels_per_unit
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn rect_count(&self) -> usize {
        self.items.len()
    }

    pub fn finish(&mut self) -> Vec<OverlayItem> {
        std::mem::take(&mut self.items)
    }

    pub fn items(&self) -> &[OverlayItem] {
        &self.items
    }

    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Vec3, alpha: f32) {
        if width <= 0.0 || height <= 0.0 || alpha <= 0.0 {
            return;
        }
        self.items.push(OverlayItem::Rect(OverlayRect {
            min: (x / CANVAS_WIDTH, y / CANVAS_HEIGHT),
            max: ((x + width) / CANVAS_WIDTH, (y + height) / CANVAS_HEIGHT),
            color,
            alpha,
        }));
    }

    pub fn outline(&mut self, x: f32, y: f32, width: f32, height: f32, thickness: f32, color: Vec3, alpha: f32) {
        self.rect(x, y, width, thickness, color, alpha);
        self.rect(x, y + height - thickness, width, thickness, color, alpha);
        self.rect(x, y + thickness, thickness, height - 2.0 * thickness, color, alpha);
        self.rect(x + width - thickness, y + thickness, thickness, height - 2.0 * thickness, color, alpha);
    }

    pub fn bar(&mut self, x: f32, y: f32, width: f32, height: f32, fraction: f32, fill: Vec3) {
        self.rect(x - 2.0, y - 2.0, width + 4.0, height + 4.0, BLACK, 0.55);
        self.rect(x, y, width * fraction.clamp(0.0, 1.0), height, fill, 1.0);
    }

    pub fn text_width(text: &str, scale: f32) -> f32 {
        font().width(text) * Ui::font_unit(scale)
    }

    fn font_unit(scale: f32) -> f32 {
        BASE_CAP_HEIGHT * scale / CAP_HEIGHT
    }

    pub fn text(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Vec3, align: Align) {
        let width = Ui::text_width(text, scale);
        let left = match align {
            Align::Left => x,
            Align::Center => x - width * 0.5,
            Align::Right => x - width,
        };
        let shadow = scale * 0.7;
        self.glyphs(left + shadow, y + shadow, text, scale, BLACK, SHADOW_ALPHA);
        self.glyphs(left, y, text, scale, color, 1.0);
    }

    fn glyphs(&mut self, left: f32, top: f32, text: &str, scale: f32, color: Vec3, alpha: f32) {
        let unit = Ui::font_unit(scale);
        let half_width = STROKE_WEIGHT * unit * 0.5;
        let mut cursor = left;
        for character in text.chars() {
            let glyph = font().glyph(character);
            for stroke in &glyph.strokes {
                let points: Vec<(f32, f32)> = stroke.iter().map(|p| (cursor + p.0 * unit, top + p.1 * unit)).collect();
                self.stroke(&points, half_width, color, alpha);
            }
            cursor += glyph.advance * unit;
        }
    }

    pub fn stroke(&mut self, points: &[(f32, f32)], half_width: f32, color: Vec3, alpha: f32) {
        if points.len() < 2 || alpha <= 0.0 {
            return;
        }
        let fringe = 1.0 / self.pixels_per_unit;
        let inner = (half_width - fringe * 0.5).max(0.0);
        let outer = half_width + fringe * 0.5;
        let peak = alpha * (half_width * 2.0 / fringe).min(1.0);
        let closed = (points[0].0 - points[points.len() - 1].0).abs() < 1e-3 && (points[0].1 - points[points.len() - 1].1).abs() < 1e-3;
        let count = points.len();
        let direction = |a: (f32, f32), b: (f32, f32)| {
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length = (dx * dx + dy * dy).sqrt().max(1e-6);
            (dx / length, dy / length)
        };
        let mut sections: Vec<[((f32, f32), f32); 4]> = Vec::with_capacity(count);
        for index in 0..count {
            let mut point = points[index];
            let before = if index > 0 { Some(direction(points[index - 1], point)) } else if closed { Some(direction(points[count - 2], point)) } else { None };
            let after = if index + 1 < count { Some(direction(point, points[index + 1])) } else if closed { Some(direction(point, points[1])) } else { None };
            let tangent = match (before, after) {
                (Some(a), Some(b)) => {
                    let sum = (a.0 + b.0, a.1 + b.1);
                    let length = (sum.0 * sum.0 + sum.1 * sum.1).sqrt();
                    if length < 1e-4 { b } else { (sum.0 / length, sum.1 / length) }
                }
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => continue,
            };
            if !closed && (before.is_none() || after.is_none()) {
                let sign = if before.is_none() { -1.0 } else { 1.0 };
                point = (point.0 + tangent.0 * sign * half_width * 0.8, point.1 + tangent.1 * sign * half_width * 0.8);
            }
            let normal = (-tangent.1, tangent.0);
            let reference = before.or(after).map(|d| (-d.1, d.0)).unwrap_or(normal);
            let cosine = (normal.0 * reference.0 + normal.1 * reference.1).max(1.0 / MITER_LIMIT);
            let scale = 1.0 / cosine;
            let at = |distance: f32| (point.0 + normal.0 * distance * scale, point.1 + normal.1 * distance * scale);
            sections.push([(at(-outer), 0.0), (at(-inner), peak), (at(inner), peak), (at(outer), 0.0)]);
        }
        for pair in sections.windows(2) {
            for band in 0..3 {
                let (a0, a1) = (pair[0][band], pair[0][band + 1]);
                let (b0, b1) = (pair[1][band], pair[1][band + 1]);
                self.push_tri([a0, a1, b1], color);
                self.push_tri([a0, b1, b0], color);
            }
        }
    }

    fn push_tri(&mut self, corners: [((f32, f32), f32); 3], color: Vec3) {
        let to_unit = |p: (f32, f32)| (p.0 / CANVAS_WIDTH, p.1 / CANVAS_HEIGHT);
        self.items.push(OverlayItem::Tri(OverlayTri {
            points: [to_unit(corners[0].0), to_unit(corners[1].0), to_unit(corners[2].0)],
            alpha: [corners[0].1, corners[1].1, corners[2].1],
            color,
        }));
    }

    pub fn crosshair(&mut self, gap: f32, length: f32, thickness: f32, color: Vec3) {
        let (cx, cy) = (CANVAS_WIDTH * 0.5, CANVAS_HEIGHT * 0.5);
        self.rect(cx - gap - length, cy - thickness * 0.5, length, thickness, color, 0.95);
        self.rect(cx + gap, cy - thickness * 0.5, length, thickness, color, 0.95);
        self.rect(cx - thickness * 0.5, cy - gap - length, thickness, length, color, 0.95);
        self.rect(cx - thickness * 0.5, cy + gap, thickness, length, color, 0.95);
    }

    pub fn hit_marker(&mut self, color: Vec3, alpha: f32) {
        let (cx, cy) = (CANVAS_WIDTH * 0.5, CANVAS_HEIGHT * 0.5);
        for step in 0..4 {
            let offset = 6.0 + step as f32 * 2.5;
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                self.rect(cx + sx * offset - 1.5, cy + sy * offset - 1.5, 3.0, 3.0, color, alpha);
            }
        }
    }

    pub fn vignette(&mut self, color: Vec3, strength: f32) {
        if strength <= 0.0 {
            return;
        }
        let band = 70.0;
        for layer in 0..5 {
            let inset = layer as f32 * band / 5.0;
            let alpha = strength * (1.0 - layer as f32 / 5.0) * 0.35;
            let thickness = band / 5.0;
            self.rect(0.0, inset, CANVAS_WIDTH, thickness, color, alpha);
            self.rect(0.0, CANVAS_HEIGHT - inset - thickness, CANVAS_WIDTH, thickness, color, alpha);
            self.rect(0.0, inset, thickness, CANVAS_HEIGHT - 2.0 * inset, color, alpha);
            self.rect(CANVAS_WIDTH - thickness - inset, inset, thickness, CANVAS_HEIGHT - 2.0 * inset, color, alpha);
        }
    }

    pub fn fade(&mut self, color: Vec3, alpha: f32) {
        self.rect(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT, color, alpha);
    }
}
