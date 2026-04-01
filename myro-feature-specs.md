# Myro — Feature Specs

> Technical specifications for: Time-Weighted Import, AI Code-Watching,
> Codeforces IDE Integration, and Stress Test Mode

---

## Table of Contents

1. [Time-Weighted Import System](#1-time-weighted-import-system)
2. [AI Code-Watching & Intervention System](#2-ai-code-watching--intervention-system)
3. [Codeforces IDE Integration](#3-codeforces-ide-integration)
4. [Stress Test Mode](#4-stress-test-mode)

---

# 1. Time-Weighted Import System

## 1.1 Problem Statement

A user's solve history from CF/LC spans months or years. Treating a solve from
3 years ago the same as yesterday's is wrong — skills decay, techniques are
forgotten, and the competitive landscape evolves. We need time-weighted import
that produces accurate *current* skill estimates.

## 1.2 Decay Model

We use **exponential decay** applied to the score fed into Glicko-2 during
bootstrap. The core formula:

```rust
/// Time decay applied to historical submission scores
fn decay_score(
    base_score: f64,          // 1.0 for AC, 0.0 for fail
    submission_time: DateTime<Utc>,
    now: DateTime<Utc>,
    verdict: Verdict,
) -> f64 {
    let days_ago = (now - submission_time).num_days() as f64;
    
    // Different half-lives for different verdicts
    let half_life_days = match verdict {
        // Solves decay slower — you DID know it once
        Verdict::AC => 365.0,        // 1-year half-life
        // Failures decay faster — you've likely learned since
        Verdict::WA | Verdict::TLE | Verdict::RE => 180.0,  // 6-month half-life
        // Compile errors are noise — decay very fast
        Verdict::CE => 60.0,         // 2-month half-life
    };
    
    let decay_factor = (0.5_f64).powf(days_ago / half_life_days);
    
    base_score * decay_factor
}
```

**Decay curves visualized:**

```
Score weight
1.0 │ ██
    │ ██▓▓
    │ ██▓▓▓▓
0.75│ ██▓▓▓▓░░                AC (365-day half-life)
    │ ██▓▓▓▓░░░░
0.5 │ ██▓▓▓▓░░░░░░░░
    │ ██▓▓░░░░░░░░░░░░░░
0.25│ ██▓▓░░░░░░░░░░░░░░░░░░░░
    │ ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░
0.0 └──────────────────────────────────── Time
    Now  3mo  6mo  9mo  1yr  1.5yr  2yr  3yr

Score weight
1.0 │ ██
    │ ██▓▓
0.75│ ██▓▓░░               WA/TLE (180-day half-life)
    │ ██▓▓░░░░
0.5 │ ██▓▓░░░░░░
    │ ██░░░░░░░░░░░░
0.25│ ██░░░░░░░░░░░░░░░░
    │ ░░░░░░░░░░░░░░░░░░░░░░░░
0.0 └──────────────────────────────────── Time
    Now  3mo  6mo  9mo  1yr  1.5yr  2yr  3yr
```

**Practical effect at key time points (AC solve):**

| Time Ago | Decay Factor | Meaning |
|---|---|---|
| 1 week | 0.987 | Basically full credit |
| 1 month | 0.942 | Almost full credit |
| 3 months | 0.835 | Strong signal |
| 6 months | 0.698 | Moderate signal |
| 1 year | 0.500 | Half credit (half-life) |
| 2 years | 0.250 | Weak signal |
| 3 years | 0.125 | Very weak — mostly forgotten |
| 5 years | 0.031 | Near zero — ancient history |

## 1.3 Import Pipeline

### Step 1: Fetch History

```rust
struct HistoricalSubmission {
    source: Source,               // CF or LC
    problem_id: String,
    problem_difficulty: u32,      // CF rating or LC mapped difficulty
    problem_tags: Vec<String>,    // Original platform tags
    verdict: Verdict,
    timestamp: DateTime<Utc>,
    language: Option<String>,
    contest_id: Option<String>,   // If solved during a contest
}

async fn fetch_codeforces_history(handle: &str) -> Result<Vec<HistoricalSubmission>> {
    // CF API: user.status?handle={handle}&from=1&count=10000
    let url = format!(
        "https://codeforces.com/api/user.status?handle={}&from=1&count=10000",
        handle
    );
    let response: CfApiResponse = reqwest::get(&url).await?.json().await?;
    
    response.result.iter().map(|s| HistoricalSubmission {
        source: Source::Codeforces,
        problem_id: format!("cf:{}{}",
            s.problem.contest_id, s.problem.index),
        problem_difficulty: s.problem.rating.unwrap_or(0),
        problem_tags: s.problem.tags.clone(),
        verdict: map_cf_verdict(&s.verdict),
        timestamp: DateTime::from_timestamp(s.creation_time_seconds, 0).unwrap(),
        language: Some(s.programming_language.clone()),
        contest_id: s.contest_id.map(|id| format!("cf:{}", id)),
    }).collect()
}

async fn fetch_leetcode_history(username: &str) -> Result<Vec<HistoricalSubmission>> {
    // LC GraphQL: recentAcSubmissionList + userSubmissionList
    let query = r#"
        query userSubmissions($username: String!, $offset: Int!, $limit: Int!) {
            submissionList(
                username: $username, offset: $offset, limit: $limit
            ) {
                submissions {
                    id, title, titleSlug, statusDisplay,
                    timestamp, lang
                }
                hasNext
            }
        }
    "#;
    // Paginate through all submissions
    // Map LC difficulty (Easy/Med/Hard) + acceptance rate → Myro rating
    // ...
}
```

### Step 2: Deduplicate & Select Best Attempt

For each problem, keep the **best verdict** but use the **timestamp of that attempt**:

```rust
fn deduplicate_submissions(
    submissions: Vec<HistoricalSubmission>
) -> Vec<HistoricalSubmission> {
    let mut best_per_problem: HashMap<String, HistoricalSubmission> = HashMap::new();
    
    for sub in submissions {
        let key = sub.problem_id.clone();
        let entry = best_per_problem.entry(key);
        
        entry
            .and_modify(|existing| {
                // Keep the best verdict (AC > WA > TLE > RE > CE)
                if verdict_rank(&sub.verdict) > verdict_rank(&existing.verdict) {
                    *existing = sub.clone();
                }
                // If same verdict, keep the earlier one (first AC is more meaningful)
                else if sub.verdict == existing.verdict
                    && sub.timestamp < existing.timestamp
                {
                    *existing = sub.clone();
                }
            })
            .or_insert(sub);
    }
    
    best_per_problem.into_values().collect()
}
```

### Step 3: Chronological Glicko-2 Replay

Process submissions in time order, grouping into **weekly rating periods**:

```rust
fn bootstrap_ratings(
    submissions: &[HistoricalSubmission],
    skill_graph: &mut SkillGraph,
    now: DateTime<Utc>,
) {
    // Sort chronologically
    let mut sorted = submissions.to_vec();
    sorted.sort_by_key(|s| s.timestamp);
    
    // Group into weekly periods
    let periods = group_into_weeks(&sorted);
    
    for period in periods {
        // For each skill affected this week, collect results
        let skills_affected = collect_affected_skills(&period, skill_graph);
        
        for skill_id in skills_affected {
            let results: Vec<ProblemResult> = period.iter()
                .filter(|s| involves_skill(s, &skill_id, skill_graph))
                .map(|s| {
                    // Apply time decay to the score
                    let base_score = if s.verdict == Verdict::AC { 1.0 } else { 0.0 };
                    let decayed = decay_score(base_score, s.timestamp, now, s.verdict);
                    
                    ProblemResult {
                        problem_difficulty: s.problem_difficulty as f64,
                        problem_deviation: 50.0,
                        score: decayed,
                    }
                })
                .collect();
            
            if !results.is_empty() {
                let skill = skill_graph.get_mut(&skill_id);
                skill.glicko2.update(&results);
                skill.problems_seen += results.len() as u32;
                skill.problems_solved += results.iter()
                    .filter(|r| r.score > 0.5).count() as u32;
            }
        }
        
        // Also update global rating with all problems this week
        let global_results: Vec<ProblemResult> = period.iter()
            .map(|s| {
                let base_score = if s.verdict == Verdict::AC { 1.0 } else { 0.0 };
                ProblemResult {
                    problem_difficulty: s.problem_difficulty as f64,
                    problem_deviation: 50.0,
                    score: decay_score(base_score, s.timestamp, now, s.verdict),
                }
            })
            .collect();
        
        skill_graph.global_rating.update(&global_results);
    }
}
```

### Step 4: Contest Bonus

Submissions that occurred during contests get extra weight:

```rust
fn apply_contest_weight(sub: &HistoricalSubmission) -> f64 {
    if sub.contest_id.is_some() {
        // Contest solves are replicated 1.5x during import
        // (less than live 2-3x because it's historical)
        1.5
    } else {
        1.0
    }
}
```

### Step 5: Post-Import Deviation Adjustment

After replay, skills with few recent data points should have **inflated deviation**
to signal uncertainty:

```rust
fn adjust_post_import_deviations(skill_graph: &mut SkillGraph, now: DateTime<Utc>) {
    for skill in skill_graph.all_skills_mut() {
        // If last practice was >6 months ago, inflate deviation
        if let Some(last) = skill.last_practiced {
            let months_ago = (now - last).num_days() as f64 / 30.0;
            if months_ago > 6.0 {
                let inflation = (months_ago / 6.0).min(3.0); // Cap at 3x
                skill.glicko2.phi *= inflation;
                skill.glicko2.phi = skill.glicko2.phi.min(350.0 / 173.7178);
            }
        }
        
        // Skills with <3 data points get max deviation regardless
        if skill.problems_seen < 3 {
            skill.glicko2.phi = 350.0 / 173.7178; // Max uncertainty
        }
    }
}
```

## 1.4 Import UX

```
$ myro import

  ── Codeforces ────────────────────────────────────────
  Handle: tourist
  Fetching submissions... ████████████████ 3,847 submissions
  Deduplicating............. 2,104 unique problems
  
  ── LeetCode ──────────────────────────────────────────
  Username: lc_grinder
  Fetching submissions... ████████████████ 892 submissions
  Deduplicating............. 743 unique problems
  
  ── Processing ────────────────────────────────────────
  Mapping tags to skill graph... done
  Replaying history chronologically...
  ████████████████████████████████████████ 156 weeks
  
  Applying time decay (half-life: 365 days)...
  Adjusting uncertainty for stale skills...
  
  ── Results ───────────────────────────────────────────
  
  Estimated global rating: 1847
  
  Strongest skills:                  Weakest skills:
  ├─ graph.shortest_path  2105      ├─ dp.bitmask        1240 ⚠
  ├─ dp.linear            1980      ├─ strings.suffix     1180 ⚠
  ├─ searching.binary     1920      ├─ math.fft          1050 ⚠
  └─ ds.segment_tree      1870      └─ graph.flow        1100 ⚠
  
  High uncertainty (need re-testing):
  ├─ dp.digit          1450 ±280   (last practiced 14 months ago)
  ├─ math.combinatorics 1380 ±310   (only 2 problems in history)
  └─ strings.hashing   1520 ±250   (last practiced 11 months ago)
  
  Ready to train! Run `myro train` to start.
```

## 1.5 Configuration

```toml
[import]
# Decay half-life in days for AC submissions
ac_half_life_days = 365

# Decay half-life for failed submissions (WA, TLE, RE)
fail_half_life_days = 180

# Minimum decay factor — even ancient solves get some credit
min_decay_factor = 0.05

# Weight multiplier for contest submissions during import
contest_weight = 1.5

# Minimum problems needed per skill before trusting the rating
min_problems_for_confidence = 3
```

---

# 2. AI Code-Watching & Intervention System

## 2.1 Architecture Overview

The AI system has three components:

```
┌────────────────────────────────────────────────────────┐
│                    Myro TUI                        │
│                                                         │
│  ┌──────────┐   ┌──────────────┐   ┌───────────────┐  │
│  │  File     │   │  Approach    │   │  Intervention │  │
│  │  Watcher  │──▶│  Analyzer    │──▶│  Engine       │  │
│  │           │   │              │   │               │  │
│  │ Watches   │   │ Parses code  │   │ Decides when  │  │
│  │ solution  │   │ Detects      │   │ and how to    │  │
│  │ file for  │   │ patterns,    │   │ intervene     │  │
│  │ changes   │   │ approach,    │   │               │  │
│  │           │   │ stuck signals│   │ Generates     │  │
│  │           │   │              │   │ Socratic      │  │
│  │           │   │              │   │ dialogue      │  │
│  └──────────┘   └──────────────┘   └───────────────┘  │
│                                           │             │
│                                    ┌──────▼──────┐     │
│                                    │  LLM Client │     │
│                                    │ Claude API  │     │
│                                    │   or Local  │     │
│                                    └─────────────┘     │
└────────────────────────────────────────────────────────┘
```

## 2.2 File Watcher

Monitors the solution file for changes using OS-level file watching:

```rust
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc;
use std::time::{Duration, Instant};

struct CodeWatcher {
    // Current state of the user's code
    current_code: String,
    // History of code snapshots with timestamps
    snapshots: Vec<CodeSnapshot>,
    // Debounce: don't analyze on every keystroke
    last_change: Instant,
    debounce_ms: u64,
    // File being watched
    file_path: PathBuf,
}

struct CodeSnapshot {
    code: String,
    timestamp: Instant,
    line_count: usize,
    // Diff from previous snapshot
    lines_added: usize,
    lines_removed: usize,
    // Detected patterns
    detected_approach: Option<ApproachSignal>,
}

impl CodeWatcher {
    fn new(file_path: PathBuf) -> Self {
        Self {
            current_code: String::new(),
            snapshots: Vec::new(),
            last_change: Instant::now(),
            debounce_ms: 2000, // Analyze at most every 2 seconds
            file_path,
        }
    }
    
    fn start(&mut self) -> mpsc::Receiver<CodeEvent> {
        let (tx, rx) = mpsc::channel();
        let path = self.file_path.clone();
        
        // OS-level file watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) => {
                        tx.send(CodeEvent::FileChanged).ok();
                    }
                    _ => {}
                }
            }
        }).unwrap();
        
        watcher.watch(&path, RecursiveMode::NonRecursive).unwrap();
        rx
    }
    
    fn on_file_changed(&mut self) -> Option<AnalysisTrigger> {
        // Debounce
        if self.last_change.elapsed() < Duration::from_millis(self.debounce_ms) {
            return None;
        }
        self.last_change = Instant::now();
        
        // Read current file
        let new_code = std::fs::read_to_string(&self.file_path).ok()?;
        if new_code == self.current_code {
            return None;
        }
        
        // Compute diff metrics
        let diff = compute_diff(&self.current_code, &new_code);
        
        let snapshot = CodeSnapshot {
            code: new_code.clone(),
            timestamp: Instant::now(),
            line_count: new_code.lines().count(),
            lines_added: diff.additions,
            lines_removed: diff.deletions,
            detected_approach: detect_approach_signals(&new_code),
        };
        
        self.current_code = new_code;
        self.snapshots.push(snapshot);
        
        // Decide if we should trigger analysis
        self.should_analyze()
    }
    
    fn should_analyze(&self) -> Option<AnalysisTrigger> {
        let snapshots = &self.snapshots;
        if snapshots.len() < 2 { return None; }
        
        let latest = snapshots.last().unwrap();
        let session_duration = latest.timestamp - snapshots[0].timestamp;
        
        // Trigger 1: Significant code rewrite (deleted >50% and rewrote)
        let recent_3 = &snapshots[snapshots.len().saturating_sub(3)..];
        let total_deleted: usize = recent_3.iter().map(|s| s.lines_removed).sum();
        if total_deleted > latest.line_count / 2 && total_deleted > 10 {
            return Some(AnalysisTrigger::MajorRewrite);
        }
        
        // Trigger 2: Long idle after writing code (>5 min no changes)
        // (detected by caller, not here)
        
        // Trigger 3: Enough code written for meaningful analysis (>15 lines)
        if latest.line_count > 15 && snapshots.len() >= 5 {
            return Some(AnalysisTrigger::SubstantialProgress);
        }
        
        // Trigger 4: Approach detected that's likely wrong for this problem
        if let Some(approach) = &latest.detected_approach {
            return Some(AnalysisTrigger::ApproachDetected(approach.clone()));
        }
        
        None
    }
}

enum AnalysisTrigger {
    MajorRewrite,            // User rewrote significant code — likely stuck
    SubstantialProgress,     // Enough code to analyze approach
    ApproachDetected(ApproachSignal), // Specific technique detected
    UserIdle(Duration),      // User stopped typing for a while
    UserRequested,           // User pressed the "help" key
}
```

## 2.3 Approach Analyzer

Lightweight **pattern detection** on the code to identify what the user is trying
before sending to the LLM (saves API calls and latency):

```rust
struct ApproachSignal {
    technique: DetectedTechnique,
    confidence: f64,           // 0.0–1.0
    evidence: Vec<String>,     // What code patterns matched
}

enum DetectedTechnique {
    BruteForce,
    Greedy,
    BinarySearch,
    DFS,
    BFS,
    Dijkstra,
    DynamicProgramming,
    SegmentTree,
    UnionFind,
    Sorting,
    TwoPointers,
    SlidingWindow,
    Backtracking,
    Unknown,
}

/// Fast pattern matching on code — no LLM needed
fn detect_approach_signals(code: &str) -> Option<ApproachSignal> {
    let mut signals: Vec<ApproachSignal> = Vec::new();
    
    // DP patterns
    let dp_indicators = [
        (r"dp\[", "dp array"),
        (r"memo\[", "memoization"),
        (r"@cache|@lru_cache|functools\.cache", "Python memoization decorator"),
        (r"vector<vector<", "2D array (possible DP table)"),
        (r"for.*in range.*for.*in range", "nested loops over state space"),
    ];
    let dp_score: f64 = dp_indicators.iter()
        .filter(|(pattern, _)| regex_matches(code, pattern))
        .map(|_| 0.3)
        .sum();
    if dp_score >= 0.6 {
        signals.push(ApproachSignal {
            technique: DetectedTechnique::DynamicProgramming,
            confidence: dp_score.min(1.0),
            evidence: dp_indicators.iter()
                .filter(|(p, _)| regex_matches(code, p))
                .map(|(_, desc)| desc.to_string())
                .collect(),
        });
    }
    
    // BFS patterns
    let bfs_indicators = [
        (r"queue|deque|Queue", "queue data structure"),
        (r"\.append\(.*\)|\.push\(", "queue operations"),
        (r"visited|seen|dist\[", "visited tracking"),
        (r"while.*queue|while.*q\b", "BFS loop pattern"),
    ];
    // ... similar scoring
    
    // Greedy patterns
    let greedy_indicators = [
        (r"sort\(|sorted\(|\.sort\(\)", "sorting (often greedy)"),
        (r"for.*in.*sorted", "iterating sorted order"),
    ];
    // ... similar scoring
    
    // Binary search patterns
    let bs_indicators = [
        (r"lo.*hi|left.*right|low.*high", "binary search bounds"),
        (r"mid\s*=\s*\(.*\+.*\)\s*/\s*2", "midpoint calculation"),
        (r"while.*lo.*<.*hi|while.*left.*<=.*right", "binary search loop"),
        (r"bisect|lower_bound|upper_bound", "library binary search"),
    ];
    // ... similar scoring
    
    // Return highest confidence signal
    signals.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    signals.into_iter().next()
}
```

## 2.4 Intervention Engine

The brain that decides **when** and **how** to intervene:

```rust
struct InterventionEngine {
    problem: Problem,
    known_approaches: Vec<String>,    // Valid approaches for this problem
    optimal_approach: String,         // Best/intended approach
    
    // Thresholds (configurable)
    idle_threshold: Duration,         // Default: 5 minutes
    wrong_approach_min_time: Duration, // Default: 3 minutes on wrong path
    rewrite_threshold: usize,         // Default: 2 major rewrites
    
    // State
    interventions_given: Vec<Intervention>,
    current_confidence: ConfidenceLevel,
    approach_mismatch_detected_at: Option<Instant>,
}

enum ConfidenceLevel {
    /// AI is unsure if user is on wrong track — stay silent
    Observing,
    /// AI suspects wrong approach — show yellow indicator
    Concerned,
    /// AI is fairly sure user is stuck/wrong — show amber, ready to help
    ReadyToHelp,
    /// AI is confident user needs help — show red indicator
    ShouldIntervene,
}

struct InterventionDecision {
    should_intervene: bool,
    level: InterventionLevel,
    reason: String,
    suggested_message: String,
}

impl InterventionEngine {
    fn evaluate(
        &mut self,
        trigger: AnalysisTrigger,
        watcher: &CodeWatcher,
        session: &SessionContext,
    ) -> InterventionDecision {
        match trigger {
            // ── User explicitly asked for help ────────────────────
            AnalysisTrigger::UserRequested => {
                InterventionDecision {
                    should_intervene: true,
                    level: self.next_intervention_level(),
                    reason: "User requested help".into(),
                    suggested_message: String::new(), // LLM will generate
                }
            }
            
            // ── User has been idle for a while ────────────────────
            AnalysisTrigger::UserIdle(duration) => {
                if duration >= self.idle_threshold
                    && watcher.current_code.lines().count() > 5
                {
                    self.current_confidence = ConfidenceLevel::ReadyToHelp;
                    InterventionDecision {
                        should_intervene: true,
                        level: InterventionLevel::Nudge,
                        reason: format!("Idle for {}+ minutes with partial code",
                            duration.as_secs() / 60),
                        suggested_message: String::new(),
                    }
                } else {
                    InterventionDecision::no_action()
                }
            }
            
            // ── Major code rewrite detected ───────────────────────
            AnalysisTrigger::MajorRewrite => {
                let rewrite_count = count_recent_rewrites(watcher);
                
                if rewrite_count >= self.rewrite_threshold {
                    self.current_confidence = ConfidenceLevel::ShouldIntervene;
                    InterventionDecision {
                        should_intervene: true,
                        level: InterventionLevel::Approach,
                        reason: format!("{} major rewrites — user is clearly stuck",
                            rewrite_count),
                        suggested_message: String::new(),
                    }
                } else {
                    // First rewrite — just note it, don't intervene yet
                    self.current_confidence = ConfidenceLevel::Concerned;
                    InterventionDecision::no_action()
                }
            }
            
            // ── Wrong approach detected ───────────────────────────
            AnalysisTrigger::ApproachDetected(signal) => {
                let is_wrong = self.is_approach_wrong(&signal);
                let is_suboptimal = self.is_approach_suboptimal(&signal);
                
                if is_wrong {
                    // Track when we first detected wrong approach
                    if self.approach_mismatch_detected_at.is_none() {
                        self.approach_mismatch_detected_at = Some(Instant::now());
                    }
                    
                    let time_on_wrong_path = self.approach_mismatch_detected_at
                        .unwrap().elapsed();
                    
                    if time_on_wrong_path >= self.wrong_approach_min_time {
                        self.current_confidence = ConfidenceLevel::ShouldIntervene;
                        InterventionDecision {
                            should_intervene: true,
                            level: InterventionLevel::Nudge,
                            reason: format!(
                                "Detected {} approach (confidence: {:.0}%) but problem \
                                 requires {}. User has been on this path for {}min.",
                                signal.technique.name(),
                                signal.confidence * 100.0,
                                self.optimal_approach,
                                time_on_wrong_path.as_secs() / 60,
                            ),
                            suggested_message: String::new(),
                        }
                    } else {
                        // Wrong but haven't been on it long — wait
                        self.current_confidence = ConfidenceLevel::Concerned;
                        InterventionDecision::no_action()
                    }
                } else if is_suboptimal {
                    // Suboptimal but will work → DON'T intervene
                    // Let them solve it their way. Note for post-session review.
                    self.current_confidence = ConfidenceLevel::Observing;
                    InterventionDecision::no_action()
                } else {
                    // On a valid approach — all good
                    self.approach_mismatch_detected_at = None;
                    self.current_confidence = ConfidenceLevel::Observing;
                    InterventionDecision::no_action()
                }
            }
            
            // ── Substantial progress — periodic check ─────────────
            AnalysisTrigger::SubstantialProgress => {
                // Only analyze if we haven't recently
                // This is the "background analysis" pass
                self.current_confidence = ConfidenceLevel::Observing;
                InterventionDecision::no_action()
                // (But update internal state for the indicator color)
            }
        }
    }
    
    /// Determines if detected approach is fundamentally wrong
    fn is_approach_wrong(&self, signal: &ApproachSignal) -> bool {
        // Example: problem needs DP, user is writing greedy
        // Example: problem has negative weights, user is writing Dijkstra
        // Example: problem needs O(n log n), user is writing O(n²) and n=10^6
        
        let detected = &signal.technique;
        let required = &self.optimal_approach;
        
        // Check known incompatibilities
        match (detected.category(), required.as_str()) {
            ("greedy", "dp") if signal.confidence > 0.7 => true,
            ("dijkstra", "bellman_ford") if signal.confidence > 0.7 => true,
            ("brute_force", _) if self.problem.difficulty > 1400 => {
                // Brute force on a 1400+ problem is usually wrong
                // But check constraints — small N might allow it
                !self.problem.allows_brute_force()
            }
            _ => false,
        }
    }
    
    /// Suboptimal but will still pass (e.g., O(n²) when O(n) exists but n≤1000)
    fn is_approach_suboptimal(&self, signal: &ApproachSignal) -> bool {
        // Don't interrupt — let them solve it. Mention in post-session.
        false // Detailed implementation based on time limits + constraints
    }
}
```

## 2.5 Intervention Levels

```rust
enum InterventionLevel {
    /// Subtle direction correction. Minimal rating impact.
    /// "Think about what happens when two intervals overlap."
    Nudge,
    
    /// Reveals the general technique needed. Moderate rating impact.
    /// "This is a segment tree problem. Consider what queries you need to support."
    Approach,
    
    /// Interactive Socratic dialogue. Significant rating impact.
    /// Back-and-forth conversation guiding user to the solution.
    Walkthrough,
    
    /// Full explanation + re-implementation. Maximum rating impact.
    /// "Here's the approach: [...]. Now try implementing it yourself."
    Explain,
}

/// Rating impact of each intervention level
fn intervention_score_multiplier(level: InterventionLevel) -> f64 {
    match level {
        InterventionLevel::Nudge       => 0.90,  // -10% — barely a hint
        InterventionLevel::Approach    => 0.70,  // -30% — technique revealed
        InterventionLevel::Walkthrough => 0.50,  // -50% — heavily guided
        InterventionLevel::Explain     => 0.25,  // -75% — solution explained
    }
}
```

## 2.6 LLM Integration (Configurable Provider)

```rust
/// Unified trait for LLM providers
#[async_trait]
trait LlmProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<String>;
    fn name(&self) -> &str;
    fn supports_streaming(&self) -> bool;
}

/// Claude API provider
struct ClaudeProvider {
    api_key: String,
    model: String,           // "claude-sonnet-4-20250514"
    base_url: String,
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "system": request.system_prompt,
            "messages": [{
                "role": "user",
                "content": request.user_message,
            }]
        });
        
        let response = reqwest::Client::new()
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;
        
        let data: serde_json::Value = response.json().await?;
        Ok(data["content"][0]["text"].as_str().unwrap_or("").to_string())
    }
    
    fn name(&self) -> &str { "Claude" }
    fn supports_streaming(&self) -> bool { true }
}

/// Local LLM provider (Ollama-compatible)
struct OllamaProvider {
    base_url: String,        // "http://localhost:11434"
    model: String,           // "codellama:13b", "deepseek-coder:33b"
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "prompt": format!("{}\n\n{}", request.system_prompt, request.user_message),
            "stream": false,
            "options": {
                "temperature": 0.3,       // Low temp for focused advice
                "num_predict": request.max_tokens,
            }
        });
        
        let response = reqwest::Client::new()
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?;
        
        let data: serde_json::Value = response.json().await?;
        Ok(data["response"].as_str().unwrap_or("").to_string())
    }
    
    fn name(&self) -> &str { "Ollama (Local)" }
    fn supports_streaming(&self) -> bool { true }
}

/// OpenAI-compatible provider (covers many local servers too)
struct OpenAICompatibleProvider {
    base_url: String,
    api_key: Option<String>,
    model: String,
}
// ... similar implementation using /v1/chat/completions
```

## 2.7 Prompt Engineering

The prompt is the most critical piece — it must understand the problem, the user's
code, their approach, and generate *Socratic* guidance, not answers:

```rust
fn build_intervention_prompt(
    problem: &Problem,
    user_code: &str,
    intervention_level: InterventionLevel,
    detected_approach: Option<&ApproachSignal>,
    user_skill_level: u32,
    conversation_history: &[DialogueTurn],
) -> CompletionRequest {
    let system_prompt = format!(r#"
You are a competitive programming coach inside a terminal training app called
Myro. Your job is to help the user learn, NOT to give them answers.

CURRENT PROBLEM:
Title: {title}
Difficulty: {difficulty}
Tags: {tags} (DO NOT reveal these to the user)
Description: {description}

USER'S SKILL LEVEL: ~{skill_level} (on an 800-3500 scale, similar to Codeforces)

INTERVENTION RULES FOR LEVEL "{level}":
{level_rules}

CRITICAL RULES:
- NEVER give the full solution or working code
- NEVER reveal the problem's tags/categories
- Ask questions that guide the user to discover the approach themselves
- Reference what you see in their code specifically
- If they're on the right track but stuck on implementation, help with the
  specific implementation issue, not the whole approach
- Match your language complexity to their skill level
- Keep responses concise — this is a terminal, not an essay
- Use short code snippets (pseudocode preferred) only when necessary
- If user explicitly asks to give up (Explain level), provide a clear
  explanation of the approach, then ask them to re-implement it
"#,
        title = problem.title,
        difficulty = problem.difficulty,
        tags = problem.tags.join(", "),
        description = truncate(&problem.description, 2000),
        skill_level = user_skill_level,
        level = intervention_level.name(),
        level_rules = level_specific_rules(intervention_level),
    );
    
    let user_message = format!(r#"
The user's current code:
```
{code}
```

{approach_context}

{history_context}

Generate a helpful {level_name} response. Remember: guide, don't solve.
"#,
        code = user_code,
        approach_context = match detected_approach {
            Some(signal) => format!(
                "I detected they're trying a {} approach (confidence: {:.0}%).\n\
                 Evidence: {}",
                signal.technique.name(),
                signal.confidence * 100.0,
                signal.evidence.join(", ")
            ),
            None => "I couldn't detect a clear approach from their code.".into(),
        },
        history_context = if conversation_history.is_empty() {
            "This is the first intervention.".into()
        } else {
            format!("Previous dialogue:\n{}",
                conversation_history.iter()
                    .map(|t| format!("{}: {}", t.role, t.message))
                    .collect::<Vec<_>>()
                    .join("\n"))
        },
        level_name = intervention_level.name(),
    );
    
    CompletionRequest {
        system_prompt,
        user_message,
        max_tokens: match intervention_level {
            InterventionLevel::Nudge => 150,
            InterventionLevel::Approach => 300,
            InterventionLevel::Walkthrough => 500,
            InterventionLevel::Explain => 800,
        },
    }
}

fn level_specific_rules(level: InterventionLevel) -> String {
    match level {
        InterventionLevel::Nudge => r#"
NUDGE LEVEL:
- Maximum 1-2 sentences
- Ask a single guiding question
- DO NOT name the technique or algorithm
- Example: "What happens to your solution when the input has duplicate values?"
- Example: "Your current approach is O(n²). What data structure could help?"
"#.into(),
        InterventionLevel::Approach => r#"
APPROACH LEVEL:
- Name the general technique/algorithm category
- Explain WHY their current approach won't work (be specific to their code)
- Give a high-level direction without implementation details
- Example: "Your greedy approach fails because [specific counterexample]. This
  needs dynamic programming. Think about what 'state' you need to track."
"#.into(),
        InterventionLevel::Walkthrough => r#"
WALKTHROUGH LEVEL:
- This is a Socratic dialogue — ask questions, wait for answers
- Break the solution into 3-4 conceptual steps
- Guide through each step with questions
- If user answers correctly, confirm and move to next step
- If user is confused, give one more hint for that step
- You may use pseudocode for complex parts
"#.into(),
        InterventionLevel::Explain => r#"
EXPLAIN LEVEL:
- User has given up — learning is the priority now
- Explain the full approach clearly and concisely
- Explain WHY this approach works and why alternatives don't
- Give pseudocode (not full solution code)
- After explaining, say: "Now try implementing this yourself. I'll be here
  if you get stuck on the implementation."
- The user will re-attempt from this understanding
"#.into(),
    }
}
```

## 2.8 TUI Presentation

The AI coach lives in a collapsible panel at the bottom of the problem view:

```
┌─ Problem: Minimum Path Cover on DAG ──── 12:47 ─────┐
│                                                       │
│  Description...                                       │
│                                                       │
│  [e] Editor  [s] Submit  [d] Discuss with coach      │
│                                                       │
├─── Coach ─────────────── 🟡 Concerned ───────────────┤
│                                                       │
│  I see you're building an adjacency list and running  │
│  DFS. Good instinct to think about graph traversal!   │
│                                                       │
│  But consider: this problem asks for a *minimum*      │
│  set of paths that cover ALL nodes. DFS gives you     │
│  *a* set of paths, but how do you know it's minimum?  │
│                                                       │
│  What mathematical concept relates the minimum        │
│  number of paths to some other graph property?        │
│                                                       │
│  > Your answer: _                                     │
│                                                       │
│  [Enter] Send  [n] Next hint  [g] Give up  [x] Close │
└───────────────────────────────────────────────────────┘
```

**Indicator states:**

```
🟢 Observing    — AI is watching, everything looks fine
🟡 Concerned    — AI suspects you might be going wrong
🟠 Ready        — AI has a suggestion when you want it
🔴 Intervening  — AI is actively recommending you pivot
```

The indicator is always visible in the status bar. The full coach panel opens
only when the user presses `d` or clicks the indicator.

## 2.9 Dialogue State Machine

```rust
enum DialogueState {
    /// No active dialogue — AI is passively observing
    Passive,
    /// AI showed an unsolicited nudge — waiting for user response
    NudgeShown,
    /// Active Socratic dialogue in progress
    InDialogue {
        level: InterventionLevel,
        turns: Vec<DialogueTurn>,
        current_step: u32,
        total_steps: u32,
    },
    /// User gave up — explanation provided, waiting for re-implementation
    ExplainedWaitingForRetry,
}

struct DialogueTurn {
    role: Role,       // Coach or User
    message: String,
    timestamp: Instant,
}

impl DialogueState {
    fn on_user_message(&mut self, message: &str, engine: &mut InterventionEngine) {
        match self {
            Self::NudgeShown => {
                // User engaged with nudge → start dialogue
                *self = Self::InDialogue {
                    level: InterventionLevel::Approach,
                    turns: vec![DialogueTurn {
                        role: Role::User,
                        message: message.to_string(),
                        timestamp: Instant::now(),
                    }],
                    current_step: 1,
                    total_steps: 3,
                };
            }
            Self::InDialogue { turns, .. } => {
                turns.push(DialogueTurn {
                    role: Role::User,
                    message: message.to_string(),
                    timestamp: Instant::now(),
                });
                // LLM generates next response based on full history
            }
            _ => {}
        }
    }
    
    fn on_give_up(&mut self) {
        *self = Self::InDialogue {
            level: InterventionLevel::Explain,
            turns: Vec::new(),
            current_step: 1,
            total_steps: 1,
        };
        // Trigger full explanation generation
    }
}
```

## 2.10 Configuration

```toml
[ai]
# Provider: "claude", "ollama", "openai_compatible"
provider = "claude"

[ai.claude]
api_key = "sk-ant-..."            # Or set ANTHROPIC_API_KEY env var
model = "claude-sonnet-4-20250514"

[ai.ollama]
base_url = "http://localhost:11434"
model = "deepseek-coder:33b"       # Good balance of quality + speed

[ai.openai_compatible]
base_url = "http://localhost:8080/v1"
api_key = ""                        # Optional
model = "local-model"

[ai.behavior]
# Enable passive code watching (vs only on-demand help)
passive_watching = true

# Minimum time on wrong approach before AI intervenes (seconds)
wrong_approach_delay_secs = 180

# Minimum idle time before AI checks in (seconds)
idle_threshold_secs = 300

# Maximum interventions per problem before suggesting "give up"
max_interventions = 4

# Show confidence indicator in status bar
show_indicator = true

# Auto-open coach panel on intervention (vs just update indicator)
auto_open_panel = false
```

---

# 3. Codeforces IDE Integration

## 3.1 Overview

Myro acts as a full CF client — browse problems, code in your editor,
submit directly, see verdicts, participate in live contests. No browser needed.

## 3.2 Authentication

CF supports two auth methods:

```rust
enum CfAuth {
    /// API key + secret (for read-only API calls)
    ApiKey {
        key: String,
        secret: String,
    },
    /// Session-based auth (required for submissions)
    Session {
        handle: String,
        password: String,
        // Managed internally:
        cookies: CookieJar,
        csrf_token: String,
        session_valid_until: DateTime<Utc>,
    },
}

struct CfClient {
    auth: CfAuth,
    http: reqwest::Client,
    rate_limiter: RateLimiter,  // CF allows ~5 req/sec
}

impl CfClient {
    /// Login via web form to get session cookies (needed for submit)
    async fn login(&mut self, handle: &str, password: &str) -> Result<()> {
        // 1. GET /enter to get CSRF token and initial cookies
        let login_page = self.http
            .get("https://codeforces.com/enter")
            .send().await?;
        
        let csrf = extract_csrf_token(&login_page.text().await?)?;
        let cookies = login_page.cookies();
        
        // 2. POST /enter with credentials
        let form = [
            ("csrf_token", csrf.as_str()),
            ("action", "enter"),
            ("handleOrEmail", handle),
            ("password", password),
            ("remember", "on"),
        ];
        
        let response = self.http
            .post("https://codeforces.com/enter")
            .form(&form)
            .send().await?;
        
        // 3. Verify login succeeded (check redirect or response)
        if response.url().path() == "/" {
            // Store session cookies
            self.auth = CfAuth::Session {
                handle: handle.to_string(),
                password: password.to_string(),
                cookies: extract_cookies(&response),
                csrf_token: csrf,
                session_valid_until: Utc::now() + Duration::hours(24),
            };
            Ok(())
        } else {
            Err(anyhow!("Login failed — check credentials"))
        }
    }
    
    /// Re-login if session expired
    async fn ensure_session(&mut self) -> Result<()> {
        match &self.auth {
            CfAuth::Session { session_valid_until, handle, password, .. } => {
                if Utc::now() > *session_valid_until {
                    let h = handle.clone();
                    let p = password.clone();
                    self.login(&h, &p).await?;
                }
                Ok(())
            }
            CfAuth::ApiKey { .. } => {
                Err(anyhow!("Session auth required for submissions"))
            }
        }
    }
}
```

## 3.3 Problem Browsing

```rust
impl CfClient {
    /// Fetch all CF problems (cached locally, incremental update)
    async fn fetch_problemset(&self) -> Result<Vec<CfProblem>> {
        // Official API endpoint — no auth needed
        let url = "https://codeforces.com/api/problemset.problems";
        let response: CfApiResponse<ProblemsetResult> =
            self.http.get(url).send().await?.json().await?;
        
        Ok(response.result.problems.into_iter().map(|p| CfProblem {
            contest_id: p.contest_id,
            index: p.index,
            name: p.name,
            rating: p.rating,
            tags: p.tags,
            solved_count: p.solved_count,
        }).collect())
    }
    
    /// Fetch full problem statement (HTML → terminal-rendered)
    async fn fetch_problem_statement(
        &self,
        contest_id: u32,
        index: &str,
    ) -> Result<ProblemStatement> {
        let url = format!(
            "https://codeforces.com/problemset/problem/{}/{}",
            contest_id, index
        );
        let html = self.http.get(&url).send().await?.text().await?;
        
        // Parse HTML to extract problem statement
        let statement = parse_cf_problem_html(&html)?;
        
        Ok(ProblemStatement {
            description: html_to_terminal(&statement.description),
            input_format: html_to_terminal(&statement.input_format),
            output_format: html_to_terminal(&statement.output_format),
            examples: statement.examples,
            note: statement.note.map(|n| html_to_terminal(&n)),
            time_limit_ms: statement.time_limit_ms,
            memory_limit_mb: statement.memory_limit_mb,
        })
    }
}

/// Convert CF HTML to terminal-renderable text
fn html_to_terminal(html: &str) -> String {
    // Convert common CF HTML patterns:
    // <p>...</p> → paragraph with newlines
    // <pre>...</pre> → code block
    // <b>...</b> → bold (ANSI escape)
    // <i>...</i> → italic
    // <sup>...</sup> → superscript notation
    // MathJax → simplified ASCII math
    // $$$x^2$$$ → x²
    // etc.
    todo!()
}
```

## 3.4 Submission System

```rust
struct SubmissionRequest {
    contest_id: u32,
    problem_index: String,       // "A", "B", "C1", etc.
    language_id: u32,            // CF language IDs (e.g., 73 = GNU G++17)
    source_code: String,
}

/// CF language ID mapping
fn cf_language_id(lang: &str) -> u32 {
    match lang {
        "cpp" | "c++" | "cpp17" => 73,     // GNU G++17 7.3.0
        "cpp20"                  => 89,     // GNU G++20 13.2
        "python" | "python3"     => 31,     // Python 3
        "pypy3"                  => 70,     // PyPy 3-64
        "rust"                   => 75,     // Rust 1.75.0
        "java"                   => 87,     // Java 21
        "go"                     => 32,     // Go 1.19.5
        "kotlin"                 => 83,     // Kotlin 1.9
        _                        => 73,     // Default to C++17
    }
}

impl CfClient {
    async fn submit(&mut self, req: SubmissionRequest) -> Result<SubmissionResult> {
        self.ensure_session().await?;
        
        let CfAuth::Session { csrf_token, cookies, .. } = &self.auth else {
            return Err(anyhow!("Session required"));
        };
        
        // 1. GET the submit page to get fresh CSRF token
        let submit_page_url = format!(
            "https://codeforces.com/contest/{}/submit",
            req.contest_id
        );
        let page = self.http
            .get(&submit_page_url)
            .headers(cookie_headers(cookies))
            .send().await?;
        
        let fresh_csrf = extract_csrf_token(&page.text().await?)?;
        
        // 2. POST the submission
        let form = [
            ("csrf_token", fresh_csrf.as_str()),
            ("action", "submitSolutionFormSubmitted"),
            ("contestId", &req.contest_id.to_string()),
            ("submittedProblemIndex", &req.problem_index),
            ("programTypeId", &req.language_id.to_string()),
            ("source", &req.source_code),
            ("tabSize", "4"),
        ];
        
        let response = self.http
            .post(&format!(
                "https://codeforces.com/contest/{}/submit?csrf_token={}",
                req.contest_id, fresh_csrf
            ))
            .headers(cookie_headers(cookies))
            .form(&form)
            .send().await?;
        
        // 3. Extract submission ID from redirect
        let submission_id = extract_submission_id(&response)?;
        
        // 4. Poll for verdict
        let verdict = self.poll_verdict(submission_id).await?;
        
        Ok(verdict)
    }
    
    /// Poll CF for submission verdict (they don't have webhooks)
    async fn poll_verdict(&self, submission_id: u64) -> Result<SubmissionResult> {
        let url = format!(
            "https://codeforces.com/api/user.status?handle={}&from=1&count=5",
            self.handle()
        );
        
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts > 60 { // 60 * 2s = 2 minute timeout
                return Err(anyhow!("Verdict polling timed out"));
            }
            
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            let response: CfApiResponse<Vec<CfSubmission>> =
                self.http.get(&url).send().await?.json().await?;
            
            if let Some(sub) = response.result.iter()
                .find(|s| s.id == submission_id)
            {
                match sub.verdict.as_deref() {
                    Some("TESTING") | None => continue, // Still judging
                    Some(verdict) => {
                        return Ok(SubmissionResult {
                            id: submission_id,
                            verdict: map_cf_verdict(verdict),
                            time_ms: sub.time_consumed_millis,
                            memory_bytes: sub.memory_consumed_bytes,
                            test_count: sub.passed_test_count,
                            failed_test: if verdict != "OK" {
                                Some(sub.passed_test_count + 1)
                            } else {
                                None
                            },
                        });
                    }
                }
            }
        }
    }
}
```

## 3.5 Verdict Display

```
┌─ Submission #284719352 ──────────────────────────────┐
│                                                       │
│  ✅ Accepted                                         │
│                                                       │
│  Time:   46ms / 2000ms                               │
│  Memory: 3.8MB / 256MB                               │
│  Tests:  48/48 passed                                │
│                                                       │
│  Language: GNU G++17                                  │
│  Submitted: 2 seconds ago                            │
│                                                       │
│  [Enter] Continue  [v] View on CF  [n] Next problem  │
└───────────────────────────────────────────────────────┘

┌─ Submission #284719353 ──────────────────────────────┐
│                                                       │
│  ❌ Wrong Answer on test 3                           │
│                                                       │
│  Time:   15ms / 2000ms                               │
│  Memory: 2.1MB / 256MB                               │
│  Tests:  2/48 passed                                 │
│                                                       │
│  ── Test 3 ──────────────────────────────────────── │
│  Input:                                              │
│  5                                                   │
│  1 3 2 5 4                                           │
│                                                       │
│  Expected: 3                                         │
│  Got:      2                                         │
│                                                       │
│  [r] Retry  [d] Discuss with coach  [v] View on CF   │
└───────────────────────────────────────────────────────┘
```

## 3.6 Live Contest Mode

```rust
struct LiveContest {
    contest_id: u32,
    name: String,
    start_time: DateTime<Utc>,
    duration: Duration,
    problems: Vec<ContestProblem>,
    
    // Live state
    submissions: Vec<SubmissionResult>,
    standings_cache: Option<Standings>,
    last_standings_fetch: Option<Instant>,
}

impl LiveContest {
    /// Full contest workflow
    async fn run(&mut self, client: &mut CfClient, ui: &mut TuiApp) -> Result<()> {
        // 1. Wait for contest start (show countdown)
        self.show_countdown(ui).await;
        
        // 2. Fetch problems as soon as contest starts
        let problems = client.fetch_contest_problems(self.contest_id).await?;
        self.problems = problems;
        
        // 3. Contest loop
        loop {
            let remaining = self.time_remaining();
            if remaining <= Duration::ZERO { break; }
            
            // Show contest interface
            ui.render_contest(self);
            
            match ui.next_event().await {
                Event::SelectProblem(idx) => {
                    ui.show_problem(&self.problems[idx]);
                }
                Event::OpenEditor(idx) => {
                    let file = self.solution_file(idx);
                    open_editor(&file).await?;
                }
                Event::Submit(idx) => {
                    ui.show_submitting();
                    let result = client.submit(SubmissionRequest {
                        contest_id: self.contest_id,
                        problem_index: self.problems[idx].index.clone(),
                        language_id: cf_language_id(&self.language),
                        source_code: std::fs::read_to_string(
                            self.solution_file(idx)
                        )?,
                    }).await?;
                    
                    self.submissions.push(result);
                    ui.show_verdict(&result);
                }
                Event::ViewStandings => {
                    // Rate-limited standings fetch
                    let standings = self.fetch_standings_cached(client).await?;
                    ui.show_standings(&standings);
                }
                Event::Quit => break,
                _ => {}
            }
        }
        
        // 4. Post-contest summary
        self.show_post_contest_summary(ui, client).await?;
        
        Ok(())
    }
    
    /// Generate contest-specific solution file path
    fn solution_file(&self, problem_idx: usize) -> PathBuf {
        let dir = dirs::data_dir().unwrap()
            .join("myro")
            .join("contests")
            .join(self.contest_id.to_string());
        
        std::fs::create_dir_all(&dir).ok();
        dir.join(format!("{}.cpp", self.problems[problem_idx].index))
    }
}
```

**Contest TUI:**

```
┌─ CF Round #987 (Div. 2) ──────── 01:23:45 remaining ┐
│                                                       │
│  #  Problem                  Diff   Status     Time   │
│  A  Array Manipulation       1000   ✅ AC      04:12  │
│  B  Binary String Balance    1300   ✅ AC      18:45  │
│  C  Cycle Decomposition      1600   ❌ WA(3)   ——    │
│  D  DAG Path Cover           1900   ⬜ ——      ——    │
│  E  Euler Tour Queries       2300   ⬜ ——      ——    │
│  F  Flow Network Minimum     2700   ⬜ ——      ——    │
│                                                       │
│  Your rank: ~1,247 / 18,432                          │
│  Points: 1,598  Penalty: 2                           │
│                                                       │
│  [Enter] Open  [e] Editor  [s] Submit                │
│  [r] Standings  [t] Test locally first  [q] End      │
│                                                       │
│  ⚠ AI Coach is DISABLED during live contests         │
└───────────────────────────────────────────────────────┘
```

**Important**: AI coaching is **automatically disabled** during live contests —
this is a competition, not practice.

## 3.7 Local Testing Before CF Submit

Always test locally first to avoid unnecessary WA penalties:

```rust
async fn test_then_submit(
    problem: &ContestProblem,
    solution_path: &Path,
    client: &mut CfClient,
    config: &JudgeConfig,
) -> Result<SubmitDecision> {
    // 1. Run against example test cases locally
    let local_results = local_judge::run(solution_path, &problem.examples, config).await?;
    
    if local_results.all_passed() {
        // All examples pass → safe to submit
        return Ok(SubmitDecision::Submit);
    }
    
    // Show local failures
    println!("⚠ Local testing found issues:");
    for failure in local_results.failures() {
        println!("  Test {}: expected '{}', got '{}'",
            failure.test_num, failure.expected, failure.actual);
    }
    
    // Ask user
    Ok(SubmitDecision::AskUser {
        message: "Local tests failed. Submit anyway? (y/n)".into(),
    })
}
```

## 3.8 Configuration

```toml
[codeforces]
handle = "tourist"
# Password stored in OS keychain, not config file
# Set via: myro cf login

# Preferred language for new solution files
language = "cpp"

# Template for new solution files
template_path = "~/.config/myro/templates/cf_template.cpp"

# Auto-test locally before submitting to CF
auto_test_before_submit = true

# Show standings update interval during contests (seconds)
standings_refresh_secs = 30

# Solution file organization
solutions_dir = "~/.local/share/myro/contests/"
```

**Secure credential storage:**

```rust
fn store_cf_password(handle: &str, password: &str) -> Result<()> {
    // Use OS keychain via keyring crate
    let entry = keyring::Entry::new("myro", handle)?;
    entry.set_password(password)?;
    Ok(())
}

fn get_cf_password(handle: &str) -> Result<String> {
    let entry = keyring::Entry::new("myro", handle)?;
    Ok(entry.get_password()?)
}
```

---

# 4. Stress Test Mode

## 4.1 Concept

Short, intense, contest-like sessions calibrated to your level. Interval
training for competitive programming — all the pressure of a contest
in 30-45 minutes.

## 4.2 Stress Test Formats

```rust
enum StressTestFormat {
    /// Mini contest: 3-4 problems, increasing difficulty, timed
    MiniContest {
        num_problems: u32,         // 3 or 4
        duration_minutes: u32,     // 30 or 45
        difficulty_spread: u32,    // Gap between easiest and hardest
    },
    
    /// Speed round: many easy problems, race against clock
    SpeedRound {
        num_problems: u32,         // 8-10
        duration_minutes: u32,     // 20-30
        max_difficulty_offset: i32, // Problems at or below user rating
    },
    
    /// Weakness blitz: targets your weakest skills under pressure
    WeaknessBlitz {
        num_problems: u32,         // 4-5
        duration_minutes: u32,     // 45
        // Engine auto-selects from weakest skills
    },
    
    /// Topic sprint: all problems from one topic, escalating difficulty
    TopicSprint {
        skill_id: String,          // e.g., "dp.bitmask"
        num_problems: u32,         // 4-5
        duration_minutes: u32,     // 30-40
    },
    
    /// Upsolve challenge: problems slightly above your rating
    UpsolveChallenge {
        num_problems: u32,         // 3
        duration_minutes: u32,     // 60
        difficulty_offset: u32,    // +200 to +400 above rating
    },
}
```

## 4.3 Problem Set Generation

```rust
struct StressTestGenerator {
    problem_db: ProblemDatabase,
    skill_graph: SkillGraph,
    user: UserProfile,
}

impl StressTestGenerator {
    fn generate(&self, format: &StressTestFormat) -> Result<StressTestProblemSet> {
        match format {
            StressTestFormat::MiniContest {
                num_problems,
                duration_minutes,
                difficulty_spread,
            } => {
                self.generate_mini_contest(
                    *num_problems,
                    *duration_minutes,
                    *difficulty_spread,
                )
            }
            StressTestFormat::WeaknessBlitz { num_problems, duration_minutes } => {
                self.generate_weakness_blitz(*num_problems, *duration_minutes)
            }
            // ... other formats
        }
    }
    
    fn generate_mini_contest(
        &self,
        num_problems: u32,
        duration_minutes: u32,
        difficulty_spread: u32,
    ) -> Result<StressTestProblemSet> {
        let user_rating = self.user.global_rating as u32;
        
        // Difficulty distribution mirrors real CF contests
        // For a Div 2 player at rating 1500 with 4 problems:
        //   A: 1200  B: 1400  C: 1600  D: 1800
        let start_diff = user_rating.saturating_sub(300);
        let step = difficulty_spread / (num_problems - 1);
        
        let mut problems = Vec::new();
        let mut used_skills: HashSet<String> = HashSet::new();
        
        for i in 0..num_problems {
            let target_diff = start_diff + (i * step);
            let diff_range = (target_diff.saturating_sub(100))..=(target_diff + 100);
            
            // Find a problem at target difficulty
            // Ensure topic diversity — don't repeat primary skills
            let candidates = self.problem_db.query(ProblemFilter {
                difficulty_range: Some(diff_range),
                exclude_solved: true,
                exclude_skills: Some(&used_skills),
                sources: None,
            });
            
            // Score candidates by difficulty fit and diversity
            let best = candidates.iter()
                .max_by_key(|p| {
                    let diff_fit = 100 - (p.difficulty as i32 - target_diff as i32).unsigned_abs();
                    let diversity = if p.primary_skill_overlaps(&used_skills) { 0 } else { 50 };
                    diff_fit + diversity
                })
                .ok_or(anyhow!("Not enough problems at difficulty {}", target_diff))?;
            
            used_skills.extend(best.skill_ids());
            problems.push(StressTestProblem {
                problem: best.clone(),
                index: (b'A' + i as u8) as char,
                target_difficulty: target_diff,
            });
        }
        
        Ok(StressTestProblemSet {
            format: StressTestFormat::MiniContest {
                num_problems, duration_minutes, difficulty_spread,
            },
            problems,
            duration: Duration::from_secs(duration_minutes as u64 * 60),
            scoring: ScoringMethod::CfStyle,
            created_at: Utc::now(),
        })
    }
    
    fn generate_weakness_blitz(
        &self,
        num_problems: u32,
        duration_minutes: u32,
    ) -> Result<StressTestProblemSet> {
        // 1. Get top N weakest skills
        let weak_skills = self.skill_graph.weakest_skills(num_problems as usize);
        
        // 2. For each weak skill, pick a problem in the stretch zone
        let mut problems = Vec::new();
        for (i, skill) in weak_skills.iter().enumerate() {
            let target_diff = (skill.rating as u32) + 150; // Stretch zone
            
            let candidates = self.problem_db.query(ProblemFilter {
                skill: Some(&skill.id),
                difficulty_range: Some((target_diff - 100)..=(target_diff + 100)),
                exclude_solved: true,
                sources: None,
            });
            
            if let Some(problem) = candidates.first() {
                problems.push(StressTestProblem {
                    problem: problem.clone(),
                    index: (b'A' + i as u8) as char,
                    target_difficulty: target_diff,
                });
            }
        }
        
        Ok(StressTestProblemSet {
            format: StressTestFormat::WeaknessBlitz {
                num_problems, duration_minutes,
            },
            problems,
            duration: Duration::from_secs(duration_minutes as u64 * 60),
            scoring: ScoringMethod::CfStyle,
            created_at: Utc::now(),
        })
    }
}
```

## 4.4 Scoring

```rust
enum ScoringMethod {
    /// CF-style: max points decrease over time, penalty for wrong submissions
    CfStyle,
    /// ICPC-style: solved count + penalty time
    IcpcStyle,
    /// Speed: pure solve count in time limit
    SpeedStyle,
}

impl ScoringMethod {
    fn calculate_score(
        &self,
        problem: &StressTestProblem,
        submissions: &[StressTestSubmission],
        elapsed: Duration,
        total_duration: Duration,
    ) -> u32 {
        match self {
            Self::CfStyle => {
                let max_points = match problem.index {
                    'A' => 500,
                    'B' => 1000,
                    'C' => 1500,
                    'D' => 2000,
                    'E' => 2500,
                    _ => 3000,
                };
                
                let accepted = submissions.iter()
                    .find(|s| s.verdict == Verdict::AC);
                
                if let Some(ac) = accepted {
                    let time_fraction = ac.elapsed.as_secs_f64()
                        / total_duration.as_secs_f64();
                    let time_penalty = (max_points as f64 * 0.5 * time_fraction) as u32;
                    let wa_penalty = submissions.iter()
                        .filter(|s| s.verdict != Verdict::AC)
                        .count() as u32 * 50;
                    
                    max_points.saturating_sub(time_penalty).saturating_sub(wa_penalty)
                        .max(max_points / 4) // Minimum 25% of max
                } else {
                    0
                }
            }
            Self::IcpcStyle => {
                // 1 point per solve, penalty = minutes to AC + 20*WA_count
                let accepted = submissions.iter()
                    .find(|s| s.verdict == Verdict::AC);
                
                if accepted.is_some() {
                    1 // Just a solve flag — penalty tracked separately
                } else {
                    0
                }
            }
            Self::SpeedStyle => {
                // 1 point per AC, no penalty
                if submissions.iter().any(|s| s.verdict == Verdict::AC) {
                    1
                } else {
                    0
                }
            }
        }
    }
}
```

## 4.5 Rating Weight

```rust
/// Stress tests carry more weight than practice, less than real contests
fn stress_test_rating_weight(format: &StressTestFormat) -> f64 {
    match format {
        // Mini contests: closest to real contest conditions
        StressTestFormat::MiniContest { .. } => 1.5,
        
        // Upsolve challenges: hard problems, real pressure
        StressTestFormat::UpsolveChallenge { .. } => 1.5,
        
        // Weakness blitz: targeted training under pressure
        StressTestFormat::WeaknessBlitz { .. } => 1.3,
        
        // Topic sprint: focused but less contest-like
        StressTestFormat::TopicSprint { .. } => 1.2,
        
        // Speed rounds: mostly easy problems, less signal
        StressTestFormat::SpeedRound { .. } => 1.0,
    }
}

/// For comparison — weights across all modes:
///
/// | Mode                  | Weight | Rationale                           |
/// |-----------------------|--------|-------------------------------------|
/// | Practice (with AI)    | 1.0×   | Lowest pressure, help available     |
/// | Speed round           | 1.0×   | Easy problems, less signal          |
/// | Topic sprint          | 1.2×   | Focused pressure                    |
/// | Weakness blitz        | 1.3×   | Targeted + timed                    |
/// | Mini contest          | 1.5×   | Full contest simulation             |
/// | Upsolve challenge     | 1.5×   | Hard problems under time pressure   |
/// | Virtual CF contest    | 2.0×   | Real problems, real format          |
/// | Live CF contest       | 2.5×   | Maximum signal — real stakes        |
/// | Live CF rated contest | 3.0×   | Ultimate ground truth               |
```

## 4.6 Post-Session Analysis

```rust
struct StressTestAnalysis {
    // Overall
    total_score: u32,
    problems_solved: u32,
    problems_attempted: u32,
    total_time: Duration,
    
    // Per-problem breakdown
    problem_analyses: Vec<ProblemAnalysis>,
    
    // Time management insights
    time_analysis: TimeAnalysis,
    
    // Rating impact
    rating_delta: f64,
    skill_deltas: Vec<(String, f64)>,
    
    // Actionable insights
    insights: Vec<String>,
}

struct ProblemAnalysis {
    problem: StressTestProblem,
    solved: bool,
    time_spent: Duration,
    submissions: u32,
    
    // AI-generated analysis (post-session, so no hint penalty)
    approach_used: String,
    optimal_approach: String,
    time_efficiency: TimeEfficiency,
    
    key_insight_missed: Option<String>,
}

enum TimeEfficiency {
    /// Solved faster than expected for this difficulty
    Fast,
    /// Solved in reasonable time
    Normal,
    /// Spent too long — either stuck or inefficient implementation
    Slow { reason: SlowReason },
    /// Didn't solve
    Unsolved,
}

enum SlowReason {
    /// Spent a long time before writing any code (thinking/wrong approach)
    SlowToStart,
    /// Started coding quickly but many rewrites
    ImplementationStruggle,
    /// Many wrong submissions → debugging issues
    DebuggingHeavy,
    /// Steady progress but just hard
    ProblemWasHard,
}

struct TimeAnalysis {
    /// Time from problem open to first meaningful code
    avg_thinking_time: Duration,
    /// Time from first code to first submission
    avg_implementation_time: Duration,
    /// Time spent debugging after first WA
    avg_debugging_time: Duration,
    
    /// Key insight
    bottleneck: TimeBottleneck,
}

enum TimeBottleneck {
    /// "You spend too long thinking before coding"
    ProblemRecognition,
    /// "You recognize problems but implementation is slow"
    Implementation,
    /// "Your code has bugs — consider more careful testing"
    Debugging,
    /// "Your time management is solid — keep it up"
    None,
}
```

**Post-session TUI:**

```
┌─ Stress Test Complete: Mini Contest ─────────────────┐
│                                                       │
│  Score: 3,240 / 5,000    Time: 28:15 / 30:00        │
│  Solved: 3/4    Rating: 1547 → 1562 (+15) ▲          │
│                                                       │
│  ── Problem Breakdown ───────────────────────────── │
│  A  Array Rotation    ✅  03:12  (Fast ⚡)           │
│  B  Binary Balance    ✅  11:24  (Normal)            │
│  C  Cycle Cover       ✅  22:47  (Slow — 2 WAs)     │
│  D  DP on Intervals   ❌  ——     (Not attempted)     │
│                                                       │
│  ── Time Breakdown ──────────────────────────────── │
│  Thinking:        ██████████░░░░░░  38%              │
│  Implementation:  ████████████░░░░  48%              │
│  Debugging:       ████░░░░░░░░░░░░  14%              │
│                                                       │
│  ── Insights ────────────────────────────────────── │
│  • Problem C: You tried greedy first (3 min) before  │
│    switching to DP. Recognizing cycle cover ↔ DP      │
│    pattern faster would save ~3 minutes.              │
│                                                       │
│  • Problem D: At your rating, D was solvable. You    │
│    had 7 min left — consider reading D earlier to     │
│    assess if it's worth attempting.                   │
│                                                       │
│  • Bottleneck: Problem recognition. Your              │
│    implementation speed is good.                      │
│                                                       │
│  [Enter] Done  [d] Detailed analysis  [r] Retry      │
│  [s] Schedule another  [h] History                    │
└───────────────────────────────────────────────────────┘
```

## 4.7 Scheduling & Progression

```rust
struct StressTestScheduler {
    user: UserProfile,
    history: Vec<StressTestResult>,
}

impl StressTestScheduler {
    /// Suggest the next stress test based on history and goals
    fn suggest_next(&self) -> StressTestSuggestion {
        let recent = &self.history[self.history.len().saturating_sub(10)..];
        
        // If user hasn't done a stress test in >3 days, suggest one
        let days_since_last = self.days_since_last_stress_test();
        
        // Analyze patterns
        let avg_solve_rate = recent.iter()
            .map(|r| r.solve_rate()).sum::<f64>() / recent.len() as f64;
        
        let suggestion = if avg_solve_rate > 0.8 {
            // Crushing it → harder format
            StressTestSuggestion {
                format: StressTestFormat::UpsolveChallenge {
                    num_problems: 3,
                    duration_minutes: 60,
                    difficulty_offset: 300,
                },
                reason: "You've been acing mini contests — time to push your ceiling.".into(),
            }
        } else if avg_solve_rate < 0.4 {
            // Struggling → focus on weaknesses at lower pressure
            StressTestSuggestion {
                format: StressTestFormat::TopicSprint {
                    skill_id: self.user.weakest_skill().id.clone(),
                    num_problems: 4,
                    duration_minutes: 40,
                },
                reason: format!(
                    "Let's build up your {} before the next contest.",
                    self.user.weakest_skill().name
                ),
            }
        } else {
            // Normal progression → standard mini contest
            StressTestSuggestion {
                format: StressTestFormat::MiniContest {
                    num_problems: 4,
                    duration_minutes: 45,
                    difficulty_spread: 600,
                },
                reason: "Solid recent performance. Standard contest simulation.".into(),
            }
        };
        
        suggestion
    }
}
```

## 4.8 Configuration

```toml
[stress_test]
# Default format
default_format = "mini_contest"

# Mini contest defaults
mini_contest_problems = 4
mini_contest_duration_minutes = 45
mini_contest_difficulty_spread = 600

# Speed round defaults
speed_round_problems = 8
speed_round_duration_minutes = 25

# AI coaching during stress tests
ai_coaching = false    # Disabled by default — this is a test!

# Post-session AI analysis
ai_post_analysis = true

# Scoring method: "cf", "icpc", "speed"  
scoring = "cf"

# Suggest stress tests if none done in N days
reminder_days = 3
```

---

# Summary: How Everything Connects

```
                    ┌──────────────┐
                    │  CF Import   │──── Time-weighted ────┐
                    │  LC Import   │   Glicko-2 bootstrap  │
                    └──────────────┘                        │
                                                           ▼
┌───────────────┐   ┌──────────────┐   ┌──────────────────────┐
│  Practice     │   │  Stress Test │   │   Adaptive Engine    │
│  Mode         │   │  Mode        │   │                      │
│               │   │              │   │  Skill Graph         │
│  AI watches   │   │  No AI help  │   │  Glicko-2 Ratings   │
│  your code    │   │  Timed       │   │  Problem Recommender │
│  Intervenes   │   │  Scored      │   │                      │
│  when stuck   │   │  Analyzed    │   └──────────┬───────────┘
│               │   │  post-session│              │
│  Weight: 1x   │   │              │              │
│               │   │  Weight: 1.5x│       Feeds back into
└───────┬───────┘   └──────┬───────┘              │
        │                  │                      │
        └──────────────────┼──────────────────────┘
                           │
        ┌──────────────────┼──────────────────────┐
        │                  │                      │
        ▼                  ▼                      ▼
┌───────────────┐   ┌──────────────┐   ┌──────────────────┐
│  CF Live      │   │  CF Virtual  │   │  LC Contest      │
│  Contest      │   │  Contest     │   │  (on site)       │
│               │   │              │   │                  │
│  Full IDE     │   │  Past rounds │   │  Results imported│
│  No AI        │   │  Full timer  │   │  via GraphQL API │
│  Submit to CF │   │  Local judge │   │                  │
│               │   │              │   │  Weight: 2.5x    │
│  Weight: 3x   │   │  Weight: 2x  │   │                  │
└───────────────┘   └──────────────┘   └──────────────────┘
```

*All modes feed results back into the adaptive engine,
which continuously refines its model of your skills and
serves increasingly targeted training.*
