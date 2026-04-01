# Source Layout

This directory is split by responsibility.

## Runtime coordinators

- `app.rs`
  Top-level application state and screen-to-screen orchestration.
- `runtime.rs`
  Periodic update loop for onboarding polling, solving runtime, timers, and recommender events.
- `event.rs`
  Terminal event loop input.

## Screen controllers

- `settings_screen.rs`
  Settings navigation, editing, and action dispatch.
- `past_screen.rs`
  Past screen navigation, command input, filter/order popups, and reopen flow.

## Solving flow

- `solving.rs`
  Builds solving sessions and updates persisted solve history.
- `solving_input.rs`
  Routes keys inside the solving screen.
  It decides whether input goes to:
  - slash-command input
  - test panel
  - statement viewer
  - main editor

## External integration boundaries

- `onboarding.rs`
  Codeforces handle validation, cookie import, and logout transitions.
- `recommender_state.rs`
  Recommendation worker state, cached pending problem, and skill profile.
- `recommend.rs`
  Background recommendation worker implementation.
- `browser.rs`
  Firefox cookie and user-agent import helpers.

## Editing guideline

Keep modules narrow:

- periodic update logic belongs in `runtime.rs`
- settings/past input logic belongs in their screen controller modules
- input routing belongs in `solving_input.rs`
- solving lifecycle and persistence rules belong in `solving.rs`
- onboarding rules belong in `onboarding.rs`
- rendering belongs in `ui/`

## Rendering layout

- `ui/mod.rs`
  Top-level render dispatcher and overlay ordering.
- `ui/handle_prompt.rs`
  Onboarding screen rendering.
- `ui/home.rs`
  Home and problem-selection rendering.
- `ui/stats.rs`
  Skill profile rendering.
- `ui/settings.rs`
  Settings screen rendering.
- `ui/past.rs`
  Past screen rendering and its popups.
- `ui/solving.rs`
  Solving screen rendering.
- `ui/overlays.rs`
  Shared overlays such as loading, confirm, debug, and skill delta popups.
- `ui/shared.rs`
  Shared symbols and small rendering helpers.

If a change touches multiple areas, keep the policy in the owned module and leave `app.rs` as the composition layer.
