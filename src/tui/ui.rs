use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Tabs},
};
use std::time::Duration;

use crate::core::Mode;
use crate::core::action::ResultType;
use crate::core::state::{AppState, Focus, PlaybackStatus};

/// Colour palette — change here to retheme the whole app.
mod colors {
    use ratatui::style::Color;
    pub const BORDER_FOCUSED: Color = Color::Cyan;
    pub const BORDER_INACTIVE: Color = Color::DarkGray;
    pub const TITLE: Color = Color::White;
    pub const SELECTED_BG: Color = Color::DarkGray;
    pub const SELECTED_FG: Color = Color::Cyan;
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let m = secs / 60;
    let s = secs % 60;
    format!("{:02}:{:02}", m, s)
}

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let mode_str = match state.mode {
        Mode::Normal => " NORMAL ",
        Mode::Insert => " INSERT ",
        Mode::Command => " COMMAND ",
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title_top(Line::from(vec![
            Span::styled(" ♪ Audiplayer ", Style::default().fg(colors::TITLE).add_modifier(Modifier::BOLD)),
        ]).alignment(Alignment::Center))
        .title_bottom(Line::from(vec![
            Span::styled(mode_str, Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)),
            if state.mode == Mode::Command {
                Span::styled(format!(" :{} ", state.command_input), Style::default().fg(Color::Yellow))
            } else {
                Span::raw(" [Ctrl + hljk] focus | [tab] switch provider | [i] search | [space] pause | :l log, :q quit ")
            }
        ]));

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Player
                Constraint::Min(0),    // Main Content (Search/Results)
            ])
            .split(inner_area);
    
        render_player(frame, main_layout[0], state);
    
        let content_area = main_layout[1];
        let show_right = state.show_now_playing || state.show_logs;
        if show_right {
            let content_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(content_area);

            render_search(frame, content_layout[0], state);
            if state.show_now_playing {
                render_now_playing(frame, content_layout[1], state);
            } else {
                render_logs(frame, content_layout[1], state);
            }
        } else {
            render_search(frame, content_area, state);
        }
    }
fn render_breadcrumbs(frame: &mut Frame, area: Rect, state: &AppState) {
    let s = state.get_active_search();
    let spans: Vec<Span> = s
        .breadcrumbs
        .iter()
        .enumerate()
        .flat_map(|(i, b)| {
            let mut parts = vec![Span::styled(
                format!(" {} ", b),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )];
            if i < s.breadcrumbs.len() - 1 {
                parts.push(Span::styled(" » ", Style::default().fg(Color::DarkGray)));
            }
            parts
        })
        .collect();

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_player(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = &state.playback;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Player ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1)])
        .split(inner);

    let (title, artist) = match &p.track {
        Some(t) => (t.title.as_str(), t.artist.as_str()),
        None => ("No track selected", ""),
    };

    let status_icon = match p.status {
        PlaybackStatus::Playing => "▶",
        PlaybackStatus::Paused => "⏸",
        PlaybackStatus::Stopped => "⏹",
    };

    let mut spans = vec![
        Span::styled(
            format!(" {} ", status_icon),
            Style::default().fg(Color::Green),
        ),
    ];
    if state.autoplay_add {
        spans.push(Span::styled(
            "[AP] ",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ));
    }
    spans.extend([
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" — "),
        Span::styled(artist, Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(
            p.last_mpv_line.as_deref().unwrap_or(""),
            Style::default().fg(Color::Yellow),
        ),
    ]);
    let player_line = Line::from(spans);

    frame.render_widget(Paragraph::new(player_line), rows[0]);
}

fn render_search(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Search;
    let s = state.get_active_search();
    
    let bottom_title = if let Some(r) = s.results.get(s.cursor) {
        let action_hint = match r.result_type {
            ResultType::Album => " [Enter: View Album] ",
            ResultType::Artist => " [Enter: View Artist (N/A)] ",
            ResultType::Track => " [Enter: Play Track] ",
        };

        Some(Line::from(vec![
            Span::styled(
                " Selection: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(r.title.clone(), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" by "),
            Span::styled(r.artist.clone(), Style::default().fg(Color::Cyan)),
            if let Some(ref album) = r.album {
                Span::raw(format!(" (Album: {})", album))
            } else {
                Span::raw("")
            },
            Span::raw(" "),
            Span::styled(
                action_hint.to_string(),
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
    } else {
        None
    };

    let block = focused_block(focused, " Search ".to_string(), bottom_title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tabs
            Constraint::Length(3), // Input
            Constraint::Length(1), // Breadcrumbs
            Constraint::Min(0),    // Results
        ])
        .split(inner);

    render_tabs(frame, rows[0], state);
    render_search_input(frame, rows[1], state);
    render_breadcrumbs(frame, rows[2], state);
    render_results(frame, rows[3], state);
}

fn render_tabs(frame: &mut Frame, area: Rect, state: &AppState) {
    let titles: Vec<Line> = state
        .providers
        .iter()
        .map(|p| Line::from(vec![Span::raw(p.to_uppercase())]))
        .collect();

    let active_idx = state
        .providers
        .iter()
        .position(|p| p == &state.active_provider)
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .select(active_idx)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" | ");

    frame.render_widget(tabs, area);
}

fn render_search_input(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.mode == Mode::Insert && state.focus == Focus::Search;
    let search = state.get_active_search();
    let input = &search.input;

    let display = if focused {
        format!("{}_", input)
    } else {
        input.clone()
    };
    let cursor_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let paragraph = Paragraph::new(Span::styled(display, cursor_style)).block(block);
    frame.render_widget(paragraph, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    let s = s.trim();
    if s.chars().count() > max_len {
        let mut t = s
            .chars()
            .take(max_len.saturating_sub(1))
            .collect::<String>();
        t.push('…');
        t
    } else {
        s.to_string()
    }
}

fn render_results(frame: &mut Frame, area: Rect, state: &AppState) {
    let s = state.get_active_search();

    if s.is_loading {
        frame.render_widget(
            Paragraph::new("  Loading…").style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }

    if s.results.is_empty() {
        let hint = if s.input.is_empty() {
            "  'i': search, 'hjkl': nav, 'tab': switch"
        } else {
            "  Enter: search"
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let total_width = area.width as usize;
    let index_width = 3;
    let type_width = 5;
    let duration_width = 8;
    let bitrate_width = 10;
    let badge_width = 4;
    let gaps = 12; // total fixed spaces

    let metadata_fixed =
        index_width + type_width + duration_width + bitrate_width + badge_width + gaps;
    let remaining = total_width.saturating_sub(metadata_fixed + 5);

    // Distribute remaining space: 50% title, 50% artist/label
    let title_max = (remaining as f64 * 0.5) as usize;
    let artist_max = remaining.saturating_sub(title_max);

    // Render Headers
    let header_line = Line::from(vec![
        Span::raw("   "),
        Span::styled(
            format!("{:<width$}", "#", width = index_width),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:<width$}", "TITLE", width = title_max),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<width$}", "ARTIST / LABEL", width = artist_max),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<width$}", "TYPE", width = type_width),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:>width$}", "TIME", width = duration_width),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:>width$}", "BITRATE", width = bitrate_width),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "PRE",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(header_line), layout[0]);

    let items: Vec<ListItem> = s
        .results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let selected = i == s.cursor;

            let is_playing = state
                .playback
                .track
                .as_ref()
                .map_or(false, |pt| pt.id == r.id && pt.provider == r.provider);

            let preload_badge = if r.stream_url.is_some() {
                Span::styled(" [P]", Style::default().fg(Color::Green))
            } else if s.resolving.contains(&r.id) {
                Span::styled(" [~]", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("    ")
            };

            let duration_str = r
                .duration
                .map(format_duration)
                .unwrap_or_else(|| "--:--".to_string());
            let bitrate_str = r
                .bitrate
                .map(|b| format!("{}kbps", b))
                .unwrap_or_else(|| "--kbps".to_string());
            let type_tag = match r.result_type {
                ResultType::Track => "Track",
                ResultType::Album => "Album",
                ResultType::Artist => "Artist",
            };

            // Format columns with padding
            let title_col = format!(
                "{:<width$}",
                truncate(&r.title, title_max),
                width = title_max
            );
            let artist_col = format!(
                "{:<width$}",
                truncate(&r.artist, artist_max),
                width = artist_max
            );

            let (title_style, artist_style, type_style, time_style, bit_style, index_style) =
                if is_playing {
                    let fg = Color::Green;
                    (
                        Style::default().fg(fg).add_modifier(Modifier::BOLD),
                        Style::default().fg(fg),
                        Style::default().fg(fg),
                        Style::default().fg(fg),
                        Style::default().fg(fg),
                        Style::default().fg(fg),
                    )
                } else {
                    (
                        if selected {
                            Style::default()
                                .fg(colors::SELECTED_FG)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                        Style::default().fg(Color::Gray),
                        Style::default().fg(Color::Blue),
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::DarkGray),
                        Style::default().fg(Color::DarkGray),
                    )
                };

            let icon = if is_playing {
                "♫"
            } else if selected {
                "▶"
            } else {
                " "
            };
            let icon_style = if is_playing {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(colors::SELECTED_FG)
            };

            let line = Line::from(vec![
                Span::styled(format!(" {} ", icon), icon_style),
                Span::styled(
                    format!("{:<width$}", i + 1, width = index_width),
                    index_style,
                ),
                Span::raw(" "),
                Span::styled(title_col, title_style),
                Span::raw("  "),
                Span::styled(artist_col, artist_style),
                Span::raw("  "),
                Span::styled(
                    format!("{:<width$}", type_tag, width = type_width),
                    type_style,
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:>width$}", duration_str, width = duration_width),
                    time_style,
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:>width$}", bitrate_str, width = bitrate_width),
                    bit_style,
                ),
                preload_badge,
            ]);

            let style = if selected {
                Style::default().bg(colors::SELECTED_BG)
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(s.cursor));
    frame.render_stateful_widget(List::new(items), layout[1], &mut list_state);
}

fn render_now_playing(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::NowPlaying;
    let np = match &state.now_playing {
        Some(np) => np,
        None => {
            let block = focused_block(focused, " Now Playing ".to_string(), None);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                Paragraph::new("  No tracks").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return;
        }
    };

    // Panel title: "Now Playing" or "History N" depending on navigation depth
    let depth = state.now_playing_future.len();
    let panel_title = if depth == 0 {
        " Now Playing ".to_string()
    } else {
        format!(" History {} ", depth)
    };

    let has_history = !state.now_playing_history.is_empty() || !state.now_playing_future.is_empty();
    let bottom_title = if has_history {
        let total = state.now_playing_history.len() + 1 + state.now_playing_future.len();
        let pos = state.now_playing_history.len() + 1;
        Some(Line::from(vec![
            Span::styled(format!(" [{}/{}] ", pos, total), Style::default().fg(Color::DarkGray)),
            Span::styled(" Shift+H/L: navigate ", Style::default().fg(Color::DarkGray)),
        ]))
    } else {
        None
    };

    let block = focused_block(focused, panel_title, bottom_title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let playing_track_id = state.playback.track.as_ref()
        .filter(|_| state.playback.status != PlaybackStatus::Stopped)
        .map(|t| t.id.as_str());

    let items: Vec<ListItem> = np
        .tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_playing = playing_track_id == Some(track.id.as_str());
            let is_selected = focused && i == np.cursor;
            let prefix = if is_playing { "♫ " } else if is_selected { "▶ " } else { "  " };
            let duration_str = track
                .duration
                .map(format_duration)
                .unwrap_or_default();
            let text = format!("{}{}.  {}  {}", prefix, i + 1, track.title, duration_str);

            let style = if is_playing {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(colors::SELECTED_FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let bg = if is_selected {
                Style::default().bg(colors::SELECTED_BG)
            } else if is_playing {
                Style::default().bg(Color::Rgb(30, 30, 0))
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(text, style))).style(bg)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(np.cursor));
    frame.render_stateful_widget(List::new(items), inner, &mut list_state);
}

fn render_logs(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Logs;
    let block = focused_block(focused, " Logs ".to_string(), None);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let log_lines: Vec<ListItem> = state
        .logs
        .iter()
        .rev()
        .map(|log| {
            ListItem::new(Line::from(vec![Span::styled(
                log,
                Style::default().fg(Color::Red),
            )]))
        })
        .collect();

    frame.render_widget(List::new(log_lines), inner);
}

fn focused_block<'a>(focused: bool, title: String, bottom_title: Option<Line<'a>>) -> Block<'a> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(style)
        .title(Span::styled(title, style));
    
    if let Some(bt) = bottom_title {
        block = block.title_bottom(bt);
    }
    
    block
}
