use std::collections::HashMap;
use std::sync::OnceLock;

pub const CAP_HEIGHT: f32 = 10.0;
const SPACE_ADVANCE: f32 = 5.0;
const GLYPH_GAP: f32 = 3.2;
const CURVE_STEPS: usize = 7;

pub struct Glyph {
    pub strokes: Vec<Vec<(f32, f32)>>,
    pub advance: f32,
}

pub struct Font {
    glyphs: HashMap<char, Glyph>,
    fallback: Glyph,
}

const OUTLINES: &[(char, &str)] = &[
    ('A', "M0,10 L3,0 L6,10 M1.2,6.6 L4.8,6.6"),
    ('B', "M0,0 L0,10 M0,0 L3.4,0 A3.4,2.5,2.5,2.5,-90,90 L0,5 M0,5 L3.8,5 A3.8,7.5,2.5,2.5,-90,90 L0,10"),
    ('C', "A3.4,5,3.2,5,-40,-320"),
    ('D', "M0,0 L0,10 M0,0 L2.4,0 A2.4,5,3.6,5,-90,90 L0,10"),
    ('E', "M6,0 L0,0 L0,10 L6,10 M0,5 L4.6,5"),
    ('F', "M6,0 L0,0 L0,10 M0,5 L4.6,5"),
    ('G', "A3.4,5,3.2,5,-40,-360 L3.9,5"),
    ('H', "M0,0 L0,10 M6,0 L6,10 M0,5 L6,5"),
    ('I', "M3,0 L3,10 M1.2,0 L4.8,0 M1.2,10 L4.8,10"),
    ('J', "M5,0 L5,7 A2.5,7,2.5,3,0,180"),
    ('K', "M0,0 L0,10 M5.8,0 L0,6 M2,4.2 L6,10"),
    ('L', "M0,0 L0,10 L5.5,10"),
    ('M', "M0,10 L0,0 L4,7 L8,0 L8,10"),
    ('N', "M0,10 L0,0 L6,10 L6,0"),
    ('O', "A3,5,3,5,0,360"),
    ('P', "M0,10 L0,0 L3.4,0 A3.4,2.7,2.6,2.7,-90,90 L0,5.4"),
    ('Q', "A3,5,3,5,0,360 M3.6,7 L6.6,10.6"),
    ('R', "M0,10 L0,0 L3.4,0 A3.4,2.7,2.6,2.7,-90,90 L0,5.4 M2.8,5.4 L6,10"),
    ('S', "A3,2.6,2.8,2.6,-35,-270 A3,7.4,3,2.6,-90,145"),
    ('T', "M0,0 L6,0 M3,0 L3,10"),
    ('U', "M0,0 L0,6.5 A3,6.5,3,3.5,180,0 L6,0"),
    ('V', "M0,0 L3,10 L6,0"),
    ('W', "M0,0 L2.2,10 L4.5,2.5 L6.8,10 L9,0"),
    ('X', "M0,0 L6,10 M6,0 L0,10"),
    ('Y', "M0,0 L3,5 L6,0 M3,5 L3,10"),
    ('Z', "M0,0 L6,0 L0,10 L6,10"),
    ('0', "A3,5,2.8,5,0,360"),
    ('1', "M1,2.2 L3.6,0 L3.6,10 M1.4,10 L5.8,10"),
    ('2', "A3,2.8,2.8,2.8,-180,40 L0,10 L6,10"),
    ('3', "A2.8,2.6,2.8,2.6,-160,90 A2.8,7.4,3,2.6,-90,160"),
    ('4', "M4.5,10 L4.5,0 L0,6.8 L6,6.8"),
    ('5', "M5.6,0 L0.8,0 L0.5,4.6 A2.9,7,3.1,3,-125,150"),
    ('6', "M4.8,0.4 Q0.4,2.5,0.2,6.8 A3,6.8,2.8,3.2,180,540"),
    ('7', "M0,0 L6,0 L2.2,10"),
    ('8', "A3,2.6,2.7,2.6,0,360 S A3,7.4,3,2.6,0,360"),
    ('9', "A3,3.2,2.8,3.2,0,360 M5.8,3.2 Q5.6,7.5,1.2,9.7"),
    ('.', "M1.2,9.5 L1.2,10"),
    (',', "M1.2,9.4 L0.4,11.6"),
    (':', "M1.2,3.8 L1.2,4.3 M1.2,9.5 L1.2,10"),
    (';', "M1.2,3.8 L1.2,4.3 M1.2,9.4 L0.5,11.6"),
    ('!', "M1.2,0 L1.2,6.6 M1.2,9.5 L1.2,10"),
    ('?', "A3,2.6,2.7,2.6,-170,70 L3,6.4 M3,9.5 L3,10"),
    ('-', "M0.8,5 L5.2,5"),
    ('+', "M3,2.4 L3,7.6 M0.4,5 L5.6,5"),
    ('=', "M0.8,3.8 L5.2,3.8 M0.8,6.2 L5.2,6.2"),
    ('/', "M0.2,10.6 L5.8,-0.6"),
    ('%', "M0.2,10 L5.8,0 S A1.5,2,1.2,1.5,0,360 S A4.5,8,1.2,1.5,0,360"),
    ('(', "A6,5,3.6,5.8,235,125"),
    (')', "A0,5,3.6,5.8,-55,55"),
    ('[', "M4,0 L2,0 L2,10 L4,10"),
    (']', "M2,0 L4,0 L4,10 L2,10"),
    ('_', "M0,11.4 L6,11.4"),
    ('*', "M3,2 L3,6 M1,3 L5,5 M5,3 L1,5"),
    ('"', "M2,0 L2,3 M4,0 L4,3"),
    ('\'', "M3,0 L3,3"),
    ('<', "M5,1.6 L1,5 L5,8.4"),
    ('>', "M1,1.6 L5,5 L1,8.4"),
    ('#', "M2,0 L1.2,10 M4.8,0 L4,10 M0,3.5 L6,3.5 M0,6.8 L6,6.8"),
    ('|', "M3,-0.6 L3,10.6"),
];

fn numbers(text: &str) -> Vec<f32> {
    text.split(',').map(|n| n.parse::<f32>().expect("outline number")).collect()
}

fn parse_outline(path: &str) -> Vec<Vec<(f32, f32)>> {
    let mut strokes: Vec<Vec<(f32, f32)>> = Vec::new();
    for token in path.split_whitespace() {
        let (command, rest) = token.split_at(1);
        if command == "S" {
            strokes.push(Vec::new());
            continue;
        }
        let values = numbers(rest);
        match command {
            "M" => strokes.push(vec![(values[0], values[1])]),
            "L" => strokes.last_mut().expect("stroke").push((values[0], values[1])),
            "Q" => {
                let stroke = strokes.last_mut().expect("stroke");
                let start = *stroke.last().expect("point");
                for step in 1..=CURVE_STEPS {
                    let t = step as f32 / CURVE_STEPS as f32;
                    let u = 1.0 - t;
                    stroke.push((u * u * start.0 + 2.0 * u * t * values[0] + t * t * values[2], u * u * start.1 + 2.0 * u * t * values[1] + t * t * values[3]));
                }
            }
            "A" => {
                let (cx, cy, rx, ry, from, to) = (values[0], values[1], values[2], values[3], values[4], values[5]);
                if strokes.is_empty() {
                    strokes.push(Vec::new());
                }
                let stroke = strokes.last_mut().expect("stroke");
                let steps = (((to - from).abs() / 14.0).ceil() as usize).max(4);
                for step in 0..=steps {
                    let angle = (from + (to - from) * step as f32 / steps as f32).to_radians();
                    stroke.push((cx + rx * angle.cos(), cy + ry * angle.sin()));
                }
            }
            other => panic!("unknown outline command {}", other),
        }
    }
    strokes
}

fn build_glyph(path: &str) -> Glyph {
    let strokes = parse_outline(path);
    let widest = strokes.iter().flatten().fold(0.0f32, |m, p| m.max(p.0));
    Glyph { strokes, advance: widest + GLYPH_GAP }
}

fn build_font() -> Font {
    let mut glyphs = HashMap::new();
    for (character, path) in OUTLINES {
        glyphs.insert(*character, build_glyph(path));
    }
    glyphs.insert(' ', Glyph { strokes: Vec::new(), advance: SPACE_ADVANCE });
    Font { glyphs, fallback: build_glyph("M0,0 L6,0 L6,10 L0,10 L0,0") }
}

pub fn font() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(build_font)
}

impl Font {
    pub fn glyph(&self, character: char) -> &Glyph {
        self.glyphs.get(&character.to_ascii_uppercase()).unwrap_or(&self.fallback)
    }

    pub fn width(&self, text: &str) -> f32 {
        text.chars().map(|c| self.glyph(c).advance).sum()
    }
}
