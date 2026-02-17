use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::apple_script;
use crate::spotify_api::{APIClient, ApiError, PlaybackState};
use crate::update;

use super::state::AppMessage;

pub(super) fn spawn_status(tx: Sender<AppMessage>) {
    thread::spawn(move || match apple_script::get_status() {
        Ok(status) => {
            let _ = tx.send(AppMessage::Status(status));
        }
        Err(err) => {
            let _ = tx.send(AppMessage::Error(err.to_string(), false));
        }
    });
}

pub(super) fn spawn_playlists(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
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

pub(super) fn spawn_queue(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
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

pub(super) fn spawn_liked(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
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

pub(super) fn spawn_playlist_tracks(
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

pub(super) fn spawn_playback(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
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

pub(super) fn spawn_devices(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>) {
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

pub(super) fn spawn_search(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>, query: String) {
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

pub(super) fn spawn_start_playback(
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

pub(super) fn spawn_transfer_device(
    tx: Sender<AppMessage>,
    client: Arc<Mutex<APIClient>>,
    device_id: String,
) {
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

pub(super) fn spawn_shuffle(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>, state: bool) {
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

pub(super) fn spawn_repeat(tx: Sender<AppMessage>, client: Arc<Mutex<APIClient>>, state: String) {
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

pub(super) fn spawn_update_check(
    tx: Sender<AppMessage>,
    version: String,
    force: bool,
    manual: bool,
) {
    thread::spawn(move || match update::check(&version, force) {
        Ok(result) => {
            let _ = tx.send(AppMessage::Update(result, manual));
        }
        Err(err) => {
            let _ = tx.send(AppMessage::UpdateErr(err.to_string(), manual));
        }
    });
}

pub(super) fn spawn_auth(
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

pub(super) fn open_browser(url: &str) -> Result<()> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .context("failed to open browser")?;
    Ok(())
}

pub(super) fn is_action_key(key: &KeyEvent) -> bool {
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

pub(super) fn next_repeat_state(p: Option<&PlaybackState>) -> String {
    match p.map(|p| p.repeat_state.as_str()) {
        Some("off") | None => "context".into(),
        Some("context") => "track".into(),
        _ => "off".into(),
    }
}

fn is_not_auth(err: &ApiError) -> bool {
    matches!(err, ApiError::NotAuthenticated(_))
}
