---
name: testing-myro
description: Use when testing myro-tui — debugging UX, verifying behavior, writing regression tests, or interactively playing the TUI via tmux to find and fix bugs.
---

# Testing myro-tui

Two approaches: **tmux interactive play** (Claude drives the real app) and **TestApp harness** (automated `cargo test`).

## tmux Interactive Play

Run the real app in tmux. Send keys and capture frames on demand.

```bash
# Boot (80×24 ≈ 2K chars per frame, mock mode = no network)
tmux new-session -d -s myro -x 80 -y 24 'MYRO_COACH_MOCK=1 cargo run -p myro-tui'

# Send keys
tmux send-keys -t myro j            # press j
tmux send-keys -t myro Enter         # Enter
tmux send-keys -t myro C-c           # Ctrl+C
tmux send-keys -t myro '/run' Enter  # type /run then Enter

# Capture (only when you need to look — ~2K chars)
tmux capture-pane -t myro -p

# Kill
tmux kill-session -t myro
```

**Context budget:** Only capture at decision points, not after every keystroke. Each capture is ~2K chars.

**Mock mode:** `MYRO_COACH_MOCK=1` or `mock = true` in config — canned coach responses, no API key needed.

## TestApp Harness (cargo test)

### Builder API

```rust
use myro_tui::testing::{TestApp, Scenario, CapturedFrame};

// Start at different screens
let mut app = TestApp::home().size(80, 24).build();
let mut app = TestApp::settings().build();
let mut app = TestApp::past().build();
let mut app = TestApp::solving(problem_statement, problem_file).build();

// Optional builder methods
TestApp::home()
    .size(120, 40)              // terminal dimensions (default 120×40)
    .user_state(custom_state)    // custom UserState
    .build()
```

### Key Simulation

```rust
app.press(KeyCode::Char('j'));           // key + 1 tick
app.press(KeyCode::Enter);
app.press_mod(KeyCode::Char('c'), KeyModifiers::CONTROL);  // Ctrl+C
app.type_str("/run");                    // each char as key event + 1 tick
app.tick_n(55);                          // N ticks (no input)
```

### Frame Capture

```rust
let frame = app.render();
frame.contains_text("feed me")     // true if any row contains text
frame.text_at_row(0)               // text at row 0
frame.dump()                       // full frame as multiline string
frame.dump_to_file("/tmp/f.txt")   // write for Claude to Read
```

### Scenario Scripting

```rust
let captures = Scenario::new()
    .press_n(KeyCode::Down, 3)
    .capture("nav")
    .press(KeyCode::Enter)
    .capture("result")
    .run(&mut app);

assert!(captures["result"].contains_text("settings"));
```

### Test Problem Helpers

```rust
use myro_tui::solving::{test_problem_file, problem_file_to_statement};
let pf = test_problem_file();        // "Sum of Two Numbers" (800 difficulty)
let ps = problem_file_to_statement(&pf);
```

### Direct State Access

```rust
// Check app state directly
assert!(matches!(app.app.state, AppState::Home { selected: 1 }));
assert!(app.app.should_quit);
assert!(app.app.status_message.is_some());
```

## Common Patterns

**Testing navigation:**
```rust
app.press(KeyCode::Char('j'));
assert!(matches!(app.app.state, AppState::Home { selected: 1 }));
```

**Testing state transitions:**
```rust
app.press(KeyCode::Down);
app.press(KeyCode::Down);
app.press(KeyCode::Enter);
assert!(matches!(app.app.state, AppState::Past { .. }));
```

**Testing render output:**
```rust
let frame = app.render();
assert!(frame.contains_text("cf handle"));
```

**Testing timeouts:**
```rust
app.app.set_status("msg");
app.tick_n(55);
assert!(app.app.status_message.is_none());
```

## Quick Reference

| Action | tmux | TestApp |
|--------|------|---------|
| Boot | `tmux new-session -d -s myro ...` | `TestApp::home().build()` |
| Press key | `tmux send-keys -t myro j` | `app.press(KeyCode::Char('j'))` |
| Ctrl+key | `tmux send-keys -t myro C-c` | `app.press_mod(KeyCode::Char('c'), KeyModifiers::CONTROL)` |
| See screen | `tmux capture-pane -t myro -p` | `app.render().dump()` |
| Check text | read capture output | `frame.contains_text("...")` |
| Wait/tick | real-time | `app.tick_n(N)` |
| Kill/cleanup | `tmux kill-session -t myro` | (drop) |

## Test Files

- `crates/myro-tui/src/testing.rs` — TestApp, CapturedFrame, Scenario
- `crates/myro-tui/tests/smoke.rs` — home screen + navigation tests
- `crates/myro-tui/tests/solving.rs` — solving screen tests

## Running Tests

```bash
cargo test -p myro-tui              # run all TUI tests
cargo test -p myro-tui -- --nocapture  # with output
```
