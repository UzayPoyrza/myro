use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

use crate::client::SupabaseClient;

/// Persisted auth tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub user_id: String,
}

/// Response from Supabase GoTrue token endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    user: TokenUser,
}

#[derive(Debug, Deserialize)]
struct TokenUser {
    id: String,
}

/// Response from signup endpoint.
#[derive(Debug, Deserialize)]
struct SignupResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    id: Option<String>,
    user: Option<TokenUser>,
}

impl AuthTokens {
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now >= self.expires_at - 60 // refresh 60s before expiry
    }

    pub fn to_client(&self) -> SupabaseClient {
        SupabaseClient::new(self.access_token.clone(), self.user_id.clone())
    }
}

fn auth_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("myro")
        .join("auth.json")
}

pub fn load_tokens() -> Option<AuthTokens> {
    let path = auth_file_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub fn save_tokens(tokens: &AuthTokens) -> Result<()> {
    let path = auth_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(tokens)?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub fn clear_tokens() {
    let path = auth_file_path();
    let _ = std::fs::remove_file(&path);
}

/// Sign up with email + password. Returns tokens on auto-confirm, or error if confirm required.
pub fn sign_up_email(email: &str, password: &str) -> Result<AuthTokens> {
    let url = SupabaseClient::auth_url("/signup");
    let body = serde_json::json!({
        "email": email,
        "password": password,
    });
    let resp: SignupResponse = SupabaseClient::raw_post(&url, &body)?;
    match (resp.access_token, resp.refresh_token, resp.expires_in) {
        (Some(at), Some(rt), Some(ei)) => {
            let user_id = resp
                .user
                .map(|u| u.id)
                .or(resp.id)
                .context("no user id in signup response")?;
            let tokens = AuthTokens {
                access_token: at,
                refresh_token: rt,
                expires_at: chrono::Utc::now().timestamp() + ei,
                user_id,
            };
            save_tokens(&tokens)?;
            Ok(tokens)
        }
        _ => anyhow::bail!("signup requires email confirmation — check your inbox"),
    }
}

/// Sign in with email + password.
pub fn sign_in_email(email: &str, password: &str) -> Result<AuthTokens> {
    let url = SupabaseClient::auth_url("/token?grant_type=password");
    let body = serde_json::json!({
        "email": email,
        "password": password,
    });
    let resp: TokenResponse = SupabaseClient::raw_post(&url, &body)?;
    let tokens = AuthTokens {
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expires_at: chrono::Utc::now().timestamp() + resp.expires_in,
        user_id: resp.user.id,
    };
    save_tokens(&tokens)?;
    Ok(tokens)
}

/// Refresh an expired access token.
pub fn refresh_token(current: &AuthTokens) -> Result<AuthTokens> {
    let url = SupabaseClient::auth_url("/token?grant_type=refresh_token");
    let body = serde_json::json!({
        "refresh_token": current.refresh_token,
    });
    let resp: TokenResponse = SupabaseClient::raw_post(&url, &body)?;
    let tokens = AuthTokens {
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expires_at: chrono::Utc::now().timestamp() + resp.expires_in,
        user_id: resp.user.id,
    };
    save_tokens(&tokens)?;
    Ok(tokens)
}

/// Generate PKCE code_verifier and code_challenge.
fn generate_pkce() -> (String, String) {
    let mut rng = rand::thread_rng();
    let verifier_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let verifier = URL_SAFE_NO_PAD.encode(&verifier_bytes);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// Start GitHub OAuth PKCE flow.
/// Opens browser, listens on localhost for callback, exchanges code for tokens.
/// Returns the auth URL to open in browser and a closure to wait for the callback.
pub fn start_github_oauth() -> Result<(String, OAuthListener)> {
    let (verifier, challenge) = generate_pkce();

    // Bind to a random available port
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to bind localhost")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let auth_url = format!(
        "{}/auth/v1/authorize?provider=github&redirect_to={}&code_challenge={}&code_challenge_method=S256&flow_type=pkce",
        SupabaseClient::base_url(),
        urlencoding(&redirect_uri),
        challenge,
    );

    Ok((
        auth_url,
        OAuthListener {
            listener,
            verifier,
            redirect_uri,
        },
    ))
}

/// Handles the OAuth callback on localhost.
pub struct OAuthListener {
    listener: TcpListener,
    verifier: String,
    redirect_uri: String,
}

impl OAuthListener {
    /// Block until the OAuth callback arrives (or timeout). Returns auth tokens.
    pub fn wait_for_callback(self) -> Result<AuthTokens> {
        self.listener
            .set_nonblocking(false)
            .context("set blocking")?;
        // Set a generous timeout for the user to complete OAuth in browser
        self.listener
            .incoming()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no incoming connection"))?
            .and_then(|mut stream| {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf)?;
                let request = String::from_utf8_lossy(&buf[..n]);

                // Extract code from GET /callback?code=...
                let code = extract_query_param(&request, "code")
                    .ok_or_else(|| std::io::Error::other("no code in callback"))?;

                // Send a simple HTML response
                let html = "<html><body><h2>authenticated! you can close this tab.</h2></body></html>";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                    html.len(),
                    html
                );
                stream.write_all(response.as_bytes())?;

                // Exchange code for tokens
                let tokens = exchange_code(&code, &self.verifier, &self.redirect_uri)
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                Ok(tokens)
            })
            .context("OAuth callback handling failed")
    }
}

fn exchange_code(code: &str, verifier: &str, redirect_uri: &str) -> Result<AuthTokens> {
    let url = SupabaseClient::auth_url("/token?grant_type=pkce");
    let body = serde_json::json!({
        "auth_code": code,
        "code_verifier": verifier,
        "redirect_to": redirect_uri,
    });
    let resp: TokenResponse = SupabaseClient::raw_post(&url, &body)?;
    let tokens = AuthTokens {
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expires_at: chrono::Utc::now().timestamp() + resp.expires_in,
        user_id: resp.user.id,
    };
    save_tokens(&tokens)?;
    Ok(tokens)
}

fn extract_query_param(request: &str, param: &str) -> Option<String> {
    // Parse "GET /callback?code=xxx&... HTTP/1.1"
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next()? == param {
            return kv.next().map(|v| v.to_string());
        }
    }
    None
}

/// Minimal URL encoding for the redirect URI.
fn urlencoding(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_valid() {
        let (verifier, challenge) = generate_pkce();
        // Verify the challenge matches the verifier
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(challenge, expected);
        assert!(verifier.len() >= 32);
    }

    #[test]
    fn extract_code_from_request() {
        let req = "GET /callback?code=abc123&state=xyz HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert_eq!(extract_query_param(req, "code"), Some("abc123".into()));
        assert_eq!(extract_query_param(req, "state"), Some("xyz".into()));
        assert_eq!(extract_query_param(req, "missing"), None);
    }

    #[test]
    fn tokens_expiry_check() {
        let tokens = AuthTokens {
            access_token: "test".into(),
            refresh_token: "test".into(),
            expires_at: chrono::Utc::now().timestamp() + 3600,
            user_id: "test".into(),
        };
        assert!(!tokens.is_expired());

        let expired = AuthTokens {
            expires_at: chrono::Utc::now().timestamp() - 100,
            ..tokens
        };
        assert!(expired.is_expired());
    }
}
