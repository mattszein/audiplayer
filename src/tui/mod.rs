pub mod ui;
pub mod events;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use tokio::sync::mpsc::Sender;

use crate::core::{action::Action, state::AppState};

pub type Term = Terminal<CrosstermBackend<Stdout>>;

/// Owns the terminal. Responsible for setup, teardown, drawing, and
/// spawning the input listener task.
pub struct Tui {
    terminal: Term,
    action_tx: Sender<Action>,
}

impl Tui {
    pub fn new(action_tx: Sender<Action>) -> Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal, action_tx })
    }

    /// Switch to alternate screen and enable raw mode.
    pub fn enter(&mut self) -> Result<()> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;

        // Spawn the input listener — it sends Actions back via action_tx
        events::spawn_input_listener(self.action_tx.clone());

        Ok(())
    }

    /// Render the current state to the terminal.
    pub fn draw(&mut self, state: &AppState) -> Result<()> {
        self.terminal.draw(|frame| ui::render(frame, state))?;
        Ok(())
    }

    /// Restore the terminal to its original state.
    pub fn exit(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}
