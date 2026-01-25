# Spotify TUI (Rust)

A brutalist terminal interface for controlling Spotify on macOS, rewritten in Rust + ratatui.

```
███████╗██████╗  ██████╗ ████████╗
██╔════╝██╔══██╗██╔═══██╗╚══██╔══╝
███████╗██████╔╝██║   ██║   ██║   
╚════██║██╔═══╝ ██║   ██║   ██║   
███████║██║     ╚██████╔╝   ██║   
╚══════╝╚═╝      ╚═════╝    ╚═╝   
```

## Features

- **Now Playing** — Control playback, see track info and progress (AppleScript)
- **Playlists** — Browse your playlists (requires OAuth)
- **Queue** — See what's coming up next
- **Liked Songs** — Browse your saved tracks
- **Search** — Tracks, playlists, and artists (requires OAuth)
- **Devices** — Switch playback device (requires OAuth)
- **Brutalist Design** — Heavy borders, ALL CAPS, terminal green

## Installation

### From Source
```bash
cargo build --release
./target/release/spotify-tui
```

### Homebrew
```bash
brew tap Wondr-design/tap
brew install spotify-tui
```

## Usage

```bash
spotify-tui
```

## OAuth Setup (Optional)

To access Playlists, Queue, Liked Songs, Search, and Devices:

1. Run `spotify-tui`
2. Press `c` to open the in-app setup guide (or run `spotify-tui --setup`)
3. Follow the prompts to paste your Client ID and authenticate

## Requirements

- macOS
- Spotify Desktop app installed
- Rust 1.70+ (for building from source)

## License

MIT
# Spotify_tui-rust
