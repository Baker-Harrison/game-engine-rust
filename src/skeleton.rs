use crate::math::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Quat = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    pub fn from_axis_angle(axis: Vec3, radians: f32) -> Quat {
        let axis = axis.normalized();
        let (s, c) = (radians * 0.5).sin_cos();
        Quat { x: axis.x * s, y: axis.y * s, z: axis.z * s, w: c }
    }

    pub fn from_euler_degrees(pitch: f32, yaw: f32, roll: f32) -> Quat {
        let qy = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), yaw.to_radians());
        let qx = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), pitch.to_radians());
        let qz = Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), roll.to_radians());
        qy.mul(qx).mul(qz)
    }

    pub fn mul(self, o: Quat) -> Quat {
        Quat {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }

    pub fn normalized(self) -> Quat {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if length < 1e-9 {
            return Quat::IDENTITY;
        }
        Quat { x: self.x / length, y: self.y / length, z: self.z / length, w: self.w / length }
    }

    pub fn conjugate(self) -> Quat {
        Quat { x: -self.x, y: -self.y, z: -self.z, w: self.w }
    }

    pub fn rotate(self, v: Vec3) -> Vec3 {
        let u = Vec3::new(self.x, self.y, self.z);
        let t = u.cross(v) * 2.0;
        v + t * self.w + u.cross(t)
    }

    pub fn slerp(self, other: Quat, t: f32) -> Quat {
        let mut dot = self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w;
        let mut target = other;
        if dot < 0.0 {
            dot = -dot;
            target = Quat { x: -other.x, y: -other.y, z: -other.z, w: -other.w };
        }
        if dot > 0.9995 {
            return Quat {
                x: self.x + (target.x - self.x) * t,
                y: self.y + (target.y - self.y) * t,
                z: self.z + (target.z - self.z) * t,
                w: self.w + (target.w - self.w) * t,
            }
            .normalized();
        }
        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        let a = ((1.0 - t) * theta).sin() / sin_theta;
        let b = (t * theta).sin() / sin_theta;
        Quat { x: self.x * a + target.x * b, y: self.y * a + target.y * b, z: self.z * a + target.z * b, w: self.w * a + target.w * b }
    }

    pub fn between(from: Vec3, to: Vec3) -> Quat {
        let (a, b) = (from.normalized(), to.normalized());
        let dot = a.dot(b);
        if dot > 0.999999 {
            return Quat::IDENTITY;
        }
        if dot < -0.999999 {
            let axis = if a.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
            return Quat::from_axis_angle(a.cross(axis), std::f32::consts::PI);
        }
        let axis = a.cross(b);
        Quat { x: axis.x, y: axis.y, z: axis.z, w: 1.0 + dot }.normalized()
    }
}

#[derive(Clone, Debug)]
pub struct Joint {
    pub name: String,
    pub parent: Option<usize>,
    pub offset: Vec3,
}

#[derive(Clone, Debug)]
pub struct Skeleton {
    pub joints: Vec<Joint>,
}

#[derive(Clone, Debug)]
pub struct Pose {
    pub rotations: Vec<Quat>,
    pub root_offset: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct JointTransform {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Skeleton {
    pub fn new() -> Skeleton {
        Skeleton { joints: Vec::new() }
    }

    pub fn add(&mut self, name: &str, parent: Option<&str>, offset: Vec3) -> usize {
        let parent = parent.map(|p| self.index(p).unwrap_or_else(|| panic!("skeleton has no joint '{}'", p)));
        self.joints.push(Joint { name: name.to_string(), parent, offset });
        self.joints.len() - 1
    }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }

    pub fn rest_pose(&self) -> Pose {
        Pose { rotations: vec![Quat::IDENTITY; self.joints.len()], root_offset: Vec3::new(0.0, 0.0, 0.0) }
    }

    pub fn world(&self, pose: &Pose) -> Vec<JointTransform> {
        let mut out: Vec<JointTransform> = Vec::with_capacity(self.joints.len());
        for (index, joint) in self.joints.iter().enumerate() {
            let local = pose.rotations[index];
            let transform = match joint.parent {
                None => JointTransform { position: joint.offset + pose.root_offset, rotation: local },
                Some(parent) => {
                    let parent = out[parent];
                    JointTransform { position: parent.position + parent.rotation.rotate(joint.offset), rotation: parent.rotation.mul(local).normalized() }
                }
            };
            out.push(transform);
        }
        out
    }

    pub fn solve_two_bone(&self, pose: &mut Pose, root: &str, mid: &str, tip: &str, target: Vec3, pole: Vec3) -> Result<f32, String> {
        let find = |name: &str| self.index(name).ok_or_else(|| format!("skeleton has no joint '{}'", name));
        let (r, m, t) = (find(root)?, find(mid)?, find(tip)?);
        if self.joints[m].parent != Some(r) || self.joints[t].parent != Some(m) {
            return Err(format!("{} -> {} -> {} is not a parent chain", root, mid, tip));
        }
        let world = self.world(pose);
        let (upper_len, lower_len) = (self.joints[m].offset.length(), self.joints[t].offset.length());
        let root_pos = world[r].position;
        let to_target = target - root_pos;
        let reach = to_target.length().clamp((upper_len - lower_len).abs() + 1e-4, upper_len + lower_len - 1e-4);
        let direction = to_target.normalized();
        let along = (upper_len * upper_len - lower_len * lower_len + reach * reach) / (2.0 * reach);
        let height = (upper_len * upper_len - along * along).max(0.0).sqrt();
        let pole_dir = pole - root_pos;
        let side = (pole_dir - direction * pole_dir.dot(direction)).normalized();
        let side = if side.length() < 1e-6 { Vec3::new(0.0, 1.0, 0.0).cross(direction).normalized() } else { side };
        let mid_pos = root_pos + direction * along + side * height;
        let tip_pos = root_pos + direction * reach;

        let parent_rotation = |index: usize, world: &[JointTransform]| match self.joints[index].parent {
            Some(p) => world[p].rotation,
            None => Quat::IDENTITY,
        };

        let upper_now = world[m].position - root_pos;
        let upper_want = mid_pos - root_pos;
        let swing = Quat::between(upper_now, upper_want);
        let new_root_world = swing.mul(world[r].rotation).normalized();
        pose.rotations[r] = parent_rotation(r, &world).conjugate().mul(new_root_world).normalized();

        let world = self.world(pose);
        let lower_now = world[t].position - world[m].position;
        let lower_want = tip_pos - mid_pos;
        let swing = Quat::between(lower_now, lower_want);
        let new_mid_world = swing.mul(world[m].rotation).normalized();
        pose.rotations[m] = parent_rotation(m, &world).conjugate().mul(new_mid_world).normalized();

        let solved = self.world(pose);
        Ok((solved[t].position - target).length())
    }
}

impl Pose {
    pub fn blend(&self, other: &Pose, weight: f32) -> Pose {
        Pose {
            rotations: self.rotations.iter().zip(&other.rotations).map(|(a, b)| a.slerp(*b, weight)).collect(),
            root_offset: self.root_offset.lerp(other.root_offset, weight),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Track {
    pub joint: usize,
    pub keys: Vec<(f32, Quat)>,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub duration: f32,
    pub looped: bool,
    pub tracks: Vec<Track>,
    pub root_keys: Vec<(f32, Vec3)>,
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn locate(times: impl Iterator<Item = f32> + Clone, t: f32) -> (usize, usize, f32) {
    let times: Vec<f32> = times.collect();
    if times.len() == 1 || t <= times[0] {
        return (0, 0, 0.0);
    }
    for i in 1..times.len() {
        if t <= times[i] {
            let span = (times[i] - times[i - 1]).max(1e-6);
            return (i - 1, i, smooth(((t - times[i - 1]) / span).clamp(0.0, 1.0)));
        }
    }
    (times.len() - 1, times.len() - 1, 0.0)
}

impl Clip {
    pub fn new(name: &str, duration: f32, looped: bool) -> Clip {
        Clip { name: name.to_string(), duration, looped, tracks: Vec::new(), root_keys: Vec::new() }
    }

    pub fn rotate(mut self, skeleton: &Skeleton, joint: &str, keys: &[(f32, Quat)]) -> Clip {
        let index = skeleton.index(joint).unwrap_or_else(|| panic!("skeleton has no joint '{}'", joint));
        self.tracks.push(Track { joint: index, keys: keys.to_vec() });
        self
    }

    pub fn translate_root(mut self, keys: &[(f32, Vec3)]) -> Clip {
        self.root_keys = keys.to_vec();
        self
    }

    pub fn wrap(&self, time: f32) -> f32 {
        if self.looped && self.duration > 0.0 {
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        }
    }

    pub fn sample(&self, skeleton: &Skeleton, time: f32) -> Pose {
        let t = self.wrap(time);
        let mut pose = skeleton.rest_pose();
        for track in &self.tracks {
            if track.keys.is_empty() {
                continue;
            }
            let (a, b, w) = locate(track.keys.iter().map(|k| k.0), t);
            pose.rotations[track.joint] = track.keys[a].1.slerp(track.keys[b].1, w);
        }
        if !self.root_keys.is_empty() {
            let (a, b, w) = locate(self.root_keys.iter().map(|k| k.0), t);
            pose.root_offset = self.root_keys[a].1.lerp(self.root_keys[b].1, w);
        }
        pose
    }

    pub fn finished(&self, time: f32) -> bool {
        !self.looped && time >= self.duration
    }
}

#[derive(Clone, Debug)]
pub struct AnimPlayer {
    pub clips: Vec<Clip>,
    current: usize,
    previous: Option<usize>,
    time: f32,
    previous_time: f32,
    blend_elapsed: f32,
    blend_duration: f32,
}

impl AnimPlayer {
    pub fn new(clips: Vec<Clip>) -> AnimPlayer {
        AnimPlayer { clips, current: 0, previous: None, time: 0.0, previous_time: 0.0, blend_elapsed: 0.0, blend_duration: 0.0 }
    }

    pub fn current(&self) -> &str {
        &self.clips[self.current].name
    }

    pub fn play(&mut self, name: &str, blend_seconds: f32) -> Result<(), String> {
        let index = self.clips.iter().position(|c| c.name == name).ok_or_else(|| format!("no clip named '{}'", name))?;
        if index == self.current && self.clips[index].looped {
            return Ok(());
        }
        self.previous = Some(self.current);
        self.previous_time = self.time;
        self.current = index;
        self.time = 0.0;
        self.blend_elapsed = 0.0;
        self.blend_duration = blend_seconds.max(0.0);
        Ok(())
    }

    pub fn finished(&self) -> bool {
        self.clips[self.current].finished(self.time)
    }

    pub fn progress(&self) -> f32 {
        let clip = &self.clips[self.current];
        if clip.duration <= 0.0 { 1.0 } else { (self.time / clip.duration).clamp(0.0, 1.0) }
    }

    pub fn update(&mut self, dt: f32, speed: f32) {
        self.time += dt * speed;
        self.previous_time += dt * speed;
        if self.previous.is_some() {
            self.blend_elapsed += dt;
            if self.blend_elapsed >= self.blend_duration {
                self.previous = None;
            }
        }
    }

    pub fn pose(&self, skeleton: &Skeleton) -> Pose {
        let now = self.clips[self.current].sample(skeleton, self.time);
        match self.previous {
            Some(previous) if self.blend_duration > 0.0 => {
                let before = self.clips[previous].sample(skeleton, self.previous_time);
                before.blend(&now, smooth((self.blend_elapsed / self.blend_duration).clamp(0.0, 1.0)))
            }
            _ => now,
        }
    }
}
