use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::theme;

use super::CoachState;

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Render the coach panel (header + wrapped message lines)
pub fn render_coach_panel(
    frame: &mut Frame,
    area: Rect,
    coach: &CoachState,
    term_width: u16,
    tick: u64,
) {
    if !coach.panel_visible || area.height < 2 {
        return;
    }

    let mut lines = Vec::new();

    // Header line with progress dots + thinking indicator
    let mut header = vec![Span::raw(" ")];

    let header_label = if coach.is_hint { "Hint" } else { "Coach" };
    header.push(Span::styled(header_label, theme::accent_bold()));

    if coach.thinking {
        let spinner = SPINNER[(tick / 2) as usize % SPINNER.len()];
        let elapsed = coach
            .thinking_started
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);
        header.push(Span::styled(
            format!("  {} thinking ({}s)", spinner, elapsed),
            theme::dim_style(),
        ));
    }

    // Right-aligned hint nudge when panel is empty (initial state)
    if coach.panel_lines.is_empty() && !coach.thinking {
        let left_len: usize = header.iter().map(|s| s.content.len()).sum();
        let hint_text = "/hint for help";
        let padding = (term_width as usize).saturating_sub(left_len + hint_text.len() + 1);
        header.push(Span::raw(" ".repeat(padding)));
        header.push(Span::styled(hint_text, theme::dim_style()));
    }

    lines.push(Line::from(header));

    // Message lines (word-wrap to terminal width - 4 for indent)
    let max_width = (term_width as usize).saturating_sub(4);
    for panel_line in &coach.panel_lines {
        let text = &panel_line.text;
        if text.len() <= max_width {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(text.clone(), theme::muted_style()),
            ]));
        } else {
            // Simple word-wrap
            let mut remaining = text.as_str();
            while !remaining.is_empty() {
                if remaining.len() <= max_width {
                    lines.push(Line::from(vec![
                        Span::raw("   "),
                        Span::styled(remaining.to_string(), theme::muted_style()),
                    ]));
                    break;
                }
                // Find last space before max_width
                let break_at = remaining[..max_width].rfind(' ').unwrap_or(max_width);
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(remaining[..break_at].to_string(), theme::muted_style()),
                ]));
                remaining = remaining[break_at..].trim_start();
            }
        }
    }

    // Cap to available height
    lines.truncate(area.height as usize);

    frame.render_widget(Paragraph::new(lines), area);
}
