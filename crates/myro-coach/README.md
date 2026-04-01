# myro-coach

AI coaching engine for competitive programming. Watches you code, detects when
you're stuck, and helps you learn — questions first, more direct help when you need it.

```
 "blitz chess for competitive programming"
  -- 20-min sessions with an AI looking over your shoulder
```

## Architecture Overview

```
                             +-------------------------------------------+
                             |              myro-tui (binary)             |
                             |                                           |
                             |  +----------+  +----------+  +----------+ |
                             |  | app.rs   |  | ui.rs    |  | theme.rs | |
                             |  | tick()   |  | render() |  | colors   | |
                             |  +----+-----+  +----+-----+  +----------+ |
                             |       |             |                      |
                             |  +----v-------------v----+                 |
                             |  |    coach/ module       |                |
                             |  |                        |                |
                             |  | mod.rs    CoachState   |                |
                             |  | bridge.rs spawn thread |                |
                             |  | panel.rs  render panel |                |
                             |  | ghost.rs  render ghost |                |
                             |  +------+-------+---------+                |
                             +---------|-------|--------------------------+
                                       |       |
                             mpsc tx   |       |  mpsc rx
                           (requests)  |       |  (events)
                                       |       |
                             +---------v-------v--------------------------+
                             |          coach background thread           |
                             |                                            |
                             |  +---------+   +----------+   +----------+ |
                             |  | context |-->| LLM call |-->| parse    | |
                             |  | builder |   | (async)  |   | response | |
                             |  +---------+   +----------+   +----------+ |
                             +--------------------------------------------+
                                       |               |
                             +---------v---+   +-------v--------+
                             |  coach.db   |   |   LLM API      |
                             |  (SQLite)   |   |   (OpenAI-     |
                             |  problems   |   |    compatible)  |
                             |  routes     |   +----------------+
                             |  obs        |
                             +-----------  +
```

## Crate Structure

```
myro-coach/
  src/
    lib.rs                  Module root
    types.rs                Core data types (Route, Observation, CoachResponse, ...)
    config.rs               TOML config + env var loading
    intervention.rs         Stall/velocity/rewrite detection engine
    context.rs              Assembles LLM prompt from DB state
    decompose.rs            Batch problem decomposition pipeline
    db/
      mod.rs                open_coach_db(), migrations
      schema.rs             SQLite CREATE TABLE statements
      queries.rs            CRUD for problems, routes, observations, sessions
    llm/
      mod.rs                LlmProvider trait, CompletionRequest
      openai_compat.rs      OpenAI-compatible HTTP client
    prompt/
      mod.rs                Module declarations
      coaching.rs           Real-time coaching system prompt builder
      decomposition.rs      Batch decomposition prompt builder
      schema.rs             JSON response parsing with 3-level fallback
    bin/
      decompose.rs          myro-decompose CLI (import/decompose/stats)

myro-tui/src/coach/
  mod.rs                    CoachState, CoachRequest/Event enums, apply_response()
  bridge.rs                 spawn_coach() -- mock or LLM background thread
  panel.rs                  render_coach_panel() -- ratatui widget
  ghost.rs                  render_ghost_text() -- direct buffer overlay
```

## Data Model

```
  Problem
  "cf:1800A"
    |
    +-- Route 1: "Hash map approach"
    |     |
    |     +-- Observation 1: "Complement lookup"
    |     |     hints: nudge -> question -> formal
    |     |     skill_tag: "ds.hash_map"
    |     |
    |     +-- Observation 2: "Single-pass scan"
    |           hints: nudge -> question -> formal
    |           skill_tag: "technique.two_pointer"
    |
    +-- Route 2: "Sorting approach"
          |
          +-- Observation 1: "Sort first"
          |     hints: nudge -> question -> formal
          |     skill_tag: "greedy.sorting"
          |
          +-- Observation 2: "Two-pointer scan"
                hints: nudge -> question -> formal
                skill_tag: "technique.two_pointer"
```

Each observation has a 3-tier hint ladder:

```
  Level 0: No hint (locked)
  Level 1: Nudge     "Think about what data structure gives O(1) lookup"
  Level 2: Question  "Can you use a hash map to store complements?"
  Level 3: Formal    "For each num, check if (target - num) is in the map"
```

Observations are unlocked automatically when the LLM detects the user has found
the insight (confidence > 0.8), or manually via the `/hint` command.

## Thread Model

The TUI runs a synchronous event loop (no tokio in the main thread). The coach
uses a background thread with mpsc channels, matching the test runner pattern.

```
  Main Thread (sync, 100ms tick)          Background Thread
  ================================        ================================

  tick() {                                loop {
    poll coach events  <------ mpsc ------  recv(CoachRequest)
    record edits                            match request {
    check stall/velocity/rewrite              Analyze => {
    if trigger:                                   build_context(db)
      send Analyze  ------- mpsc ------>          llm.complete(prompt)
    render panel + ghost                          parse_response(json)
  }                                               send CoachEvent
                                                }
  /hint pressed:                              UserMessage => { ... }
    send RequestHint  ----- mpsc ------>      RequestHint => { ... }
                                              Quit => break
  /coach msg pressed:                       }
    send UserMessage  ----- mpsc ------>  }
  ```

The background thread creates its own single-threaded tokio runtime for async
LLM HTTP calls. SQLite is opened in the background thread (not shared across
threads).

## Intervention Engine

The intervention engine runs on the main thread, tracking edit patterns to
decide when to call the LLM.

```
  User Activity                    Tracked State
  ===============                  ===============
  Keystrokes  ------>  record_edit(line_count)
                         |
                         +-->  velocity_samples[]  (rolling 2-min window)
                         +-->  last_edit_at         (for stall detection)
                         +-->  stall_triggered      (reset on edit)

  Every 5s  -------->  update_snapshot(code)
                         |
                         +-->  last_snapshot        (for rewrite detection)
                         +-->  last_snapshot_lines

  Every tick  ------->  check_for_triggers()
                         |
                         +-->  Stall?         idle >= 45s (configurable)
                         +-->  VelocityDrop?  was >5 l/min, now <1 for 30s
                         +-->  MajorRewrite?  >40% lines deleted

  Test failure  ----->  on_test_failure(num, verdict)
  /hint pressed  ---->  on_user_request()
```

### Trigger Types

| Trigger | Condition | Description |
|---------|-----------|-------------|
| `Stall` | idle >= 45s | User stopped typing (configurable threshold) |
| `VelocityDrop` | >5 -> <1 lines/min | Was coding fast, suddenly stopped |
| `MajorRewrite` | >40% lines deleted | User is scrapping their approach |
| `TestFailure` | test returned WA/RE/TLE | External signal from test runner |
| `UserRequested` | `/hint` command | Explicit help request |

All triggers respect a per-session cap (`max_interventions`, default 8).
Stall does not re-trigger until the user edits again.

## LLM Integration

### Provider Abstraction

```
  LlmProvider trait
    fn complete(CompletionRequest) -> Result<String>
         |
         v
  OpenAiCompatibleProvider
    POST /v1/chat/completions
         |
         +-- OpenRouter  (base_url: https://openrouter.ai/api/v1)
         +-- Ollama      (base_url: http://localhost:11434/v1)
         +-- llama.cpp   (base_url: http://localhost:8080/v1)
```

Single implementation covers all OpenAI-compatible APIs. API key is optional
(not needed for local providers like Ollama).

### Response Parsing

LLM output is parsed with a 3-level fallback:

```
  Raw LLM output
    |
    +-- Try: direct JSON parse  --> CoachResponse
    |
    +-- Try: extract from ```json ... ``` code fence  --> CoachResponse
    |
    +-- Try: find { ... } embedded in text  --> CoachResponse
    |
    +-- Fallback: use raw text as coach_message
            state = "uncertain", confidence = 0.0
```

### CoachResponse Schema

```json
{
  "state": "approaching|found|moving_away|uncertain",
  "confidence": 0.85,
  "matched_observation_id": "cf:1800A:route:1:obs:2",
  "coach_message": "You've got the right structure -- what about negative sums?",
  "ghost_text": "if current_sum < 0:",
  "ghost_format": "code|natural",
  "next_action": "Consider when to reset your running sum"
}
```

## TUI Rendering

### Screen Layout

```
  +------------------------------------------------------+
  | Problem Statement (collapsible)                      |  [0]
  |------------------------------------------------------|  [1]
  | Editor (vim mode)                                    |  [2]
  |                                                      |
  |   def two_sum(nums, target):                         |
  |       seen = {}                                      |
  |       for i, num in enumerate(nums):                 |
  |         ~ if complement in seen:          <-- ghost  |
  |                                                      |
  |------------------------------------------------------|  [3]
  | * Coach  2/4 insights                                |  [4]
  |   You're close -- what if you tracked what           |
  |   you've already seen?                               |
  |------------------------------------------------------|  [5]
  | Test Results                                         |  [6]
  |   #1 PASS  #2 PASS  #3 FAIL (WA)                    |
  |------------------------------------------------------|  [7]
  | NORMAL          /run  /hint  /coach  /quit           |  [8]
  +------------------------------------------------------+
```

### Coach Panel (3 lines)

```
  Line 1:  [dot] Coach  X/Y insights
  Line 2:  [message body, word-wrapped]
  Line 3:  [message continuation if needed]

  Confidence dot colors:
    * green   = Observing   (user is in flow)
    * yellow  = Concerned   (may be struggling)
    * orange  = ReadyToHelp (coach has insight)
    * red     = Intervening (actively helping)
```

### Ghost Text

Rendered directly to the ratatui `Buffer` AFTER the editor widget renders.
Does not modify editor state.

```
  Position:   1 line below cursor
  Prefix:     "  ~ "
  Fade-in:    3-tick delay (300ms)
  Dismissed:  any keypress
  Max width:  editor area width - 6
  Truncation: UTF-8 safe with "..." suffix

  Styles:
    Natural:  Color::Rgb(90, 90, 110) + italic    "what if you tracked complements?"
    Code:     Color::Rgb(80, 140, 140)             "if complement in seen:"
```

## Mock Mode

Run without an API key for testing the full coaching UX:

```bash
just mock-tui                          # or:
MYRO_COACH_MOCK=1 cargo run -p myro-tui
```

Mock mode provides:
- 6 cycling analyze responses (approaching, uncertain, found, moving_away)
- Context-aware user message responses (detects "stuck", "slow", "TLE")
- 3 progressive hint responses
- Simulated LLM latency (100-300ms)
- 4 mock observations with ghost text in both code and natural formats

```
  spawn_coach()
    |
    +-- config.mock == true?
    |     |
    |     yes --> spawn_mock_thread()
    |               Uses canned responses, no network
    |
    +-- config.mock == false?
          |
          yes --> spawn_llm_thread()
                    Opens coach.db, creates tokio runtime
                    Makes real HTTP calls to LLM API
```

## SQLite Schema

Database: `~/.local/share/myro/coach.db`

```
  problems -------< routes -------< observations
     |                                    |
     |                                    |
  coaching_sessions                session_observations
     |                                    |
     +--------< coach_messages            |
     +--------< session_observations >----+

  decomposition_jobs >---- problems
```

| Table | Purpose |
|-------|---------|
| `problems` | CF problem metadata (title, tags, difficulty, spec) |
| `routes` | Solution approaches per problem |
| `observations` | Key insights per route, with 3-tier hints |
| `coaching_sessions` | Per-user session tracking |
| `session_observations` | Observation unlock state per session |
| `coach_messages` | Chat history (user + coach messages) |
| `decomposition_jobs` | Batch decomposition progress tracking |

## Configuration

`~/.config/myro/config.toml`:

```toml
[coach]
enabled = true                              # master toggle (default: true)
base_url = "https://openrouter.ai/api/v1"  # OpenAI-compatible endpoint
api_key = "sk-or-..."                       # optional for local providers
model = "anthropic/claude-sonnet-4"         # model ID
stall_threshold_secs = 45                   # idle seconds before intervention
max_interventions = 8                       # per session cap
ghost_text_enabled = true                   # toggle ghost text
```

Environment variable overrides:

| Env Var | Overrides |
|---------|-----------|
| `MYRO_LLM_BASE_URL` | `base_url` |
| `MYRO_LLM_API_KEY` | `api_key` |
| `MYRO_LLM_MODEL` | `model` |
| `MYRO_COACH_MOCK` | sets `mock = true` |

If no `base_url` is configured and `mock` is false, the coach is silently
disabled (no error, just no panel or ghost text).

## Commands

| Command | Action |
|---------|--------|
| `/hint` | Request next hint for current observation |
| `/coach` | Toggle coach panel visibility |
| `/coach <msg>` | Send a direct message to the coach |

## Decomposition Pipeline

Pre-analyze problems into routes and observations using batch LLM calls:

```bash
just decompose import --limit 100          # Import CF problems into coach.db
just decompose decompose --batch-size 10   # Run LLM decomposition
just decompose stats                       # Show progress
```

## Tests

```bash
cargo test -p myro-coach    # 50 tests (db, intervention, prompts, parsing, decompose)
cargo test --workspace      # 60 total (50 coach + 10 predict)
```
