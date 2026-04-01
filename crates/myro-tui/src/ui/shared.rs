use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::theme;

pub(crate) const ARROW_RIGHT: &str = "▸";
pub(crate) const CHECK: &str = "✓";
pub(crate) const CROSS: &str = "✗";
pub(crate) const DOT: &str = "●";
pub(crate) const DIAMOND: &str = "◆";
pub(crate) const TIMER: &str = "⏱";
pub(crate) const SEPARATOR_CHAR: char = '─';

pub(crate) const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
pub(crate) const LOGO_FRAMES: &[&str] = &["◈", "◇", "◈", "◆"];
pub(crate) const SPARKLE: &[&str] = &["✦", "✧", "⭑", "✧", "✦", " "];

pub(crate) fn separator(width: u16) -> String {
    SEPARATOR_CHAR.to_string().repeat(width as usize)
}

pub(crate) fn render_panel_separator(width: u16, statement_focused: bool) -> Paragraph<'static> {
    if statement_focused {
        let label = " ↑↓ scroll · tab: editor ";
        let pad = (width as usize).saturating_sub(label.chars().count());
        let left = pad / 2;
        let right = pad - left;
        Paragraph::new(Line::from(vec![
            Span::styled(SEPARATOR_CHAR.to_string().repeat(left), theme::accent_dim_style()),
            Span::styled(label, theme::accent_dim_style()),
            Span::styled(SEPARATOR_CHAR.to_string().repeat(right), theme::accent_dim_style()),
        ]))
    } else {
        Paragraph::new(Line::styled(separator(width), theme::dim_style()))
    }
}

pub(crate) fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
