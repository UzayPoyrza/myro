# Coaching Benchmark Methodology

This document describes how the Myro coaching benchmark works, how to use it to systematically improve coaching quality, and how to expand its coverage.

## Recipes

Quick-start command blocks. Copy-paste and go.

### Run full benchmark

```bash
just bench
```

Reads `~/.config/myro/config.toml` for model/endpoint. Results written to `bench/results/`.

### Test a single scenario

```bash
just bench --filter knapsack-greedy
```

### Inspect prompts (dry run, no LLM calls)

```bash
just bench --dry-run
just bench --dry-run --filter theatre-sq-empty   # single scenario
```

### Evaluate a prompt change

```bash
cp bench/results/latest.json bench/results/baseline.json
# edit prompts/coaching-system.md
just bench
diff bench/results/baseline.json bench/results/latest.json
```

### Test a different model

```bash
just bench --model "gpt-4o" --base-url "https://api.openai.com/v1" --api-key "$OPENAI_API_KEY"
```

### Add a new scenario (condensed)

```bash
# 1. Create bench/scenarios/your-scenario.toml (see "Adding a New Scenario" below)
# 2. Validate it parses and runs:
just bench --filter your-scenario
# 3. Calibrate against gold-standard:
just bench --dry-run --filter your-scenario
# Feed the printed prompts to a frontier model. Adjust expectations to match.
```

### Generate report from existing results

```bash
just bench-report
```

Reads `bench/results/*.json`, cross-references with scenario TOMLs, writes `docs/coach-eval-report.md`.

## What We're Testing

The coaching LLM receives a system prompt describing a competitive programming problem, a solution route, and ordered observations (key insights the student must discover). It also receives the student's current code and recent conversation. It must produce a structured JSON response that:

1. Correctly classifies the student's progress (state)
2. Identifies which observation is most relevant (obs ID)
3. Provides appropriately-calibrated help (not too much for the context, not too little)
4. Avoids giving away the complete solution when the student hasn't engaged yet

The benchmark tests whether a given model can do this reliably. It measures structural correctness of coaching responses across a set of frozen scenarios.

## The Coaching Model

```
Problem
  └── Route (one solution approach)
        └── Observation 1  [FOUND / APPROACHING / LOCKED]
        └── Observation 2
        └── Observation 3
        └── ...
```

Each problem has a single route (the intended solution approach) with 2-4 observations ordered from most fundamental to most specific. Observations have three states:

- **FOUND**: The student's code demonstrates this insight.
- **APPROACHING**: The student shows partial understanding.
- **LOCKED**: The student hasn't engaged with this insight yet.

The coach's job is to guide the student from one observation to the next, providing calibrated help along the way.

## Scenario Format

Scenarios are TOML files in `bench/scenarios/`. Each defines a frozen moment in a coaching session.

```toml
[scenario]
name = "Knapsack — greedy value/weight ratio (wrong for 0/1)"
problem = "cf-730J"              # references test-problem-set/cf-730J.json
route = 0                        # index into the problem's routes array
trigger = "user idle for 45s"    # what caused the coaching intervention
elapsed_secs = 120               # seconds since session started
observations_found = 0           # display counter for the progress line

[scenario.obs_states]
# Override observation states (0-indexed). Unspecified = "locked".
# 0 = "found"
# 1 = "approaching"

[scenario.code]
text = """
n, W = map(int, input().split())
items = []
for _ in range(n):
    w, v = map(int, input().split())
    items.append((w, v))
items.sort(key=lambda x: x[1]/x[0], reverse=True)
total_w, total_v = 0, 0
for w, v in items:
    if total_w + w <= W:
        total_w += w
        total_v += v
print(total_v)
"""

[[scenario.recent_messages]]
role = "user"
content = "I think sorting by ratio should give the optimal solution"

[expect]
state = "moving_away"                         # exact expected state
# OR: valid_states = ["approaching", "moving_away"]   # multiple acceptable
# OR: banned_states = ["found"]                       # anything except these
obs_id = "cf:730J:route:1:obs:1"              # expected matched observation ID
min_confidence = 0.5                          # confidence floor
max_confidence = 1.0                          # confidence ceiling (default 1.0)
must_be_socratic = true                       # coach_message must contain "?"
banned_patterns = ["dp[i][w]", "max(dp"]      # answer-leaking keywords
```

**Note on `observations_found` vs `obs_states`:** These are independent fields. `observations_found` is a display counter injected into the system prompt's progress line ("Student has found N of M observations"). `obs_states` controls the actual state markers (`[FOUND]`, `[APPROACHING]`, `[LOCKED]`) shown next to each observation in the prompt. Set both to match: if `obs_states` marks obs 0 as `"found"`, set `observations_found = 1`.

### State Expectation Modes

The `[expect]` section supports three ways to specify acceptable states, checked in order:

1. **Exact match** (`state = "approaching"`): Only this state passes.
2. **Allowlist** (`valid_states = ["approaching", "moving_away"]`): Any listed state passes. Use when frontier models disagree on classification.
3. **Blocklist** (`banned_states = ["found"]`): Any state NOT in the list passes. Use when the scenario constrains what the state should NOT be.

If none of the three are specified, any state passes the check.

## Scenario Coverage Matrix

39 scenarios across 14 problems. This matrix shows current coverage and remaining gaps.

| Axis | Current Coverage | Gaps |
|------|-----------------|------|
| **Difficulty** | 800, 1000, 1200, 1300, 1400, 1500, 1600, 1900, 2200 | No 1100, 1700, 2000+ besides 2200 |
| **States tested** | approaching, moving_away, uncertain, found | No scenario expects "uncertain" as exact match |
| **Trigger types** | idle 20-90s, test failure, velocity drop, major rewrite, syntax error | Good coverage |
| **Conversation depth** | 0-4 messages + 4 true multi-turn scenarios (2-3 turns) | No 4+ turn scenarios |
| **Code quality** | Clean, messy names, missing output, syntax errors | No dead code / commented-out attempts |
| **Observation progress** | 0-3 found, all found | Good coverage |
| **Ghost text** | All expect null (non-null tested via answer leak detection) | No non-null ghost_text expected scenarios |
| **User behavior** | Neutral, frustrated, overconfident | No silent/verbose user archetypes |
| **Problems used** | 14 of 120+ available | Expanding to more problems is straightforward |

### Scenarios by Problem

| Problem | Difficulty | Scenarios | Focus |
|---------|-----------|-----------|-------|
| cf-1A (Theatre Square) | 1000 | 4 (empty, bruteforce, correct, float) | Math/ceiling division |
| cf-4A (Watermelon) | 800 | 4 (complete, only-even, frustrated, mt-stuck) | Edge cases, user personality |
| cf-158A (Next Round) | 800 | 1 (correct) | Found state |
| cf-339D (Xenia) | 2200 | 5 (only-xor, no-segtree, wrong-parity, mt-fixing-parity, multiturn-found) | Segment tree |
| cf-340D (LIS) | 1500 | 4 (n-squared, misleading-names, multiturn-redirect, mt-regression) | DP, O(n log n) |
| cf-552D (Triangles) | 1900 | 1 (cross-product-correct) | Found state, geometry |
| cf-730J (Knapsack) | 1700 | 2 (greedy, wrong-recurrence) | DP |
| cf-760B (Aggressive Cows) | 1900 | 2 (fixed-dist, bad-check) | Binary search |
| cf-996B (Coin Change) | 1400 | 5 (greedy, correct-dp, syntax-error, multiturn-approaching, mt-journey) | Greedy vs DP boundary |
| cf-1015B (Obtain the String) | 1200 | 3 (nested-loop, velocity-drop, overconfident) | Two pointers |
| cf-1195C (Sliding Window) | 1600 | 2 (bruteforce, values-not-indices) | Monotone deque |
| cf-1292B (Counting Rooms) | 1300 | 4 (recursive-dfs, found-first-obs, major-rewrite, mt-unlock) | BFS/DFS |
| cf-1800D (Remove Letters) | 1200 | 2 (set-approach, multiturn-found) | Set/string |
| cf-2040C (Ordered Perms) | 1600 | 1 (brute-force) | Higher difficulty |

## Prompt Pipeline

The bench uses the same prompt pipeline as the production TUI coach. No special bench-only prompts.

### System Prompt Assembly

Template: `prompts/coaching-system.md`

```
Template variables filled by coaching.rs:
  {{user_name}}            → "Bench User"
  {{problem_title}}        → from problem JSON
  {{problem_difficulty}}   → from problem JSON
  {{problem_description}}  → from problem JSON, truncated to 1500 bytes
  {{route_name}}           → from problem JSON routes[scenario.route]
  {{route_description}}    → from problem JSON routes[scenario.route]
  {{observations}}         → formatted observation list (see below)
  {{elapsed_secs}}         → from scenario
  {{observations_found}}   → from scenario
  {{observations_total}}   → count of observations in the route
```

Observations are formatted as:

```
- [FOUND] `cf:730J:route:1:obs:1`: Take-or-skip recurrence
  What it means: For each item, you either take it or skip it. dp[i][w] = max(...)
- [LOCKED] `cf:730J:route:1:obs:2`: Base case: no items, no value
  What it means: dp[0][w] = 0 for all w.
```

Observation IDs follow the format `{problem_id}:route:{n}:obs:{m}` where n and m are 1-indexed. These are generated by `scenario.rs:build_context()` from the problem's contest ID, index, route number, and observation position.

### User Message Assembly

Template: `prompts/coaching-user.md`

```
Trigger: {{trigger}}

{{recent_messages}}

Current code:
\`\`\`
{{code}}
\`\`\`

Respond with a single JSON object. Use the exact observation IDs from the system prompt.
```

Code is truncated to 3000 characters by lines if necessary.

### LLM Call

The assembled prompts are sent to an OpenAI-compatible API endpoint:

| Parameter | Value | Notes |
|-----------|-------|-------|
| `max_tokens` | 2048 | Generous ceiling; typical responses are ~200 tokens |
| `temperature` | 0.6 | Tuned during evaluation; 0.7 baseline, 0.4 caused regressions |
| Provider | `OpenAiCompatibleProvider` | From myro-coach; supports OpenRouter, llama.cpp, Ollama |
| Timeout | 120s | Inherited from myro-coach provider |

Model and endpoint are configured via `~/.config/myro/config.toml` or `--model`/`--base-url` CLI flags.

## Response Parsing

The raw LLM response is parsed with a 3-level fallback (from `myro-coach/src/prompt/schema.rs`):

1. **Direct JSON parse**: Try `serde_json::from_str` on the trimmed response.
2. **Code fence extraction**: Find content between `` ```json `` / `` ``` `` markers, parse that.
3. **Embedded object extraction**: Find the first `{`, track brace depth to find the matching `}`, parse the substring.
4. **Fallback**: If all three fail, create a default `CoachResponse` with `state = "uncertain"` and the raw text as `coach_message`.

The scorer separately checks whether the raw response contained parseable JSON (step 1-3 succeeding = valid parse). The fallback response (step 4) is still scored on all other checks but loses the valid_parse points.

## Scoring Rubric

Each scenario is scored independently. Not all checks apply to every scenario — the max score varies.

### Check 1: Valid Parse (2 points)

**Condition:** The raw response contains JSON with at least `state` and `coach_message` fields, extractable by any of the 3 parsing methods.

**Rationale:** The coach must produce structured output that the TUI can render. All three parsing methods (direct, code-fenced, embedded) award full points — the production parser handles all of them identically.

### Check 2: State Match (2 points)

**Condition:** The response's `state` field satisfies the scenario's expectation (exact match, in allowlist, or not in blocklist).

**Valid states:** `approaching`, `found`, `moving_away`, `uncertain`.

**Rationale:** State classification drives the TUI's coaching behavior — `moving_away` triggers redirection, `found` advances the observation tracker. Wrong state = wrong coaching action.

### Check 3: Confidence Range (1 point)

**Condition:** `confidence` is between `min_confidence` (default 0.0) and `max_confidence` (default 1.0).

**Rationale:** Confidence drives the observation auto-unlock threshold (>0.8 in the web trainer). Miscalibrated confidence causes premature or missed unlocks. Most scenarios only set a floor to verify the model isn't defaulting to zero.

### Check 4: Observation ID Match (2 points, conditional)

**Condition:** `matched_observation_id` exactly equals the expected ID. Only scored when `obs_id` is set in the scenario.

**Rationale:** The observation ID links the coach's assessment to a specific insight in the problem's observation chain. Wrong ID = the coach is talking about the wrong thing. Exact string match is required because the TUI uses it as a database key.

### Check 5: Socratic Check (1 point, conditional)

**Condition:** `coach_message` contains at least one `?` character. Only scored when `must_be_socratic = true` (the default).

**Rationale:** Questions are the primary coaching tool. A response without any question is unusual, though not always wrong (e.g., acknowledging a correct insight). The check is intentionally simple (presence of `?`) rather than checking question quality, which would require a second LLM call.

### Check 6: Answer Leak Detection (2 points, conditional)

**Condition:** None of the `banned_patterns` appear (case-insensitive) in `coach_message` or `ghost_text`. Only scored when `banned_patterns` is non-empty.

**Rationale:** The coach shouldn't give away complete solution details in early-stage scenarios where the student hasn't engaged with the observation yet. Banned patterns are chosen per-scenario to match inappropriate reveals for that context. Each scenario defines patterns that would constitute a leak — formulas (`(n + a - 1) // a`), implementation details (`dp[i][w]`), or domain terms the student hasn't mentioned (`alternating`). Matching is case-insensitive substring search on both text output fields.

### Score Calculation

```
score = sum of passed checks
max_score = sum of applicable checks (varies per scenario, 6-10)
```

Scenarios without `obs_id`, `must_be_socratic = false`, or empty `banned_patterns` have lower max scores. The overall benchmark score is `sum(scores) / sum(max_scores)`.

## Gold-Standard Calibration

Scenario expectations were calibrated against gold-standard responses from Claude Sonnet 4.6 and Claude Opus 4.6. Each gold-standard model received the same system prompt and user message as the model under test and roleplayed as the ideal coaching LLM.

**Process:**

1. Run `just bench --dry-run` to extract all prompt pairs.
2. Feed each prompt pair to Sonnet and Opus subagents.
3. Compare state, observation ID, and coaching message across all three models.
4. Where frontier models disagree (e.g. `approaching` vs `moving_away`), loosen the expectation to `valid_states` covering both.
5. Where frontier models unanimously prefer a different obs_id than the original expectation, update the expected obs_id.

This calibration was performed once during the initial evaluation. It should be repeated when adding new scenarios or after significant prompt changes.

## The Iteration Loop

The benchmark exists to drive systematic improvement. Here's the cycle:

### The Cycle

```
1. Run bench → identify failures
2. Classify each failure (see below)
3. Apply ONE change
4. Re-run bench → compare
5. Improved? Commit. Regressed? Revert.
6. Repeat.
```

**One variable at a time.** If you change the prompt and the model and the temperature simultaneously, you can't attribute improvement or regression to any single change.

**Version results between rounds.** Copy `bench/results/latest.json` to `bench/results/roundN.json` before each change so you can track the trajectory.

### Classifying Failures

When a scenario fails, ask: *what went wrong and where does the fix belong?*

| Failure pattern | Likely cause | Where to fix |
|----------------|-------------|-------------|
| Wrong state on most scenarios | Prompt doesn't anchor states clearly | `prompts/coaching-system.md` |
| Wrong state on one scenario | Ambiguous scenario or borderline case | The scenario's `[expect]` section |
| Wrong obs_id | Observation descriptions too similar, or prompt doesn't emphasize ID matching | Observation text in problem JSON, or prompt |
| Answer leak | Coach giving too much for the context (e.g., full solution on idle trigger) | `prompts/coaching-system.md` |
| Parse failure | Model doesn't follow JSON format | Prompt format instructions, or try a more capable model |
| Confidence miscalibrated | Prompt doesn't explain the confidence scale | `prompts/coaching-system.md` |
| Consistent failures on a model | Model capability limit | Try a different model or adjust temperature |

### What to Iterate On

The bench isn't just for prompt tuning. Everything that feeds into the LLM call is a variable:

**Prompts**
- The system prompt template (`prompts/coaching-system.md`)
- The user message template (`prompts/coaching-user.md`)
- Phrasing of state definitions and examples
- How much of the observation "what it means" text is shown (full formulas? redacted?)

**Observation Data**
- The observation descriptions in problem JSONs (too vague? too specific?)
- Number of observations per route (too many = confusing, too few = coarse)
- Ordering of observations (does the model latch onto the first one?)

**Model Parameters**
- Temperature (0.4-0.8 range tested; 0.6 current)
- Max tokens (lowering can force conciseness)
- Model choice (capability vs. cost vs. latency)

**Scoring**
- Add new checks (e.g., message length, question quality proxy)
- Adjust weights (state match could be worth 3 instead of 2)
- Add partial credit (state in `valid_states` but not exact = 1/2 points)

**Scenarios**
- Calibrate expectations against gold-standard when ambiguous
- Add scenarios for gaps identified in the coverage matrix
- Remove or rewrite scenarios that produce noisy results

### Iteration Mindset

Iterate on scenarios as aggressively as on prompts. Bad scenarios hide real failures and flag false ones. If a scenario consistently produces borderline results across multiple good models, the scenario's expectation is probably miscalibrated — fix the scenario, not the prompt.

## Expanding Coverage

The coverage matrix above identifies remaining gaps. Items marked ✅ were addressed in the Round 2 expansion.

### 1. Multi-Turn Scenarios ✅

**Status:** 4 pre-filled multi-turn scenarios (via `recent_messages`) and 4 true multi-turn scenarios (via `[[turns]]`) now exist. True multi-turn uses `build_context_dynamic()` with rolling conversation history and monotonic observation state tracking.

**Remaining gap:** Scenarios are limited to 2-3 turns. Longer sessions (5+) are not tested.

**Multi-turn scenario format:**
```toml
[scenario]
name = "Problem — multi-turn journey"
problem = "cf-XXXX"
route = 0

[[turns]]
trigger = "test failure on test 1"
elapsed_secs = 120
[turns.code]
text = """student code here"""
[turns.expect]
state = "moving_away"
obs_id = "cf:XXXX:route:1:obs:1"
banned_patterns = ["dp["]

[[turns]]
trigger = "user idle for 30s"
elapsed_secs = 300
user_message = "I think I need a different approach"
[turns.code]
text = """updated student code"""
[turns.expect]
state = "approaching"
```

**Trajectory scoring** (5 points): progression (2pts — found count non-decreasing), no_contradiction (2pts — no found→moving_away), no_repetition (1pt — word Jaccard < 0.8).

### 2. "Found" State Scenarios ✅

**Status:** 3 scenarios now expect `state = "found"`: `next-round-correct`, `watermelon-complete`, `coin-change-correct-dp`. Also tested in multi-turn turn 3 of `mt-coin-change-journey`.

### 3. Harder Code Inputs ✅ (partial)

**Status:** Added `lis-misleading-names` (misleading "dp" variable in greedy code) and `coin-change-syntax-error` (syntax error in code).

**Remaining gaps:**
- Dead code / commented-out attempts
- Off-by-one errors
- Right idea, wrong implementation (partially covered by `knapsack-wrong-recurrence`)

### 4. User Personality Archetypes ✅ (partial)

**Status:** Added `watermelon-frustrated-student` (frustrated) and `obtain-string-overconfident` (overconfident).

**Remaining gaps:**
- Silent user (code-only, no messages)
- Verbose/rambling user

### 5. State Progression Scenarios ✅

**Status:** True multi-turn scenarios test full progression chains:
- **Happy path**: `mt-coin-change-journey` (moving_away → approaching → found)
- **Regression**: `mt-lis-regression` (approaching → abandons for sorting)
- **Stuck**: `mt-watermelon-stuck` (same wrong code, coach escalates)
- **Progressive unlock**: `mt-counting-rooms-unlock` (3-turn observation chain)

### 6. Trigger Variety ✅

**Status:** Added `obtain-string-velocity-drop` (velocity drop), `counting-rooms-major-rewrite` (major rewrite), `coin-change-syntax-error` (syntax error).

### 7. Problem Expansion ✅

**Status:** Expanded from 5 to 14 problems. All high-priority problems from the original roadmap are now covered:

| Problem | Difficulty | Status |
|---------|-----------|--------|
| cf-996B (Coin Change) | 1400 | ✅ 5 scenarios |
| cf-1015B (Obtain the String) | 1200 | ✅ 3 scenarios |
| cf-1292B (Counting Rooms) | 1300 | ✅ 4 scenarios |
| cf-340D (LIS) | 1500 | ✅ 4 scenarios |
| cf-4A (Watermelon) | 800 | ✅ 4 scenarios |
| cf-158A (Next Round) | 800 | ✅ 1 scenario |
| cf-552D (Triangles) | 1900 | ✅ 1 scenario |
| cf-1800D (Remove Letters) | 1200 | ✅ 2 scenarios |
| cf-2040C (Ordered Perms) | 1600 | ✅ 1 scenario |

## Adding a New Scenario

1. **Pick a problem** from `test-problem-set/`. Note its ID (e.g. `cf-1A`) and route index.

2. **Decide the student state**: What code has the student written? What bug or approach are they exhibiting?

3. **Write the scenario TOML** in `bench/scenarios/`:

```toml
[scenario]
name = "Problem Name — what the student is doing wrong"
problem = "cf-XXXX"
route = 0
trigger = "user idle for 45s"    # or "test failure on test N"
elapsed_secs = 180
observations_found = 0           # how many obs are already FOUND

[scenario.obs_states]
# Set any non-locked observation states
# 0 = "found"

[scenario.code]
text = """
# Student's Python code here
"""

[[scenario.recent_messages]]     # Optional conversation history
role = "user"
content = "Student's message"

[expect]
state = "approaching"            # or valid_states/banned_states
obs_id = "cf:XXXX:route:1:obs:2" # expected observation match
min_confidence = 0.4
must_be_socratic = true
banned_patterns = ["solution_formula", "specific_technique"]
```

4. **Determine the observation ID format**: IDs are generated as `{problem_id}:route:{route+1}:obs:{obs+1}`. For problem `cf-1A`, route 0, observation 0: `cf:1A:route:1:obs:1`.

5. **Run and validate**:

```bash
just bench --filter your-new-scenario
```

6. **Calibrate against gold-standard** (recommended): Run `--dry-run`, feed the prompts to a frontier model, and check whether your expected state/obs_id match.

### Choosing Banned Patterns

Banned patterns should be strings that would constitute an answer leak if they appeared in the coach's message. Guidelines:

- Include the **literal formula** from the observation description (e.g. `(n + a - 1) // a`)
- Include **code fragments** that implement the insight (e.g. `dp[i][w]`, `tree[2*i]`)
- Include **domain terms** the student hasn't used that would reveal the technique (e.g. `alternating` when the student hasn't mentioned operation alternation)
- Keep patterns short and specific to avoid false positives
- Use lowercase for case-insensitive matching

### Choosing State Expectations

Ask: "if a frontier model saw this code and context, what state would it pick?"

- Use `state = "X"` when the classification is unambiguous (brute force on a problem requiring O(1) = clearly `moving_away`)
- Use `valid_states = ["X", "Y"]` when the classification is genuinely borderline (right concept with a bug = could be `approaching` or `moving_away`)
- Use `banned_states = ["X"]` when you only know what's wrong (correct code should never be `moving_away`)

## Limitations

The benchmark measures structural correctness of the coaching response, not coaching quality per se. Specific limitations:

- **Keyword-based leak detection**: Can miss semantic leaks (describing a pattern without naming it) and can false-positive on innocent usage of common words.
- **Binary scoring**: A slightly wrong state loses the same 2 points as a completely wrong state. No partial credit.
- **No ghost_text quality testing**: All scenarios expect null ghost_text. We test for answer leaks but never test the quality of *appropriate* inline hints.
- **Sampling variance**: LLM outputs are stochastic. Running the same benchmark twice can produce different scores, especially for borderline state classifications. Temperature 0.6 was chosen to balance determinism with naturalness. Single-run comparisons have ~5% noise.
- **No silent/verbose user archetypes**: Coverage includes frustrated and overconfident users, but not silent (code-only) or verbose (rambling) users.
- **Multi-turn limited to 3 turns**: True multi-turn scenarios have 2-3 turns. Longer coaching sessions (5+ turns) are not tested.
- **14 of 120+ problems used**: Coverage is broad (800-2200 difficulty, 9 difficulty levels) but many available problems remain unused.
