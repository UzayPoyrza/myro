# Myro — Strategy & Planning

> Monetization, theming, MVP scope, community, sync, and go-to-market.

---

## Table of Contents

1. [Monetization & Open-Source Strategy](#1-monetization--open-source-strategy)
2. [Theming System & Terminal Aesthetics](#2-theming-system--terminal-aesthetics)
3. [MVP Scope — What Ships When](#3-mvp-scope--what-ships-when)
4. [Community Features](#4-community-features)
5. [Offline Mode & Sync Strategy](#5-offline-mode--sync-strategy)
6. [Marketing & Launch Plan](#6-marketing--launch-plan)

---

# 1. Monetization & Open-Source Strategy

## 1.1 The Core Tension

The CP community is allergic to paywalls. LeetCode Premium works because it
targets interview preppers who'll pay $35/mo to get a FAANG job. But the
competitive programming crowd — CF grinders, ICPC students — they expect free
tools and respect open-source. Myro needs to live in that culture while being
financially sustainable.

## 1.2 Recommended Model: Open Core

The TUI itself is **fully open-source** (MIT or Apache 2.0). Everything that
runs locally is free forever. This builds trust, enables contributions, and
lets the tool spread organically through the CP community.

Revenue comes from a **lightweight cloud layer** that adds convenience and
AI features on top.

### What's Free (Forever)

Everything that runs on your machine:

- Full TUI application
- Problem browsing & caching from CF/LC
- Local judge (compile, run, compare)
- CF submission integration
- Glicko-2 rating engine (local)
- Adaptive recommendation engine (local)
- Skill graph & progress tracking
- Stress test mode
- Virtual contests
- Theming, config, keybindings
- Import from CF/LC
- All future core features

### What's Paid: Myro Pro ($8/mo or $60/yr)

Features that require server infrastructure or significant ongoing cost:

| Feature | Why it costs money |
|---|---|
| **AI Coach** (Claude API) | LLM API calls cost real money per session |
| **Cloud sync** | Server + database for cross-device sync |
| **Community features** | Leaderboards, shared plans, matchmaking servers |
| **Myro contests** | Hosted rated contests with a real contestant pool |
| **Advanced analytics** | Historical trend analysis, peer comparison |
| **Priority problem DB updates** | Faster sync of new CF/LC problems |

### Pricing Psychology

- **$8/mo** is less than a Netflix sub, less than one LeetCode Premium month
- **$60/yr** (~$5/mo) rewards commitment — most serious CP practitioners
  will go annual
- **Student discount**: $4/mo or $30/yr with .edu email
- **Free trial**: 14 days of Pro, no credit card required
- **BYOK (Bring Your Own Key)**: Users can plug in their own Claude/OpenAI API
  key and get AI coaching for free (they pay the API provider directly). This
  is crucial — power users respect this option and it removes the objection
  "I'm paying you to call an API I could call myself"

### Why BYOK Matters

The CP community will immediately ask "why can't I just use my own API key?"
Offering BYOK means:
- Pro subscription is for *convenience + community*, not gatekeeping AI
- Users who don't want to manage API keys pay for the managed experience
- Users who prefer control get it
- Eliminates the strongest objection to paid AI features
- Local LLM (Ollama) users get AI coaching completely free

### Revenue Projections (Conservative)

| Metric | 6 months | 1 year | 2 years |
|---|---|---|---|
| GitHub stars | 2,000 | 8,000 | 25,000 |
| Active users (weekly) | 500 | 3,000 | 15,000 |
| Pro subscribers | 50 | 400 | 2,500 |
| Monthly revenue | $400 | $3,200 | $20,000 |
| Annual revenue | $2,400 | $24,000 | $150,000 |

These are conservative. A tool that genuinely helps people improve their CF
rating will get word-of-mouth traction fast in a community that talks.

## 1.3 Open-Source License

**Recommendation: AGPL-3.0 for core + MIT for libraries**

Why AGPL over MIT:
- Prevents someone from taking Myro, adding a subscription layer, and
  competing without contributing back
- Still fully free for individual users and self-hosting
- Companies that want to embed Myro must either contribute back or buy a
  commercial license (future revenue stream)
- The CP community respects strong copyleft — it signals "this stays free"

Alternative: **MIT** if maximum adoption is the priority and you're less
concerned about commercial forks.

## 1.4 Sponsorship & Grants

Additional revenue that doesn't gate features:

- **GitHub Sponsors**: Many devs will throw $5/mo at a tool they use daily
- **CF/ICPC sponsorships**: Competitive programming organizations may sponsor
  a tool that helps their community
- **University licenses**: Bulk Pro access for CS departments running CP clubs
- **Open Collective**: Transparent community funding

---

# 2. Theming System & Terminal Aesthetics

## 2.1 Design Philosophy

Myro should look like it belongs in the terminal of someone who cares about
their setup. That means shipping with beautiful defaults AND letting people
customize everything. Think ricing culture — if someone can't match Myro to
their terminal theme, they won't use it.

## 2.2 Theme Architecture

```toml
# ~/.config/myro/themes/catppuccin-mocha.toml

[meta]
name = "Catppuccin Mocha"
author = "catppuccin"
variant = "dark"

[palette]
# Base colors
bg         = "#1e1e2e"
surface    = "#313244"
overlay    = "#45475a"
text       = "#cdd6f4"
subtext    = "#a6adc8"
muted      = "#585b70"

# Accent colors
primary    = "#89b4fa"   # Blue — main accent
secondary  = "#a6e3a1"   # Green — success/AC
warning    = "#f9e2af"   # Yellow — caution
error      = "#f38ba8"   # Red — error/WA
info       = "#89dceb"   # Teal — informational

# Rating tier colors (CF-inspired but theme-adapted)
rating_newbie     = "#585b70"   # Gray
rating_pupil      = "#a6e3a1"   # Green
rating_specialist = "#89dceb"   # Cyan
rating_expert     = "#89b4fa"   # Blue
rating_candidate  = "#cba6f7"   # Mauve
rating_master     = "#f9e2af"   # Yellow
rating_grandmaster= "#f38ba8"   # Red

# UI element colors
border           = "#45475a"
border_focused   = "#89b4fa"
selection_bg     = "#313244"
selection_fg     = "#cdd6f4"
header_bg        = "#181825"
header_fg        = "#cdd6f4"
statusbar_bg     = "#181825"
statusbar_fg     = "#a6adc8"

# Sparkline gradient (low to high)
sparkline = ["#45475a", "#585b70", "#a6adc8", "#89b4fa", "#a6e3a1"]

# Coach indicator
coach_observing  = "#a6e3a1"   # Green
coach_concerned  = "#f9e2af"   # Yellow
coach_ready      = "#fab387"   # Peach
coach_intervene  = "#f38ba8"   # Red

[typography]
# Bold usage
headers    = "bold"
emphasis   = "bold"
keybinds   = "bold"
ratings    = "bold"

# Italic usage (if terminal supports)
descriptions = "italic"
coach_text   = "italic"
```

## 2.3 Built-in Themes

Ship with 6–8 themes covering the most popular terminal aesthetics:

```
┌─ Theme Gallery ──────────────────────────────────────────────────────────┐
│                                                                          │
│  ❯ Myro Default        Dark theme with blue/cyan accents                │
│    Catppuccin Mocha     Warm pastels on dark background                  │
│    Catppuccin Latte     Light theme with soft colors                     │
│    Gruvbox Dark         Earthy retro tones                               │
│    Gruvbox Light        Light variant with warm colors                   │
│    Tokyo Night          Cool blues and purples                           │
│    Nord                 Arctic, blue-gray palette                        │
│    Dracula              Purple-heavy dark theme                          │
│    Solarized Dark       Precision-designed dark palette                  │
│    Solarized Light      Precision-designed light palette                 │
│    Rosé Pine            Warm muted tones                                │
│    Kanagawa             Japanese ink-painting inspired                   │
│    Terminal Native      Inherits your terminal's ANSI colors             │
│                                                                          │
│  [Enter] Apply  [p] Preview  [c] Create custom                         │
└──────────────────────────────────────────────────────────────────────────┘
```

**Terminal Native** is important — it maps to the user's configured 16 ANSI
colors, so Myro automatically matches whatever terminal theme they're running.
This is the zero-config option that "just works."

## 2.4 Aesthetic Details

### Box Drawing

Use rounded corners for a modern feel, with graceful fallback:

```
Modern (Unicode):    Fallback (ASCII):
╭──────────╮         +----------+
│  Content │         |  Content |
╰──────────╯         +----------+
```

Config option:
```toml
[ui]
border_style = "rounded"  # rounded, sharp, thick, double, ascii
```

### Status Icons

Two modes — Unicode (default) and ASCII-safe:

```
Unicode:                  ASCII:
✅ Accepted               [AC] Accepted
❌ Wrong Answer            [WA] Wrong Answer
⏳ In progress             [...] In progress
⬜ Not attempted           [ ] Not attempted
🔥 Streak                  * Streak
🟢🟡🟠🔴 Coach indicator   [OK] [..] [!!] [XX]
⚡ Logo                    >> Logo
▁▂▃▄▅▆▇█ Sparkline        ___----## Sparkline
```

Config:
```toml
[ui]
icons = "unicode"  # unicode, ascii, nerdfont
```

### Nerd Font Support

For users with Nerd Fonts installed, richer icons:

```
 Problems    Dashboard    Stats    Contest
 Code        Terminal     Config   Profile
 Success     Error        Warning
```

### Syntax Highlighting in Code Preview

The code preview panel in the training view should have basic syntax
highlighting. Use ANSI colors mapped to the theme palette:

```
Keywords     → primary color (bold)      if, for, while, return
Types        → secondary color           int, vector, string
Strings      → warning color             "hello", 'c'
Comments     → muted color (italic)      // comment
Numbers      → info color                42, 3.14
Functions    → text color (bold)         main(), solve()
Operators    → subtext color             +, -, =, <<
```

### Animation & Transitions

Subtle animations that respect terminal capabilities:

- **Loading spinners**: Braille dots `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` cycling
- **Progress bars**: Smooth fill with partial blocks `█▉▊▋▌▍▎▏`
- **Rating change**: Number counts up/down briefly on update
- **Streak fire**: Subtle color pulse on streak display
- **Toast notifications**: Slide in from right, auto-dismiss after 3s

All animations configurable:
```toml
[ui]
animations = true       # Master toggle
animation_speed = 1.0   # Multiplier (0.5 = slow, 2.0 = fast)
toast_duration_ms = 3000
```

## 2.5 Compact Mode

For small terminals or users who want maximum information density:

```
Normal:                              Compact:
┌──────────────┐                     ┌──────┐
│  ⚡ myro      │                     │⚡1559 │
│              │                     │█▇▆▅▄ │
│  Rating:1559 │                     │🔥 13  │
│  ▁▂▃▄▅▆▇█   │                     │──────│
│  🔥 13 days  │                     │❯D P T│
│              │                     │ S C St│
│──────────────│                     │ Pr   │
│              │                     └──────┘
│ ❯ Dashboard  │
│   Problems   │
│   Train      │                     Width: 8 cols
│   ...        │
│              │
└──────────────┘

Width: 16 cols
```

---

# 3. MVP Scope — What Ships When

## 3.1 Philosophy

Ship the smallest thing that's genuinely useful, then iterate fast. A CP
practitioner should be able to install Myro and immediately get value from
it — even if half the features aren't built yet.

## 3.2 Release Timeline

### v0.1 — "It Works" (Weeks 1–6)

The absolute minimum to be useful as a daily training tool.

**Ships with:**
- [ ] TUI shell (sidebar, navigation, status bar, help overlay)
- [ ] CF problem fetcher (full problemset with tags + ratings)
- [ ] Problem browser with search, filter by difficulty/tags, sort
- [ ] Problem detail view with description rendering
- [ ] $EDITOR integration (create file, open editor, detect save)
- [ ] Local judge (compile + run against example test cases)
- [ ] Basic submission tracking (SQLite)
- [ ] Difficulty-based problem recommendation ("show me an unsolved 1400")
- [ ] One theme (default dark)
- [ ] Config file (language, editor, basic preferences)
- [ ] `myro init` setup wizard (minimal)

**Doesn't ship with:**
- No adaptive engine (just manual filtering)
- No rating system
- No AI coaching
- No CF submission
- No LC integration
- No contests
- No stats dashboard

**Why this is enough:**
Someone can install Myro, browse CF problems, open them in their editor,
test locally, and track what they've solved. It's already better than
opening codeforces.com in a browser for daily practice.

### v0.2 — "Smart" (Weeks 7–10)

The adaptive engine and rating system come online.

**Adds:**
- [ ] Glicko-2 rating engine (global + per-skill)
- [ ] Skill taxonomy (full CF tag → Myro skill mapping)
- [ ] Adaptive recommendation engine
- [ ] CF history import with time-weighted decay
- [ ] Per-skill tracking and weak-skill identification
- [ ] Training session mode (engine picks problems, tracks session)
- [ ] Session summary with rating deltas
- [ ] Stats dashboard (rating trend, skill breakdown, activity)
- [ ] `myro train` command

**The unlock:**
Now Myro isn't just a problem browser — it *knows what you should practice*
and tracks your improvement. This is the moment it becomes genuinely
differentiated from everything else.

### v0.3 — "Compete" (Weeks 11–14)

Contest and stress test modes.

**Adds:**
- [ ] CF session auth & submission (submit solutions from TUI)
- [ ] Verdict polling and display
- [ ] Local test-before-submit
- [ ] Virtual contest mode (replay past CF rounds)
- [ ] Stress test mode (mini contest, speed round, weakness blitz)
- [ ] Post-session analysis
- [ ] Contest result import for rating updates
- [ ] Rating weight hierarchy (practice < stress < virtual < live)

**The unlock:**
Myro replaces the CF web interface entirely for practice AND contests.
No browser needed.

### v0.4 — "Coach" (Weeks 15–18)

AI features land.

**Adds:**
- [ ] LLM provider abstraction (Claude API + Ollama + OpenAI-compatible)
- [ ] File watcher for code monitoring
- [ ] Approach detection (pattern matching on code)
- [ ] Intervention engine (when/how to help)
- [ ] Coach panel in TUI
- [ ] Socratic dialogue system
- [ ] Intervention levels (nudge → approach → walkthrough → explain)
- [ ] BYOK support (user provides their own API key)
- [ ] Coach indicator in status bar

**The unlock:**
The "stuck for 30 minutes" problem is solved. Myro becomes a teacher,
not just a tool.

### v0.5 — "Community" (Weeks 19-22)

Social and sync features (requires server infrastructure).

**Adds:**
- [ ] User accounts & authentication
- [ ] Cloud sync (ratings, progress, solve history)
- [ ] LeetCode problem integration
- [ ] LC contest result import
- [ ] Leaderboards (global, friends, skill-specific)
- [ ] Shared study plans
- [ ] Myro Pro subscription (payment integration)
- [ ] Additional themes (Catppuccin, Gruvbox, Nord, etc.)

### v1.0 — "Complete" (Weeks 23-26)

Polish, stability, and the full vision.

**Adds:**
- [ ] Myro-hosted rated contests
- [ ] Head-to-head duels
- [ ] Spaced repetition review system
- [ ] Advanced analytics (time-of-day patterns, peer comparison)
- [ ] Plugin system for custom judges/languages
- [ ] Full Nerd Font icon set
- [ ] Documentation site
- [ ] Installation via Homebrew, AUR, cargo install, binaries

## 3.3 Feature Dependency Graph

```
v0.1 TUI Shell ──────────► v0.2 Adaptive Engine ──► v0.3 Contests
  │ Problem browser             │ Glicko-2              │ CF submit
  │ Local judge                 │ Skill graph           │ Stress test
  │ Editor integration          │ Import                │ Virtual contest
  │                             │ Stats                 │
  │                             ▼                       │
  │                        v0.4 AI Coach ◄──────────────┘
  │                             │ LLM integration
  │                             │ Code watching
  │                             │ Socratic dialogue
  │                             ▼
  │                        v0.5 Community
  │                             │ Cloud sync
  │                             │ Leaderboards
  │                             │ LC integration
  │                             ▼
  └────────────────────►   v1.0 Complete
                                │ Hosted contests
                                │ Duels
                                │ Spaced repetition
```

---

# 4. Community Features

## 4.1 Why Community Matters

CP is inherently social — contest standings, rating comparisons, study groups,
editorial discussions. A training tool without community features feels lonely.
But the community features need to enhance training, not distract from it.

## 4.2 Feature Breakdown

### Friends & Following

```
┌─ Friends ──────────────────────────────────────────────┐
│                                                         │
│  Handle          Rating   Streak   Active Today         │
│  ──────────────────────────────────────────────────── │
│  rival42         1723     28d 🔥   5 problems           │
│  study_buddy     1489     14d 🔥   Training now...      │
│  cp_newbie       1102      3d      Idle                  │
│  icpc_grinder    2105     45d 🔥   Contest prep          │
│                                                         │
│  [Enter] View profile  [c] Challenge  [m] Message       │
└─────────────────────────────────────────────────────────┘
```

Friends are imported from CF friend lists or added by Myro handle. You can see
their activity, ratings, and streaks. **Privacy controls**: users choose what's
visible (rating only, activity, full solve history).

### Leaderboards

Multiple leaderboard types so everyone can compete on something:

| Leaderboard | Metric | Resets |
|---|---|---|
| **Global Rating** | Myro Glicko-2 rating | Never |
| **Weekly Grind** | Problems solved this week | Every Monday |
| **Streak Champions** | Current consecutive days | Never (ongoing) |
| **Skill Leaders** | Per-skill rating (e.g., best at DP) | Never |
| **Improvement** | Rating gain over 30 days | Rolling window |
| **Contest Stars** | Contest performance score | Monthly |

**Anti-gaming**: Weekly Grind counts *unique problems at your level*, not
easy spam. Solving 50 problems rated 500 below your rating counts for very
little.

### Shared Study Plans

Users can create and share structured problem lists:

```
┌─ Study Plan: "Road to Expert (1600+)" ─── by cp_master ──────────┐
│                                                                    │
│  ⭐ 847 followers  │  📊 73% avg completion  │  🕐 ~4 weeks        │
│                                                                    │
│  Week 1: Binary Search Mastery                                    │
│    □ CF 1201C  Binary Search on Answer          1300              │
│    □ CF 1117C  Magic Ship (BS + geometry)       1500              │
│    □ LC 410    Split Array Largest Sum           1600              │
│    □ CF 1201D  Treasure Hunting                  1500              │
│                                                                    │
│  Week 2: Segment Trees                                            │
│    □ CF 339D   Xenia and Bit Operations          1500              │
│    □ CF 380C   Sereja and Brackets               1700              │
│    ...                                                             │
│                                                                    │
│  [s] Start plan  [f] Fork & customize  [r] Rate plan              │
└────────────────────────────────────────────────────────────────────┘
```

Features:
- **Fork & customize**: Take someone's plan, modify it for your level
- **Progress tracking**: See your completion percentage vs the community's
- **Rating-aware**: Plans can adapt difficulty to your level automatically
- **AI-generated plans**: "Generate a 4-week plan to improve my graph skills"

### Head-to-Head Duels

Real-time competitive practice:

```
┌─ Duel: you (1559) vs rival42 (1723) ─── Best of 5 ───────────────┐
│                                                                    │
│  Problem 3 of 5           Difficulty: ~1550 (avg of both ratings) │
│                                                                    │
│  Score:  YOU  2 ████░  1  rival42                                 │
│                                                                    │
│  ┌─ Problem: XOR Subsequence ──────────────────────────────────┐  │
│  │ ...                                                          │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                    │
│  rival42 is typing...  ████████░░░░ (estimated 60% done)         │
│                                                                    │
│  [e] Editor  [s] Submit  [t] Test                                 │
└────────────────────────────────────────────────────────────────────┘
```

Duel mechanics:
- Matchmaking based on rating (±200 range)
- Problem difficulty = average of both players' ratings
- Best of 3 or 5 problems
- First to AC wins the round (speed matters)
- Rating impact: 1.5× weight (between practice and contest)
- Can challenge friends directly

### Myro-Hosted Contests

Weekly rated contests run entirely on Myro:

| Contest | When | Format | Who |
|---|---|---|---|
| **Myro Weekly** | Saturday 18:00 UTC | 5 problems, 2hr, CF-style | Open |
| **Myro Blitz** | Wednesday 20:00 UTC | 8 problems, 30min, speed | Open |
| **Div-specific** | Biweekly | 5 problems for rating range | Filtered |

Requires server infrastructure for:
- Problem hosting (curated or auto-generated from DB)
- Simultaneous timing
- Anti-cheat (basic: time limits, no AI coach; advanced: plagiarism detection)
- Standings computation
- Rating updates

## 4.3 Privacy & Safety

- All social features are **opt-in** — you can use Myro fully offline/private
- Solve history sharing has granular controls (public/friends/private)
- No toxicity in community features — no chat, just structured interactions
- Block/report functionality for duels
- Leaderboard pseudonymity option (show rating but not handle)

---

# 5. Offline Mode & Sync Strategy

## 5.1 Design Principle: Offline-First

Myro works fully offline. The cloud is an enhancement, never a requirement.
This is critical for:
- Practicing on planes, trains, cafes with bad wifi
- Users who don't want cloud accounts
- Competitions/exams where internet isn't available
- Trust — your data is yours, locally

## 5.2 What's Local vs Cloud

```
┌─────────────────────────────┬──────────────────────────────────┐
│  Always Local               │  Synced to Cloud (if opted in)   │
├─────────────────────────────┼──────────────────────────────────┤
│ Problem database (cached)   │ Solve history & submissions      │
│ Solution files              │ Ratings (global + per-skill)     │
│ Local judge                 │ Session history                  │
│ Adaptive engine (runs local)│ Streak data                      │
│ Rating computation          │ Study plan progress              │
│ Config & themes             │ Contest results                  │
│ AI coach (with local LLM)   │ Friend connections               │
│                             │ Leaderboard entries              │
└─────────────────────────────┴──────────────────────────────────┘
```

## 5.3 Problem Caching

The problem database is the biggest offline requirement (~50-100MB for all
CF + LC problems with descriptions):

```rust
struct ProblemCache {
    // SQLite database at ~/.local/share/myro/problems.db
    db: Connection,
    
    // Cache metadata
    last_full_sync: Option<DateTime<Utc>>,
    last_incremental_sync: Option<DateTime<Utc>>,
    problem_count: u32,
}

impl ProblemCache {
    /// Initial sync — downloads entire CF + LC problem set
    /// This takes 2-5 minutes on first run
    async fn full_sync(&mut self) -> Result<SyncReport> {
        // CF: ~9000 problems via problemset.problems API
        // LC: ~3000 problems via GraphQL
        // Total: ~12000 problems, ~50MB compressed
        
        // 1. Fetch problem metadata (fast — just titles, tags, ratings)
        // 2. Store in SQLite
        // 3. Queue full descriptions for background download
        // 4. Descriptions fetched lazily (on first view) or in background
    }
    
    /// Incremental sync — only new/updated problems since last sync
    async fn incremental_sync(&mut self) -> Result<SyncReport> {
        // CF: Check for new contests since last sync
        // LC: Check for new problems since last sync
        // Usually <100 new problems per week
    }
    
    /// Lazy description fetch — only download full problem text when needed
    async fn ensure_description(&mut self, problem_id: &str) -> Result<String> {
        if let Some(desc) = self.db.get_description(problem_id)? {
            return Ok(desc);
        }
        
        // Fetch from API, cache locally
        let desc = self.fetch_description(problem_id).await?;
        self.db.set_description(problem_id, &desc)?;
        Ok(desc)
    }
}
```

**Sync schedule:**
- Full sync: First run only (or manual `myro sync --full`)
- Incremental sync: Every 24 hours (configurable)
- Background sync: While you're solving a problem, sync new problems
- Manual sync: `myro sync` anytime

## 5.4 Cloud Sync Protocol

For Pro users, cross-device sync using a simple conflict resolution model:

```
Device A (laptop)                    Cloud                    Device B (desktop)
     │                                │                            │
     │ Solve 3 problems               │                            │
     │ Rating: 1547 → 1559            │                            │
     │                                │                            │
     ├──── Push: {submissions,   ─────►│                            │
     │           ratings,              │                            │
     │           timestamp}            │                            │
     │                                │◄──── Pull ─────────────────┤
     │                                │                            │
     │                                ├──── {merged state} ────────►│
     │                                │                            │
     │                                │     Solve 2 problems       │
     │                                │     Rating: 1559 → 1571    │
     │                                │                            │
     │                                │◄──── Push ─────────────────┤
     │◄──── Pull ─────────────────────┤                            │
     │                                │                            │
     │ Local state updated:           │                            │
     │ Rating: 1559 → 1571            │                            │
     │ +2 submissions merged          │                            │
```

**Conflict resolution:**
- **Submissions**: Append-only — never conflicts (just merge both sets)
- **Ratings**: Last-write-wins with timestamp (most recent device's rating is authoritative since it has the most data)
- **Config/preferences**: Per-device (not synced)
- **Skill levels**: Recomputed from merged submission history

**Sync API:**

```
POST /api/v1/sync
{
  "device_id": "laptop-abc123",
  "last_sync": "2026-02-17T10:00:00Z",
  "submissions": [...new submissions since last sync...],
  "ratings": {
    "global": { "rating": 1559, "deviation": 85, "updated_at": "..." },
    "skills": { "dp.bitmask": { ... }, ... }
  }
}

Response:
{
  "new_submissions": [...submissions from other devices...],
  "authoritative_ratings": { ... },
  "server_time": "2026-02-18T14:23:00Z"
}
```

## 5.5 Data Export

Users can always export everything, even without Pro:

```bash
$ myro export --format json > myro-backup.json
$ myro export --format csv --submissions > submissions.csv
$ myro export --format csv --ratings > ratings.csv
```

No lock-in. Your data is your data.

---

# 6. Marketing & Launch Plan

## 6.1 The CP Community Landscape

Where competitive programmers hang out:

| Channel | Audience | Best for |
|---|---|---|
| **Codeforces blog** | 500K+ active users | Launch announcement, feature updates |
| **r/competitiveprogramming** | 120K subscribers | Discussion, feedback |
| **r/csMajors** | 300K+ subscribers | Interview prep angle |
| **CP Discord servers** | 10K-50K per server | Direct community engagement |
| **Twitter/X CP community** | Varies | Visual demos, GIFs |
| **YouTube** | Large CP audience | Demo videos, tutorials |
| **GitHub** | Developers | Stars, contributions |
| **Hacker News** | Tech generalists | Launch day traffic spike |

## 6.2 Launch Phases

### Phase 0: Stealth Build (Before v0.1)

- Set up GitHub repo (private initially)
- Create a minimal landing page (myro.dev) with email waitlist
- "Coming soon" teaser post on CF blog — gauge interest
- Build in public on Twitter with progress screenshots/GIFs
- Target: 200-500 email signups before launch

### Phase 1: v0.1 Launch — "The Tool"

**Timing:** As soon as v0.1 is stable enough for daily use

**Channels:**
1. **Codeforces blog post** — This is the most important single action.
   A well-written CF blog post with:
   - Clear problem statement ("existing tools suck because...")
   - Demo GIF showing the workflow
   - GitHub link
   - "Built with Rust + Ratatui" (the CF crowd respects this)
   - Call for feedback and contributors

2. **r/competitiveprogramming post** — Same narrative, tailored for Reddit

3. **GitHub repo goes public** — Clean README with:
   - Hero GIF/screenshot
   - One-line install (`cargo install myro`)
   - Feature list with ✅ shipped / 🚧 coming soon
   - Contributing guide
   - Roadmap

4. **Hacker News "Show HN"** — "Show HN: Myro – A TUI for competitive
   programming training (Rust)"

**Goal:** 500 GitHub stars, 100 weekly active users, 50 bug reports/feature
requests (engagement > vanity metrics)

### Phase 2: v0.2 Launch — "The Brain"

**Narrative shift:** "Myro now knows what you should practice"

**Channels:**
1. CF blog post: "Myro v0.2: Adaptive training with per-skill ratings"
   - Show before/after: "I imported my CF history and it found my weaknesses"
   - Screenshots of skill breakdown, adaptive recommendations
   - Real user testimonials (from v0.1 users)

2. **YouTube demo** (3-5 min): Full workflow from import → training session
   → session summary showing rating improvements

3. **Targeted outreach**: DM 10-20 well-known CP practitioners, offer early
   access, ask for feedback. If even one says "this is cool" publicly, it's
   worth 1000 ads.

**Goal:** 2000 stars, 500 WAU, first community contributions

### Phase 3: v0.3 Launch — "The Arena"

**Narrative:** "Never open codeforces.com again"

**Channels:**
1. CF blog: "Submit to CF from your terminal. Plus: stress test mode."
   - Demo GIF of live contest submission from Myro
   - Stress test results and analysis screenshots

2. **CP Discord presence** — Create a Myro Discord server for users,
   feature requests, and community

3. **AUR / Homebrew packages** — Make installation trivially easy

**Goal:** 5000 stars, 1500 WAU, active Discord community

### Phase 4: v0.4 Launch — "The Coach"

**Narrative:** "AI that watches your code and teaches you"

This is the biggest launch moment — AI coaching is genuinely novel in the
CP space.

**Channels:**
1. CF blog: "Myro v0.4: An AI coach that watches you code"
   - Compelling demo: show the AI noticing a wrong approach and guiding
     the user to the right one through Socratic dialogue
   - Emphasize: works with local LLMs too (no vendor lock-in)

2. **YouTube deep dive** (10 min): Full coaching session, show all
   intervention levels, show the dialogue

3. **Hacker News round 2**: "Myro now has AI coaching that watches your
   code in real-time"

4. **Twitter thread**: Visual thread with terminal screenshots showing
   the coach in action

**Goal:** 10K stars, 3000 WAU, 50 Pro subscribers, first revenue

### Phase 5: v0.5+ — "The Community"

**Narrative:** "Train together. Compete together."

1. Launch Myro Pro (payment integration)
2. Launch community features with fanfare
3. First Myro-hosted contest (make it an event)
4. University outreach (CS departments, CP clubs)

## 6.3 Content Strategy

**Ongoing content that keeps Myro visible:**

| Content | Frequency | Channel |
|---|---|---|
| "Myro weekly stats" (community solve stats) | Weekly | CF blog, Twitter |
| Release notes | Per release | GitHub, Discord |
| "Problem of the week" analysis | Weekly | CF blog |
| User spotlight (rating improvement stories) | Biweekly | Twitter, blog |
| Dev log / technical deep dive | Monthly | Blog, HN |
| "How I went from X to Y rating with Myro" | User-generated | CF blog |

## 6.4 Key Messaging

**Tagline options:**
- "Train smarter. In your terminal."
- "Your CP coach lives in the terminal."
- "The training tool competitive programmers deserve."
- "Adaptive. Terminal-native. Open source."

**Core value props by audience:**

| Audience | Message |
|---|---|
| CF grinders | "Myro knows your weak skills and drills them. Stop wasting time on random problems." |
| Interview preppers | "Structured training with real difficulty ratings. Know exactly when you're ready." |
| Terminal enthusiasts | "Beautiful TUI, Rust-powered, vim keybindings. Finally a CP tool that belongs in your workflow." |
| Open source advocates | "Fully open-source core. Your data stays local. No lock-in, ever." |

## 6.5 Competitive Positioning

```
                    Training Intelligence
                          ▲
                          │
                   Myro ──┤──── (future: with AI coach)
                          │
                          │
          LeetCode ───────┤
                          │
                          │
     Codeforces ──────────┤
                          │
     ──────────────────────┼──────────────────────► Terminal-Native
     Browser-based         │                        Workflow
                          │
```

**vs LeetCode:** Myro is free, terminal-native, has per-skill ratings
(not just Easy/Med/Hard), and the adaptive engine is smarter than LC's
recommendation system. LC wins on interview-specific content (company tags,
mock interviews).

**vs Codeforces:** Myro doesn't replace CF — it makes CF better. It's a
client that adds training intelligence on top. CF is the problem source
and contest platform; Myro is the training layer.

**vs cf-tool/competitive-companion:** These are submission tools, not
training platforms. Myro includes their functionality but adds rating,
adaptation, coaching, and stats.

**The unique position:** No tool combines adaptive training + terminal UX +
AI coaching + cross-platform problem aggregation. Myro owns this intersection.

---

# Summary: The Full Picture

```
Month 1-2:   v0.1 ships. Open source. Basic but useful.
             First CF blog post. GitHub goes public.
             
Month 3-4:   v0.2 ships. Adaptive engine + ratings.
             "This is actually smart" moment.
             Community starts forming.
             
Month 5-6:   v0.3 ships. CF integration + contests.
             "I never open CF in a browser anymore."
             Active Discord. Contributors appearing.
             
Month 7-8:   v0.4 ships. AI coaching.
             "Holy shit the AI noticed my approach was wrong."
             Pro subscriptions begin. First revenue.
             
Month 9-10:  v0.5 ships. Community features.
             Leaderboards, duels, shared plans.
             University partnerships.
             
Month 11-12: v1.0 ships. The full vision.
             Hosted contests. Mature product.
             Sustainable revenue from Pro subs.
```

*The goal isn't to build a business — it's to build the tool that helps
people get better at competitive programming. The business model exists
to keep the tool alive and improving. Open core ensures it stays honest.*
