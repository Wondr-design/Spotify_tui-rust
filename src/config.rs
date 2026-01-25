use crate::spotify_api::Token;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub client_id: String,
}

pub fn config_dir() -> PathBuf {
    if let Ok(override_dir) = env::var("SPOTIFY_TUI_CONFIG_DIR") {
        if !override_dir.trim().is_empty() {
            return PathBuf::from(override_dir);
        }
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".spotify-tui")
}

pub fn load_config() -> Result<Config> {
    let path = config_dir().join("config.json");
    let data = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let cfg = serde_json::from_slice(&data).context("failed to parse config.json")?;
    Ok(cfg)
}

pub fn save_config(cfg: &Config) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let path = dir.join("config.json");
    let data = serde_json::to_vec_pretty(cfg).context("failed to serialize config")?;
    fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn load_token() -> Result<Token> {
    let path = config_dir().join("token.json");
    let data = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let token = serde_json::from_slice(&data).context("failed to parse token.json")?;
    Ok(token)
}

pub fn save_token(token: &Token) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let path = dir.join("token.json");
    let data = serde_json::to_vec_pretty(token).context("failed to serialize token")?;
    fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spotify_api::Token;

    #[test]
    fn config_and_token_roundtrip() {
        let dir = std::env::temp_dir().join(format!("spotify-tui-test-{}", rand::random::<u64>()));
        std::env::set_var("SPOTIFY_TUI_CONFIG_DIR", &dir);

        let cfg = Config {
            client_id: "abc123".to_string(),
        };
        save_config(&cfg).unwrap();
        let loaded = load_config().unwrap();
        assert_eq!(loaded.client_id, cfg.client_id);

        let token = Token {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            expires_in: 3600,
            token_type: "Bearer".into(),
            scope: "scope".into(),
            expires_at: 123456,
        };
        save_token(&token).unwrap();
        let loaded_token = load_token().unwrap();
        assert_eq!(loaded_token.access_token, token.access_token);
    }
}
