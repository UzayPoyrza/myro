# Myro — Design Document

> A terminal-native competitive programming trainer that adapts to you.
> Rust + Ratatui | Aggregates LeetCode + Codeforces | Adaptive Engine

---

## 1. Vision

Myro is a TUI application that makes competitive programming training **efficient and measurable**. It pulls problems from LeetCode and Codeforces, tracks your performance across fine-grained skill topics, and uses an adaptive engine to always serve you the *right* problem at the *right* difficulty. Think of it as a personal CP coach that lives in your terminal.

### Why a TUI?
- CP practitioners already live in terminals — zero context switching
- Keyboard-driven speed matches the competitive mindset
- No browser bloat, no distractions, no loading spinners
- Offline-first with sync — solve anywhere
- The aesthetic *is* the brand

### Core Principles
1. **Efficiency over volume** — 20 targeted problems beat 200 random ones
2. **Measurable growth** — rating, skill graphs, and trend lines you can trust
3. **Zero friction** — problem → editor → submit → results in seconds
4. **Honest feedback** — no hand-holding, real difficulty ratings, no inflated stats

---

## 2. Target Users

| Persona | Goal | How Myro Helps |
|---|---|---|
| **Contest grinder** | Improve CF rating from 1400→1900 | Adaptive drills on weak topics at +100–200 above current level |
| **Interview prepper** | Pass FAANG-tier interviews | Structured paths with LC-style problems, timed mock sessions |
| **CS student** | Learn DSA properly | Skill tree progression from basics to advanced topics |
| **Returner** | Rust off after a break | Spaced repetition surfaces forgotten topics |

---

## 3. Architecture

### 3.1 High-Level Overview

```
┌─────────────────────────────────────────────────────┐
│                   Myro TUI                      │
│              (Rust + Ratatui + Crossterm)             │
├──────────┬──────────┬───────────┬───────────────────┤
│  Views   │  State   │  Engine   │  Data Layer       │
│          │ Manager  │           │                   │
│ Problems │          │ Adaptive  │ SQLite (local)    │
│ Train    │ App-wide │ Recomm.   │ Problem cache     │
│ Contest  │ state    │ Engine    │ User stats        │
│ Stats    │ machine  │           │ Skill graph       │
│ Profile  │          │ Rating    │                   │
│          │          │ Calculator│ API Clients       │
│          │          │           │ ├─ Codeforces API │
│          │          │ Skill     │ ├─ LC (unofficial)│
│          │          │ Graph     │ └─ Judge (local)  │
└──────────┴──────────┴───────────┴───────────────────┘
```

### 3.2 Crate / Module Structure

```
myro/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point, arg parsing (clap)
│   ├── app.rs                  # App state machine, event loop
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── layout.rs           # Main layout frames
│   │   ├── problems.rs         # Problem list & detail view
│   │   ├── train.rs            # Training session view
│   │   ├── contest.rs          # Virtual contest view
│   │   ├── stats.rs            # Stats dashboard, charts
│   │   ├── profile.rs          # Profile & settings
│   │   └── components/
│   │       ├── table.rs        # Reusable sortable table
│   │       ├── chart.rs        # Sparklines, bar charts
│   │       ├── progress.rs     # Progress bars, gauges
│   │       ├── modal.rs        # Popup dialogs
│   │       └── tabs.rs         # Tab navigation
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── adaptive.rs         # Problem recommendation engine
│   │   ├── rating.rs           # Elo/Glicko-2 rating system
│   │   ├── skill_graph.rs      # Topic dependency graph
│   │   └── scheduler.rs        # Session planner
│   ├── api/
│   │   ├── mod.rs
│   │   ├── codeforces.rs       # CF API client
│   │   ├── leetcode.rs         # LC GraphQL client
│   │   └── judge.rs            # Local test runner / submission
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs           # SQLite schema & migrations
│   │   ├── problems.rs         # Problem CRUD
│   │   └── stats.rs            # Stats queries
│   ├── models/
│   │   ├── mod.rs
│   │   ├── problem.rs          # Problem, TestCase, Tag
│   │   ├── submission.rs       # Submission, Verdict
│   │   ├── user.rs             # UserProfile, Rating
│   │   └── skill.rs            # SkillNode, SkillLevel
│   └── config.rs               # Config file parsing (TOML)
├── migrations/
│   └── 001_init.sql
├── config/
│   └── default.toml
└── tests/
    ├── engine_tests.rs
    └── api_tests.rs
```

### 3.3 Key Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` | TUI rendering framework |
| `crossterm` | Terminal backend (cross-platform) |
| `tokio` | Async runtime for API calls |
| `reqwest` | HTTP client for CF/LC APIs |
| `rusqlite` | Local SQLite database |
| `serde` / `serde_json` | Serialization |
| `clap` | CLI argument parsing |
| `toml` | Config file parsing |
| `chrono` | Timestamps, durations |
| `directories` | XDG-compliant config/data paths |

---

## 4. Data Model

### 4.1 Core Entities

```sql
-- Problems from LC and CF, normalized into one schema
CREATE TABLE problems (
    id              TEXT PRIMARY KEY,        -- "cf:1800A" or "lc:1"
    source          TEXT NOT NULL,           -- "codeforces" | "leetcode"
    source_id       TEXT NOT NULL,           -- Original ID on platform
    title           TEXT NOT NULL,
    difficulty       INTEGER,                -- Unified 800–3500 scale
    description     TEXT,                    -- Markdown/plain text
    examples        TEXT,                    -- JSON array of input/output pairs
    constraints     TEXT,
    tags            TEXT NOT NULL,           -- JSON array of fine-grained tags
    url             TEXT,                    -- Link to original problem
    time_limit_ms   INTEGER DEFAULT 2000,
    memory_limit_mb INTEGER DEFAULT 256,
    fetched_at      TEXT NOT NULL,           -- ISO timestamp
    editorial_url   TEXT
);

-- Fine-grained skill taxonomy
CREATE TABLE skills (
    id              TEXT PRIMARY KEY,        -- "dp.bitmask", "graph.shortest_path.dijkstra"
    name            TEXT NOT NULL,           -- "Bitmask DP"
    category        TEXT NOT NULL,           -- "dp", "graph", "math", etc.
    parent_id       TEXT,                    -- For skill tree hierarchy
    description     TEXT,
    prerequisites   TEXT,                    -- JSON array of skill IDs
    FOREIGN KEY (parent_id) REFERENCES skills(id)
);

-- Maps problems to skills (many-to-many)
CREATE TABLE problem_skills (
    problem_id      TEXT NOT NULL,
    skill_id        TEXT NOT NULL,
    relevance       REAL DEFAULT 1.0,       -- How central this skill is (0–1)
    PRIMARY KEY (problem_id, skill_id),
    FOREIGN KEY (problem_id) REFERENCES problems(id),
    FOREIGN KEY (skill_id) REFERENCES skills(id)
);

-- Every submission attempt
CREATE TABLE submissions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    problem_id      TEXT NOT NULL,
    submitted_at    TEXT NOT NULL,
    verdict         TEXT NOT NULL,           -- "AC", "WA", "TLE", "MLE", "RE", "CE"
    language        TEXT NOT NULL,
    time_ms         INTEGER,                -- Solve time (thinking + coding)
    used_hint       INTEGER DEFAULT 0,      -- 0 = no, 1+ = hint level used
    code_path       TEXT,                    -- Path to saved solution file
    FOREIGN KEY (problem_id) REFERENCES problems(id)
);

-- User's rating over time
CREATE TABLE rating_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp       TEXT NOT NULL,
    rating          REAL NOT NULL,           -- Glicko-2 rating
    deviation       REAL NOT NULL,           -- Rating deviation (uncertainty)
    volatility      REAL NOT NULL,
    event_type      TEXT NOT NULL,           -- "problem", "contest", "session"
    event_id        TEXT                     -- Reference to what caused the change
);

-- Per-skill proficiency tracking
CREATE TABLE skill_levels (
    skill_id        TEXT PRIMARY KEY,
    rating          REAL DEFAULT 1000.0,     -- Skill-specific rating
    problems_seen   INTEGER DEFAULT 0,
    problems_solved INTEGER DEFAULT 0,
    avg_solve_time  REAL,                    -- Average seconds
    last_practiced  TEXT,
    streak          INTEGER DEFAULT 0,
    FOREIGN KEY (skill_id) REFERENCES skills(id)
);

-- Training sessions
CREATE TABLE sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at      TEXT NOT NULL,
    ended_at        TEXT,
    session_type    TEXT NOT NULL,           -- "adaptive", "contest", "path", "review"
    problems_attempted INTEGER DEFAULT 0,
    problems_solved    INTEGER DEFAULT 0,
    rating_before   REAL,
    rating_after    REAL
);
```

### 4.2 Skill Taxonomy (Excerpt)

The skill graph is hierarchical with prerequisites, enabling the adaptive engine to understand *what to teach next*.

```
algorithms
├── sorting
│   ├── sorting.comparison        (merge sort, quicksort)
│   └── sorting.non_comparison    (counting, radix)
├── searching
│   ├── searching.binary_search
│   └── searching.ternary_search
├── dp
│   ├── dp.linear
│   ├── dp.knapsack
│   ├── dp.interval
│   ├── dp.bitmask               (prereq: dp.knapsack, bitwise)
│   ├── dp.digit                 (prereq: dp.linear, math.digits)
│   ├── dp.trees                 (prereq: dp.linear, graph.trees)
│   └── dp.probability           (prereq: dp.linear, math.probability)
├── graph
│   ├── graph.traversal          (BFS, DFS)
│   ├── graph.shortest_path
│   │   ├── graph.shortest_path.dijkstra
│   │   ├── graph.shortest_path.bellman_ford
│   │   └── graph.shortest_path.floyd_warshall
│   ├── graph.trees
│   │   ├── graph.trees.lca
│   │   └── graph.trees.hld
│   ├── graph.mst
│   ├── graph.flow
│   └── graph.matching
├── strings
│   ├── strings.hashing
│   ├── strings.kmp
│   ├── strings.trie
│   └── strings.suffix_array
├── math
│   ├── math.number_theory
│   ├── math.combinatorics
│   └── math.probability
└── data_structures
    ├── ds.segment_tree
    ├── ds.fenwick
    ├── ds.sparse_table
    ├── ds.dsu
    └── ds.balanced_bst
```

---

## 5. Core Features — v1

### 5.1 Adaptive Problem Engine

This is the heart of Myro. The engine answers: *"What problem should I solve next to improve the most?"*

**Algorithm:**

```
fn recommend_next(user: &UserProfile, n: usize) -> Vec<Problem> {
    // 1. Identify weak skills
    //    - Skills where skill_rating << user.global_rating
    //    - Skills not practiced in >7 days
    //    - Skills with low solve rates (<40%)

    // 2. Check prerequisites
    //    - Only recommend skills whose prerequisites are ≥ threshold
    //    - This prevents throwing bitmask DP at someone who can't do basic DP

    // 3. Select target difficulty
    //    - Sweet spot: user_skill_rating + 100 to + 300
    //    - Too easy = no learning; too hard = frustration
    //    - Adjust based on recent performance trend

    // 4. Filter problem pool
    //    - Unsolved problems matching target skills + difficulty
    //    - Prefer problems tagged with multiple weak skills (2-for-1)
    //    - Deprioritize problems the user has already seen hints for

    // 5. Rank and return top N
    //    - Score = skill_weakness_weight * difficulty_fit * freshness
}
```

**Difficulty Mapping:**

Since LC and CF use different scales, Myro normalizes to a unified rating:

| LC Difficulty | CF Rating | Myro Rating |
|---|---|---|
| Easy | 800–1200 | 800–1200 |
| Medium (lower) | 1200–1500 | 1200–1500 |
| Medium (upper) | 1500–1800 | 1500–1800 |
| Hard | 1800–2200 | 1800–2200 |
| — | 2200–3500 | 2200–3500 |

LC problems are further refined using community acceptance rate and contest data when available.

### 5.2 Rating System

**Glicko-2** (same family as CF) with adaptations for practice:

- **Global rating**: Updated after every solved problem and contest. Starts at 1000.
- **Per-skill rating**: Independent Glicko-2 rating per skill node. Lets you see "I'm 1800 at graphs but 1200 at DP."
- **Rating deviation**: High deviation = uncertain rating (new skill or haven't practiced). Engine prioritizes high-deviation skills.
- **Contest multiplier**: Virtual contest results affect rating 2× compared to regular practice (higher stakes = more signal).

**Rating update triggers:**
- Solve a problem → rating adjusts based on problem difficulty vs your rating
- Fail a problem → small negative adjustment (softened — we want people to attempt hard things)
- Contest performance → batch update across all problems in contest
- Time factor → faster solves at your rating level = slight positive bonus

### 5.3 Virtual Contests

Simulate real contest conditions in the terminal:

```
┌─ Contest: Div 2 Simulation ──── 01:47:23 remaining ─┐
│                                                       │
│  #  Problem              Diff   Status    Time        │
│  A  Array Rotation       1000   ✅ AC     00:04:12    │
│  B  Binary Pairs         1300   ✅ AC     00:18:45    │
│  C  Cycle Detection      1600   ⬜ —      —           │
│  D  DP on DAG            1900   ⬜ —      —           │
│  E  Euler Tour Queries   2200   ⬜ —      —           │
│                                                       │
│  [Enter] Open  [s] Submit  [r] Standings  [q] End     │
└───────────────────────────────────────────────────────┘
```

**Contest modes:**
- **CF-style**: 5–6 problems, increasing difficulty, 2-hour time limit, penalty scoring
- **LC-style**: 4 problems, weekly contest format
- **Custom**: Pick topic + difficulty range, auto-generates a problem set
- **Past contests**: Replay real CF rounds as virtual contests with timer

### 5.4 Progress Stats & Skill Graphs

The stats dashboard gives honest, actionable insights.

```
┌─ Dashboard ──────────────────────────────────────────┐
│                                                       │
│  Rating: 1547 (+23 this week)    Streak: 12 days     │
│  ▁▂▃▃▄▅▅▆▆▇▇█  Rating trend (30d)                   │
│                                                       │
│  ── Skill Breakdown ──────────────────────────────── │
│  dp.linear       ████████████████░░░░  1680  ↑       │
│  graph.traversal ██████████████░░░░░░  1520          │
│  dp.bitmask      ████████░░░░░░░░░░░░  1120  ⚠ weak │
│  math.ntheory    ██████░░░░░░░░░░░░░░   980  ⚠ weak │
│  strings.kmp     ████████████░░░░░░░░  1340          │
│                                                       │
│  ── This Week ────────────────────────────────────── │
│  Problems solved: 14    |  Contests: 2               │
│  Avg difficulty:  1420  |  Avg solve time: 22min     │
│  Hardest AC:      1800  |  Topics covered: 6         │
│                                                       │
│  ── Recommendations ─────────────────────────────── │
│  "Your bitmask DP is 400+ below your rating.         │
│   Try 3 targeted problems this week."                │
│                                                       │
│  [w] Weekly  [m] Monthly  [s] Skills  [h] History    │
└───────────────────────────────────────────────────────┘
```

**Stats tracked:**
- Rating trend (daily/weekly/monthly sparklines)
- Per-skill rating with trend arrows
- Solve rate by difficulty bucket
- Average solve time by difficulty
- Weak skill identification with actionable suggestions
- Contest performance history
- Streak and consistency metrics
- Time-of-day performance patterns

---

## 6. API Integration

### 6.1 Codeforces

CF has an official, well-documented REST API.

| Endpoint | Use |
|---|---|
| `problemset.problems` | Fetch full problem list with tags + ratings |
| `contest.list` | List contests for virtual contest replay |
| `contest.standings` | Standings for contest simulation |
| `user.info` | Import existing CF profile/rating |
| `user.status` | Import solve history to bootstrap skill levels |

**Sync strategy:** Full problem list fetched on first run (~9000 problems), then incremental updates. Cache in SQLite. Problems are tagged with CF's native tags, then mapped to Myro's skill taxonomy.

### 6.2 LeetCode

LC has no official public API, but a well-known **GraphQL endpoint** is usable:

| Query | Use |
|---|---|
| `problemsetQuestionList` | Fetch problems with difficulty, tags, acceptance rate |
| `question` (by slug) | Full problem description, examples, constraints |
| `userProfile` | Import existing LC profile |
| `userSubmissionList` | Import solve history |

**Considerations:**
- Rate limiting is aggressive — implement exponential backoff + caching
- Problem descriptions may need HTML→terminal rendering (use a markdown converter)
- No official test case execution — local judge needed for LC problems

### 6.3 Local Judge

For problems where online submission isn't practical, Myro runs a local judge:

```
1. User writes solution in $EDITOR
2. Myro reads the file
3. Runs against cached test cases (from problem examples)
4. Compares output (exact match, float tolerance, special judges)
5. Reports verdict: AC / WA / TLE / RE
6. For CF: can also submit via browser automation (optional, v2)
```

Supported languages (v1): C++, Python, Rust, Java, Go

---

## 7. User Experience

### 7.1 First Run

```bash
$ myro init

  Welcome to Myro! Let's set up your profile.

  → Language preference: [C++] Python Rust Java Go
  → Import from Codeforces? (username): tourist
  → Import from LeetCode? (username): —
  → Editor: $EDITOR (nvim)

  Fetching problem database... ████████████████ 9,247 problems
  Importing CF history........  ████████████████ 342 solves
  Analyzing skill levels...... done

  Your estimated rating: 1580
  Weakest skills: dp.bitmask (est. 1050), math.combinatorics (est. 1100)

  Run `myro` to start training. Happy grinding!
```

### 7.2 Keybindings

| Key | Action |
|---|---|
| `j/k` or `↑/↓` | Navigate lists |
| `Enter` | Open/select |
| `e` | Open problem in `$EDITOR` |
| `s` | Submit solution |
| `h` | Show hint (affects rating) |
| `t` | Toggle tags/difficulty visibility |
| `Tab` | Switch panels |
| `1–5` | Switch views (Problems, Train, Contest, Stats, Profile) |
| `/` | Search/filter problems |
| `?` | Help overlay |
| `q` | Quit / back |

### 7.3 Training Flow

```
User launches `myro train`
  │
  ├─ Engine selects problem based on adaptive algorithm
  │
  ├─ Problem displayed in TUI with description, examples, constraints
  │
  ├─ User presses `e` → solution file created, $EDITOR opens
  │
  ├─ User writes solution, saves, returns to TUI
  │
  ├─ User presses `s` → local judge runs test cases
  │
  ├─ Verdict shown:
  │   ├─ AC → rating updated, next problem offered
  │   ├─ WA → failed cases shown, user can retry
  │   └─ TLE/RE → diagnostics shown
  │
  └─ After session: summary with rating change, skills practiced
```

---

## 8. Configuration

`~/.config/myro/config.toml`

```toml
[profile]
username = "grinder42"
preferred_language = "cpp"
editor = "nvim"

[training]
session_length_minutes = 60          # Default session length
difficulty_range = [200, 400]        # Offset from rating: solve problems rated [you+200, you+400]
problems_per_session = 5             # Target problems per session
show_tags_before_solve = false       # Hide tags for realistic practice
show_difficulty_before_solve = true

[contest]
default_format = "cf_div2"           # cf_div2, cf_div1, lc_weekly, custom
penalty_scoring = true

[judge]
cpp_compiler = "g++"
cpp_flags = ["-std=c++17", "-O2", "-Wall"]
python_cmd = "python3"
rust_cmd = "rustc"
timeout_ms = 3000

[sync]
auto_sync = true
sync_interval_hours = 24
codeforces_handle = "tourist"
leetcode_handle = ""

[ui]
theme = "default"                     # default, monokai, gruvbox, catppuccin
show_sparklines = true
compact_mode = false
```

---

## 9. Development Roadmap

### Phase 1 — Foundation (Weeks 1–3)
- [ ] Project scaffolding, CI, Cargo workspace
- [ ] Basic TUI shell with navigation (ratatui + crossterm)
- [ ] SQLite database with schema + migrations
- [ ] Codeforces API client — fetch problems, tags, ratings
- [ ] Problem list view with filtering and sorting
- [ ] Problem detail view with description rendering

### Phase 2 — Core Training Loop (Weeks 4–6)
- [ ] `$EDITOR` integration — create solution files, launch editor
- [ ] Local judge — compile, run against test cases, report verdict
- [ ] Submission tracking in DB
- [ ] Basic difficulty-based problem recommendation
- [ ] LeetCode GraphQL client — fetch problems and descriptions

### Phase 3 — Intelligence (Weeks 7–9)
- [ ] Skill taxonomy definition (full tag mapping from CF/LC → Myro skills)
- [ ] Glicko-2 rating engine (global + per-skill)
- [ ] Adaptive recommendation algorithm
- [ ] CF/LC profile import — bootstrap ratings from solve history
- [ ] Skill level tracking and weak-skill identification

### Phase 4 — Stats & Contests (Weeks 10–12)
- [ ] Stats dashboard — rating trend, skill breakdown, weekly summary
- [ ] Sparkline and bar chart rendering in terminal
- [ ] Virtual contest mode — timer, problem set generation, scoring
- [ ] Past CF contest replay mode
- [ ] Session summaries with rating deltas

### Phase 5 — Polish & Ship (Weeks 13–14)
- [ ] Theming support (Catppuccin, Gruvbox, etc.)
- [ ] Config file support (TOML)
- [ ] Keybinding customization
- [ ] `myro init` onboarding wizard
- [ ] README, docs, install instructions
- [ ] Release binaries (cargo-dist or cross-compilation)
- [ ] Publish to crates.io, AUR, Homebrew

### Future (v2+)
- [ ] Spaced repetition review system
- [ ] Head-to-head duels (P2P or server)
- [ ] Problem editorials / hints system
- [ ] Community problem lists / shared study plans
- [ ] Online submission to CF via browser automation
- [ ] Plugin system for custom judges / languages
- [ ] Mobile companion (read problems on the go)

---

## 10. Technical Decisions & Trade-offs

| Decision | Rationale |
|---|---|
| **SQLite over Postgres** | Single-user app, no server needed, portable, fast |
| **Glicko-2 over Elo** | Handles rating uncertainty — crucial for new skills with few data points |
| **Local judge over online submit** | Instant feedback, works offline, no API abuse; online submit as opt-in v2 feature |
| **Unified difficulty scale** | Allows cross-platform comparison; CF's 800–3500 is well-calibrated, LC maps onto it |
| **Hide tags by default** | Realistic practice — knowing "this is a DP problem" makes it easier |
| **Async with tokio** | API calls shouldn't block the TUI; async fetching with loading indicators |
| **TOML config** | Standard in Rust ecosystem, readable, well-supported |

---

## 11. Inspiration & References

- **Ratatui ecosystem**: `lazygit`, `btop`, `gitui`, `spotify-tui` — proof that TUIs can be beautiful
- **Anki**: Spaced repetition principles for the review system
- **Chess.com puzzles**: Adaptive difficulty after each solve, Elo-rated puzzles
- **Exercism**: Track-based learning with mentor feedback (future inspiration)
- **CF Problemset**: Tag + rating system is the gold standard

---

*This is a living document. Update as design evolves.*
