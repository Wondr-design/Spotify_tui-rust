use serde::{Deserialize, Deserializer, Serialize};

fn de_string_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Artist {
    #[serde(default, deserialize_with = "de_string_or_default")]
    pub name: String,
    #[serde(default)]
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Album {
    #[serde(default, deserialize_with = "de_string_or_default")]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Track {
    #[serde(default, deserialize_with = "de_string_or_default")]
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub album: Album,
}

impl Track {
    pub fn first_artist_name(&self) -> String {
        if let Some(artist) = self.artists.first() {
            artist.name.clone()
        } else {
            String::new()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaylistTracks {
    #[serde(default)]
    pub total: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Playlist {
    #[serde(default, deserialize_with = "de_string_or_default")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_or_default")]
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub tracks: PlaylistTracks,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Device {
    #[serde(default, deserialize_with = "de_string_or_default")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_or_default")]
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
    pub items: Vec<Option<T>>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_results_tolerate_null_playlist_items() {
        let payload = r#"{
            "tracks": { "items": [] },
            "playlists": {
                "items": [
                    null,
                    {
                        "id": "37i9dQZF1DX4dyzvuaRJ0n",
                        "name": "Naija Mix",
                        "uri": "spotify:playlist:37i9dQZF1DX4dyzvuaRJ0n",
                        "tracks": { "total": 42 }
                    }
                ]
            },
            "artists": { "items": [] }
        }"#;

        let parsed: SearchResults = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.playlists.items.len(), 2);
        assert!(parsed.playlists.items[0].is_none());
        assert_eq!(
            parsed.playlists.items[1].as_ref().unwrap().name,
            "Naija Mix"
        );
    }
}
