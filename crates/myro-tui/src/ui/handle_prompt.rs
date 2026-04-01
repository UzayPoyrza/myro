use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::{App, AppState, OnboardingPhase},
    theme,
};

use super::shared::{separator, ARROW_RIGHT, CHECK, DIAMOND, LOGO_FRAMES, SPINNER};

pub(crate) fn render_handle_prompt(frame: &mut Frame, app: &App, area: Rect) {
    let (phase, handle_input, error, validating) = match &app.state {
        AppState::HandlePrompt {
            phase,
            handle_input,
            error,
            validating,
            ..
        } => (phase, handle_input.as_str(), error.as_deref(), *validating),
        _ => return,
    };

    let is_cookie_import = matches!(phase, OnboardingPhase::CookieImport);

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(2), // separator + confirmed handle (or blank)
        Constraint::Length(2), // step prompt
        Constraint::Length(3), // input area
        Constraint::Length(1), // error/status
        Constraint::Min(0),   // spacer
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

    // Separator + confirmed handle line (cookie import phase only)
    let sep = separator(area.width);
    if is_cookie_import {
        let confirmed = Line::from(vec![
            Span::raw("  "),
            Span::styled(CHECK, theme::success_style()),
            Span::styled(format!(" {}", handle_input), theme::success_style()),
        ]);
        frame.render_widget(
            Paragraph::new(vec![Line::styled(&*sep, theme::dim_style()), confirmed]),
            chunks[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(vec![Line::styled(&*sep, theme::dim_style()), Line::raw("")]),
            chunks[1],
        );
    }

    // Step prompt
    let (step, prompt_text) = if is_cookie_import {
        ("step 2/2", "import cookies from firefox")
    } else {
        ("step 1/2", "enter your codeforces handle")
    };
    let prompt = Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::accent_dim_style()),
        Span::styled(format!(" {}: {}", step, prompt_text), theme::muted_style()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![Line::raw(""), prompt]),
        chunks[2],
    );

    // Input/instruction area
    if is_cookie_import {
        let instruction = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "log into codeforces.com in firefox, then press enter to import",
                theme::muted_style(),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(vec![Line::raw(""), instruction]),
            chunks[3],
        );
    } else if validating {
        let spinner = SPINNER[(app.tick / 2) as usize % SPINNER.len()];
        let input_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(spinner, theme::accent_style()),
            Span::raw(" "),
            Span::styled(handle_input, theme::bold_style()),
            Span::styled("  validating...", theme::muted_style()),
        ]);
        frame.render_widget(
            Paragraph::new(vec![Line::raw(""), input_line]),
            chunks[3],
        );
    } else {
        let cursor = if (app.tick / 5).is_multiple_of(2) { "\u{2588}" } else { " " };
        let input_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(ARROW_RIGHT, theme::accent_style()),
            Span::raw(" "),
            Span::styled(handle_input, theme::bold_style()),
            Span::styled(cursor, theme::accent_style()),
        ]);
        frame.render_widget(
            Paragraph::new(vec![Line::raw(""), input_line]),
            chunks[3],
        );
    }

    // Error message
    if let Some(err) = error {
        let err_line = Line::from(vec![
            Span::raw("    "),
            Span::styled(err, theme::fail_style()),
        ]);
        frame.render_widget(Paragraph::new(err_line), chunks[4]);
    }

    // Footer
    let footer = if is_cookie_import {
        Line::from(vec![
            Span::raw("  "),
            Span::styled("esc", theme::accent_dim_style()),
            Span::styled(" back  ", theme::dim_style()),
            Span::styled("enter", theme::accent_dim_style()),
            Span::styled(" import  ", theme::dim_style()),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled("enter", theme::accent_dim_style()),
            Span::styled(" confirm  ", theme::dim_style()),
        ])
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(separator(area.width), theme::dim_style()),
            footer,
        ]),
        chunks[6],
    );
}
