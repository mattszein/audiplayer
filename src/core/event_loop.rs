use anyhow::Result;
use tokio::sync::mpsc::Receiver;
use std::sync::Arc;
use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};

use crate::core::{
    action::{Action, PlayerEvent, PluginResult, ResultType},
    state::{AppState, Focus, NowPlaying, PlaybackStatus},
    Mode,
};
use crate::tui::Tui;
use crate::tui::theme::{Theme, ThemeMode};
use crate::player::{Player, mpv::MpvPlayer};
use crate::plugins::PluginManager;

/// The heart of the app. Receives Actions from all sources,
/// mutates AppState, and asks the TUI to re-render.
pub async fn run(
    mut state: AppState,
    mut tui: Tui,
    player: MpvPlayer,
    plugins: PluginManager,
    mut action_rx: Receiver<Action>,
) -> Result<()> {
    let player = Arc::new(player);
    let plugins = Arc::new(plugins);

    tui.enter()?;
    tui.draw(&state)?;

    loop {
        let action = action_rx.recv().await;
        match action {
            None => break,
            Some(Action::Quit) => {
                let _ = player.stop_mpv().await;
                break;
            }
            Some(action) => {
                let quit = handle_action(action, &mut state, &player, &plugins);
                tui.draw(&state)?;
                if quit {
                    let _ = player.stop_mpv().await;
                    break;
                }
            }
        }
    }

    tui.exit()?;
    Ok(())
}

/// Side effects and state mutation. Returns true if app should quit.
fn handle_action(action: Action, state: &mut AppState, player: &Arc<MpvPlayer>, plugins: &Arc<PluginManager>) -> bool {
    match action {
        Action::Quit => return true,
        Action::Log(msg) => {
            eprintln!("{}", msg);
            state.logs.push(msg);
            if state.logs.len() > 500 { state.logs.remove(0); }
        }
        Action::Key(key) => {
             if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                 return true;
             }
             return handle_key_event(key, state, player, plugins);
        }

        Action::SetMode(mode) => {
            state.mode = mode;
            if mode == Mode::Command { state.command_input = String::new(); }
        }

        // ── Command Handling ──────────────────────────────────────────
        Action::CommandInput(c) => state.command_input.push(c),
        Action::CommandBackspace => { state.command_input.pop(); }
        Action::CommandExecute => {
            let cmd = state.command_input.trim();
            match cmd {
                "q" | "quit" => {
                    if state.focus == Focus::Logs {
                        state.show_logs = false;
                        state.focus = Focus::Search;
                    } else {
                        return true;
                    }
                }
                "l" | "log" => {
                    state.show_logs = !state.show_logs;
                    if state.show_logs { state.focus = Focus::Logs; }
                    else if state.focus == Focus::Logs { state.focus = Focus::Search; }
                }
                "theme" | "t" => {
                    handle_action(Action::OpenThemeSelector, state, player, plugins);
                }
                "mode" | "dm" => {
                    handle_action(Action::CycleThemeMode, state, player, plugins);
                }
                "help" | "keys" | "k" => {
                    handle_action(Action::ToggleHelp, state, player, plugins);
                }
                _ => { state.logs.push(format!("Unknown command: {}", cmd)); }
            }
            state.mode = Mode::Normal;
        }

        Action::SwitchProvider(provider) => {
            if state.providers.contains(&provider) {
                state.active_provider = provider;
                preload_selected_track(state, plugins);
            }
        }

        Action::SearchInput(c) => state.get_active_search_mut().input.push(c),
        Action::SearchBackspace => { state.get_active_search_mut().input.pop(); }
        Action::SearchSubmit => {
            let provider = state.active_provider.clone();
            let search = state.get_active_search_mut();
            search.is_loading = true;
            search.resolving.clear();
            search.history.clear();
            search.breadcrumbs = vec!["Search".to_string(), search.input.clone()];
            let query = search.input.clone();
            let plugins_clone = plugins.clone();
            tokio::spawn(async move {
                plugins_clone.handle_search(&provider, query).await;
            });
        }
        Action::CursorDown => {
            if state.focus == Focus::Search {
                let search = state.get_active_search_mut();
                if !search.results.is_empty() {
                    search.cursor = (search.cursor + 1).min(search.results.len() - 1);
                    preload_selected_track(state, plugins);
                }
            } else if state.focus == Focus::NowPlaying {
                if let Some(ref mut np) = state.now_playing {
                    if !np.tracks.is_empty() {
                        np.cursor = (np.cursor + 1).min(np.tracks.len() - 1);
                    }
                }
            }
        }
        Action::CursorUp => {
            if state.focus == Focus::Search {
                let search = state.get_active_search_mut();
                search.cursor = search.cursor.saturating_sub(1);
                preload_selected_track(state, plugins);
            } else if state.focus == Focus::NowPlaying {
                if let Some(ref mut np) = state.now_playing {
                    np.cursor = np.cursor.saturating_sub(1);
                }
            }
        }

        Action::Play(track) => {
            let mut track_to_play = track.clone();
            if let Some(stream_url) = &track.stream_url {
                track_to_play.url = stream_url.clone();
                state.playback.track = Some(track_to_play.clone());
                state.playback.status = PlaybackStatus::Playing;
                state.playback.last_mpv_line = None;
                
                let player_clone = player.clone();
                tokio::spawn(async move {
                    let _ = player_clone.play(&track_to_play).await;
                });
            } else {
                state.playback.track = Some(track.clone());
                state.playback.status = PlaybackStatus::Playing;
                state.playback.last_mpv_line = Some("Resolving URL...".to_string());
                
                let track_clone = track.clone();
                let plugins_clone = plugins.clone();
                tokio::spawn(async move {
                    plugins_clone.resolve_stream_url(track_clone).await;
                });
            }
        }
        Action::PlayPause => match state.playback.status {
            PlaybackStatus::Playing => {
                 state.playback.status = PlaybackStatus::Paused;
                 let player_clone = player.clone();
                 tokio::spawn(async move { let _ = player_clone.pause().await; });
            }
            PlaybackStatus::Paused => {
                 state.playback.status = PlaybackStatus::Playing;
                 let player_clone = player.clone();
                 tokio::spawn(async move { let _ = player_clone.resume().await; });
            }
            PlaybackStatus::Stopped => {
                // Nothing playing — do nothing
            }
        }
        Action::PlaySelected => {
            if state.focus == Focus::NowPlaying {
                // NowPlaying: play the track at the cursor
                if let Some(ref mut np) = state.now_playing {
                    if np.cursor < np.tracks.len() {
                        np.current_index = np.cursor;
                        let track = np.tracks[np.cursor].clone();
                        handle_action(Action::Play(track), state, player, plugins);
                    }
                }
            } else {
                // Search: drill into albums or play tracks directly
                let search = state.get_active_search();
                if let Some(track) = search.results.get(search.cursor) {
                    match track.result_type {
                        ResultType::Album => {
                            handle_action(Action::FetchAlbumTracks(track.clone()), state, player, plugins);
                        }
                        ResultType::Artist => {
                            state.logs.push(format!("Discography fetch for artist {} not implemented yet", track.artist));
                        }
                        ResultType::Track => {
                            let track_clone = track.clone();
                            handle_action(Action::Play(track_clone), state, player, plugins);
                        }
                    }
                }
            }
        }

        Action::GoBack => {
            let search = state.get_active_search_mut();
            if let Some((old_results, old_cursor)) = search.history.pop() {
                search.results = old_results;
                search.cursor = old_cursor;
                search.breadcrumbs.pop();
            }
        }

        Action::FetchAlbumTracks(track) => {
            let provider = state.active_provider.clone();
            let search = state.get_active_search_mut();
            
            // Save current state to history
            search.history.push((search.results.clone(), search.cursor));
            search.breadcrumbs.push(track.title.clone());

            search.is_loading = true;
            search.resolving.clear();
            let plugins_clone = plugins.clone();
            tokio::spawn(async move {
                plugins_clone.handle_fetch_album_tracks(&provider, track).await;
            });
        }

        Action::NowPlayingAdd => {
            let search = state.get_active_search();
            if let Some(track) = search.results.get(search.cursor) {
                if track.result_type == ResultType::Track {
                    let track_clone = track.clone();
                    if let Some(ref mut np) = state.now_playing {
                        if !np.tracks.iter().any(|t| t.id == track_clone.id) {
                            np.tracks.push(track_clone);
                        }
                    } else {
                        state.now_playing = Some(NowPlaying {
                            tracks: vec![track_clone],
                            current_index: 0,
                            cursor: 0,
                        });
                    }
                    state.show_now_playing = true;
                    maybe_autoplay(state, player, plugins);
                }
            }
        }
        Action::NowPlayingReplace => {
            let search = state.get_active_search();
            if let Some(track) = search.results.get(search.cursor) {
                if track.result_type == ResultType::Track {
                    let track_clone = track.clone();
                    push_now_playing_to_history(state);
                    state.now_playing = Some(NowPlaying {
                        tracks: vec![track_clone],
                        current_index: 0,
                        cursor: 0,
                    });
                    state.show_now_playing = true;
                    maybe_autoplay(state, player, plugins);
                }
            }
        }
        Action::NowPlayingAddAll => {
            let search = state.get_active_search();
            let tracks: Vec<_> = search.results.iter()
                .filter(|t| t.result_type == ResultType::Track)
                .cloned()
                .collect();
            if !tracks.is_empty() {
                if let Some(ref mut np) = state.now_playing {
                    for t in tracks {
                        if !np.tracks.iter().any(|existing| existing.id == t.id) {
                            np.tracks.push(t);
                        }
                    }
                } else {
                    state.now_playing = Some(NowPlaying {
                        tracks,
                        current_index: 0,
                        cursor: 0,
                    });
                }
                state.show_now_playing = true;
                maybe_autoplay(state, player, plugins);
            }
        }
        Action::NowPlayingReplaceAll => {
            let search = state.get_active_search();
            let tracks: Vec<_> = search.results.iter()
                .filter(|t| t.result_type == ResultType::Track)
                .cloned()
                .collect();
            if !tracks.is_empty() {
                push_now_playing_to_history(state);
                state.now_playing = Some(NowPlaying {
                    tracks,
                    current_index: 0,
                    cursor: 0,
                });
                state.show_now_playing = true;
                maybe_autoplay(state, player, plugins);
            }
        }

        Action::ToggleAutoplayAdd => {
            state.autoplay_add = !state.autoplay_add;
        }

        Action::ToggleNowPlaying => {
            state.show_now_playing = !state.show_now_playing;
            if state.show_now_playing {
                state.focus = Focus::NowPlaying;
            } else if state.focus == Focus::NowPlaying {
                state.focus = Focus::Search;
            }
        }
        Action::NowPlayingBack => {
            if let Some(current) = state.now_playing.take() {
                if let Some(prev) = state.now_playing_history.pop() {
                    state.now_playing_future.push(current);
                    state.now_playing = Some(prev);
                } else {
                    state.now_playing = Some(current);
                }
            }
        }
        Action::NowPlayingForward => {
            if let Some(current) = state.now_playing.take() {
                if let Some(next) = state.now_playing_future.pop() {
                    state.now_playing_history.push(current);
                    state.now_playing = Some(next);
                } else {
                    state.now_playing = Some(current);
                }
            }
        }

        Action::OpenThemeSelector => {
            let names = Theme::preset_names();
            let idx = names.iter().position(|&n| n == state.theme.name).unwrap_or(0);
            state.theme_selector_cursor = idx;
            state.theme_before_selector = Some(state.theme.name.to_string());
            state.show_theme_selector = true;
        }
        Action::CycleThemeMode => {
            let new_mode = match state.theme.mode {
                ThemeMode::Dark => ThemeMode::Light,
                ThemeMode::Light => ThemeMode::Dark,
            };
            state.theme = state.theme.with_mode(new_mode);
        }
        Action::ToggleHelp => {
            state.show_help = !state.show_help;
            state.help_scroll = 0;
        }

        Action::MpvStdout(line) => state.playback.last_mpv_line = Some(line),
        Action::PlayerEvent(event) => match event {
            PlayerEvent::TrackEnded => {
                // Try auto-advance in now_playing context
                let should_advance = state.now_playing.as_ref().is_some_and(|np| {
                    np.current_index + 1 < np.tracks.len()
                });
                if should_advance {
                    let np = state.now_playing.as_mut().unwrap();
                    np.current_index += 1;
                    np.cursor = np.current_index;
                    let next_track = np.tracks[np.current_index].clone();
                    handle_action(Action::Play(next_track), state, player, plugins);
                } else {
                    state.playback.status = PlaybackStatus::Stopped;
                    state.playback.last_mpv_line = None;
                }
            }
            _ => {}
        }

        Action::PluginResponse { id, result } => {
            match result {
                PluginResult::Search(tracks) => {
                    if let Some(search) = state.search_states.get_mut(&id) {
                        search.results = tracks;
                        search.is_loading = false;
                        search.cursor = 0;
                        search.resolving.clear();
                        if id == state.active_provider {
                            preload_selected_track(state, plugins);
                        }
                    }
                }
                PluginResult::AlbumTracks(tracks) => {
                    if let Some(search) = state.search_states.get_mut(&id) {
                        search.results = tracks;
                        search.is_loading = false;
                        search.cursor = 0;
                        search.resolving.clear();
                        if id == state.active_provider {
                            preload_selected_track(state, plugins);
                        }
                    }
                }
                PluginResult::StreamUrl { track_id, url, duration, bitrate } => {
                    for search in state.search_states.values_mut() {
                        search.resolving.remove(&track_id);
                        for track in &mut search.results {
                            if track.id == track_id {
                                track.stream_url = Some(url.clone());
                                if duration.is_some() { track.duration = duration; }
                                if bitrate.is_some() { track.bitrate = bitrate; }
                            }
                        }
                    }
                    // Also update tracks in now_playing context
                    if let Some(ref mut np) = state.now_playing {
                        for track in &mut np.tracks {
                            if track.id == track_id {
                                track.stream_url = Some(url.clone());
                                if duration.is_some() { track.duration = duration; }
                                if bitrate.is_some() { track.bitrate = bitrate; }
                            }
                        }
                    }
                    if let Some(ref mut current) = state.playback.track {
                        if current.id == track_id {
                            current.stream_url = Some(url.clone());
                            if duration.is_some() { current.duration = duration; }
                            if bitrate.is_some() { current.bitrate = bitrate; }
                            
                            // If we were waiting for this track to resolve, play it now
                            if state.playback.status == PlaybackStatus::Playing {
                                let mut t = current.clone();
                                t.url = url;
                                let player_clone = player.clone();
                                tokio::spawn(async move { let _ = player_clone.play(&t).await; });
                            }
                        }
                    }
                }
                PluginResult::Error(e) => {
                    if let Some(search) = state.search_states.get_mut(&id) {
                        search.is_loading = false;
                    }
                    state.logs.push(format!("Error: {}", e));
                }
            }
        }
        _ => {}
    }
    false
}

fn maybe_autoplay(state: &mut AppState, player: &Arc<MpvPlayer>, plugins: &Arc<PluginManager>) {
    if !state.autoplay_add || state.playback.status != PlaybackStatus::Stopped {
        return;
    }
    if let Some(ref mut np) = state.now_playing {
        if !np.tracks.is_empty() {
            np.current_index = 0;
            np.cursor = 0;
            let track = np.tracks[0].clone();
            handle_action(Action::Play(track), state, player, plugins);
        }
    }
}

fn push_now_playing_to_history(state: &mut AppState) {
    if let Some(current_np) = state.now_playing.take() {
        state.now_playing_history.push(current_np);
        if state.now_playing_history.len() > 5 {
            state.now_playing_history.remove(0);
        }
    }
    state.now_playing_future.clear();
}

fn preload_selected_track(state: &mut AppState, plugins: &Arc<PluginManager>) {
    let active_provider = state.active_provider.clone();
    let search = state.search_states.get_mut(&active_provider).unwrap();
    if let Some(track) = search.results.get(search.cursor) {
        if track.result_type == ResultType::Track && track.stream_url.is_none() && !search.resolving.contains(&track.id) {
            let track_clone = track.clone();
            let plugins_clone = plugins.clone();
            search.resolving.insert(track.id.clone());
            tokio::spawn(async move {
                plugins_clone.resolve_stream_url(track_clone).await;
            });
        }
    }
}

fn handle_key_event(key: KeyEvent, state: &mut AppState, player: &Arc<MpvPlayer>, plugins: &Arc<PluginManager>) -> bool {
    let last_key = state.last_key.take();

    match state.mode {
        Mode::Normal if state.show_theme_selector => {
            let names = Theme::preset_names();
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    state.theme_selector_cursor = (state.theme_selector_cursor + 1) % names.len();
                    state.theme = Theme::from_name(names[state.theme_selector_cursor], state.theme.mode);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    state.theme_selector_cursor = (state.theme_selector_cursor + names.len() - 1) % names.len();
                    state.theme = Theme::from_name(names[state.theme_selector_cursor], state.theme.mode);
                }
                KeyCode::Enter => {
                    state.show_theme_selector = false;
                    state.theme_before_selector = None;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    if let Some(ref original) = state.theme_before_selector {
                        state.theme = Theme::from_name(original, state.theme.mode);
                    }
                    state.show_theme_selector = false;
                    state.theme_before_selector = None;
                }
                _ => {}
            }
        }
        Mode::Normal if state.show_help => {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => { state.help_scroll = state.help_scroll.saturating_add(1); }
                KeyCode::Char('k') | KeyCode::Up => { state.help_scroll = state.help_scroll.saturating_sub(1); }
                KeyCode::Char('q') | KeyCode::Esc => { state.show_help = false; }
                _ => {}
            }
        }
        Mode::Normal => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Char(':')) => {
                    state.mode = Mode::Command;
                    state.command_input = String::new();
                }
                (KeyModifiers::NONE, KeyCode::Char('i')) => {
                    state.mode = Mode::Insert;
                    state.focus = Focus::Search;
                }

                (KeyModifiers::NONE, KeyCode::Tab) => {
                    let idx = state.providers.iter().position(|p| p == &state.active_provider).unwrap_or(0);
                    let next_idx = (idx + 1) % state.providers.len();
                    let next_provider = state.providers[next_idx].clone();
                    handle_action(Action::SwitchProvider(next_provider), state, player, plugins);
                }
                (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                    let idx = state.providers.iter().position(|p| p == &state.active_provider).unwrap_or(0);
                    let next_idx = (idx + state.providers.len() - 1) % state.providers.len();
                    let next_provider = state.providers[next_idx].clone();
                    handle_action(Action::SwitchProvider(next_provider), state, player, plugins);
                }

                (KeyModifiers::CONTROL, KeyCode::Left) | (KeyModifiers::NONE, KeyCode::Left) | (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                    state.focus = Focus::Search;
                }
                (KeyModifiers::CONTROL, KeyCode::Right) | (KeyModifiers::NONE, KeyCode::Right) | (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    if state.show_now_playing { state.focus = Focus::NowPlaying; }
                    else if state.show_logs { state.focus = Focus::Logs; }
                }

                // Now Playing: add/replace selected track or all results
                (KeyModifiers::NONE, KeyCode::Char('a')) => {
                    handle_action(Action::NowPlayingAdd, state, player, plugins);
                }
                (KeyModifiers::NONE, KeyCode::Char('r')) => {
                    handle_action(Action::NowPlayingReplace, state, player, plugins);
                }
                (KeyModifiers::SHIFT, KeyCode::Char('A')) => {
                    handle_action(Action::NowPlayingAddAll, state, player, plugins);
                }
                (KeyModifiers::SHIFT, KeyCode::Char('R')) => {
                    handle_action(Action::NowPlayingReplaceAll, state, player, plugins);
                }

                // Now Playing history navigation
                (KeyModifiers::SHIFT, KeyCode::Char('H')) if state.focus == Focus::NowPlaying => {
                    handle_action(Action::NowPlayingBack, state, player, plugins);
                }
                (KeyModifiers::SHIFT, KeyCode::Char('L')) if state.focus == Focus::NowPlaying => {
                    handle_action(Action::NowPlayingForward, state, player, plugins);
                }

                (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                    handle_action(Action::CursorDown, state, player, plugins);
                }
                (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                    handle_action(Action::CursorUp, state, player, plugins);
                }

                // gg -> first
                (KeyModifiers::NONE, KeyCode::Char('g')) => {
                    if let Some(KeyEvent { code: KeyCode::Char('g'), .. }) = last_key {
                        if state.focus == Focus::NowPlaying {
                            if let Some(ref mut np) = state.now_playing {
                                np.cursor = 0;
                            }
                        } else {
                            let search = state.get_active_search_mut();
                            search.cursor = 0;
                            preload_selected_track(state, plugins);
                        }
                    } else {
                        state.last_key = Some(key);
                    }
                }
                // G -> last
                (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                    if state.focus == Focus::NowPlaying {
                        if let Some(ref mut np) = state.now_playing {
                            if !np.tracks.is_empty() {
                                np.cursor = np.tracks.len() - 1;
                            }
                        }
                    } else {
                        let search = state.get_active_search_mut();
                        if !search.results.is_empty() {
                            search.cursor = search.results.len() - 1;
                            preload_selected_track(state, plugins);
                        }
                    }
                }

                (KeyModifiers::NONE, KeyCode::Char('e')) => {
                    handle_action(Action::ToggleNowPlaying, state, player, plugins);
                }
                (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                    handle_action(Action::PlayPause, state, player, plugins);
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    handle_action(Action::PlaySelected, state, player, plugins);
                }
                (KeyModifiers::NONE, KeyCode::Backspace) if state.focus == Focus::Search => {
                    handle_action(Action::GoBack, state, player, plugins);
                }

                (KeyModifiers::NONE, KeyCode::Char('p')) => {
                    handle_action(Action::ToggleAutoplayAdd, state, player, plugins);
                }

                (KeyModifiers::NONE, KeyCode::Char('q')) if state.focus == Focus::Logs => {
                    state.show_logs = false;
                    state.focus = Focus::Search;
                }

                (KeyModifiers::NONE, KeyCode::Char('1')) => { handle_action(Action::SwitchProvider("bandcamp".into()), state, player, plugins); }
                (KeyModifiers::NONE, KeyCode::Char('2')) => { handle_action(Action::SwitchProvider("youtube".into()), state, player, plugins); }
                _ => {}
            },
        Mode::Insert => match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => state.mode = Mode::Normal,
            (KeyModifiers::NONE, KeyCode::Enter) => {
                handle_action(Action::SearchSubmit, state, player, plugins);
                state.mode = Mode::Normal;
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => { state.get_active_search_mut().input.pop(); }
            (KeyModifiers::NONE, KeyCode::Char(c)) => { state.get_active_search_mut().input.push(c); }
            _ => {}
        },
        Mode::Command => match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => state.mode = Mode::Normal,
            (KeyModifiers::NONE, KeyCode::Backspace) => { state.command_input.pop(); }
            (KeyModifiers::NONE, KeyCode::Char(c)) => { state.command_input.push(c); }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                return handle_action(Action::CommandExecute, state, player, plugins);
            }
            _ => {}
        }
    }
    false
}
