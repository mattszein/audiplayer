mod core;
mod tui;
mod player;
mod plugins;

use anyhow::Result;
use core::state::AppState;
use tokio::sync::mpsc;
use tui::Tui;
use player::mpv::MpvPlayer;
use plugins::PluginManager;
use std::fs::File;
use std::os::unix::io::AsRawFd;

#[tokio::main]
async fn main() -> Result<()> {
    // Redirect stderr to a file to prevent corrupting the TUI
    let log_file = File::create("audiplayer.log")?;
    let fd = log_file.as_raw_fd();
    unsafe {
        libc::dup2(fd, libc::STDERR_FILENO);
    }

    // 1. Communication channel — everything flows through here
    let (action_tx, action_rx) = mpsc::channel(256);

    // 2. Subsystems
    let player = MpvPlayer::spawn(action_tx.clone()).await?;
    let plugins = PluginManager::new(action_tx.clone());

    // 3. Initial application state
    let state = AppState::new();

    // 4. Terminal UI (owns the terminal, sends Actions on input)
    let tui = Tui::new(action_tx.clone())?;

    // 5. Run the event loop
    core::event_loop::run(state, tui, player, plugins, action_rx).await?;

    Ok(())
}
