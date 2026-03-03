# ♪ Audiplayer

An agnostic, modular Terminal User Interface (TUI) media player written in Rust.

## Overview

Audiplayer is a provider-agnostic music player TUI that decouples the user interface from the audio engine and the content providers. It uses **Ratatui** for a performant, Vim-like terminal interface, **mpv** as a headless audio engine, and a modular plugin system to fetch metadata and streams from various sources like YouTube and Bandcamp.

## Summary

The project follows a micro architecture where the core orchestrates communication between the TUI, the audio player, and the plugins. Everything flows through a single `Action` language, making the state predictable and easy to reason about.

## Features

- **Agnostic Provider System**: Easily switch between Bandcamp and YouTube using tabs.
- **Vim-like Experience**:
  - Modes: `NORMAL`, `INSERT` (search), `COMMAND` (colon commands).
  - Navigation: `hjkl` for lists, `gg`/`G` for top/bottom, `Ctrl+h/l` for panel focus.
- **Performance Optimized**:
  - Background pre-fetching of stream URLs using `yt-dlp` while scrolling.
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
  - `bandcamp.rs`: Bandcamp bridge using autocomplete API.
  - `youtube.rs`: YouTube bridge using `yt-dlp` discovery.
- `src/tui/`: Terminal User Interface.
  - `ui.rs`: Pure functional rendering logic.
  - `events.rs`: Blocking input listener thread.

## Installation & Usage

1. Ensure `mpv` and `yt-dlp` are installed on your system.
2. Run with `cargo run`.
3. Commands:
    - `:q` to quit.
    - `:l` to toggle logs.
    - `i` to enter search mode.
    - `Tab` to switch providers.
