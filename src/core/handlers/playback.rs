use std::time::Duration;

use notify_rust::Notification;

use crate::core::action::{Action, PlayerEvent, ResultType, Track};
use crate::core::state::{Focus, PlaybackStatus};
use crate::player::Player;

use super::Ctx;

pub fn handle(action: Action, ctx: &mut Ctx) -> bool {
    match action {
        Action::Play(track) => handle_play(track, ctx),
        Action::PlayPause => handle_play_pause(ctx),
        Action::Stop => handle_stop(ctx),
        Action::PlaySelected => handle_play_selected(ctx),
        Action::SeekForward(dur) => handle_seek_forward(dur, ctx),
        Action::SeekBackward(dur) => handle_seek_backward(dur, ctx),
        Action::VolumeUp => handle_volume_up(ctx),
        Action::VolumeDown => handle_volume_down(ctx),
        Action::ToggleMute => handle_toggle_mute(ctx),
        Action::PlayerEvent(event) => return handle_player_event(event, ctx),
        Action::Skip => {}
        _ => {}
    }
    false
}

pub fn handle_play(track: Track, ctx: &mut Ctx) {
    let mut track_to_play = track.clone();
    let track_info = format!("{} by {}", track.title, track.artist);
    let _ = Notification::new()
        .summary("Audiplayer")
        .body(&format!("Now Playing:\n{}", track_info))
        .icon("audio-x-generic")
        .show();

    if let Some(stream_url) = &track.stream_url {
        track_to_play.url = stream_url.clone();
        ctx.state.playback.track = Some(track_to_play.clone());
        ctx.state.playback.status = PlaybackStatus::Playing;
        ctx.state.playback.status_message = None;

        let player_clone = ctx.player.clone();
        tokio::spawn(async move {
            let _ = player_clone.play(&track_to_play).await;
        });
    } else {
        ctx.state.playback.track = Some(track.clone());
        ctx.state.playback.status = PlaybackStatus::Playing;
        ctx.state.playback.status_message = Some("Resolving URL...".to_string());

        let track_clone = track.clone();
        let plugins_clone = ctx.plugins.clone();
        tokio::spawn(async move {
            plugins_clone.resolve_stream_url(track_clone).await;
        });
    }
}

fn handle_play_pause(ctx: &mut Ctx) {
    match ctx.state.playback.status {
        PlaybackStatus::Playing => {
            ctx.state.playback.status = PlaybackStatus::Paused;
            let player_clone = ctx.player.clone();
            tokio::spawn(async move {
                let _ = player_clone.pause().await;
            });
        }
        PlaybackStatus::Paused => {
            ctx.state.playback.status = PlaybackStatus::Playing;
            let player_clone = ctx.player.clone();
            tokio::spawn(async move {
                let _ = player_clone.resume().await;
            });
        }
        PlaybackStatus::Stopped => {}
    }
}

fn handle_stop(ctx: &mut Ctx) {
    ctx.state.playback.status = PlaybackStatus::Stopped;
    ctx.state.playback.track = None;
    ctx.state.playback.status_message = None;
    ctx.state.playback.position = Duration::from_secs(0);
    ctx.state.playback.duration = Duration::from_secs(0);
    ctx.state.playback.percent = 0;
    let player_clone = ctx.player.clone();
    tokio::spawn(async move {
        let _ = player_clone.stop().await;
    });
}

fn handle_play_selected(ctx: &mut Ctx) {
    if ctx.state.focus == Focus::NowPlaying {
        if let Some(ref mut np) = ctx.state.now_playing
            && np.cursor < np.tracks.len()
        {
            np.current_index = np.cursor;
            let track = np.tracks[np.cursor].clone();
            handle_play(track, ctx);
        }
    } else {
        let search = ctx.state.get_active_search();
        if let Some(track) = search.results.get(search.cursor) {
            match track.result_type {
                ResultType::Album => {
                    let track_clone = track.clone();
                    super::search::handle_fetch_album_tracks(track_clone, ctx);
                }
                ResultType::Artist => {
                    ctx.state.logs.push(format!(
                        "Discography fetch for artist {} not implemented yet",
                        track.artist
                    ));
                }
                ResultType::Track => {
                    let track_clone = track.clone();
                    handle_play(track_clone, ctx);
                }
            }
        }
    }
}

fn handle_seek_forward(dur: Duration, ctx: &mut Ctx) {
    let player_clone = ctx.player.clone();
    tokio::spawn(async move {
        let _ = player_clone.seek_relative(dur.as_secs() as i64).await;
    });
}

fn handle_seek_backward(dur: Duration, ctx: &mut Ctx) {
    let player_clone = ctx.player.clone();
    tokio::spawn(async move {
        let _ = player_clone.seek_relative(-(dur.as_secs() as i64)).await;
    });
}

fn handle_volume_up(ctx: &mut Ctx) {
    ctx.state.playback.volume = (ctx.state.playback.volume + 5).min(100);
    let volume = ctx.state.playback.volume;
    let player_clone = ctx.player.clone();
    tokio::spawn(async move {
        let _ = player_clone.set_volume(volume).await;
    });
}

fn handle_volume_down(ctx: &mut Ctx) {
    ctx.state.playback.volume = ctx.state.playback.volume.saturating_sub(5);
    let volume = ctx.state.playback.volume;
    let player_clone = ctx.player.clone();
    tokio::spawn(async move {
        let _ = player_clone.set_volume(volume).await;
    });
}

fn handle_toggle_mute(ctx: &mut Ctx) {
    ctx.state.playback.muted = !ctx.state.playback.muted;
    let muted = ctx.state.playback.muted;
    let player_clone = ctx.player.clone();
    tokio::spawn(async move {
        let _ = player_clone.set_mute(muted).await;
    });
}

fn handle_player_event(event: PlayerEvent, ctx: &mut Ctx) -> bool {
    match event {
        PlayerEvent::TrackEnded => {
            let should_advance = ctx
                .state
                .now_playing
                .as_ref()
                .is_some_and(|np| np.current_index + 1 < np.tracks.len());
            if should_advance {
                let np = ctx.state.now_playing.as_mut().unwrap();
                np.current_index += 1;
                np.cursor = np.current_index;
                let next_track = np.tracks[np.current_index].clone();
                handle_play(next_track, ctx);
            } else {
                ctx.state.playback.status = PlaybackStatus::Stopped;
                ctx.state.playback.status_message = None;
                ctx.state.playback.position = Duration::from_secs(0);
                ctx.state.playback.duration = Duration::from_secs(0);
                ctx.state.playback.percent = 0;
            }
        }
        PlayerEvent::TimePosChanged(pos) => ctx.state.playback.position = pos,
        PlayerEvent::DurationChanged(dur) => ctx.state.playback.duration = dur,
        PlayerEvent::PercentChanged(per) => ctx.state.playback.percent = per,
        _ => {}
    }
    false
}
