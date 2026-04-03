<p align="center">
  <h1 align="center">myro</h1>
  <p align="center">
    A terminal-native competitive programming trainer with AI coaching and solve-probability prediction.
  </p>
  <p align="center">
    <a href="#install">Install</a> &middot;
    <a href="#features">Features</a> &middot;
    <a href="#architecture">Architecture</a> &middot;
    <a href="#development">Development</a> &middot;
    <a href="#license">License</a>
  </p>
</p>

<br>

<p align="center">
  <img src="assets/demo.gif" width="700" alt="myro demo" />
</p>

<br>

## What is myro?

Myro is a TUI app that turns your terminal into a competitive programming gym. It pulls problems from Codeforces, predicts which ones match your skill level using a logistic matrix factorization model, and coaches you through solutions with an AI assistant — all without leaving your editor.

Think of it as Anki meets Codeforces, inside a terminal.

**Key ideas:**

- **Personalized difficulty** — a cold-start ML model fits your Codeforces history on the fly and recommends problems at your sweet spot (configurable target probability)
- **AI coach** — an LLM-powered coach watches your code, detects when you're stuck, and gives Socratic hints — not answers
- **TikTok flow** — solve a problem, get the next one instantly. No menus, no context switching
- **Zero setup** — one command to install, enter your CF handle, start solving

---

## Install

**One-line install** (macOS / Linux):

```bash
curl -fsSL https://raw.githubusercontent.com/UzayPoyrza/myro/main/scripts/install.sh | bash
```

This detects your OS and architecture, downloads a pre-built binary, installs it to `~/.local/bin`, and adds it to your PATH.

### Update

Run the same command — it skips if you're already on the latest version:

```bash
curl -fsSL https://raw.githubusercontent.com/UzayPoyrza/myro/main/scripts/install.sh | bash
```

### Uninstall

Removes the binary, config, data, and PATH entries:

```bash
curl -fsSL https://raw.githubusercontent.com/UzayPoyrza/myro/main/scripts/install.sh | bash -s -- --uninstall
```

<details>
<summary><strong>Other install methods</strong></summary>

**Build from source** (requires Rust 1.75+):

```bash
git clone https://github.com/UzayPoyrza/myro.git
cd myro
cargo install --path crates/myro-tui
```

**From crates.io** (coming soon):

```bash
cargo install myro-tui
```

</details>

### Supported platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS | Apple Silicon (aarch64) | Supported |
| macOS | Intel (x86_64) | Supported |
| Linux | x86_64 | Supported |
| Linux | aarch64 | Supported |
| Windows | — | Use WSL |

---

## Quick start

```bash
# 1. Install
curl -fsSL https://raw.githubusercontent.com/UzayPoyrza/myro/main/scripts/install.sh | bash

# 2. Run
myro
```

On first launch, myro asks for your Codeforces handle. It fetches your submission history, fits a skill model, and drops you into your first recommended problem.

### Setting up the AI coach

The coach requires an LLM API key. In the app, press `s` to open Settings and configure your provider:

| Provider | What you need |
|----------|--------------|
| **OpenRouter** | API key from [openrouter.ai](https://openrouter.ai) |
| **Anthropic** | API key from [console.anthropic.com](https://console.anthropic.com) |
| **OpenAI** | API key from [platform.openai.com](https://platform.openai.com) |
| **Google** | API key from [aistudio.google.com](https://aistudio.google.com) |

Or set via environment variables:

```bash
export MYRO_LLM_BASE_URL="https://openrouter.ai/api/v1"
export MYRO_LLM_API_KEY="sk-..."
export MYRO_LLM_MODEL="anthropic/claude-sonnet-4"
```

---

## Features

### Problem recommendation

Myro uses a **logistic matrix factorization** model trained on Codeforces submission data. On launch, it computes your skill embedding from your solve history (time-weighted, recent solves matter more) and recommends problems near your target solve probability.

- No cloud dependency — the model runs locally
- Cold-start friendly — works from your first session using only your CF history
- Per-tag skill ratings — tracks your strengths/weaknesses across algorithm topics

### AI coach

A context-aware LLM coach that lives in a panel beside your editor:

- **Automatic stall detection** — notices when you haven't made progress and offers a nudge
- **Socratic hints** — guides you toward the insight without spoiling the solution
- **`/hint`** — request a context-aware hint based on your current code
- **`/coach <msg>`** — ask the coach a direct question with your code as context
- **Observation-based model** — the coach tracks which key observations you've discovered and which you're missing

### Built-in vim editor

Full vim-mode editor with syntax highlighting (Python). Write, test, and submit without leaving the terminal.

### Codeforces integration

- Fetches problem statements directly from CF
- **`/run`** — run your solution against sample test cases (Python, 5s timeout)
- **`/submit`** — submit directly to Codeforces and poll for the verdict
- **`/isuck`** — skip to the next problem if you're stuck

### Skill tracking

- Per-tag skill ratings that update as you solve problems
- Skill history over time
- Weak-skill targeting in recommendations

---

## Architecture

```
myro/
├── crates/
│   ├── myro-tui        # Ratatui terminal app (the main binary)
│   ├── myro-cf         # Codeforces API client + problem parser
│   ├── myro-predict    # ML model: logistic matrix factorization
│   ├── myro-coach      # AI coaching engine + prompt templates
│   ├── myro-api        # Supabase client (auth, sync, analytics)
│   └── myro-bench      # Coaching quality benchmark suite
├── apps/
│   └── web/            # Next.js Observation Coach (web trainer)
├── prompts/            # Editable LLM prompt templates
├── test-problem-set/   # Decomposed problem definitions (JSON)
└── bench/              # Benchmark scenarios + results
```

### Crate dependency graph

```
myro-tui ─┬─► myro-predict ──► myro-cf
           ├─► myro-coach ────► myro-cf
           └─► myro-api
```

### Key design decisions

- **Synchronous event loop** — no async runtime. Background work (tests, LLM calls, CF submissions) runs on spawned threads with `mpsc` channels polled in `tick()`.
- **Cold-start ML** — the deployed artifact is a `ProblemModel` (problem embeddings only). User embeddings are fit on-the-fly via time-weighted SGD against CF history.
- **No database in the TUI** — state is flat JSON files in `~/.local/share/myro/`. SQLite is only used by `myro-predict` for the training pipeline.
- **XDG-compliant paths** — config in `~/.config/myro/`, data in `~/.local/share/myro/`.
- **rustls-tls** — no OpenSSL dependency.

---

## Development

### Prerequisites

- Rust 1.75+ (install via [rustup.rs](https://rustup.rs))
- [just](https://github.com/casey/just) (optional, for task runner shortcuts)

### Common commands

```bash
just                # Run the TUI
just build          # Build all crates
just check          # Type-check
just test           # Run tests
just clippy         # Lint
just fmt            # Format
just mock-tui       # Run with mock coach (no API key needed)
```

### Running from source

```bash
cargo run -p myro-tui
```

### Web trainer (Observation Coach)

```bash
cd apps/web
just web-setup      # Install deps + setup DB
just web-dev        # Start dev server at localhost:3000
```

Requires `LLM_PROVIDER` and `LLM_API_KEY` in `apps/web/.env`.

### Running tests

```bash
cargo test                    # All tests
cargo test -p myro-tui        # TUI tests only
cargo test <test_name>        # Single test
```

### Coaching benchmark

```bash
just bench                    # Run all scenarios
just bench --dry-run          # Preview prompts without LLM calls
just bench --filter <name>    # Run specific scenarios
just bench-report             # Generate report from results
```

---

## Configuration

Config lives at `~/.config/myro/config.toml`:

```toml
[codeforces]
handle = "your_cf_handle"

[coach]
# Set via Settings screen or manually here
# base_url = "https://openrouter.ai/api/v1"
# api_key = "sk-..."
# model = "anthropic/claude-sonnet-4"

[recommender]
target_probability = 0.35   # 0.1–0.9, lower = harder problems
```

Environment variable overrides:

| Variable | Overrides |
|----------|----------|
| `MYRO_LLM_BASE_URL` | `coach.base_url` |
| `MYRO_LLM_API_KEY` | `coach.api_key` |
| `MYRO_LLM_MODEL` | `coach.model` |
| `MYRO_COACH_MOCK` | Set to `1` for mock coach (no network) |

---

## License

[PolyForm Noncommercial 1.0.0](LICENSE). Free for personal, academic, and non-commercial use. See [LICENSE](LICENSE) for details.
