use std::sync::Arc;

use crate::core::action::{Action, ResultType, Track};
use crate::core::state::{AppState, Focus};
use crate::plugins::PluginManager;

use super::Ctx;

pub fn handle(action: Action, ctx: &mut Ctx) -> bool {
    match action {
        Action::SearchInput(c) => ctx.state.get_active_search_mut().input.push(c),
        Action::SearchBackspace => {
            ctx.state.get_active_search_mut().input.pop();
        }
        Action::SearchSubmit => handle_search_submit(ctx),
        Action::CursorDown => handle_cursor_down(ctx),
        Action::CursorUp => handle_cursor_up(ctx),
        Action::GoBack => handle_go_back(ctx),
        Action::SwitchProvider(provider) => handle_switch_provider(provider, ctx),
        Action::FetchAlbumTracks(track) => handle_fetch_album_tracks(track, ctx),
        _ => {}
    }
    false
}

fn handle_search_submit(ctx: &mut Ctx) {
    let provider = ctx.state.active_provider.clone();
    let search = ctx.state.get_active_search_mut();
    search.is_loading = true;
    search.resolving.clear();
    search.history.clear();
    search.breadcrumbs = vec!["Search".to_string(), search.input.clone()];
    let query = search.input.clone();
    let plugins_clone = ctx.plugins.clone();
    tokio::spawn(async move {
        plugins_clone.handle_search(&provider, query).await;
    });
}

fn handle_cursor_down(ctx: &mut Ctx) {
    if ctx.state.focus == Focus::Search {
        let search = ctx.state.get_active_search_mut();
        if !search.results.is_empty() {
            search.cursor = (search.cursor + 1).min(search.results.len() - 1);
            preload_selected_track(ctx.state, ctx.plugins);
        }
    } else if ctx.state.focus == Focus::NowPlaying
        && let Some(ref mut np) = ctx.state.now_playing
        && !np.tracks.is_empty()
    {
        np.cursor = (np.cursor + 1).min(np.tracks.len() - 1);
    }
}

fn handle_cursor_up(ctx: &mut Ctx) {
    if ctx.state.focus == Focus::Search {
        let search = ctx.state.get_active_search_mut();
        search.cursor = search.cursor.saturating_sub(1);
        preload_selected_track(ctx.state, ctx.plugins);
    } else if ctx.state.focus == Focus::NowPlaying
        && let Some(ref mut np) = ctx.state.now_playing
    {
        np.cursor = np.cursor.saturating_sub(1);
    }
}

fn handle_go_back(ctx: &mut Ctx) {
    let search = ctx.state.get_active_search_mut();
    if let Some((old_results, old_cursor)) = search.history.pop() {
        search.results = old_results;
        search.cursor = old_cursor;
        search.breadcrumbs.pop();
    }
}

fn handle_switch_provider(provider: String, ctx: &mut Ctx) {
    if ctx.state.providers.contains(&provider) {
        ctx.state.active_provider = provider;
        preload_selected_track(ctx.state, ctx.plugins);
    }
}

pub fn handle_fetch_album_tracks(track: Track, ctx: &mut Ctx) {
    let provider = ctx.state.active_provider.clone();
    let search = ctx.state.get_active_search_mut();

    search.history.push((search.results.clone(), search.cursor));
    search.breadcrumbs.push(track.title.clone());

    search.is_loading = true;
    search.resolving.clear();
    let plugins_clone = ctx.plugins.clone();
    tokio::spawn(async move {
        plugins_clone
            .handle_fetch_album_tracks(&provider, track)
            .await;
    });
}

pub fn preload_selected_track(state: &mut AppState, plugins: &Arc<PluginManager>) {
    let active_provider = state.active_provider.clone();
    let search = state.search_states.get_mut(&active_provider).unwrap();
    if let Some(track) = search.results.get(search.cursor)
        && track.result_type == ResultType::Track
        && track.stream_url.is_none()
        && !search.resolving.contains(&track.id)
    {
        let track_clone = track.clone();
        let plugins_clone = plugins.clone();
        search.resolving.insert(track.id.clone());
        tokio::spawn(async move {
            plugins_clone.resolve_stream_url(track_clone).await;
        });
    }
}
