use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use myro_predict::model::skills::SkillDelta;

use crate::{
    app::ConfirmPopup,
    theme,
};

use super::shared::{centered_rect, SPARKLE, SPINNER};

pub(crate) fn render_loading_overlay(frame: &mut Frame, area: Rect, message: &str, tick: u64) {
    use ratatui::widgets::{Block, Borders, Clear};

    let overlay_w = 40u16.min(area.width.saturating_sub(4));
    let overlay_h = 3u16;
    let overlay_area = centered_rect(area, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let spinner = SPINNER[(tick / 2) as usize % SPINNER.len()];
    let line = Line::from(vec![
        Span::styled(format!(" {} ", spinner), theme::accent_style()),
        Span::styled(message, theme::muted_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::dim_style());

    frame.render_widget(Paragraph::new(line).block(block), overlay_area);
}

pub(crate) fn render_confirm_popup(frame: &mut Frame, area: Rect, popup: &ConfirmPopup) {
    use ratatui::widgets::{Block, Borders, Clear};

    let overlay_w = 50u16.min(area.width.saturating_sub(4));
    let inner_w = (overlay_w as usize).saturating_sub(4); // border + padding

    // Wrap message text
    let mut msg_lines: Vec<String> = Vec::new();
    for word in popup.message.split_whitespace() {
        if let Some(last) = msg_lines.last_mut() {
            if last.len() + 1 + word.len() <= inner_w {
                last.push(' ');
                last.push_str(word);
                continue;
            }
        }
        msg_lines.push(word.to_string());
    }

    let overlay_h = (msg_lines.len() as u16 + 4).min(area.height.saturating_sub(2)); // +2 border +1 blank +1 footer
    let overlay_area = centered_rect(area, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();
    for ml in &msg_lines {
        lines.push(Line::from(Span::styled(
            format!(" {}", ml),
            theme::muted_style(),
        )));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(" enter", theme::accent_dim_style()),
        Span::styled(" confirm  ", theme::dim_style()),
        Span::styled("esc", theme::accent_dim_style()),
        Span::styled(" cancel", theme::dim_style()),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::dim_style())
        .title(Span::styled(
            format!(" {} ", popup.title),
            theme::warn_style(),
        ));

    frame.render_widget(Paragraph::new(lines).block(block), overlay_area);
}


pub(crate) fn render_skill_deltas(frame: &mut Frame, area: Rect, deltas: &[SkillDelta], elapsed_ticks: u64) {
    use ratatui::widgets::{Block, Borders, Clear};

    let visible_count = deltas
        .len()
        .min(((elapsed_ticks / 3) as usize + 1).min(deltas.len()));
    // Layout: 2 pad + tag(26) + rating(13 "1823 → 1847") + delta(8 "✦ +24") + 2 pad = 51
    let overlay_w = 55u16.min(area.width.saturating_sub(4));
    let overlay_h = (visible_count as u16 + 4).min(area.height.saturating_sub(2));
    let overlay_area = centered_rect(area, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::raw(""));

    for (i, delta) in deltas.iter().enumerate() {
        if i >= visible_count {
            break;
        }
        let (indicator, style) = if delta.delta > 0 {
            let sparkle_idx = (elapsed_ticks as usize / 2 + i) % SPARKLE.len();
            let s = SPARKLE[sparkle_idx];
            (format!("{} {:>+4}", s, delta.delta), theme::success_style())
        } else {
            (format!("\u{25bc} {:>+4}", delta.delta), theme::fail_style())
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<26}", delta.tag), theme::muted_style()),
            Span::styled(
                format!("{:>4} \u{2192} {:<4}", delta.old_rating, delta.new_rating),
                theme::accent_dim_style(),
            ),
            Span::styled(format!("  {}", indicator), style),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::dim_style())
        .title(Span::styled(" skill update ", theme::purple_style()));

    frame.render_widget(Paragraph::new(lines).block(block), overlay_area);
}


pub(crate) fn render_debug_overlay(frame: &mut Frame, area: Rect, log: &[String], scroll: usize) {
    use ratatui::widgets::{Block, Borders, Clear};

    // Centered box: 80% width, 70% height, clamped
    let overlay_w = (area.width * 4 / 5).max(40).min(area.width.saturating_sub(4));
    let overlay_h = (area.height * 7 / 10).max(8).min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(overlay_w)) / 2;
    let y = (area.height.saturating_sub(overlay_h)) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    // Clear the area behind the overlay
    frame.render_widget(Clear, overlay_area);

    let inner_h = overlay_h.saturating_sub(2) as usize; // border top + bottom
    let max_width = (overlay_w as usize).saturating_sub(4); // border + padding

    // Compute scroll position
    let max_scroll = log.len().saturating_sub(inner_h);
    let effective_scroll = if scroll == 0 {
        // Default: pin to bottom (most recent)
        max_scroll
    } else {
        scroll.min(max_scroll)
    };

    let mut lines = Vec::new();
    for entry in log.iter().skip(effective_scroll).take(inner_h) {
        let display = if entry.len() > max_width {
            &entry[..max_width]
        } else {
            entry.as_str()
        };
        lines.push(Line::from(Span::styled(
            display.to_string(),
            theme::dim_style(),
        )));
    }

    // Pad remaining lines so the block fills
    while lines.len() < inner_h {
        lines.push(Line::raw(""));
    }

    let scroll_indicator = if log.len() > inner_h {
        format!(
            " {}-{}/{} ",
            effective_scroll + 1,
            (effective_scroll + inner_h).min(log.len()),
            log.len(),
        )
    } else {
        format!(" {} entries ", log.len())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::dim_style())
        .title(Span::styled(" Debug Log ", theme::warn_style()))
        .title_bottom(Line::from(vec![
            Span::styled(scroll_indicator, theme::dim_style()),
            Span::styled(" j/k scroll  /debug copy  esc close ", theme::dim_style()),
        ]));

    frame.render_widget(Paragraph::new(lines).block(block), overlay_area);
}
