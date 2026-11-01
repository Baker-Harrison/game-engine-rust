use crate::sfx::Sounds;
use crate::state::Oilyard;
use engine::math::Vec3;
use engine::mixer::{PlayParams, Sound};
use engine::runtime::Engine;

pub type SoundPick = fn(&Sounds) -> &Sound;

impl Oilyard {
    pub fn play_at(&self, engine: &Engine, pick: SoundPick, position: Vec3, gain: f32, pitch: f32) {
        let params = PlayParams { gain, pitch, position: Some(position), max_distance: 110.0, ..PlayParams::default() };
        engine.play(pick(self.sounds()), params);
    }

    pub fn play_flat(&self, engine: &Engine, pick: SoundPick, gain: f32, pitch: f32) {
        engine.play(pick(self.sounds()), PlayParams { gain, pitch, ..PlayParams::default() });
    }

    pub fn start_ambience(&self, engine: &Engine) {
        engine.play(&self.sounds().wind, PlayParams { gain: 0.22, looped: true, ..PlayParams::default() });
    }
}
