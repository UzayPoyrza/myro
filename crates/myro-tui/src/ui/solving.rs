use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{App, AppState, SolveMode},
    editor,
    theme,
};

use super::shared::{
    render_panel_separator, separator, CHECK, CROSS, DIAMOND, DOT,
    SEPARATOR_CHAR, SPARKLE, SPINNER, TIMER,
};

pub(crate) fn results_render_height(results: &[crate::runner::TestResult]) -> u16 {
    // Summary line always
    let mut h: u16 = 1;
    // Find first failure and compute its height
    if let Some(first_fail) = results.iter().find(|r| !r.passed) {
        h += 1; // expected/got line
        if let Some(err) = &first_fail.error {
            if !err.is_empty() {
                h += err.lines().count() as u16;
            }
        }
    }
    h
}

pub(crate) fn render_solving(frame: &mut Frame, app: &mut App, area: Rect) {
    let show_statement = match &app.state {
        AppState::Solving { show_statement, .. } => *show_statement,
        _ => return,
    };

    let is_running = matches!(
        &app.state,
        AppState::Solving { running: Some(_), .. }
    );

    let has_coach = matches!(
        &app.state,
        AppState::Solving { coach: Some(ref c), .. } if c.panel_visible
    );

    let has_test_panel = matches!(
        &app.state,
        AppState::Solving { test_panel: Some(ref tp), .. } if tp.visible
    );
    // Results side panel (show when test panel is not visible)
    let bottom_height = if !has_test_panel {
        if is_running {
            1
        } else {
            match &app.state {
                AppState::Solving { results: Some(results), .. } => results_render_height(results),
                _ => 0,
            }
        }
    } else {
        0
    };

    let statement_height = if show_statement { 16u16.min(area.height / 2) } else { 0 };

    let has_bottom = bottom_height > 0;
    // Compute coach panel height dynamically: 1 header + wrapped message lines, capped at 6
    let coach_height: u16 = if has_coach {
        let max_wrap_width = (area.width as usize).saturating_sub(4);
        let msg_lines: u16 = match &app.state {
            AppState::Solving { coach: Some(ref c), .. } => {
                c.panel_lines.iter().map(|l| {
                    if max_wrap_width == 0 { return 1u16; }
                    // Simulate word-wrap to count lines accurately
                    let mut remaining = l.text.as_str();
                    let mut count = 0u16;
                    while !remaining.is_empty() {
                        count += 1;
                        if remaining.len() <= max_wrap_width { break; }
                        let break_at = remaining[..max_wrap_width].rfind(' ').unwrap_or(max_wrap_width);
                        remaining = remaining[break_at..].trim_start();
                    }
                    count.max(1)
                }).sum()
            }
            _ => 1,
        };
        (1 + msg_lines).min(6) // header + message, cap at 6 lines
    } else {
        0
    };
    let coach_sep_height: u16 = if has_coach { 1 } else { 0 };

    let chunks = Layout::vertical([
        Constraint::Length(statement_height),                           // [0] statement
        Constraint::Length(if statement_height > 0 { 1 } else { 0 }), // [1] sep after statement
        Constraint::Min(3),                                            // [2] editor row (horiz split below)
        Constraint::Length(coach_sep_height),                          // [3] sep above coach
        Constraint::Length(coach_height),                              // [4] coach panel
        Constraint::Length(1),                                         // [5] separator
        Constraint::Length(1),                                         // [6] status line
    ])
    .split(area);

    // Horizontal split: editor | right panel (test panel or results)
    const EDITOR_MIN_WIDTH: u16 = 40;
    const RESULTS_MIN_WIDTH: u16 = 26;
    const RESULTS_MAX_WIDTH: u16 = 55;
    let right_width: u16 = if has_test_panel {
        let available = area.width.saturating_sub(EDITOR_MIN_WIDTH);
        available.min(55).max(if available >= 30 { 30 } else { 0 })
    } else if has_bottom {
        let available = area.width.saturating_sub(EDITOR_MIN_WIDTH);
        if available < RESULTS_MIN_WIDTH { 0 } else { available.min(RESULTS_MAX_WIDTH) }
    } else {
        0
    };
    let editor_row = Layout::horizontal([
        Constraint::Min(10),
        Constraint::Length(right_width),
    ])
    .split(chunks[2]);

    let status_message = app.status_message.clone();
    let recommend_status = app.recommender.status.clone();
    let pending_problem = app.recommender.pending_problem.clone();
    let tick = app.tick;
    let term_width = area.width;

    if let AppState::Solving {
        problem,
        editor_state,
        results,
        running,
        command_input,
        coach,
        statement_scroll,
        statement_focused,
        test_panel,
        mode: solve_mode_ref,
        timer_started,
        timer_paused_secs,
        timer_expired,
        from_past: from_past_ref,
        ..
    } = &mut app.state
    {
        let solve_mode = *solve_mode_ref;
        let from_past = *from_past_ref;
        let paused = *timer_paused_secs;
        // Also account for currently-paused time (skill popup open)
        let extra_pause = app.timer_pause_start
            .map(|ps: std::time::Instant| ps.elapsed().as_secs())
            .unwrap_or(0);
        let timer_remaining = match (solve_mode, timer_started.as_ref(), *timer_expired) {
            (SolveMode::Intense, Some(_), true) => Some(0u64),
            (SolveMode::Intense, Some(started), false) => {
                let effective = started.elapsed().as_secs().saturating_sub(paused + extra_pause);
                Some(crate::app::INTENSE_TIMER_SECS.saturating_sub(effective))
            }
            _ => None,
        };
        // Statement
        if show_statement {
            render_statement(frame, chunks[0], problem, *statement_scroll, *statement_focused);
            frame.render_widget(
                render_panel_separator(area.width, *statement_focused),
                chunks[1],
            );
        }

        // Editor — inactive when test panel is open
        let editor_area = editor_row[0];
        let tp_visible = test_panel.as_ref().is_some_and(|tp| tp.visible);
        let editor_active = !tp_visible && !*statement_focused;
        editor::render_editor(frame, editor_area, editor_state, editor_active);

        // Ghost text overlay (rendered after editor into the buffer)
        if let Some(ref coach_state) = coach {
            if let Some(ref ghost) = coach_state.ghost_text {
                let cursor_row = editor_state.cursor.row as u16;
                crate::coach::ghost::render_ghost_text(
                    frame.buffer_mut(),
                    editor_area,
                    ghost,
                    cursor_row,
                    tick,
                );
            }
        }

        // Right panel: test panel (with input + output) or plain results
        if right_width > 0 {
            let side = editor_row[1];
            // Left border
            for y in side.top()..side.bottom() {
                frame.buffer_mut()[ratatui::layout::Position { x: side.left(), y }]
                    .set_symbol(&SEPARATOR_CHAR.to_string())
                    .set_style(theme::dim_style());
            }
            let inner = Rect::new(side.left() + 1, side.top(), side.width.saturating_sub(1), side.height);

            if tp_visible {
                if let Some(ref mut tp) = test_panel {
                    render_test_panel(frame, inner, tp, results, running, tick);
                }
            } else {
                // Standalone results (test panel hidden or absent)
                if running.is_some() {
                    let spinner = SPINNER[(tick / 2) as usize % SPINNER.len()];
                    frame.render_widget(
                        Paragraph::new(Line::from(vec![
                            Span::raw(" "),
                            Span::styled(spinner, theme::accent_style()),
                            Span::styled(" Running...", theme::muted_style()),
                        ])),
                        inner,
                    );
                } else if let Some(results) = results {
                    render_results(frame, inner, results, tick);
                }
            }
        }

        // Coach panel
        if let Some(ref coach_state) = coach {
            if coach_state.panel_visible {
                if coach_state.thinking {
                    render_animated_separator(frame, chunks[3], tick);
                } else {
                    frame.render_widget(
                        Paragraph::new(Line::styled(
                            separator(area.width),
                            theme::dim_style(),
                        )),
                        chunks[3],
                    );
                }
                crate::coach::panel::render_coach_panel(
                    frame,
                    chunks[4],
                    coach_state,
                    term_width,
                    tick,
                );
            }
        }

        // Bottom separator
        frame.render_widget(
            Paragraph::new(Line::styled(separator(area.width), theme::dim_style())),
            chunks[5],
        );

        // Status line — show test input's vim mode when test panel is open
        let has_active_coach = coach.is_some();
        let has_hints = coach.is_some();
        let test_panel_open = tp_visible;
        let display_mode = if let Some(ref tp) = test_panel {
            if tp.visible { &tp.input_state.mode } else { &editor_state.mode }
        } else {
            &editor_state.mode
        };
        let skill_popup_open = app.recommender.skill_popup_open();
        render_status_line(
            frame,
            chunks[6],
            display_mode,
            command_input,
            &status_message,
            &recommend_status,
            &pending_problem,
            has_active_coach,
            has_hints,
            test_panel_open,
            tick,
            solve_mode,
            timer_remaining,
            from_past,
            skill_popup_open,
        );
    }

}

/// Build a single-line span list with styled ↵ between lines.
pub(crate) fn inline_with_newlines(prefix: &str, label: &str, label_style: Style, text: &str) -> Vec<Span<'static>> {
    inline_with_newlines_styled(prefix, label, label_style, text, Style::default())
}

pub(crate) fn inline_with_newlines_styled(prefix: &str, label: &str, label_style: Style, text: &str, content_style: Style) -> Vec<Span<'static>> {
    let mut spans = vec![
        Span::raw(prefix.to_string()),
        Span::styled(label.to_string(), label_style),
    ];
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        spans.push(Span::styled(line.to_string(), content_style));
        if i + 1 < lines.len() {
            spans.push(Span::styled(" ↵ ", theme::newline_style()));
        }
    }
    spans
}

/// Convert a text line with `$$$...$$$` math into styled Spans.
pub(crate) fn styled_text_line(prefix: &str, text: &str) -> Line<'static> {
    let segments = myro_cf::convert_cf_latex_styled(text);
    let mut spans: Vec<Span<'static>> = Vec::new();
    if !prefix.is_empty() {
        spans.push(Span::raw(prefix.to_string()));
    }
    for seg in segments {
        if seg.is_math {
            spans.push(Span::styled(seg.text, theme::math_style()));
        } else {
            spans.push(Span::raw(seg.text));
        }
    }
    Line::from(spans)
}

pub(crate) fn render_statement(frame: &mut Frame, area: Rect, problem: &myro_cf::ProblemStatement, scroll: u16, _focused: bool) {
    let mut lines = vec![];

    // Title line
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(DIAMOND, theme::purple_style()),
        Span::raw(" "),
        Span::styled(problem.title.clone(), theme::accent_bold()),
    ]));

    // Constraints line
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(TIMER, theme::dim_style()),
        Span::styled(format!(" {} ", problem.time_limit), theme::muted_style()),
        Span::styled(DOT, theme::dim_style()),
        Span::styled(format!(" {}", problem.memory_limit), theme::muted_style()),
    ]));
    lines.push(Line::raw(""));

    // Description (with styled math)
    for desc_line in problem.description.lines() {
        lines.push(styled_text_line("  ", desc_line));
    }
    lines.push(Line::raw(""));

    // Input spec (with styled math) — all lines
    let mut in_spec_lines = problem.input_spec.lines();
    if let Some(first) = in_spec_lines.next() {
        let mut input_spans = vec![
            Span::raw("  "),
            Span::styled("▸ input   ".to_string(), theme::accent_dim_style()),
        ];
        for seg in myro_cf::convert_cf_latex_styled(first) {
            if seg.is_math {
                input_spans.push(Span::styled(seg.text, theme::math_style()));
            } else {
                input_spans.push(Span::raw(seg.text));
            }
        }
        lines.push(Line::from(input_spans));
    }
    for line in in_spec_lines {
        lines.push(styled_text_line("            ", line));
    }

    // Output spec (with styled math) — all lines
    let mut out_spec_lines = problem.output_spec.lines();
    if let Some(first) = out_spec_lines.next() {
        let mut output_spans = vec![
            Span::raw("  "),
            Span::styled("▸ output  ".to_string(), theme::accent_dim_style()),
        ];
        for seg in myro_cf::convert_cf_latex_styled(first) {
            if seg.is_math {
                output_spans.push(Span::styled(seg.text, theme::math_style()));
            } else {
                output_spans.push(Span::raw(seg.text));
            }
        }
        lines.push(Line::from(output_spans));
    }
    for line in out_spec_lines {
        lines.push(styled_text_line("            ", line));
    }
    lines.push(Line::raw(""));

    // Examples
    for (i, ex) in problem.examples.iter().enumerate() {
        let label = format!("  ── example {} ", i + 1);
        let dashes =
            SEPARATOR_CHAR.to_string().repeat((area.width as usize).saturating_sub(label.len() + 2));
        lines.push(Line::styled(
            format!("{}{}", label, dashes),
            theme::dim_style(),
        ));

        lines.push(Line::from(inline_with_newlines("    ", "in  ", theme::warn_style(), &ex.input)));
        lines.push(Line::from(inline_with_newlines("    ", "out ", theme::purple_style(), &ex.output)));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn render_test_panel(
    frame: &mut Frame,
    area: Rect,
    tp: &mut crate::test_panel::TestPanelState,
    results: &Option<Vec<crate::runner::TestResult>>,
    running: &Option<std::sync::mpsc::Receiver<crate::runner::TestResult>>,
    tick: u64,
) {
    // Split vertically: upper (input editor) | separator | lower (output)
    let input_height = (area.height / 2).max(3);
    let output_height = area.height.saturating_sub(input_height + 1);
    let panel = Layout::vertical([
        Constraint::Length(input_height),
        Constraint::Length(1),
        Constraint::Length(output_height),
    ])
    .split(area);

    // Input editor header label on first row, editor below
    let input_area = panel[0];
    let label_area = Rect::new(input_area.x, input_area.y, input_area.width, 1);
    let editor_area = Rect::new(
        input_area.x,
        input_area.y + 1,
        input_area.width,
        input_area.height.saturating_sub(1),
    );

    let input_label_style = if tp.output_focused {
        theme::dim_style()
    } else {
        theme::accent_dim_style()
    };
    frame.render_widget(
        Paragraph::new(Line::styled(" input", input_label_style)),
        label_area,
    );
    let input_active = !tp.output_focused;
    crate::editor::render_editor_plain(frame, editor_area, &mut tp.input_state, input_active);

    // Horizontal separator
    frame.render_widget(
        Paragraph::new(Line::styled(
            SEPARATOR_CHAR.to_string().repeat(area.width as usize),
            theme::dim_style(),
        )),
        panel[1],
    );

    // Output area
    let out_area = panel[2];
    let output_label_style = if tp.output_focused {
        theme::accent_dim_style()
    } else {
        theme::dim_style()
    };

    // Runall progress: show per-test results as they arrive
    if let Some((done, total)) = tp.run_progress {
        let mut lines = vec![];
        lines.push(Line::styled(" output", output_label_style));
        if let Some(ref r) = results {
            for tr in r {
                let (sym, style) = if tr.passed {
                    (CHECK, theme::success_style())
                } else {
                    (CROSS, theme::fail_style())
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(sym, style),
                    Span::styled(format!(" Test {}", tr.test_num), style),
                    Span::styled(format!("  {}ms", tr.runtime_ms), theme::dim_style()),
                ]));
            }
        }
        if done < total {
            let spinner = SPINNER[(tick / 2) as usize % SPINNER.len()];
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(spinner, theme::accent_style()),
                Span::styled(format!(" Running {}/{}...", done + 1, total), theme::muted_style()),
            ]));
        }
        let scroll = tp.output_scroll as u16;
        frame.render_widget(
            Paragraph::new(lines).scroll((scroll, 0)).wrap(Wrap { trim: false }),
            out_area,
        );
    } else if running.is_some() {
        let spinner = SPINNER[(tick / 2) as usize % SPINNER.len()];
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(" output", output_label_style),
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(spinner, theme::accent_style()),
                    Span::styled(" Running...", theme::muted_style()),
                ]),
            ]),
            out_area,
        );
    } else if let Some(ref r) = results {
        // Finished: show label + results with scroll
        let mut lines = vec![Line::styled(" output", output_label_style)];
        let result_lines = build_result_lines(r, tick);
        lines.extend(result_lines);
        let scroll = tp.output_scroll as u16;
        frame.render_widget(
            Paragraph::new(lines).scroll((scroll, 0)).wrap(Wrap { trim: false }),
            out_area,
        );
    } else {
        frame.render_widget(
            Paragraph::new(Line::styled(" output", output_label_style)),
            out_area,
        );
    }
}

pub(crate) fn build_result_lines<'a>(results: &[crate::runner::TestResult], tick: u64) -> Vec<Line<'a>> {
    let mut lines = vec![];
    let total_ms: u64 = results.iter().map(|r| r.runtime_ms).sum();
    let time_str = format!("{:.2}s", total_ms as f64 / 1000.0);

    // Custom input: just show output, no pass/fail comparison
    if results.len() == 1 && results[0].is_custom {
        let r = &results[0];
        if let Some(err) = &r.error {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(CROSS, theme::fail_style()),
                Span::styled(format!("  {}", time_str), theme::dim_style()),
            ]));
            for err_line in err.lines() {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(err_line.to_string(), theme::warn_style()),
                ]));
            }
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("out", theme::accent_dim_style()),
                Span::styled(format!("  {}", time_str), theme::dim_style()),
            ]));
            for out_line in r.actual.lines() {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(out_line.to_string(), theme::accent_style()),
                ]));
            }
        }
        return lines;
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    let all_passed = passed == total;

    // Summary line
    if all_passed {
        let s1 = SPARKLE[(tick / 2) as usize % SPARKLE.len()];
        let s2 = SPARKLE[((tick / 2) as usize + 2) % SPARKLE.len()];
        let s3 = SPARKLE[((tick / 2) as usize + 4) % SPARKLE.len()];
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(CHECK, theme::success_style()),
            Span::styled(
                format!(" {}/{} passed", passed, total),
                theme::success_style(),
            ),
            Span::styled(format!("  {}  ", time_str), theme::dim_style()),
            Span::styled(s1, theme::accent_style()),
            Span::styled(s2, theme::purple_style()),
            Span::styled(s3, theme::accent_style()),
        ]));
    } else {
        let first_fail = results.iter().find(|r| !r.passed).unwrap();
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(CROSS, theme::fail_style()),
            Span::styled(
                format!(" {}/{} passed", passed, total),
                theme::fail_style(),
            ),
            Span::styled(format!("  {}", time_str), theme::dim_style()),
            Span::styled(
                format!(" — test {} failed", first_fail.test_num),
                theme::muted_style(),
            ),
        ]));

        lines.push(Line::from(inline_with_newlines_styled("    ", "expected: ", theme::muted_style(), &first_fail.expected, theme::success_style())));
        lines.push(Line::from(inline_with_newlines_styled("    ", "got:      ", theme::muted_style(), &first_fail.actual, theme::fail_style())));

        if let Some(err) = &first_fail.error {
            if !err.is_empty() {
                for err_line in err.lines() {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(err_line.to_string(), theme::warn_style()),
                    ]));
                }
            }
        }
    }

    lines
}

pub(crate) fn render_results(
    frame: &mut Frame,
    area: Rect,
    results: &[crate::runner::TestResult],
    tick: u64,
) {
    let lines = build_result_lines(results, tick);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

/// Animated separator with a bright segment scanning left-to-right while thinking.
fn render_animated_separator(frame: &mut Frame, area: Rect, tick: u64) {
    let width = area.width as usize;
    if width == 0 {
        return;
    }
    let highlight_len = 4;
    // Sweep position moves every 2 ticks
    let pos = ((tick / 2) as usize) % (width + highlight_len);
    let mut spans = Vec::new();
    let sep = SEPARATOR_CHAR.to_string();
    for i in 0..width {
        let in_highlight = i + highlight_len > pos && i < pos;
        let style = if in_highlight {
            theme::accent_style()
        } else {
            theme::dim_style()
        };
        spans.push(Span::styled(sep.clone(), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_status_line(
    frame: &mut Frame,
    area: Rect,
    mode: &edtui::EditorMode,
    command_input: &Option<String>,
    status_message: &Option<String>,
    recommend_status: &Option<String>,
    pending_problem: &Option<(i64, String, f64, Option<i32>)>,
    has_coach: bool,
    has_hints: bool,
    test_panel_open: bool,
    tick: u64,
    solve_mode: SolveMode,
    timer_remaining: Option<u64>,
    from_past: bool,
    skill_popup_open: bool,
) {
    // Recommend status with spinner takes priority
    if let Some(status) = recommend_status {
        let spinner = SPINNER[(tick / 2) as usize % SPINNER.len()];
        let p = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(spinner, theme::accent_style()),
            Span::styled(format!(" {}", status), theme::muted_style()),
        ]));
        frame.render_widget(p, area);
        return;
    }

    // Status message gets full width with wrapping (no right hints)
    if let Some(msg) = status_message {
        // Color based on content
        let style = if msg.starts_with('\u{2713}') {
            theme::success_style()
        } else if msg.starts_with('\u{2717}') {
            theme::fail_style()
        } else {
            theme::warn_style()
        };
        let p = Paragraph::new(Line::from(Span::styled(format!("  {}", msg), style)))
            .wrap(Wrap { trim: false });
        frame.render_widget(p, area);
        return;
    }

    let left = if let Some(cmd) = command_input {
        Span::styled(format!("  /{}", cmd), theme::accent_style())
    } else {
        let (mode_str, style) = match mode {
            edtui::EditorMode::Normal => ("● NORMAL", theme::muted_style()),
            edtui::EditorMode::Insert => ("● INSERT", theme::accent_style()),
            edtui::EditorMode::Visual => ("● VISUAL", theme::purple_style()),
            edtui::EditorMode::Search => ("● SEARCH", theme::warn_style()),
        };
        Span::styled(format!("  {}", mode_str), style)
    };

    // Show timer for Intense mode or recommendation info
    let mid = if let Some(secs) = timer_remaining {
        let mins = secs / 60;
        let s = secs % 60;
        format!("  {} {:02}:{:02}", TIMER, mins, s)
    } else if let Some((_, _, pred_p, rating)) = pending_problem {
        let rating_str = rating
            .map(|r| format!("  {} {}", DOT, r))
            .unwrap_or_default();
        format!("  P(solve): {:.0}%{}", pred_p * 100.0, rating_str)
    } else {
        String::new()
    };

    let mid_style = if let Some(secs) = timer_remaining {
        if secs > 600 { theme::muted_style() }
        else if secs > 300 { theme::warn_style() }
        else { theme::fail_style() }
    } else {
        theme::accent_dim_style()
    };

    let right_spans: Option<Vec<Span>> = if skill_popup_open {
        Some(vec![
            Span::styled("enter", theme::accent_dim_style()),
            Span::styled(" next  ", theme::dim_style()),
            Span::styled("esc", theme::accent_dim_style()),
            Span::styled(" menu  ", theme::dim_style()),
        ])
    } else if test_panel_open {
        Some(vec![
            Span::styled("/run  /runall  /sample N  ", theme::dim_style()),
            Span::styled("esc", theme::accent_dim_style()),
            Span::styled(" back  ", theme::dim_style()),
        ])
    } else {
        None
    };
    let right_text = if right_spans.is_some() {
        "" // handled by right_spans
    } else {
        match (solve_mode, from_past) {
            (SolveMode::Chill, true) => {
                if has_coach && has_hints {
                    "/run  /submit  /coach  /hint  /quit  "
                } else {
                    "/run  /submit  /coach  /debug  /quit  "
                }
            }
            (SolveMode::Chill, false) => {
                if has_coach && has_hints {
                    "/run  /submit  /coach  /hint  /skip  /quit  "
                } else {
                    "/run  /submit  /coach  /skip  /debug  /quit  "
                }
            }
            (SolveMode::Intense, _) => {
                "/run  /submit  /isuck  "
            }
        }
    };
    // Calculate padding and build right side
    let left_len = left.content.len();
    let mid_len = mid.len();
    let right_len = if let Some(ref rs) = right_spans {
        rs.iter().map(|s| s.content.len()).sum()
    } else {
        right_text.len()
    };
    let padding = (area.width as usize).saturating_sub(left_len + mid_len + right_len);

    let mut spans = vec![left];
    if !mid.is_empty() {
        spans.push(Span::styled(mid, mid_style));
    }
    spans.push(Span::raw(" ".repeat(padding)));
    if let Some(rs) = right_spans {
        spans.extend(rs);
    } else {
        spans.push(Span::styled(right_text, theme::dim_style()));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
