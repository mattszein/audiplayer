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
  - `state.rs`: Holds search results, playback status, logs, and UI modes. It uses a `HashMap<String, SearchState>` to keep search state alive per provider. The `SearchState` maintains a `history` stack and `breadcrumbs` to support drill-down navigation (e.g., Search -> Album).
  - `event_loop.rs`: The heart. It selects on incoming actions and performs "side effects" (like resolving streams or sending a command to `mpv`).

### `src/player/`

- **Responsibility**: Audio playback and engine state.
- **Key Concepts**:
  - `Player` Trait: Defines `play`, `pause`, `resume`, `stop`, and `seek`.
  - `mpv.rs`:
    - Spawns a headless `mpv` process.
    - Connects via **Unix Domain Sockets (IPC)** for control.
    - **Terminal Monitoring**: Captures `stdout` byte-by-byte to handle `\r` (carriage return) progress lines without blocking.
    - Ensures clean process cleanup on exit.

### `src/plugins/`

- **Responsibility**: Content discovery and stream resolution.
- **Key Concepts**:
  - `Provider` Trait: Contract requiring `id`, `display_name`, `capabilities`, `search`, and optionally `get_stream_url` and `get_album_tracks`.
  - `PluginManager`:
    - **Semaphore**: Limits concurrent `yt-dlp` calls (currently 3) to prevent `429 Too Many Requests` errors for providers that rely on it.
    - **Preloading**: Triggered by the core as the user scrolls. It resolves stream URLs for tracks in the background and caches them in the `Track` metadata.
  - **Resolution**: Tries to use the provider's native `get_stream_url` method first (e.g., Bandcamp). If that fails or is unsupported, it falls back to `yt-dlp --get-url --get-duration` (e.g., YouTube) to fetch high-quality stream links and metadata simultaneously.

### `src/tui/`

- **Responsibility**: User interaction and rendering.
- **Key Concepts**:
  - `events.rs`: A dedicated blocking thread that reads `crossterm` events and maps them to `Action::Key`.
  - `ui.rs`:
    - **Pure Rendering**: Takes a reference to `AppState` and draws the screen.
    - **Dynamic Layouts**: Adjusts the layout when logs are toggled and injects contextual selection info into the bottom bar.
    - **Visual Feedback**: Highlights the currently playing track in the active search results.
    - **Overlays**: Floating help panel (`:k`) and theme selector (`:t`) rendered on top of content.
  - `theme.rs`:
    - **Palette System**: Each theme defines accent and text colors for both dark and light modes via a `Palette` struct.
    - **9 Presets**: default, gruvbox, tokyo-night, rose-pine, catppuccin, everforest, kanagawa, nord, magenta.
    - **Convenience Methods**: `Theme` provides `base()`, `selected()`, `badge()`, `header()`, `muted()`, etc. so `ui.rs` never uses inline colors.

---

## Communication Flow

1. **Input**: User enter in Insert mode, search for a query and press `Enter`.
2. **Event**: `tui/events.rs` sends `Action::Key(Enter)`.
3. **Core**: `event_loop.rs` receives it, sees `Mode::Insert`, and triggers `Action::SearchSubmit`.
4. **Action**: `handle_action` calls `PluginManager::handle_search`.
5. **Async**: `PluginManager` spawns a task, queries the active provider (e.g., native Bandcamp API or `yt-dlp`), and eventually sends `Action::PluginResponse`.
6. **Update**: Core receives the response, updates `AppState.search_states`, and asks TUI to re-render.
7. **Preload**: As the user moves the cursor over a track, core sends `resolve_stream_url` commands to start pre-fetching the stream immediately via the native provider or `yt-dlp`.

## Important Constraints to Maintain

- **No `eprintln!`**: Always use `Action::Log` to send errors to the log panel. `stderr` is redirected to `audiplayer.log` in `main.rs` via `libc::dup2` to prevent TUI corruption. `Action::Log` also writes to `stderr` so logs are captured in the file.
- **Clone Arcs**: Always clone `Arc` handles (`player`, `plugins`) before moving them into `tokio::spawn` blocks in the event loop.
- **Mode Awareness**: Input handling must always check `state.mode` to ensure keys like `j/k` don't leak into the search buffer.
- **Semaphore Discipline**: Always acquire a semaphore permit before spawning `yt-dlp` processes (max 3 concurrent) to avoid rate-limiting (429 errors).
- **Native vs Fallback Resolution**: Always attempt native stream resolution via the provider trait first before falling back to `yt-dlp`. For Bandcamp, `yt-dlp` fallback is explicitly disabled.
