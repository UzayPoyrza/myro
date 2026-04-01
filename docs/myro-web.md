# Myro Web Trainer — Observation Coach MVP Spec

> The web app is a **top-level Myro product surface** — not a minor add-on.
> It delivers the Observation Coach: an LLM-powered training tool that guides
> users through competitive programming problems by detecting and unlocking
> key insights (observations) — questions first, more direct help when stuck.

---

## Table of Contents

1. [Trainer Concept](#1-trainer-concept)
2. [Coach Output Contract](#2-coach-output-contract)
3. [Hint Ladder](#3-hint-ladder)
4. [Web UX Requirements](#4-web-ux-requirements)
5. [Data Requirements](#5-data-requirements)
6. [LLM Requirements](#6-llm-requirements)
7. [Seed Data](#7-seed-data)
8. [Architecture](#8-architecture)

---

## 1. Trainer Concept

### Core Model: Problems → Routes → Observations

Every competitive programming problem has one or more **optimal solution routes**.
Each route is a sequence of **observations** — the key insights a solver must
discover to arrive at the solution.

```
Problem: "Maximum Subarray Sum"
├── Route 1: Kadane's Algorithm
│   ├── obs_1: "Subproblem structure — max ending at each position"
│   ├── obs_2: "Reset condition — when running sum goes negative, restart"
│   ├── obs_3: "Single pass sufficiency — O(n) by tracking global max"
│   └── obs_4: "Edge case — all-negative array returns least negative"
│
└── Route 2: Divide and Conquer
    ├── obs_1: "Split at midpoint — answer is in left, right, or crossing"
    ├── obs_2: "Crossing subarray — extend from mid in both directions"
    ├── obs_3: "Recurrence — T(n) = 2T(n/2) + O(n) → O(n log n)"
    └── obs_4: "Base case — single element subarray"
```

### Coach Behavior

The coach's job is to **detect** which observation the user is approaching,
has found, is moving away from, or is uncertain about — and **nudge** accordingly.

**The coach must NOT:**
- Write the user's solution for them
- Read observation titles or descriptions verbatim
- Skip ahead to later observations before earlier ones are found

**The coach MUST:**
- Help the user make progress — questions first, more direct help when stuck
- Confirm observations when the user demonstrates understanding
- Be encouraging but honest about wrong directions
- Name concepts and techniques when the user is struggling
- Escalate help naturally: questions → pointed questions → partial formalizations → near-complete descriptions

### Mode Transition

When the user finds **all observations for a route**, the UI switches to
**Implementation Mode**:
- Observations become a visible checklist
- The coach shifts from "guide" to "reviewer"
- A code editor appears with a structured checklist:
  1. State definition
  2. Transition logic
  3. Complexity analysis
  4. Edge cases

---

## 2. Coach Output Contract

Every coach response MUST return structured JSON. This is enforced at the
application layer — invalid responses are auto-repaired or retried.

```typescript
interface CoachResponse {
  /** What state the user is in relative to an observation */
  state: "approaching" | "found" | "moving_away" | "uncertain";

  /** How confident the coach is in this assessment (0.0–1.0) */
  confidence: number;

  /** Which observation the user is closest to (null if unclear) */
  matched_observation_id: string | null;

  /** Short coaching message — directional, not solution-dumping */
  coach_message: string;

  /** One concrete next action for the user */
  next_action: string;

  /** Signals that led to this assessment */
  why_signals: string[];
}
```

**Validation rules (enforced by Zod):**
- `state` must be one of the 4 enum values
- `confidence` must be a number in [0.0, 1.0]
- `matched_observation_id` must be null or match an existing observation ID
- `coach_message` must be non-empty, max 500 chars
- `next_action` must be non-empty, max 200 chars
- `why_signals` must have 1–5 entries

**When validation fails:**
1. Auto-repair: attempt to extract valid JSON from the response
2. If repair fails: retry the LLM call once with a stricter prompt
3. If retry fails: return a generic "I'm thinking..." response and log the error

---

## 3. Hint Ladder

Each observation has **3 reveal levels**. The coach advances the ladder
**only** when:
- The user explicitly clicks "More Hint", OR
- The user stalls for N consecutive turns (default: 3 turns with no progress)

### Hint Levels

| Level | Name | Description | Example (for "Reset when sum < 0") |
|-------|------|-------------|--------------------------------------|
| 1 | Nudge | A gentle directional push | "What happens to your running sum when it dips below zero?" |
| 2 | Pointed Question | A question that narrows the search space | "If your current sum is negative, can it ever help a future subarray? What should you do instead?" |
| 3 | Partial Formalization | Nearly states the observation, stops short of full implementation | "When the running sum becomes negative, the optimal strategy is to discard everything and start fresh from the next element. Why?" |

### Ladder Rules

- Level 0: No hint given (user is exploring freely)
- Advancing from level N to N+1 is **irreversible** for that observation in that session
- The UI shows the current hint level per observation (as dots: ○○○, ●○○, ●●○, ●●●)
- Hint usage is tracked and affects the quality score of the solve (not blocking, just informational)

---

## 4. Web UX Requirements

### Page 1: Landing

- Headline: "Train your problem-solving instincts, not your Ctrl+C skills"
- Brief explanation of the observation model (3-step visual)
- CTA button: "Start Training" → routes to problem library or directly to a session
- No login required to view, login required to start training

### Page 2: Problem Library

- Grid/list of available problems
- Filters: topic (DP, graphs, greedy, etc.), difficulty (800–2400 range)
- Each problem card shows:
  - Title, difficulty, topics
  - Number of routes available
  - User's best completion (if any): "3/5 observations found"
- Click → starts a new training session (or resumes existing)

### Page 3: Training Session (Core Experience)

**Layout:** Three-panel split

```
┌─────────────────────────────────────────────────────────────┐
│ Problem Statement                                     [hide]│
│ ─────────────────────────────────────────────────────────── │
│ A. Maximum Subarray Sum          time: 2s | mem: 256MB      │
│                                                             │
│ Given an array of n integers, find the contiguous subarray  │
│ with the maximum sum.                                       │
│ ...                                                         │
├─────────────────────────┬───────────────────────────────────┤
│                         │  Insight Map      [3/5 unlocked]  │
│  Chat                   │  ─────────────────────────────── │
│  ─────────────────────  │  ✓ Insight 1: Subproblem struct. │
│                         │  ✓ Insight 2: Reset condition     │
│  Coach: What patterns   │  ✓ Insight 3: Single pass O(n)   │
│  do you notice about    │  ? Insight 4                      │
│  optimal subarrays?     │  ? Insight 5                      │
│                         │  ─────────────────────────────── │
│  You: I think we can    │  Progress: ████████░░░░ 60%       │
│  track a running sum    │                                   │
│  and reset when...      │  Route: Kadane's Algorithm        │
│                         │  [Switch Route] [More Hint]       │
│  [Send]                 │                                   │
├─────────────────────────┴───────────────────────────────────┤
│ Scratchpad (optional, collapsible)                          │
│ ─────────────────────────────────────────────────────────── │
│ - running sum approach                                      │
│ - reset when negative?                                      │
│ - need to track global max                                  │
└─────────────────────────────────────────────────────────────┘
```

**Components:**

1. **Problem Statement** (top, collapsible)
   - Full problem description, examples, constraints
   - Toggle hide/show to maximize workspace

2. **Chat Panel** (left)
   - User types observations, questions, approaches
   - Coach responds with structured coaching
   - Messages stream in real-time (SSE or WebSocket)
   - Chat history persisted per session

3. **Insight Map** (right)
   - Shows all observations for the current route
   - Locked ones: "Insight #k" (no title revealed)
   - Unlocked ones: show real title + brief description
   - Hint dots showing ladder progress (○○○ → ●○○ → ●●○ → ●●●)
   - Progress bar: X/Y observations unlocked

4. **Route Picker**
   - Only shown if multiple routes exist
   - Appears after user asks or after 1–2 initial turns
   - Warning on switch: "Progress is per-route. Your current progress on Route 1 will be saved but you'll start fresh on Route 2."

5. **Action Buttons**
   - "More Hint" — advances hint ladder for the nearest relevant observation
   - "Switch Route" — shows route picker (if multiple routes)
   - "I'm Done" — triggers implementation mode if enough observations found

6. **Scratchpad** (bottom, collapsible)
   - Free-form text area for notes
   - Persisted per session
   - Not sent to the LLM (user's private workspace)

### Page 4: Implementation Mode

Triggered when all observations for a route are unlocked. The layout transforms:

```
┌─────────────────────────────────────────────────────────────┐
│ Implementation Mode — Maximum Subarray Sum (Kadane's)       │
│ ─────────────────────────────────────────────────────────── │
│ Checklist:                                                  │
│ □ State definition: what variables do you need?             │
│ □ Transition: how does state update per element?            │
│ □ Complexity: what's the time and space complexity?         │
│ □ Edge cases: what inputs could break your solution?        │
├─────────────────────────┬───────────────────────────────────┤
│                         │  Observations (all unlocked)      │
│  Code Editor            │  ─────────────────────────────── │
│  ─────────────────────  │  ✓ Subproblem structure           │
│  def max_subarray(a):   │  ✓ Reset condition                │
│    max_sum = a[0]       │  ✓ Single pass O(n)               │
│    cur = a[0]           │  ✓ Edge: all-negative             │
│    for x in a[1:]:      │                                   │
│      cur = max(x,       │  Coach (reviewer mode):           │
│        cur + x)         │  "Your state definition looks     │
│      max_sum = max(     │   correct. Consider: what if the  │
│        max_sum, cur)    │   array is empty?"                │
│    return max_sum       │                                   │
│                         │  [Run Tests] [Submit]             │
│  [Python ▼]             │                                   │
└─────────────────────────┴───────────────────────────────────┘
```

**Key differences from training mode:**
- Chat becomes reviewer mode (coach reviews code, not approaches)
- Code editor replaces scratchpad
- Checklist tracks implementation progress
- All observations visible as reference
- "Run Tests" executes against example test cases
- "Submit" marks the problem as solved

---

## 5. Data Requirements

### Persistence (server-side, no offline-first requirement)

Must persist across sessions:
- Chat history (all messages with roles and timestamps)
- Scratchpad content (per session)
- Unlocked observations (per session, per route)
- Current route selection
- Hint ladder step per observation
- Implementation mode state (checklist items, code)

### Database Choice: SQLite (MVP) → Postgres (production)

**Justification:** The existing Myro repo uses SQLite everywhere (rusqlite with
bundled feature in both myro-tui and myro-predict). For the web MVP, SQLite with
Prisma is perfectly adequate for single-server deployment. Migration to Postgres
is a one-line Prisma config change when scaling.

### Auth

Email magic link via NextAuth (fastest setup, no password management).
GitHub OAuth as secondary option. For MVP: demo mode with a seeded user
to enable immediate testing without auth setup.

### Schema (Prisma)

```prisma
model User {
  id        String   @id @default(cuid())
  email     String   @unique
  name      String?
  sessions  TrainingSession[]
  createdAt DateTime @default(now())
}

model Problem {
  id          String   @id @default(cuid())
  title       String
  difficulty  Int
  topic       String
  description String
  inputSpec   String
  outputSpec  String
  examples    String   // JSON: [{input, output}]
  routes      Route[]
  sessions    TrainingSession[]
}

model Route {
  id           String        @id @default(cuid())
  problemId    String
  problem      Problem       @relation(fields: [problemId], references: [id])
  name         String
  description  String
  order        Int
  observations Observation[]
  sessions     TrainingSession[]
}

model Observation {
  id          String   @id @default(cuid())
  routeId     String
  route       Route    @relation(fields: [routeId], references: [id])
  order       Int
  title       String
  description String
  hints       String   // JSON: [level1_nudge, level2_question, level3_formalization]
  unlocks     UnlockedObservation[]
}

model TrainingSession {
  id            String    @id @default(cuid())
  userId        String
  user          User      @relation(fields: [userId], references: [id])
  problemId     String
  problem       Problem   @relation(fields: [problemId], references: [id])
  routeId       String?
  route         Route?    @relation(fields: [routeId], references: [id])
  status        String    @default("active") // active | observing | implementing | completed
  scratchpad    String    @default("")
  code          String    @default("")
  messages      ChatMessage[]
  unlocked      UnlockedObservation[]
  createdAt     DateTime  @default(now())
  updatedAt     DateTime  @updatedAt
}

model ChatMessage {
  id        String   @id @default(cuid())
  sessionId String
  session   TrainingSession @relation(fields: [sessionId], references: [id])
  role      String   // user | coach
  content   String
  metadata  String?  // JSON: CoachResponse for coach messages
  createdAt DateTime @default(now())
}

model UnlockedObservation {
  id            String   @id @default(cuid())
  sessionId     String
  session       TrainingSession @relation(fields: [sessionId], references: [id])
  observationId String
  observation   Observation @relation(fields: [observationId], references: [id])
  hintLevel     Int      @default(0) // 0=found naturally, 1-3=hint level used
  unlockedAt    DateTime @default(now())

  @@unique([sessionId, observationId])
}
```

---

## 6. LLM Requirements

### Provider Abstraction

```typescript
interface LLMProvider {
  name: string;
  chat(messages: ChatMessage[], options?: LLMOptions): Promise<string>;
  stream(messages: ChatMessage[], options?: LLMOptions): AsyncIterable<string>;
}

interface LLMOptions {
  temperature?: number;   // default: 0.7
  maxTokens?: number;     // default: 1024
  systemPrompt?: string;
}
```

Implementations:
- `AnthropicProvider` — uses `@anthropic-ai/sdk`, Claude Sonnet 4
- `OpenAIProvider` — uses `openai` SDK, GPT-4o
- Provider selected by `LLM_PROVIDER` env var, key by `LLM_API_KEY`

### System Prompt Structure

```
You are Myro, an observation coach for competitive programming.

PROBLEM:
{title} ({difficulty})
{description}
{input_spec}
{output_spec}
{examples}

ROUTE: {route_name}
{route_description}

OBSERVATIONS (key insights for this route):
{for each observation:}
- {obs_id}: {title} — {description} [STATUS: locked|unlocked]
  {if hint_level > 0: "Hint given: level {N}"}

UNLOCKED SO FAR: {list of unlocked observation titles}
CHAT HISTORY: {last 10 messages}

RULES:
1. Detect which observation the user is approaching/found/moving_away/uncertain
2. Help the user make progress — prefer questions, give more when stuck
3. Confirm an observation ONLY when the user clearly demonstrates understanding
4. Be encouraging and constructive. Name concepts when helpful.
5. Be honest about wrong directions — redirect clearly
6. Keep coach_message under 500 characters
7. Keep next_action under 200 characters

Respond with ONLY valid JSON matching this schema:
{CoachResponse schema}
```

### JSON Validation

Use Zod to validate every LLM response:

```typescript
const CoachResponseSchema = z.object({
  state: z.enum(["approaching", "found", "moving_away", "uncertain"]),
  confidence: z.number().min(0).max(1),
  matched_observation_id: z.string().nullable(),
  coach_message: z.string().min(1).max(500),
  next_action: z.string().min(1).max(200),
  why_signals: z.array(z.string()).min(1).max(5),
});
```

### Safeguards

1. **JSON extraction**: If response contains text before/after JSON, extract the JSON block
2. **Auto-repair**: Fix common issues (missing quotes, trailing commas)
3. **Retry**: On validation failure, retry once with an appended "Your previous response was invalid JSON. Respond with ONLY valid JSON."
4. **Fallback**: If retry fails, return `{ state: "uncertain", confidence: 0, matched_observation_id: null, coach_message: "Let me think about that differently. Can you tell me more about your approach?", next_action: "Describe your current thinking about the problem", why_signals: ["response_parse_error"] }`

---

## 7. Seed Data

10 problems with 1–2 routes each, 4–7 observations per route. Topics span
the core CP skill areas. Difficulty ranges from 1000 to 2000.

### Problem List

| # | Title | Diff | Topic | Routes | Obs/Route |
|---|-------|------|-------|--------|-----------|
| 1 | Two Sum | 1000 | arrays, hashing | 1 | 4 |
| 2 | Maximum Subarray Sum | 1200 | dp, greedy | 2 | 4, 4 |
| 3 | Binary Search on Answer | 1300 | binary search | 1 | 5 |
| 4 | BFS Shortest Path | 1300 | graphs | 1 | 5 |
| 5 | Coin Change (Min Coins) | 1400 | dp | 1 | 5 |
| 6 | Merge Intervals | 1200 | sorting, greedy | 1 | 4 |
| 7 | Longest Increasing Subsequence | 1500 | dp, binary search | 2 | 5, 4 |
| 8 | Number of Islands | 1300 | graphs, dfs | 1 | 4 |
| 9 | Sliding Window Maximum | 1600 | deque, sliding window | 1 | 5 |
| 10 | Knapsack 0/1 | 1500 | dp | 1 | 6 |

Full seed data with all observations and hints is defined in
`apps/web/prisma/seed.ts`.

---

## 8. Architecture

### Stack

- **Framework**: Next.js 15 (App Router)
- **Styling**: Tailwind CSS
- **Database**: Prisma + SQLite (MVP), one-line migration to Postgres
- **Auth**: NextAuth with demo mode (MVP), email magic link (post-MVP)
- **LLM**: Provider abstraction over Anthropic/OpenAI SDKs
- **Validation**: Zod for all LLM responses and API inputs
- **Language**: TypeScript (strict mode)

### Directory Structure

```
apps/web/
├── package.json
├── next.config.ts
├── tsconfig.json
├── tailwind.config.ts
├── .env.example
├── prisma/
│   ├── schema.prisma
│   └── seed.ts
└── src/
    ├── app/
    │   ├── layout.tsx
    │   ├── page.tsx                    # Landing page
    │   ├── problems/
    │   │   └── page.tsx                # Problem library
    │   ├── train/
    │   │   └── [sessionId]/
    │   │       └── page.tsx            # Training session
    │   └── api/
    │       ├── sessions/route.ts       # Create/get sessions
    │       ├── chat/route.ts           # Send message → coach response
    │       └── hints/route.ts          # Advance hint ladder
    ├── components/
    │   ├── chat.tsx
    │   ├── insight-map.tsx
    │   ├── problem-statement.tsx
    │   └── code-editor.tsx
    └── lib/
        ├── db.ts                       # Prisma client singleton
        ├── llm/
        │   ├── provider.ts             # LLM abstraction
        │   ├── anthropic.ts
        │   └── openai.ts
        ├── coach/
        │   ├── prompt.ts               # System prompt builder
        │   └── schema.ts               # Zod schemas
        └── seed-data.ts                # Problem/route/observation data
```

### API Routes

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/sessions` | POST | Create new training session for a problem |
| `/api/sessions` | GET | List user's sessions |
| `/api/chat` | POST | Send user message, get coach JSON response |
| `/api/hints` | POST | Advance hint ladder for an observation |

### Data Flow

```
User types message
    │
    ▼
POST /api/chat { sessionId, message }
    │
    ├─ Save user message to DB
    ├─ Load session context (problem, route, observations, chat history)
    ├─ Build system prompt with current state
    ├─ Call LLM provider
    ├─ Validate JSON response (Zod)
    │   ├─ Valid → continue
    │   └─ Invalid → auto-repair → retry → fallback
    ├─ If state === "found" && confidence > 0.8:
    │   └─ Unlock the matched observation in DB
    ├─ Save coach message to DB
    ├─ If all observations unlocked:
    │   └─ Update session status to "implementing"
    └─ Return { coachResponse, unlockedObservations, sessionStatus }
```
