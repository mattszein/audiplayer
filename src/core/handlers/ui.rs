use crate::core::Mode;
use crate::core::action::Action;
use crate::core::settings::Settings;
use crate::core::state::AppState;
use crate::tui::theme::{Theme, ThemeMode};

use super::Ctx;

pub fn handle(action: Action, ctx: &mut Ctx) -> bool {
    match action {
        Action::OpenThemeSelector => handle_open_theme_selector(ctx.state),
        Action::CycleThemeMode => handle_cycle_theme_mode(ctx.state),
        Action::ToggleHelp => handle_toggle_help(ctx.state),
        Action::OpenConfig => handle_open_config(ctx.state),
        _ => {}
    }
    false
}

pub fn handle_log(msg: String, state: &mut AppState) {
    eprintln!("{}", msg);
    state.logs.push(msg);
    if state.logs.len() > 500 {
        state.logs.remove(0);
    }
}

pub fn handle_set_mode(mode: Mode, state: &mut AppState) {
    state.mode = mode;
    if mode == Mode::Command {
        state.command_input = String::new();
    }
}

fn handle_open_theme_selector(state: &mut AppState) {
    let names = Theme::preset_names();
    let idx = names
        .iter()
        .position(|&n| n == state.theme.name)
        .unwrap_or(0);
    state.theme_selector_cursor = idx;
    state.theme_before_selector = Some(state.theme.name.to_string());
    state.show_theme_selector = true;
}

fn handle_cycle_theme_mode(state: &mut AppState) {
    let new_mode = match state.theme.mode {
        ThemeMode::Dark => ThemeMode::Light,
        ThemeMode::Light => ThemeMode::Dark,
    };
    state.theme = state.theme.with_mode(new_mode);
}

fn handle_toggle_help(state: &mut AppState) {
    state.show_help = !state.show_help;
    state.help_scroll = 0;
}

fn handle_open_config(state: &mut AppState) {
    match Settings::config_path() {
        Some(path) => {
            if !path.exists() {
                if let Some(parent) = path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        handle_log(format!("Failed to create config dir: {}", e), state);
                        return;
                    }
                }
                let example = include_str!("../../../config/example.toml");
                if let Err(e) = std::fs::write(&path, example) {
                    handle_log(format!("Failed to create config file: {}", e), state);
                    return;
                }
                handle_log(format!("Created config: {}", path.display()), state);
            } else {
                handle_log(format!("Config: {}", path.display()), state);
            }
        }
        None => {
            handle_log("Could not determine config directory".to_string(), state);
        }
    }
}
