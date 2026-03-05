use std::time::Duration;
use crossterm::event::KeyEvent;
use crate::core::Mode;

#[derive(Debug, Clone, PartialEq)]
pub enum ResultType {
    Track,
    Album,
    Artist,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Action {
    // ── User intent ──────────────────────────────────────────────
    Search { provider: String, query: String },
    Play(Track),
    Pause,
    Resume,
    Stop,
    PlayPause,
    PlaySelected,
    Skip,
    SeekTo(Duration),
    SeekForward(Duration),
    SeekBackward(Duration),
    VolumeUp,
    VolumeDown,
    ToggleMute,
    EnqueueTrack(Track),
    SwitchProvider(String),
    SetMode(Mode),

    // ── Focus & UI Navigation ─────────────────────────────────────
    FocusPlayer,
    FocusSearch,
    FocusLogs,
    ToggleLogs,
    Log(String),
    SearchInput(char),
    SearchBackspace,
    SearchSubmit,
    CursorDown,
    CursorUp,
    GoBack,

    // ── Command Line ──────────────────────────────────────────────
    CommandInput(char),
    CommandBackspace,
    CommandExecute,

    // ── Resolve Stream URL ────────────────────────────────────────
    ResolveStreamUrl(Track),
    FetchAlbumTracks(Track),

    // ── Input ─────────────────────────────────────────────────────
    Key(KeyEvent),

    // ── Player feedback ───────────────────────────────────────────
    PlayerEvent(PlayerEvent),

    // ── Plugin responses ──────────────────────────────────────────
    PluginResponse { id: String, result: PluginResult },

    // ── Now Playing ──────────────────────────────────────────────
    ToggleNowPlaying,
    NowPlayingAdd,         // Add selected track to now playing
    NowPlayingReplace,     // Replace now playing with selected track
    NowPlayingAddAll,      // Add all search results to now playing
    NowPlayingReplaceAll,  // Replace now playing with all search results
    NowPlayingBack,
    NowPlayingForward,
    ToggleAutoplayAdd,

    // ── UI ─────────────────────────────────────────────────────────
    OpenThemeSelector,
    CycleThemeMode,
    ToggleHelp,

    // ── App lifecycle ─────────────────────────────────────────────
    Quit,
    Tick,
    Resize(u16, u16),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub artist_id: Option<String>,
    pub album_id: Option<String>,
    pub url: String,
    pub stream_url: Option<String>,
    pub provider: String,
    pub duration: Option<Duration>,
    pub bitrate: Option<u32>, // in kbps
    pub result_type: ResultType,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackEnded,
    TimePosChanged(Duration),
    DurationChanged(Duration),
    PercentChanged(u8),
    Stopped,
    MetadataLoaded {
        title: Option<String>,
        artist: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum PluginResult {
    Search(Vec<Track>),
    AlbumTracks(Vec<Track>),
    StreamUrl { 
        track_id: String, 
        url: String,
        duration: Option<Duration>,
        bitrate: Option<u32>,
    },
    Error(String),
}
