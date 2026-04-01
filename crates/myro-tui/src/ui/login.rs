use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::{App, AppState, LoginPhase},
    theme,
};

use super::shared::{separator, ARROW_RIGHT, DIAMOND, LOGO_FRAMES, SPINNER};

pub(crate) fn render_login(frame: &mut Frame, app: &App, area: Rect) {
    let phase = match &app.state {
        AppState::Login { phase, .. } => phase,
        _ => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(1), // separator
        Constraint::Length(2), // title
        Constraint::Length(6), // menu / input area
        Constraint::Length(1), // error
        Constraint::Min(0),    // spacer
        Constraint::Length(2), // footer
    ])
    .split(area);

    // Header
    let logo_frame = LOGO_FRAMES[(app.tick / 5) as usize % LOGO_FRAMES.len()];
    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(logo_frame, theme::accent_style()),
        Span::styled(" myro", theme::accent_bold()),
        Span::styled("  competitive programming trainer  ", theme::dim_style()),
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

    match phase {
        LoginPhase::ChooseMethod => render_choose_method(frame, app, chunks[2], chunks[3], chunks[6]),
        LoginPhase::EmailInput {
            email,
            password,
            is_signup,
            field_focus,
            error,
        } => render_email_input(
            frame,
            app,
            chunks[2],
            chunks[3],
            chunks[4],
            chunks[6],
            email,
            password,
            *is_signup,
            *field_focus,
            error.as_deref(),
        ),
        LoginPhase::OAuthWaiting => render_oauth_waiting(frame, app, chunks[2], chunks[3], chunks[6]),
        LoginPhase::OAuthSuccess => render_oauth_success(frame, chunks[2], chunks[3]),
    }
}

fn render_choose_method(
    frame: &mut Frame,
    app: &App,
    title_area: Rect,
    menu_area: Rect,
    footer_area: Rect,
) {
    let title = Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::accent_dim_style()),
        Span::styled(" sign in to sync progress across devices", theme::muted_style()),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::raw(""), title]), title_area);

    let selected = match &app.state {
        AppState::Login {
            phase: LoginPhase::ChooseMethod,
            selected,
            ..
        } => *selected,
        _ => 0,
    };

    let items = ["sign in with github", "sign in with email", "create account", "cancel"];
    let mut lines = vec![Line::raw("")];
    for (i, item) in items.iter().enumerate() {
        let marker = if i == selected { ARROW_RIGHT } else { " " };
        let style = if i == selected {
            theme::accent_bold()
        } else {
            theme::muted_style()
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(marker, theme::accent_style()),
            Span::raw(" "),
            Span::styled(*item, style),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), menu_area);

    let footer = Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", theme::accent_dim_style()),
        Span::styled(" navigate  ", theme::dim_style()),
        Span::styled("enter", theme::accent_dim_style()),
        Span::styled(" select  ", theme::dim_style()),
        Span::styled("esc", theme::accent_dim_style()),
        Span::styled(" back  ", theme::dim_style()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(separator(footer_area.width), theme::dim_style()),
            footer,
        ]),
        footer_area,
    );
}

fn render_email_input(
    frame: &mut Frame,
    app: &App,
    title_area: Rect,
    input_area: Rect,
    error_area: Rect,
    footer_area: Rect,
    email: &str,
    password: &str,
    is_signup: bool,
    field_focus: u8,
    error: Option<&str>,
) {
    let label = if is_signup { "create account" } else { "sign in with email" };
    let title = Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::accent_dim_style()),
        Span::styled(format!(" {}", label), theme::muted_style()),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::raw(""), title]), title_area);

    let cursor = if (app.tick / 5).is_multiple_of(2) { "\u{2588}" } else { " " };

    let email_marker = if field_focus == 0 { ARROW_RIGHT } else { " " };
    let email_style = if field_focus == 0 { theme::bold_style() } else { theme::muted_style() };
    let email_cursor = if field_focus == 0 { cursor } else { "" };

    let pass_marker = if field_focus == 1 { ARROW_RIGHT } else { " " };
    let pass_style = if field_focus == 1 { theme::bold_style() } else { theme::muted_style() };
    let pass_cursor = if field_focus == 1 { cursor } else { "" };
    let masked: String = "*".repeat(password.len());

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(email_marker, theme::accent_style()),
            Span::styled(" email: ", theme::muted_style()),
            Span::styled(email, email_style),
            Span::styled(email_cursor, theme::accent_style()),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(pass_marker, theme::accent_style()),
            Span::styled(" password: ", theme::muted_style()),
            Span::styled(&masked, pass_style),
            Span::styled(pass_cursor, theme::accent_style()),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), input_area);

    if let Some(err) = error {
        let err_line = Line::from(vec![
            Span::raw("    "),
            Span::styled(err, theme::fail_style()),
        ]);
        frame.render_widget(Paragraph::new(err_line), error_area);
    }

    let footer = Line::from(vec![
        Span::raw("  "),
        Span::styled("tab", theme::accent_dim_style()),
        Span::styled(" next field  ", theme::dim_style()),
        Span::styled("enter", theme::accent_dim_style()),
        Span::styled(" submit  ", theme::dim_style()),
        Span::styled("esc", theme::accent_dim_style()),
        Span::styled(" back  ", theme::dim_style()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(separator(footer_area.width), theme::dim_style()),
            footer,
        ]),
        footer_area,
    );
}

fn render_oauth_waiting(
    frame: &mut Frame,
    app: &App,
    title_area: Rect,
    content_area: Rect,
    footer_area: Rect,
) {
    let title = Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::accent_dim_style()),
        Span::styled(" github authentication", theme::muted_style()),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::raw(""), title]), title_area);

    let spinner = SPINNER[(app.tick / 2) as usize % SPINNER.len()];
    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(spinner, theme::accent_style()),
            Span::styled(" waiting for github authentication...", theme::muted_style()),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("a browser window should have opened.", theme::dim_style()),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("complete the login there, then return here.", theme::dim_style()),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), content_area);

    let footer = Line::from(vec![
        Span::raw("  "),
        Span::styled("esc", theme::accent_dim_style()),
        Span::styled(" cancel  ", theme::dim_style()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(separator(footer_area.width), theme::dim_style()),
            footer,
        ]),
        footer_area,
    );
}

fn render_oauth_success(frame: &mut Frame, title_area: Rect, content_area: Rect) {
    let title = Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::accent_dim_style()),
        Span::styled(" github authentication", theme::muted_style()),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::raw(""), title]), title_area);

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("✓", theme::success_style()),
            Span::styled(" authenticated!", theme::success_style()),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), content_area);
}
