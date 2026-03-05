pub mod keys;
pub mod now_playing;
pub mod playback;
pub mod plugin;
pub mod search;
pub mod ui;

use std::sync::Arc;

use crate::core::state::AppState;
use crate::player::mpv::MpvPlayer;
use crate::plugins::PluginManager;

pub struct Ctx<'a> {
    pub state: &'a mut AppState,
    pub player: &'a Arc<MpvPlayer>,
    pub plugins: &'a Arc<PluginManager>,
}
