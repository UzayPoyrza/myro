use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::{App, AppState, MENU_ITEMS},
    theme,
};

use super::{
    overlays::render_loading_overlay,
    shared::{separator, ARROW_RIGHT, CHECK, DIAMOND, LOGO_FRAMES, SPINNER},
};

pub(crate) fn render_home(frame: &mut Frame, app: &App, area: Rect) {
    let selected = match &app.state {
        AppState::Home { selected } => *selected,
        _ => 0,
    };

    let has_update = app.update_available.is_some();
    let chunks = Layout::vertical([
        Constraint::Length(3),                               // header + blank
        Constraint::Length(2),                               // separator + prompt
        Constraint::Min(0),                                  // menu
        Constraint::Length(if has_update { 1 } else { 0 }),  // update notification
        Constraint::Length(2),                               // footer
    ])
    .split(area);

    // Header with animated logo
    let logo_frame = LOGO_FRAMES[(app.tick / 5) as usize % LOGO_FRAMES.len()];
    let spinner = SPINNER[(app.tick / 2) as usize % SPINNER.len()];
    let greeting = app
        .user_state
        .name
        .as_ref()
        .map(|n| format!("  hey, {}", n))
        .unwrap_or_default();
    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(logo_frame, theme::accent_style()),
        Span::styled(" myro", theme::accent_bold()),
        Span::styled(greeting, theme::muted_style()),
        Span::styled("  ", theme::dim_style()),
        Span::styled(spinner, theme::accent_dim_style()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![Line::raw(""), header, Line::raw("")]),
        chunks[0],
    );

    // Separator + prompt
    let sep = separator(area.width);
    let prompt = Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::accent_dim_style()),
        Span::styled(" what would you like to do?", theme::muted_style()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![Line::styled(&*sep, theme::dim_style()), prompt]),
        chunks[1],
    );

    // Menu items with symbols
    let menu_icons = ["\u{1F525}", "\u{1F3AF}", "\u{1F4DA}", "\u{1F527}"];
    let mut lines = vec![Line::raw("")];
    for (i, item) in MENU_ITEMS.iter().enumerate() {
        let icon = menu_icons.get(i).unwrap_or(&" ");
        if i == selected {
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
                Span::raw(format!("{} ", icon)),
                Span::styled(*item, theme::bold_style()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::raw(format!("{} ", icon)),
                Span::styled(*item, theme::muted_style()),
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lines), chunks[2]);

    // Update notification (above footer)
    let update_line = app.update_available.as_ref().map(|v| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("update available: v{} \u{2014} run ", v),
                theme::dim_style(),
            ),
            Span::styled("myro update", theme::accent_dim_style()),
        ])
    });

    // Footer hints (or status message)
    let footer = if app.feed_me_menu.is_some() {
        Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", theme::accent_dim_style()),
            Span::styled(" navigate  ", theme::dim_style()),
            Span::styled("enter", theme::accent_dim_style()),
            Span::styled(" select  ", theme::dim_style()),
            Span::styled("esc", theme::accent_dim_style()),
            Span::styled(" back", theme::dim_style()),
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
            Span::styled(" navigate  ", theme::dim_style()),
            Span::styled("enter", theme::accent_dim_style()),
            Span::styled(" select  ", theme::dim_style()),
            Span::styled("q", theme::accent_dim_style()),
            Span::styled(" quit", theme::dim_style()),
        ])
    };
    // Render update notification
    if let Some(line) = update_line {
        frame.render_widget(Paragraph::new(line), chunks[3]);
    }

    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(separator(area.width), theme::dim_style()),
            footer,
        ]),
        chunks[4],
    );

    // Loading overlay for recommender
    if let Some(status) = &app.recommender.status {
        render_loading_overlay(frame, area, status, app.tick);
    }

    // Feed me sub-menu popup
    if let Some(sel) = app.feed_me_menu {
        render_feed_me_menu(frame, area, sel);
    }
}


pub(crate) fn render_problem_select(frame: &mut Frame, app: &App, area: Rect) {
    let (problems, selected, scroll_offset) = match &app.state {
        AppState::ProblemSelect {
            problems,
            selected,
            scroll_offset,
        } => (problems, *selected, *scroll_offset),
        _ => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(1), // separator
        Constraint::Min(0),   // problem list
        Constraint::Length(2), // footer
    ])
    .split(area);

    // Header
    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::accent_style()),
        Span::styled(" Select a problem", theme::accent_bold()),
        Span::styled(
            format!("  ({} available)", problems.len()),
            theme::dim_style(),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(vec![Line::raw(""), header, Line::raw("")]),
        chunks[0],
    );

    // Separator
    frame.render_widget(
        Paragraph::new(Line::styled(separator(area.width), theme::dim_style())),
        chunks[1],
    );

    // Problem list
    let visible_height = chunks[2].height as usize;
    let mut lines = vec![];
    for (i, p) in problems.iter().enumerate().skip(scroll_offset).take(visible_height) {
        let rating_str = format!("{:>4}", p.difficulty);
        let tags_str = if p.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", p.tags.join(", "))
        };

        let solved = app.user_state.is_solved(&p.id());
        let solved_marker = if solved {
            Span::styled(format!(" {}", CHECK), theme::success_style())
        } else {
            Span::raw("")
        };

        if i == selected {
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
                Span::styled(rating_str, theme::purple_style()),
                Span::raw("  "),
                Span::styled(p.title.clone(), theme::bold_style()),
                Span::styled(tags_str, theme::dim_style()),
                solved_marker,
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(rating_str, theme::muted_style()),
                Span::raw("  "),
                Span::raw(p.title.clone()),
                Span::styled(tags_str, theme::dim_style()),
                solved_marker,
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lines), chunks[2]);

    // Footer
    let footer = Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", theme::accent_dim_style()),
        Span::styled(" navigate  ", theme::dim_style()),
        Span::styled("enter", theme::accent_dim_style()),
        Span::styled(" select  ", theme::dim_style()),
        Span::styled("esc", theme::accent_dim_style()),
        Span::styled(" back", theme::dim_style()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(separator(area.width), theme::dim_style()),
            footer,
        ]),
        chunks[3],
    );
}


pub(crate) fn render_feed_me_menu(frame: &mut Frame, area: Rect, selected: usize) {
    use ratatui::widgets::{Block, Borders, Clear};

    let items = ["chill", "intense"];
    let descs = [
        "coach on, no rating effects",
        "no coach, 30-min timer, ratings",
    ];

    let overlay_w = 50u16.min(area.width.saturating_sub(4));
    let overlay_h = 6u16;
    let x = (area.width.saturating_sub(overlay_w)) / 2;
    let y = (area.height.saturating_sub(overlay_h)) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::raw(""));
    for (i, (item, desc)) in items.iter().zip(descs.iter()).enumerate() {
        if i == selected {
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
                Span::styled(*item, theme::bold_style()),
                Span::styled(format!("  {}", desc), theme::dim_style()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(*item, theme::muted_style()),
                Span::styled(format!("  {}", desc), theme::dim_style()),
            ]));
        }
    }
    lines.push(Line::raw(""));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::dim_style())
        .title(Span::styled(" feed me ", theme::accent_bold()));

    frame.render_widget(Paragraph::new(lines).block(block), overlay_area);
}


