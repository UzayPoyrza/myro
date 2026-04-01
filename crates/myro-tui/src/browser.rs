use std::collections::HashMap;
use std::path::PathBuf;

/// Detect the actual Firefox user-agent string by reading the installed version.
/// Falls back to a generic recent Firefox UA if detection fails.
pub fn detect_firefox_ua() -> String {
    let version = detect_firefox_version().unwrap_or_else(|| "128.0".to_string());
    let major = version.split('.').next().unwrap_or("128");
    format!("Mozilla/5.0 (X11; Linux x86_64; rv:{major}.0) Gecko/20100101 Firefox/{major}.0")
}

fn detect_firefox_version() -> Option<String> {
    // Try compatibility.ini from profile (fast, no process spawn)
    if let Ok(profile) = find_firefox_profile() {
        let compat = profile.join("compatibility.ini");
        if let Ok(contents) = std::fs::read_to_string(compat) {
            for line in contents.lines() {
                if let Some(rest) = line.strip_prefix("LastVersion=") {
                    // Format: "135.0.1_20250101/20250101" -> "135.0.1"
                    let ver = rest.split('_').next()?;
                    return Some(ver.to_string());
                }
            }
        }
    }

    // Try firefox --version
    if let Ok(output) = std::process::Command::new("firefox")
        .arg("--version")
        .output()
    {
        if let Ok(s) = String::from_utf8(output.stdout) {
            // "Mozilla Firefox 135.0.1" -> "135.0.1"
            if let Some(ver) = s.trim().strip_prefix("Mozilla Firefox ") {
                return Some(ver.to_string());
            }
        }
    }

    None
}

/// Import Codeforces cookies from Firefox — both persistent (cookies.sqlite)
/// and session-only (recovery.jsonlz4) cookies like JSESSIONID.
/// Returns a list of (name, value) pairs.
pub fn import_cf_cookies() -> Result<Vec<(String, String)>, String> {
    let profile_dir = find_firefox_profile()?;

    // Merge persistent + session cookies (session cookies override persistent)
    let mut cookies: HashMap<String, String> = HashMap::new();

    // 1. Persistent cookies from cookies.sqlite
    let cookies_db = profile_dir.join("cookies.sqlite");
    if cookies_db.exists() {
        let tmp = std::env::temp_dir().join("myro_firefox_cookies.sqlite");
        std::fs::copy(&cookies_db, &tmp)
            .map_err(|e| format!("Failed to copy cookies.sqlite: {}", e))?;

        let conn = rusqlite::Connection::open_with_flags(
            &tmp,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| format!("Failed to open cookies database: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT name, value FROM moz_cookies \
                 WHERE host LIKE '%codeforces.com' \
                 AND expiry > unixepoch()",
            )
            .map_err(|e| format!("Failed to query cookies: {}", e))?;

        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Failed to read cookies: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let _ = std::fs::remove_file(&tmp);

        for (name, value) in rows {
            cookies.insert(name, value);
        }
    }

    // 2. Session cookies from recovery.jsonlz4 (includes JSESSIONID)
    let recovery = profile_dir
        .join("sessionstore-backups")
        .join("recovery.jsonlz4");
    if recovery.exists() {
        if let Ok(session_cookies) = read_session_cookies(&recovery) {
            for (name, value) in session_cookies {
                cookies.insert(name, value);
            }
        }
    }

    if cookies.is_empty() {
        return Err(
            "No Codeforces cookies found. Log into codeforces.com in Firefox first.".into(),
        );
    }

    if !cookies.contains_key("JSESSIONID") {
        return Err(
            "No active CF session found (missing JSESSIONID). \
             Log into codeforces.com in Firefox first."
                .into(),
        );
    }

    Ok(cookies.into_iter().collect())
}

/// Read session cookies from Firefox's recovery.jsonlz4.
/// Mozilla LZ4 format: 8-byte magic "mozLz40\0" + 4-byte LE size + LZ4 block data.
fn read_session_cookies(path: &PathBuf) -> Result<Vec<(String, String)>, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read recovery file: {}", e))?;

    if data.len() < 12 || &data[..8] != b"mozLz40\0" {
        return Err("Invalid recovery.jsonlz4 format".into());
    }

    let size = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let decompressed = lz4_flex::decompress(&data[12..], size)
        .map_err(|e| format!("Failed to decompress session data: {}", e))?;

    let json: serde_json::Value = serde_json::from_slice(&decompressed)
        .map_err(|e| format!("Failed to parse session JSON: {}", e))?;

    let mut result = Vec::new();
    if let Some(cookies) = json.get("cookies").and_then(|c| c.as_array()) {
        for cookie in cookies {
            let host = cookie.get("host").and_then(|h| h.as_str()).unwrap_or("");
            if !host.contains("codeforces.com") {
                continue;
            }
            let name = cookie.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if !name.is_empty() {
                result.push((name.to_string(), value.to_string()));
            }
        }
    }

    Ok(result)
}

/// Find a Firefox profile directory containing cookies.sqlite.
fn find_firefox_profile() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let ff_dir = home.join(".mozilla").join("firefox");
    if !ff_dir.exists() {
        return Err("Firefox profile directory not found (~/.mozilla/firefox)".into());
    }

    let entries = std::fs::read_dir(&ff_dir)
        .map_err(|e| format!("Cannot read Firefox profiles: {}", e))?;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.contains(".default") && entry.path().join("cookies.sqlite").exists() {
            return Ok(entry.path());
        }
    }

    Err("No Firefox profile with cookies.sqlite found".into())
}
