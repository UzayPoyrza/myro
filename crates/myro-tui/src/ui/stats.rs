use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::{App, AppState},
    theme,
};

use super::shared::{separator, DIAMOND};

pub(crate) fn render_stats(frame: &mut Frame, app: &App, area: Rect) {
    let scroll = match &app.state {
        AppState::Stats { scroll } => *scroll,
        _ => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(1), // separator
        Constraint::Min(1),    // tag list
        Constraint::Length(2), // footer
    ])
    .split(area);

    // Header
    let overall = app
        .recommender
        .skill_profile
        .as_ref()
        .map(|p| p.overall_rating)
        .unwrap_or(0);
    let header = Line::from(vec![
        Span::styled(format!(" {} ", DIAMOND), theme::purple_style()),
        Span::styled("rate me", theme::bold_style()),
        Span::styled(format!("  overall: {}", overall), theme::muted_style()),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::raw(""), header]), chunks[0]);

    // Separator
    frame.render_widget(
        Paragraph::new(separator(area.width)).style(theme::dim_style()),
        chunks[1],
    );

    // Tag list
    if let Some(profile) = &app.recommender.skill_profile {
        let visible_height = chunks[2].height as usize;

        let mut lines: Vec<Line> = Vec::new();

        // Column header
        lines.push(Line::from(vec![
            Span::styled("  ", theme::dim_style()),
            Span::styled(format!("{:<28}", "tag"), theme::dim_style()),
            Span::styled(format!("{:<8}", "rating"), theme::dim_style()),
            Span::styled(format!("{:<24}", "p(solve)"), theme::dim_style()),
        ]));

        for (i, tag) in profile.tag_ratings.iter().enumerate() {
            if i < scroll {
                continue;
            }
            if lines.len() >= visible_height {
                break;
            }

            // Progress bar: 20 chars wide, based on avg P(solve)
            let bar_width = 20;
            let filled = (tag.avg_p_solve * bar_width as f64).round() as usize;
            let filled = filled.min(bar_width);
            let empty = bar_width - filled;
            let bar_filled = "\u{2588}".repeat(filled);
            let bar_empty = "\u{2591}".repeat(empty);

            let line = Line::from(vec![
                Span::styled("  ", theme::dim_style()),
                Span::styled(format!("{:<28}", tag.tag), theme::muted_style()),
                Span::styled(format!("{:<8}", tag.effective_rating), theme::accent_style()),
                Span::styled(bar_filled, theme::purple_style()),
                Span::styled(bar_empty, theme::dim_style()),
                Span::styled(
                    format!(" {:>3.0}%", tag.avg_p_solve * 100.0),
                    theme::muted_style(),
                ),
            ]);
            lines.push(line);
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "  Loading skill profile...",
                theme::muted_style(),
            )));
        }

        frame.render_widget(Paragraph::new(lines), chunks[2]);
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Loading skill profile...",
                theme::muted_style(),
            ))),
            chunks[2],
        );
    }

    // Footer
    let footer = Line::from(vec![
        Span::styled(" j", theme::accent_dim_style()),
        Span::styled("/", theme::dim_style()),
        Span::styled("k", theme::accent_dim_style()),
        Span::styled(" scroll  ", theme::dim_style()),
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

