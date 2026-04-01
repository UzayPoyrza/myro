# TUI Redesign: Minimal CLI-like Experience

## Problem

The current myro-tui has 4 screens, 9 blocks, a 2D grid navigation system, responsive sidebar with 3 modes, and a help overlay. This is overengineered for the core workflow: myro recommends a problem, you solve it.

## Design

### Inspiration

Claude Code CLI: single scrolling view, no chrome, whitespace-driven layout, one accent color, slash commands for actions.

### Flow

```
Home  ──[Enter "Train"]──>  Solving  ──[\quit]──>  Home
                                │
                                ├── \run   → test results inline
                                ├── \help  → help message inline
                                └── \quit  → back to Home
```

Three states, linear progression. No sidebar, no blocks, no multi-screen navigation.

### Home State

- Header: `myro` left-aligned, rating + topic tags right-aligned
- Text picker menu (j/k + Enter):
  ```
  What would you like to do?

  > Start training
    Browse problems (coming soon)
    Settings (coming soon)
  ```

### Solving State

Full screen split into two zones separated by dim `─` rules:

- **Top: problem statement** — title, time/memory limits, description, examples. Scrollable, read-only. Can be toggled hidden.
- **Bottom: vim editor** — `edtui` with Python syntax highlighting, essential vim motions (i/Esc, hjkl, w/b/e, dd/yy/p, o/O, x, undo).
- **Status line** — mode indicator left (NORMAL/INSERT), slash command hints right.

Backslash `\` is the command prefix (not `/` which conflicts with vim search). Typing `\` in normal mode enters command input mode. Enter executes, Esc cancels.

After `\run`, test results appear as a panel between statement and editor:
```
  Test 1  PASS  0.02s
  Test 2  FAIL  0.01s  expected: 8  got: 7
```

### Aesthetic

"Quiet terminal" — minimal, monochrome with one accent.

- No box borders anywhere. Thin `─` horizontal rules in dim gray.
- One accent color: soft teal/cyan for problem title and active elements only.
- Line numbers: very dim gray with thin `│` separator.
- Empty editor lines: vim-style `~` in very dim gray.
- Syntax highlighting: muted palette (Catppuccin-inspired, not neon).
- Mode indicator: bottom-left, understated.
- Slash commands: bottom-right, dim.
- Header: `myro` left, rating + tags right, one line.
- Examples: `stdin`/`stdout` labels (terse).
- No emoji, no flashing status messages, no colored borders.

Visual reference:
```
  myro                                          1842 > dp, greedy
  ────────────────────────────────────────────────────────────────

  A. Sum of Two Numbers                           time: 2s | mem: 256MB

  You are given two integers a and b. Print their sum.

  Input   One line containing two integers a and b (1 <= a, b <= 10^9)
  Output  Print the sum a + b.

  -- Example 1 ---------------------
  stdin     3 5
  stdout    8

  ────────────────────────────────────────────────────────────────
  1 | a, b = map(int, input().split())
  2 | print(a + b)
  ~
  ~
  ────────────────────────────────────────────────────────────────
  NORMAL                                       \run  \help  \quit
```

### Architecture

**Files** (`crates/myro-tui/src/`):

| File | Purpose | ~Lines |
|------|---------|--------|
| `main.rs` | Event loop, terminal init/restore | 80 |
| `app.rs` | State machine, key dispatch | 200 |
| `event.rs` | Crossterm event reader | 40 |
| `theme.rs` | Colors, styles | 50 |
| `editor.rs` | EdTui wrapper + customization | 100 |
| `runner.rs` | Local Python3 test runner (existing) | 80 |
| `ui.rs` | Single render function | 300 |

No `ui/` subdirectory. No views, layout, sidebar modules.

**App state:**

```rust
enum AppState {
    Home { selected: usize },
    Solving {
        problem: ProblemStatement,
        editor: EditorState,
        results: Option<Vec<TestResult>>,
        show_statement: bool,
        command_input: Option<String>,
    },
}
```

**Key handling layers:**
1. Command input mode (`\` prefix active): capture command text, Enter/Esc
2. Home state: j/k selection, Enter picks
3. Solving state: all keys to edtui, except `\` in normal mode starts command input

**Dependencies:**
- Upgrade: `ratatui` 0.30, `crossterm` 0.29
- Add: `edtui` (with `syntax-highlighting` feature)
- Remove: `clap`, `open`, `chrono`
- Keep: `myro-cf`, `rusqlite`, `tokio`, `serde`, `serde_json`, `anyhow`, `dirs`

### MVP Scope

- Hardcoded test problem: "Sum of Two Numbers" (a + b). No CF fetching.
- Solution files saved to `~/.local/share/myro/solutions/`
- `\run` runs Python3 against example test cases
- `\help` prints keybinding summary inline
- `\quit` returns to Home
- "Browse problems" and "Settings" show "coming soon"

### What Gets Deleted

Everything in the current TUI except `event.rs` (minor tweaks) and `runner.rs` (keep as-is):
- `app.rs` — rewritten from scratch
- `main.rs` — rewritten (simpler)
- `theme.rs` — rewritten (fewer styles)
- `ui/` directory — replaced by single `ui.rs`
- All views: dashboard, problems, problem_detail, settings, help, sidebar, status_bar, layout
