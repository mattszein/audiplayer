use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

use crate::core::action::Action;
use crate::core::handlers::{self, Ctx};
use crate::core::state::AppState;
use crate::player::mpv::MpvPlayer;
use crate::plugins::PluginManager;
use crate::tui::Tui;

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

/// Dispatches actions to handler modules. Returns true if app should quit.
fn handle_action(
    action: Action,
    state: &mut AppState,
    player: &Arc<MpvPlayer>,
    plugins: &Arc<PluginManager>,
) -> bool {
    let ctx = &mut Ctx {
        state,
        player,
        plugins,
    };

    match action {
        Action::Quit => return true,

        Action::Key(key) => {
            if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                return true;
            }
            return handlers::keys::handle_key_event(key, ctx);
        }

        Action::Log(msg) => handlers::ui::handle_log(msg, ctx.state),
        Action::SetMode(mode) => handlers::ui::handle_set_mode(mode, ctx.state),

        // Playback
        Action::Play(_)
        | Action::PlayPause
        | Action::Stop
        | Action::PlaySelected
        | Action::Skip
        | Action::SeekForward(_)
        | Action::SeekBackward(_)
        | Action::VolumeUp
        | Action::VolumeDown
        | Action::ToggleMute
        | Action::PlayerEvent(_) => {
            return handlers::playback::handle(action, ctx);
        }

        // Search & Navigation
        Action::SearchInput(_)
        | Action::SearchBackspace
        | Action::SearchSubmit
        | Action::CursorDown
        | Action::CursorUp
        | Action::GoBack
        | Action::SwitchProvider(_)
        | Action::FetchAlbumTracks(_) => {
            return handlers::search::handle(action, ctx);
        }

        // Now Playing
        Action::NowPlayingAdd
        | Action::NowPlayingReplace
        | Action::NowPlayingAddAll
        | Action::NowPlayingReplaceAll
        | Action::ToggleNowPlaying
        | Action::NowPlayingBack
        | Action::NowPlayingForward
        | Action::ToggleAutoplayAdd => {
            return handlers::now_playing::handle(action, ctx);
        }

        // Command input (handled inline in keys.rs Command mode; these are fallback paths)
        Action::CommandInput(c) => ctx.state.command_input.push(c),
        Action::CommandBackspace => {
            ctx.state.command_input.pop();
        }
        Action::CommandExecute => {}

        // Plugin responses
        Action::PluginResponse { .. } => {
            return handlers::plugin::handle(action, ctx);
        }

        // UI
        Action::OpenThemeSelector | Action::CycleThemeMode | Action::ToggleHelp => {
            return handlers::ui::handle(action, ctx);
        }

        _ => {}
    }
    false
}
