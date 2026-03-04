use std::time::Duration;
use std::collections::{HashSet, HashMap};
use crate::core::action::Track;
use crate::core::Mode;
use crossterm::event::KeyEvent;

/// Which panel currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Player,
    Search,
    Logs,
    NowPlaying,
}

/// A playing context: list of tracks, current playing index, and scroll cursor.
#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub tracks: Vec<Track>,
    pub current_index: usize, // currently playing track
    pub cursor: usize,        // scroll/selection cursor in the panel
}

/// Playback status of the audio engine.
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackStatus {
    Stopped,
    Playing,
    Paused,
}

/// Everything the player bar needs to render.
#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub status: PlaybackStatus,
    pub position: Duration,
    pub duration: Duration,
    pub percent: u8,
    pub track: Option<Track>,
    pub last_mpv_line: Option<String>,
}

impl PlaybackState {
    pub fn format_duration(d: Duration) -> String {
        let secs = d.as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        if h > 0 {
            format!("{:02}:{:02}:{:02}", h, m, s)
        } else {
            format!("{:02}:{:02}", m, s)
        }
    }
}

/// Everything the search panel needs to render for a SPECIFIC provider.
#[derive(Debug, Clone)]
pub struct SearchState {
    pub input: String,
    pub results: Vec<Track>,
    pub cursor: usize,       // selected row in the results list
    pub is_loading: bool,
    pub resolving: HashSet<String>, // IDs currently being preloaded
    pub history: Vec<(Vec<Track>, usize)>, // (results, cursor)
    pub breadcrumbs: Vec<String>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            results: Vec::new(),
            cursor: 0,
            is_loading: false,
            resolving: HashSet::new(),
            history: Vec::new(),
            breadcrumbs: vec!["Search".to_string()],
        }
    }
}

/// The single source of truth. The TUI reads this; the event loop mutates it.
#[derive(Debug)]
pub struct AppState {
    pub focus: Focus,
    pub mode: Mode,
    pub command_input: String,
    pub playback: PlaybackState,
    
    // Per-provider state
    pub search_states: HashMap<String, SearchState>,
    pub active_provider: String,
    pub providers: Vec<String>,

    pub show_logs: bool,
    pub logs: Vec<String>,

    pub now_playing: Option<NowPlaying>,
    pub now_playing_history: Vec<NowPlaying>,
    pub now_playing_future: Vec<NowPlaying>,
    pub show_now_playing: bool,
    pub autoplay_add: bool,

    pub last_key: Option<KeyEvent>,
}

impl AppState {
    pub fn new() -> Self {
        let providers = vec!["bandcamp".to_string(), "youtube".to_string()];
        let mut search_states = HashMap::new();
        for p in &providers {
            search_states.insert(p.clone(), SearchState::new());
        }

        Self {
            focus: Focus::Search,
            mode: Mode::Normal,
            command_input: String::new(),
            playback: PlaybackState {
                status: PlaybackStatus::Stopped,
                position: Duration::from_secs(0),
                duration: Duration::from_secs(0),
                percent: 0,
                track: None,
                last_mpv_line: None,
            },
            search_states,
            active_provider: "bandcamp".to_string(),
            providers,
            show_logs: false,
            logs: Vec::new(),
            now_playing: None,
            now_playing_history: Vec::new(),
            now_playing_future: Vec::new(),
            show_now_playing: false,
            autoplay_add: false,
            last_key: None,
        }
    }

    pub fn get_active_search_mut(&mut self) -> &mut SearchState {
        self.search_states.get_mut(&self.active_provider).expect("Active provider missing from states")
    }

    pub fn get_active_search(&self) -> &SearchState {
        self.search_states.get(&self.active_provider).expect("Active provider missing from states")
    }
}
