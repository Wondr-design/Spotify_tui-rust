//! Lightweight terminal visualizer driven by playback progress and tick state.

use crate::app::{format_time, truncate, App};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::theme::Theme;

pub(super) fn render(app: &App, width: u16, height: u16, theme: Theme) -> Vec<Line<'static>> {
    let chart_height = height.saturating_sub(6).clamp(4, 12) as usize;
    let bars = (width.saturating_sub(4) as usize / 2).clamp(8, 24);
    let mut out = Vec::new();

    let energy = if app.status.is_playing { 1.0 } else { 0.35 };
    let progress = if app.status.duration > 0.0 {
        (app.status.position / app.status.duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let tick_phase = app.animation_tick as f64 * 0.35;

    let heights: Vec<f64> = (0..bars)
        .map(|idx| {
            let phase = tick_phase + (idx as f64 * 0.9) + (progress * std::f64::consts::TAU);
            let wave = (phase.sin() + 1.0) * 0.5;
            (1.0 + wave * (chart_height as f64 - 1.0)) * energy
        })
        .collect();

    for row in (0..chart_height).rev() {
        let mut spans = Vec::new();
        for (idx, h) in heights.iter().enumerate() {
            let filled = *h > row as f64;
            let color = bar_color(theme, row, chart_height, filled);
            let symbol = if filled { "█" } else { "·" };
            spans.push(Span::styled(symbol, Style::default().fg(color)));
            if idx + 1 < heights.len() {
                spans.push(Span::raw(" "));
            }
        }
        out.push(Line::from(spans));
    }

    out.push(Line::from(""));
    let track = truncate(
        &app.status.track.to_lowercase(),
        width.saturating_sub(2) as usize,
    );
    let artist = truncate(
        &app.status.artist.to_lowercase(),
        width.saturating_sub(2) as usize,
    );
    out.push(Line::from(Span::styled(track, theme.text_style())));
    out.push(Line::from(Span::styled(artist, theme.muted_style())));

    if app.status.duration > 0.0 {
        let now = format_time(app.status.position);
        let total = format_time(app.status.duration);
        out.push(Line::from(Span::styled(
            format!("{} / {}", now, total).to_lowercase(),
            theme.muted_style(),
        )));
    }

    out
}

fn bar_color(theme: Theme, row: usize, chart_height: usize, filled: bool) -> Color {
    if !filled {
        return theme.border;
    }
    let ratio = if chart_height > 1 {
        row as f32 / (chart_height as f32 - 1.0)
    } else {
        0.0
    };
    match ratio {
        r if r > 0.66 => theme.accent,
        r if r > 0.33 => theme.accent_soft,
        _ => theme.text,
    }
}
