mod handle_prompt;
mod home;
mod login;
mod overlays;
mod past;
mod settings;
mod shared;
mod solving;
mod stats;

use ratatui::{layout::Rect, Frame};

use crate::app::{App, AppState};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.terminal_width = area.width;
    app.terminal_height = area.height;

    match &app.state {
        AppState::Login { .. } => login::render_login(frame, app, area),
        AppState::HandlePrompt { .. } => handle_prompt::render_handle_prompt(frame, app, area),
        AppState::Home { .. } => home::render_home(frame, app, area),
        AppState::Stats { .. } => stats::render_stats(frame, app, area),
        AppState::Settings { .. } => settings::render_settings(frame, app, area),
        AppState::ProblemSelect { .. } => home::render_problem_select(frame, app, area),
        AppState::Past { .. } => past::render_past(frame, app, area),
        AppState::Solving { .. } => solving::render_solving(frame, app, area),
    }

    render_overlays(frame, app, area);
}

fn render_overlays(frame: &mut Frame, app: &App, area: Rect) {
    if app.debug_visible {
        overlays::render_debug_overlay(frame, area, &app.debug_log, app.debug_scroll);
    }

    if let Some(popup) = &app.confirm_popup {
        overlays::render_confirm_popup(frame, area, popup);
    }

    if let Some(deltas) = &app.recommender.skill_deltas {
        let elapsed = app.tick.wrapping_sub(app.recommender.skill_delta_tick);
        overlays::render_skill_deltas(frame, area, deltas, elapsed);
    }
}
