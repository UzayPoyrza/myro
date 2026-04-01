use edtui::{EditorState, EditorTheme, EditorView, LineNumbers, SyntaxHighlighter};
use ratatui::{layout::Rect, style::Style, Frame};

use crate::theme;

pub fn editor_theme() -> EditorTheme<'static> {
    EditorTheme::default()
        .base(Style::default().fg(theme::TEXT))
        .cursor_style(
            Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(theme::TEXT),
        )
        .selection_style(Style::default().bg(ratatui::style::Color::Rgb(60, 60, 80)))
        .line_numbers_style(Style::default().fg(theme::DIM))
        .hide_status_line()
}

pub fn python_highlighter() -> Option<SyntaxHighlighter> {
    SyntaxHighlighter::new("base16-ocean-dark", "py").ok()
}

/// Render a plain editor (no line numbers) — used for the test panel input.
pub fn render_editor_plain(frame: &mut Frame, area: Rect, state: &mut EditorState, active: bool) {
    let theme = if active {
        editor_theme()
    } else {
        editor_theme().cursor_style(Style::default())
    };
    let view = EditorView::new(state).theme(theme).tab_width(4);
    frame.render_widget(view, area);
}

pub fn render_editor(frame: &mut Frame, area: Rect, state: &mut EditorState, active: bool) {
    let highlighter = python_highlighter();
    let theme = if active {
        editor_theme()
    } else {
        editor_theme().cursor_style(Style::default())
    };
    let view = EditorView::new(state)
        .theme(theme)
        .syntax_highlighter(highlighter)
        .line_numbers(LineNumbers::Absolute)
        .tab_width(4);
    frame.render_widget(view, area);
}
