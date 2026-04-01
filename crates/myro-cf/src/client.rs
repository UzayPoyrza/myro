use anyhow::{bail, Context, Result};

use super::rate_limiter::RateLimiter;
use super::types::*;

const CF_API_BASE: &str = "https://codeforces.com/api";

pub struct CfClient {
    client: reqwest::Client,
    rate_limiter: RateLimiter,
}

impl CfClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .user_agent("myro/0.1.0")
                .build()
                .expect("Failed to build HTTP client"),
            rate_limiter: RateLimiter::new(),
        }
    }

    async fn get_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T> {
        let max_retries = 5;
        let mut attempt = 0;

        loop {
            self.rate_limiter.wait().await;
            attempt += 1;

            let response = self
                .client
                .get(url)
                .send()
                .await
                .context("HTTP request failed")?;

            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if attempt >= max_retries {
                    bail!("Rate limited after {} retries for {}", max_retries, url);
                }
                let backoff = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                eprintln!(
                    "Rate limited (429), backing off for {:?} (attempt {}/{})",
                    backoff, attempt, max_retries
                );
                tokio::time::sleep(backoff).await;
                continue;
            }

            if !status.is_success() {
                if status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS {
                    bail!("HTTP {} for {}", status, url);
                }
                if attempt >= max_retries {
                    bail!(
                        "HTTP {} after {} retries for {}",
                        status,
                        max_retries,
                        url
                    );
                }
                let backoff = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                eprintln!(
                    "HTTP {}, retrying in {:?} (attempt {}/{})",
                    status, backoff, attempt, max_retries
                );
                tokio::time::sleep(backoff).await;
                continue;
            }

            let body = response.text().await.context("Failed to read response body")?;
            let parsed: CfApiResponse<T> =
                serde_json::from_str(&body).context("Failed to parse CF API response")?;

            if parsed.status != "OK" {
                let msg = parsed.comment.unwrap_or_default();
                if attempt >= max_retries {
                    bail!("CF API error after {} retries: {}", max_retries, msg);
                }
                let backoff = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                eprintln!(
                    "CF API error: {}, retrying in {:?} (attempt {}/{})",
                    msg, backoff, attempt, max_retries
                );
                tokio::time::sleep(backoff).await;
                continue;
            }

            return parsed
                .result
                .ok_or_else(|| anyhow::anyhow!("CF API returned OK but no result"));
        }
    }

    pub async fn fetch_contest_list(&self) -> Result<Vec<CfContest>> {
        let url = format!("{}/contest.list", CF_API_BASE);
        self.get_with_retry(&url).await
    }

    pub async fn fetch_contest_standings(&self, contest_id: i64) -> Result<CfStandingsResult> {
        let url = format!(
            "{}/contest.standings?contestId={}&showUnofficial=false",
            CF_API_BASE, contest_id
        );
        self.get_with_retry(&url).await
    }

    pub async fn fetch_rating_changes(&self, contest_id: i64) -> Result<Vec<CfRatingChange>> {
        let url = format!(
            "{}/contest.ratingChanges?contestId={}",
            CF_API_BASE, contest_id
        );
        self.get_with_retry(&url).await
    }

    pub async fn fetch_user_status(&self, handle: &str) -> Result<Vec<CfSubmission>> {
        let url = format!("{}/user.status?handle={}", CF_API_BASE, handle);
        self.get_with_retry(&url).await
    }

    pub async fn fetch_user_info(&self, handle: &str) -> Result<CfUser> {
        let url = format!("{}/user.info?handles={}", CF_API_BASE, handle);
        let users: Vec<CfUser> = self.get_with_retry(&url).await?;
        users
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No user found for handle '{}'", handle))
    }

    pub async fn fetch_problemset_problems(&self) -> Result<CfProblemsetResult> {
        let url = format!("{}/problemset.problems", CF_API_BASE);
        self.get_with_retry(&url).await
    }

    /// Fetch raw HTML for a CF problem page (not an API endpoint).
    /// Tries `/contest/` URL first, falls back to `/problemset/problem/` on failure.
    pub async fn fetch_problem_html(&self, contest_id: i64, index: &str) -> Result<String> {
        let urls = [
            format!(
                "https://codeforces.com/contest/{}/problem/{}",
                contest_id, index
            ),
            format!(
                "https://codeforces.com/problemset/problem/{}/{}",
                contest_id, index
            ),
        ];

        for url in &urls {
            self.rate_limiter.wait().await;
            let response = match self.client.get(url).send().await {
                Ok(r) => r,
                Err(_) => continue,
            };

            if response.status().is_success() {
                return response
                    .text()
                    .await
                    .context("Failed to read problem page body");
            }
        }

        bail!(
            "Failed to fetch problem page {}{} (tried both /contest/ and /problemset/ URLs)",
            contest_id,
            index
        );
    }
}

impl Default for CfClient {
    fn default() -> Self {
        Self::new()
    }
}
