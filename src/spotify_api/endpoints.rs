use super::client::APIClient;
use super::error::ApiError;
use super::types::*;
use reqwest::Method;
use serde_json::json;
use url::form_urlencoded::byte_serialize;

impl APIClient {
    pub fn get_playlists(&mut self) -> Result<Vec<Playlist>, ApiError> {
        let data = self.api_request(Method::GET, "/me/playlists?limit=20")?;
        let resp: PlaylistsResponse =
            serde_json::from_str(&data).map_err(|e| ApiError::Request(e.to_string()))?;
        Ok(resp.items)
    }

    pub fn get_queue(&mut self) -> Result<QueueResponse, ApiError> {
        let data = self.api_request(Method::GET, "/me/player/queue")?;
        let resp: QueueResponse =
            serde_json::from_str(&data).map_err(|e| ApiError::Request(e.to_string()))?;
        Ok(resp)
    }

    pub fn get_liked_songs(&mut self) -> Result<Vec<Track>, ApiError> {
        let data = self.api_request(Method::GET, "/me/tracks?limit=20")?;
        let resp: LikedTracksResponse =
            serde_json::from_str(&data).map_err(|e| ApiError::Request(e.to_string()))?;
        Ok(resp.items.into_iter().map(|i| i.track).collect())
    }

    pub fn get_playlist_tracks(&mut self, playlist_id: &str) -> Result<Vec<Track>, ApiError> {
        let data = self.api_request(
            Method::GET,
            &format!("/playlists/{}/tracks?limit=50", playlist_id),
        )?;
        let resp: PlaylistTracksResponse =
            serde_json::from_str(&data).map_err(|e| ApiError::Request(e.to_string()))?;
        Ok(resp.items.into_iter().map(|i| i.track).collect())
    }

    pub fn get_playback_state(&mut self) -> Result<PlaybackState, ApiError> {
        let data = self.api_request(Method::GET, "/me/player")?;
        let resp: PlaybackState =
            serde_json::from_str(&data).map_err(|e| ApiError::Request(e.to_string()))?;
        Ok(resp)
    }

    pub fn get_devices(&mut self) -> Result<Vec<Device>, ApiError> {
        let data = self.api_request(Method::GET, "/me/player/devices")?;
        let resp: DevicesResponse =
            serde_json::from_str(&data).map_err(|e| ApiError::Request(e.to_string()))?;
        Ok(resp.devices)
    }

    pub fn transfer_playback(&mut self, device_id: &str) -> Result<(), ApiError> {
        let body = json!({"device_ids": [device_id], "play": true});
        self.api_request_with_body(Method::PUT, "/me/player", body)
    }

    pub fn set_shuffle(&mut self, state: bool) -> Result<(), ApiError> {
        let endpoint = format!("/me/player/shuffle?state={}", state);
        self.api_request(Method::PUT, &endpoint).map(|_| ())
    }

    pub fn set_repeat(&mut self, state: &str) -> Result<(), ApiError> {
        let enc: String = byte_serialize(state.as_bytes()).collect();
        let endpoint = format!("/me/player/repeat?state={}", enc);
        self.api_request(Method::PUT, &endpoint).map(|_| ())
    }

    pub fn start_playback(&mut self, uris: &[String], context_uri: &str) -> Result<(), ApiError> {
        let mut body = serde_json::Map::new();
        if !uris.is_empty() {
            body.insert("uris".to_string(), json!(uris));
        }
        if !context_uri.is_empty() {
            body.insert("context_uri".to_string(), json!(context_uri));
        }
        self.api_request_with_body(
            Method::PUT,
            "/me/player/play",
            serde_json::Value::Object(body),
        )
    }

    pub fn search(&mut self, query: &str) -> Result<SearchResults, ApiError> {
        let enc: String = byte_serialize(query.as_bytes()).collect();
        let endpoint = format!("/search?type=track,playlist,artist&limit=10&q={}", enc);
        let data = self.api_request(Method::GET, &endpoint)?;
        let resp: SearchResults =
            serde_json::from_str(&data).map_err(|e| ApiError::Request(e.to_string()))?;
        Ok(resp)
    }
}
