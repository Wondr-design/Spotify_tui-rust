use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config;

const REPO_OWNER: &str = "Wondr-design";
const REPO_NAME: &str = "Spotify_tui-rust";
const GITHUB_API: &str = "https://api.github.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultInfo {
    pub current: String,
    pub latest: String,
    pub update_available: bool,
    pub checked_at: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Cache {
    latest: String,
    checked_at: u64,
}

pub fn check(current_version: &str, force: bool) -> Result<ResultInfo> {
    let now = now_ts();
    if !force {
        if let Ok(cache) = read_cache() {
            if !cache.latest.is_empty() && now.saturating_sub(cache.checked_at) < 86400 {
                return Ok(build_result(
                    current_version,
                    &cache.latest,
                    cache.checked_at,
                ));
            }
        }
    }

    let latest = fetch_latest()?;
    let res = build_result(current_version, &latest, now);
    let _ = write_cache(Cache {
        latest,
        checked_at: now,
    });
    Ok(res)
}

fn build_result(current: &str, latest: &str, checked_at: u64) -> ResultInfo {
    let mut res = ResultInfo {
        current: current.to_string(),
        latest: latest.to_string(),
        update_available: false,
        checked_at,
        message: String::new(),
    };

    if is_newer(latest, current) {
        res.update_available = true;
        res.message = format!("Update available: {} (brew upgrade spotify-tui-rs)", latest);
    }
    res
}

fn fetch_latest() -> Result<String> {
    let url = format!(
        "{}/repos/{}/{}/releases/latest",
        GITHUB_API, REPO_OWNER, REPO_NAME
    );
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "spotify-tui-rs")
        .send()
        .context("failed to call GitHub API")?;

    if !resp.status().is_success() {
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("update check failed: {}", text);
    }

    #[derive(Deserialize)]
    struct Payload {
        tag_name: String,
    }
    let payload: Payload = resp.json().context("invalid GitHub response")?;
    Ok(payload.tag_name)
}

fn is_newer(latest: &str, current: &str) -> bool {
    let latest = normalize(latest);
    let current = normalize(current);
    if latest.is_empty() || current.is_empty() {
        return false;
    }
    if current == "dev" {
        return false;
    }
    let lv = parse_semver(&latest);
    let cv = parse_semver(&current);
    if lv.is_none() || cv.is_none() {
        return false;
    }
    let lv = lv.unwrap();
    let cv = cv.unwrap();
    for i in 0..3 {
        if lv[i] > cv[i] {
            return true;
        }
        if lv[i] < cv[i] {
            return false;
        }
    }
    false
}

fn normalize(v: &str) -> String {
    let v = v.trim().trim_start_matches('v');
    let mut out = v.to_string();
    if let Some(i) = out.find(['-', '+']) {
        out.truncate(i);
    }
    out
}

fn parse_semver(v: &str) -> Option<[u64; 3]> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let mut out = [0u64; 3];
    for (i, p) in parts.iter().take(3).enumerate() {
        out[i] = p.parse().ok()?;
    }
    Some(out)
}

fn cache_path() -> PathBuf {
    config::config_dir().join("update.json")
}

fn read_cache() -> Result<Cache> {
    let path = cache_path();
    let data = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let cache = serde_json::from_slice(&data).context("invalid cache")?;
    Ok(cache)
}

fn write_cache(cache: Cache) -> Result<()> {
    let path = cache_path();
    let fallback = PathBuf::from(".");
    let dir = path.parent().unwrap_or(fallback.as_path());
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let data = serde_json::to_vec_pretty(&cache).context("failed to serialize cache")?;
    fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_compare() {
        assert!(is_newer("v1.2.4", "1.2.3"));
        assert!(!is_newer("v1.2.3", "1.2.3"));
        assert!(!is_newer("v1.2.3", "1.3.0"));
    }
}
