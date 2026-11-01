#[derive(Clone)]
pub struct Texture {
    pub width: usize,
    pub height: usize,
    pub texels: Vec<[u8; 4]>,
}

impl Texture {
    pub fn new(width: usize, height: usize) -> Texture {
        Texture { width, height, texels: vec![[0, 0, 0, 255]; width * height] }
    }

    pub fn get(&self, x: usize, y: usize) -> [u8; 4] {
        self.texels[(y % self.height) * self.width + (x % self.width)]
    }

    pub fn set(&mut self, x: usize, y: usize, texel: [u8; 4]) {
        self.texels[y * self.width + x] = texel;
    }

    pub fn sample_repeat(&self, u: f32, v: f32) -> [f32; 4] {
        let x = u.rem_euclid(1.0) * self.width as f32 - 0.5;
        let y = v.rem_euclid(1.0) * self.height as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let wrap = |value: f32, size: usize| (value as i64).rem_euclid(size as i64) as usize;
        let (ix0, iy0) = (wrap(x0, self.width), wrap(y0, self.height));
        let (ix1, iy1) = ((ix0 + 1) % self.width, (iy0 + 1) % self.height);
        let mut result = [0.0f32; 4];
        for channel in 0..4 {
            let at = |x: usize, y: usize| self.texels[y * self.width + x][channel] as f32 / 255.0;
            let top = at(ix0, iy0) * (1.0 - fx) + at(ix1, iy0) * fx;
            let bottom = at(ix0, iy1) * (1.0 - fx) + at(ix1, iy1) * fx;
            result[channel] = top * (1.0 - fy) + bottom * fy;
        }
        result
    }
}
