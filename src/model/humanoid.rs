use super::sdf::Sdf;
use crate::math::Vec3;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

#[derive(Clone, Copy)]
pub struct BodySpec {
    pub height: f32,
    pub build: f32,
    pub shoulder_width_ratio: f32,
}

impl BodySpec {
    pub fn adult(height: f32) -> BodySpec {
        BodySpec { height, build: 0.5, shoulder_width_ratio: 0.25 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Landmarks {
    pub head_top: Vec3,
    pub chin: Vec3,
    pub neck_base: Vec3,
    pub shoulder_left: Vec3,
    pub shoulder_right: Vec3,
    pub elbow_left: Vec3,
    pub elbow_right: Vec3,
    pub wrist_left: Vec3,
    pub wrist_right: Vec3,
    pub hip_left: Vec3,
    pub hip_right: Vec3,
    pub knee_left: Vec3,
    pub knee_right: Vec3,
    pub ankle_left: Vec3,
    pub ankle_right: Vec3,
    pub fingertip_left: Vec3,
    pub fingertip_right: Vec3,
}

pub struct RatioCheck {
    pub name: &'static str,
    pub measured: f32,
    pub low: f32,
    pub high: f32,
}

impl RatioCheck {
    pub fn passes(&self) -> bool {
        self.measured >= self.low && self.measured <= self.high
    }

    pub fn message(&self) -> String {
        format!("{}: {:.3}, target {:.3} to {:.3}{}", self.name, self.measured, self.low, self.high, if self.passes() { "" } else { " FAIL" })
    }
}

impl Landmarks {
    pub fn for_spec(spec: &BodySpec) -> Landmarks {
        let h = spec.height;
        let half_shoulder = h * spec.shoulder_width_ratio * 0.5 * 0.8;
        let half_hip = h * 0.095;
        let arm_drop = h * 0.025;
        let shoulder_y = h * 0.785;
        let elbow_y = h * 0.620;
        let wrist_y = h * 0.485;
        let hand_length = h * 0.108;
        let point = |x: f32, y: f32, z: f32| (v(x, y, z), v(-x, y, z));
        let (shoulder_right, shoulder_left) = point(half_shoulder, shoulder_y, 0.0);
        let (elbow_right, elbow_left) = point(half_shoulder + arm_drop, elbow_y, -h * 0.01);
        let (wrist_right, wrist_left) = point(half_shoulder + arm_drop * 1.6, wrist_y, h * 0.015);
        let (fingertip_right, fingertip_left) = point(half_shoulder + arm_drop * 1.6, wrist_y - hand_length, h * 0.02);
        let (hip_right, hip_left) = point(half_hip, h * 0.530, 0.0);
        let (knee_right, knee_left) = point(half_hip * 0.95, h * 0.285, h * 0.008);
        let (ankle_right, ankle_left) = point(half_hip * 0.85, h * 0.039, 0.0);
        Landmarks {
            head_top: v(0.0, h, 0.0),
            chin: v(0.0, h * 0.870, h * 0.03),
            neck_base: v(0.0, h * 0.835, 0.0),
            shoulder_left,
            shoulder_right,
            elbow_left,
            elbow_right,
            wrist_left,
            wrist_right,
            hip_left,
            hip_right,
            knee_left,
            knee_right,
            ankle_left,
            ankle_right,
            fingertip_left,
            fingertip_right,
        }
    }

    pub fn ratio_checks(&self, height: f32) -> Vec<RatioCheck> {
        let arm = |shoulder: Vec3, elbow: Vec3, wrist: Vec3, tip: Vec3| (shoulder - elbow).length() + (elbow - wrist).length() + (wrist - tip).length();
        let span = (self.shoulder_left - self.shoulder_right).length()
            + arm(self.shoulder_left, self.elbow_left, self.wrist_left, self.fingertip_left)
            + arm(self.shoulder_right, self.elbow_right, self.wrist_right, self.fingertip_right);
        let leg = self.hip_right.y - self.ankle_right.y + self.ankle_right.y;
        let upper_arm = (self.shoulder_right - self.elbow_right).length();
        let forearm = (self.elbow_right - self.wrist_right).length();
        let thigh = (self.hip_right - self.knee_right).length();
        let shin = (self.knee_right - self.ankle_right).length();
        vec![
            RatioCheck { name: "head height / height", measured: (self.head_top.y - self.chin.y) / height, low: 0.115, high: 0.145 },
            RatioCheck { name: "leg length / height", measured: leg / height, low: 0.47, high: 0.55 },
            RatioCheck { name: "shoulder joint spacing / height", measured: (self.shoulder_left - self.shoulder_right).length() / height, low: 0.17, high: 0.23 },
            RatioCheck { name: "arm span / height", measured: span / height, low: 0.94, high: 1.04 },
            RatioCheck { name: "forearm / upper arm", measured: forearm / upper_arm, low: 0.78, high: 0.95 },
            RatioCheck { name: "shin / thigh", measured: shin / thigh, low: 0.85, high: 1.02 },
        ]
    }
}

pub struct FingerSpec {
    pub base: Vec3,
    pub lengths: [f32; 3],
    pub radius: f32,
}

#[derive(Clone, Copy)]
pub struct HandPose {
    pub curl: [f32; 5],
    pub spread_degrees: f32,
}

impl HandPose {
    pub fn open() -> HandPose {
        HandPose { curl: [0.1, 0.05, 0.05, 0.05, 0.08], spread_degrees: 8.0 }
    }

    pub fn fist() -> HandPose {
        HandPose { curl: [0.7, 0.95, 0.95, 0.95, 0.95], spread_degrees: 0.0 }
    }

    pub fn pistol_grip() -> HandPose {
        HandPose { curl: [0.35, 0.78, 0.82, 0.85, 0.88], spread_degrees: 2.0 }
    }

    pub fn trigger_finger() -> HandPose {
        HandPose { curl: [0.35, 0.28, 0.82, 0.85, 0.88], spread_degrees: 2.0 }
    }

    pub fn foregrip() -> HandPose {
        HandPose { curl: [0.45, 0.62, 0.66, 0.68, 0.7], spread_degrees: 4.0 }
    }
}

pub const FINGER_LENGTH_RATIOS: [[f32; 3]; 5] = [
    [0.22, 0.17, 0.14],
    [0.24, 0.14, 0.11],
    [0.27, 0.16, 0.12],
    [0.25, 0.15, 0.11],
    [0.19, 0.12, 0.10],
];

pub fn hand(hand_length: f32, pose: &HandPose) -> Sdf {
    let palm_length = hand_length * 0.52;
    let palm_width = hand_length * 0.46;
    let palm_thickness = hand_length * 0.2;
    let palm = Sdf::limb(v(0.0, -hand_length * 0.02, 0.0), v(0.0, palm_length, 0.0), [palm_width * 0.34, palm_thickness * 0.42], [palm_width * 0.5, palm_thickness * 0.36], palm_thickness * 0.34);
    let mut parts = vec![palm];
    for finger in 0..5 {
        let thumb = finger == 0;
        let lateral = if thumb { -0.5 } else { -0.5 + (finger as f32 - 1.0) / 3.0 };
        let x = lateral * palm_width * if thumb { 1.05 } else { 0.78 };
        let base = if thumb { v(-palm_width * 0.30, palm_length * 0.08, palm_thickness * 0.2) } else { v(x, palm_length, 0.0) };
        let ratios = FINGER_LENGTH_RATIOS[finger];
        let radius = hand_length * if thumb { 0.07 } else { 0.056 - 0.003 * (finger as f32 - 1.0).abs() };
        let spread = (lateral * pose.spread_degrees).to_radians();
        let mut position = base;
        let mut heading = if thumb { 0.85f32 } else { spread };
        let mut pitch = 0.0f32;
        for (joint, ratio) in ratios.iter().enumerate() {
            let length = ratio * hand_length;
            pitch += pose.curl[finger] * std::f32::consts::FRAC_PI_2 * if joint == 0 { 0.55 } else { 0.8 };
            let direction = v(-heading.sin() * pitch.cos(), heading.cos() * pitch.cos(), pitch.sin());
            let end = position + direction * length;
            let taper = radius * (1.0 - 0.12 * joint as f32);
            parts.push(Sdf::limb(position, end, [taper, taper], [taper * 0.9, taper * 0.9], taper * 0.7));
            position = end;
            if thumb {
                heading = heading.max(0.2);
            }
        }
    }
    Sdf::smooth_union_all(parts, hand_length * 0.02)
}

pub fn head(height: f32) -> Sdf {
    let head_height = height * 0.13;
    let skull = Sdf::sphere(head_height * 0.46).translate(v(0.0, head_height * 0.52, -head_height * 0.04));
    let jaw = Sdf::round_box(v(head_height * 0.62, head_height * 0.52, head_height * 0.72), head_height * 0.2).translate(v(0.0, head_height * 0.22, head_height * 0.06));
    let nose = Sdf::limb(v(0.0, head_height * 0.42, head_height * 0.42), v(0.0, head_height * 0.28, head_height * 0.6), [head_height * 0.04, head_height * 0.05], [head_height * 0.05, head_height * 0.06], head_height * 0.02);
    let brow = Sdf::round_box(v(head_height * 0.6, head_height * 0.07, head_height * 0.12), head_height * 0.03).translate(v(0.0, head_height * 0.52, head_height * 0.38));
    let socket = |x: f32| Sdf::sphere(head_height * 0.07).translate(v(x * head_height * 0.17, head_height * 0.47, head_height * 0.43));
    let sockets = socket(1.0).union(socket(-1.0));
    let ears = Sdf::limb(v(head_height * 0.3, head_height * 0.44, 0.0), v(head_height * 0.36, head_height * 0.46, -head_height * 0.04), [head_height * 0.03, head_height * 0.09], [head_height * 0.03, head_height * 0.08], head_height * 0.01);
    let ear_pair = ears.clone().union(ears.mirror(super::sdf::Axis::X));
    Sdf::smooth_union_all(vec![skull, jaw, nose, brow, ear_pair], head_height * 0.04).smooth_subtract(sockets, head_height * 0.03)
}

pub fn body(spec: &BodySpec, landmarks: &Landmarks) -> Sdf {
    let h = spec.height;
    let girth = 0.85 + 0.3 * spec.build;
    let limb = |a: Vec3, b: Vec3, start: f32, end: f32| Sdf::limb(a, b, [start * girth, start * girth * 0.9], [end * girth, end * girth * 0.9], h * 0.01);
    let mut parts = Vec::new();
    let chest_center = v(0.0, h * 0.72, 0.0);
    parts.push(Sdf::round_box(v(h * 0.255 * girth, h * 0.20, h * 0.13 * girth), h * 0.05).translate(chest_center));
    parts.push(Sdf::round_box(v(h * 0.20 * girth, h * 0.12, h * 0.115 * girth), h * 0.045).translate(v(0.0, h * 0.60, 0.0)));
    parts.push(Sdf::round_box(v(h * 0.20 * girth, h * 0.11, h * 0.12 * girth), h * 0.05).translate(v(0.0, h * 0.52, 0.0)));
    parts.push(limb(v(0.0, h * 0.80, 0.0), v(0.0, h * 0.88, h * 0.01), h * 0.045, h * 0.04));
    for (shoulder, elbow, wrist, hip, knee, ankle) in [
        (landmarks.shoulder_left, landmarks.elbow_left, landmarks.wrist_left, landmarks.hip_left, landmarks.knee_left, landmarks.ankle_left),
        (landmarks.shoulder_right, landmarks.elbow_right, landmarks.wrist_right, landmarks.hip_right, landmarks.knee_right, landmarks.ankle_right),
    ] {
        parts.push(Sdf::sphere(h * 0.034 * girth).translate(shoulder));
        parts.push(limb(shoulder, elbow, h * 0.032, h * 0.026));
        parts.push(limb(elbow, wrist, h * 0.026, h * 0.019));
        parts.push(limb(hip, knee, h * 0.056, h * 0.036));
        parts.push(limb(knee, ankle, h * 0.036, h * 0.024));
        let toe = v(ankle.x, h * 0.02, ankle.z + h * 0.11);
        parts.push(Sdf::limb(v(ankle.x, ankle.y + h * 0.01, ankle.z - h * 0.015), toe, [h * 0.025, h * 0.02], [h * 0.02, h * 0.014], h * 0.008));
    }
    Sdf::smooth_union_all(parts, h * 0.025)
}

#[derive(Clone, Copy, Debug)]
pub struct GripSocket {
    pub position: Vec3,
    pub rotation_degrees: [f32; 3],
}

impl GripSocket {
    pub fn place(&self, hand: Sdf) -> Sdf {
        hand.rotate_x(self.rotation_degrees[0]).rotate_y(self.rotation_degrees[1]).rotate_z(self.rotation_degrees[2]).translate(self.position)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WeaponSockets {
    pub pistol_grip: GripSocket,
    pub foregrip: GripSocket,
}
