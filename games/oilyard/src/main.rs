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
    if std::env::args().nth(1).as_deref() == Some("check") {
        let level = level::build_level(false);
        let findings = level.layout.check();
        print!("{}", engine::layout::check::report(&findings));
        if findings.iter().any(|f| f.severity == engine::probe::Severity::Error) {
            std::process::exit(1);
        }
        return;
    }
    engine::cli::game_main(make_game);
}
