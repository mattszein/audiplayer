# ♪ Audiplayer

An agnostic, modular Terminal User Interface (TUI) media player written in Rust.

## Overview

Hey, I'm Matt — this is my first vibe-coded app, born from late-night terminal sessions and a love for music. I built it to scratch my own itch and now I'm sharing it with the world.

Audiplayer is a provider-agnostic music player TUI that decouples the user interface from the audio engine and the content providers. It uses **Ratatui** for a performant, Vim-like terminal interface, **mpv** as a headless audio engine, and a modular plugin system to fetch metadata and streams from various sources like YouTube and Bandcamp.

## Requirements

- **Rust** (edition 2024)
- **mpv** — headless audio engine (controlled via IPC socket)
- **yt-dlp** — stream URL resolution and YouTube search (required for YouTube provider)

## Installation & Usage

1. Ensure `mpv` and `yt-dlp` are installed and available in `$PATH`.
2. Build with `cargo build --release` or run directly with `cargo run`.
3. Keybindings:

| Key | Mode | Action |
|-----|------|--------|
| `i` | Normal | Enter search (Insert) mode |
| `Esc` | Insert/Command | Return to Normal mode |
| `Enter` | Insert | Submit search |
| `Enter` | Normal | Play track / View album tracks |
| `Backspace` | Normal | Go back (e.g., from album to search) |
| `Space` | Normal | Play/Pause toggle |
| `j` / `k` / arrows | Normal | Navigate results |
| `gg` / `G` | Normal | Jump to first / last result |
| `Tab` / `Shift+Tab` | Normal | Cycle providers |
| `1` / `2` | Normal | Switch to Bandcamp / YouTube directly |
| `Ctrl+h` / `Ctrl+l` | Normal | Focus search / logs panel |
| `:q` | Command | Quit (or close logs if focused) |
| `:l` | Command | Toggle log panel |
| `Ctrl+c` | Any | Force quit |

## Features

- **Agnostic Provider System**: Easily switch between Bandcamp and YouTube using tabs.
- **Advanced Navigation**:
  - Search for albums, tracks, or artists.
  - **Drill-down**: Press Enter on an album to view its tracklist, and press Backspace to return to your previous search.
- **Provider-Specific Integrations**:
  - **Bandcamp**: Uses native API for ultra-fast search and stream resolution without external dependencies. Supports search filters (`@album`, `@track`, `@artist`).
  - **YouTube**: Powered by `yt-dlp` for discovery and high-quality stream extraction.
- **Vim-like Experience**:
  - Modes: `NORMAL`, `INSERT` (search), `COMMAND` (colon commands).
  - Navigation: `hjkl` / arrow keys for lists, `gg`/`G` for top/bottom, `Ctrl+h/l` or arrow keys for panel focus.
- **Performance Optimized**:
  - Background pre-fetching of stream URLs while scrolling.
  - Concurrency control via Semaphores to prevent rate-limiting (429 errors).
- **Audio Engine**:
  - Uses a persistent `mpv` instance controlled via IPC sockets.
  - Real-time progress monitoring via stdout/stderr capture.
- **Robust Logging**: Split-screen log view (`:l`) for real-time error tracking without TUI corruption.
- **High Quality**: Explicitly prioritizes the highest bitrate audio streams available.

## Directory Structure

- `src/main.rs`: Initialization, it calls the subsystems, and runs the event loop.
- `src/core/`: The brain of the application.
  - `action.rs`: Defines the single language (`Action` enum) all subsystems use.
  - `state.rs`: The single source of truth (`AppState`).
  - `event_loop.rs`: Orchestrates state mutation and side effects.
  - `mode.rs`: Defines interaction modes (Normal, Insert, Command).
- `src/player/`: Audio engine abstraction.
  - `mpv.rs`: Concrete implementation using a headless `mpv` process.
- `src/plugins/`: Content provider bridges.
  - `traits.rs`: The contract every provider must implement.
  - `bandcamp.rs`: Bandcamp bridge using native API for search and streaming.
  - `youtube.rs`: YouTube bridge using `yt-dlp` discovery.
- `src/tui/`: Terminal User Interface.
  - `ui.rs`: Pure functional rendering logic.
  - `events.rs`: Blocking input listener thread.

## License

MIT
