# TUI Recommender Integration Design

## Goal

Integrate the myro-predict recommender into the TUI so users get personalized CF problem recommendations, can submit solutions in-app, and see predictions update as they solve problems — a TikTok-like seamless training loop.

## Architecture

Core flow:
```
First run → CF handle prompt → fetch history (public API) → fit user embedding → recommend problem at target P(solve)
  ↓
Solve problem → submit via CF (authenticated) → update local history → refit embedding → new recommendation
```

New dependency: `myro-tui` gains a dependency on `myro-predict` (lib crate).

New module in myro-cf: `auth.rs` — CF session-based login and `submit_solution()`.

## CF Authentication & Submission (myro-cf/auth.rs)

### Login Flow (mirrors cf-tool's Go implementation)

`CfAuthClient` wraps a `reqwest::Client` with a persistent cookie jar (`reqwest::cookie::Jar`):

1. `login(handle, password)` → GET `codeforces.com/enter` → regex extract `csrf='(.+?)'`
2. POST `/enter` with `{csrf_token, handleOrEmail, password, ftaa (random 18-char hex), bfaa (fixed hash), _tta, remember: "on"}`
3. Verify login by checking response for `handle = "..."` pattern
4. Session cookies persist in the jar for subsequent requests

### Submit Flow

1. `submit_solution(contest_id, problem_index, source_code, lang_id)` → GET `/contest/{id}/submit` → extract CSRF
2. POST to `/contest/{id}/submit?csrf_token={csrf}` with form fields: `action: "submitSolutionFormSubmitted"`, `submittedProblemIndex`, `programTypeId`, `contestId`, `source`, `tabSize: "4"`, `sourceCodeConfirmed: "true"`
3. Return submission ID parsed from response, or error message

### Credential Storage

Stored in `~/.config/myro/config.toml` under `[codeforces]` section. Password encrypted with AES-GCM (key derived from handle, same scheme as cf-tool) so it's not plaintext on disk.

### Language Mapping

Python 3 → `lang_id = 31` (CF's PyPy 3). Only Python needed since the TUI editor uses `.py`.

## Recommender Integration & Problem Suggestion

### Recommendation Algorithm

1. On startup (or entering "Suggested problem"), load `ProblemModel` from configured path (default `~/.local/share/myro/problem_model.bin.gz`)
2. Fetch user's CF submissions via `fetch_user_status(handle)` — public, no auth needed
3. Build `WeightedObservation`s with time decay, call `fit_user_weighted` to get `UserParams`
4. Call `predict_all` to get P(solve) for all problems in model
5. Filter to unsolved problems, find those within `[p - 0.1, p + 0.1]` of target probability `p`
6. Pick one at random from that band
7. Fetch problem statement on-demand via `fetch_problem_html` + parser

### Target Probability

Default 0.5 (50% chance of solving). Configurable in Settings menu. Range 0.1–0.9.

### Caching

User params cached locally (using existing `cache.rs`) with hash-based invalidation. On subsequent launches, if history hasn't changed, skip re-fitting.

### Live Updates

After a successful submission or `/isuck`, append to local `SolveHistory`, invalidate cache, refit embedding. Next recommendation reflects updated skill profile.

### Background Threading

All network calls (fetch history, fetch problem HTML, submit) happen on a background thread with mpsc channels back to the main event loop — same pattern as the coach. A loading/spinner state shows while fetching.

## Settings Menu & Config

### Config File

`~/.config/myro/config.toml`:

```toml
[codeforces]
handle = "kalimm"
password = "encrypted:base64..."  # AES-GCM encrypted

[recommender]
target_probability = 0.5
model_path = "~/.local/share/myro/problem_model.bin.gz"
```

### Settings TUI Screen

`AppState::Settings` with a vertical list of editable fields:
- CF Handle (text input)
- CF Password (text input, masked with `*`)
- Target Probability (number input, 0.1–0.9)
- Model Path (text input, path)

Navigation: `j/k` to move between fields, `Enter` to edit, `Esc` to cancel, `Enter` to confirm. `q`/`Esc` (when not editing) returns to Home. Changes save to `config.toml` on exit.

### First-Run Flow

- `HandlePrompt` → user enters CF handle → validate via `user_info(handle)` → save to config → `Home`
- If handle already in config, skip straight to `Home`
- Password prompt only on first submit attempt (lazy auth)
- Replaces the old `NamePrompt` — CF handle is now the user identity

## App States & Menu Structure

### Menu Items

```
1. Start training        (existing — local problem set with coach)
2. Suggested problem     (NEW — recommender-driven)
3. Settings              (NEW — replaces "coming soon")
```

### New AppState Variants

- `HandlePrompt { input: String }` — replaces `NamePrompt`. Validates handle against CF API.
- `Settings { fields, selected, editing }` — editable config fields.
- No new state for "Suggested problem" — transitions directly to `Solving` after background fetch. Loading overlay shows on Home while fetching.

### Submit Integration in Solving State

New command: `/submit` in vim command mode.

Flow: authenticate if needed (prompt for password inline) → submit solution → poll verdict every 3s → show verdict in coach panel area.

- **AC:** Record `solved=true` → refit embedding → pick next problem → transition seamlessly to new `Solving` state (TikTok flow).
- **WA/TLE/RE:** Record `solved=false`, show verdict. User can edit and `/submit` again.
- **Multiple failures then AC:** Each attempt recorded. Entry upgrades to `solved=true` on first AC. Failed attempts provide negative signal to embedding.

### /isuck Command

Graceful skip that provides negative signal:

- First time: popup explaining "This marks the problem as attempted-but-failed and moves to a new recommendation. Your predictions will update to reflect this was too hard right now." Dismiss with Enter/Esc.
- Subsequent uses: execute directly (tracked via `isuck_explained: bool` in `UserState`).
- Records `solved=false` → refits embedding → predicts → picks new problem → transitions directly to new `Solving` state (seamless, no menu).

### Unchanged

- "Start training" works as before (local JSON problems + coach)
- All coach commands (`/hint`, `/coach`, `/debug`, etc.)
- `Ctrl+C` double-press, vim keybindings, theme
