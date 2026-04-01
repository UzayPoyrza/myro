# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Philosophy

Lean, clean code that is easy to reason about. Prefer flat files over databases until persistence is truly needed. Minimize abstraction layers.

## Project Overview

Myro is a competitive programming trainer with two surfaces: a Rust TUI and a web-based Observation Coach. The TUI aggregates Codeforces problems with a logistic MF model for solve probability prediction. The web trainer guides users through algorithmic insights using LLM coaching. Early development (v0.1.0 planned).

## Build & Development Commands

A `justfile` is provided for common workflows:

```bash
# Rust TUI
just                     # Run the TUI app (cargo run -p myro-tui)
just build               # Build all workspace crates
just check               # Type-check all crates
just clippy              # Lint all crates
just test                # Run all tests
just fmt                 # Format code
just predict <args>      # Run myro-predict with args
just mock-tui            # Run TUI with mock coach (no API key needed)
# Coaching Bench
just bench               # Run coaching benchmark (all scenarios)
just bench --dry-run     # Print prompts without LLM calls
just bench --filter <name> # Run matching scenarios only
just bench-report        # Generate report from existing results
# Web Trainer (apps/web)
just web-setup           # npm install + prisma generate + db push + seed
just web-dev             # Start Next.js dev server (localhost:3000)
just web-build           # Production build
just web-seed            # Re-seed the database
just web-reset           # Drop and re-seed the database
```

Raw cargo equivalents:
```bash
cargo run -p myro-tui       # Run TUI (produces `myro` binary)
cargo run -p myro-predict   # Run prediction CLI
cargo test <test_name>      # Run a single test
cargo build --release       # Release build
```

## Architecture

**Workspace** (`crates/`):

| Crate | Type | Responsibility |
|-------|------|----------------|
| `myro-api` | lib | Supabase client — auth (email/GitHub OAuth PKCE), PostgREST CRUD, progress sync, analytics event batching |
| `myro-cf` | lib | Codeforces API client, HTML problem parser, rate limiter, shared types, CF session auth + submission |
| `myro-predict` | lib+bin | CF solve probability prediction (logistic matrix factorization model) |
| `myro-coach` | lib | AI coaching engine — JSON problem loading, LLM integration, prompt templates, intervention detection |
| `myro-tui` | bin (`myro`) | Ratatui TUI — problem browser, vim editor, test runner, recommender integration, CF submission |
| `myro-bench` | bin | Coaching quality benchmark — scenario-based LLM evaluation with scoring rubric |

**Web app** (`apps/web/`): Next.js 15 (App Router) + Tailwind CSS v4 + Prisma + SQLite

**Dependency graph:** `myro-predict` and `myro-tui` both depend on `myro-cf`. `myro-tui` also depends on `myro-predict` (lib) for `ProblemModel`, `fit_user_weighted`, history, and cache, and on `myro-api` for Supabase auth/sync/analytics. `apps/web` is the Next.js frontend. All five are part of the same product.

**Data directories:**
- `test-problem-set/` — JSON files defining decomposed problems (one per file, `cf-{contestId}{index}.json`)
- `prompts/` — Editable markdown templates for LLM prompts (`coaching-system.md`, `coaching-user.md`)
- `bench/scenarios/` — TOML scenario files for coaching benchmark (single-turn and multi-turn)
- `bench/results/` — Benchmark result JSONs (per-model, per-round)

### Web Trainer (apps/web)

The Observation Coach — an LLM-powered coaching system that guides users through algorithmic problem-solving.

**Core model:** Problem → Route (solution approach) → Observations (ordered key insights). The coach detects which observation the user is approaching/has found/is moving away from and provides directional guidance.

**Key files:**
- `src/lib/coach/prompt.ts` — System prompt builder (assembles problem, route, observations, unlock state)
- `src/lib/coach/schema.ts` — Zod schema for structured coach JSON output (state, confidence, matched_observation_id, coach_message)
- `src/lib/llm/provider.ts` — Swappable LLM abstraction (Anthropic Claude Sonnet 4 / OpenAI GPT-4o)
- `src/lib/seed-data.ts` — 10 problems with full observation/hint data
- `prisma/schema.prisma` — SQLite models: User, Problem, Route, Observation, TrainingSession, ChatMessage, UnlockedObservation

**API routes:**
- `POST /api/chat` — Send user message → LLM coaching → auto-unlock observations (confidence > 0.8) → transition to implementing when all unlocked
- `POST /api/hints` — Advance hint ladder (3 levels: nudge → pointed question → partial formalization)
- `GET/POST /api/sessions` — Session CRUD

**Env vars** (in `apps/web/.env`): `DATABASE_URL`, `LLM_PROVIDER` (anthropic|openai), `LLM_API_KEY`

### myro-api: Supabase Client

Handles all Supabase interaction for the TUI. No async — uses `reqwest::blocking::Client`.

**Supabase project:** `yblyfpanzpfmwmupedwx` (`https://yblyfpanzpfmwmupedwx.supabase.co`)

**Key modules:**
- `auth.rs` — Email signup/signin, GitHub OAuth PKCE flow (localhost TcpListener callback), token refresh, persist to `~/.config/myro/auth.json`
- `client.rs` — `SupabaseClient` struct with PostgREST CRUD (`get`, `post`, `patch`, `delete`), RLS-aware headers
- `sync.rs` — Push/pull functions for `solved_problems`, `past_entries`, `skill_snapshots`, `coaching_sessions`, `solutions`, `profiles`
- `events.rs` — `EventBatch` with buffered analytics, auto-flush at 30s/100 events, flush-on-drop
- `types.rs` — Row structs for all Supabase tables

**Two tiers:**
- **BYOK free tier** — no login required. Users set their own LLM endpoint/key in Settings. All data stays local.
- **Login tier** — optional sign-in via Settings → "sign in (sync)". Enables progress sync to Supabase. Future: pro subscription for hosted API.

**Auth flow in TUI:**
- Login is **optional** — accessed from Settings, not a gate. App starts normally without login.
- `AppState::Login` with `LoginPhase`: `ChooseMethod` → `EmailInput` / `OAuthWaiting` → `OAuthSuccess` → back to Settings
- GitHub OAuth opens browser via `xdg-open`, listens on random localhost port for PKCE callback
- Token refresh attempted silently on startup — if valid session exists, sync activates automatically
- `App.api: Option<SupabaseClient>` and `App.events: Option<Arc<EventBatch>>` — `None` when not authenticated
- Sync is best-effort: failures logged but never block UI

**LLM config in Settings:**
- Settings screen has section headers: "ai coach", "codeforces", "account"
- LLM settings editable in-app: `llm endpoint` (coach.base_url), `api key` (coach.api_key, masked display), `model` (coach.model)
- Changes saved to `~/.config/myro/config.toml` `[coach]` section (merged with existing file)
- Env var overrides still work: `MYRO_LLM_BASE_URL`, `MYRO_LLM_API_KEY`, `MYRO_LLM_MODEL`

**Supabase tables:** `profiles`, `solved_problems`, `past_entries`, `skill_snapshots`, `coaching_sessions`, `solutions`, `events` — all with RLS policies scoped to `auth.uid()`. Auto-profile creation via trigger on `auth.users` insert.

### myro-tui: Synchronous Event Loop

The TUI uses a **synchronous** event loop (not async/tokio). Key patterns:
- `App` is a multi-state machine: `HandlePrompt` (first run, CF handle validation) → `Home` (menu) → `Stats` (skill ratings) / `Settings` / `ProblemSelect` / `Solving` (editor + problem)
- `EventReader` polls crossterm events on a background thread at 100ms ticks
- `/` in vim Normal mode enters app-level command mode (intercepted before edtui): `/run`, `/quit`, `/submit`, `/isuck`, `/help`
- Test execution spawns a background thread, results arrive via `mpsc::Receiver` polled in `tick()`
- Coach uses the same background thread + mpsc pattern: `CoachRequest` sent via `request_tx`, `CoachEvent` polled via `event_rx.try_recv()` in `tick()`
- Recommender uses the same background thread + mpsc pattern: `RecommendRequest`/`RecommendEvent` via `recommend.rs`. Handles model loading, user embedding fitting, problem recommendation, CF submission, verdict polling, and history recording.
- Coach commands: `/hint` (context-aware hint via LLM), `/coach` (toggle panel), `/coach <msg>` (direct message with current code context)
- Recommender commands: `/submit` (submit solution to CF, poll verdict), `/isuck` (mark problem as too hard, skip to next)
- Config: `AppConfig` in `config.rs` loads from `~/.config/myro/config.toml` with sections `[codeforces]` (handle, encrypted password) and `[recommender]` (target_probability, model_path)
- Suggested problem flow: Home menu → fetch CF history → fit user embedding on-the-fly via `myro-predict` → pick problem near target P(solve) → fetch statement → open editor. On AC verdict, automatically gets next problem (TikTok flow).
- Debug: `/debug` opens scrollable overlay (j/k scroll, g/G top/bottom, Esc close), `/debug copy` copies full session log to clipboard (OSC 52 → wl-copy/xclip → file fallback)
- Mock mode: `MYRO_COACH_MOCK=1` or `mock = true` in config — canned responses, no network needed
- Thinking indicator: spinner in coach panel while LLM request is in-flight, requests deduplicated (skipped if already thinking)
- `Ctrl+C` requires double-press to quit (first press shows warning)
- Theme colors are centralized in `theme.rs` — use the style helper functions, don't hardcode colors

### Testing myro-tui

See the `testing-myro` skill for full details. Two approaches:

- **TestApp harness** (`cargo test -p myro-tui`): `TestApp::home().build()` constructs `App` + `Terminal<TestBackend>` without filesystem/network. Press keys, tick, render, assert on `CapturedFrame`. Uses `RecommenderState::empty()` for no-IO construction.
- **tmux interactive play**: Run real app via `tmux new-session -d -s myro -x 80 -y 24 'MYRO_COACH_MOCK=1 cargo run -p myro-tui'`, send keys with `tmux send-keys`, capture with `tmux capture-pane -t myro -p`.

Test files: `crates/myro-tui/src/testing.rs` (harness), `crates/myro-tui/tests/smoke.rs`, `crates/myro-tui/tests/solving.rs`.

### myro-coach: AI Coaching Engine

Provides the observation-based coaching model for the TUI. DB-free: loads problems from JSON files, holds state in memory.

**Key modules:**
- `seed.rs` — JSON serde types (`ProblemFile`, `RouteFile`, `ObservationFile`) + `load_problem_set(dir)` loader
- `intervention.rs` — Stall/velocity/rewrite detection engine (runs on main thread)
- `llm/openai_compat.rs` — Single OpenAI-compatible provider (covers OpenRouter, Ollama, llama.cpp); supports reasoning models (`reasoning_content` field); 120s timeout; no connection pooling to avoid stale connection RSTs
- `prompt/coaching.rs` — Template-based prompt builder (loads from `prompts/` or falls back to compiled-in defaults)
- `prompt/schema.rs` — Response parsing with 3-level fallback (direct JSON → code fence → embedded object → fallback)
- `config.rs` — TOML config from `~/.config/myro/config.toml` with env var overrides

**TUI integration** (`crates/myro-tui/src/coach/`):
- `bridge.rs` — Spawns background thread (mock or LLM) with mpsc channels; holds problem data + observation states in memory; builds `CoachingPromptContext` directly without DB; maintains rolling conversation history (last 20 messages) for LLM context; `/hint` sends code + static hint text to LLM for context-aware delivery
- `panel.rs` — 3-line coach panel widget with confidence dot indicator and thinking spinner
- `ghost.rs` — Inline hint overlay, writes directly to ratatui `Buffer` after editor renders
- `mod.rs` — `CoachState` struct, `CoachRequest`/`CoachEvent` enums, `apply_response()` logic, request deduplication via `send_request()`

**User state** (`crates/myro-tui/src/state.rs`): Persisted as `~/.local/share/myro/state.json` — stores user name and solved problem IDs.

**Dependency graph:** `myro-tui -> myro-coach -> myro-cf`

### myro-predict: CLI Pipeline (lib+bin)

Uses clap subcommands for an ML pipeline: `collect` → `train` → `export-model` → `eval` → `query`. Data stored in SQLite (`predict.db`), full trained model serialized to `model.bin.gz` (bincode + gzip), problem-only model to `problem_model.bin.gz`. See [docs/myro-predict.md](docs/myro-predict.md) for full CLI reference.

**Cold-start architecture:** The `ProblemModel` (problem embeddings only, no user params) is the deployment artifact. User embeddings are computed on-the-fly via `fit_user_weighted` — time-weighted SGD against the user's CF submission history. This means any CF user gets accurate predictions without retraining.

**Key lib modules** (importable by other crates):
- `model::types` — `SolvePredictionModel`, `ProblemModel`, `UserParams`, `ProblemParams`, `WeightedObservation`
- `model::inference` — `predict`, `predict_all`, `fit_user_weighted`, `time_decay_weight`, `build_observations_from_submissions`
- `model::eval` — `compute_auc`, `compute_logloss`, `run_temporal_eval`, `per_depth_metrics`
- `model::skills` — `compute_skill_profile`, `compute_skill_deltas`, `TagSkillRating`, `SkillProfile`, `SkillDelta`
- `db::model_store` — `save_model`/`load_model`, `save_problem_model`/`load_problem_model`
- `history` — `SolveHistory` with `record`, `save`/`load`, `content_hash` (JSON at `~/.local/share/myro/history.json`); `SkillHistory`/`SkillSnapshot` (JSON at `~/.local/share/myro/skill_history.json`)
- `cache` — `CachedUserParams` with `save_cached_params`/`load_cached_params` (bincode at `~/.local/share/myro/user_params.bin`)

**Pipeline rerun protocol:** After pulling new CF data (`collect`), or after retraining the model, the full pipeline must be re-run and all docs updated. See `crates/myro-predict/PIPELINE.md` for the step-by-step runbook. Key rule: **any change to the model or data requires updating REPORT.md, DIVERGENCE.md, and re-running the kalimm divergence query.** If the user runs `collect`, ask whether they want to re-run the train/eval pipeline afterward.

### myro-cf: API + Scraping

- `CfClient` wraps reqwest with a 2-second rate limiter and exponential backoff retry (up to 5 attempts)
- `parser` module scrapes CF problem page HTML into `ProblemStatement` structs using the `scraper` crate
- All CF API types use `#[serde(rename_all = "camelCase")]` to match the CF JSON format

### myro-bench: Coaching Quality Benchmark

Evaluates coaching LLM responses against frozen scenarios. 39 scenarios across 14 problems, including 4 true multi-turn scenarios.

- `bench/scenarios/*.toml` — Scenario definitions (single-turn with `[expect]`, multi-turn with `[[turns]]`)
- `bench/results/` — Per-model JSON results, versioned by round (`round0-v2/`, etc.)
- `bench/METHODOLOGY.md` — Scoring rubric, iteration process, scenario format reference, coverage matrix
- `bench/REPORT2.md` — Latest evaluation report with score progression and analysis
- Scoring rubric: valid_parse (2), state_match (2), obs_id_match (2), confidence (1), socratic (1), no_leaks (2) — max varies per scenario (6-10 points)
- Multi-turn uses `build_context_dynamic()` with monotonic observation state tracking (locked→approaching→found) and trajectory scoring (progression + no_contradiction + no_repetition)
- Observation IDs follow format `{problem_id}:route:{n}:obs:{m}` (1-indexed)

## Key Technical Decisions

- **Ratatui + Crossterm** for the TUI (cross-platform terminal backend)
- **edtui** for built-in vim editor with syntax highlighting (base16-ocean-dark theme, `"py"` extension)
- **SQLite (rusqlite)** with `bundled` feature for myro-predict persistence; coach uses flat JSON files
- **Glicko-2** rating system (planned) applied both globally and per-skill, with time-weighted import using exponential decay on historical submissions
- **Hierarchical skill taxonomy** as a DAG (e.g., `dp.bitmask`, `graph.shortest_path.dijkstra`) with ~50+ skills and prerequisite enforcement
- **XDG-compliant paths**: config in `~/.config/myro/`, data in `~/.local/share/myro/`
- **rustls-tls** for reqwest — no OpenSSL dependency. Always use `default-features = false, features = ["json", "rustls-tls"]`
- **AGPL-3.0** license

## edtui Gotchas

- `Lines` (alias for `Jagged<char>`): use `.iter_row()` for row-wise iteration, `.to_string()` for conversion. `.iter()` yields `(Option<&char>, Index2)` tuples — not rows.
- Syntax theme names use hyphens: `"base16-ocean-dark"` not `"base16-ocean.dark"`
- `EditorView` implements `Widget` (not `StatefulWidget`) — use `frame.render_widget(view, area)`
- `/` key in vim Normal mode is intercepted by the app for command input before reaching edtui's search

## Design Documents

Detailed specifications live in the repo root:

- `myro-design.md` — Architecture, data models, module structure, database schema
- `docs/algorithm.md` — Logistic MF algorithm deep-dive: math, derivations, design choices, limitations
- `docs/myro-predict.md` — myro-predict CLI reference: all commands, flags, typical pipeline
- `myro-feature-specs.md` — Feature details: CF IDE integration, AI code coach, stress testing
- `myro-wireframes.md` — TUI screen layouts and navigation flow
- `myro-strategy.md` — Monetization, MVP scope, theming
- `docs/myro-web.md` — Web Trainer (Observation Coach) product spec
- `docs/skill-rating.md` — Per-tag skill rating algorithm, exploratory analysis
- `crates/myro-coach/README.md` — AI Coach architecture, data model, thread model, intervention engine
- `bench/METHODOLOGY.md` — Benchmark scoring rubric, scenario format, iteration loop, coverage matrix

## Gotchas

- **UTF-8 string truncation**: Never slice strings by byte index without checking `is_char_boundary()`. Use a helper like `truncate_at_char_boundary(s, max_bytes)` (see `prompt/schema.rs`). This caused panics in 4 locations during initial development.

## Conventions

- Problem IDs use composite format: `"cf:1800A"` (Codeforces) or `"lc:1"` (LeetCode)
- Solutions are saved to `~/.local/share/myro/solutions/{contestId}{index}.py`
- The test runner shells out to `python3` with a 5-second timeout per test case
- The recommendation engine targets problems at `user_skill_rating + 100 to +300`, identifies weak skills, enforces prerequisite ordering, and uses spaced repetition for reinforcement
