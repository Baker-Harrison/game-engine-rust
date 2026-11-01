use crate::texture::Texture;

#[derive(Clone)]
pub struct MipTexture {
    pub levels: Vec<Texture>,
}

fn halve(source: &Texture) -> Texture {
    let (width, height) = ((source.width / 2).max(1), (source.height / 2).max(1));
    let mut result = Texture::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let texel = source.get(x * 2 + dx, y * 2 + dy);
                for channel in 0..4 {
                    sum[channel] += texel[channel] as u32;
                }
            }
            result.set(x, y, [(sum[0] / 4) as u8, (sum[1] / 4) as u8, (sum[2] / 4) as u8, (sum[3] / 4) as u8]);
        }
    }
    result
}

impl MipTexture {
    pub fn new(base: &Texture) -> MipTexture {
        let mut levels = vec![base.clone()];
        while levels.last().map_or(false, |last| last.width > 1 || last.height > 1) {
            let next = halve(levels.last().unwrap());
            levels.push(next);
        }
        MipTexture { levels }
    }

    pub fn base_size(&self) -> (usize, usize) {
        (self.levels[0].width, self.levels[0].height)
    }

    pub fn sample(&self, u: f32, v: f32, lod: f32) -> [f32; 4] {
        let top = (self.levels.len() - 1) as f32;
        let lod = lod.clamp(0.0, top);
        let lower = lod.floor() as usize;
        let upper = (lower + 1).min(self.levels.len() - 1);
        let blend = lod - lod.floor();
        let a = self.levels[lower].sample_repeat(u, v);
        if blend < 0.001 || upper == lower {
            return a;
        }
        let b = self.levels[upper].sample_repeat(u, v);
        std::array::from_fn(|channel| a[channel] + (b[channel] - a[channel]) * blend)
    }
}
