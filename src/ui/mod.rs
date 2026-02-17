//! Top-level UI composition for header, body panels, and footer.

mod center;
mod chrome;
mod grid;
mod nav;
mod right;
mod theme;
mod visualizer;

use crate::app::{App, Section};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use theme::Theme;

pub fn draw(f: &mut Frame, app: &App) {
    let hue = if app.section == Section::Setup {
        app.setup_hue
    } else {
        app.accent_hue
    };
    let theme = Theme::from_hue(hue);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(f.size());

    chrome::render_header(f, root[0], app, theme);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(48),
            Constraint::Length(36),
        ])
        .split(root[1]);

    nav::render(f, body[0], app, theme);
    center::render(f, body[1], app, theme);
    right::render(f, body[2], app, theme);

    chrome::render_footer(f, root[2], app, theme);
}
