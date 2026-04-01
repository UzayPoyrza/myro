use crossterm::event::{KeyCode, KeyModifiers};
use myro_tui::app::AppState;
use myro_tui::testing::{Scenario, TestApp};

#[test]
fn home_screen_renders() {
    let mut app = TestApp::home().size(80, 24).build();
    let frame = app.render();
    assert!(frame.contains_text("feed me"), "missing 'feed me':\n{}", frame.dump());
    assert!(frame.contains_text("rate me"), "missing 'rate me'");
    assert!(frame.contains_text("my past"), "missing 'my past'");
    assert!(frame.contains_text("settings"), "missing 'settings'");
}

#[test]
fn home_navigate_j() {
    let mut app = TestApp::home().size(80, 24).build();
    assert!(matches!(app.app.state, AppState::Home { selected: 0 }));
    app.press(KeyCode::Char('j'));
    assert!(matches!(app.app.state, AppState::Home { selected: 1 }));
}

#[test]
fn home_navigate_k() {
    let mut app = TestApp::home().size(80, 24).build();
    // Move down first, then back up
    app.press(KeyCode::Char('j'));
    app.press(KeyCode::Char('k'));
    assert!(matches!(app.app.state, AppState::Home { selected: 0 }));
}

#[test]
fn ctrl_c_double_quits() {
    let mut app = TestApp::home().size(80, 24).build();

    // First Ctrl+C: warns
    app.press_mod(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(app.app.quit_pending);
    assert!(!app.app.should_quit);
    assert!(app.app.status_message.as_deref().unwrap().contains("ctrl+c"));

    // Second Ctrl+C: quits
    app.press_mod(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(app.app.should_quit);
}

#[test]
fn navigate_to_settings() {
    let mut app = TestApp::home().size(80, 24).build();
    let captures = Scenario::new()
        .press_n(KeyCode::Down, 3)
        .press(KeyCode::Enter)
        .capture("settings")
        .run(&mut app);

    let frame = &captures["settings"];
    assert!(frame.contains_text("cf handle"), "settings screen should show 'cf handle':\n{}", frame.dump());
}

#[test]
fn navigate_to_stats() {
    let mut app = TestApp::home().size(80, 24).build();
    app.press(KeyCode::Down); // selected: 1 (rate me)
    app.press(KeyCode::Enter);
    assert!(matches!(app.app.state, AppState::Stats { .. }));
}

#[test]
fn navigate_to_past() {
    let mut app = TestApp::home().size(80, 24).build();
    // Down×2 → "my past"
    app.press(KeyCode::Down);
    app.press(KeyCode::Down);
    app.press(KeyCode::Enter);
    assert!(matches!(app.app.state, AppState::Past { .. }));
}

#[test]
fn escape_returns_home() {
    let mut app = TestApp::settings().size(80, 24).build();
    assert!(matches!(app.app.state, AppState::Settings { .. }));
    app.press(KeyCode::Esc);
    assert!(matches!(app.app.state, AppState::Home { .. }));
}

#[test]
fn q_from_home_quits() {
    let mut app = TestApp::home().size(80, 24).build();
    app.press(KeyCode::Char('q'));
    assert!(app.app.should_quit);
}

#[test]
fn status_clears_after_timeout() {
    let mut app = TestApp::home().size(80, 24).build();
    app.app.set_status("test message");
    assert!(app.app.status_message.is_some());

    // Tick past the 50-tick timeout
    app.tick_n(55);
    assert!(app.app.status_message.is_none(), "status should clear after ~50 ticks");
}

#[test]
fn scenario_scripting_works() {
    let mut app = TestApp::home().size(80, 24).build();
    let captures = Scenario::new()
        .capture("initial")
        .press(KeyCode::Char('j'))
        .capture("after_j")
        .run(&mut app);

    assert!(captures.contains_key("initial"));
    assert!(captures.contains_key("after_j"));
    assert!(captures["initial"].contains_text("feed me"));
}
