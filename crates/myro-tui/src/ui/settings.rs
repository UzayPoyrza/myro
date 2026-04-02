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
    let (selected, editing, dropdown) = match &app.state {
        AppState::Settings { selected, editing, dropdown } => (*selected, editing.as_deref(), *dropdown),
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
                    render_editing_item(&mut lines, label, editing.unwrap_or(""), app.tick);
                } else {
                    let display = app.read_setting_display(field);
                    let needs_attention = *field == "coach.api_key"
                        && app.coach_config.api_key.as_ref().map_or(true, |k| k.is_empty());
                    render_display_item_with_dot(&mut lines, i == selected, label, &display, needs_attention);
                }
            }
            SettingsItem::Dropdown { label, field, .. } => {
                let display = app.read_setting_display(field);
                if i == selected {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
                        Span::styled(format!("{}: ", label), theme::bold_style()),
                        Span::styled(display, theme::accent_style()),
                        Span::styled("  \u{25C0}\u{25B6}", theme::dim_style()),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(format!("{}: ", label), theme::muted_style()),
                        Span::styled(display, theme::dim_style()),
                    ]));
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

    // Dropdown popup
    if let Some(dropdown_sel) = dropdown {
        if let Some(SettingsItem::Dropdown { options, .. }) = SETTINGS_ITEMS.get(selected) {
            render_dropdown_popup(frame, area, options, dropdown_sel);
        }
    }
}

fn render_dropdown_popup(frame: &mut Frame, area: Rect, options: &[&str], selected: usize) {
    use ratatui::widgets::{Block, Borders, Clear};

    let overlay_w = 40u16.min(area.width.saturating_sub(4));
    let overlay_h = (options.len() as u16 + 2).min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(overlay_w)) / 2;
    let y = (area.height.saturating_sub(overlay_h)) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();
    for (i, option) in options.iter().enumerate() {
        if i == selected {
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
                Span::styled(*option, theme::bold_style()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(*option, theme::muted_style()),
            ]));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::dim_style())
        .title(Span::styled(" llm provider ", theme::accent_bold()));

    frame.render_widget(Paragraph::new(lines).block(block), overlay_area);
}

fn render_display_item_with_dot(
    lines: &mut Vec<Line<'static>>,
    selected: bool,
    label: &str,
    display: &str,
    needs_attention: bool,
) {
    let dot = if needs_attention {
        Span::styled("\u{25CF} ", theme::fail_style()) // ● red dot
    } else {
        Span::raw("  ")
    };
    if selected {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", ARROW_RIGHT), theme::accent_style()),
            dot,
            Span::styled(format!("{}: ", label), theme::bold_style()),
            Span::styled(display.to_string(), theme::dim_style()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("  "),
            dot,
            Span::styled(format!("{}: ", label), theme::muted_style()),
            Span::styled(display.to_string(), theme::dim_style()),
        ]));
    }
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
