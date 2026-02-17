//! Async message handling and periodic state updates from background effects.

use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::spotify_api;

use super::effects::{spawn_playback, spawn_status};
use super::state::{App, AppMessage, SearchItem, Section};

impl App {
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

    pub(super) fn handle_message(&mut self, msg: AppMessage) {
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

    pub(super) fn on_tick(&mut self, tx: &Sender<AppMessage>) {
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
