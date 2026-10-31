/// Platform-independent input state. Platform backends fill this in `poll`.
/// Held keys persist between frames; mouse deltas and edge flags are cleared
/// by `begin_frame`.
#[derive(Default, Debug)]
pub struct Input {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub sprint: bool,
    pub quit: bool,
    /// Mouse movement since the last frame, in pixels (right and down are positive).
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    /// Edge: Tab was pressed this frame (toggle mouse capture).
    pub toggle_capture: bool,
    /// Edge: a mouse button was pressed this frame.
    pub click: bool,
    /// Set by the backend when the OS took mouse capture away (focus lost).
    pub capture_lost: bool,
}

impl Input {
    pub fn begin_frame(&mut self) {
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        self.toggle_capture = false;
        self.click = false;
        self.capture_lost = false;
    }
}
