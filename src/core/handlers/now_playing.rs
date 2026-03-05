use crate::core::action::{Action, ResultType};
use crate::core::state::{AppState, Focus, NowPlaying, PlaybackStatus};

use super::Ctx;

pub fn handle(action: Action, ctx: &mut Ctx) -> bool {
    match action {
        Action::NowPlayingAdd => handle_add(ctx),
        Action::NowPlayingReplace => handle_replace(ctx),
        Action::NowPlayingAddAll => handle_add_all(ctx),
        Action::NowPlayingReplaceAll => handle_replace_all(ctx),
        Action::ToggleNowPlaying => handle_toggle(ctx),
        Action::NowPlayingBack => handle_back(ctx),
        Action::NowPlayingForward => handle_forward(ctx),
        Action::ToggleAutoplayAdd => {
            ctx.state.autoplay_add = !ctx.state.autoplay_add;
        }
        _ => {}
    }
    false
}

fn handle_add(ctx: &mut Ctx) {
    let search = ctx.state.get_active_search();
    if let Some(track) = search.results.get(search.cursor)
        && track.result_type == ResultType::Track
    {
        let track_clone = track.clone();
        if let Some(ref mut np) = ctx.state.now_playing {
            if !np.tracks.iter().any(|t| t.id == track_clone.id) {
                np.tracks.push(track_clone);
            }
        } else {
            ctx.state.now_playing = Some(NowPlaying {
                tracks: vec![track_clone],
                current_index: 0,
                cursor: 0,
            });
        }
        ctx.state.show_now_playing = true;
        maybe_autoplay(ctx);
    }
}

fn handle_replace(ctx: &mut Ctx) {
    let search = ctx.state.get_active_search();
    if let Some(track) = search.results.get(search.cursor)
        && track.result_type == ResultType::Track
    {
        let track_clone = track.clone();
        push_now_playing_to_history(ctx.state);
        ctx.state.now_playing = Some(NowPlaying {
            tracks: vec![track_clone],
            current_index: 0,
            cursor: 0,
        });
        ctx.state.show_now_playing = true;
        maybe_autoplay(ctx);
    }
}

fn handle_add_all(ctx: &mut Ctx) {
    let search = ctx.state.get_active_search();
    let tracks: Vec<_> = search
        .results
        .iter()
        .filter(|t| t.result_type == ResultType::Track)
        .cloned()
        .collect();
    if !tracks.is_empty() {
        if let Some(ref mut np) = ctx.state.now_playing {
            for t in tracks {
                if !np.tracks.iter().any(|existing| existing.id == t.id) {
                    np.tracks.push(t);
                }
            }
        } else {
            ctx.state.now_playing = Some(NowPlaying {
                tracks,
                current_index: 0,
                cursor: 0,
            });
        }
        ctx.state.show_now_playing = true;
        maybe_autoplay(ctx);
    }
}

fn handle_replace_all(ctx: &mut Ctx) {
    let search = ctx.state.get_active_search();
    let tracks: Vec<_> = search
        .results
        .iter()
        .filter(|t| t.result_type == ResultType::Track)
        .cloned()
        .collect();
    if !tracks.is_empty() {
        push_now_playing_to_history(ctx.state);
        ctx.state.now_playing = Some(NowPlaying {
            tracks,
            current_index: 0,
            cursor: 0,
        });
        ctx.state.show_now_playing = true;
        maybe_autoplay(ctx);
    }
}

fn handle_toggle(ctx: &mut Ctx) {
    ctx.state.show_now_playing = !ctx.state.show_now_playing;
    if ctx.state.show_now_playing {
        ctx.state.focus = Focus::NowPlaying;
    } else if ctx.state.focus == Focus::NowPlaying {
        ctx.state.focus = Focus::Search;
    }
}

fn handle_back(ctx: &mut Ctx) {
    if let Some(current) = ctx.state.now_playing.take() {
        if let Some(prev) = ctx.state.now_playing_history.pop() {
            ctx.state.now_playing_future.push(current);
            ctx.state.now_playing = Some(prev);
        } else {
            ctx.state.now_playing = Some(current);
        }
    }
}

fn handle_forward(ctx: &mut Ctx) {
    if let Some(current) = ctx.state.now_playing.take() {
        if let Some(next) = ctx.state.now_playing_future.pop() {
            ctx.state.now_playing_history.push(current);
            ctx.state.now_playing = Some(next);
        } else {
            ctx.state.now_playing = Some(current);
        }
    }
}

fn maybe_autoplay(ctx: &mut Ctx) {
    if !ctx.state.autoplay_add || ctx.state.playback.status != PlaybackStatus::Stopped {
        return;
    }
    if let Some(ref mut np) = ctx.state.now_playing
        && !np.tracks.is_empty()
    {
        np.current_index = 0;
        np.cursor = 0;
        let track = np.tracks[0].clone();
        super::playback::handle_play(track, ctx);
    }
}

fn push_now_playing_to_history(state: &mut AppState) {
    if let Some(current_np) = state.now_playing.take() {
        state.now_playing_history.push(current_np);
        if state.now_playing_history.len() > 5 {
            state.now_playing_history.remove(0);
        }
    }
    state.now_playing_future.clear();
}
