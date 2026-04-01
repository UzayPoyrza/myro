use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::{App, AppState, OrderSortBy, PastFilter, PastOrder},
    theme,
};

use super::{
    overlays::render_loading_overlay,
    shared::{separator, ARROW_RIGHT, CHECK, CROSS, DIAMOND, DOT},
};

pub(crate) fn render_past(frame: &mut Frame, app: &App, area: Rect) {
    let (scroll, command_input) = match &app.state {
        AppState::Past { scroll, command_input, .. } => (*scroll, command_input.as_deref()),
        _ => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(1), // separator
        Constraint::Min(1),    // list
        Constraint::Length(2), // footer
    ])
    .split(area);

    // Header
    let count = app.past_entries.len();
    let header = Line::from(vec![
        Span::styled(format!(" {} ", DIAMOND), theme::purple_style()),
        Span::styled("my past", theme::bold_style()),
        Span::styled(format!("  {} problems", count), theme::muted_style()),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::raw(""), header]), chunks[0]);

    // Separator
    frame.render_widget(
        Paragraph::new(separator(area.width)).style(theme::dim_style()),
        chunks[1],
    );

    // Filtered + sorted entries
    let entries = app.filtered_past_entries();
    let visible_height = chunks[2].height as usize;

    if entries.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  No problems yet. Go solve some!",
                theme::muted_style(),
            ))),
            chunks[2],
        );
    } else {
        let now = chrono::Utc::now().timestamp();
        let mut lines: Vec<Line> = Vec::new();
        for (i, entry) in entries.iter().enumerate().skip(scroll).take(visible_height) {
            let is_selected = i == scroll;

            let id_str = format!("{}{}", entry.contest_id, entry.index);
            let rating_str = entry.rating
                .map(|r| format!("{:>5}", r))
                .unwrap_or_else(|| "    -".to_string());
            let tags_str = if entry.tags.is_empty() {
                String::new()
            } else {
                entry.tags.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
            };
            let mode_str = if entry.mode == "chill" { "chill" } else { "intense" };
            let (outcome_icon, outcome_label, outcome_style) = if entry.ever_accepted {
                (CHECK, "solved", theme::success_style())
            } else {
                (CROSS, "unsolved", theme::fail_style())
            };
            let time_ago = match app.past_order.sort_by {
                OrderSortBy::FirstSeen => format_time_ago(now, entry.first_seen_at),
                OrderSortBy::LastSeen => format_time_ago(now, entry.last_seen_at),
                OrderSortBy::FirstSubmission => entry
                    .first_submitted_at
                    .map(|t| format_time_ago(now, t))
                    .unwrap_or_else(|| "-".into()),
                OrderSortBy::LastSubmission => entry
                    .last_submitted_at
                    .map(|t| format_time_ago(now, t))
                    .unwrap_or_else(|| "-".into()),
                OrderSortBy::Rating => String::new(),
            };

            let title = if entry.title.len() > 20 {
                format!("{:.20}..", entry.title)
            } else {
                format!("{:<22}", entry.title)
            };

            if is_selected {
                lines.push(Line::from(vec![
                    Span::styled(format!(" {} ", ARROW_RIGHT), theme::accent_style()),
                    Span::styled(format!("{:<7}", id_str), theme::bold_style()),
                    Span::styled(title, theme::bold_style()),
                    Span::styled(format!(" {}", rating_str), theme::purple_style()),
                    Span::styled(format!("  {:<14}", tags_str), theme::dim_style()),
                    Span::styled(format!("{:<8}", mode_str), theme::muted_style()),
                    Span::styled(outcome_icon, outcome_style),
                    Span::styled(format!(" {:<10}", outcome_label), outcome_style),
                    Span::styled(time_ago, theme::dim_style()),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(format!("{:<7}", id_str), theme::muted_style()),
                    Span::raw(title),
                    Span::styled(format!(" {}", rating_str), theme::muted_style()),
                    Span::styled(format!("  {:<14}", tags_str), theme::dim_style()),
                    Span::styled(format!("{:<8}", mode_str), theme::dim_style()),
                    Span::styled(outcome_icon, outcome_style),
                    Span::styled(format!(" {:<10}", outcome_label), outcome_style),
                    Span::styled(time_ago, theme::dim_style()),
                ]));
            }
        }
        frame.render_widget(Paragraph::new(lines), chunks[2]);
    }

    // Footer
    let (filter_open, order_open) = match &app.state {
        AppState::Past { filter_open, order_open, .. } => (*filter_open, *order_open),
        _ => (false, false),
    };
    let footer = if filter_open {
        Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", theme::accent_dim_style()),
            Span::styled(" navigate  ", theme::dim_style()),
            Span::styled("space", theme::accent_dim_style()),
            Span::styled(" toggle  ", theme::dim_style()),
            Span::styled("esc", theme::accent_dim_style()),
            Span::styled(" close", theme::dim_style()),
        ])
    } else if order_open {
        Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", theme::accent_dim_style()),
            Span::styled(" navigate  ", theme::dim_style()),
            Span::styled("space", theme::accent_dim_style()),
            Span::styled(" select  ", theme::dim_style()),
            Span::styled("esc", theme::accent_dim_style()),
            Span::styled(" close", theme::dim_style()),
        ])
    } else if let Some(cmd) = command_input {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("/{}", cmd), theme::accent_style()),
        ])
    } else if let Some(msg) = &app.status_message {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(msg.as_str(), theme::warn_style()),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", theme::accent_dim_style()),
            Span::styled(" scroll  ", theme::dim_style()),
            Span::styled("enter", theme::accent_dim_style()),
            Span::styled(" open  ", theme::dim_style()),
            Span::styled("/filter", theme::accent_dim_style()),
            Span::styled("  ", theme::dim_style()),
            Span::styled("/order", theme::accent_dim_style()),
            Span::styled("  ", theme::dim_style()),
            Span::styled("esc", theme::accent_dim_style()),
            Span::styled(" back", theme::dim_style()),
        ])
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(separator(area.width), theme::dim_style()),
            footer,
        ]),
        chunks[3],
    );

    // Loading overlay for recommender (when fetching from Past)
    if let Some(status) = &app.recommender.status {
        render_loading_overlay(frame, area, status, app.tick);
    }

    // Filter popup
    if matches!(&app.state, AppState::Past { filter_open: true, .. }) {
        let cursor = match &app.state {
            AppState::Past { filter_cursor, .. } => *filter_cursor,
            _ => 0,
        };
        render_filter_popup(frame, area, &app.past_filter, cursor);
    }

    // Order popup
    if matches!(&app.state, AppState::Past { order_open: true, .. }) {
        let cursor = match &app.state {
            AppState::Past { order_cursor, .. } => *order_cursor,
            _ => 0,
        };
        render_order_popup(frame, area, &app.past_order, cursor);
    }
}

pub(crate) fn render_filter_popup(frame: &mut Frame, area: Rect, filter: &PastFilter, cursor: usize) {
    use ratatui::widgets::{Block, Borders, Clear};

    let overlay_w = 30u16.min(area.width.saturating_sub(4));
    // 6 items + section headers (3) + footer + border = ~14
    let overlay_h = 14u16.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(overlay_w)) / 2;
    let y = (area.height.saturating_sub(overlay_h)) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();

    for i in 0..PastFilter::COUNT {
        if let Some(header) = PastFilter::section_header(i) {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                format!(" {}", header),
                theme::accent_dim_style(),
            )));
        }
        let checked = if filter.get(i) { "[x]" } else { "[ ]" };
        let label = PastFilter::label(i);
        if i == cursor {
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", ARROW_RIGHT), theme::accent_style()),
                Span::styled(checked, theme::accent_style()),
                Span::styled(format!(" {}", label), theme::bold_style()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(checked, theme::muted_style()),
                Span::styled(format!(" {}", label), theme::muted_style()),
            ]));
        }
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(" j/k", theme::accent_dim_style()),
        Span::styled(" navigate  ", theme::dim_style()),
        Span::styled("space", theme::accent_dim_style()),
        Span::styled(" toggle  ", theme::dim_style()),
        Span::styled("esc", theme::accent_dim_style()),
        Span::styled(" close", theme::dim_style()),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::dim_style())
        .title(Span::styled(" filter ", theme::accent_bold()));

    frame.render_widget(Paragraph::new(lines).block(block), overlay_area);
}

pub(crate) fn render_order_popup(frame: &mut Frame, area: Rect, order: &PastOrder, cursor: usize) {
    use ratatui::widgets::{Block, Borders, Clear};

    let overlay_w = 30u16.min(area.width.saturating_sub(4));
    let overlay_h = 14u16.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(overlay_w)) / 2;
    let y = (area.height.saturating_sub(overlay_h)) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();

    // "By" section header
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(" by", theme::accent_dim_style())));

    for (i, sort_key) in OrderSortBy::ALL.iter().enumerate() {
        let selected = *sort_key == order.sort_by;
        let marker = if selected { DOT } else { " " };
        if i == cursor {
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", ARROW_RIGHT), theme::accent_style()),
                Span::styled(marker, theme::accent_style()),
                Span::styled(format!(" {}", sort_key.label()), theme::bold_style()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(marker, if selected { theme::accent_style() } else { theme::dim_style() }),
                Span::styled(format!(" {}", sort_key.label()), theme::muted_style()),
            ]));
        }
    }

    // "Direction" section header
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(" direction", theme::accent_dim_style())));

    let (desc_label, asc_label) = order.direction_labels();
    for (j, (label, is_asc)) in [(desc_label, false), (asc_label, true)].iter().enumerate() {
        let idx = 5 + j;
        let selected = order.ascending == *is_asc;
        let marker = if selected { DOT } else { " " };
        if idx == cursor {
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", ARROW_RIGHT), theme::accent_style()),
                Span::styled(marker, theme::accent_style()),
                Span::styled(format!(" {}", label), theme::bold_style()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(marker, if selected { theme::accent_style() } else { theme::dim_style() }),
                Span::styled(format!(" {}", label), theme::muted_style()),
            ]));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(" j/k", theme::accent_dim_style()),
        Span::styled(" navigate  ", theme::dim_style()),
        Span::styled("space", theme::accent_dim_style()),
        Span::styled(" select  ", theme::dim_style()),
        Span::styled("esc", theme::accent_dim_style()),
        Span::styled(" close", theme::dim_style()),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::dim_style())
        .title(Span::styled(" order ", theme::accent_bold()));

    frame.render_widget(Paragraph::new(lines).block(block), overlay_area);
}

pub(crate) fn format_time_ago(now: i64, timestamp: i64) -> String {
    let diff = (now - timestamp).max(0);
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}
