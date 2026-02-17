use crate::apple_script;
use crate::config;
use crate::spotify_api::{
    self, APIClient, ApiError, Device, PlaybackState, Playlist, QueueResponse, Track,
};
use crate::ui;
use crate::update;
use anyhow::{Context, Result};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const DASHBOARD_URL: &str = "https://developer.spotify.com/dashboard";
const FEEDBACK_TIMEOUT: Duration = Duration::from_millis(2200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    NowPlaying,
    Playlists,
    Queue,
    Liked,
    Search,
    Devices,
    PlaylistTracks,
    Setup,
    Help,
    Auth,
}

#[derive(Debug, Clone)]
pub struct SearchItem {
    pub kind: String,
    pub name: String,
    pub uri: String,
}

#[derive(Debug)]
enum AppMessage {
    Status(apple_script::Status),
    Playlists(Vec<Playlist>),
    Queue(QueueResponse),
    Liked(Vec<Track>),
    PlaylistTracks(Vec<Track>),
    Playback(PlaybackState),
    Devices(Vec<Device>),
    Search(spotify_api::SearchResults),
    Update(update::ResultInfo, bool),
    UpdateErr(String, bool),
    AuthComplete,
    Error(String, bool),
}

pub struct App {
    pub section: Section,
    pub prev_section: Section,
    pub status: apple_script::Status,
    pub playback: Option<PlaybackState>,
    pub playlists: Vec<Playlist>,
    pub playlist_tracks: Vec<Track>,
    pub selected_playlist: Option<Playlist>,
    pub queue: Option<QueueResponse>,
    pub liked_songs: Vec<Track>,
    pub search_query: String,
    pub search_items: Vec<SearchItem>,
    pub devices: Vec<Device>,
    pub selected_index: usize,
    pub err: Option<String>,
    pub update_result: Option<update::ResultInfo>,
    pub update_err: Option<String>,
    pub setup_client_id: String,
    pub setup_auto_open: bool,
    pub accent_hue: u16,
    pub setup_hue: u16,
    pub animation_tick: u64,
    pub feedback: Option<String>,
    feedback_until: Instant,

    pub api_client: Option<Arc<Mutex<APIClient>>>,
    pub authenticated: bool,
    pub auth_url: String,
    pub auth_in_progress: bool,

    pub width: u16,
    pub height: u16,

    last_action_at: Instant,
    last_status_fetch: Instant,
    last_api_fetch: Instant,
    pub version: String,
}

impl App {
    pub fn new(version: String, start_setup: bool) -> Result<Self> {
        let mut app = Self {
            section: Section::NowPlaying,
            prev_section: Section::NowPlaying,
            status: apple_script::Status::default(),
            playback: None,
            playlists: Vec::new(),
            playlist_tracks: Vec::new(),
            selected_playlist: None,
            queue: None,
            liked_songs: Vec::new(),
            search_query: String::new(),
            search_items: Vec::new(),
            devices: Vec::new(),
            selected_index: 0,
            err: None,
            update_result: None,
            update_err: None,
            setup_client_id: String::new(),
            setup_auto_open: false,
            accent_hue: config::DEFAULT_ACCENT_HUE,
            setup_hue: config::DEFAULT_ACCENT_HUE,
            animation_tick: 0,
            feedback: None,
            feedback_until: Instant::now(),
            api_client: None,
            authenticated: false,
            auth_url: String::new(),
            auth_in_progress: false,
            width: 0,
            height: 0,
            last_action_at: Instant::now(),
            last_status_fetch: Instant::now()
                .checked_sub(Duration::from_secs(5))
                .unwrap_or_else(Instant::now),
            last_api_fetch: Instant::now()
                .checked_sub(Duration::from_secs(10))
                .unwrap_or_else(Instant::now),
            version,
        };

        if let Ok(cfg) = config::load_config() {
            let client_id = cfg.client_id.trim().to_string();
            app.setup_client_id = client_id.clone();
            app.accent_hue = cfg.accent_hue % 360;
            app.setup_hue = app.accent_hue;

            if !client_id.is_empty() {
                let mut client = APIClient::new(client_id)?;
                let _ = client.load_token_from_disk();
                app.authenticated = client.is_authenticated();
                app.api_client = Some(Arc::new(Mutex::new(client)));
            } else {
                app.section = Section::Setup;
            }
        } else {
            app.section = Section::Setup;
        }

        if start_setup {
            app.section = Section::Setup;
            app.setup_auto_open = true;
            app.setup_hue = app.accent_hue;
        }

        Ok(app)
    }

    fn update_checks_enabled() -> bool {
        let val = std::env::var("SPOTIFY_TUI_NO_UPDATE_CHECK").unwrap_or_default();
        matches!(
            val.trim().to_lowercase().as_str(),
            "" | "0" | "false" | "no"
        )
    }

    fn ensure_auth_required(&mut self) -> bool {
        if self.authenticated {
            return false;
        }
        self.prev_section = self.section;
        self.section = Section::Auth;
        self.auth_in_progress = false;
        self.set_feedback("auth required");
        true
    }

    fn set_feedback<S: Into<String>>(&mut self, message: S) {
        self.feedback = Some(message.into());
        self.feedback_until = Instant::now() + FEEDBACK_TIMEOUT;
    }

    fn bump_setup_hue(&mut self, step: i16) {
        let mut hue = self.setup_hue as i16 + step;
        while hue < 0 {
            hue += 360;
        }
        self.setup_hue = (hue as u16) % 360;
        self.accent_hue = self.setup_hue;
        self.set_feedback(format!("accent hue {}", self.setup_hue));
    }

    fn set_section(&mut self, section: Section) {
        self.prev_section = self.section;
        self.section = section;
        self.selected_index = 0;
    }

    fn navigate_to(&mut self, section: Section) {
        self.set_section(section);
        self.set_feedback(format!("section: {}", section_label(section)));
    }

    fn list_len(&self) -> usize {
        match self.section {
            Section::Playlists => self.playlists.len(),
            Section::Queue => self.queue.as_ref().map(|q| q.queue.len()).unwrap_or(0),
            Section::Liked => self.liked_songs.len(),
            Section::PlaylistTracks => self.playlist_tracks.len(),
            Section::Search => self.search_items.len(),
            Section::Devices => self.devices.len(),
            _ => 0,
        }
    }

    fn handle_enter(&mut self, tx: &Sender<AppMessage>) {
        match self.section {
            Section::Playlists => {
                if self.ensure_auth_required() {
                    return;
                }
                if self.selected_index < self.playlists.len() {
                    let pl = self.playlists[self.selected_index].clone();
                    self.selected_playlist = Some(pl.clone());
                    self.set_section(Section::PlaylistTracks);
                    if let Some(client) = self.api_client.clone() {
                        spawn_playlist_tracks(tx.clone(), client, pl.id);
                    }
                }
            }
            Section::PlaylistTracks => {
                if self.ensure_auth_required() {
                    return;
                }
                if self.selected_index < self.playlist_tracks.len() {
                    let track = self.playlist_tracks[self.selected_index].clone();
                    if let Some(client) = self.api_client.clone() {
                        spawn_start_playback(tx.clone(), client, vec![track.uri], String::new());
                    }
                }
            }
            Section::Queue => {
                if self.ensure_auth_required() {
                    return;
                }
                if let Some(queue) = &self.queue {
                    if self.selected_index < queue.queue.len() {
                        let track = queue.queue[self.selected_index].clone();
                        if let Some(client) = self.api_client.clone() {
                            spawn_start_playback(
                                tx.clone(),
                                client,
                                vec![track.uri],
                                String::new(),
                            );
                        }
                    }
                }
            }
            Section::Liked => {
                if self.ensure_auth_required() {
                    return;
                }
                if self.selected_index < self.liked_songs.len() {
                    let track = self.liked_songs[self.selected_index].clone();
                    if let Some(client) = self.api_client.clone() {
                        spawn_start_playback(tx.clone(), client, vec![track.uri], String::new());
                    }
                }
            }
            Section::Search => {
                if self.ensure_auth_required() {
                    return;
                }
                if self.selected_index < self.search_items.len() {
                    let item = self.search_items[self.selected_index].clone();
                    if let Some(client) = self.api_client.clone() {
                        if item.kind == "track" {
                            spawn_start_playback(tx.clone(), client, vec![item.uri], String::new());
                        } else {
                            spawn_start_playback(tx.clone(), client, Vec::new(), item.uri);
                        }
                    }
                }
            }
            Section::Devices => {
                if self.ensure_auth_required() {
                    return;
                }
                if self.selected_index < self.devices.len() {
                    let device = self.devices[self.selected_index].clone();
                    if let Some(client) = self.api_client.clone() {
                        spawn_transfer_device(tx.clone(), client, device.id);
                    }
                }
            }
            _ => {}
        }
    }

    fn begin_auth(&mut self, tx: &Sender<AppMessage>) {
        let (verifier, challenge) = spotify_api::generate_pkce();
        let state = spotify_api::random_state();
        if let Some(client) = self.api_client.clone() {
            let url = {
                let client = client.lock().unwrap();
                client.get_auth_url(&state, &challenge).unwrap_or_default()
            };
            self.auth_url = url.clone();
            self.auth_in_progress = true;
            if let Err(err) = open_browser(&url) {
                self.err = Some(err.to_string());
            } else {
                self.set_feedback("opened spotify auth");
            }
            spawn_auth(tx.clone(), client, state, verifier);
        }
    }

    fn build_search_items(&mut self, res: spotify_api::SearchResults) {
        let mut items = Vec::new();
        for t in res.tracks.items.into_iter().flatten() {
            let artist = t.first_artist_name();
            items.push(SearchItem {
                kind: "track".into(),
                name: format!("{} - {}", t.name.to_lowercase(), artist.to_lowercase()),
                uri: t.uri,
            });
        }
        for p in res.playlists.items.into_iter().flatten() {
            items.push(SearchItem {
                kind: "playlist".into(),
                name: p.name.to_lowercase(),
                uri: p.uri,
            });
        }
        for a in res.artists.items.into_iter().flatten() {
            items.push(SearchItem {
                kind: "artist".into(),
                name: a.name.to_lowercase(),
                uri: a.uri,
            });
        }

        let total = items.len();
        self.search_items = items;
        self.selected_index = 0;
        self.set_feedback(format!("{} search matches", total));
    }

    fn handle_key(&mut self, key: KeyEvent, tx: &Sender<AppMessage>) -> Result<bool> {
        if key.kind == KeyEventKind::Release {
            return Ok(false);
        }

        // Section-specific handling
        match self.section {
            Section::Auth => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.set_section(self.prev_section);
                        self.set_feedback("back");
                    }
                    KeyCode::Char('b') => {
                        self.set_section(self.prev_section);
                        self.set_feedback("back");
                    }
                    KeyCode::Char('a') => {
                        if self.api_client.is_none() {
                            if let Ok(cfg) = config::load_config() {
                                let client_id = cfg.client_id.trim().to_string();
                                if !client_id.is_empty() {
                                    let client = APIClient::new(client_id)?;
                                    self.api_client = Some(Arc::new(Mutex::new(client)));
                                } else {
                                    self.err = Some(
                                        "missing client_id in ~/.spotify-tui/config.json".into(),
                                    );
                                    self.set_feedback("setup is required");
                                    return Ok(false);
                                }
                            } else {
                                self.err =
                                    Some("missing client_id in ~/.spotify-tui/config.json".into());
                                self.set_feedback("setup is required");
                                return Ok(false);
                            }
                        }
                        self.begin_auth(tx);
                    }
                    _ => {}
                }
                return Ok(false);
            }
            Section::Setup => {
                match key.code {
                    KeyCode::Esc => {
                        self.navigate_to(Section::NowPlaying);
                    }
                    KeyCode::Enter => {
                        let client_id = self.setup_client_id.trim().to_string();
                        if client_id.is_empty() {
                            self.err = Some("client_id required".into());
                            self.set_feedback("client id is required");
                            return Ok(false);
                        }
                        self.accent_hue = self.setup_hue;
                        config::save_config(&config::Config {
                            client_id: client_id.clone(),
                            accent_hue: self.accent_hue,
                        })?;
                        let client = APIClient::new(client_id)?;
                        self.api_client = Some(Arc::new(Mutex::new(client)));
                        self.authenticated = false;
                        self.set_feedback("saved setup");
                        self.set_section(Section::Auth);
                        self.begin_auth(tx);
                    }
                    KeyCode::Backspace => {
                        self.setup_client_id.pop();
                    }
                    KeyCode::Left => self.bump_setup_hue(-1),
                    KeyCode::Right => self.bump_setup_hue(1),
                    KeyCode::Down => self.bump_setup_hue(-10),
                    KeyCode::Up => self.bump_setup_hue(10),
                    KeyCode::Char('o') => {
                        if let Err(err) = open_browser(DASHBOARD_URL) {
                            self.err = Some(err.to_string());
                        } else {
                            self.set_feedback("opened spotify dashboard");
                        }
                    }
                    KeyCode::Char(c) => {
                        if !c.is_control() {
                            self.setup_client_id.push(c);
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }
            Section::Search => match key.code {
                KeyCode::Esc => {
                    self.set_section(self.prev_section);
                    self.set_feedback("back");
                    return Ok(false);
                }
                KeyCode::Enter => {
                    if !self.search_items.is_empty()
                        && self.selected_index < self.search_items.len()
                    {
                        self.handle_enter(tx);
                        return Ok(false);
                    }
                    let q = self.search_query.trim().to_string();
                    if !q.is_empty() {
                        if self.api_client.is_none() {
                            self.err = Some(
                                "missing API client; configure ~/.spotify-tui/config.json".into(),
                            );
                        } else if let Some(client) = self.api_client.clone() {
                            self.set_feedback(format!("searching for {}", q.to_lowercase()));
                            spawn_search(tx.clone(), client, q);
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.search_items.clear();
                    self.selected_index = 0;
                    return Ok(false);
                }
                KeyCode::Up => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                    return Ok(false);
                }
                KeyCode::Down => {
                    if self.selected_index + 1 < self.search_items.len() {
                        self.selected_index += 1;
                    }
                    return Ok(false);
                }
                KeyCode::Char(c) => {
                    if !c.is_control() {
                        self.search_query.push(c);
                        self.search_items.clear();
                        self.selected_index = 0;
                    }
                    return Ok(false);
                }
                _ => {}
            },
            Section::Devices | Section::PlaylistTracks => {
                if matches!(key.code, KeyCode::Esc) || matches!(key.code, KeyCode::Char('b')) {
                    self.set_section(self.prev_section);
                    self.set_feedback("back");
                    return Ok(false);
                }
            }
            _ => {}
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
            KeyCode::Char('1') => self.navigate_to(Section::NowPlaying),
            KeyCode::Char('2') => {
                self.navigate_to(Section::Playlists);
                if self.ensure_auth_required() {
                    return Ok(false);
                }
                if let Some(client) = self.api_client.clone() {
                    self.set_feedback("loading playlists");
                    spawn_playlists(tx.clone(), client);
                }
            }
            KeyCode::Char('3') => {
                self.navigate_to(Section::Queue);
                if self.ensure_auth_required() {
                    return Ok(false);
                }
                if let Some(client) = self.api_client.clone() {
                    self.set_feedback("loading queue");
                    spawn_queue(tx.clone(), client);
                }
            }
            KeyCode::Char('4') => {
                self.navigate_to(Section::Liked);
                if self.ensure_auth_required() {
                    return Ok(false);
                }
                if let Some(client) = self.api_client.clone() {
                    self.set_feedback("loading liked songs");
                    spawn_liked(tx.clone(), client);
                }
            }
            KeyCode::Char('c') => {
                self.setup_hue = self.accent_hue;
                self.navigate_to(Section::Setup);
            }
            KeyCode::Char('/') => {
                self.navigate_to(Section::Search);
                if self.ensure_auth_required() {
                    return Ok(false);
                }
                self.search_query.clear();
                self.search_items.clear();
            }
            KeyCode::Char('d') => {
                self.navigate_to(Section::Devices);
                if self.ensure_auth_required() {
                    return Ok(false);
                }
                if let Some(client) = self.api_client.clone() {
                    self.set_feedback("loading devices");
                    spawn_devices(tx.clone(), client);
                }
            }
            KeyCode::Char('a') => {
                self.navigate_to(Section::Auth);
            }
            KeyCode::Char('s') => {
                if self.authenticated {
                    if let Some(client) = self.api_client.clone() {
                        let next = self
                            .playback
                            .as_ref()
                            .map(|p| !p.shuffle_state)
                            .unwrap_or(true);
                        self.set_feedback(format!("shuffle {}", if next { "on" } else { "off" }));
                        spawn_shuffle(tx.clone(), client, next);
                    }
                }
            }
            KeyCode::Char('r') => {
                if self.authenticated {
                    if let Some(client) = self.api_client.clone() {
                        let next = next_repeat_state(self.playback.as_ref());
                        self.set_feedback(format!("repeat {}", next));
                        spawn_repeat(tx.clone(), client, next);
                    }
                }
            }
            KeyCode::Char('u') => {
                if Self::update_checks_enabled() {
                    self.set_feedback("checking updates");
                    spawn_update_check(tx.clone(), self.version.clone(), true, true);
                }
            }
            KeyCode::Char('?') | KeyCode::Char('h') => self.navigate_to(Section::Help),
            _ => {}
        }

        // Section-specific list controls
        match self.section {
            Section::NowPlaying => {
                if is_action_key(&key) && self.last_action_at.elapsed() < Duration::from_millis(150)
                {
                    return Ok(false);
                }
                match key.code {
                    KeyCode::Char(' ') => {
                        if let Err(e) = apple_script::play_pause() {
                            self.err = Some(e.to_string());
                        }
                        self.set_feedback("play / pause");
                        self.last_action_at = Instant::now();
                        spawn_status(tx.clone());
                    }
                    KeyCode::Char('n') | KeyCode::Right => {
                        if let Err(e) = apple_script::next_track() {
                            self.err = Some(e.to_string());
                        }
                        self.set_feedback("next track");
                        self.last_action_at = Instant::now();
                        spawn_status(tx.clone());
                    }
                    KeyCode::Char('p') | KeyCode::Left => {
                        if let Err(e) = apple_script::previous_track() {
                            self.err = Some(e.to_string());
                        }
                        self.set_feedback("previous track");
                        self.last_action_at = Instant::now();
                        spawn_status(tx.clone());
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Up => {
                        if self.status.volume < 100 {
                            let mut new_vol = self.status.volume + 10;
                            if new_vol > 100 {
                                new_vol = 100;
                            }
                            if let Err(e) = apple_script::set_volume(new_vol) {
                                self.err = Some(e.to_string());
                            }
                            self.status.volume = new_vol;
                            self.set_feedback(format!("volume {}%", self.status.volume));
                            self.last_action_at = Instant::now();
                        }
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Down => {
                        if self.status.volume > 0 {
                            let mut new_vol = self.status.volume - 10;
                            if new_vol < 0 {
                                new_vol = 0;
                            }
                            if let Err(e) = apple_script::set_volume(new_vol) {
                                self.err = Some(e.to_string());
                            }
                            self.status.volume = new_vol;
                            self.set_feedback(format!("volume {}%", self.status.volume));
                            self.last_action_at = Instant::now();
                        }
                    }
                    _ => {}
                }
            }
            Section::Playlists | Section::Queue | Section::Liked | Section::Search => {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if self.selected_index + 1 < self.list_len() {
                            self.selected_index += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    KeyCode::Enter => self.handle_enter(tx),
                    _ => {}
                }
            }
            Section::Devices | Section::PlaylistTracks => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.selected_index + 1 < self.list_len() {
                        self.selected_index += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                }
                KeyCode::Enter => self.handle_enter(tx),
                _ => {}
            },
            _ => {}
        }

        Ok(false)
    }

    fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::Status(status) => {
                self.status = status;
                self.last_status_fetch = Instant::now();
            }
            AppMessage::Playlists(pl) => {
                self.playlists = pl;
                self.set_feedback(format!("loaded {} playlists", self.playlists.len()));
            }
            AppMessage::Queue(q) => {
                self.queue = Some(q);
                let count = self
                    .queue
                    .as_ref()
                    .map(|item| item.queue.len())
                    .unwrap_or(0);
                self.set_feedback(format!("loaded {} queued tracks", count));
            }
            AppMessage::Liked(tracks) => {
                self.liked_songs = tracks;
                self.set_feedback(format!("loaded {} liked tracks", self.liked_songs.len()));
            }
            AppMessage::PlaylistTracks(tracks) => {
                self.playlist_tracks = tracks;
                self.set_feedback(format!("loaded {} tracks", self.playlist_tracks.len()));
            }
            AppMessage::Playback(pb) => {
                self.playback = Some(pb);
                self.last_api_fetch = Instant::now();
            }
            AppMessage::Devices(devs) => {
                self.devices = devs;
                self.set_feedback(format!("loaded {} devices", self.devices.len()));
            }
            AppMessage::Search(res) => self.build_search_items(res),
            AppMessage::Update(result, manual) => {
                if result.update_available {
                    self.set_feedback(format!("upgrade available: {}", result.latest));
                } else if manual {
                    self.set_feedback("you are up to date");
                }
                self.update_result = Some(result);
                self.update_err = None;
            }
            AppMessage::UpdateErr(err, manual) => {
                if manual {
                    self.update_err = Some(err.clone());
                    self.set_feedback("update check failed");
                }
            }
            AppMessage::AuthComplete => {
                self.authenticated = true;
                self.auth_in_progress = false;
                self.set_section(Section::NowPlaying);
                self.set_feedback("auth complete");
            }
            AppMessage::Error(err, not_auth) => {
                if not_auth {
                    self.authenticated = false;
                    self.set_section(Section::Auth);
                }
                self.set_feedback("request failed");
                self.err = Some(err);
            }
        }
    }

    fn on_tick(&mut self, tx: &Sender<AppMessage>) {
        self.animation_tick = self.animation_tick.wrapping_add(1);
        if self.feedback.is_some() && Instant::now() > self.feedback_until {
            self.feedback = None;
        }

        if self.last_status_fetch.elapsed() > Duration::from_secs(1) {
            spawn_status(tx.clone());
        }
        if self.authenticated && self.last_api_fetch.elapsed() > Duration::from_secs(5) {
            if let Some(client) = self.api_client.clone() {
                spawn_playback(tx.clone(), client);
            }
        }
    }
}

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

fn spawn_status(tx: Sender<AppMessage>) {
    thread::spawn(move || match apple_script::get_status() {
        Ok(status) => {
            let _ = tx.send(AppMessage::Status(status));
        }
        Err(err) => {
            let _ = tx.send(AppMessage::Error(err.to_string(), false));
        }
    });
}

fn spawn_playlists(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        match client.get_playlists() {
            Ok(pl) => {
                let _ = tx.send(AppMessage::Playlists(pl));
            }
            Err(err) => {
                let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
            }
        }
    });
}

fn spawn_queue(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        match client.get_queue() {
            Ok(q) => {
                let _ = tx.send(AppMessage::Queue(q));
            }
            Err(err) => {
                let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
            }
        }
    });
}

fn spawn_liked(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        match client.get_liked_songs() {
            Ok(tracks) => {
                let _ = tx.send(AppMessage::Liked(tracks));
            }
            Err(err) => {
                let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
            }
        }
    });
}

fn spawn_playlist_tracks(
    tx: Sender<AppMessage>,
    client: Arc<Mutex<APIClient>>,
    playlist_id: String,
) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        match client.get_playlist_tracks(&playlist_id) {
            Ok(tracks) => {
                let _ = tx.send(AppMessage::PlaylistTracks(tracks));
            }
            Err(err) => {
                let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
            }
        }
    });
}

fn spawn_playback(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        match client.get_playback_state() {
            Ok(state) => {
                let _ = tx.send(AppMessage::Playback(state));
            }
            Err(err) => {
                let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
            }
        }
    });
}

fn spawn_devices(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        match client.get_devices() {
            Ok(devs) => {
                let _ = tx.send(AppMessage::Devices(devs));
            }
            Err(err) => {
                let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
            }
        }
    });
}

fn spawn_search(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>, query: String) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        match client.search(&query) {
            Ok(results) => {
                let _ = tx.send(AppMessage::Search(results));
            }
            Err(err) => {
                let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
            }
        }
    });
}

fn spawn_start_playback(
    tx: Sender<AppMessage>,
    client: Arc<Mutex<APIClient>>,
    uris: Vec<String>,
    context_uri: String,
) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        let result = if !uris.is_empty() {
            client.start_playback(&uris, "")
        } else {
            client.start_playback(&[], &context_uri)
        };
        if let Err(err) = result {
            let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
        } else {
            let _ = tx.send(AppMessage::Playback(
                client.get_playback_state().unwrap_or_default(),
            ));
        }
    });
}

fn spawn_transfer_device(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>, device_id: String) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        if let Err(err) = client.transfer_playback(&device_id) {
            let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
        } else {
            let _ = tx.send(AppMessage::Playback(
                client.get_playback_state().unwrap_or_default(),
            ));
        }
    });
}

fn spawn_shuffle(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>, state: bool) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        if let Err(err) = client.set_shuffle(state) {
            let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
        } else {
            let _ = tx.send(AppMessage::Playback(
                client.get_playback_state().unwrap_or_default(),
            ));
        }
    });
}

fn spawn_repeat(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>, state: String) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        if let Err(err) = client.set_repeat(&state) {
            let _ = tx.send(AppMessage::Error(err.to_string(), is_not_auth(&err)));
        } else {
            let _ = tx.send(AppMessage::Playback(
                client.get_playback_state().unwrap_or_default(),
            ));
        }
    });
}

fn spawn_update_check(tx: Sender<AppMessage>, version: String, force: bool, manual: bool) {
    thread::spawn(move || match update::check(&version, force) {
        Ok(result) => {
            let _ = tx.send(AppMessage::Update(result, manual));
        }
        Err(err) => {
            let _ = tx.send(AppMessage::UpdateErr(err.to_string(), manual));
        }
    });
}

fn spawn_auth(
    tx: Sender<AppMessage>,
    client: Arc<Mutex<APIClient>>,
    state: String,
    verifier: String,
) {
    thread::spawn(move || {
        let mut client = client.lock().unwrap();
        let result = client.start_auth_server(&state, &verifier);
        match result {
            Ok(_) => {
                let _ = tx.send(AppMessage::AuthComplete);
            }
            Err(err) => {
                let _ = tx.send(AppMessage::Error(err.to_string(), false));
            }
        }
    });
}

fn is_not_auth(err: &ApiError) -> bool {
    matches!(err, ApiError::NotAuthenticated(_))
}

fn open_browser(url: &str) -> Result<()> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .context("failed to open browser")?;
    Ok(())
}

fn is_action_key(key: &KeyEvent) -> bool {
    matches!(
        key.code,
        KeyCode::Char(' ')
            | KeyCode::Char('n')
            | KeyCode::Char('p')
            | KeyCode::Right
            | KeyCode::Left
            | KeyCode::Char('+')
            | KeyCode::Char('=')
            | KeyCode::Up
            | KeyCode::Char('-')
            | KeyCode::Char('_')
            | KeyCode::Down
    )
}

fn next_repeat_state(p: Option<&PlaybackState>) -> String {
    match p.map(|p| p.repeat_state.as_str()) {
        Some("off") | None => "context".into(),
        Some("context") => "track".into(),
        _ => "off".into(),
    }
}

pub fn section_label(section: Section) -> &'static str {
    match section {
        Section::NowPlaying => "now playing",
        Section::Playlists => "playlists",
        Section::Queue => "queue",
        Section::Liked => "liked",
        Section::Search => "search",
        Section::Devices => "devices",
        Section::PlaylistTracks => "playlist tracks",
        Section::Setup => "setup",
        Section::Help => "help",
        Section::Auth => "auth",
    }
}

pub fn paginate(total: usize, selected: usize, page_size: usize) -> (usize, usize, usize, usize) {
    if total == 0 {
        return (0, 0, 0, 0);
    }
    let page_size = if page_size == 0 { total } else { page_size };
    let selected = selected.min(total.saturating_sub(1));
    let start = if selected >= page_size {
        selected - page_size + 1
    } else {
        0
    };
    let end = (start + page_size).min(total);
    let pages = total.div_ceil(page_size);
    let page = (selected / page_size) + 1;
    (start, end, page, pages)
}

pub fn list_page_size(height: u16) -> usize {
    if height <= 6 {
        return 5;
    }
    let size = height.saturating_sub(6);
    if size < 5 {
        5
    } else {
        size as usize
    }
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    if max_len == 1 {
        return "…".to_string();
    }
    if s.chars().count() <= max_len {
        return s.to_string();
    }

    let mut out = String::with_capacity(max_len);
    out.extend(s.chars().take(max_len - 1));
    out.push('…');
    out
}

pub fn format_time(seconds: f64) -> String {
    let m = (seconds as u64) / 60;
    let s = (seconds as u64) % 60;
    format!("{}:{:02}", m, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginate_uses_selected_page() {
        let (_start, _end, page, pages) = paginate(20, 16, 5);
        assert_eq!(page, 4);
        assert_eq!(pages, 4);
    }

    #[test]
    fn list_page_size_minimum() {
        assert_eq!(list_page_size(0), 5);
        assert_eq!(list_page_size(4), 5);
    }

    #[test]
    fn truncate_handles_multibyte_characters() {
        let title = "2000'S NAIJA HITS 🇳🇬🇳🇬";
        let got = truncate(title, 21);
        assert_eq!(got, "2000'S NAIJA HITS 🇳🇬…");
    }
}
