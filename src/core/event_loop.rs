use anyhow::Result;
use tokio::sync::mpsc::Receiver;
use std::sync::Arc;
use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};

use crate::core::{
    action::{Action, PlayerEvent, PluginResult, Track},
    state::{AppState, Focus, PlaybackStatus},
    Mode,
};
use crate::tui::Tui;
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
            state.logs.push(msg);
            if state.logs.len() > 500 { state.logs.remove(0); }
        }
        Action::Key(key) => {
             if (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c')) ||
                (state.mode == Mode::Normal && key.code == KeyCode::Char('q') && state.focus != Focus::Logs) {
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
            }
        }
        Action::CursorUp => {
            if state.focus == Focus::Search {
                let search = state.get_active_search_mut();
                search.cursor = search.cursor.saturating_sub(1);
                preload_selected_track(state, plugins);
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
                let search = state.get_active_search();
                if let Some(track) = search.results.get(search.cursor) {
                    handle_action(Action::Play(track.clone()), state, player, plugins);
                }
            }
        }
        Action::PlaySelected => {
            let search = state.get_active_search();
            if let Some(track) = search.results.get(search.cursor) {
                handle_action(Action::Play(track.clone()), state, player, plugins);
            }
        }

        Action::MpvStdout(line) => state.playback.last_mpv_line = Some(line),
        Action::PlayerEvent(event) => match event {
            PlayerEvent::TrackEnded => {
                state.playback.status = PlaybackStatus::Stopped;
                state.playback.last_mpv_line = None;
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
                    if let Some(ref mut current) = state.playback.track {
                        if current.id == track_id {
                            current.stream_url = Some(url.clone());
                            if duration.is_some() { current.duration = duration; }
                            if bitrate.is_some() { current.bitrate = bitrate; }
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

fn preload_selected_track(state: &mut AppState, plugins: &Arc<PluginManager>) {
    let active_provider = state.active_provider.clone();
    let search = state.search_states.get_mut(&active_provider).unwrap();
    if let Some(track) = search.results.get(search.cursor) {
        if track.stream_url.is_none() && !search.resolving.contains(&track.id) {
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
                if state.show_logs { state.focus = Focus::Logs; }
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
                    let search = state.get_active_search_mut();
                    search.cursor = 0;
                    preload_selected_track(state, plugins);
                } else {
                    state.last_key = Some(key);
                }
            }
            // G -> last
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                let search = state.get_active_search_mut();
                if !search.results.is_empty() {
                    search.cursor = search.results.len() - 1;
                    preload_selected_track(state, plugins);
                }
            }

            (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                handle_action(Action::PlayPause, state, player, plugins);
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                handle_action(Action::PlaySelected, state, player, plugins);
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
