use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub is_running: bool,
    pub is_playing: bool,
    pub track: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub position: f64,
    pub volume: i32,
    pub album_art_url: String,
}

fn run_script(script: &str) -> Result<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .context("failed to run osascript")?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout.trim().to_string())
}

pub fn check_running() -> Result<bool> {
    let out = run_script("application \"Spotify\" is running")?;
    Ok(out.trim() == "true")
}

pub fn get_status() -> Result<Status> {
    let running = check_running()?;
    if !running {
        return Ok(Status {
            is_running: false,
            ..Default::default()
        });
    }

    let script = r#"
    tell application "Spotify"
        set tName to name of current track
        set tArtist to artist of current track
        set tAlbum to album of current track
        set tDur to duration of current track
        set tPos to player position
        set pState to player state
        set tVol to sound volume
        set tUrl to artwork url of current track
        return tName & "|||" & tArtist & "|||" & tAlbum & "|||" & tDur & "|||" & tPos & "|||" & pState & "|||" & tVol & "|||" & tUrl
    end tell
    "#;

    let out = match run_script(script) {
        Ok(v) => v,
        Err(_) => {
            return Ok(Status {
                is_running: true,
                ..Default::default()
            })
        }
    };

    let parts: Vec<&str> = out.split("|||").collect();
    if parts.len() < 8 {
        return Ok(Status {
            is_running: true,
            ..Default::default()
        });
    }

    let dur_ms: f64 = parts[3].parse().unwrap_or(0.0);
    let pos: f64 = parts[4].parse().unwrap_or(0.0);
    let vol: i32 = parts[6].parse().unwrap_or(0);

    Ok(Status {
        is_running: true,
        track: parts[0].to_string(),
        artist: parts[1].to_string(),
        album: parts[2].to_string(),
        duration: dur_ms / 1000.0,
        position: pos,
        is_playing: parts[5] == "playing",
        volume: vol,
        album_art_url: parts[7].to_string(),
    })
}

pub fn play_pause() -> Result<()> {
    run_script("tell application \"Spotify\" to playpause")?;
    Ok(())
}

pub fn next_track() -> Result<()> {
    run_script("tell application \"Spotify\" to next track")?;
    Ok(())
}

pub fn previous_track() -> Result<()> {
    run_script("tell application \"Spotify\" to previous track")?;
    Ok(())
}

pub fn set_volume(vol: i32) -> Result<()> {
    let script = format!(
        "tell application \"Spotify\" to set sound volume to {}",
        vol
    );
    run_script(&script)?;
    Ok(())
}
