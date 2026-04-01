# myro-tui

`myro-tui` is the terminal application for Myro.

## Architecture

- `src/app.rs`
  Holds the top-level `App` state and the cross-screen navigation flow.
- `src/recommender_state.rs`
  Owns recommendation worker lifecycle, cached pending problem state, and skill-profile runtime state.
- `src/onboarding.rs`
  Owns handle validation, cookie import, logout, and onboarding state transitions.
- `src/runtime.rs`
  Owns periodic update orchestration for coach polling, timers, recommender events, and verdict polling.
- `src/past_screen.rs`
  Owns Past screen keyboard routing, filter/order popups, and reopen flows.
- `src/settings_screen.rs`
  Owns Settings screen editing, actions, and logout/reset dispatch.
- `src/solving.rs`
  Owns solving-session construction, problem-file loading, solution persistence, and `PastEntry` mutations.
- `src/solving_input.rs`
  Owns solving-screen keyboard routing and slash-command handling.
- `src/ui/`
  Screen-oriented rendering modules and overlays.

## Design rule

`app.rs` should coordinate subsystems, not reimplement their internals.

If a feature is primarily about one of these concerns:

- recommendation lifecycle
- onboarding/auth
- runtime polling
- past/settings screen controllers
- solving-session state
- solving-screen input routing
- rendering

then new logic should usually go in the dedicated module first, and only be wired through `App`.
