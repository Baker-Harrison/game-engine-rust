use crate::render::Framebuffer;

const GLYPH_WIDTH: usize = 5;
const GLYPH_HEIGHT: usize = 7;

pub fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

pub fn set_pixel(framebuffer: &mut Framebuffer, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && (x as usize) < framebuffer.width && (y as usize) < framebuffer.height {
        framebuffer.pixels[y as usize * framebuffer.width + x as usize] = color;
    }
}

pub fn draw_line(framebuffer: &mut Framebuffer, from: (f32, f32), to: (f32, f32), color: u32) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as usize;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        set_pixel(framebuffer, (from.0 + dx * t).round() as i32, (from.1 + dy * t).round() as i32, color);
    }
}

pub fn draw_rect(framebuffer: &mut Framebuffer, min: (f32, f32), max: (f32, f32), color: u32) {
    draw_line(framebuffer, (min.0, min.1), (max.0, min.1), color);
    draw_line(framebuffer, (max.0, min.1), (max.0, max.1), color);
    draw_line(framebuffer, (max.0, max.1), (min.0, max.1), color);
    draw_line(framebuffer, (min.0, max.1), (min.0, min.1), color);
}

pub fn fill_rect(framebuffer: &mut Framebuffer, x: i32, y: i32, width: i32, height: i32, color: u32) {
    for row in y..y + height {
        for column in x..x + width {
            set_pixel(framebuffer, column, row, color);
        }
    }
}

pub fn text_width(text: &str, scale: usize) -> usize {
    text.chars().count() * (GLYPH_WIDTH + 1) * scale
}

pub fn draw_text(framebuffer: &mut Framebuffer, x: i32, y: i32, text: &str, color: u32, scale: usize) {
    let backdrop = pack_rgb(20, 20, 24);
    fill_rect(framebuffer, x - 2, y - 2, text_width(text, scale) as i32 + 3, (GLYPH_HEIGHT * scale) as i32 + 4, backdrop);
    for (index, character) in text.chars().enumerate() {
        let origin_x = x + (index * (GLYPH_WIDTH + 1) * scale) as i32;
        for (row, bits) in glyph(character).iter().enumerate() {
            for column in 0..GLYPH_WIDTH {
                if bits & (1 << (GLYPH_WIDTH - 1 - column)) != 0 {
                    fill_rect(framebuffer, origin_x + (column * scale) as i32, y + (row * scale) as i32, scale as i32, scale as i32, color);
                }
            }
        }
    }
}

pub fn blit(destination: &mut Framebuffer, source: &Framebuffer, x: usize, y: usize) {
    for row in 0..source.height {
        let from = row * source.width;
        let to = (y + row) * destination.width + x;
        destination.pixels[to..to + source.width].copy_from_slice(&source.pixels[from..from + source.width]);
    }
}

fn glyph(character: char) -> [u8; GLYPH_HEIGHT] {
    match character.to_ascii_lowercase() {
        'a' => [0, 0, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111],
        'b' => [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110],
        'c' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110],
        'd' => [0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10001, 0b01111],
        'e' => [0, 0, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110],
        'f' => [0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000],
        'g' => [0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'h' => [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001],
        'i' => [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110],
        'j' => [0b00010, 0, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100],
        'k' => [0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010],
        'l' => [0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'm' => [0, 0, 0b11010, 0b10101, 0b10101, 0b10001, 0b10001],
        'n' => [0, 0, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001],
        'o' => [0, 0, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
        'p' => [0, 0, 0b11110, 0b10001, 0b11110, 0b10000, 0b10000],
        'q' => [0, 0, 0b01101, 0b10011, 0b01111, 0b00001, 0b00001],
        'r' => [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000],
        's' => [0, 0, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        't' => [0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110],
        'u' => [0, 0, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101],
        'v' => [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'w' => [0, 0, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010],
        'x' => [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001],
        'y' => [0, 0b10001, 0b10001, 0b01111, 0b00001, 0b10001, 0b01110],
        'z' => [0, 0, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111],
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
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        ' ' => [0; GLYPH_HEIGHT],
        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}
