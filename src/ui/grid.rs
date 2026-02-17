//! Generic grid renderer for playlist, queue, liked, search, and devices.

use crate::app::{list_page_size, paginate, truncate};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::theme::Theme;

#[derive(Debug, Clone)]
pub struct GridItem {
    pub title: String,
    pub subtitle: String,
    pub meta: String,
}

pub struct GridRender {
    pub lines: Vec<Line<'static>>,
    pub page: usize,
    pub pages: usize,
}

pub fn render(
    items: &[GridItem],
    selected: usize,
    width: u16,
    height: u16,
    theme: Theme,
) -> GridRender {
    if items.is_empty() {
        return GridRender {
            lines: vec![Line::from(Span::styled("loading...", theme.muted_style()))],
            page: 0,
            pages: 0,
        };
    }

    let cols = if width >= 96 {
        3
    } else if width >= 64 {
        2
    } else {
        1
    };

    let row_height = 3usize;
    let rows = (list_page_size(height) / row_height).max(1);
    let page_size = (rows * cols).max(1);
    let (start, end, page, pages) = paginate(items.len(), selected, page_size);

    let gutter = 2usize;
    let total_gutter = gutter * cols.saturating_sub(1);
    let cell_width = ((width as usize).saturating_sub(total_gutter + 2) / cols).max(18);

    let mut lines = Vec::new();
    let count = end.saturating_sub(start);
    let used_rows = count.div_ceil(cols);

    for row in 0..used_rows {
        let mut line1 = Vec::new();
        let mut line2 = Vec::new();

        for col in 0..cols {
            if col > 0 {
                line1.push(Span::raw(" ".repeat(gutter)));
                line2.push(Span::raw(" ".repeat(gutter)));
            }

            let idx = start + row * cols + col;
            if idx >= end {
                line1.push(Span::raw(" ".repeat(cell_width)));
                line2.push(Span::raw(" ".repeat(cell_width)));
                continue;
            }

            let item = &items[idx];
            let selected_style = if idx == selected {
                theme.accent_style()
            } else {
                theme.text_style()
            };
            let muted_style = if idx == selected {
                Style::default().fg(theme.accent_soft)
            } else {
                theme.muted_style()
            };

            let lead = if idx == selected { "▸" } else { " " };
            let title_w = cell_width.saturating_sub(2);
            let title = truncate(&item.title.to_lowercase(), title_w);
            let title_cell = pad(&format!("{} {}", lead, title), cell_width);

            let subtitle = truncate(&item.subtitle.to_lowercase(), cell_width);
            let meta = truncate(&item.meta.to_lowercase(), cell_width);
            let detail = if meta.is_empty() {
                subtitle
            } else if subtitle.is_empty() {
                meta
            } else {
                truncate(&format!("{} | {}", subtitle, meta), cell_width)
            };
            let detail_cell = pad(&detail, cell_width);

            line1.push(Span::styled(title_cell, selected_style));
            line2.push(Span::styled(detail_cell, muted_style));
        }

        lines.push(Line::from(line1));
        lines.push(Line::from(line2));
        lines.push(Line::from(""));
    }

    GridRender { lines, page, pages }
}

fn pad(input: &str, width: usize) -> String {
    let len = input.chars().count();
    if len >= width {
        return truncate(input, width);
    }
    format!("{}{}", input, " ".repeat(width - len))
}
