use crate::app::{section_label, truncate, App};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::theme::Theme;

pub(super) fn render_header(f: &mut Frame, area: Rect, app: &App, theme: Theme) {
    let left = format!("spotify tui rs - {}", section_label(app.section));
    let right = format!("v{}", app.version.to_lowercase());
    let gap = area
        .width
        .saturating_sub(left.chars().count() as u16 + right.chars().count() as u16 + 1)
        as usize;

    let line = Line::from(vec![
        Span::styled(left, theme.title_style()),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, theme.muted_style()),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme.border_style());
    f.render_widget(Paragraph::new(line).block(block), area);
}

pub(super) fn render_footer(f: &mut Frame, area: Rect, app: &App, theme: Theme) {
    let left = app
        .feedback
        .as_deref()
        .unwrap_or("ready - press ? for help")
        .to_lowercase();

    let state = if app.status.is_playing {
        "playing"
    } else {
        "paused"
    };
    let shuffle = app
        .playback
        .as_ref()
        .map(|p| if p.shuffle_state { "on" } else { "off" })
        .unwrap_or("?");
    let repeat = app
        .playback
        .as_ref()
        .map(|p| p.repeat_state.to_lowercase())
        .unwrap_or_else(|| "?".to_string());

    let mut right = format!(
        "vol {}% - {} - sh {} - rp {}",
        app.status.volume, state, shuffle, repeat
    );
    if let Some(update) = &app.update_result {
        if update.update_available {
            right = format!("{} - upd {}", right, update.latest.to_lowercase());
        }
    }

    let gap = area
        .width
        .saturating_sub(left.chars().count() as u16 + right.chars().count() as u16 + 1)
        as usize;
    let line = Line::from(vec![
        Span::styled(truncate(&left, area.width as usize), theme.text_style()),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, theme.muted_style()),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.border_style());
    f.render_widget(Paragraph::new(line).block(block), area);
}
