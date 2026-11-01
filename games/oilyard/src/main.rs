mod audio;
mod bodies;
mod bots;
mod combat;
mod effects;
mod game;
mod hud;
mod level;
mod player;
mod pose;
mod props;
mod render;
mod sfx;
mod state;
mod weapons;

fn make_game() -> Box<dyn engine::runtime::Game> {
    Box::new(state::Oilyard::new())
}

fn main() {
    engine::cli::game_main(make_game);
}
