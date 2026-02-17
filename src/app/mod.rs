mod effects;
mod input;
mod messages;
mod state;
mod text;

use anyhow::{Context, Result};
use crossterm::event::{self, Event as CEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::ui;

use effects::{open_browser, spawn_status, spawn_update_check};
use state::{AppMessage, DASHBOARD_URL};

pub use state::{App, Section};
pub use text::{format_time, list_page_size, paginate, section_label, truncate};

pub fn run(mut app: App) -> Result<()> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel::<AppMessage>();
    spawn_status(tx.clone());
    if App::update_checks_enabled() {
        spawn_update_check(tx.clone(), app.version.clone(), false, false);
    }
    if app.setup_auto_open {
        if let Err(err) = open_browser(DASHBOARD_URL) {
            app.err = Some(err.to_string());
        } else {
            app.set_feedback("opened spotify dashboard");
        }
    }

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(200);

    loop {
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_millis(0));
        if event::poll(timeout)? {
            match event::read()? {
                CEvent::Key(key) => {
                    if app.handle_key(key, &tx)? {
                        break;
                    }
                }
                CEvent::Resize(w, h) => {
                    app.width = w;
                    app.height = h;
                }
                _ => {}
            }
        }

        while let Ok(msg) = rx.try_recv() {
            app.handle_message(msg);
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick(&tx);
            last_tick = Instant::now();
        }

        terminal.draw(|f| {
            let size = f.size();
            app.width = size.width;
            app.height = size.height;
            ui::draw(f, &app);
        })?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
