use crate::math::Vec3;
use crate::scene::OverlayRect;

pub const CANVAS_WIDTH: f32 = 1280.0;
pub const CANVAS_HEIGHT: f32 = 720.0;
const GLYPH_WIDTH: usize = 5;
const GLYPH_HEIGHT: usize = 7;

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

#[derive(Default)]
pub struct Ui {
    rects: Vec<OverlayRect>,
}

impl Ui {
    pub fn new() -> Ui {
        Ui::default()
    }

    pub fn clear(&mut self) {
        self.rects.clear();
    }

    pub fn rect_count(&self) -> usize {
        self.rects.len()
    }

    pub fn finish(&mut self) -> Vec<OverlayRect> {
        std::mem::take(&mut self.rects)
    }

    pub fn rects(&self) -> &[OverlayRect] {
        &self.rects
    }

    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Vec3, alpha: f32) {
        if width <= 0.0 || height <= 0.0 || alpha <= 0.0 {
            return;
        }
        self.rects.push(OverlayRect {
            min: (x / CANVAS_WIDTH, y / CANVAS_HEIGHT),
            max: ((x + width) / CANVAS_WIDTH, (y + height) / CANVAS_HEIGHT),
            color,
            alpha,
        });
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
        text.chars().count() as f32 * (GLYPH_WIDTH + 1) as f32 * scale
    }

    pub fn text(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Vec3, align: Align) {
        let width = Ui::text_width(text, scale);
        let left = match align {
            Align::Left => x,
            Align::Center => x - width * 0.5,
            Align::Right => x - width,
        };
        self.glyphs(left + scale, y + scale, text, scale, BLACK, 0.6);
        self.glyphs(left, y, text, scale, color, 1.0);
    }

    fn glyphs(&mut self, left: f32, top: f32, text: &str, scale: f32, color: Vec3, alpha: f32) {
        for (index, character) in text.chars().enumerate() {
            let origin = left + index as f32 * (GLYPH_WIDTH + 1) as f32 * scale;
            for (row, bits) in glyph(character).iter().enumerate() {
                let mut column = 0;
                while column < GLYPH_WIDTH {
                    if bits & (1 << (GLYPH_WIDTH - 1 - column)) == 0 {
                        column += 1;
                        continue;
                    }
                    let start = column;
                    while column < GLYPH_WIDTH && bits & (1 << (GLYPH_WIDTH - 1 - column)) != 0 {
                        column += 1;
                    }
                    self.rect(origin + start as f32 * scale, top + row as f32 * scale, (column - start) as f32 * scale, scale, color, alpha);
                }
            }
        }
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

fn glyph(character: char) -> [u8; GLYPH_HEIGHT] {
    match character.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ',' => [0, 0, 0, 0, 0b01100, 0b00100, 0b01000],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        ';' => [0, 0b01100, 0b01100, 0, 0b01100, 0b00100, 0b01000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0, 0b00100],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        '=' => [0, 0, 0b11111, 0, 0b11111, 0, 0],
        '*' => [0, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0],
        '\'' => [0b00100, 0b00100, 0b01000, 0, 0, 0, 0],
        '"' => [0b01010, 0b01010, 0, 0, 0, 0, 0],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        '#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        ' ' => [0; GLYPH_HEIGHT],
        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}
