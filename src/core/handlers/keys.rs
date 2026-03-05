use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::core::Mode;
use crate::core::action::Action;
use crate::core::settings::Settings;
use crate::core::state::Focus;
use crate::tui::theme::Theme;

/// Execute a `:` command. Returns true if the app should quit.
fn execute_command(ctx: &mut Ctx) -> bool {
    let cmd = ctx.state.command_input.trim().to_string();
    let quit = match cmd.as_str() {
        "q" | "quit" => {
            if ctx.state.focus == Focus::Logs {
                ctx.state.show_logs = false;
                ctx.state.focus = Focus::Search;
                false
            } else {
                true
            }
        }
        "l" | "log" => {
            ctx.state.show_logs = !ctx.state.show_logs;
            if ctx.state.show_logs {
                ctx.state.focus = Focus::Logs;
            } else if ctx.state.focus == Focus::Logs {
                ctx.state.focus = Focus::Search;
            }
            false
        }
        "theme" | "t" => {
            super::ui::handle(Action::OpenThemeSelector, ctx);
            false
        }
        "mode" | "dm" => {
            super::ui::handle(Action::CycleThemeMode, ctx);
            false
        }
        "help" | "keys" | "h" => {
            super::ui::handle(Action::ToggleHelp, ctx);
            false
        }
        "config" => {
            let msg = Settings::ensure_config_file();
            super::ui::handle_log(msg.clone(), ctx.state);
            ctx.state.status_message = Some(msg);
            false
        }
        _ => {
            ctx.state.logs.push(format!("Unknown command: {}", cmd));
            false
        }
    };
    ctx.state.mode = Mode::Normal;
    quit
}

use super::Ctx;
use super::search::preload_selected_track;

pub fn handle_key_event(key: KeyEvent, ctx: &mut Ctx) -> bool {
    ctx.state.status_message = None;
    let last_key = ctx.state.last_key.take();

    match ctx.state.mode {
        Mode::Normal if ctx.state.show_theme_selector => {
            let names = Theme::preset_names();
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    ctx.state.theme_selector_cursor =
                        (ctx.state.theme_selector_cursor + 1) % names.len();
                    ctx.state.theme =
                        Theme::from_name(names[ctx.state.theme_selector_cursor], ctx.state.theme.mode);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    ctx.state.theme_selector_cursor =
                        (ctx.state.theme_selector_cursor + names.len() - 1) % names.len();
                    ctx.state.theme =
                        Theme::from_name(names[ctx.state.theme_selector_cursor], ctx.state.theme.mode);
                }
                KeyCode::Enter => {
                    ctx.state.show_theme_selector = false;
                    ctx.state.theme_before_selector = None;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    if let Some(ref original) = ctx.state.theme_before_selector {
                        ctx.state.theme = Theme::from_name(original, ctx.state.theme.mode);
                    }
                    ctx.state.show_theme_selector = false;
                    ctx.state.theme_before_selector = None;
                }
                _ => {}
            }
        }
        Mode::Normal if ctx.state.show_help => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                ctx.state.help_scroll = ctx.state.help_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                ctx.state.help_scroll = ctx.state.help_scroll.saturating_sub(1);
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                ctx.state.show_help = false;
            }
            _ => {}
        },
        Mode::Normal => match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char(':')) => {
                ctx.state.mode = Mode::Command;
                ctx.state.command_input = String::new();
            }
            (KeyModifiers::NONE, KeyCode::Char('i')) => {
                ctx.state.mode = Mode::Insert;
                ctx.state.focus = Focus::Search;
            }

            (KeyModifiers::NONE, KeyCode::Tab) => {
                let idx = ctx
                    .state
                    .providers
                    .iter()
                    .position(|p| p == &ctx.state.active_provider)
                    .unwrap_or(0);
                let next_idx = (idx + 1) % ctx.state.providers.len();
                let next_provider = ctx.state.providers[next_idx].clone();
                super::search::handle(Action::SwitchProvider(next_provider), ctx);
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                let idx = ctx
                    .state
                    .providers
                    .iter()
                    .position(|p| p == &ctx.state.active_provider)
                    .unwrap_or(0);
                let next_idx =
                    (idx + ctx.state.providers.len() - 1) % ctx.state.providers.len();
                let next_provider = ctx.state.providers[next_idx].clone();
                super::search::handle(Action::SwitchProvider(next_provider), ctx);
            }

            (KeyModifiers::CONTROL, KeyCode::Left)
            | (KeyModifiers::NONE, KeyCode::Left)
            | (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                ctx.state.focus = Focus::Search;
            }
            (KeyModifiers::CONTROL, KeyCode::Right)
            | (KeyModifiers::NONE, KeyCode::Right)
            | (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                if ctx.state.show_now_playing {
                    ctx.state.focus = Focus::NowPlaying;
                } else if ctx.state.show_logs {
                    ctx.state.focus = Focus::Logs;
                }
            }

            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                super::now_playing::handle(Action::NowPlayingAdd, ctx);
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                super::now_playing::handle(Action::NowPlayingReplace, ctx);
            }
            (KeyModifiers::SHIFT, KeyCode::Char('A')) => {
                super::now_playing::handle(Action::NowPlayingAddAll, ctx);
            }
            (KeyModifiers::SHIFT, KeyCode::Char('R')) => {
                super::now_playing::handle(Action::NowPlayingReplaceAll, ctx);
            }

            (KeyModifiers::SHIFT, KeyCode::Char('H')) if ctx.state.focus == Focus::NowPlaying => {
                super::now_playing::handle(Action::NowPlayingBack, ctx);
            }
            (KeyModifiers::SHIFT, KeyCode::Char('L')) if ctx.state.focus == Focus::NowPlaying => {
                super::now_playing::handle(Action::NowPlayingForward, ctx);
            }

            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                super::search::handle(Action::CursorDown, ctx);
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                super::search::handle(Action::CursorUp, ctx);
            }

            // gg -> first
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                if let Some(KeyEvent {
                    code: KeyCode::Char('g'),
                    ..
                }) = last_key
                {
                    if ctx.state.focus == Focus::NowPlaying {
                        if let Some(ref mut np) = ctx.state.now_playing {
                            np.cursor = 0;
                        }
                    } else {
                        let search = ctx.state.get_active_search_mut();
                        search.cursor = 0;
                        preload_selected_track(ctx.state, ctx.plugins);
                    }
                } else {
                    ctx.state.last_key = Some(key);
                }
            }
            // G -> last
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                if ctx.state.focus == Focus::NowPlaying {
                    if let Some(ref mut np) = ctx.state.now_playing
                        && !np.tracks.is_empty()
                    {
                        np.cursor = np.tracks.len() - 1;
                    }
                } else {
                    let search = ctx.state.get_active_search_mut();
                    if !search.results.is_empty() {
                        search.cursor = search.results.len() - 1;
                        preload_selected_track(ctx.state, ctx.plugins);
                    }
                }
            }

            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                super::now_playing::handle(Action::ToggleNowPlaying, ctx);
            }
            (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                super::playback::handle(Action::PlayPause, ctx);
            }
            (KeyModifiers::SHIFT, KeyCode::Char('S')) => {
                super::playback::handle(Action::Stop, ctx);
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                super::playback::handle(Action::PlaySelected, ctx);
            }
            (KeyModifiers::NONE, KeyCode::Backspace) if ctx.state.focus == Focus::Search => {
                super::search::handle(Action::GoBack, ctx);
            }

            (KeyModifiers::NONE, KeyCode::Char('p')) => {
                super::now_playing::handle(Action::ToggleAutoplayAdd, ctx);
            }

            (KeyModifiers::NONE, KeyCode::Char('[')) => {
                super::playback::handle(Action::VolumeDown, ctx);
            }
            (KeyModifiers::NONE, KeyCode::Char(']')) => {
                super::playback::handle(Action::VolumeUp, ctx);
            }
            (KeyModifiers::NONE, KeyCode::Char('m')) => {
                super::playback::handle(Action::ToggleMute, ctx);
            }
            (KeyModifiers::NONE, KeyCode::Char('.')) => {
                super::playback::handle(Action::SeekForward(Duration::from_secs(5)), ctx);
            }
            (KeyModifiers::NONE, KeyCode::Char(',')) => {
                super::playback::handle(Action::SeekBackward(Duration::from_secs(5)), ctx);
            }

            (KeyModifiers::NONE, KeyCode::Char('q')) if ctx.state.focus == Focus::Logs => {
                ctx.state.show_logs = false;
                ctx.state.focus = Focus::Search;
            }

            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                super::search::handle(Action::SwitchProvider("bandcamp".into()), ctx);
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                super::search::handle(Action::SwitchProvider("youtube".into()), ctx);
            }
            _ => {}
        },
        Mode::Insert => match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => ctx.state.mode = Mode::Normal,
            (KeyModifiers::NONE, KeyCode::Enter) => {
                super::search::handle(Action::SearchSubmit, ctx);
                ctx.state.mode = Mode::Normal;
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                ctx.state.get_active_search_mut().input.pop();
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                ctx.state.get_active_search_mut().input.push(c);
            }
            _ => {}
        },
        Mode::Command => match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => ctx.state.mode = Mode::Normal,
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                ctx.state.command_input.pop();
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                ctx.state.command_input.push(c);
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                return execute_command(ctx);
            }
            _ => {}
        },
    }
    false
}
