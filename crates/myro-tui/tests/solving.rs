use crossterm::event::KeyCode;
use myro_tui::app::AppState;
use myro_tui::solving::{problem_file_to_statement, test_problem_file};
use myro_tui::testing::TestApp;

fn build_solving_app() -> TestApp {
    let pf = test_problem_file();
    let ps = problem_file_to_statement(&pf);
    TestApp::solving(ps, pf).size(120, 40).build()
}

#[test]
fn solving_shows_problem_title() {
    let mut app = build_solving_app();
    let frame = app.render();
    assert!(
        frame.contains_text("Sum of Two Numbers"),
        "problem title not visible:\n{}",
        frame.dump()
    );
}

#[test]
fn slash_enters_command_mode() {
    let mut app = build_solving_app();
    app.press(KeyCode::Char('/'));
    match &app.app.state {
        AppState::Solving { command_input, .. } => {
            assert!(command_input.is_some(), "command_input should be Some after /");
        }
        _ => panic!("expected Solving state"),
    }
}

#[test]
fn tab_toggles_statement_focus() {
    let mut app = build_solving_app();
    // Initially statement_focused is false
    match &app.app.state {
        AppState::Solving { statement_focused, .. } => {
            assert!(!statement_focused, "initially not focused on statement");
        }
        _ => panic!("expected Solving state"),
    }

    app.press(KeyCode::Tab);

    match &app.app.state {
        AppState::Solving { statement_focused, .. } => {
            assert!(*statement_focused, "Tab should focus statement");
        }
        _ => panic!("expected Solving state"),
    }
}
