pub struct Random {
    state: u64,
}

impl Random {
    pub fn new(seed: u64) -> Random {
        let mut random = Random { state: 0 };
        random.next_u32();
        random.state = random.state.wrapping_add(seed);
        random.next_u32();
        random
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rotation = (old >> 59) as u32;
        xorshifted.rotate_right(rotation)
    }

    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
}

pub fn hash_to_unit(value: u32) -> f32 {
    let mut x = value.wrapping_mul(0x9E3779B1);
    x ^= x >> 15;
    x = x.wrapping_mul(0x85EBCA77);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2AE3D);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}
