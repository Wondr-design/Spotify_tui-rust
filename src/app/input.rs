use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config;
use crate::spotify_api::{self, APIClient};

use super::effects::{
    is_action_key, next_repeat_state, open_browser, spawn_auth, spawn_devices, spawn_liked,
    spawn_playlist_tracks, spawn_playlists, spawn_queue, spawn_repeat, spawn_search, spawn_shuffle,
    spawn_start_playback, spawn_transfer_device, spawn_update_check,
};
use super::state::{App, AppMessage, Section, DASHBOARD_URL};

impl App {
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

    pub(super) fn handle_key(&mut self, key: KeyEvent, tx: &Sender<AppMessage>) -> Result<bool> {
        if key.kind == KeyEventKind::Release {
            return Ok(false);
        }

        // section-specific handling
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

        // global keys
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

        // list controls
        match self.section {
            Section::NowPlaying => {
                if is_action_key(&key) && self.last_action_at.elapsed() < Duration::from_millis(150)
                {
                    return Ok(false);
                }
                match key.code {
                    KeyCode::Char(' ') => {
                        if let Err(e) = crate::apple_script::play_pause() {
                            self.err = Some(e.to_string());
                        }
                        self.set_feedback("play / pause");
                        self.last_action_at = std::time::Instant::now();
                        super::effects::spawn_status(tx.clone());
                    }
                    KeyCode::Char('n') | KeyCode::Right => {
                        if let Err(e) = crate::apple_script::next_track() {
                            self.err = Some(e.to_string());
                        }
                        self.set_feedback("next track");
                        self.last_action_at = std::time::Instant::now();
                        super::effects::spawn_status(tx.clone());
                    }
                    KeyCode::Char('p') | KeyCode::Left => {
                        if let Err(e) = crate::apple_script::previous_track() {
                            self.err = Some(e.to_string());
                        }
                        self.set_feedback("previous track");
                        self.last_action_at = std::time::Instant::now();
                        super::effects::spawn_status(tx.clone());
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Up => {
                        if self.status.volume < 100 {
                            let mut new_vol = self.status.volume + 10;
                            if new_vol > 100 {
                                new_vol = 100;
                            }
                            if let Err(e) = crate::apple_script::set_volume(new_vol) {
                                self.err = Some(e.to_string());
                            }
                            self.status.volume = new_vol;
                            self.set_feedback(format!("volume {}%", self.status.volume));
                            self.last_action_at = std::time::Instant::now();
                        }
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Down => {
                        if self.status.volume > 0 {
                            let mut new_vol = self.status.volume - 10;
                            if new_vol < 0 {
                                new_vol = 0;
                            }
                            if let Err(e) = crate::apple_script::set_volume(new_vol) {
                                self.err = Some(e.to_string());
                            }
                            self.status.volume = new_vol;
                            self.set_feedback(format!("volume {}%", self.status.volume));
                            self.last_action_at = std::time::Instant::now();
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
}
