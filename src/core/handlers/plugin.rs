use crate::core::action::{Action, PluginResult};
use crate::core::state::PlaybackStatus;
use crate::player::Player;

use super::Ctx;
use super::search::preload_selected_track;

pub fn handle(action: Action, ctx: &mut Ctx) -> bool {
    if let Action::PluginResponse { id, result } = action {
        match result {
            PluginResult::Search(tracks) => {
                if let Some(search) = ctx.state.search_states.get_mut(&id) {
                    search.results = tracks;
                    search.is_loading = false;
                    search.cursor = 0;
                    search.resolving.clear();
                    if id == ctx.state.active_provider {
                        preload_selected_track(ctx.state, ctx.plugins);
                    }
                }
            }
            PluginResult::AlbumTracks(tracks) => {
                if let Some(search) = ctx.state.search_states.get_mut(&id) {
                    search.results = tracks;
                    search.is_loading = false;
                    search.cursor = 0;
                    search.resolving.clear();
                    if id == ctx.state.active_provider {
                        preload_selected_track(ctx.state, ctx.plugins);
                    }
                }
            }
            PluginResult::StreamUrl {
                track_id,
                url,
                duration,
                bitrate,
            } => {
                for search in ctx.state.search_states.values_mut() {
                    search.resolving.remove(&track_id);
                    for track in &mut search.results {
                        if track.id == track_id {
                            track.stream_url = Some(url.clone());
                            if duration.is_some() {
                                track.duration = duration;
                            }
                            if bitrate.is_some() {
                                track.bitrate = bitrate;
                            }
                        }
                    }
                }
                if let Some(ref mut np) = ctx.state.now_playing {
                    for track in &mut np.tracks {
                        if track.id == track_id {
                            track.stream_url = Some(url.clone());
                            if duration.is_some() {
                                track.duration = duration;
                            }
                            if bitrate.is_some() {
                                track.bitrate = bitrate;
                            }
                        }
                    }
                }
                if let Some(ref mut current) = ctx.state.playback.track
                    && current.id == track_id
                {
                    current.stream_url = Some(url.clone());
                    if duration.is_some() {
                        current.duration = duration;
                    }
                    if bitrate.is_some() {
                        current.bitrate = bitrate;
                    }

                    if ctx.state.playback.status == PlaybackStatus::Playing {
                        let mut t = current.clone();
                        t.url = url;
                        let player_clone = ctx.player.clone();
                        tokio::spawn(async move {
                            let _ = player_clone.play(&t).await;
                        });
                    }
                }
            }
            PluginResult::Error(e) => {
                if let Some(search) = ctx.state.search_states.get_mut(&id) {
                    search.is_loading = false;
                }
                ctx.state.logs.push(format!("Error: {}", e));
            }
        }
    }
    false
}
