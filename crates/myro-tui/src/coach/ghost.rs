use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Modifier, Style},
};

use myro_coach::types::GhostFormat;

use crate::theme;

use super::GhostTextState;

/// Render ghost text below the cursor by writing directly to the buffer.
/// Called AFTER the editor renders, so we overlay on top.
pub fn render_ghost_text(
    buf: &mut Buffer,
    editor_area: Rect,
    ghost: &GhostTextState,
    cursor_row: u16,
    current_tick: u64,
) {
    // 3-tick fade-in delay
    if current_tick.wrapping_sub(ghost.appeared_at) < 3 {
        return;
    }

    // Position: one line below cursor within the editor area
    let ghost_row = editor_area.y + cursor_row + 1;
    if ghost_row >= editor_area.y + editor_area.height {
        return; // No room
    }

    let style = match ghost.format {
        GhostFormat::Code => theme::ghost_code_style(),
        GhostFormat::Natural => {
            Style::default()
                .fg(theme::GHOST)
                .add_modifier(Modifier::ITALIC)
        }
    };

    // Prefix with ~ and truncate to editor width
    let prefix = "  ~ ";
    let max_text_width = (editor_area.width as usize).saturating_sub(prefix.len() + 2);
    let display_text = if ghost.text.len() > max_text_width {
        let mut end = max_text_width.saturating_sub(3);
        while end > 0 && !ghost.text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}{}...", prefix, &ghost.text[..end])
    } else {
        format!("{}{}", prefix, &ghost.text)
    };

    // Write to buffer character by character
    let x_start = editor_area.x;
    for (i, ch) in display_text.chars().enumerate() {
        let x = x_start + i as u16;
        if x >= editor_area.x + editor_area.width {
            break;
        }
        if let Some(cell) = buf.cell_mut(Position::new(x, ghost_row)) {
            cell.set_char(ch);
            cell.set_style(style);
        }
    }
}
