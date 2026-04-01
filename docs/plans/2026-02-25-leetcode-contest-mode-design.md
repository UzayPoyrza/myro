# LeetCode Contest Mode — Implementation Plan

## Context

Myro currently has a minimal 2-state TUI (Home → Solving) with a single hardcoded test problem. The user wants to add LeetCode contest mode: the ability to enter a contest slug (e.g., `weekly-contest-380`) and solve all 4 problems with a timer, tab-based navigation, and LC function-signature-style solutions. This also requires building a new `myro-lc` crate for LeetCode GraphQL integration and cookie-based authentication.

## Scope

- Virtual contests: replay past LC weekly/biweekly contests with timer
- Live contest companion: solve during active contests
- Entry by contest slug
- Cookie-paste auth flow
- LC function signature solutions with auto-generated test driver
- Tab bar (Q1-Q4) + number key navigation
- Countdown timer in status bar
- LC-style scoring (problems solved + total penalty time)

## Architecture

### New crate: `myro-lc`

```
crates/myro-lc/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── client.rs       # LcClient: GraphQL queries, rate limiting, retry
    ├── auth.rs         # Cookie storage/loading (~/.config/myro/lc_session.json)
    ├── types.rs        # LcContest, LcProblem, LcCodeSnippet, LcTestCase
    └── driver.rs       # Generate Python driver script for function-signature execution
```

**Dependencies** (mirrors myro-cf pattern):
- `reqwest` (rustls-tls, json)
- `serde` + `serde_json`
- `anyhow`
- `tokio` (time)
- `dirs` (for config path)

### TUI changes: `myro-tui`

New app state `AppState::Contest` alongside existing `Home` and `Solving`:

```rust
AppState::Contest {
    contest: LcContest,           // contest metadata
    problems: Vec<ContestProblem>, // all 4 problems with per-problem state
    active_idx: usize,            // which problem tab is selected (0-3)
    start_time: Instant,          // when contest started
    duration: Duration,           // 90 minutes
    command_input: Option<String>,
}

struct ContestProblem {
    problem: LcProblem,
    editor_state: EditorState,
    editor_handler: EditorEventHandler,
    results: Option<Vec<TestResult>>,
    running: Option<mpsc::Receiver<Vec<TestResult>>>,
    solution_path: PathBuf,
    solve_time: Option<Duration>,  // when AC'd (from contest start)
    status: ProblemStatus,         // Unsolved, Attempted, Accepted
}
```

### New files

| File | Purpose |
|------|---------|
| `crates/myro-lc/src/lib.rs` | Crate root, re-exports |
| `crates/myro-lc/src/client.rs` | `LcClient` with GraphQL queries |
| `crates/myro-lc/src/auth.rs` | Cookie load/save/validate |
| `crates/myro-lc/src/types.rs` | LC-specific types |
| `crates/myro-lc/src/driver.rs` | Python driver script generation |
| `crates/myro-tui/src/contest.rs` | Contest state management + key handling |
| `crates/myro-tui/src/lc_runner.rs` | LC-specific test runner (wraps driver + solution) |

### Modified files

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `myro-lc` to workspace members |
| `crates/myro-tui/Cargo.toml` | Add `myro-lc` dependency |
| `crates/myro-tui/src/app.rs` | Add `Contest` variant to `AppState`, menu item, dispatch |
| `crates/myro-tui/src/ui.rs` | Add `render_contest()` with tab bar, timer, per-problem view |
| `crates/myro-tui/src/main.rs` | Add CLI arg parsing for `myro contest <slug>` |

---

## Detailed Design

### 1. `myro-lc` — GraphQL Client

**LcClient** follows the same pattern as `CfClient`: reqwest with rate limiter and retry.

Key GraphQL queries:

```graphql
# Fetch contest metadata + problem list
query contestInfo($titleSlug: String!) {
  contest(titleSlug: $titleSlug) {
    title
    titleSlug
    startTime      # unix timestamp
    duration       # seconds
    questions {
      questionId
      title
      titleSlug
      difficulty
      creditPoint   # LC scoring points
    }
  }
}

# Fetch full problem details (called per problem)
query questionData($titleSlug: String!) {
  question(titleSlug: $titleSlug) {
    questionId
    title
    titleSlug
    content          # HTML description
    difficulty
    exampleTestcaseList   # raw input strings
    codeSnippets {
      lang
      langSlug
      code             # function stub
    }
    metaData          # JSON: param names, types, return type
  }
}
```

**Auth**: Session cookie stored at `~/.config/myro/lc_session.json`:
```json
{
  "leetcode_session": "eyJ...",
  "csrf_token": "abc123"
}
```

All GraphQL requests include `Cookie: LEETCODE_SESSION=...; csrftoken=...` and `X-CSRFToken: ...` headers.

### 2. LC Problem Types

```rust
pub struct LcContest {
    pub title: String,
    pub title_slug: String,
    pub start_time: i64,      // unix timestamp
    pub duration_secs: i64,
    pub questions: Vec<LcContestQuestion>,
}

pub struct LcContestQuestion {
    pub question_id: String,
    pub title: String,
    pub title_slug: String,
    pub difficulty: String,
    pub credit: i32,
}

pub struct LcProblem {
    pub question_id: String,
    pub title: String,
    pub title_slug: String,
    pub content_html: String,
    pub difficulty: String,
    pub example_testcases: Vec<String>,   // raw input strings, one per test case
    pub code_snippets: Vec<LcCodeSnippet>,
    pub metadata: LcMetadata,
}

pub struct LcCodeSnippet {
    pub lang: String,
    pub lang_slug: String,
    pub code: String,
}

pub struct LcMetadata {
    pub name: String,           // method name
    pub params: Vec<LcParam>,   // parameter names + types
    pub return_type: String,
}

pub struct LcParam {
    pub name: String,
    pub param_type: String,     // "integer", "integer[]", "string", "TreeNode", etc.
}
```

### 3. Python Driver Script Generation

For each LC problem, generate a `_driver.py` that:
1. Imports the user's solution
2. Reads test input (one JSON value per parameter, one per line)
3. Constructs arguments (with special handling for TreeNode/ListNode)
4. Calls `Solution.{method_name}()`
5. Serializes + prints the result

Example for "Two Sum" (`twoSum(nums: List[int], target: int) -> List[int]`):

```python
import json, sys
sys.path.insert(0, "{solution_dir}")
from solution import Solution

sol = Solution()
nums = json.loads(input())
target = json.loads(input())
result = sol.twoSum(nums, target)
print(json.dumps(result))
```

The driver is generated from `LcMetadata` — each param becomes a `json.loads(input())` call. The output is `json.dumps(result)`.

**Special type handling** (TreeNode, ListNode): Include helper functions in the driver that convert LC's serialization format (`[1,null,2,3]` for trees, `[1,2,3]` for linked lists) to/from Python class instances.

**File layout for LC solutions:**
```
~/.local/share/myro/lc-contests/{contest_slug}/
├── q1_two_sum.py          # user's solution (the code snippet template)
├── q1_driver.py           # auto-generated driver script
├── q2_lru_cache.py
├── q2_driver.py
└── ...
```

The test runner executes `python3 q1_driver.py` (not the solution directly), piping test input to stdin.

### 4. Contest State + Navigation

**Entry flow:**
1. Home menu: new item "Contest mode" (index 1, shifts others down)
2. User enters contest slug in a text input field
3. App spawns a background thread to fetch contest data + all 4 problems
4. Loading screen with spinner while fetching
5. Transitions to `AppState::Contest` once all data is loaded

**Tab bar** (top of screen):
```
  [Q1 Two Sum ✓]  [Q2 LRU Cache ·]  [Q3 Median ✗]  [Q4 Hard Prob ·]    ⏱ 01:23:45
```

- `✓` green = Accepted
- `✗` red = Attempted (has WA)
- `·` dim = Unsolved
- Active tab highlighted with accent color + underline

**Key bindings in Contest state:**
- `1`/`2`/`3`/`4` — Switch to problem tab (only in Normal mode)
- `Tab` / `Shift+Tab` — Cycle through problems
- `/run` — Run tests on current problem
- `/submit` — Mark current problem as solved (records solve time)
- `/quit` — Exit contest (with confirmation)
- All other keys — Pass to editor (same as Solving state)

**Timer:**
- Displayed in status bar: `⏱ 01:23:45 remaining`
- Counts down from `contest.duration` (90 min)
- Updates every tick (100ms) via `app.tick()`
- When timer reaches 0: show "Time's up!" message, freeze editor, show final score

**Scoring:**
- After all problems or time expires, show summary:
  ```
  Contest Complete — 3/4 solved  Total: 42:15
  Q1 ✓ 03:22  |  Q2 ✓ 12:45  |  Q3 ✓ 26:08  |  Q4 ·
  ```

### 5. Auth Command

Add CLI subcommand: `myro auth leetcode`

Flow:
1. Print instructions: "Opening LeetCode login page..."
2. Open `https://leetcode.com/accounts/login/` in default browser (via `open` / `xdg-open`)
3. Prompt: "After logging in, open Developer Tools (F12) → Application → Cookies → leetcode.com"
4. Prompt: "Paste your LEETCODE_SESSION cookie value:"
5. Read input, validate by hitting LC GraphQL with a simple query
6. Save to `~/.config/myro/lc_session.json`

### 6. HTML → Terminal Rendering

LC problem `content` is HTML. Need to convert to terminal-renderable text:
- Strip HTML tags, preserve structure (paragraphs, lists, code blocks)
- Use `html2text` crate or simple custom stripping
- Render `<code>` inline, `<pre>` as indented blocks
- Convert `<strong>`, `<em>` to terminal bold/italic where possible

---

## Implementation Order

### Step 1: `myro-lc` crate skeleton
- Create `crates/myro-lc/` with Cargo.toml, lib.rs, types.rs
- Define all LC types (LcContest, LcProblem, LcCodeSnippet, LcMetadata, etc.)
- Add to workspace

### Step 2: Auth module
- Implement `crates/myro-lc/src/auth.rs` — load/save/validate session cookie
- Add CLI `myro auth leetcode` flow in main.rs (uses clap or simple arg matching)

### Step 3: GraphQL client
- Implement `crates/myro-lc/src/client.rs` — LcClient with contest + problem queries
- Rate limiter (reuse pattern from CfClient or extract shared util)
- Test against a real past contest

### Step 4: Driver script generation
- Implement `crates/myro-lc/src/driver.rs`
- Generate Python driver from LcMetadata
- Include TreeNode/ListNode helpers
- Unit test with known LC problem signatures

### Step 5: Contest state in TUI
- Add `AppState::Contest` to `app.rs`
- Add `contest.rs` module for contest-specific state management and key handling
- Add `lc_runner.rs` — wraps existing runner pattern but uses driver script
- Wire up menu item "Contest mode" → slug input → fetch → contest state

### Step 6: Contest UI rendering
- Add `render_contest()` to `ui.rs`
- Tab bar with problem status indicators
- Timer in status bar
- Per-problem editor + results (reuse existing `render_statement`, `render_results` patterns)
- Contest summary screen

### Step 7: Polish + edge cases
- Handle fetch failures gracefully (show error, offer retry)
- Handle expired sessions (prompt to re-auth)
- Handle contest not found / not yet started
- Timer expiry behavior

---

## Verification

1. **Auth**: `myro auth leetcode` → paste cookie → verify it saves to `~/.config/myro/lc_session.json` and validates against LC
2. **Fetch**: `myro contest weekly-contest-380` → verify all 4 problems load with descriptions, examples, and code snippets
3. **Editor**: Solution template pre-filled with LC function stub, editable with vim keybinds
4. **Test runner**: `/run` executes driver script with example test cases, displays pass/fail correctly
5. **Navigation**: `1-4` keys switch problems, tab bar updates, editor state preserved per problem
6. **Timer**: Counts down correctly, updates in status bar every tick
7. **Scoring**: `/submit` records solve time, contest summary shows correct totals
8. **Build**: `just build` compiles cleanly, `just clippy` passes, `just test` passes
