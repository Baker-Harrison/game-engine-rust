#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub min_x: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_z: f32,
}

impl Rect {
    pub fn new(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Rect {
        Rect { min_x, min_z, max_x, max_z }
    }

    pub fn center(&self) -> (f32, f32) {
        ((self.min_x + self.max_x) * 0.5, (self.min_z + self.max_z) * 0.5)
    }

    pub fn contains(&self, x: f32, z: f32) -> bool {
        x >= self.min_x && x <= self.max_x && z >= self.min_z && z <= self.max_z
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Footprint {
    pub x: f32,
    pub z: f32,
    pub half_x: f32,
    pub half_z: f32,
    pub yaw_degrees: f32,
    pub y_min: f32,
    pub y_max: f32,
}

impl Footprint {
    pub fn new(x: f32, z: f32, size_x: f32, size_z: f32, yaw_degrees: f32, y_min: f32, y_max: f32) -> Footprint {
        Footprint { x, z, half_x: size_x * 0.5, half_z: size_z * 0.5, yaw_degrees, y_min, y_max }
    }

    fn axes(&self) -> [(f32, f32); 2] {
        let (s, c) = self.yaw_degrees.to_radians().sin_cos();
        [(c, -s), (s, c)]
    }

    pub fn corners(&self) -> [(f32, f32); 4] {
        let [ax, az] = self.axes();
        let mut out = [(0.0, 0.0); 4];
        for (k, (sx, sz)) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].into_iter().enumerate() {
            out[k] = (
                self.x + ax.0 * self.half_x * sx + az.0 * self.half_z * sz,
                self.z + ax.1 * self.half_x * sx + az.1 * self.half_z * sz,
            );
        }
        out
    }

    pub fn area(&self) -> f32 {
        4.0 * self.half_x * self.half_z
    }

    pub fn height_overlap(&self, other: &Footprint) -> f32 {
        self.y_max.min(other.y_max) - self.y_min.max(other.y_min)
    }

    pub fn contains_point(&self, x: f32, z: f32) -> bool {
        self.distance_to_point(x, z) <= 0.0
    }

    pub fn distance_to_point(&self, x: f32, z: f32) -> f32 {
        let [ax, az] = self.axes();
        let (dx, dz) = (x - self.x, z - self.z);
        let local_x = dx * ax.0 + dz * ax.1;
        let local_z = dx * az.0 + dz * az.1;
        let outside_x = local_x.abs() - self.half_x;
        let outside_z = local_z.abs() - self.half_z;
        if outside_x <= 0.0 && outside_z <= 0.0 {
            return outside_x.max(outside_z);
        }
        outside_x.max(0.0).hypot(outside_z.max(0.0))
    }

    pub fn inside(&self, rect: &Rect) -> bool {
        self.corners().iter().all(|(x, z)| rect.contains(*x, *z))
    }

    pub fn separation(&self, other: &Footprint) -> f32 {
        let mut best = f32::MIN;
        for footprint in [self, other] {
            for axis in footprint.axes() {
                let (a_low, a_high) = self.project(axis);
                let (b_low, b_high) = other.project(axis);
                best = best.max((b_low - a_high).max(a_low - b_high));
            }
        }
        best
    }

    fn project(&self, axis: (f32, f32)) -> (f32, f32) {
        let values = self.corners().map(|(x, z)| x * axis.0 + z * axis.1);
        (values.iter().cloned().fold(f32::MAX, f32::min), values.iter().cloned().fold(f32::MIN, f32::max))
    }
}
