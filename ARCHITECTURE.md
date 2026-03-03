# Architecture Documentation

This document details the architectural patterns, component responsibilities, and communication flow of the Audiplayer project.

## Core Patterns

### 1. Unidirectional Data Flow

Audiplayer follows a strict unidirectional data flow similar to Elm or Redux:

- **State**: The `AppState` is the single source of truth.
- **Actions**: The `Action` enum is the only way to trigger changes.
- **Reducer**: The `handle_action` function in `event_loop.rs` is the only place where state is mutated.
- **View**: The TUI reads the state and renders it. It cannot mutate state directly.

### 2. Micro Architecture

The "core" (`core/event_loop.rs`) is small and agnostic. It doesn't know how to play audio or how to search YouTube. It only knows how to route `Action`s to subsystems and update the state.

### 3. Asynchronous Subsystems

Every major subsystem (`player`, `plugins`, `tui` events) runs in its own task or thread. They communicate with the core via an `mpsc` channel. This ensures the UI stays responsive even when a plugin is waiting for a slow network request.

---

## Folder Responsibilities

### `src/core/`

- **Responsibility**: State management and orchestration.
- **Key Concepts**:
  - `action.rs`: The "API" of the app. If a behavior isn't here, it doesn't exist.
  - `state.rs`: Holds search results, playback status, logs, and UI modes. It uses a `HashMap<String, SearchState>` to keep search history alive per provider.
  - `event_loop.rs`: The heart. It selects on incoming actions and performs "side effects" (like spawning `yt-dlp` or sending a command to `mpv`).

### `src/player/`

- **Responsibility**: Audio playback and engine state.
- **Key Concepts**:
  - `Player` Trait: Defines `play`, `pause`, `resume`, etc.
  - `mpv.rs`:
    - Spawns a headless `mpv` process.
    - Connects via **Unix Domain Sockets (IPC)** for control.
    - **Terminal Monitoring**: Captures `stdout` byte-by-byte to handle `
` (carriage return) progress lines without blocking.
    - Ensures clean process cleanup on exit.

### `src/plugins/`

- **Responsibility**: Content discovery and stream resolution.
- **Key Concepts**:
  - `Provider` Trait: Contract for `search` and `get_stream_url`.
  - `PluginManager`:
    - **Semaphore**: Limits concurrent `yt-dlp` calls (currently 3) to prevent `429 Too Many Requests` errors.
    - **Preloading**: Triggered by the core as the user scrolls. It resolves stream URLs in the background and caches them in the `Track` metadata.
  - **Resolution**: Uses `yt-dlp --get-url --get-duration` to fetch high-quality stream links and metadata simultaneously.

### `src/tui/`

- **Responsibility**: User interaction and rendering.
- **Key Concepts**:
  - `events.rs`: A dedicated blocking thread that reads `crossterm` events and maps them to `Action::Key`.
  - `ui.rs`:
    - **Pure Rendering**: Takes a reference to `AppState` and draws the screen.
    - **Split Layout**: Dynamically adjusts the layout when the log panel is toggled.
    - **Centered UI**: Uses `Alignment::Center` for the main app header.

---

## Communication Flow

1. **Input**: User enter in Insert mode, search for a track and press `Enter` in search.
2. **Event**: `tui/events.rs` sends `Action::Key(Enter)`.
3. **Core**: `event_loop.rs` receives it, sees `Mode::Insert`, and triggers `Action::SearchSubmit`.
4. **Action**: `handle_action` calls `PluginManager::handle_search`.
5. **Async**: `PluginManager` spawns a task, runs `yt-dlp --dump-json`, and eventually sends `Action::PluginResponse`.
6. **Update**: Core receives the response, updates `AppState.search_states`, and asks TUI to re-render.
7. **Preload**: As the user moves the cursor, core sends `resolve_stream_url` commands to start pre-fetching the next tracks immediately.

## Important Constraints to Maintain

- **No `eprintln!`**: Always use `Action::Log` to send errors to the log panel or redirect `stderr` to a file (`audiplayer.log`) to avoid TUI corruption.
- **Clone Arcs**: Always clone `Arc` handles (`player`, `plugins`) before moving them into `tokio::spawn` blocks in the event loop.
- **Mode Awareness**: Input handling must always check `state.mode` to ensure keys like `j/k` don't leak into the search buffer.
