use anyhow::{bail, Context, Result};
use regex::Regex;
use reqwest::cookie::Jar;
use std::sync::Arc;

const CF_BASE: &str = "https://codeforces.com";

fn default_user_agent() -> &'static str {
    if cfg!(target_os = "macos") {
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7; rv:128.0) Gecko/20100101 Firefox/128.0"
    } else {
        "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0"
    }
}

pub struct CfAuthClient {
    client: reqwest::Client,
    #[allow(dead_code)]
    jar: Arc<Jar>,
    handle: Option<String>,
    /// Raw cookies for curl-based requests (bypasses Cloudflare TLS fingerprinting).
    raw_cookies: Vec<(String, String)>,
    /// User-agent string matching the browser that created cf_clearance.
    raw_user_agent: String,
}

impl Default for CfAuthClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CfAuthClient {
    pub fn new() -> Self {
        let jar = Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(default_user_agent())
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
            raw_cookies: Vec::new(),
            raw_user_agent: default_user_agent().to_string(),
        }
    }

    /// Load cookies imported from the browser and rebuild the HTTP client
    /// with the given user-agent (must match the browser that created cf_clearance).
    pub fn load_cookies(
        &mut self,
        handle: &str,
        cookies: &[(String, String)],
        user_agent: &str,
    ) {
        let jar = Arc::new(Jar::default());
        let url: reqwest::Url = "https://codeforces.com/".parse().unwrap();
        for (name, value) in cookies {
            jar.add_cookie_str(
                &format!("{}={}; Domain=.codeforces.com; Path=/", name, value),
                &url,
            );
        }
        self.jar = jar.clone();
        self.client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(user_agent)
            .cookie_provider(jar)
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
        self.handle = Some(handle.to_string());
        self.raw_cookies = cookies.to_vec();
        self.raw_user_agent = user_agent.to_string();
    }

    /// Write cookies to a Netscape cookie jar file for curl.
    fn write_cookie_jar(&self) -> Result<std::path::PathBuf> {
        let path = std::env::temp_dir().join("myro_curl_cookies.txt");
        let mut contents = String::from("# Netscape HTTP Cookie File\n");
        for (name, value) in &self.raw_cookies {
            // Format: domain \t include_subdomains \t path \t secure \t expiry \t name \t value
            contents.push_str(&format!(
                ".codeforces.com\tTRUE\t/\tTRUE\t0\t{}\t{}\n",
                name, value
            ));
        }
        std::fs::write(&path, &contents).context("Failed to write cookie jar")?;
        Ok(path)
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

    /// Check if we have an active session.
    pub fn is_logged_in(&self) -> bool {
        self.handle.is_some()
    }

    pub fn handle(&self) -> Option<&str> {
        self.handle.as_deref()
    }

    /// Submit a solution to a Codeforces problem using curl (bypasses Cloudflare TLS fingerprinting).
    /// Uses a shared cookie jar file so intermediate cookies from the GET carry over to the POST.
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

        let jar_path = self.write_cookie_jar()?;
        let jar = jar_path.to_str().context("Cookie jar path not UTF-8")?;

        // Step 1: GET submit page to extract CSRF token
        let submit_url = format!("{}/contest/{}/submit", CF_BASE, contest_id);
        let output = std::process::Command::new("curl")
            .args([
                "-sS", "-L", "--max-time", "30",
                "-A", &self.raw_user_agent,
                "-b", jar, "-c", jar,
                "-H", "Accept-Language: en-US,en;q=0.9",
                &submit_url,
            ])
            .output()
            .context("Failed to run curl (is curl installed?)")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("curl GET failed: {}", stderr.trim());
        }
        let body = String::from_utf8(output.stdout).context("curl returned non-UTF-8")?;

        let csrf = match Self::extract_csrf(&body) {
            Ok(c) => c,
            Err(_) => {
                let _ = std::fs::remove_file(&jar_path);
                if body.contains("Checking your browser") || body.contains("challenge-platform") {
                    bail!("Cloudflare blocked request. Re-import cookies in Settings.");
                } else if body.contains("/enter") || body.contains("handleOrEmail") {
                    bail!("Not logged in — session cookies expired. Re-import in Settings.");
                } else {
                    bail!("Cannot find CSRF token. Cookies may be stale — re-import in Settings.");
                }
            }
        };

        // Step 2: POST the solution (same cookie jar preserves session cookies from GET)
        let post_url = format!("{}?csrf_token={}", submit_url, csrf);
        let ftaa = Self::random_ftaa();
        let contest_str = contest_id.to_string();
        let fields: [(&str, &str); 11] = [
            ("csrf_token", &csrf),
            ("ftaa", &ftaa),
            ("bfaa", "f1b3f18c715565b589b7823cda7448ce"),
            ("action", "submitSolutionFormSubmitted"),
            ("submittedProblemIndex", problem_index),
            ("programTypeId", lang_id),
            ("contestId", &contest_str),
            ("source", source_code),
            ("tabSize", "4"),
            ("_tta", "594"),
            ("sourceCodeConfirmed", "true"),
        ];

        let referer = format!("{}/contest/{}/submit", CF_BASE, contest_id);
        let mut cmd = std::process::Command::new("curl");
        cmd.args([
            "-sS", "-L", "--max-time", "30",
            "-A", &self.raw_user_agent,
            "-b", jar, "-c", jar,
            "-H", "Accept-Language: en-US,en;q=0.9",
            "-H", &format!("Referer: {}", referer),
            "-H", &format!("Origin: {}", CF_BASE),
        ]);
        for (key, value) in &fields {
            cmd.arg("--data-urlencode");
            cmd.arg(format!("{}={}", key, value));
        }
        cmd.arg(&post_url);

        let output = cmd.output().context("Failed to run curl for POST")?;
        let _ = std::fs::remove_file(&jar_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("curl POST failed: {}", stderr.trim());
        }
        let body = String::from_utf8(output.stdout).context("curl returned non-UTF-8")?;

        // Check for CF error messages in response
        let err_re = Regex::new(r#"error[^>]*>([^<]+)<"#).unwrap();
        if let Some(caps) = err_re.captures(&body) {
            let msg = caps[1].trim().to_string();
            if !msg.is_empty() {
                bail!("Submission error: {}", msg);
            }
        }

        // Check for "for__source" error span (e.g., "You have submitted exactly the same code before")
        let src_err_re = Regex::new(r#"for__source[^>]*>([^<]+)<"#).unwrap();
        if let Some(caps) = src_err_re.captures(&body) {
            let msg = caps[1].trim().to_string();
            if !msg.is_empty() {
                bail!("Submission error: {}", msg);
            }
        }

        // If response still contains the submit form, submission didn't go through
        if body.contains("name=\"submittedProblemIndex\"") {
            // Try to find any error message in the page
            let span_err = Regex::new(r#"<span\s+class="error[^"]*"[^>]*>([^<]+)<"#).unwrap();
            if let Some(caps) = span_err.captures(&body) {
                bail!("Submission rejected: {}", caps[1].trim());
            }
            // Cloudflare / login checks only matter if submission didn't go through
            if body.contains("Checking your browser") || body.contains("challenge-platform") {
                bail!("Cloudflare blocked POST. Re-import cookies in Settings.");
            }
            if body.contains("handleOrEmail") {
                bail!("Session expired during submit. Re-import cookies in Settings.");
            }
            bail!("Submission failed (submit form returned). Session may be invalid — re-import cookies.");
        }

        // Submit form is gone — submission was accepted
        Ok(0)
    }

    /// Poll for the verdict of the most recent submission via curl.
    /// Returns (verdict, problem_index) or None if still judging.
    pub async fn poll_latest_verdict(
        &self,
        contest_id: i64,
    ) -> Result<Option<(String, String)>> {
        let handle = self.handle.as_deref().context("Not logged in")?;

        let url = format!(
            "https://codeforces.com/api/user.status?handle={}&from=1&count=5",
            handle
        );
        let output = std::process::Command::new("curl")
            .args(["-sS", "--max-time", "15", &url])
            .output()
            .context("Failed to run curl for verdict poll")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Verdict poll failed: {}", stderr.trim());
        }
        let body = String::from_utf8(output.stdout)
            .context("Verdict response not UTF-8")?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body).context("Failed to parse verdict JSON")?;

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
}

/// CF language ID for PyPy 3-64 (Python 3).
pub const LANG_PYPY3: &str = "70";

