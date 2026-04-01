# Myro — TUI Wireframe Designs

> Every screen, every state, every interaction.
> Terminal dimensions assumed: 120×40 (adapts down to 80×24 minimum)

---

## Design Principles

1. **Information density without clutter** — every pixel earns its place
2. **Consistent navigation** — sidebar always visible, keybindings always shown
3. **Color as meaning** — green=good, yellow=caution, red=problem, cyan=info
4. **Progressive disclosure** — summaries first, details on demand
5. **Vim-like muscle memory** — j/k navigation, / search, modal editing

## Color Palette

```
┌─ Color Assignments ──────────────────────────────────┐
│                                                       │
│  Borders & chrome    │  gray (dimmed)                 │
│  Active selection    │  cyan bg, black fg              │
│  Headers & titles    │  bold white                    │
│  Rating text         │  colored by CF tier             │
│  Accepted / success  │  green                         │
│  Wrong / error       │  red                           │
│  Warning / caution   │  yellow                        │
│  Muted / secondary   │  dark gray                     │
│  Streak / fire       │  orange/yellow                 │
│  AI coach indicator  │  green/yellow/orange/red        │
│  Keybinding hints    │  cyan brackets, white keys      │
│                                                       │
│  CF Rating Colors:                                    │
│  Newbie (<1200)      │  gray                          │
│  Pupil (1200-1399)   │  green                         │
│  Specialist (1400+)  │  cyan                          │
│  Expert (1600+)      │  blue                          │
│  Cand.Master (1900+) │  magenta                       │
│  Master (2100+)      │  yellow                        │
│  Grandmaster (2400+) │  red                           │
└───────────────────────────────────────────────────────┘
```

---

## 1. Global Layout

All screens share this frame:

```
┌─ myro ─────────────────────────────────────── v0.1.0 ─┐
│                                                              │
│  ┌────────────┐  ┌────────────────────────────────────────┐ │
│  │            │  │                                        │ │
│  │  Sidebar   │  │          Main Content Area             │ │
│  │            │  │                                        │ │
│  │  Nav +     │  │          (varies by screen)            │ │
│  │  Quick     │  │                                        │ │
│  │  Stats     │  │                                        │ │
│  │            │  │                                        │ │
│  │            │  │                                        │ │
│  │            │  │                                        │ │
│  │            │  │                                        │ │
│  └────────────┘  └────────────────────────────────────────┘ │
│                                                              │
│  ┌──────────────────────────────────────────────────────── │
│  │ Status Bar: keybindings │ AI indicator │ time │ mode     │
│  └──────────────────────────────────────────────────────── │
└──────────────────────────────────────────────────────────────┘
```

### Sidebar (always visible)

```
┌──────────────┐
│  ⚡ myro │
│              │
│  Rating:1547 │  ← colored by tier (cyan = Specialist)
│  ▁▂▃▄▅▆▇█   │  ← 30-day sparkline
│  🔥 12 days  │  ← streak
│              │
│──────────────│
│              │
│ ❯ Dashboard  │  ← active item highlighted
│   Problems   │
│   Train      │
│   Stress     │
│   Contest    │
│   Stats      │
│   Profile    │
│              │
│──────────────│
│              │
│  3 due today │  ← from spaced repetition / adaptive
│  CF #987 in  │
│    2h 14m    │  ← upcoming contest countdown
│              │
│──────────────│
│              │
│  [1-7] Nav   │
│  [/] Search  │
│  [?] Help    │
│  [q] Quit    │
│              │
└──────────────┘
```

### Status Bar (always visible)

```
 [j/k] Navigate  [Enter] Select  [/] Search  │  🟢 Coach  │  14:23  │  Practice
```

During a contest:

```
 [e] Editor  [s] Submit  [t] Test  │  ⚠ Coach OFF  │  01:23:45 remaining  │  CF #987
```

---

## 2. Dashboard (Home Screen)

The first thing you see. Quick overview + actionable next steps.

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Dashboard                                                        │
│              │                                                                    │
│  Rating:1547 │  ┌─ Your Progress ──────────────────────────────────────────────┐  │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 12 days  │  │  Rating: 1547  Specialist                +23 this week      │  │
│              │  │                                                              │  │
│──────────────│  │   1300 ┤                                                     │  │
│              │  │        │     ╭──╮                                             │  │
│ ❯ Dashboard  │  │   1400 ┤    ╭╯  ╰╮  ╭─╮     ╭╮                              │  │
│   Problems   │  │        │  ╭─╯    ╰──╯ ╰╮   ╭╯╰╮    ╭─╮                     │  │
│   Train      │  │   1500 ┤╭─╯             ╰───╯  ╰────╯ ╰──╮  ╭──╮           │  │
│   Stress     │  │        ││                                  ╰──╯  ╰──█        │  │
│   Contest    │  │   1600 ┤│                                            █        │  │
│   Stats      │  │        └┬──────┬──────┬──────┬──────┬──────┬──────┬──────┤   │  │
│   Profile    │  │        Jan    Feb    Mar    Apr    May    Jun    Jul    Aug   │  │
│              │  │                                                              │  │
│──────────────│  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│  3 due today │  ┌─ Weakest Skills ──────────────┐  ┌─ Quick Actions ───────────┐ │
│  CF #987 in  │  │                                │  │                           │ │
│    2h 14m    │  │  dp.bitmask     ██░░░░  1120  │  │  [t] Start training       │ │
│              │  │  math.ntheory   █░░░░░   980  │  │  [s] Stress test          │ │
│──────────────│  │  graph.flow     ██░░░░  1100  │  │  [c] Upcoming contest     │ │
│              │  │  strings.suffix █░░░░░  1180  │  │  [r] Review due problems  │ │
│  [1-7] Nav   │  │                                │  │  [i] Import history       │ │
│  [/] Search  │  └────────────────────────────────┘  └───────────────────────────┘ │
│  [?] Help    │                                                                    │
│  [q] Quit    │  ┌─ Recent Activity ─────────────────────────────────────────────┐ │
│              │  │  Today     5 solved   avg 1420   +12 rating   dp, graphs      │ │
│              │  │  Yesterday 3 solved   avg 1380    +8 rating   binary search   │ │
│              │  │  2 days    7 solved   avg 1510   +18 rating   contest sim     │ │
│              │  └───────────────────────────────────────────────────────────────┘ │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [t] Train  [s] Stress  [c] Contest  [r] Review  │  🟢 Coach  │  14:23  │ Home   │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Problem Browser

Searchable, filterable, sortable list of all problems.

### 3a. Problem List View

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Problems                                          2,847 loaded   │
│              │                                                                    │
│  Rating:1547 │  ┌─ Filters ────────────────────────────────────────────────────┐  │
│  ▁▂▃▄▅▆▇█   │  │ Source: [All] CF LC  │  Diff: 800 ━━━━●━━━━━━━━ 2400       │  │
│  🔥 12 days  │  │ Tags: [any]          │  Status: [All] Unsolved Solved       │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│──────────────│                                                                    │
│              │  ┌──────────────────────────────────────────────────────────────┐  │
│   Dashboard  │  │  #     Source  Title                        Diff  Tags    ✓  │  │
│ ❯ Problems   │  │ ─────────────────────────────────────────────────────────── │  │
│   Train      │  │  1     CF     Two Sum                      800   arrays  ✅ │  │
│   Stress     │  │  2     CF     Watermelon                   800   math   ✅ │  │
│   Contest    │  │  3     LC     Add Two Numbers              1200  linked  ✅ │  │
│   Stats      │  │❯ 4     CF     Bitmask DP Practice          1400  dp      ⬜ │  │
│   Profile    │  │  5     LC     Longest Substring            1300  string  ⬜ │  │
│              │  │  6     CF     Dijkstra Paths               1600  graph   ⬜ │  │
│──────────────│  │  7     CF     Segment Tree Beats           2200  ds      ⬜ │  │
│              │  │  8     LC     Merge K Sorted Lists         1500  heap    ✅ │  │
│  3 due today │  │  9     CF     Euler Tour on Trees          1800  graph   ⬜ │  │
│  CF #987 in  │  │  10    LC     Trapping Rain Water          1600  stack   ⬜ │  │
│    2h 14m    │  │  11    CF     Convex Hull Trick            2000  dp      ⬜ │  │
│              │  │  12    CF     String Hashing               1400  string  ✅ │  │
│──────────────│  │                                                             │  │
│              │  │  ─── Page 1/237 ──────────────────────────────── 2,847 ──── │  │
│  [1-7] Nav   │  └──────────────────────────────────────────────────────────────┘  │
│  [/] Search  │                                                                    │
│  [?] Help    │  ┌─ Preview ────────────────────────────────────────────────────┐  │
│  [q] Quit    │  │  CF 1400 • Bitmask DP Practice • dp, bitmasks               │  │
│              │  │  Given n items (n ≤ 20) with weights and values, find the    │  │
│              │  │  maximum value subset where total weight ≤ W...              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [Enter] Open  [e] Editor  [/] Search  [f] Filter  [s] Sort  │  🟢  │  Problems  │
└───────────────────────────────────────────────────────────────────────────────────┘
```

### 3b. Problem Detail View

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  CF 1400B · Bitmask DP Practice                                   │
│              │                                                                    │
│  Rating:1547 │  Difficulty: 1400           Time Limit: 2s                         │
│  ▁▂▃▄▅▆▇█   │  Source: Codeforces         Memory Limit: 256MB                    │
│  🔥 12 days  │  Tags: hidden (toggle [t])  Solved by: 12,847                      │
│              │                                                                    │
│──────────────│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│              │                                                                    │
│   Dashboard  │  You are given n items, each with a weight wᵢ and value vᵢ.       │
│   Problems   │  Find a subset of items with maximum total value such that the     │
│ ❯  ← detail  │  total weight does not exceed W.                                   │
│   Train      │                                                                    │
│   Stress     │  Additionally, some pairs of items are incompatible — you cannot   │
│   Contest    │  select both items in a pair. Given that n ≤ 20, find the optimal  │
│   Stats      │  subset.                                                           │
│   Profile    │                                                                    │
│              │  Input                                                             │
│──────────────│  The first line contains two integers n and W (1 ≤ n ≤ 20,        │
│              │  1 ≤ W ≤ 10⁹). The next n lines each contain wᵢ and vᵢ.          │
│  3 due today │  The next line contains m — the number of incompatible pairs.      │
│              │  The next m lines each contain two integers aⱼ and bⱼ.             │
│              │                                                                    │
│──────────────│  Output                                                            │
│              │  Print the maximum total value achievable.                          │
│  [1-7] Nav   │                                                                    │
│  [/] Search  │  ┌─ Example 1 ──────────────────┐  ┌─ Example 2 ────────────────┐ │
│  [?] Help    │  │ Input:        │ Output:       │  │ Input:       │ Output:      │ │
│  [q] Quit    │  │ 3 10          │ 8             │  │ 4 15         │ 13           │ │
│              │  │ 5 3           │               │  │ 7 5          │              │ │
│              │  │ 4 5           │               │  │ 3 4          │              │ │
│              │  │ 6 7           │               │  │ 5 6          │              │ │
│              │  │ 1             │               │  │ 4 3          │              │ │
│              │  │ 1 3           │               │  │ 2            │              │ │
│              │  │               │               │  │ 1 2          │              │ │
│              │  │               │               │  │ 3 4          │              │ │
│              │  └───────────────┴───────────────┘  └──────────────┴──────────────┘ │
│              │                                                                    │
│              │  Note: In example 1, items 1 and 3 are incompatible. The best      │
│              │  choice is items 1 and 2 (weight 9 ≤ 10, value 8).                │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [e] Editor  [s] Submit  [t] Tags  [h] Hint  [d] Coach  [Esc] Back  │  🟢  │ Prob│
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Training Session

### 4a. Session Start

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Start Training Session                                            │
│              │                                                                    │
│  Rating:1547 │  ┌─ Session Type ───────────────────────────────────────────────┐  │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 12 days  │  │  ❯ ⚡ Adaptive       Engine picks optimal problems for you   │  │
│              │  │    🎯 Focus Skill    Choose a specific skill to drill        │  │
│──────────────│  │    📋 Review         Re-test problems you've struggled with  │  │
│              │  │    🎲 Random         Random unsolved problems                │  │
│   Dashboard  │  │                                                              │  │
│   Problems   │  └──────────────────────────────────────────────────────────────┘  │
│ ❯ Train      │                                                                    │
│   Stress     │  ┌─ Session Settings ───────────────────────────────────────────┐  │
│   Contest    │  │                                                              │  │
│   Stats      │  │  Duration:     [30 min]  45 min   60 min   No limit         │  │
│   Profile    │  │  Difficulty:   Auto (engine decides)                         │  │
│              │  │  AI Coach:     [On — passive watching]                       │  │
│──────────────│  │  Show tags:    Off (realistic practice)                      │  │
│              │  │  Show diff:    [On]                                          │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│──────────────│  ┌─ Engine Recommendation ──────────────────────────────────────┐  │
│              │  │                                                              │  │
│  [1-7] Nav   │  │  Based on your recent performance, I recommend focusing on: │  │
│  [/] Search  │  │                                                              │  │
│  [?] Help    │  │    dp.bitmask     1120  (400 below your rating — priority)  │  │
│  [q] Quit    │  │    math.ntheory    980  (high uncertainty — needs testing)   │  │
│              │  │    graph.flow     1100  (not practiced in 18 days)           │  │
│              │  │                                                              │  │
│              │  │  Estimated problems this session: 4-6                        │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │                    [ Enter: Start Session ]                        │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [Enter] Start  [Tab] Change setting  [Esc] Back          │  🟢  │  14:23  │ Train│
└───────────────────────────────────────────────────────────────────────────────────┘
```

### 4b. Training Session — Active (Problem View with Coach)

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Training Session                    Problem 2/~5    ⏱ 12:47      │
│              │                                                                    │
│  Rating:1547 │  ┌─ CF 1350C · Bitmask Subset Sum ────────────── Diff: 1350 ───┐  │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 12 days  │  │  Given an array of n integers (n ≤ 20), determine if there   │  │
│              │  │  exists a subset whose sum equals exactly S. Print "YES" or   │  │
│──────────────│  │  "NO", and if YES, print the subset.                         │  │
│              │  │                                                              │  │
│   Session    │  │  Input: First line: n, S. Second line: n integers aᵢ.        │  │
│   Progress:  │  │  Output: "YES"/"NO". If YES, next line: subset elements.     │  │
│              │  │                                                              │  │
│  1 ✅ 04:12  │  │  ┌─ Example ────────────┐                                    │  │
│  2 ⏳ 12:47  │  │  │ In:  4 6             │  Note: n ≤ 20, so consider         │  │
│  3 ⬜ ──:──  │  │  │      2 3 5 1         │  approaches that use the           │  │
│  4 ⬜ ──:──  │  │  │ Out: YES             │  small constraint on n.            │  │
│  ~ ⬜ ──:──  │  │  │      1 2 3           │                                    │  │
│              │  │  └──────────────────────┘                                    │  │
│──────────────│  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│  Rating:     │                                                                    │
│  +12 so far  │  ┌─ Your Solution ──────────────────── solution.cpp ────────────┐  │
│              │  │                                                              │  │
│──────────────│  │  #include <bits/stdc++.h>                                    │  │
│              │  │  using namespace std;                                         │  │
│  Skill focus:│  │                                                              │  │
│  dp.bitmask  │  │  int main() {                                                │  │
│              │  │      int n, s;                                                │  │
│              │  │      cin >> n >> s;                                           │  │
│              │  │      vector<int> a(n);                                        │  │
│              │  │      for (int i = 0; i < n; i++) cin >> a[i];                │  │
│              │  │      // ... (last saved 30s ago)                              │  │
│              │  │  }                                                            │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┼────────────────────────────────────────────────────────────────────┤
│              │ [e] Editor  [s] Submit  [h] Hint  [d] Coach  [n] Skip  [x] End   │
│              │                                                          🟢 Coach │
└──────────────┴────────────────────────────────────────────────────────────────────┘
```

### 4c. Training — AI Coach Panel Active

When user presses `d` or coach indicator turns 🟠/🔴:

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Training Session                    Problem 2/~5    ⏱ 18:32      │
│              │                                                                    │
│  Rating:1547 │  ┌─ CF 1350C · Bitmask Subset Sum ────────── Diff: 1350 ───────┐  │
│              │  │                                                              │  │
│  ▁▂▃▄▅▆▇█   │  │  (Problem description scrolled up — press [p] to review)    │  │
│  🔥 12 days  │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│──────────────│                                                                    │
│              │  ┌─ Your Code (last saved) ─────────────────────────────────────┐  │
│   Session    │  │  for (int mask = 0; mask < (1 << n); mask++) {              │  │
│   Progress:  │  │      int sum = 0;                                            │  │
│              │  │      for (int i = 0; i < n; i++)                             │  │
│  1 ✅ 04:12  │  │          if (mask & (1 << i)) sum += a[i];                  │  │
│  2 ⏳ 18:32  │  │      if (sum == s) { /* print subset */ }                   │  │
│  3 ⬜ ──:──  │  │  }                                                           │  │
│  4 ⬜ ──:──  │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│──────────────│                                                                    │
│              │  ┌─ Coach ────────────────────────────────────── 🟡 Concerned ──┐  │
│  Rating:     │  │                                                              │  │
│  +12 so far  │  │  Good — you're iterating over all 2ⁿ subsets with bitmasks, │  │
│              │  │  and your logic for checking the sum is correct.             │  │
│──────────────│  │                                                              │  │
│              │  │  Your current approach is O(2ⁿ · n) which works fine here   │  │
│  Skill focus:│  │  since n ≤ 20. But I notice you're only tracking the sum.   │  │
│  dp.bitmask  │  │                                                              │  │
│              │  │  The problem also asks you to print the actual subset        │  │
│              │  │  elements. How will you reconstruct which items are in the   │  │
│              │  │  optimal subset?                                             │  │
│              │  │                                                              │  │
│              │  │  > _                                                         │  │
│              │  │                                                              │  │
│              │  │  [Enter] Reply  [n] Next hint  [g] Give up  [x] Close       │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ Type response...  [Enter] Send  [Esc] Close coach  │  🟡 Coach active  │  Train  │
└───────────────────────────────────────────────────────────────────────────────────┘
```

### 4d. Training — Submission Result (Accepted)

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Training Session                    Problem 2/~5    ⏱ 22:15      │
│              │                                                                    │
│  Rating:1547 │  ┌─ ✅ Accepted ───────────────────────────────────────────────┐   │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 12 days  │  │      ██████╗  ██████╗                                        │  │
│              │  │      ██╔══██╗██╔════╝    Problem: Bitmask Subset Sum         │  │
│──────────────│  │      ██████╔╝██║         Difficulty: 1350                    │  │
│              │  │      ██╔══██╗██║         Solve time: 22:15                   │  │
│              │  │      ██║  ██║╚██████╗    Attempts: 1                         │  │
│              │  │      ╚═╝  ╚═╝ ╚═════╝   Hints used: 0                       │  │
│              │  │                                                              │  │
│              │  │  Tests: 12/12 passed                                         │  │
│              │  │  Time:  46ms / 2000ms                                        │  │
│              │  │  Memory: 3.8MB / 256MB                                       │  │
│              │  │                                                              │  │
│              │  │  ── Rating Impact ───────────────────────────────────────── │  │
│              │  │                                                              │  │
│              │  │  Global:     1547 → 1559 (+12) ▲                             │  │
│              │  │  dp.bitmask: 1120 → 1178 (+58) ▲▲  Big gain!                │  │
│              │  │                                                              │  │
│              │  │  ── Coach Note ──────────────────────────────────────────── │  │
│              │  │                                                              │  │
│              │  │  Clean solution! Your bitmask enumeration was correct.       │  │
│              │  │  For future reference: when n ≤ 20 and you need subsets,     │  │
│              │  │  bitmask enumeration in O(2ⁿ) is the go-to technique.       │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │              [ Enter: Next Problem ]    [ x: End Session ]         │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [Enter] Next problem  [x] End session  [v] View solution     │  🟢  │  Train    │
└───────────────────────────────────────────────────────────────────────────────────┘
```

### 4e. Training — Session Summary

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Session Complete                                                  │
│              │                                                                    │
│  Rating:1559 │  ┌─ Summary ────────────────────────────────────────────────────┐  │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 13 days  │  │  Duration: 47 minutes          Problems: 4/5 solved (80%)   │  │
│              │  │  Avg difficulty: 1380           Avg solve time: 11:42        │  │
│──────────────│  │                                                              │  │
│              │  │  Rating: 1547 → 1559 (+12)                                   │  │
│              │  │  ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▂▂▂▃▃▃▃▄▄▅▅▅▆▆▇█  30-day trend           │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Problem Breakdown ──────────────────────────────────────────┐  │
│              │  │                                                              │  │
│              │  │  #  Problem              Diff  Time    Verdict  Δ Rating     │  │
│              │  │  ─────────────────────────────────────────────────────────── │  │
│              │  │  1  Array Partitioning   1100  04:12   ✅ AC     +3         │  │
│              │  │  2  Bitmask Subset Sum   1350  22:15   ✅ AC    +12         │  │
│              │  │  3  Number Theory GCD    1200  08:33   ✅ AC     +5         │  │
│              │  │  4  Flow Network Basic   1400  ──:──   ❌ WA     -2         │  │
│              │  │  5  Binary Search Ans    1300  12:42   ✅ AC     +6         │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Skill Deltas ─────────────────┐  ┌─ Insights ──────────────┐  │
│              │  │                                 │  │                         │  │
│              │  │  dp.bitmask  1120→1178 (+58) ▲▲│  │  Your bitmask DP       │  │
│              │  │  math.nthry   980→1010 (+30) ▲ │  │  jumped +58 — great    │  │
│              │  │  graph.flow  1100→1092  (-8) ▼ │  │  progress. The flow    │  │
│              │  │  search.bin  1340→1355 (+15) ▲ │  │  problem exposed a     │  │
│              │  │                                 │  │  gap in capacity       │  │
│              │  │  🔥 Streak: 13 days             │  │  reasoning — review    │  │
│              │  │                                 │  │  max-flow concepts.    │  │
│              │  └─────────────────────────────────┘  └─────────────────────────┘  │
│              │                                                                    │
│              │     [ Enter: New Session ]  [ s: Stats ]  [ q: Home ]             │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [Enter] New session  [s] Full stats  [q] Home              │  🟢  │  Summary    │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Stress Test Mode

### 5a. Stress Test Selection

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Stress Test                                                       │
│              │                                                                    │
│  Rating:1559 │  Choose your challenge:                                            │
│  ▁▂▃▄▅▆▇█   │                                                                    │
│  🔥 13 days  │  ┌──────────────────────────────────────────────────────────────┐  │
│              │  │                                                              │  │
│──────────────│  │  ❯ ⚔ Mini Contest        4 problems · 45 min · CF scoring   │  │
│              │  │                           A(1200) B(1400) C(1600) D(1800)    │  │
│   Dashboard  │  │                                                              │  │
│   Problems   │  │    💨 Speed Round         8 problems · 25 min · solve count  │  │
│   Train      │  │                           All ≤1400 · race the clock         │  │
│ ❯ Stress     │  │                                                              │  │
│   Contest    │  │    🎯 Weakness Blitz      4 problems · 45 min · CF scoring   │  │
│   Stats      │  │                           dp.bitmask, math.ntheory,          │  │
│   Profile    │  │                           graph.flow, strings.suffix         │  │
│              │  │                                                              │  │
│──────────────│  │    📚 Topic Sprint        5 problems · 30 min               │  │
│              │  │                           Choose: [dp] [graph] [string] ...  │  │
│              │  │                                                              │  │
│              │  │    🏔 Upsolve Challenge    3 problems · 60 min               │  │
│              │  │                           +200 to +400 above your rating     │  │
│              │  │                                                              │  │
│──────────────│  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Recent Stress Tests ────────────────────────────────────────┐  │
│              │  │  2 days ago   Mini Contest   3/4 solved   Score: 3240   +15  │  │
│              │  │  5 days ago   Weakness Blitz 2/4 solved   Score: 1800   +8   │  │
│              │  │  1 week ago   Speed Round    7/8 solved   Score: 7      +3   │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [Enter] Start  [j/k] Select  [Esc] Back                │  ⚠ No AI  │  Stress    │
└───────────────────────────────────────────────────────────────────────────────────┘
```

### 5b. Stress Test — In Progress

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  ⚔ Mini Contest                              ⏱ 31:15 remaining    │
│              │                                                                    │
│  Rating:1559 │  ┌─ Scoreboard ─────────────────────────────────────────────────┐  │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 13 days  │  │  #  Problem                 Diff   Score    Status    Time   │  │
│              │  │  ────────────────────────────────────────────────────────── │  │
│──────────────│  │  A  Two Pointer Basics       1200   480/500  ✅ AC    03:45  │  │
│              │  │  B  Interval Scheduling      1400   ···/1000 ⏳ ···   ···    │  │
│              │  │❯ C  Tree DP with Rerooting   1600   ···/1500 ⬜ ···   ···    │  │
│   ┌────────┐ │  │  D  Persistent Segment Tree  1800   ···/2000 ⬜ ···   ···    │  │
│   │CONTEST │ │  │                                                              │  │
│   │ MODE   │ │  │  Total Score: 480 / 5000          Penalty: 0                │  │
│   └────────┘ │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│──────────────│                                                                    │
│              │  ┌─ C · Tree DP with Rerooting ────────────── Diff: 1600 ──────┐  │
│   No AI      │  │                                                              │  │
│   coaching   │  │  You are given a tree of n nodes. For each node, find the    │  │
│   during     │  │  sum of distances to all other nodes. Print n integers.      │  │
│   stress     │  │                                                              │  │
│   tests.     │  │  Input: n, then n-1 edges.                                  │  │
│              │  │  Constraints: 1 ≤ n ≤ 2·10⁵                                 │  │
│              │  │                                                              │  │
│──────────────│  │  ┌─ Example ──────────────────────┐                          │  │
│              │  │  │ In:  5           Out: 6 7 8 9 8│                          │  │
│              │  │  │      1 2                       │                          │  │
│              │  │  │      1 3                       │                          │  │
│              │  │  │      2 4                       │                          │  │
│              │  │  │      2 5                       │                          │  │
│              │  │  └────────────────────────────────┘                          │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [e] Editor  [s] Submit  [t] Test locally  [Tab] Switch problem  │  31:15  │ Stress│
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. CF Contest Mode

### 6a. Contest Lobby

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Codeforces Contests                                               │
│              │                                                                    │
│  Rating:1559 │  ┌─ Upcoming ───────────────────────────────────────────────────┐  │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 13 days  │  │  ❯ CF Round #987 (Div. 2)      Feb 20, 19:35 UTC   in 2d 5h │  │
│              │  │    Educational CF Round 178     Feb 23, 14:00 UTC   in 5d 0h │  │
│──────────────│  │    CF Round #988 (Div. 1+2)    Feb 27, 19:35 UTC   in 9d 5h │  │
│              │  │                                                              │  │
│   Dashboard  │  │  [Enter] Register  [r] Set reminder  [n] Notify me           │  │
│   Problems   │  └──────────────────────────────────────────────────────────────┘  │
│   Train      │                                                                    │
│   Stress     │  ┌─ Virtual Contests ───────────────────────────────────────────┐  │
│ ❯ Contest    │  │                                                              │  │
│   Stats      │  │  Replay any past CF round as if it were live.               │  │
│   Profile    │  │                                                              │  │
│              │  │  Recent rounds:                                              │  │
│──────────────│  │    CF #986 (Div. 2)   Feb 13   A-F   2hr                    │  │
│              │  │    CF #985 (Div. 2)   Feb 10   A-E   2hr                    │  │
│              │  │    CF #984 (Div. 1)   Feb 7    A-F   2.5hr                  │  │
│              │  │    Edu CF #177        Feb 4    A-E   2hr                    │  │
│              │  │                                                              │  │
│              │  │  [v] Start virtual  [/] Search rounds  [h] History          │  │
│──────────────│  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Your CF Contest History ─────────────────────────────────────┐ │
│              │  │  #986  Rank: 1247/18432  Solved: A,B,C  Δ Rating: +23       │ │
│              │  │  #984  Rank: 892/12105   Solved: A,B,C,D Δ Rating: +47      │ │
│              │  │  #981  Rank: 2105/15822  Solved: A,B    Δ Rating: -15       │ │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [Enter] Select  [v] Virtual  [/] Search  [Esc] Back    │  ⚠ No AI  │  Contest   │
└───────────────────────────────────────────────────────────────────────────────────┘
```

### 6b. Live CF Contest — In Progress

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  🔴 LIVE · CF Round #987 (Div. 2)            ⏱ 01:23:45 remaining │
│              │                                                                    │
│  CF: 1547    │  ┌─ Problems ───────────────────────────────────────────────────┐  │
│              │  │                                                              │  │
│  ┌────────┐  │  │  #   Title                        Score     Status    Time   │  │
│  │ LIVE   │  │  │  ──────────────────────────────────────────────────────────│  │
│  │CONTEST │  │  │  A   Array Manipulation           468/500   ✅ AC    04:12  │  │
│  └────────┘  │  │  B   Binary String Balance        872/1000  ✅ AC    18:45  │  │
│              │  │❯ C   Cycle Decomposition          ···/1500  ❌ WA(3) ──:──  │  │
│──────────────│  │  D   DAG Path Cover               ···/2000  ⬜ ───   ──:──  │  │
│              │  │  E   Euler Tour Queries            ···/2500  ⬜ ───   ──:──  │  │
│  AI: OFF     │  │  F   Flow Network Minimum         ···/3000  ⬜ ───   ──:──  │  │
│              │  │                                                              │  │
│  Rank:       │  │  Total: 1340    Penalty: 2                                  │  │
│  ~1,247 /    │  │                                                              │  │
│  18,432      │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│──────────────│  ┌─ C · Cycle Decomposition ─────────────────────── 1500 ──────┐  │
│              │  │                                                              │  │
│  Submissions:│  │  Given a directed graph with n vertices and m edges where   │  │
│  A: ✅ (1)   │  │  each vertex has exactly one outgoing edge, decompose the   │  │
│  B: ✅ (1)   │  │  graph into the minimum number of vertex-disjoint cycles    │  │
│  C: ❌ (3)   │  │  and paths.                                                 │  │
│              │  │                                                              │  │
│              │  │  ⚠ Your last submission failed on test 7.                   │  │
│──────────────│  │  Test 7: n=100000, dense graph (edge case?)                 │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [e] Editor  [s] Submit to CF  [t] Test local  [r] Standings │ 01:23:45 │ 🔴 LIVE │
└───────────────────────────────────────────────────────────────────────────────────┘
```

### 6c. CF Standings View (overlay / modal)

```
┌─ CF Round #987 Standings ──────────────────────────────── 01:23:45 remaining ──┐
│                                                                                 │
│  Your rank: 1,247 / 18,432    Rating prediction: +23 → 1570                   │
│                                                                                 │
│  Rank  Handle          Score   A      B      C       D       E       F         │
│  ───────────────────────────────────────────────────────────────────────────── │
│  1     tourist         4968    500    998    1496    1974    ···     ···        │
│  2     Benq            4460    499    995    1490    ···     1476    ···        │
│  3     ecnerwala       3988    498    990    1488    ···     ···     ···        │
│  ...                                                                            │
│  1245  competitor1     1345    468    877    ···     ···     ···     ···        │
│  1246  someone_else    1342    470    872    ···     ···     ···     ···        │
│❯ 1247  >> YOU <<       1340    468    872    ···     ···     ···     ···        │
│  1248  another_user    1338    465    873    ···     ···     ···     ···        │
│  1249  coder42         1335    460    875    ···     ···     ···     ···        │
│  ...                                                                            │
│                                                                                 │
│  Friends:                                                                       │
│  412   your_friend1    2830    498    990    1342    ···     ···     ···        │
│  2841  your_friend2    872     464    408    ···     ···     ···     ···        │
│                                                                                 │
│  [j/k] Scroll  [f] Friends only  [/] Search handle  [Esc] Close               │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Stats Dashboard

### 7a. Overview

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Stats                                         [w]eek [m]onth [a]ll│
│              │                                                                    │
│  Rating:1559 │  ┌─ Rating Trend (30 days) ─────────────────────────────────────┐  │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 13 days  │  │  1600 ┤                                                      │  │
│              │  │       │                                              ╭──█     │  │
│──────────────│  │  1550 ┤                                    ╭────────╯        │  │
│              │  │       │              ╭──╮    ╭─────────────╯                  │  │
│   Dashboard  │  │  1500 ┤         ╭────╯  ╰───╯                               │  │
│   Problems   │  │       │    ╭────╯                                            │  │
│   Train      │  │  1450 ┤────╯                                                 │  │
│   Stress     │  │       │                                                      │  │
│   Contest    │  │  1400 ┤                                                      │  │
│ ❯ Stats      │  │       └─┬────┬────┬────┬────┬────┬────┬────┬────┬────┬───── │  │
│   Profile    │  │        Jan19 Jan26 Feb2  Feb9  Feb16                         │  │
│              │  │                                                              │  │
│──────────────│  │  Current: 1559 (+112 this month)    Peak: 1559              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Skill Breakdown ────────────────────────────────────────────┐  │
│              │  │                                                              │  │
│              │  │  Skill               Rating   Bar                Trend       │  │
│              │  │  ──────────────────────────────────────────────────────────  │  │
│──────────────│  │  dp.linear           1680     ████████████████░░░░   ▲ +20  │  │
│              │  │  graph.traversal     1590     ██████████████░░░░░░   ▲ +15  │  │
│              │  │  search.binary       1555     █████████████░░░░░░░   ▲ +12  │  │
│              │  │  ds.segment_tree     1520     ████████████░░░░░░░░   ─  +0  │  │
│              │  │  dp.bitmask          1178     ████████░░░░░░░░░░░░   ▲ +58  │  │
│              │  │  graph.flow          1092     ███████░░░░░░░░░░░░░   ▼  -8  │  │
│              │  │  math.ntheory        1010     ██████░░░░░░░░░░░░░░   ▲ +30  │  │
│              │  │  strings.suffix       980     █████░░░░░░░░░░░░░░░   ─  +0  │  │
│              │  │                                                              │  │
│              │  │  [Enter] Drill into skill  [s] Sort by: rating/trend/gap     │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Activity ──────┐  ┌─ Solve Distribution ───────────────────┐  │
│              │  │  This week:     │  │                                        │  │
│              │  │  Problems:  23  │  │  800-1200  ████████████████████  62%   │  │
│              │  │  Sessions:   5  │  │  1200-1500 ██████████░░░░░░░░░░  28%   │  │
│              │  │  Contests:   1  │  │  1500-1800 ████░░░░░░░░░░░░░░░░   8%   │  │
│              │  │  Stress:     2  │  │  1800+     █░░░░░░░░░░░░░░░░░░░   2%   │  │
│              │  │  Avg time: 14m  │  │                                        │  │
│              │  │  Streak: 13d 🔥 │  │  Hardest AC this week: 1800           │  │
│              │  └─────────────────┘  └────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [w] Week  [m] Month  [a] All time  [Enter] Skill detail  │  🟢  │  14:23 │ Stats│
└───────────────────────────────────────────────────────────────────────────────────┘
```

### 7b. Skill Detail View (drilled in)

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Skill Detail: dp.bitmask                                          │
│              │                                                                    │
│  Rating:1559 │  ┌─ Overview ───────────────────────────────────────────────────┐  │
│              │  │                                                              │  │
│              │  │  Rating: 1178  Developing        Gap from global: -381      │  │
│              │  │  Deviation: ±120 (moderate uncertainty)                      │  │
│              │  │  Mastery: ████████░░░░░░░░░░░░  Developing                  │  │
│              │  │                                                              │  │
│              │  │  Problems solved: 8/14 (57%)    Avg time: 24 min            │  │
│              │  │  Last practiced: today           Streak: 3 sessions          │  │
│              │  │                                                              │  │
│              │  │  Prerequisites:                                              │  │
│              │  │    ✅ dp.knapsack (1420)  ✅ bitwise.basics (1380)           │  │
│              │  │                                                              │  │
│              │  │  Unlocks:                                                    │  │
│              │  │    🔒 dp.sos (requires bitmask ≥1400)                        │  │
│              │  │    🔒 dp.broken_profile (requires bitmask ≥1500)             │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Rating History ──────────────────────────────────────────────┐ │
│              │  │  1200 ┤                                            █          │ │
│              │  │       │                                     ╭─────╯           │ │
│              │  │  1100 ┤              ╭──╮           ╭───────╯                 │ │
│              │  │       │    ╭─────────╯  ╰───╮╭─────╯                         │ │
│              │  │  1000 ┤────╯                ╰╯                               │ │
│              │  │       └──┬──────┬──────┬──────┬──────┬──────────────────────  │ │
│              │  │        Feb1   Feb5   Feb9   Feb13  Feb17                      │ │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Problem History ─────────────────────────────────────────────┐ │
│              │  │  Date     Problem               Diff  Verdict  Time   Δ      │ │
│              │  │  Today    Bitmask Subset Sum     1350  ✅ AC   22:15  +58    │ │
│              │  │  Feb 15   Bitmask Knapsack       1200  ✅ AC   14:30  +25    │ │
│              │  │  Feb 12   SOS DP Introduction    1500  ❌ WA   ──:──  -12    │ │
│              │  │  Feb 10   Subset Enumeration     1100  ✅ AC   08:45  +10    │ │
│              │  │  Feb 8    Bitmask + Graphs       1400  ❌ WA   ──:──   -8    │ │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │     [ t: Train this skill ]   [ p: Suggested problems ]            │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [t] Train skill  [p] Problems  [Esc] Back to stats        │  🟢  │  Stats       │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 8. Profile & Settings

```
┌──────────────┬────────────────────────────────────────────────────────────────────┐
│  ⚡ myro │  Profile & Settings                                                │
│              │                                                                    │
│  Rating:1559 │  ┌─ Profile ────────────────────────────────────────────────────┐  │
│  ▁▂▃▄▅▆▇█   │  │                                                              │  │
│  🔥 13 days  │  │  Username:    grinder42                                      │  │
│              │  │  Rating:      1559  Specialist                               │  │
│──────────────│  │  Joined:      Feb 1, 2026        Sessions: 42                │  │
│              │  │  Problems:    187 solved          Streak: 13 days 🔥          │  │
│   Dashboard  │  │                                                              │  │
│   Problems   │  │  CF Handle:  tourist              ✅ Linked                  │  │
│   Train      │  │  LC Handle:  lc_grinder           ✅ Linked                  │  │
│   Stress     │  │                                                              │  │
│   Contest    │  │  [i] Re-import history  [u] Unlink account                   │  │
│   Stats      │  └──────────────────────────────────────────────────────────────┘  │
│ ❯ Profile    │                                                                    │
│              │  ┌─ Training Settings ──────────────────────────────────────────┐  │
│──────────────│  │                                                              │  │
│              │  │  Preferred language:   [C++]  Python  Rust  Java  Go         │  │
│              │  │  Editor:               nvim ($EDITOR)                        │  │
│              │  │  Default session:      45 min                                │  │
│              │  │  Show tags:            [Off] On                              │  │
│              │  │  Show difficulty:      [On] Off                              │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│──────────────│                                                                    │
│              │  ┌─ AI Coach Settings ──────────────────────────────────────────┐  │
│              │  │                                                              │  │
│              │  │  Provider:        [Claude API]  Ollama  OpenAI-compat        │  │
│              │  │  Model:           claude-sonnet-4-20250514                      │  │
│              │  │  API Key:         sk-ant-•••••••••••••  [Edit]               │  │
│              │  │                                                              │  │
│              │  │  Passive watching: [On] Off                                  │  │
│              │  │  Wrong path delay: 3 min                                     │  │
│              │  │  Idle threshold:   5 min                                     │  │
│              │  │  Auto-open panel:  On [Off]                                  │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│              │  ┌─ Appearance ─────────────────────────────────────────────────┐  │
│              │  │                                                              │  │
│              │  │  Theme:  [Default]  Catppuccin  Gruvbox  Monokai  Nord       │  │
│              │  │  Compact mode:  On [Off]                                     │  │
│              │  │  Sparklines:    [On] Off                                     │  │
│              │  │                                                              │  │
│              │  └──────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
├──────────────┴────────────────────────────────────────────────────────────────────┤
│ [Tab] Next field  [Enter] Toggle/Edit  [Esc] Back       │  🟢  │  14:23 │ Profile│
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 9. First-Run Onboarding

```
┌───────────────────────────────────────────────────────────────────────────────────┐
│                                                                                   │
│                                                                                   │
│                                                                                   │
│                ███╗   ███╗██╗   ██╗██████╗  ██████╗                                │
│                ████╗ ████║╚██╗ ██╔╝██╔══██╗██╔═══██╗                               │
│                ██╔████╔██║ ╚████╔╝ ██████╔╝██║   ██║                               │
│                ██║╚██╔╝██║  ╚██╔╝  ██╔══██╗██║   ██║                               │
│                ██║ ╚═╝ ██║   ██║   ██║  ██║╚██████╔╝                               │
│                ╚═╝     ╚═╝   ╚═╝   ╚═╝  ╚═╝ ╚═════╝                                │
│                                                                                   │
│                     Train smarter. Compete harder. In your terminal.              │
│                                                                                   │
│                                                                                   │
│        ┌─ Step 1 of 4: Language ─────────────────────────────────────┐            │
│        │                                                             │            │
│        │  What's your primary competitive programming language?      │            │
│        │                                                             │            │
│        │    ❯ C++ (G++17)                                            │            │
│        │      Python 3                                               │            │
│        │      Rust                                                   │            │
│        │      Java                                                   │            │
│        │      Go                                                     │            │
│        │                                                             │            │
│        │  Your $EDITOR is: nvim  ✅                                  │            │
│        │                                                             │            │
│        └─────────────────────────────────────────────────────────────┘            │
│                                                                                   │
│                                                                                   │
│                           [Enter] Next   [Esc] Skip setup                        │
│                                                                                   │
└───────────────────────────────────────────────────────────────────────────────────┘

  Step 2: Import accounts

        ┌─ Step 2 of 4: Import ────────────────────────────────────────┐
        │                                                              │
        │  Import your history to bootstrap your skill ratings.        │
        │                                                              │
        │  Codeforces handle:  tourist█                                │
        │                      ✅ Found! 3,847 submissions             │
        │                                                              │
        │  LeetCode username:  (skip)█                                 │
        │                                                              │
        │  ┌─ Decay Settings ──────────────────────────────────────┐   │
        │  │  AC half-life:    [12 months]  6mo  18mo  24mo        │   │
        │  │  Fail half-life:  [6 months]   3mo  12mo              │   │
        │  │                                                       │   │
        │  │  Recent history weighted more heavily. Older solves   │   │
        │  │  still count, but with reduced confidence.            │   │
        │  └───────────────────────────────────────────────────────┘   │
        │                                                              │
        └──────────────────────────────────────────────────────────────┘

  Step 3: Import processing (animated)

        ┌─ Step 3 of 4: Analyzing ─────────────────────────────────────┐
        │                                                              │
        │  Importing Codeforces history for: tourist                   │
        │                                                              │
        │  Fetching submissions   ████████████████████████████  3,847  │
        │  Deduplicating          ████████████████████████████  2,104  │
        │  Mapping skills         ████████████████████████████  done   │
        │  Replaying history      ████████████████░░░░░░░░░░░░  67%   │
        │                         Processing week 105 of 156...        │
        │  Applying time decay    ░░░░░░░░░░░░░░░░░░░░░░░░░░░░        │
        │                                                              │
        │  Skills discovered: 47                                       │
        │  Estimated rating: ~1847 (calibrating...)                    │
        │                                                              │
        └──────────────────────────────────────────────────────────────┘

  Step 4: Results

        ┌─ Step 4 of 4: Ready! ────────────────────────────────────────┐
        │                                                              │
        │  Your profile has been set up!                               │
        │                                                              │
        │  Estimated rating: 1847                                      │
        │                                                              │
        │  Top skills:                     Weakest skills:             │
        │  ├─ graph.shortest  2105         ├─ dp.bitmask     1240 ⚠   │
        │  ├─ dp.linear       1980         ├─ strings.suffix  1180 ⚠   │
        │  ├─ search.binary   1920         ├─ math.fft        1050 ⚠   │
        │  └─ ds.seg_tree     1870         └─ graph.flow      1100 ⚠   │
        │                                                              │
        │  High uncertainty (will re-test):                            │
        │  ├─ dp.digit        1450 ±280                                │
        │  └─ math.combin     1380 ±310                                │
        │                                                              │
        │  Tip: The engine will prioritize your weakest skills         │
        │  and uncertain ratings first. Let's get training!            │
        │                                                              │
        │               [ Enter: Start your first session! ]           │
        │                                                              │
        └──────────────────────────────────────────────────────────────┘
```

---

## 10. Help Overlay (? key)

```
┌─ myro Keybindings ──────────────────────────────────────────────────────────┐
│                                                                                   │
│  ── Global ──────────────────────────  ── Problem View ─────────────────────────  │
│  1-7       Switch to screen            e        Open in $EDITOR                  │
│  /         Search / filter             s        Submit solution                   │
│  ?         This help screen            t        Toggle tags visibility            │
│  q         Quit / back                 h        Request hint (affects rating)     │
│  Tab       Switch focus/panels         d        Open AI coach dialogue            │
│  Esc       Close overlay / back        v        View on original site             │
│                                                                                   │
│  ── Navigation ─────────────────────  ── Training Session ──────────────────────  │
│  j / ↓     Move down                  n        Next problem / skip                │
│  k / ↑     Move up                    x        End session                        │
│  g         Go to top                  Enter    Confirm / proceed                  │
│  G         Go to bottom                                                           │
│  Ctrl+d    Page down                  ── Contest ────────────────────────────────  │
│  Ctrl+u    Page up                    Tab      Switch between problems            │
│  Enter     Select / open              r        View standings                     │
│                                        t        Test solution locally              │
│  ── Filters ────────────────────────   s        Submit to Codeforces              │
│  f         Open filter menu                                                       │
│  F         Clear all filters          ── AI Coach ──────────────────────────────  │
│  [         Decrease difficulty min     Enter    Send message                       │
│  ]         Increase difficulty max     n        Next hint level                    │
│                                        g        Give up (full explanation)         │
│                                        x        Close coach panel                 │
│                                                                                   │
│                              [Esc] or [?] to close                               │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 11. Responsive Layout (Narrow Terminal)

For terminals narrower than 100 columns, the sidebar collapses:

```
┌─ my ─┬──────────────────────────────────────────────────────────────┐
│      │  Dashboard                                                   │
│ 1559 │                                                              │
│ █▇▆▅ │  Rating: 1559 Specialist  +23 this week                     │
│🔥 13 │                                                              │
│      │  1400┤         ╭──╮           ╭╮                             │
│──────│      │    ╭────╯  ╰──╮   ╭───╯╰╮    ╭─╮                    │
│❯ D   │  1500┤────╯          ╰───╯     ╰────╯ ╰──█                 │
│  P   │      └─┬──────┬──────┬──────┬──────┬──────┤                 │
│  T   │                                                              │
│  S   │  Weakest Skills          Quick Actions                       │
│  C   │  dp.bitmask    1120      [t] Train                           │
│  St  │  math.ntheory   980      [s] Stress test                     │
│  Pr  │  graph.flow    1100      [c] Contest                         │
│      │                                                              │
│──────│  Recent: 5 solved today, avg 1420, +12 rating                │
│ [?]  │                                                              │
└──────┴──────────────────────────────────────────────────────────────┘
```

For minimum terminal (80×24):

```
┌─ myro ── 1559 ── 🔥13 ──────────────────────────────────┐
│                                                                │
│  Dashboard                                                     │
│  Rating: 1559 (+23)  ▁▂▃▄▅▆▇█                                │
│                                                                │
│  Weak: dp.bitmask(1120) math(980) flow(1100)                  │
│                                                                │
│  Today: 5 solved · avg 1420 · +12 rating                      │
│                                                                │
│  [t]rain [s]tress [c]ontest [p]roblems [?]help                │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## 12. Notification Toasts

Ephemeral notifications that appear top-right and auto-dismiss:

```
                                          ┌──────────────────────────┐
                                          │  ✅ Accepted! +12 rating │
                                          │  dp.bitmask: 1120 → 1178│
                                          └──────────────────────────┘

                                          ┌──────────────────────────┐
                                          │  ❌ Wrong Answer (test 3)│
                                          │  [r] Retry  [d] Coach    │
                                          └──────────────────────────┘

                                          ┌──────────────────────────┐
                                          │  🔔 CF Round #987 starts │
                                          │  in 30 minutes!          │
                                          │  [j] Join  [x] Dismiss   │
                                          └──────────────────────────┘

                                          ┌──────────────────────────┐
                                          │  🔥 14-day streak!       │
                                          │  Keep it up!             │
                                          └──────────────────────────┘
```

---

## Design Notes for Implementation

### Ratatui Widget Mapping

| Wireframe Element | Ratatui Widget |
|---|---|
| Sidebar navigation | `List` with custom styling |
| Problem list | `Table` with sortable headers |
| Rating chart | `Chart` with `Dataset` (line) |
| Skill bars | `Gauge` or custom `BarChart` |
| Sparklines | `Sparkline` |
| Code preview | `Paragraph` with syntax highlighting |
| Modal overlays | `Clear` + `Block` centered |
| Status bar | `Paragraph` in bottom `Layout` chunk |
| Tabs (w/m/a) | `Tabs` widget |
| Coach indicator | Custom styled `Span` in status |

### Layout Strategy

```rust
// Main layout: sidebar + content
let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Length(16),     // Sidebar (fixed width)
        Constraint::Min(60),       // Main content (fills remaining)
    ])
    .split(frame.area());

// Sidebar internal layout
let sidebar = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(7),     // Logo + rating + sparkline + streak
        Constraint::Min(10),       // Navigation
        Constraint::Length(6),     // Quick info (due today, upcoming)
        Constraint::Length(5),     // Keybinding hints
    ])
    .split(chunks[0]);

// Content area: varies by screen, but typical:
let content = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Min(0),       // Main content (scrollable)
        Constraint::Length(1),     // Status bar
    ])
    .split(chunks[1]);
```

### Terminal Size Breakpoints

| Width | Layout |
|---|---|
| ≥120 cols | Full layout — expanded sidebar + spacious content |
| 100–119 | Standard layout — full sidebar, tighter content |
| 80–99 | Compact sidebar (icons only) + content |
| <80 | No sidebar — header bar + content + status |

*All wireframes above assume 120-column width. Implement graceful
degradation for narrower terminals.*
