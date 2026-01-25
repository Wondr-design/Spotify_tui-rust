use crate::app::{format_time, list_page_size, paginate, truncate, App, Section};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(size);

    render_header(f, chunks[0], app);
    render_body(f, chunks[1], app);
    render_footer(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let green = Style::default()
        .fg(Color::Rgb(0, 255, 0))
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::Gray);
    let left = "SPOTIFY TUI";
    let right = format!("v{}", app.version);
    let gap = area
        .width
        .saturating_sub(left.len() as u16 + right.len() as u16 + 1) as usize;
    let line = Line::from(vec![
        Span::styled(left, green),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, dim),
    ]);
    let block = Block::default().borders(Borders::BOTTOM);
    let paragraph = Paragraph::new(line).block(block);
    f.render_widget(paragraph, area);
}

fn render_body(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),
            Constraint::Min(0),
            Constraint::Length(30),
        ])
        .split(area);

    render_left(f, cols[0], app);
    render_center(f, cols[1], app);
    render_right(f, cols[2], app);
}

fn render_left(f: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    let dirs = vec![
        Line::from("[ LIBRARY ]"),
        Line::from("[ SEARCH ]"),
        Line::from("[ PODCASTS ]"),
    ];
    let dir_block = Block::default().title("/DIRECTORIES").borders(Borders::ALL);
    f.render_widget(Paragraph::new(dirs).block(dir_block), sections[0]);

    let mut lines = Vec::new();
    if !app.authenticated {
        lines.push(Line::from(Span::styled(
            "[ AUTH REQ ]",
            Style::default().fg(Color::Gray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            " > ROOT_ACCESS",
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(Span::styled(
            " > DEEP_SLEEP",
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(Span::styled(
            " // END OF LIST",
            Style::default().fg(Color::Gray),
        )));
        let limit = sections[1].height.saturating_sub(5) as usize;
        for (i, pl) in app.playlists.iter().take(limit).enumerate() {
            let name = truncate(&pl.name.to_uppercase(), 22);
            let prefix = if i == app.selected_index && app.section == Section::Playlists {
                Span::styled("▶ ", Style::default().fg(Color::Green))
            } else {
                Span::raw("  ")
            };
            lines.push(Line::from(vec![
                prefix,
                Span::styled(name, Style::default().fg(Color::Gray)),
            ]));
        }
    }
    let block = Block::default().title("/PLAYLISTS").borders(Borders::ALL);
    f.render_widget(Paragraph::new(lines).block(block), sections[1]);
}

fn render_center(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.section {
        Section::NowPlaying => "/MAIN_BUFFER",
        Section::Playlists => "/PLAYLISTS",
        Section::PlaylistTracks => "/PLAYLISTS/TRACKS",
        Section::Queue => "/QUEUE_BUFFER",
        Section::Liked => "/LIKED_BUFFER",
        Section::Search => "/SEARCH",
        Section::Devices => "/DEVICES",
        Section::Setup => "/SETUP",
        Section::Help => "/MANUAL",
        Section::Auth => "/AUTH",
    };

    let content = match app.section {
        Section::NowPlaying => render_now_playing(app),
        Section::Playlists => render_playlists(app, area.height),
        Section::PlaylistTracks => render_playlist_tracks(app, area.height),
        Section::Queue => render_queue(app, area.height),
        Section::Liked => render_liked(app, area.height),
        Section::Search => render_search(app, area.height),
        Section::Devices => render_devices(app, area.height),
        Section::Setup => render_setup(app),
        Section::Help => render_help(),
        Section::Auth => render_auth(app),
    };

    let block = Block::default().title(title).borders(Borders::ALL);
    f.render_widget(
        Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_right(f: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    let vis_block = Block::default().title("/VISUALIZER").borders(Borders::ALL);
    let mut vis_lines = vec![
        Line::from("██████████"),
        Line::from("██      ██"),
        Line::from("██      ██"),
        Line::from("██████████"),
        Line::from(""),
        Line::from(app.status.track.to_uppercase()),
        Line::from(app.status.artist.to_uppercase()),
    ];
    if !app.status.album_art_url.is_empty() {
        let label = format!("ART: {}", app.status.album_art_url);
        vis_lines.push(Line::from(truncate(&label, 26)));
    }
    f.render_widget(Paragraph::new(vis_lines).block(vis_block), sections[0]);

    let mut logs = vec![
        Line::from("[8:46:59] SYS: SPOTIFY TUI KERNEL INITIALIZED..."),
        Line::from("[8:46:59] SYS: CONNECTING TO AUDIO DAEMON... OK"),
        Line::from("[8:46:59] SYS: READY FOR INPUT. TYPE \"HELP\"."),
    ];
    if let Some(err) = &app.err {
        logs.push(Line::from(Span::styled(
            format!("ERR: {}", err),
            Style::default().fg(Color::Red),
        )));
    }
    if let Some(update) = &app.update_result {
        if update.update_available {
            logs.push(Line::from(format!("UPDATE: {}", update.message)));
            logs.push(Line::from("RUN: brew upgrade spotify-tui"));
        }
    }
    if let Some(err) = &app.update_err {
        logs.push(Line::from(format!("UPDATE CHECK FAILED: {}", err)));
    }
    let block = Block::default().title("/SYS_LOGS").borders(Borders::ALL);
    f.render_widget(
        Paragraph::new(logs).block(block).wrap(Wrap { trim: false }),
        sections[1],
    );
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let status = if app.status.is_playing {
        "PLAYING"
    } else {
        "PAUSED"
    };
    let shuffle = app
        .playback
        .as_ref()
        .map(|p| if p.shuffle_state { "ON" } else { "OFF" })
        .unwrap_or("?");
    let repeat = app
        .playback
        .as_ref()
        .map(|p| p.repeat_state.to_uppercase())
        .unwrap_or_else(|| "?".to_string());
    let device = app
        .playback
        .as_ref()
        .map(|p| truncate(&p.device.name.to_uppercase(), 12))
        .unwrap_or_else(|| "-".into());

    let left = "root@spotify-tui:~$ enter command (press '/' to focus)";
    let right = format!(
        "VOL: {}%  {}  SHUF:{} REP:{} DEV:{}",
        app.status.volume, status, shuffle, repeat, device
    );
    let gap = area
        .width
        .saturating_sub(left.len() as u16 + right.len() as u16 + 1) as usize;
    let line = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::Green)),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, Style::default().fg(Color::Gray)),
    ]);
    let block = Block::default().borders(Borders::TOP);
    f.render_widget(Paragraph::new(line).block(block), area);
}

fn progress_bar(width: usize, progress: f64) -> String {
    if width == 0 {
        return String::new();
    }
    let clamped = progress.max(0.0).min(1.0);
    let filled = (clamped * width as f64).round() as usize;
    let filled = filled.min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

fn render_now_playing(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if !app.status.is_running {
        lines.push(Line::from("SPOTIFY NOT RUNNING"));
        return lines;
    }

    if !app.status.track.is_empty() {
        let title = truncate(&app.status.track.to_uppercase(), 30);
        let artist = truncate(&app.status.artist.to_uppercase(), 15);
        lines.push(Line::from(format!(
            "▶  {:<30} {:<15} {}",
            title,
            artist,
            format_time(app.status.duration)
        )));
        lines.push(Line::from(format!(
            "   {}",
            app.status.album.to_uppercase()
        )));
        let position = format_time(app.status.position);
        let total = format_time(app.status.duration);
        lines.push(Line::from(format!("   TIME: {} / {}", position, total)));
        if app.status.duration > 0.0 {
            let progress = app.status.position / app.status.duration;
            lines.push(Line::from(format!("   [{}]", progress_bar(24, progress))));
        }
    }
    lines
}

fn render_playlists(app: &App, height: u16) -> Vec<Line<'static>> {
    if !app.authenticated {
        return render_auth_required();
    }
    if app.playlists.is_empty() {
        return vec![Line::from("LOADING...")];
    }
    let mut lines = vec![Line::from("YOUR PLAYLISTS"), Line::from("")];
    let (start, end, page, pages) = paginate(
        app.playlists.len(),
        app.selected_index,
        list_page_size(height),
    );
    for i in start..end {
        let pl = &app.playlists[i];
        let prefix = if i == app.selected_index {
            "▶ "
        } else {
            "  "
        };
        let name = truncate(&pl.name.to_uppercase(), 30);
        lines.push(Line::from(format!(
            "{}{: <30} {} TRACKS",
            prefix, name, pl.tracks.total
        )));
    }
    if pages > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("PAGE {}/{}", page, pages)));
    }
    lines
}

fn render_queue(app: &App, height: u16) -> Vec<Line<'static>> {
    if !app.authenticated {
        return render_auth_required();
    }
    let queue = match &app.queue {
        Some(q) if !q.queue.is_empty() => q,
        _ => return vec![Line::from("LOADING...")],
    };
    let mut lines = vec![Line::from("UP NEXT"), Line::from("")];
    let (start, end, page, pages) = paginate(
        queue.queue.len(),
        app.selected_index,
        list_page_size(height),
    );
    for i in start..end {
        let track = &queue.queue[i];
        let artist = track
            .artists
            .get(0)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let prefix = if i == app.selected_index {
            "▶ "
        } else {
            "  "
        };
        lines.push(Line::from(format!(
            "{}{: <25} {}",
            prefix,
            truncate(&track.name.to_uppercase(), 25),
            artist.to_uppercase()
        )));
    }
    if pages > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("PAGE {}/{}", page, pages)));
    }
    lines
}

fn render_liked(app: &App, height: u16) -> Vec<Line<'static>> {
    if !app.authenticated {
        return render_auth_required();
    }
    if app.liked_songs.is_empty() {
        return vec![Line::from("LOADING...")];
    }
    let mut lines = vec![Line::from("LIKED SONGS"), Line::from("")];
    let (start, end, page, pages) = paginate(
        app.liked_songs.len(),
        app.selected_index,
        list_page_size(height),
    );
    for i in start..end {
        let track = &app.liked_songs[i];
        let artist = track
            .artists
            .get(0)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let prefix = if i == app.selected_index {
            "♥ "
        } else {
            "  "
        };
        lines.push(Line::from(format!(
            "{}{: <25} {}",
            prefix,
            truncate(&track.name.to_uppercase(), 25),
            artist.to_uppercase()
        )));
    }
    if pages > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("PAGE {}/{}", page, pages)));
    }
    lines
}

fn render_playlist_tracks(app: &App, height: u16) -> Vec<Line<'static>> {
    if !app.authenticated {
        return render_auth_required();
    }
    if app.playlist_tracks.is_empty() {
        return vec![Line::from("LOADING...")];
    }
    let mut lines = vec![Line::from("TRACKS"), Line::from("")];
    let (start, end, page, pages) = paginate(
        app.playlist_tracks.len(),
        app.selected_index,
        list_page_size(height),
    );
    for i in start..end {
        let track = &app.playlist_tracks[i];
        let artist = track
            .artists
            .get(0)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let prefix = if i == app.selected_index {
            "▶ "
        } else {
            "  "
        };
        lines.push(Line::from(format!(
            "{}{: <25} {}",
            prefix,
            truncate(&track.name.to_uppercase(), 25),
            artist.to_uppercase()
        )));
    }
    if pages > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("PAGE {}/{}", page, pages)));
    }
    lines
}

fn render_search(app: &App, height: u16) -> Vec<Line<'static>> {
    if !app.authenticated {
        return render_auth_required();
    }
    let mut lines = vec![
        Line::from("SEARCH"),
        Line::from(""),
        Line::from(format!("QUERY: {}", app.search_query)),
        Line::from(""),
    ];

    if app.search_items.is_empty() {
        lines.push(Line::from("TYPE AND PRESS ENTER"));
        return lines;
    }
    let page_size = list_page_size(height).saturating_sub(4).max(3);
    let (start, end, page, pages) = paginate(app.search_items.len(), app.selected_index, page_size);
    for i in start..end {
        let item = &app.search_items[i];
        let prefix = if i == app.selected_index {
            "▶ "
        } else {
            "  "
        };
        let label = format!("{}[{}] {}", prefix, item.kind, item.name);
        lines.push(Line::from(truncate(&label, 44)));
    }
    if pages > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("PAGE {}/{}", page, pages)));
    }
    lines
}

fn render_devices(app: &App, height: u16) -> Vec<Line<'static>> {
    if !app.authenticated {
        return render_auth_required();
    }
    if app.devices.is_empty() {
        return vec![Line::from("LOADING...")];
    }
    let mut lines = vec![Line::from("DEVICES"), Line::from("")];
    let (start, end, page, pages) = paginate(
        app.devices.len(),
        app.selected_index,
        list_page_size(height),
    );
    for i in start..end {
        let device = &app.devices[i];
        let prefix = if i == app.selected_index {
            "▶ "
        } else if device.is_active {
            "● "
        } else {
            "  "
        };
        lines.push(Line::from(format!(
            "{}{: <20} {}",
            prefix,
            truncate(&device.name.to_uppercase(), 20),
            device.device_type.to_uppercase()
        )));
    }
    if pages > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(format!("PAGE {}/{}", page, pages)));
    }
    lines
}

fn render_setup(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from("SETUP"),
        Line::from(""),
        Line::from("1) CREATE A SPOTIFY APP"),
        Line::from("   developer.spotify.com/dashboard"),
        Line::from("2) ADD REDIRECT URI:"),
        Line::from("   http://localhost:8888/callback"),
        Line::from("3) PASTE CLIENT ID BELOW"),
        Line::from(""),
        Line::from(format!("CLIENT ID: {}", app.setup_client_id)),
        Line::from(""),
        Line::from("PRESS ENTER TO SAVE & AUTH"),
        Line::from("PRESS O TO OPEN DASHBOARD"),
        Line::from("PRESS ESC TO SKIP"),
    ]
}

fn render_help() -> Vec<Line<'static>> {
    vec![
        Line::from("KEYBOARD SHORTCUTS"),
        Line::from(""),
        Line::from("SPACE  PLAY / PAUSE"),
        Line::from("N / →  NEXT TRACK"),
        Line::from("P / ←  PREVIOUS TRACK"),
        Line::from("+ / ↑  VOLUME UP"),
        Line::from("- / ↓  VOLUME DOWN"),
        Line::from("J / K  NAVIGATE LISTS"),
        Line::from("ENTER  SELECT ITEM"),
        Line::from("S  SHUFFLE TOGGLE"),
        Line::from("R  REPEAT MODE"),
        Line::from("U  CHECK UPDATES"),
        Line::from("O  OPEN DASHBOARD"),
        Line::from("C  SETUP GUIDE"),
        Line::from("/  SEARCH"),
        Line::from("D  DEVICES"),
        Line::from("A  AUTH"),
        Line::from("ESC/B  BACK"),
        Line::from("1-4  SWITCH SECTIONS"),
        Line::from("?  HELP"),
        Line::from("Q  QUIT"),
    ]
}

fn render_auth(app: &App) -> Vec<Line<'static>> {
    let status = if app.auth_in_progress {
        "WAITING FOR AUTH..."
    } else {
        "PRESS A TO AUTHENTICATE"
    };
    vec![
        Line::from("AUTHENTICATION REQUIRED"),
        Line::from(""),
        Line::from("TO ACCESS PLAYLISTS, QUEUE, AND LIKED SONGS,"),
        Line::from("YOU NEED TO AUTHENTICATE WITH SPOTIFY."),
        Line::from(""),
        Line::from(status),
        Line::from(""),
        Line::from(app.auth_url.clone()),
        Line::from(""),
        Line::from("PRESS ESC TO GO BACK"),
    ]
}

fn render_auth_required() -> Vec<Line<'static>> {
    vec![
        Line::from("AUTHENTICATION REQUIRED"),
        Line::from(""),
        Line::from("THIS FEATURE REQUIRES SPOTIFY WEB API ACCESS."),
        Line::from(""),
        Line::from("PRESS C FOR SETUP GUIDE."),
        Line::from("PRESS A TO AUTHENTICATE."),
    ]
}
