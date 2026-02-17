use crate::app::{App, Section};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::theme::Theme;

pub(super) fn render_left_panel(f: &mut Frame, area: Rect, app: &App, theme: Theme) {
    let mut lines = Vec::new();
    push_nav_line(
        &mut lines,
        app,
        Section::NowPlaying,
        "1",
        "now playing",
        theme,
    );
    push_nav_line(&mut lines, app, Section::Playlists, "2", "playlists", theme);
    push_nav_line(&mut lines, app, Section::Queue, "3", "queue", theme);
    push_nav_line(&mut lines, app, Section::Liked, "4", "liked", theme);
    push_nav_line(&mut lines, app, Section::Search, "/", "search", theme);
    push_nav_line(&mut lines, app, Section::Devices, "d", "devices", theme);
    push_nav_line(&mut lines, app, Section::Auth, "a", "auth", theme);
    push_nav_line(&mut lines, app, Section::Setup, "c", "setup", theme);
    push_nav_line(&mut lines, app, Section::Help, "?", "help", theme);

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("controls", theme.accent_style())));
    lines.push(Line::from(Span::styled(
        "space play/pause",
        theme.muted_style(),
    )));
    lines.push(Line::from(Span::styled(
        "n / p next/prev",
        theme.muted_style(),
    )));
    lines.push(Line::from(Span::styled("j / k move", theme.muted_style())));
    lines.push(Line::from(Span::styled(
        "enter select",
        theme.muted_style(),
    )));
    lines.push(Line::from(Span::styled("q quit", theme.muted_style())));

    let block = Block::default()
        .title(Span::styled("/nav", theme.title_style()))
        .borders(Borders::ALL)
        .border_style(theme.border_style());

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn push_nav_line(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    section: Section,
    key: &str,
    label: &str,
    theme: Theme,
) {
    let active = app.section == section;
    let lead = if active { "▸" } else { " " };
    let style = if active {
        theme.accent_style()
    } else {
        theme.text_style()
    };
    lines.push(Line::from(Span::styled(
        format!("{} [{}] {}", lead, key, label),
        style,
    )));
}
