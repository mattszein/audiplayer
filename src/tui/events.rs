use crossterm::event::{self, Event, KeyEvent};
use tokio::sync::mpsc::Sender;

use crate::core::action::Action;

/// Spawns a blocking thread that reads keyboard events and converts them
/// into Actions.
pub fn spawn_input_listener(action_tx: Sender<Action>) {
    std::thread::spawn(move || loop {
        match event::read() {
            Ok(Event::Key(key)) => {
                if action_tx.blocking_send(Action::Key(key)).is_err() {
                    break;
                }
            }
            Ok(Event::Resize(w, h)) => {
                if action_tx.blocking_send(Action::Resize(w, h)).is_err() {
                    break;
                }
            }
            _ => {}
        }
    });
}
