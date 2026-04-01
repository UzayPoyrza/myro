//! Self-update system for myro.
//!
//! - Background update check on startup (24h cooldown)
//! - `myro update` command for interactive self-replacement
//! - Forgejo/GitHub-compatible release API

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::config::UpdateConfig;

pub const CURRENT_VERSION: &str = env!("MYRO_VERSION");

#[derive(Debug)]
pub enum UpdateEvent {
    Available { version: String },
    UpToDate,
    Error(String),
}

#[derive(Deserialize)]
struct ReleaseInfo {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// Returns the target triple string for the current build.
fn current_target() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { "x86_64-unknown-linux-gnu" }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { "aarch64-unknown-linux-gnu" }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { "x86_64-apple-darwin" }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { "aarch64-apple-darwin" }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    { "unknown" }
}

fn check_timestamp_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("update_check.json")
}

fn should_check(cooldown_secs: u64) -> bool {
    let path = check_timestamp_path();
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return true;
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return true;
    };
    let Some(ts) = val.get("last_check").and_then(|v| v.as_i64()) else {
        return true;
    };
    let now = chrono::Utc::now().timestamp();
    (now - ts) as u64 >= cooldown_secs
}

fn record_check() {
    let path = check_timestamp_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let now = chrono::Utc::now().timestamp();
    let json = serde_json::json!({ "last_check": now });
    let _ = std::fs::write(&path, json.to_string());
}

fn build_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(format!("myro/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client")
}

/// Fetch latest release info from the Forgejo/GitHub API.
fn fetch_latest_release(config: &UpdateConfig) -> Result<ReleaseInfo> {
    let url = format!("{}/releases/latest", config.release_url);
    let client = build_client()?;
    let resp = client.get(&url).send().context("failed to fetch release")?;
    if !resp.status().is_success() {
        bail!("release API returned {}", resp.status());
    }
    resp.json::<ReleaseInfo>()
        .context("failed to parse release JSON")
}

/// Check if a newer version is available. Returns `Some(version)` if yes.
pub fn check_latest_version(config: &UpdateConfig) -> Result<Option<String>> {
    let release = fetch_latest_release(config)?;
    let latest_str = release.tag_name.trim_start_matches('v');
    let latest = semver::Version::parse(latest_str)
        .with_context(|| format!("invalid version tag: {}", release.tag_name))?;
    let current = semver::Version::parse(CURRENT_VERSION)
        .unwrap_or_else(|_| semver::Version::new(0, 0, 0));

    if latest > current {
        Ok(Some(latest.to_string()))
    } else {
        Ok(None)
    }
}

/// Spawn a background thread that checks for updates (respects 24h cooldown).
/// Returns a receiver for the result, or None if auto_check is disabled.
pub fn spawn_update_check(config: &UpdateConfig) -> Option<mpsc::Receiver<UpdateEvent>> {
    if !config.auto_check {
        return None;
    }
    if !should_check(86400) {
        return None;
    }

    let config = config.clone();
    let (tx, rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("update-check".into())
        .spawn(move || {
            record_check();
            let event = match check_latest_version(&config) {
                Ok(Some(version)) => UpdateEvent::Available { version },
                Ok(None) => UpdateEvent::UpToDate,
                Err(e) => UpdateEvent::Error(e.to_string()),
            };
            let _ = tx.send(event);
        })
        .ok()?;

    Some(rx)
}

/// Interactive update flow for `myro update`.
pub fn run_update() -> Result<()> {
    let config = UpdateConfig::default();
    println!("myro {} ({})", CURRENT_VERSION, current_target());
    println!("checking for updates...");

    let release = fetch_latest_release(&config)?;
    let latest_str = release.tag_name.trim_start_matches('v');
    let latest = semver::Version::parse(latest_str)
        .with_context(|| format!("invalid version tag: {}", release.tag_name))?;
    let current = semver::Version::parse(CURRENT_VERSION)
        .unwrap_or_else(|_| semver::Version::new(0, 0, 0));

    if latest <= current {
        println!("already up to date.");
        return Ok(());
    }

    println!("new version available: v{} -> v{}", current, latest);

    // Find the right asset for our target
    let target = current_target();
    let tarball_name = format!("myro-v{}-{}.tar.gz", latest, target);
    let checksums_name = "checksums.sha256";

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == tarball_name)
        .with_context(|| format!("no release asset for {}", target))?;

    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name == checksums_name);

    // Download tarball
    println!("downloading {}...", tarball_name);
    let client = build_client()?;
    let tarball_bytes = client
        .get(&asset.browser_download_url)
        .send()
        .context("failed to download tarball")?
        .bytes()
        .context("failed to read tarball")?;

    // Verify checksum if available
    if let Some(cs_asset) = checksum_asset {
        println!("verifying checksum...");
        let checksums_text = client
            .get(&cs_asset.browser_download_url)
            .send()
            .context("failed to download checksums")?
            .text()
            .context("failed to read checksums")?;

        verify_checksum(&tarball_bytes, &tarball_name, &checksums_text)?;
    }

    // Extract binary from tarball
    println!("extracting...");
    let binary_data = extract_binary_from_tarball(&tarball_bytes)?;

    // Self-replace
    let current_exe = std::env::current_exe().context("failed to determine current executable")?;
    self_replace(&current_exe, &binary_data)?;

    println!("updated to v{}!", latest);
    record_check();
    Ok(())
}

fn verify_checksum(data: &[u8], filename: &str, checksums_text: &str) -> Result<()> {
    use sha2::Digest;
    let actual = format!("{:x}", sha2::Sha256::digest(data));

    for line in checksums_text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 && parts[1] == filename {
            if parts[0] == actual {
                return Ok(());
            } else {
                bail!(
                    "checksum mismatch for {}: expected {}, got {}",
                    filename,
                    parts[0],
                    actual
                );
            }
        }
    }
    bail!("no checksum found for {} in checksums file", filename);
}

fn extract_binary_from_tarball(tarball_bytes: &[u8]) -> Result<Vec<u8>> {
    let decoder = flate2::read::GzDecoder::new(tarball_bytes);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries().context("failed to read tarball entries")? {
        let mut entry = entry.context("failed to read tarball entry")?;
        let path = entry.path().context("failed to read entry path")?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name == "myro" {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .context("failed to read binary from tarball")?;
            return Ok(buf);
        }
    }
    bail!("no 'myro' binary found in tarball")
}

/// Atomically replace the current binary.
/// Write to temp → rename current to .old → rename temp to current → delete .old.
fn self_replace(current_exe: &std::path::Path, new_binary: &[u8]) -> Result<()> {
    let dir = current_exe
        .parent()
        .context("executable has no parent directory")?;
    let tmp_path = dir.join(".myro.new");
    let old_path = dir.join(".myro.old");

    // Write new binary to temp file
    {
        let mut f = std::fs::File::create(&tmp_path).context("failed to create temp file")?;
        f.write_all(new_binary)
            .context("failed to write temp file")?;
        f.sync_all()?;
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // Atomic swap: rename current → old, rename tmp → current
    // On failure to rename tmp→current, try to restore old→current
    let _ = std::fs::remove_file(&old_path);
    std::fs::rename(current_exe, &old_path).context("failed to move current binary aside")?;

    if let Err(e) = std::fs::rename(&tmp_path, current_exe) {
        // Try to restore
        let _ = std::fs::rename(&old_path, current_exe);
        return Err(e).context("failed to install new binary");
    }

    // Clean up
    let _ = std::fs::remove_file(&old_path);
    Ok(())
}
