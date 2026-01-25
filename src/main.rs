mod app;
mod apple_script;
mod config;
mod spotify_api;
mod ui;
mod update;

use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let show_version = args.iter().any(|a| a == "--version" || a == "-v");
    let start_setup = args.iter().any(|a| a == "--setup" || a == "-s");

    if show_version {
        println!("spotify-tui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let app = app::App::new(env!("CARGO_PKG_VERSION").to_string(), start_setup)?;
    app::run(app)
}
