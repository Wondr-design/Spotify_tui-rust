use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Artist {
    pub name: String,
    #[serde(default)]
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Album {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Track {
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub album: Album,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaylistTracks {
    #[serde(default)]
    pub total: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub tracks: PlaylistTracks,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Device {
    pub id: String,
    pub name: String,
    #[serde(default, rename = "type")]
    pub device_type: String,
    #[serde(default, rename = "is_active")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaybackState {
    #[serde(default)]
    pub device: Device,
    #[serde(default, rename = "is_playing")]
    pub is_playing: bool,
    #[serde(default, rename = "shuffle_state")]
    pub shuffle_state: bool,
    #[serde(default, rename = "repeat_state")]
    pub repeat_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueResponse {
    #[serde(default, rename = "currently_playing")]
    pub currently_playing: Track,
    #[serde(default)]
    pub queue: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchItems<T> {
    #[serde(default)]
    pub items: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResults {
    #[serde(default)]
    pub tracks: SearchItems<Track>,
    #[serde(default)]
    pub playlists: SearchItems<Playlist>,
    #[serde(default)]
    pub artists: SearchItems<Artist>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaylistsResponse {
    #[serde(default)]
    pub items: Vec<Playlist>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LikedTrackItem {
    pub track: Track,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LikedTracksResponse {
    #[serde(default)]
    pub items: Vec<LikedTrackItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaylistTrackItem {
    pub track: Track,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaylistTracksResponse {
    #[serde(default)]
    pub items: Vec<PlaylistTrackItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevicesResponse {
    #[serde(default)]
    pub devices: Vec<Device>,
}
