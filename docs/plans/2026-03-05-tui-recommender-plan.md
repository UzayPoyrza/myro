# TUI Recommender Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate the myro-predict recommender into the TUI with CF authentication, in-app submission, and TikTok-like seamless problem recommendation.

**Architecture:** New `auth.rs` module in myro-cf handles CF session login + submission via web scraping. TUI gains `myro-predict` dependency for on-the-fly user embedding fitting and prediction. Background threads (same mpsc pattern as coach) handle all network I/O. New app states: `HandlePrompt`, `Settings`. Recommender runs prediction → picks problem near target P(solve) → fetches statement on-demand.

**Tech Stack:** Rust, ratatui, reqwest (cookie jar), myro-predict (lib), myro-cf, AES-GCM (aes-gcm crate), regex

**Design doc:** `docs/plans/2026-03-05-tui-recommender-design.md`

---

## Key Files Reference

| File | Role |
|------|------|
| `crates/myro-cf/src/auth.rs` | NEW — CF login + submit via web scraping |
| `crates/myro-cf/src/client.rs` | Existing CfClient — unauthenticated API |
| `crates/myro-cf/src/lib.rs` | Re-export auth module |
| `crates/myro-cf/Cargo.toml` | Add regex, aes-gcm deps |
| `crates/myro-tui/src/app.rs` | AppState enum, App struct, state machine |
| `crates/myro-tui/src/ui.rs` | Rendering functions per state |
| `crates/myro-tui/src/state.rs` | UserState persistence |
| `crates/myro-tui/src/config.rs` | NEW — app config (codeforces + recommender sections) |
| `crates/myro-tui/src/recommend.rs` | NEW — recommender bridge (background thread + mpsc) |
| `crates/myro-tui/src/theme.rs` | Color/style helpers |
| `crates/myro-tui/src/main.rs` | Event loop |
| `crates/myro-tui/Cargo.toml` | Add myro-predict, rand, aes-gcm deps |
| `crates/myro-predict/src/lib.rs` | Re-exports model, db, history, cache |
| `crates/myro-predict/src/model/inference.rs` | fit_user_weighted, predict_all, build_observations_from_submissions |

## Important Patterns

**NixOS build:** All cargo commands must be wrapped in `nix develop /home/yunus/myro --command bash -c "cargo ..."`.

**Background threads in TUI:** The TUI is synchronous. Network/compute happens on background threads via `std::thread::spawn`. Communication via `mpsc::channel`. Main thread polls `rx.try_recv()` in `app.tick()` (every 100ms). See `crates/myro-tui/src/coach/bridge.rs` for the canonical pattern.

**Config loading:** See `crates/myro-coach/src/config.rs` — TOML at `~/.config/myro/config.toml`, parsed via serde, env var overrides applied after.

**Problem key formats differ:** Model uses `"1800:A"` (contest_id:index). TUI ProblemFile uses `"cf:1800A"` (cf:{contestId}{index}). Conversion needed.

**reqwest TLS:** Always use `default-features = false, features = ["json", "rustls-tls"]` for reqwest. The project uses rustls, not OpenSSL.

---

## Task 0: Add dependencies to Cargo.toml files

**Modify:** `crates/myro-cf/Cargo.toml`, `crates/myro-tui/Cargo.toml`

**Step 1: Update myro-cf Cargo.toml**

Add to `[dependencies]`:
```toml
regex = "1"
aes-gcm = "0.10"
md-5 = "0.10"
rand = "0.8"
```

These are needed for CSRF token extraction (regex), password encryption (aes-gcm + md-5), and ftaa generation (rand).

**Step 2: Update myro-tui Cargo.toml**

Add to `[dependencies]`:
```toml
myro-predict = { path = "../myro-predict" }
rand = "0.8"
toml = "0.8"
```

myro-predict is the lib crate with ProblemModel, fit_user_weighted, predict_all. rand for random problem selection. toml for config serialization.

**Step 3: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-cf -p myro-tui 2>&1"`
Expected: compiles (warnings OK)

**Step 4: Commit**

```bash
git add crates/myro-cf/Cargo.toml crates/myro-tui/Cargo.toml
git commit -m "feat: add deps for CF auth and TUI recommender"
```

---

## Task 1: CF Authentication — CSRF extraction and login

**Create:** `crates/myro-cf/src/auth.rs`
**Modify:** `crates/myro-cf/src/lib.rs`

**Step 1: Create auth.rs with CfAuthClient**

```rust
use anyhow::{bail, Context, Result};
use regex::Regex;
use reqwest::cookie::Jar;
use std::sync::Arc;

const CF_BASE: &str = "https://codeforces.com";

pub struct CfAuthClient {
    client: reqwest::Client,
    jar: Arc<Jar>,
    handle: Option<String>,
}

impl CfAuthClient {
    pub fn new() -> Self {
        let jar = Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("myro/0.1.0")
            .cookie_provider(jar.clone())
            .default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(
                    reqwest::header::ACCEPT_LANGUAGE,
                    "en-US,en;q=0.9".parse().unwrap(),
                );
                h
            })
            .build()
            .expect("Failed to build auth HTTP client");
        Self {
            client,
            jar,
            handle: None,
        }
    }

    /// Extract CSRF token from an HTML page body.
    fn extract_csrf(body: &str) -> Result<String> {
        let re = Regex::new(r"csrf='([^']+)'").unwrap();
        let caps = re
            .captures(body)
            .context("Cannot find CSRF token in page")?;
        Ok(caps[1].to_string())
    }

    /// Generate random 18-character hex string for ftaa.
    fn random_ftaa() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..18)
            .map(|_| format!("{:x}", rng.gen::<u8>() % 16))
            .collect()
    }

    /// Encrypt password using AES-GCM with key derived from handle.
    /// Key = MD5("glhf" + handle + "233"), truncated to 128 bits.
    pub fn encrypt_password(handle: &str, password: &str) -> Result<String> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes128Gcm, Nonce,
        };
        use md5::{Digest, Md5};

        let key_input = format!("glhf{}233", handle);
        let mut hasher = Md5::new();
        hasher.update(key_input.as_bytes());
        let key_bytes = hasher.finalize();

        let cipher = Aes128Gcm::new_from_slice(&key_bytes)
            .context("Failed to create AES cipher")?;
        let nonce = Nonce::from_slice(b"myro_cf_nonce"); // 12 bytes, fixed
        let ciphertext = cipher
            .encrypt(nonce, password.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        Ok(base64_encode(&ciphertext))
    }

    /// Decrypt password.
    pub fn decrypt_password(handle: &str, encrypted: &str) -> Result<String> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes128Gcm, Nonce,
        };
        use md5::{Digest, Md5};

        let key_input = format!("glhf{}233", handle);
        let mut hasher = Md5::new();
        hasher.update(key_input.as_bytes());
        let key_bytes = hasher.finalize();

        let cipher = Aes128Gcm::new_from_slice(&key_bytes)
            .context("Failed to create AES cipher")?;
        let nonce = Nonce::from_slice(b"myro_cf_nonce");
        let ciphertext = base64_decode(encrypted)?;
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).context("Decrypted password is not valid UTF-8")
    }

    /// Log in to Codeforces using handle + password.
    pub async fn login(&mut self, handle: &str, password: &str) -> Result<()> {
        // Step 1: GET /enter to get CSRF token
        let enter_url = format!("{}/enter", CF_BASE);
        let resp = self
            .client
            .get(&enter_url)
            .send()
            .await
            .context("Failed to fetch login page")?;
        let body = resp.text().await.context("Failed to read login page")?;
        let csrf = Self::extract_csrf(&body)?;

        // Step 2: POST /enter with credentials
        let params = [
            ("csrf_token", csrf.as_str()),
            ("handleOrEmail", handle),
            ("password", password),
            ("ftaa", &Self::random_ftaa()),
            ("bfaa", "f1b3f18c715565b589b7823cda7448ce"),
            ("_tta", "176"),
            ("remember", "on"),
            ("action", "enter"),
        ];

        let resp = self
            .client
            .post(&enter_url)
            .form(&params)
            .send()
            .await
            .context("Login request failed")?;

        let body = resp.text().await.context("Failed to read login response")?;

        // Verify login succeeded by checking for handle
        let handle_re = Regex::new(&format!(r#"handle = "{}""#, regex::escape(handle))).unwrap();
        if !handle_re.is_match(&body) {
            // Check for error message
            let err_re = Regex::new(r#"<span class="for__password"[^>]*>([^<]+)</span>"#).unwrap();
            if let Some(caps) = err_re.captures(&body) {
                bail!("Login failed: {}", &caps[1]);
            }
            bail!("Login failed: could not verify handle in response");
        }

        self.handle = Some(handle.to_string());
        Ok(())
    }

    /// Check if we have an active session.
    pub fn is_logged_in(&self) -> bool {
        self.handle.is_some()
    }

    pub fn handle(&self) -> Option<&str> {
        self.handle.as_deref()
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(data: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .context("Base64 decode failed")
}
```

**Step 2: Add `pub mod auth;` to `crates/myro-cf/src/lib.rs`**

Add after existing module declarations.

**Step 3: Add `base64 = "0.22"` to myro-cf Cargo.toml** (if not already present)

**Step 4: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-cf 2>&1"`
Expected: compiles

**Step 5: Commit**

```bash
git add crates/myro-cf/src/auth.rs crates/myro-cf/src/lib.rs crates/myro-cf/Cargo.toml
git commit -m "feat(myro-cf): add CF session auth with CSRF login"
```

---

## Task 2: CF Solution Submission

**Modify:** `crates/myro-cf/src/auth.rs`

**Step 1: Add submit_solution method to CfAuthClient**

```rust
/// Submit a solution to a Codeforces problem.
/// Returns the submission ID on success.
pub async fn submit_solution(
    &self,
    contest_id: i64,
    problem_index: &str,
    source_code: &str,
    lang_id: &str,
) -> Result<i64> {
    if !self.is_logged_in() {
        bail!("Not logged in — call login() first");
    }

    // GET the submit page to extract CSRF
    let submit_url = format!("{}/contest/{}/submit", CF_BASE, contest_id);
    let resp = self
        .client
        .get(&submit_url)
        .send()
        .await
        .context("Failed to fetch submit page")?;
    let body = resp.text().await.context("Failed to read submit page")?;
    let csrf = Self::extract_csrf(&body)?;

    // POST the solution
    let post_url = format!("{}?csrf_token={}", submit_url, csrf);
    let params = [
        ("csrf_token", csrf.as_str()),
        ("ftaa", &Self::random_ftaa()),
        ("bfaa", "f1b3f18c715565b589b7823cda7448ce"),
        ("action", "submitSolutionFormSubmitted"),
        ("submittedProblemIndex", problem_index),
        ("programTypeId", lang_id),
        ("contestId", &contest_id.to_string()),
        ("source", source_code),
        ("tabSize", "4"),
        ("_tta", "594"),
        ("sourceCodeConfirmed", "true"),
    ];

    let resp = self
        .client
        .post(&post_url)
        .form(&params)
        .send()
        .await
        .context("Submit request failed")?;

    let body = resp.text().await.context("Failed to read submit response")?;

    // Check for errors
    let err_re = Regex::new(r#"error[^>]*>([^<]+)<"#).unwrap();
    if let Some(caps) = err_re.captures(&body) {
        let msg = caps[1].trim().to_string();
        if !msg.is_empty() {
            bail!("Submission error: {}", msg);
        }
    }

    // Extract submission ID from the response/redirect
    // CF redirects to /contest/{id}/my after successful submit
    // We can get the latest submission from user.status instead
    Ok(0) // placeholder — verdict polling uses user.status API
}
```

**Step 2: Add verdict polling method**

```rust
/// Poll for the verdict of the most recent submission.
/// Returns (verdict, problem_index) or None if still judging.
pub async fn poll_latest_verdict(
    &self,
    contest_id: i64,
) -> Result<Option<(String, String)>> {
    let handle = self
        .handle
        .as_deref()
        .context("Not logged in")?;

    // Use the public API — no auth needed
    let url = format!(
        "https://codeforces.com/api/user.status?handle={}&from=1&count=5",
        handle
    );
    let resp = self
        .client
        .get(&url)
        .send()
        .await
        .context("Failed to poll verdict")?;
    let body = resp.text().await?;

    let parsed: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse verdict response")?;

    if parsed["status"] != "OK" {
        bail!("CF API error while polling verdict");
    }

    if let Some(submissions) = parsed["result"].as_array() {
        for sub in submissions {
            if sub["contestId"].as_i64() == Some(contest_id) {
                let verdict = sub["verdict"]
                    .as_str()
                    .unwrap_or("TESTING")
                    .to_string();
                let prob_idx = sub["problem"]["index"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                if verdict == "TESTING" {
                    return Ok(None); // still judging
                }
                return Ok(Some((verdict, prob_idx)));
            }
        }
    }

    Ok(None) // no matching submission found yet
}
```

**Step 3: Add Python language constant**

```rust
/// CF language ID for PyPy 3-64 (Python 3).
pub const LANG_PYPY3: &str = "70";
```

**Step 4: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-cf 2>&1"`

**Step 5: Commit**

```bash
git add crates/myro-cf/src/auth.rs
git commit -m "feat(myro-cf): add solution submission and verdict polling"
```

---

## Task 3: App Config (codeforces + recommender sections)

**Create:** `crates/myro-tui/src/config.rs`
**Modify:** `crates/myro-tui/src/app.rs` (add config field to App)

**Step 1: Create config.rs**

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// App-level config stored in ~/.config/myro/config.toml
/// Separate from CoachConfig — this covers codeforces auth and recommender settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub codeforces: CfConfig,
    #[serde(default)]
    pub recommender: RecommenderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CfConfig {
    pub handle: Option<String>,
    /// AES-GCM encrypted, base64-encoded password.
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommenderConfig {
    /// Target solve probability for recommendations (0.1–0.9).
    #[serde(default = "default_target_p")]
    pub target_probability: f64,
    /// Path to the problem model file.
    #[serde(default = "default_model_path")]
    pub model_path: PathBuf,
}

fn default_target_p() -> f64 {
    0.5
}

fn default_model_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("problem_model.bin.gz")
}

impl Default for RecommenderConfig {
    fn default() -> Self {
        Self {
            target_probability: default_target_p(),
            model_path: default_model_path(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            codeforces: CfConfig::default(),
            recommender: RecommenderConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load from ~/.config/myro/config.toml. Returns default if file absent.
    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save to ~/.config/myro/config.toml.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(&path, contents)
            .context("Failed to write config file")?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("myro")
            .join("config.toml")
    }
}
```

**Step 2: Add `mod config;` to app.rs or main.rs**

Add `mod config;` in the module declarations section. Add `pub use config::AppConfig;`.

**Step 3: Add `app_config: AppConfig` to the `App` struct**

In `crates/myro-tui/src/app.rs`, add field to `App` struct (line ~20):
```rust
pub app_config: config::AppConfig,
```

In `App::new()`, load it:
```rust
let app_config = config::AppConfig::load();
```

Use `app_config.codeforces.handle` instead of `user_state.name` to decide initial state.

**Step 4: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-tui 2>&1"`

**Step 5: Commit**

```bash
git add crates/myro-tui/src/config.rs crates/myro-tui/src/app.rs
git commit -m "feat(myro-tui): add AppConfig with codeforces and recommender sections"
```

---

## Task 4: HandlePrompt state (replaces NamePrompt)

**Modify:** `crates/myro-tui/src/app.rs`, `crates/myro-tui/src/ui.rs`, `crates/myro-tui/src/state.rs`

**Step 1: Replace NamePrompt with HandlePrompt in AppState**

In `app.rs` (line 27), change:
```rust
NamePrompt {
    input: String,
},
```
to:
```rust
HandlePrompt {
    input: String,
    error: Option<String>,
    validating: bool,
    validate_rx: Option<mpsc::Receiver<Result<myro_cf::CfUser, String>>>,
},
```

**Step 2: Update App::new() initial state logic**

```rust
let initial_state = if app_config.codeforces.handle.is_some() {
    AppState::Home { selected: 0 }
} else {
    AppState::HandlePrompt {
        input: String::new(),
        error: None,
        validating: false,
        validate_rx: None,
    }
};
```

**Step 3: Add handle_handle_prompt_key method**

Replace `handle_name_prompt_key`. On `Enter`:
- If input is empty, show error "Handle cannot be empty"
- If not validating, spawn a background thread that calls `CfClient::new().fetch_user_info(&input)` via a one-shot tokio runtime
- Set `validating: true`, store receiver
- On `Backspace`, `Char` — same text editing as before

**Step 4: Add validation polling in tick()**

In `app.tick()`, if state is `HandlePrompt { validating: true, validate_rx: Some(rx), .. }`:
- `try_recv()` on the receiver
- On `Ok(Ok(user))` — save handle to `app_config.codeforces`, save `app_config`, update `user_state.name` to handle, transition to `Home { selected: 0 }`
- On `Ok(Err(msg))` — set `error: Some(msg)`, `validating: false`

**Step 5: Update all NamePrompt references**

Search for `NamePrompt` across the crate. Update:
- `ui.rs:render` match arm → `HandlePrompt`
- `ui.rs:render_name_prompt` → `render_handle_prompt` — change prompt text to `"Enter your Codeforces handle"`, show error if present, show spinner if validating
- `app.rs:handle_key` dispatch → `HandlePrompt`

**Step 6: Update UserState**

In `state.rs`, keep `name` field for backward compat but populate it from CF handle. The `UserState.name` becomes derived from the config handle.

**Step 7: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-tui 2>&1"`

**Step 8: Commit**

```bash
git add crates/myro-tui/src/app.rs crates/myro-tui/src/ui.rs crates/myro-tui/src/state.rs
git commit -m "feat(myro-tui): replace NamePrompt with HandlePrompt (CF handle validation)"
```

---

## Task 5: Settings menu state

**Modify:** `crates/myro-tui/src/app.rs`, `crates/myro-tui/src/ui.rs`

**Step 1: Add Settings variant to AppState**

```rust
Settings {
    selected: usize,
    editing: Option<String>,  // current edit buffer, None = not editing
},
```

**Step 2: Define settings fields**

```rust
pub const SETTINGS_FIELDS: &[(&str, &str)] = &[
    ("CF Handle", "codeforces.handle"),
    ("CF Password", "codeforces.password"),
    ("Target P(solve)", "recommender.target_probability"),
    ("Model Path", "recommender.model_path"),
];
```

**Step 3: Update MENU_ITEMS**

```rust
pub const MENU_ITEMS: &[&str] = &[
    "Start training",
    "Suggested problem",
    "Settings",
];
```

**Step 4: Update handle_home_key**

- Index 0: `start_training()` (unchanged)
- Index 1: `start_suggested_problem()` (new — Task 7)
- Index 2: transition to `AppState::Settings { selected: 0, editing: None }`

**Step 5: Add handle_settings_key method**

- `j`/`Down`: increment selected (mod len)
- `k`/`Up`: decrement selected
- `Enter` (not editing): read current value from `app_config` into `editing: Some(value)`
- `Enter` (editing): write value back to `app_config`, call `app_config.save()`, set `editing: None`
- `Esc` (editing): discard, set `editing: None`
- `Esc`/`q` (not editing): transition to `Home { selected: 2 }`
- `Char(c)`, `Backspace` (editing): standard text editing on the buffer

For password field: store the encrypted form in config. When editing, show plaintext in the buffer. On confirm, encrypt with `CfAuthClient::encrypt_password(handle, &plaintext)` and save the `"encrypted:{base64}"` form.

For target_probability: validate it's a float in `[0.1, 0.9]` on confirm. Reject invalid input with a status message.

**Step 6: Add render_settings to ui.rs**

Renders a list of labeled fields. Each row: `"  {label}:  {value}"`. Selected row highlighted with `ARROW_RIGHT`. If editing, show the edit buffer with a cursor. Password field masked with `"****"` when not editing.

**Step 7: Update render() match and handle_key dispatch**

Add `AppState::Settings { .. }` arm to both.

**Step 8: Verify compilation and test manually**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-tui 2>&1"`

**Step 9: Commit**

```bash
git add crates/myro-tui/src/app.rs crates/myro-tui/src/ui.rs
git commit -m "feat(myro-tui): add Settings menu with CF handle, password, target probability"
```

---

## Task 6: Recommender bridge (background thread + mpsc)

**Create:** `crates/myro-tui/src/recommend.rs`
**Modify:** `crates/myro-tui/src/app.rs` (add module)

This follows the exact same pattern as `crates/myro-tui/src/coach/bridge.rs`.

**Step 1: Create recommend.rs**

```rust
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Requests sent from main thread to recommender thread.
pub enum RecommendRequest {
    /// Fetch user history from CF and fit embedding.
    FetchAndFit {
        handle: String,
        model_path: PathBuf,
    },
    /// Pick a problem near target probability.
    Recommend {
        target_p: f64,
        solved_keys: Vec<String>,
    },
    /// Fetch a problem statement by contest_id and index.
    FetchProblem {
        contest_id: i64,
        index: String,
    },
    /// Submit a solution.
    Submit {
        contest_id: i64,
        index: String,
        source_code: String,
        handle: String,
        password: String,
    },
    /// Poll for verdict.
    PollVerdict {
        contest_id: i64,
    },
    /// Record a solve/fail and refit.
    RecordAndRefit {
        problem_key: String,
        solved: bool,
    },
    Quit,
}

/// Events sent from recommender thread back to main thread.
pub enum RecommendEvent {
    /// User embedding fitted successfully.
    EmbeddingReady {
        num_observations: usize,
        user_rating: Option<i32>,
    },
    /// A problem has been recommended.
    ProblemRecommended {
        contest_id: i64,
        index: String,
        predicted_p: f64,
        rating: Option<i32>,
        tags: Vec<String>,
    },
    /// Problem statement fetched and parsed.
    ProblemFetched {
        statement: myro_cf::ProblemStatement,
    },
    /// Solution submitted, waiting for verdict.
    Submitted,
    /// Verdict received.
    Verdict {
        verdict: String,
        problem_index: String,
    },
    /// Embedding refitted after recording a solve/fail.
    Refitted,
    /// Error occurred.
    Error {
        message: String,
    },
    /// Status update for display.
    Status {
        message: String,
    },
}

pub struct RecommendHandle {
    pub request_tx: mpsc::Sender<RecommendRequest>,
    pub event_rx: mpsc::Receiver<RecommendEvent>,
}

/// Spawn the recommender background thread.
pub fn spawn_recommender() -> RecommendHandle {
    let (req_tx, req_rx) = mpsc::channel::<RecommendRequest>();
    let (evt_tx, evt_rx) = mpsc::channel::<RecommendEvent>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime for recommender");

        rt.block_on(async {
            recommender_loop(req_rx, evt_tx).await;
        });
    });

    RecommendHandle {
        request_tx: req_tx,
        event_rx: evt_rx,
    }
}

async fn recommender_loop(
    req_rx: mpsc::Receiver<RecommendRequest>,
    evt_tx: mpsc::Sender<RecommendEvent>,
) {
    use myro_predict::model::inference::{
        build_observations_from_submissions, fit_user_weighted, predict_all,
        DEFAULT_HALF_LIFE_DAYS,
    };
    use myro_predict::model::types::{ProblemModel, UserParams};
    use myro_predict::db::model_store;

    let mut model: Option<ProblemModel> = None;
    let mut user_params: Option<UserParams> = None;
    let mut predictions: Option<Vec<f64>> = None;
    let mut auth_client = myro_cf::auth::CfAuthClient::new();
    let cf_client = myro_cf::CfClient::new();
    let mut local_history = myro_predict::history::SolveHistory::load();
    let mut solved_keys_set: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        let req = match req_rx.recv() {
            Ok(r) => r,
            Err(_) => break, // channel closed
        };

        match req {
            RecommendRequest::FetchAndFit { handle, model_path } => {
                // Load model
                let _ = evt_tx.send(RecommendEvent::Status {
                    message: "Loading model...".into(),
                });
                match model_store::load_problem_model(&model_path) {
                    Ok(m) => model = Some(m),
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Failed to load model: {}", e),
                        });
                        continue;
                    }
                }

                // Fetch submissions
                let _ = evt_tx.send(RecommendEvent::Status {
                    message: format!("Fetching history for {}...", handle),
                });
                let submissions = match cf_client.fetch_user_status(&handle).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Failed to fetch submissions: {}", e),
                        });
                        continue;
                    }
                };

                // Fetch user info for rating
                let user_rating = cf_client
                    .fetch_user_info(&handle)
                    .await
                    .ok()
                    .and_then(|u| u.rating);

                // Build observations and fit
                let now_ts = chrono::Utc::now().timestamp();
                let m = model.as_ref().unwrap();
                let (obs, sk) = build_observations_from_submissions(
                    m, &submissions, now_ts, DEFAULT_HALF_LIFE_DAYS,
                );
                solved_keys_set = sk.into_iter()
                    .filter(|(_, v)| *v)
                    .map(|(k, _)| k)
                    .collect();

                let params = fit_user_weighted(m, &obs, 0.01, 100, 0.01);
                let preds = predict_all(&params, m);
                user_params = Some(params);
                predictions = Some(preds);

                let _ = evt_tx.send(RecommendEvent::EmbeddingReady {
                    num_observations: obs.len(),
                    user_rating,
                });
            }

            RecommendRequest::Recommend { target_p, solved_keys } => {
                let m = match model.as_ref() {
                    Some(m) => m,
                    None => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: "Model not loaded".into(),
                        });
                        continue;
                    }
                };
                let preds = match predictions.as_ref() {
                    Some(p) => p,
                    None => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: "Predictions not computed".into(),
                        });
                        continue;
                    }
                };

                let solved_set: std::collections::HashSet<&str> =
                    solved_keys.iter().map(|s| s.as_str()).collect();

                // Build candidate list: unsolved, rated, near target_p
                let margin = 0.1;
                let mut candidates: Vec<(usize, &str, f64, Option<i32>)> = Vec::new();
                for (key, &idx) in &m.problem_index {
                    if solved_set.contains(key.as_str()) || solved_keys_set.contains(key.as_str()) {
                        continue;
                    }
                    let p = preds[idx];
                    if (p - target_p).abs() <= margin {
                        let rating = m.problem_ratings.get(idx).and_then(|r| *r);
                        candidates.push((idx, key.as_str(), p, rating));
                    }
                }

                if candidates.is_empty() {
                    let _ = evt_tx.send(RecommendEvent::Error {
                        message: format!(
                            "No unsolved problems near P(solve)={:.0}% ± 10%",
                            target_p * 100.0
                        ),
                    });
                    continue;
                }

                // Pick random candidate
                use rand::seq::SliceRandom;
                let (idx, key, pred_p, rating) =
                    *candidates.choose(&mut rand::thread_rng()).unwrap();
                let parts: Vec<&str> = key.split(':').collect();
                let (contest_id, index) = if parts.len() == 2 {
                    (parts[0].parse::<i64>().unwrap_or(0), parts[1].to_string())
                } else {
                    (0, key.to_string())
                };

                let tags = m.problem_tags
                    .get(idx)
                    .cloned()
                    .unwrap_or_default();

                let _ = evt_tx.send(RecommendEvent::ProblemRecommended {
                    contest_id,
                    index,
                    predicted_p: pred_p,
                    rating,
                    tags,
                });
            }

            RecommendRequest::FetchProblem { contest_id, index } => {
                let _ = evt_tx.send(RecommendEvent::Status {
                    message: format!("Fetching problem {}{}...", contest_id, index),
                });
                match cf_client.fetch_problem_html(contest_id, &index).await {
                    Ok(html) => {
                        match myro_cf::parser::parse_problem_html(&html, contest_id, &index) {
                            Ok(statement) => {
                                let _ = evt_tx.send(RecommendEvent::ProblemFetched { statement });
                            }
                            Err(e) => {
                                let _ = evt_tx.send(RecommendEvent::Error {
                                    message: format!("Failed to parse problem: {}", e),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Failed to fetch problem: {}", e),
                        });
                    }
                }
            }

            RecommendRequest::Submit {
                contest_id,
                index,
                source_code,
                handle,
                password,
            } => {
                // Login if needed
                if !auth_client.is_logged_in() {
                    let _ = evt_tx.send(RecommendEvent::Status {
                        message: "Logging in to Codeforces...".into(),
                    });
                    if let Err(e) = auth_client.login(&handle, &password).await {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Login failed: {}", e),
                        });
                        continue;
                    }
                }

                let _ = evt_tx.send(RecommendEvent::Status {
                    message: "Submitting solution...".into(),
                });
                match auth_client
                    .submit_solution(
                        contest_id,
                        &index,
                        &source_code,
                        myro_cf::auth::LANG_PYPY3,
                    )
                    .await
                {
                    Ok(_) => {
                        let _ = evt_tx.send(RecommendEvent::Submitted);
                    }
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Submit failed: {}", e),
                        });
                    }
                }
            }

            RecommendRequest::PollVerdict { contest_id } => {
                match auth_client.poll_latest_verdict(contest_id).await {
                    Ok(Some((verdict, idx))) => {
                        let _ = evt_tx.send(RecommendEvent::Verdict {
                            verdict,
                            problem_index: idx,
                        });
                    }
                    Ok(None) => {
                        // Still judging — caller should poll again
                    }
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Verdict poll failed: {}", e),
                        });
                    }
                }
            }

            RecommendRequest::RecordAndRefit { problem_key, solved } => {
                // Record in local history
                let now = chrono::Utc::now().timestamp();
                local_history.record(&problem_key, solved, now);
                if let Err(e) = local_history.save() {
                    let _ = evt_tx.send(RecommendEvent::Error {
                        message: format!("Failed to save history: {}", e),
                    });
                }

                if solved {
                    solved_keys_set.insert(problem_key);
                }

                // Refit embedding if model loaded
                if let Some(m) = model.as_ref() {
                    let obs: Vec<myro_predict::model::types::WeightedObservation> =
                        local_history.entries.iter().filter_map(|e| {
                            let idx = m.problem_index.get(&e.problem_id)?;
                            let days_ago = (now - e.timestamp) as f64 / 86400.0;
                            let w = myro_predict::model::inference::time_decay_weight(
                                days_ago,
                                myro_predict::model::inference::DEFAULT_HALF_LIFE_DAYS,
                            );
                            Some(myro_predict::model::types::WeightedObservation {
                                problem_idx: *idx,
                                solved: e.solved,
                                weight: w,
                            })
                        }).collect();

                    let params = myro_predict::model::inference::fit_user_weighted(
                        m, &obs, 0.01, 100, 0.01,
                    );
                    let preds = myro_predict::model::inference::predict_all(&params, m);
                    user_params = Some(params);
                    predictions = Some(preds);
                }

                let _ = evt_tx.send(RecommendEvent::Refitted);
            }

            RecommendRequest::Quit => break,
        }
    }
}
```

**Step 2: Add `mod recommend;` to app.rs module declarations**

**Step 3: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-tui 2>&1"`

Note: Will need `chrono` added to myro-tui Cargo.toml: `chrono = "0.4"`

**Step 4: Commit**

```bash
git add crates/myro-tui/src/recommend.rs crates/myro-tui/src/app.rs crates/myro-tui/Cargo.toml
git commit -m "feat(myro-tui): add recommender bridge with background thread"
```

---

## Task 7: Suggested problem flow

**Modify:** `crates/myro-tui/src/app.rs`, `crates/myro-tui/src/ui.rs`

**Step 1: Add recommender state to App struct**

```rust
pub recommend: Option<recommend::RecommendHandle>,
pub recommend_status: Option<String>,
pub pending_problem: Option<(i64, String, f64, Option<i32>)>,  // contest_id, index, pred_p, rating
```

Initialize all to `None` in `App::new()`.

**Step 2: Add start_suggested_problem method**

Called from Home menu index 1:

```rust
fn start_suggested_problem(&mut self) {
    let handle = match &self.app_config.codeforces.handle {
        Some(h) => h.clone(),
        None => {
            self.status_message = Some("Set your CF handle in Settings first".into());
            return;
        }
    };

    // Spawn recommender thread if not already running
    if self.recommend.is_none() {
        self.recommend = Some(recommend::spawn_recommender());
    }

    let rec = self.recommend.as_ref().unwrap();
    let _ = rec.request_tx.send(recommend::RecommendRequest::FetchAndFit {
        handle,
        model_path: self.app_config.recommender.model_path.clone(),
    });

    self.recommend_status = Some("Loading model and fetching history...".into());
}
```

**Step 3: Add recommender event polling in tick()**

In `app.tick()`, after coach polling:

```rust
if let Some(rec) = &self.recommend {
    while let Ok(event) = rec.event_rx.try_recv() {
        match event {
            RecommendEvent::EmbeddingReady { num_observations, user_rating } => {
                self.recommend_status = Some(format!(
                    "Fitted on {} observations. Picking problem...",
                    num_observations
                ));
                // Trigger recommendation
                let solved: Vec<String> = self.user_state.solved.clone();
                let _ = rec.request_tx.send(RecommendRequest::Recommend {
                    target_p: self.app_config.recommender.target_probability,
                    solved_keys: solved,
                });
            }
            RecommendEvent::ProblemRecommended { contest_id, index, predicted_p, rating, .. } => {
                self.recommend_status = Some(format!(
                    "Fetching problem {}{}...", contest_id, index
                ));
                self.pending_problem = Some((contest_id, index.clone(), predicted_p, rating));
                let _ = rec.request_tx.send(RecommendRequest::FetchProblem {
                    contest_id,
                    index,
                });
            }
            RecommendEvent::ProblemFetched { statement } => {
                self.recommend_status = None;
                // Transition to Solving with the fetched problem
                self.start_solving_recommended(statement);
            }
            RecommendEvent::Submitted => {
                self.recommend_status = Some("Submitted! Waiting for verdict...".into());
                // Start verdict polling (handled in tick via a counter)
            }
            RecommendEvent::Verdict { verdict, .. } => {
                self.recommend_status = None;
                self.handle_verdict(&verdict);
            }
            RecommendEvent::Refitted => {
                // Embedding updated — next recommendation will use new params
            }
            RecommendEvent::Error { message } => {
                self.status_message = Some(format!("Error: {}", message));
                self.recommend_status = None;
            }
            RecommendEvent::Status { message } => {
                self.recommend_status = Some(message);
            }
        }
    }
}
```

**Step 4: Add start_solving_recommended method**

Similar to `start_solving_problem` but takes a `ProblemStatement` directly (no `ProblemFile` needed). Create a minimal `ProblemFile` from the statement for coach compatibility, or make coach optional for recommended problems.

**Step 5: Show loading overlay on Home screen**

In `render_home`, if `app.recommend_status.is_some()`, render a centered overlay box with the status message and a spinner animation (use `app.tick` counter for rotation).

**Step 6: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-tui 2>&1"`

**Step 7: Commit**

```bash
git add crates/myro-tui/src/app.rs crates/myro-tui/src/ui.rs
git commit -m "feat(myro-tui): add suggested problem flow with loading overlay"
```

---

## Task 8: /submit command

**Modify:** `crates/myro-tui/src/app.rs`

**Step 1: Add /submit to execute_command**

In the `execute_command` method (after `"hint"` handler):

```rust
"submit" => {
    if let AppState::Solving { solution_path, problem, .. } = &self.state {
        let handle = match &self.app_config.codeforces.handle {
            Some(h) => h.clone(),
            None => {
                self.status_message = Some("Set CF handle in Settings first".into());
                return;
            }
        };

        // Get password — prompt if not saved
        let password = match &self.app_config.codeforces.password {
            Some(enc) if enc.starts_with("encrypted:") => {
                match myro_cf::auth::CfAuthClient::decrypt_password(
                    &handle, &enc["encrypted:".len()..],
                ) {
                    Ok(p) => p,
                    Err(_) => {
                        self.status_message = Some("Invalid saved password. Update in Settings.".into());
                        return;
                    }
                }
            }
            Some(plain) => plain.clone(),
            None => {
                self.status_message = Some("Set CF password in Settings first".into());
                return;
            }
        };

        // Read current editor content
        let source_code = /* extract from editor_state */;
        let contest_id = problem.contest_id;
        let index = problem.index.clone();

        // Spawn recommender if needed
        if self.recommend.is_none() {
            self.recommend = Some(recommend::spawn_recommender());
        }

        let rec = self.recommend.as_ref().unwrap();
        let _ = rec.request_tx.send(recommend::RecommendRequest::Submit {
            contest_id,
            index,
            source_code,
            handle,
            password,
        });

        self.recommend_status = Some("Submitting...".into());
    }
}
```

**Step 2: Add verdict handling**

```rust
fn handle_verdict(&mut self, verdict: &str) {
    let is_ac = verdict == "OK";
    let verdict_display = match verdict {
        "OK" => "Accepted!",
        "WRONG_ANSWER" => "Wrong Answer",
        "TIME_LIMIT_EXCEEDED" => "Time Limit Exceeded",
        "RUNTIME_ERROR" => "Runtime Error",
        "MEMORY_LIMIT_EXCEEDED" => "Memory Limit Exceeded",
        "COMPILATION_ERROR" => "Compilation Error",
        other => other,
    };

    if is_ac {
        self.status_message = Some(format!("✓ {}", verdict_display));
        // Record solve, update state, get next problem
        if let AppState::Solving { problem, .. } = &self.state {
            let key = format!("{}:{}", problem.contest_id, problem.index);
            let problem_id = format!("cf:{}{}", problem.contest_id, problem.index);
            self.user_state.solved.push(problem_id);
            let _ = state::save_state(&self.user_state);

            if let Some(rec) = &self.recommend {
                let _ = rec.request_tx.send(recommend::RecommendRequest::RecordAndRefit {
                    problem_key: key,
                    solved: true,
                });
                // After refit, automatically get next problem
                let _ = rec.request_tx.send(recommend::RecommendRequest::Recommend {
                    target_p: self.app_config.recommender.target_probability,
                    solved_keys: self.user_state.solved.clone(),
                });
            }
        }
    } else {
        self.status_message = Some(format!("✗ {}", verdict_display));
        // Record failure
        if let AppState::Solving { problem, .. } = &self.state {
            let key = format!("{}:{}", problem.contest_id, problem.index);
            if let Some(rec) = &self.recommend {
                let _ = rec.request_tx.send(recommend::RecommendRequest::RecordAndRefit {
                    problem_key: key,
                    solved: false,
                });
            }
        }
    }
}
```

**Step 3: Add verdict polling in tick()**

After submitting, poll every ~30 ticks (3 seconds):

```rust
// In tick(), within Solving state:
if self.recommend_status.as_deref() == Some("Submitted! Waiting for verdict...") {
    if self.tick % 30 == 0 {
        if let Some(rec) = &self.recommend {
            if let AppState::Solving { problem, .. } = &self.state {
                let _ = rec.request_tx.send(recommend::RecommendRequest::PollVerdict {
                    contest_id: problem.contest_id,
                });
            }
        }
    }
}
```

**Step 4: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-tui 2>&1"`

**Step 5: Commit**

```bash
git add crates/myro-tui/src/app.rs
git commit -m "feat(myro-tui): add /submit command with verdict polling"
```

---

## Task 9: /isuck command

**Modify:** `crates/myro-tui/src/app.rs`, `crates/myro-tui/src/state.rs`

**Step 1: Add isuck_explained to UserState**

```rust
pub struct UserState {
    pub name: Option<String>,
    pub solved: Vec<String>,
    #[serde(default)]
    pub isuck_explained: bool,
}
```

**Step 2: Add /isuck to execute_command**

```rust
"isuck" => {
    if !self.user_state.isuck_explained {
        // Show explanation popup
        self.status_message = Some(
            "⚠ /isuck marks this problem as too hard and moves to the next one. \
             Your predictions will update. Press /isuck again to confirm.".into()
        );
        self.user_state.isuck_explained = true;
        let _ = state::save_state(&self.user_state);
        return;
    }

    if let AppState::Solving { problem, .. } = &self.state {
        let key = format!("{}:{}", problem.contest_id, problem.index);

        // Record failure
        if let Some(rec) = &self.recommend {
            let _ = rec.request_tx.send(recommend::RecommendRequest::RecordAndRefit {
                problem_key: key,
                solved: false,
            });
            // Get next problem immediately
            let _ = rec.request_tx.send(recommend::RecommendRequest::Recommend {
                target_p: self.app_config.recommender.target_probability,
                solved_keys: self.user_state.solved.clone(),
            });
        }

        self.recommend_status = Some("Finding next problem...".into());
    }
}
```

**Step 3: Handle seamless transition in tick()**

When `RecommendEvent::ProblemFetched` arrives while already in `Solving` state, call `start_solving_recommended(statement)` — this replaces the current problem seamlessly.

This is already handled by the tick() code from Task 7 — the `ProblemFetched` event handler calls `start_solving_recommended` regardless of current state.

**Step 4: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-tui 2>&1"`

**Step 5: Commit**

```bash
git add crates/myro-tui/src/app.rs crates/myro-tui/src/state.rs
git commit -m "feat(myro-tui): add /isuck command for graceful skip with negative signal"
```

---

## Task 10: UI polish — status display in Solving state

**Modify:** `crates/myro-tui/src/ui.rs`

**Step 1: Show recommendation info in Solving state**

When a recommended problem is being solved, show a small info bar at the top or bottom:
- `"P(solve): 52% | Rating: 1600 | Tags: dp, math"`
- During submission: `"Submitting..."` with spinner
- After verdict: `"✓ Accepted!"` (green) or `"✗ Wrong Answer"` (red)

Use `accent_style()` for prediction info, `success_style()` for AC, `fail_style()` for rejection.

**Step 2: Show spinner during loading**

When `app.recommend_status.is_some()`, show the status text with a rotating spinner character from `['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']` indexed by `app.tick % 10`.

**Step 3: Verify compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check -p myro-tui 2>&1"`

**Step 4: Commit**

```bash
git add crates/myro-tui/src/ui.rs
git commit -m "feat(myro-tui): add recommendation info bar and submission status display"
```

---

## Task 11: Integration testing and cleanup

**Step 1: Full workspace compilation**

Run: `nix develop /home/yunus/myro --command bash -c "cargo check 2>&1"`

**Step 2: Run all tests**

Run: `nix develop /home/yunus/myro --command bash -c "cargo test 2>&1"`

**Step 3: Clippy**

Run: `nix develop /home/yunus/myro --command bash -c "cargo clippy 2>&1"`

Fix any warnings.

**Step 4: Manual smoke test**

Run: `nix develop /home/yunus/myro --command bash -c "cargo run -p myro-tui 2>&1"`

Test:
- Handle prompt appears on first run
- Settings menu is accessible and editable
- "Suggested problem" shows loading → problem (requires model file and network)
- `/submit` and `/isuck` commands work

**Step 5: Update CLAUDE.md**

Add to the myro-tui section:
- New commands: `/submit`, `/isuck`
- New app states: `HandlePrompt`, `Settings`
- Config file sections: `[codeforces]`, `[recommender]`
- Recommender architecture: background thread, on-the-fly embedding fitting

**Step 6: Final commit**

```bash
git add -A
git commit -m "docs: update CLAUDE.md with recommender integration details"
```

---

## Verification Checklist

1. `cargo check` — full workspace compiles
2. `cargo test` — all tests pass
3. `cargo clippy` — no warnings
4. Manual: Handle prompt → enter CF handle → validates → Home
5. Manual: Settings → edit CF handle, password, target probability → saves to config.toml
6. Manual: Suggested problem → loads model → fetches history → recommends problem → opens editor
7. Manual: `/submit` → authenticates → submits → shows verdict
8. Manual: `/isuck` → shows explanation first time → records failure → gets new problem seamlessly
9. Manual: After AC → records solve → gets new problem seamlessly (TikTok flow)
