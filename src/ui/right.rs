//! Right-side panel rendering for visualizer and system logs.

use crate::app::{truncate, App};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::theme::Theme;
use super::visualizer;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App, theme: Theme) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(14), Constraint::Min(8)])
        .split(area);

    let vis_block = Block::default()
        .title(Span::styled("/visualizer", theme.title_style()))
        .borders(Borders::ALL)
        .border_style(theme.border_style());
    let vis_inner = vis_block.inner(rows[0]);
    f.render_widget(vis_block, rows[0]);
    let vis_lines = visualizer::render(app, vis_inner.width, vis_inner.height, theme);
    f.render_widget(
        Paragraph::new(vis_lines)
            .style(theme.text_style())
            .wrap(Wrap { trim: false }),
        vis_inner,
    );

    let logs_block = Block::default()
        .title(Span::styled("/sys_logs", theme.title_style()))
        .borders(Borders::ALL)
        .border_style(theme.border_style());
    let logs_inner = logs_block.inner(rows[1]);
    f.render_widget(logs_block, rows[1]);

    let logs = render_logs(app, logs_inner.width, theme);
    f.render_widget(
        Paragraph::new(logs)
            .style(theme.text_style())
            .wrap(Wrap { trim: false }),
        logs_inner,
    );
}

fn render_logs(app: &App, width: u16, theme: Theme) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled("system initialized", theme.muted_style())),
        Line::from(Span::styled(
            "apple script bridge ready",
            theme.muted_style(),
        )),
    ];

    if let Some(feedback) = &app.feedback {
        lines.push(Line::from(Span::styled(
            truncate(&feedback.to_lowercase(), width as usize),
            Style::default().fg(theme.success),
        )));
    }

    if let Some(err) = &app.err {
        lines.push(Line::from(Span::styled(
            truncate(&format!("err: {}", err.to_lowercase()), width as usize),
            Style::default().fg(theme.danger),
        )));
    }

    if let Some(update) = &app.update_result {
        if update.update_available {
            lines.push(Line::from(Span::styled(
                format!("upgrade: {}", update.latest.to_lowercase()),
                theme.accent_style(),
            )));
            lines.push(Line::from(Span::styled(
                "run brew upgrade spotify-tui-rs",
                theme.muted_style(),
            )));
        }
    }

    if let Some(update_err) = &app.update_err {
        lines.push(Line::from(Span::styled(
            truncate(
                &format!("update err: {}", update_err.to_lowercase()),
                width as usize,
            ),
            Style::default().fg(theme.danger),
        )));
    }

    lines
}
