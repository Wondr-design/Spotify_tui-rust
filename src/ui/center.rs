use crate::app::{format_time, section_label, truncate, App, Section};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::grid::{self, GridItem};
use super::theme::{color_wheel_cursor, color_wheel_line, Theme};

pub(super) fn render_center_panel(f: &mut Frame, area: Rect, app: &App, theme: Theme) {
    let title = format!("/{}", section_label(app.section).replace(' ', "_"));
    let block = Block::default()
        .title(Span::styled(title, theme.title_style()))
        .borders(Borders::ALL)
        .border_style(theme.border_style());
    let inner = block.inner(area);

    f.render_widget(block, area);

    let lines = match app.section {
        Section::NowPlaying => render_now_playing(app, inner, theme),
        Section::Playlists => render_playlists(app, inner, theme),
        Section::PlaylistTracks => render_playlist_tracks(app, inner, theme),
        Section::Queue => render_queue(app, inner, theme),
        Section::Liked => render_liked(app, inner, theme),
        Section::Search => render_search(app, inner, theme),
        Section::Devices => render_devices(app, inner, theme),
        Section::Setup => render_setup(app, inner, theme),
        Section::Help => render_help(theme),
        Section::Auth => render_auth(app, theme),
    };

    f.render_widget(
        Paragraph::new(lines)
            .style(theme.text_style())
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_now_playing(app: &App, area: Rect, theme: Theme) -> Vec<Line<'static>> {
    if !app.status.is_running {
        return vec![
            Line::from(Span::styled(
                "spotify desktop app is not running",
                theme.muted_style(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "start spotify, then press n or p",
                theme.muted_style(),
            )),
        ];
    }

    let mut lines = Vec::new();
    let title = truncate(
        &app.status.track.to_lowercase(),
        area.width.saturating_sub(2) as usize,
    );
    let artist = truncate(
        &app.status.artist.to_lowercase(),
        area.width.saturating_sub(2) as usize,
    );
    let album = truncate(
        &app.status.album.to_lowercase(),
        area.width.saturating_sub(2) as usize,
    );
    lines.push(Line::from(vec![
        Span::styled("track: ", theme.muted_style()),
        Span::styled(title, theme.text_style()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("artist: ", theme.muted_style()),
        Span::styled(artist, theme.text_style()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("album: ", theme.muted_style()),
        Span::styled(album, theme.text_style()),
    ]));
    if !app.status.album_art_url.is_empty() {
        let art = truncate(
            &format!("art: {}", app.status.album_art_url.to_lowercase()),
            area.width.saturating_sub(2) as usize,
        );
        lines.push(Line::from(Span::styled(art, theme.muted_style())));
    }
    lines.push(Line::from(""));

    let duration = if app.status.duration > 0.0 {
        app.status.duration
    } else {
        1.0
    };
    let progress = (app.status.position / duration).clamp(0.0, 1.0);
    let bar_width = area.width.saturating_sub(12) as usize;
    let bar = progress_bar(bar_width, progress);
    lines.push(Line::from(vec![
        Span::styled("progress ", theme.muted_style()),
        Span::styled(bar, Style::default().fg(theme.accent)),
    ]));
    lines.push(Line::from(Span::styled(
        format!(
            "{} / {}",
            format_time(app.status.position),
            format_time(app.status.duration.max(0.0))
        )
        .to_lowercase(),
        theme.muted_style(),
    )));
    lines.push(Line::from(""));

    let play_state = if app.status.is_playing {
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

    lines.push(Line::from(Span::styled(
        format!("state: {}", play_state),
        theme.text_style(),
    )));
    lines.push(Line::from(Span::styled(
        format!("shuffle: {} | repeat: {}", shuffle, repeat),
        theme.text_style(),
    )));
    lines
}

fn render_playlists(app: &App, area: Rect, theme: Theme) -> Vec<Line<'static>> {
    if !app.authenticated {
        return auth_required(theme);
    }

    let items: Vec<GridItem> = app
        .playlists
        .iter()
        .map(|playlist| GridItem {
            title: playlist.name.clone(),
            subtitle: format!("{} tracks", playlist.tracks.total),
            meta: "playlist".to_string(),
        })
        .collect();

    render_grid_section(items, app.selected_index, area, theme)
}

fn render_playlist_tracks(app: &App, area: Rect, theme: Theme) -> Vec<Line<'static>> {
    if !app.authenticated {
        return auth_required(theme);
    }

    let items: Vec<GridItem> = app
        .playlist_tracks
        .iter()
        .map(|track| GridItem {
            title: track.name.clone(),
            subtitle: track.first_artist_name(),
            meta: track.album.name.clone(),
        })
        .collect();

    render_grid_section(items, app.selected_index, area, theme)
}

fn render_queue(app: &App, area: Rect, theme: Theme) -> Vec<Line<'static>> {
    if !app.authenticated {
        return auth_required(theme);
    }

    let items: Vec<GridItem> = app
        .queue
        .as_ref()
        .map(|queue| {
            queue
                .queue
                .iter()
                .map(|track| GridItem {
                    title: track.name.clone(),
                    subtitle: track.first_artist_name(),
                    meta: "queue".to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    render_grid_section(items, app.selected_index, area, theme)
}

fn render_liked(app: &App, area: Rect, theme: Theme) -> Vec<Line<'static>> {
    if !app.authenticated {
        return auth_required(theme);
    }

    let items: Vec<GridItem> = app
        .liked_songs
        .iter()
        .map(|track| GridItem {
            title: track.name.clone(),
            subtitle: track.first_artist_name(),
            meta: "liked".to_string(),
        })
        .collect();

    render_grid_section(items, app.selected_index, area, theme)
}

fn render_search(app: &App, area: Rect, theme: Theme) -> Vec<Line<'static>> {
    if !app.authenticated {
        return auth_required(theme);
    }

    let mut lines = vec![Line::from(vec![
        Span::styled("query: ", theme.muted_style()),
        Span::styled(app.search_query.to_lowercase(), theme.text_style()),
    ])];
    lines.push(Line::from(Span::styled(
        "type and press enter to search",
        theme.muted_style(),
    )));
    lines.push(Line::from(""));

    if app.search_items.is_empty() {
        lines.push(Line::from(Span::styled(
            "no search results yet",
            theme.muted_style(),
        )));
        return lines;
    }

    let items: Vec<GridItem> = app
        .search_items
        .iter()
        .map(|item| GridItem {
            title: item.name.clone(),
            subtitle: item.kind.clone(),
            meta: String::new(),
        })
        .collect();

    let content_height = area.height.saturating_sub(4);
    let grid_render = grid::render(
        &items,
        app.selected_index,
        area.width,
        content_height,
        theme,
    );

    lines.extend(grid_render.lines);
    if grid_render.pages > 1 {
        lines.push(Line::from(Span::styled(
            format!("page {}/{}", grid_render.page, grid_render.pages),
            theme.muted_style(),
        )));
    }

    lines
}

fn render_devices(app: &App, area: Rect, theme: Theme) -> Vec<Line<'static>> {
    if !app.authenticated {
        return auth_required(theme);
    }

    let items: Vec<GridItem> = app
        .devices
        .iter()
        .map(|device| GridItem {
            title: device.name.clone(),
            subtitle: device.device_type.clone(),
            meta: if device.is_active {
                "active".to_string()
            } else {
                String::new()
            },
        })
        .collect();

    render_grid_section(items, app.selected_index, area, theme)
}

fn render_grid_section(
    items: Vec<GridItem>,
    selected: usize,
    area: Rect,
    theme: Theme,
) -> Vec<Line<'static>> {
    if items.is_empty() {
        return vec![Line::from(Span::styled("loading...", theme.muted_style()))];
    }

    let grid_render = grid::render(&items, selected, area.width, area.height, theme);
    let mut lines = grid_render.lines;
    if grid_render.pages > 1 {
        lines.push(Line::from(Span::styled(
            format!("page {}/{}", grid_render.page, grid_render.pages),
            theme.muted_style(),
        )));
    }
    lines
}

fn render_setup(app: &App, area: Rect, theme: Theme) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled("create a spotify app", theme.text_style())),
        Line::from(Span::styled(
            "redirect uri: http://127.0.0.1:8888/callback",
            theme.muted_style(),
        )),
        Line::from(Span::styled("scopes: web api", theme.muted_style())),
        Line::from(""),
        Line::from(vec![
            Span::styled("client id: ", theme.muted_style()),
            Span::styled(app.setup_client_id.clone(), theme.text_style()),
        ]),
        Line::from(Span::styled(
            "left/right: +/ -1 hue | up/down: +/ -10 hue",
            theme.muted_style(),
        )),
        Line::from(vec![
            Span::styled("accent hue: ", theme.muted_style()),
            Span::styled(app.setup_hue.to_string(), theme.accent_style()),
        ]),
        Line::from(""),
    ];

    let wheel_width = area.width.saturating_sub(2);
    lines.push(color_wheel_line(wheel_width));
    lines.push(color_wheel_cursor(wheel_width, app.setup_hue));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "enter save + auth",
        theme.text_style(),
    )));
    lines.push(Line::from(Span::styled(
        "o open dashboard",
        theme.text_style(),
    )));
    lines.push(Line::from(Span::styled("esc back", theme.text_style())));

    lines
}

fn render_help(theme: Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("keyboard", theme.accent_style())),
        Line::from(Span::styled("space play/pause", theme.text_style())),
        Line::from(Span::styled("n / p next / previous", theme.text_style())),
        Line::from(Span::styled("+ / - volume", theme.text_style())),
        Line::from(Span::styled("j / k move selection", theme.text_style())),
        Line::from(Span::styled("enter open / play", theme.text_style())),
        Line::from(Span::styled("1..4 switch sections", theme.text_style())),
        Line::from(Span::styled("/ search", theme.text_style())),
        Line::from(Span::styled("d devices", theme.text_style())),
        Line::from(Span::styled("a auth", theme.text_style())),
        Line::from(Span::styled("c setup", theme.text_style())),
        Line::from(Span::styled("u update check", theme.text_style())),
        Line::from(Span::styled("q quit", theme.text_style())),
    ]
}

fn render_auth(app: &App, theme: Theme) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled("authentication required", theme.text_style())),
        Line::from(""),
        Line::from(Span::styled("press a to begin oauth", theme.text_style())),
        Line::from(Span::styled(
            "press b or esc to return",
            theme.muted_style(),
        )),
    ];

    if app.auth_in_progress {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "waiting for callback...",
            theme.accent_style(),
        )));
    }

    if !app.auth_url.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            truncate(&app.auth_url, 80).to_lowercase(),
            theme.muted_style(),
        )));
    }

    lines
}

fn auth_required(theme: Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("auth required", theme.text_style())),
        Line::from(Span::styled("open setup with c", theme.muted_style())),
        Line::from(Span::styled(
            "or press a to authenticate",
            theme.muted_style(),
        )),
    ]
}

fn progress_bar(width: usize, progress: f64) -> String {
    if width == 0 {
        return String::new();
    }
    let clamped = progress.clamp(0.0, 1.0);
    let filled = (clamped * width as f64).round() as usize;
    format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}
