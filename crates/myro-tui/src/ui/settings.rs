use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::{App, AppState, SettingsItem, SETTINGS_ITEMS},
    theme,
};

use super::shared::{separator, ARROW_RIGHT, DIAMOND};

pub(crate) fn render_settings(frame: &mut Frame, app: &App, area: Rect) {
    let (selected, editing) = match &app.state {
        AppState::Settings { selected, editing } => (*selected, editing.as_deref()),
        _ => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(1), // separator
        Constraint::Min(0),    // settings list
        Constraint::Length(2), // footer
    ])
    .split(area);

    // Header
    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::accent_style()),
        Span::styled(" settings", theme::accent_bold()),
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

    // Settings items
    let mut lines = Vec::new();
    for (i, item) in SETTINGS_ITEMS.iter().enumerate() {
        match item {
            SettingsItem::Section { label } => {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("── {} ", label), theme::accent_dim_style()),
                    Span::styled(
                        "─".repeat((area.width as usize).saturating_sub(label.len() + 6)),
                        theme::dim_style(),
                    ),
                ]));
            }
            SettingsItem::Display { label, field } => {
                let display = app.read_setting_display(field);
                render_display_item(&mut lines, i == selected, label, &display);
            }
            SettingsItem::Editable { label, field } => {
                if i == selected && editing.is_some() {
                    render_editing_item(&mut lines, label, editing.unwrap_or(""), app.tick);
                } else {
                    let display = app.read_setting_display(field);
                    render_display_item(&mut lines, i == selected, label, &display);
                }
            }
            SettingsItem::EditableSensitive { label, field } => {
                if i == selected && editing.is_some() {
                    // Show actual characters while typing (not masked)
                    render_editing_item(&mut lines, label, editing.unwrap_or(""), app.tick);
                } else {
                    let display = app.read_setting_display(field);
                    render_display_item(&mut lines, i == selected, label, &display);
                }
            }
            SettingsItem::Action { label, action } => {
                // Dynamic label for sign_in based on auth state
                let display_label = if *action == "sign_in" && app.is_signed_in() {
                    "signed in ✓"
                } else {
                    label
                };

                let is_danger = matches!(*action, "reset_history" | "logout");
                if i == selected {
                    let style = if is_danger {
                        theme::fail_style()
                    } else {
                        theme::accent_bold()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
                        Span::styled(format!("{} →", display_label), style),
                    ]));
                } else {
                    let style = if is_danger {
                        theme::warn_style()
                    } else {
                        theme::muted_style()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(format!("{} →", display_label), style),
                    ]));
                }
            }
        }
    }
    frame.render_widget(Paragraph::new(lines), chunks[2]);

    // Footer
    let footer = if editing.is_some() {
        Line::from(vec![
            Span::raw("  "),
            Span::styled("enter", theme::accent_dim_style()),
            Span::styled(" save  ", theme::dim_style()),
            Span::styled("esc", theme::accent_dim_style()),
            Span::styled(" cancel", theme::dim_style()),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", theme::accent_dim_style()),
            Span::styled(" navigate  ", theme::dim_style()),
            Span::styled("enter", theme::accent_dim_style()),
            Span::styled(" select  ", theme::dim_style()),
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
}

fn render_display_item(lines: &mut Vec<Line<'static>>, selected: bool, label: &str, display: &str) {
    if selected {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
            Span::styled(format!("{}: ", label), theme::bold_style()),
            Span::styled(display.to_string(), theme::dim_style()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("{}: ", label), theme::muted_style()),
            Span::styled(display.to_string(), theme::dim_style()),
        ]));
    }
}

fn render_editing_item(lines: &mut Vec<Line<'static>>, label: &str, buf: &str, tick: u64) {
    let cursor = if (tick / 5).is_multiple_of(2) {
        "\u{2588}"
    } else {
        " "
    };
    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
        Span::styled(format!("{}: ", label), theme::bold_style()),
        Span::styled(buf.to_string(), theme::accent_style()),
        Span::styled(cursor.to_string(), theme::accent_style()),
    ]));
}
