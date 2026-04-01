use anyhow::{Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde::Serialize;

const SUPABASE_URL: &str = "https://yblyfpanzpfmwmupedwx.supabase.co";
const SUPABASE_ANON_KEY: &str = "sb_publishable_MTDZPlFsA1Mtag19TUnb-A_cgQ2t_wg";

/// Client for Supabase PostgREST and Auth APIs.
#[derive(Clone)]
pub struct SupabaseClient {
    http: Client,
    pub access_token: String,
    pub user_id: String,
}

impl SupabaseClient {
    pub fn new(access_token: String, user_id: String) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            http,
            access_token,
            user_id,
        }
    }

    fn rest_url(&self, table: &str) -> String {
        format!("{}/rest/v1/{}", SUPABASE_URL, table)
    }

    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("apikey", HeaderValue::from_static(SUPABASE_ANON_KEY));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token))
                .expect("invalid access token"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    /// GET rows from a table with PostgREST query string.
    /// Example: `client.get("solved_problems", "user_id=eq.{uid}&select=*")`
    pub fn get<T: DeserializeOwned>(&self, table: &str, query: &str) -> Result<Vec<T>> {
        let url = format!("{}?{}", self.rest_url(table), query);
        let resp = self
            .http
            .get(&url)
            .headers(self.default_headers())
            .send()
            .context("GET request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("GET {table} returned {status}: {body}");
        }
        resp.json().context("failed to parse GET response")
    }

    /// POST (insert) rows. Use `upsert=true` for upsert with merge-duplicates.
    pub fn post<T: Serialize + ?Sized>(&self, table: &str, body: &T, upsert: bool) -> Result<()> {
        let mut headers = self.default_headers();
        headers.insert("Prefer", HeaderValue::from_static("return=minimal"));
        if upsert {
            headers.insert(
                "Prefer",
                HeaderValue::from_static("return=minimal,resolution=merge-duplicates"),
            );
        }
        let resp = self
            .http
            .post(self.rest_url(table))
            .headers(headers)
            .json(body)
            .send()
            .context("POST request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("POST {table} returned {status}: {body}");
        }
        Ok(())
    }

    /// PATCH (update) rows matching a filter.
    pub fn patch<T: Serialize + ?Sized>(&self, table: &str, query: &str, body: &T) -> Result<()> {
        let url = format!("{}?{}", self.rest_url(table), query);
        let mut headers = self.default_headers();
        headers.insert("Prefer", HeaderValue::from_static("return=minimal"));
        let resp = self
            .http
            .patch(&url)
            .headers(headers)
            .json(body)
            .send()
            .context("PATCH request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("PATCH {table} returned {status}: {body}");
        }
        Ok(())
    }

    /// DELETE rows matching a filter.
    pub fn delete(&self, table: &str, query: &str) -> Result<()> {
        let url = format!("{}?{}", self.rest_url(table), query);
        let resp = self
            .http
            .delete(&url)
            .headers(self.default_headers())
            .send()
            .context("DELETE request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("DELETE {table} returned {status}: {body}");
        }
        Ok(())
    }

    /// Raw POST to an arbitrary Supabase endpoint (used by auth).
    pub(crate) fn raw_post<T: Serialize, R: DeserializeOwned>(
        url: &str,
        body: &T,
    ) -> Result<R> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        let mut headers = HeaderMap::new();
        headers.insert("apikey", HeaderValue::from_static(SUPABASE_ANON_KEY));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let resp = http
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .context("auth request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("auth POST returned {status}: {body}");
        }
        resp.json().context("failed to parse auth response")
    }

    pub(crate) fn auth_url(path: &str) -> String {
        format!("{}/auth/v1{}", SUPABASE_URL, path)
    }

    pub(crate) fn base_url() -> &'static str {
        SUPABASE_URL
    }
}
