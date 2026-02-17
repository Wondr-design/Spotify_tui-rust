use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::apple_script;
use crate::config;
use crate::spotify_api::{self, APIClient, Device, PlaybackState, Playlist, QueueResponse, Track};
use crate::update;

use super::text::section_label;

pub(super) const DASHBOARD_URL: &str = "https://developer.spotify.com/dashboard";
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
pub(super) enum AppMessage {
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
    pub(super) feedback_until: Instant,

    pub api_client: Option<Arc<Mutex<APIClient>>>,
    pub authenticated: bool,
    pub auth_url: String,
    pub auth_in_progress: bool,

    pub width: u16,
    pub height: u16,

    pub(super) last_action_at: Instant,
    pub(super) last_status_fetch: Instant,
    pub(super) last_api_fetch: Instant,
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

    pub(super) fn update_checks_enabled() -> bool {
        let val = std::env::var("SPOTIFY_TUI_NO_UPDATE_CHECK").unwrap_or_default();
        matches!(
            val.trim().to_lowercase().as_str(),
            "" | "0" | "false" | "no"
        )
    }

    pub(super) fn ensure_auth_required(&mut self) -> bool {
        if self.authenticated {
            return false;
        }
        self.prev_section = self.section;
        self.section = Section::Auth;
        self.auth_in_progress = false;
        self.set_feedback("auth required");
        true
    }

    pub(super) fn set_feedback<S: Into<String>>(&mut self, message: S) {
        self.feedback = Some(message.into());
        self.feedback_until = Instant::now() + FEEDBACK_TIMEOUT;
    }

    pub(super) fn bump_setup_hue(&mut self, step: i16) {
        let mut hue = self.setup_hue as i16 + step;
        while hue < 0 {
            hue += 360;
        }
        self.setup_hue = (hue as u16) % 360;
        self.accent_hue = self.setup_hue;
        self.set_feedback(format!("accent hue {}", self.setup_hue));
    }

    pub(super) fn set_section(&mut self, section: Section) {
        self.prev_section = self.section;
        self.section = section;
        self.selected_index = 0;
    }

    pub(super) fn navigate_to(&mut self, section: Section) {
        self.set_section(section);
        self.set_feedback(format!("section: {}", section_label(section)));
    }

    pub(super) fn list_len(&self) -> usize {
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
}
