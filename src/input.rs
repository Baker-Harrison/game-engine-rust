use std::collections::HashMap;

macro_rules! keys {
    ($($variant:ident => $name:expr,)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        pub enum Key { $($variant),* }

        impl Key {
            pub const ALL: &'static [Key] = &[$(Key::$variant),*];

            pub fn name(self) -> &'static str {
                match self { $(Key::$variant => $name),* }
            }

            pub fn from_name(name: &str) -> Option<Key> {
                match name { $($name => Some(Key::$variant),)* _ => None }
            }
        }
    };
}

keys! {
    A => "a", B => "b", C => "c", D => "d", E => "e", F => "f", G => "g", H => "h", I => "i",
    J => "j", K => "k", L => "l", M => "m", N => "n", O => "o", P => "p", Q => "q", R => "r",
    S => "s", T => "t", U => "u", V => "v", W => "w", X => "x", Y => "y", Z => "z",
    Digit0 => "0", Digit1 => "1", Digit2 => "2", Digit3 => "3", Digit4 => "4",
    Digit5 => "5", Digit6 => "6", Digit7 => "7", Digit8 => "8", Digit9 => "9",
    Space => "space", Shift => "shift", Control => "control", Alt => "alt",
    Tab => "tab", Escape => "escape", Enter => "enter", Backspace => "backspace",
    Up => "up", Down => "down", Left => "left", Right => "right",
    F1 => "f1", F2 => "f2", F3 => "f3", F4 => "f4",
}

pub const KEY_COUNT: usize = Key::ALL.len();

impl Key {
    pub fn from_char(character: char) -> Option<Key> {
        let lowered = character.to_ascii_lowercase();
        if lowered == ' ' {
            return Some(Key::Space);
        }
        Key::from_name(&lowered.to_string())
    }

    fn index(self) -> usize {
        Key::ALL.iter().position(|k| *k == self).unwrap_or(0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Button {
    Left,
    Right,
    Middle,
}

impl Button {
    pub const ALL: [Button; 3] = [Button::Left, Button::Right, Button::Middle];

    pub fn name(self) -> &'static str {
        match self {
            Button::Left => "mouse_left",
            Button::Right => "mouse_right",
            Button::Middle => "mouse_middle",
        }
    }

    pub fn from_name(name: &str) -> Option<Button> {
        Button::ALL.into_iter().find(|b| b.name() == name)
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    Back,
    Start,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    LeftTrigger,
    RightTrigger,
}

impl PadButton {
    pub const ALL: [PadButton; 16] = [
        PadButton::South,
        PadButton::East,
        PadButton::West,
        PadButton::North,
        PadButton::LeftShoulder,
        PadButton::RightShoulder,
        PadButton::Back,
        PadButton::Start,
        PadButton::LeftStick,
        PadButton::RightStick,
        PadButton::DpadUp,
        PadButton::DpadDown,
        PadButton::DpadLeft,
        PadButton::DpadRight,
        PadButton::LeftTrigger,
        PadButton::RightTrigger,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PadButton::South => "pad_south",
            PadButton::East => "pad_east",
            PadButton::West => "pad_west",
            PadButton::North => "pad_north",
            PadButton::LeftShoulder => "pad_lb",
            PadButton::RightShoulder => "pad_rb",
            PadButton::Back => "pad_back",
            PadButton::Start => "pad_start",
            PadButton::LeftStick => "pad_ls",
            PadButton::RightStick => "pad_rs",
            PadButton::DpadUp => "pad_up",
            PadButton::DpadDown => "pad_down",
            PadButton::DpadLeft => "pad_left",
            PadButton::DpadRight => "pad_right",
            PadButton::LeftTrigger => "pad_lt",
            PadButton::RightTrigger => "pad_rt",
        }
    }

    pub fn from_name(name: &str) -> Option<PadButton> {
        PadButton::ALL.into_iter().find(|b| b.name() == name)
    }

    fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Pad {
    pub connected: bool,
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub buttons: u32,
}

impl Pad {
    pub fn set_button(&mut self, button: PadButton, down: bool) {
        if down {
            self.buttons |= button.bit();
        } else {
            self.buttons &= !button.bit();
        }
    }

    pub fn button(&self, button: PadButton) -> bool {
        self.buttons & button.bit() != 0
    }
}

#[derive(Debug)]
pub struct Input {
    keys_down: Vec<bool>,
    keys_pressed: Vec<bool>,
    keys_released: Vec<bool>,
    buttons_down: [bool; 3],
    buttons_pressed: [bool; 3],
    buttons_released: [bool; 3],
    pad_pressed: u32,
    pad_released: u32,
    pub pad: Pad,
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    pub wheel: f32,
    pub quit: bool,
    pub capture_lost: bool,
}

impl Default for Input {
    fn default() -> Input {
        Input::new()
    }
}

impl Input {
    pub fn new() -> Input {
        Input {
            keys_down: vec![false; KEY_COUNT],
            keys_pressed: vec![false; KEY_COUNT],
            keys_released: vec![false; KEY_COUNT],
            buttons_down: [false; 3],
            buttons_pressed: [false; 3],
            buttons_released: [false; 3],
            pad_pressed: 0,
            pad_released: 0,
            pad: Pad::default(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            wheel: 0.0,
            quit: false,
            capture_lost: false,
        }
    }

    pub fn set_key(&mut self, key: Key, down: bool) {
        let index = key.index();
        if self.keys_down[index] == down {
            return;
        }
        self.keys_down[index] = down;
        if down {
            self.keys_pressed[index] = true;
        } else {
            self.keys_released[index] = true;
        }
    }

    pub fn set_button(&mut self, button: Button, down: bool) {
        let index = button.index();
        if self.buttons_down[index] == down {
            return;
        }
        self.buttons_down[index] = down;
        if down {
            self.buttons_pressed[index] = true;
        } else {
            self.buttons_released[index] = true;
        }
    }

    pub fn set_pad(&mut self, pad: Pad) {
        let changed = pad.buttons ^ self.pad.buttons;
        self.pad_pressed |= changed & pad.buttons;
        self.pad_released |= changed & !pad.buttons;
        self.pad = pad;
    }

    pub fn release_all(&mut self) {
        for key in Key::ALL {
            self.set_key(*key, false);
        }
        for button in Button::ALL {
            self.set_button(button, false);
        }
    }

    pub fn key_down(&self, key: Key) -> bool {
        self.keys_down[key.index()]
    }

    pub fn key_pressed(&self, key: Key) -> bool {
        self.keys_pressed[key.index()]
    }

    pub fn key_released(&self, key: Key) -> bool {
        self.keys_released[key.index()]
    }

    pub fn button_down(&self, button: Button) -> bool {
        self.buttons_down[button.index()]
    }

    pub fn button_pressed(&self, button: Button) -> bool {
        self.buttons_pressed[button.index()]
    }

    pub fn button_released(&self, button: Button) -> bool {
        self.buttons_released[button.index()]
    }

    pub fn pad_pressed(&self, button: PadButton) -> bool {
        self.pad_pressed & button.bit() != 0
    }

    pub fn any_press(&self) -> bool {
        self.keys_pressed.iter().any(|p| *p) || self.buttons_pressed.iter().any(|p| *p) || self.pad_pressed != 0
    }

    pub fn clear_edges(&mut self) {
        self.keys_pressed.iter_mut().for_each(|v| *v = false);
        self.keys_released.iter_mut().for_each(|v| *v = false);
        self.buttons_pressed = [false; 3];
        self.buttons_released = [false; 3];
        self.pad_pressed = 0;
        self.pad_released = 0;
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        self.wheel = 0.0;
        self.capture_lost = false;
    }

    pub fn begin_frame(&mut self) {
        self.clear_edges();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Binding {
    Key(Key),
    Button(Button),
    Pad(PadButton),
}

impl Binding {
    pub fn parse(name: &str) -> Result<Binding, String> {
        if let Some(key) = Key::from_name(name) {
            return Ok(Binding::Key(key));
        }
        if let Some(button) = Button::from_name(name) {
            return Ok(Binding::Button(button));
        }
        if let Some(button) = PadButton::from_name(name) {
            return Ok(Binding::Pad(button));
        }
        Err(format!("unknown input '{}'", name))
    }

    pub fn down(self, input: &Input) -> bool {
        match self {
            Binding::Key(key) => input.key_down(key),
            Binding::Button(button) => input.button_down(button),
            Binding::Pad(button) => match button {
                PadButton::LeftTrigger => input.pad.left_trigger > 0.5,
                PadButton::RightTrigger => input.pad.right_trigger > 0.5,
                other => input.pad.button(other),
            },
        }
    }

    pub fn pressed(self, input: &Input) -> bool {
        match self {
            Binding::Key(key) => input.key_pressed(key),
            Binding::Button(button) => input.button_pressed(button),
            Binding::Pad(button) => input.pad_pressed(button),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActionMap {
    actions: HashMap<String, Vec<Binding>>,
}

impl ActionMap {
    pub fn parse(text: &str) -> Result<ActionMap, String> {
        let mut map = ActionMap::default();
        for (number, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (action, inputs) = line.split_once('=').ok_or_else(|| format!("bindings line {}: expected action = input, input", number + 1))?;
            let mut bindings = Vec::new();
            for name in inputs.split(',') {
                bindings.push(Binding::parse(name.trim()).map_err(|e| format!("bindings line {}: {}", number + 1, e))?);
            }
            map.actions.insert(action.trim().to_string(), bindings);
        }
        Ok(map)
    }

    pub fn to_text(&self) -> String {
        let mut names: Vec<&String> = self.actions.keys().collect();
        names.sort();
        let mut out = String::new();
        for name in names {
            let inputs: Vec<&str> = self.actions[name]
                .iter()
                .map(|b| match b {
                    Binding::Key(key) => key.name(),
                    Binding::Button(button) => button.name(),
                    Binding::Pad(button) => button.name(),
                })
                .collect();
            out.push_str(&format!("{} = {}\n", name, inputs.join(", ")));
        }
        out
    }

    pub fn down(&self, input: &Input, action: &str) -> bool {
        self.actions.get(action).map_or(false, |list| list.iter().any(|b| b.down(input)))
    }

    pub fn pressed(&self, input: &Input, action: &str) -> bool {
        self.actions.get(action).map_or(false, |list| list.iter().any(|b| b.pressed(input)))
    }

    pub fn axis(&self, input: &Input, negative: &str, positive: &str) -> f32 {
        let value = |name: &str| if self.down(input, name) { 1.0 } else { 0.0 };
        value(positive) - value(negative)
    }

    pub fn has(&self, action: &str) -> bool {
        self.actions.contains_key(action)
    }
}

#[derive(Clone, Debug)]
enum ScriptAction {
    Hold(Binding),
    Release(Binding),
    Tap(Binding),
    Look(f32, f32),
    Quit,
}

#[derive(Clone, Debug)]
struct ScriptEvent {
    tick: u64,
    action: ScriptAction,
    repeat: u64,
}

#[derive(Clone, Debug, Default)]
pub struct InputScript {
    events: Vec<ScriptEvent>,
}

impl InputScript {
    pub fn parse(text: &str) -> Result<InputScript, String> {
        let mut script = InputScript::default();
        for (number, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let fail = |message: &str| format!("input script line {}: {}", number + 1, message);
            if words.len() < 2 || words[0] != "tick" && words[0] != "at" {
                return Err(fail("expected: tick N hold|release|tap|look|quit ..."));
            }
            let tick: u64 = words[1].parse().map_err(|_| fail("tick number expected"))?;
            let verb = *words.get(2).ok_or_else(|| fail("missing verb"))?;
            let mut repeat = 1;
            let mut arguments: Vec<&str> = words[3..].to_vec();
            if let Some(position) = arguments.iter().position(|w| *w == "for") {
                repeat = arguments.get(position + 1).and_then(|n| n.parse().ok()).ok_or_else(|| fail("for needs a tick count"))?;
                arguments.truncate(position);
            }
            let binding = |arguments: &[&str]| -> Result<Binding, String> {
                Binding::parse(arguments.first().ok_or_else(|| fail("missing input name"))?).map_err(|e| fail(&e))
            };
            let action = match verb {
                "hold" => ScriptAction::Hold(binding(&arguments)?),
                "release" => ScriptAction::Release(binding(&arguments)?),
                "tap" => ScriptAction::Tap(binding(&arguments)?),
                "look" => {
                    let number = |i: usize| arguments.get(i).and_then(|v| v.parse::<f32>().ok()).ok_or_else(|| fail("look needs dx dy"));
                    ScriptAction::Look(number(0)?, number(1)?)
                }
                "quit" => ScriptAction::Quit,
                other => return Err(fail(&format!("unknown verb '{}'", other))),
            };
            script.events.push(ScriptEvent { tick, action, repeat });
        }
        Ok(script)
    }

    pub fn last_tick(&self) -> u64 {
        self.events.iter().map(|e| e.tick + e.repeat).max().unwrap_or(0)
    }

    pub fn apply(&self, tick: u64, input: &mut Input) {
        for event in &self.events {
            let active = tick >= event.tick && tick < event.tick + event.repeat;
            match event.action {
                ScriptAction::Hold(binding) if tick == event.tick => set_binding(input, binding, true),
                ScriptAction::Release(binding) if tick == event.tick => set_binding(input, binding, false),
                ScriptAction::Tap(binding) => {
                    if tick == event.tick {
                        set_binding(input, binding, true);
                    } else if tick == event.tick + 1 {
                        set_binding(input, binding, false);
                    }
                }
                ScriptAction::Look(dx, dy) if active => {
                    input.mouse_dx += dx;
                    input.mouse_dy += dy;
                }
                ScriptAction::Quit if tick == event.tick => input.quit = true,
                _ => {}
            }
        }
    }
}

fn set_binding(input: &mut Input, binding: Binding, down: bool) {
    match binding {
        Binding::Key(key) => input.set_key(key, down),
        Binding::Button(button) => input.set_button(button, down),
        Binding::Pad(button) => {
            let mut pad = input.pad;
            pad.connected = true;
            pad.set_button(button, down);
            input.set_pad(pad);
        }
    }
}
