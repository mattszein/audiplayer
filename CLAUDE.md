# CLAUDE.md — Audiplayer

A provider-agnostic TUI music player written in Rust. Uses mpv for audio, Ratatui for the terminal UI, and a plugin system with native provider APIs (Bandcamp) and yt-dlp fallback (YouTube) for stream resolution.

## Quick Reference

- **Build**: `cargo build` or `cargo build --release`
- **Run**: `cargo run`
- **Check**: `cargo check`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`
- **Test**: `cargo test`
- **Rust edition**: 2024
- **Runtime deps**: `mpv` must be in `$PATH`; `yt-dlp` required for YouTube provider

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for full details.

### Core Pattern: Unidirectional Data Flow

All communication flows through a single `mpsc` channel as `Action` variants. The event loop (`core/event_loop.rs`) is the only place state is mutated. The TUI is a pure renderer — it reads `AppState` and draws, never mutates state.

```
Input → Action::Key → event_loop (handle_action) → mutate AppState → tui.draw()
```

### Module Map

| Module | Purpose | Key types |
|--------|---------|-----------|
| `src/core/action.rs` | All possible actions — the app's "API" | `Action`, `Track`, `PlayerEvent`, `PluginResult`, `ResultType` |
| `src/core/state.rs` | Single source of truth | `AppState`, `SearchState` (with `history` stack + `breadcrumbs`), `PlaybackState`, `Focus`, `PlaybackStatus` |
| `src/core/event_loop.rs` | State mutation and side-effect dispatch | `run()`, `handle_action()`, `handle_key_event()` |
| `src/core/mode.rs` | Interaction modes | `Mode` (Normal, Insert, Command) |
| `src/player/mod.rs` | Player trait definition | `Player` trait |
| `src/player/mpv.rs` | mpv IPC implementation | `MpvPlayer` |
| `src/plugins/mod.rs` | Plugin manager + native/yt-dlp stream resolution | `PluginManager` |
| `src/plugins/traits.rs` | Provider trait definition | `Provider` trait (with `get_album_tracks`), `Capability` |
| `src/plugins/bandcamp.rs` | Bandcamp native API (search, streaming, albums) | `BandcampProvider` |
| `src/plugins/youtube.rs` | YouTube search via yt-dlp | `YouTubeProvider` |
| `src/tui/mod.rs` | Terminal setup/teardown | `Tui` |
| `src/tui/ui.rs` | Pure rendering functions | `render()` and sub-renderers |
| `src/tui/events.rs` | Blocking input listener thread | `spawn_input_listener()` |

## Critical Conventions

### Never use `eprintln!` or write to stderr directly
stderr is redirected to `audiplayer.log` via `libc::dup2` in `main.rs`. Use `Action::Log(msg)` to surface errors in the TUI log panel. `Action::Log` also writes to stderr so logs are captured in the file.

### Always clone Arc before `tokio::spawn`
The event loop holds `Arc<MpvPlayer>` and `Arc<PluginManager>`. Clone the Arc *before* moving into spawn blocks:
```rust
let player_clone = player.clone();
tokio::spawn(async move { player_clone.play(&track).await; });
```

### Mode-aware key handling
`handle_key_event()` matches on `state.mode` first. Keys like `j`/`k` only apply in `Mode::Normal`. When adding new keybindings, always place them under the correct mode branch.

### Semaphore for yt-dlp
`PluginManager` uses a `Semaphore::new(3)` to cap concurrent yt-dlp processes (only for providers that rely on it, like YouTube). Always acquire a permit before spawning yt-dlp to avoid 429 rate-limiting.

### Native vs fallback stream resolution
Always attempt native stream resolution via the provider's `get_stream_url` first. Only fall back to yt-dlp if the native method fails or is unsupported. For Bandcamp, yt-dlp fallback is explicitly disabled — it uses its own native API.

### Stream URL lifecycle
1. Search results arrive without `stream_url` (it's `None`)
2. As the user scrolls, `preload_selected_track()` resolves the URL via the provider's native method or yt-dlp fallback
3. Resolved URLs are cached in `Track.stream_url` and a `[P]` badge appears in the UI
4. On play: if `stream_url` exists, play immediately; otherwise resolve first, showing "Resolving URL..."

### Drill-down navigation
`SearchState` maintains a `history` stack and `breadcrumbs` for drill-down navigation. When a user presses Enter on an album, the current results are pushed onto the history stack and album tracks are displayed. Backspace pops the stack to return to the previous view.

### Adding a new provider
1. Create `src/plugins/newprovider.rs` implementing the `Provider` trait (including `get_stream_url` and optionally `get_album_tracks`)
2. Register it in `PluginManager::new()` in `src/plugins/mod.rs`
3. Add the provider ID string to `AppState::new()` providers list in `src/core/state.rs`

### Adding a new Action
1. Add the variant to `Action` enum in `src/core/action.rs`
2. Handle it in `handle_action()` in `src/core/event_loop.rs`
3. If triggered by a key, add the keybinding in `handle_key_event()`

## Project Layout

```
src/
├── main.rs              # Entry point: stderr redirect, channel setup, subsystem init
├── core/
│   ├── mod.rs
│   ├── action.rs        # Action enum, Track struct, PlayerEvent, PluginResult
│   ├── state.rs         # AppState (single source of truth, history stack, breadcrumbs)
│   ├── event_loop.rs    # Main loop, handle_action, handle_key_event
│   └── mode.rs          # Normal / Insert / Command
├── player/
│   ├── mod.rs           # Player trait
│   └── mpv.rs           # MpvPlayer: IPC socket + stdout parsing
├── plugins/
│   ├── mod.rs           # PluginManager + native/yt-dlp stream resolution
│   ├── traits.rs        # Provider trait (search, get_stream_url, get_album_tracks) + Capability
│   ├── bandcamp.rs      # Bandcamp native API (search, streaming, album tracks)
│   └── youtube.rs       # YouTube search via yt-dlp --dump-json
└── tui/
    ├── mod.rs           # Tui struct: terminal lifecycle
    ├── ui.rs            # Pure rendering (render, render_player, render_search, etc.)
    └── events.rs        # Blocking crossterm event reader thread
```

## mpv IPC Details

- Socket path: `/tmp/mpv-socket-audiplayer`
- mpv is spawned with `--idle --no-video --ytdl=no --terminal=yes`
- Commands sent as JSON over Unix domain socket (e.g., `{"command": ["loadfile", url, "replace"]}`)
- stdout is read byte-by-byte to handle `\r` carriage-return progress lines (mpv uses `\r` for status updates like `A: 1.23 / 4.56 (27%)`)
- The `end-file` IPC event triggers `PlayerEvent::TrackEnded`
