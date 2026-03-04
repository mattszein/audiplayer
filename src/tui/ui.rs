use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, LineGauge, List, ListItem, ListState, Paragraph, Tabs,
    },
};
use std::time::Duration;

use crate::core::Mode;
use crate::core::action::ResultType;
use crate::core::state::{AppState, Focus, PlaybackState, PlaybackStatus};
use crate::tui::theme::Theme;

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let m = secs / 60;
    let s = secs % 60;
    format!("{:02}:{:02}", m, s)
}

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let theme = &state.theme;

    let mode_str = match state.mode {
        Mode::Normal => " NORMAL ",
        Mode::Insert => " INSERT ",
        Mode::Command => " COMMAND ",
    };

    // Build bottom bar
    let bottom_line = if state.mode == Mode::Command {
        Line::from(vec![
            Span::styled(mode_str, theme.badge()),
            Span::styled(
                format!(" :{} ", state.command_input),
                Style::default().fg(theme.secondary),
            ),
        ])
    } else {
        let mut spans = vec![
            Span::styled(mode_str, theme.badge()),
            Span::styled(" :h help | :q quit ", theme.muted()),
        ];

        // Append selection info from focused panel
        let selected_track = if state.focus == Focus::NowPlaying {
            state
                .now_playing
                .as_ref()
                .and_then(|np| np.tracks.get(np.cursor))
        } else {
            let s = state.get_active_search();
            s.results.get(s.cursor)
        };
        if let Some(r) = selected_track {
            let action_hint = match r.result_type {
                ResultType::Album => "[Enter: View Album]",
                ResultType::Artist => "[Enter: View Artist]",
                ResultType::Track => "[Enter: Play Track]",
            };
            spans.push(Span::styled(" Selection: ", theme.header()));
            spans.push(Span::styled(
                &r.title,
                Style::default()
                    .fg(theme.default)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(" by ", theme.muted()));
            spans.push(Span::styled(&r.artist, Style::default().fg(theme.primary)));
            if let Some(ref album) = r.album {
                spans.push(Span::styled(format!(" ({})", album), theme.muted()));
            }
            spans.push(Span::raw(" "));
            spans.push(Span::styled(action_hint, theme.badge()));
        }

        Line::from(spans)
    };

    let mode_label = match theme.mode {
        crate::tui::theme::ThemeMode::Dark => "dark",
        crate::tui::theme::ThemeMode::Light => "light",
    };
    let theme_label = Line::from(vec![
        Span::styled(
            format!(" {} ", theme.name),
            Style::default().fg(theme.support),
        ),
        Span::styled(format!("{} ", mode_label), theme.muted()),
    ])
    .alignment(Alignment::Right);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(theme.base())
        .title_top(
            Line::from(vec![Span::styled(
                " ♪ Audiplayer ",
                Style::default()
                    .fg(theme.default)
                    .add_modifier(Modifier::BOLD),
            )])
            .alignment(Alignment::Center),
        )
        .title_bottom(bottom_line)
        .title_bottom(theme_label);

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Player
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

    if state.show_help {
        render_help_overlay(frame, area, state);
    }
    if state.show_theme_selector {
        render_theme_selector(frame, area, state);
    }
}

fn render_breadcrumbs(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let s = state.get_active_search();
    let spans: Vec<Span> = s
        .breadcrumbs
        .iter()
        .enumerate()
        .flat_map(|(i, b)| {
            let mut parts = vec![Span::styled(format!(" {} ", b), theme.selected())];
            if i < s.breadcrumbs.len() - 1 {
                parts.push(Span::styled(" » ", theme.muted()));
            }
            parts
        })
        .collect();

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_player(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let p = &state.playback;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Player ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Info & Volume
            Constraint::Length(1), // Progress bar
        ])
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

    let mut info_spans = vec![Span::styled(
        format!(" {} ", status_icon),
        Style::default().fg(theme.highlight),
    )];
    if state.autoplay_add {
        info_spans.push(Span::styled(
            "[AP] ",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ));
    }
    info_spans.extend([
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" — "),
        Span::styled(artist, theme.muted()),
    ]);

    if let Some(msg) = &p.status_message {
        info_spans.push(Span::raw("  "));
        info_spans.push(Span::styled(
            msg,
            Style::default().fg(theme.secondary),
        ));
    }

    // Volume status
    let vol_style = if p.muted {
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::CROSSED_OUT)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let vol_text = if p.muted {
        format!(" MUTED ({}%) ", p.volume)
    } else {
        format!(" VOL: {}% ", p.volume)
    };

    let info_line = Line::from(info_spans);
    let vol_line = Line::from(vec![
        Span::styled(vol_text, vol_style),
        Span::styled(" [ / ] ", Style::default().fg(Color::DarkGray)),
    ]);

    let info_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(20)])
        .split(rows[0]);

    frame.render_widget(Paragraph::new(info_line), info_layout[0]);
    frame.render_widget(
        Paragraph::new(vol_line).alignment(Alignment::Right),
        info_layout[1],
    );

    // Progress Bar
    let gauge = LineGauge::default()
        .block(Block::default())
        .filled_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
        .filled_symbol(ratatui::symbols::line::THICK_HORIZONTAL)
        .ratio(p.percent as f64 / 100.0)
        .label(format!(
            " {} / {} ({}%) ",
            PlaybackState::format_duration(p.position),
            PlaybackState::format_duration(p.duration),
            p.percent
        ));

    frame.render_widget(gauge, rows[1]);
}

fn render_search(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Search;

    let block = focused_block(focused, " Search ".to_string(), None, &state.theme);
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
    let theme = &state.theme;
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
        .style(theme.muted())
        .highlight_style(theme.selected())
        .divider(" | ");

    frame.render_widget(tabs, area);
}

fn render_search_input(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let focused = state.mode == Mode::Insert && state.focus == Focus::Search;
    let search = state.get_active_search();
    let input = &search.input;

    let display = if focused {
        format!("{}_", input)
    } else {
        input.clone()
    };
    let cursor_style = if focused {
        Style::default().fg(theme.primary)
    } else {
        theme.muted()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            theme.border_focused()
        } else {
            theme.border_inactive()
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
    let theme = &state.theme;
    let s = state.get_active_search();

    if s.is_loading {
        frame.render_widget(Paragraph::new("  Loading…").style(theme.muted()), area);
        return;
    }

    if s.results.is_empty() {
        let hint = if s.input.is_empty() {
            "  'i': search, 'hjkl': nav, 'tab': switch"
        } else {
            "  Enter: search"
        };
        frame.render_widget(Paragraph::new(hint).style(theme.muted()), area);
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
    let gaps = 12;

    let metadata_fixed =
        index_width + type_width + duration_width + bitrate_width + badge_width + gaps;
    let remaining = total_width.saturating_sub(metadata_fixed + 5);

    let title_max = (remaining as f64 * 0.5) as usize;
    let artist_max = remaining.saturating_sub(title_max);

    // Headers
    let header_style = theme.header();
    let header_line = Line::from(vec![
        Span::raw("   "),
        Span::styled(
            format!("{:<width$}", "#", width = index_width),
            header_style,
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:<width$}", "TITLE", width = title_max),
            header_style,
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<width$}", "ARTIST / LABEL", width = artist_max),
            header_style,
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<width$}", "TYPE", width = type_width),
            header_style,
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:>width$}", "TIME", width = duration_width),
            header_style,
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:>width$}", "BITRATE", width = bitrate_width),
            header_style,
        ),
        Span::raw(" "),
        Span::styled("PRE", header_style),
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
                Span::styled(" [P]", Style::default().fg(theme.highlight))
            } else if s.resolving.contains(&r.id) {
                Span::styled(" [~]", Style::default().fg(theme.secondary))
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
                    let s = Style::default().fg(theme.highlight);
                    let sb = s.add_modifier(Modifier::BOLD);
                    (sb, s, s, s, s, s)
                } else {
                    (
                        if selected {
                            theme.selected()
                        } else {
                            Style::default()
                        },
                        theme.muted(),
                        Style::default().fg(theme.primary),
                        Style::default().fg(theme.secondary),
                        theme.muted(),
                        theme.muted(),
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
                Style::default().fg(theme.highlight)
            } else {
                Style::default().fg(theme.primary)
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
                theme.selected_bg()
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
    let theme = &state.theme;
    let focused = state.focus == Focus::NowPlaying;
    let np = match &state.now_playing {
        Some(np) => np,
        None => {
            let block = focused_block(focused, " Now Playing ".to_string(), None, theme);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(Paragraph::new("  No tracks").style(theme.muted()), inner);
            return;
        }
    };

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
            Span::styled(format!(" [{}/{}] ", pos, total), theme.muted()),
            Span::styled(" Shift+H/L: navigate ", theme.muted()),
        ]))
    } else {
        None
    };

    let block = focused_block(focused, panel_title, bottom_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let playing_track_id = state
        .playback
        .track
        .as_ref()
        .filter(|_| state.playback.status != PlaybackStatus::Stopped)
        .map(|t| t.id.as_str());

    let items: Vec<ListItem> = np
        .tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_playing = playing_track_id == Some(track.id.as_str());
            let is_selected = focused && i == np.cursor;
            let prefix = if is_playing {
                "♫ "
            } else if is_selected {
                "▶ "
            } else {
                "  "
            };
            let duration_str = track.duration.map(format_duration).unwrap_or_default();
            let text = format!("{}{}.  {}  {}", prefix, i + 1, track.title, duration_str);

            let style = if is_playing {
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                theme.selected()
            } else {
                theme.muted()
            };

            let bg = if is_selected {
                theme.selected_bg()
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
    let theme = &state.theme;
    let focused = state.focus == Focus::Logs;
    let block = focused_block(focused, " Logs ".to_string(), None, theme);
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

fn render_help_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;

    let width = area.width * 60 / 100;
    let height = area.height * 70 / 100;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_rect = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.border_focused())
        .title(Span::styled(" Help — Keybindings ", theme.header()));

    let inner = block.inner(popup_rect);
    frame.render_widget(block, popup_rect);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let left_text = vec![
        Line::from(Span::styled(" Navigation", theme.header())),
        Line::from(""),
        Line::from("   j / ↓         Move cursor down"),
        Line::from("   k / ↑         Move cursor up"),
        Line::from("   gg            Jump to first item"),
        Line::from("   G             Jump to last item"),
        Line::from("   Tab           Next provider"),
        Line::from("   Shift+Tab     Previous provider"),
        Line::from("   1 / 2         Switch to provider"),
        Line::from("   Ctrl+h / ←    Focus search panel"),
        Line::from("   Ctrl+l / →    Focus right panel"),
        Line::from("   Backspace     Go back (history)"),
        Line::from(""),
        Line::from(Span::styled(" Playback", theme.header())),
        Line::from(""),
        Line::from("   Enter         Play / view album"),
        Line::from("   Space         Play / Pause"),
        Line::from("   Shift+S       Stop playback"),
        Line::from("   [ / ]         Volume Down / Up"),
        Line::from("   m             Toggle Mute"),
        Line::from("   , / .         Seek Backward / Forward (5s)"),
        Line::from(""),
        Line::from(Span::styled(" Modes", theme.header())),
        Line::from(""),
        Line::from("   i             Insert mode (search)"),
        Line::from("   :             Command mode"),
        Line::from("   Esc           Normal mode"),
        Line::from("   q             Close log panel"),
    ];

    let right_text = vec![
        Line::from(Span::styled(" Queue / Now Playing", theme.header())),
        Line::from(""),
        Line::from("   a             Add track to queue"),
        Line::from("   r             Replace queue w/ track"),
        Line::from("   A             Add all to queue"),
        Line::from("   R             Replace queue w/ all"),
        Line::from("   e             Toggle Now Playing"),
        Line::from("   p             Toggle auto-play"),
        Line::from("   Shift+H/L     Queue history nav"),
        Line::from(""),
        Line::from(Span::styled(" Commands", theme.header())),
        Line::from(""),
        Line::from("   :q / :quit    Quit / close panel"),
        Line::from("   :l / :log     Toggle log panel"),
        Line::from("   :h / :help    Toggle this help"),
        Line::from("   :t / :theme   Theme selector"),
        Line::from("   :dm / :mode   Toggle dark/light"),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(" Press q or Esc to close", theme.muted())),
    ];

    let left = Paragraph::new(left_text).scroll((state.help_scroll as u16, 0));
    let right = Paragraph::new(right_text).scroll((state.help_scroll as u16, 0));

    frame.render_widget(left, columns[0]);
    frame.render_widget(right, columns[1]);
}

fn render_theme_selector(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let names = Theme::preset_names();

    let list_height = names.len() as u16 + 2; // +2 for borders
    let width = 28;
    let height = list_height.min(area.height * 70 / 100);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_rect = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.border_focused())
        .style(theme.base())
        .title(Span::styled(" Theme ", theme.header()));

    let inner = block.inner(popup_rect);
    frame.render_widget(block, popup_rect);

    let items: Vec<ListItem> = names
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            let is_selected = i == state.theme_selector_cursor;
            let prefix = if is_selected { " ▶ " } else { "   " };
            let style = if is_selected {
                theme.selected()
            } else {
                Style::default().fg(theme.default)
            };
            let bg = if is_selected {
                theme.selected_bg()
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, name),
                style,
            )))
            .style(bg)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.theme_selector_cursor));
    frame.render_stateful_widget(List::new(items), inner, &mut list_state);
}

fn focused_block<'a>(
    focused: bool,
    title: String,
    bottom_title: Option<Line<'a>>,
    theme: &Theme,
) -> Block<'a> {
    let style = if focused {
        theme.border_focused()
    } else {
        theme.border_inactive()
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
