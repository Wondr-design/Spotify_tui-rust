//! Theme primitives and color utilities derived from a configurable hue.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Color,
    pub accent_soft: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub danger: Color,
    pub success: Color,
}

impl Theme {
    pub fn from_hue(hue: u16) -> Self {
        let (r, g, b) = hsv_to_rgb(hue as f32, 0.70, 0.95);
        let (rs, gs, bs) = hsv_to_rgb(hue as f32, 0.35, 0.55);
        Self {
            accent: Color::Rgb(r, g, b),
            accent_soft: Color::Rgb(rs, gs, bs),
            text: Color::Rgb(228, 231, 235),
            muted: Color::Rgb(137, 145, 158),
            border: Color::Rgb(88, 96, 110),
            danger: Color::Rgb(236, 106, 94),
            success: Color::Rgb(109, 220, 149),
        }
    }

    pub fn title_style(self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border_style(self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn text_style(self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn muted_style(self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn accent_style(self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn color_wheel_line(width: u16) -> Line<'static> {
    let mut spans = Vec::new();
    let count = width.max(24) as usize;
    for i in 0..count {
        let hue = ((i * 360) / count) as u16;
        let (r, g, b) = hsv_to_rgb(hue as f32, 0.90, 0.95);
        spans.push(Span::styled("*", Style::default().fg(Color::Rgb(r, g, b))));
    }
    Line::from(spans)
}

pub fn color_wheel_cursor(width: u16, hue: u16) -> Line<'static> {
    let mut chars = vec![' '; width.max(24) as usize];
    if !chars.is_empty() {
        let idx = (hue as usize * chars.len()) / 360;
        let clamped = idx.min(chars.len().saturating_sub(1));
        chars[clamped] = '▲';
    }
    Line::from(chars.into_iter().collect::<String>())
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = (h % 360.0) / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());

    let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h_prime) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h_prime) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h_prime) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h_prime) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let m = v - c;
    let r = ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (r, g, b)
}
