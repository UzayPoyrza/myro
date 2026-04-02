use std::collections::HashMap;
use std::path::PathBuf;

/// Which browser cookies were imported from.
#[derive(Debug, Clone, Copy)]
pub enum Browser {
    Firefox,
    Chrome,
    Safari,
}

impl std::fmt::Display for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Browser::Firefox => write!(f, "Firefox"),
            Browser::Chrome => write!(f, "Chrome"),
            Browser::Safari => write!(f, "Safari"),
        }
    }
}

/// Result of a successful cookie import.
pub struct CookieImportResult {
    pub cookies: Vec<(String, String)>,
    pub browser: Browser,
}

/// Detect the browser user-agent string for the browser cookies were imported from.
pub fn detect_browser_ua(browser: Browser) -> String {
    match browser {
        Browser::Firefox => detect_firefox_ua(),
        Browser::Chrome => detect_chrome_ua(),
        Browser::Safari => detect_safari_ua(),
    }
}

fn detect_firefox_ua() -> String {
    let version = detect_firefox_version().unwrap_or_else(|| "128.0".to_string());
    let major = version.split('.').next().unwrap_or("128");
    let platform = if cfg!(target_os = "macos") {
        "Macintosh; Intel Mac OS X 10.15"
    } else {
        "X11; Linux x86_64"
    };
    format!("Mozilla/5.0 ({platform}; rv:{major}.0) Gecko/20100101 Firefox/{major}.0")
}

fn detect_chrome_ua() -> String {
    let version = detect_chrome_version().unwrap_or_else(|| "125.0.0.0".to_string());
    let platform = if cfg!(target_os = "macos") {
        "Macintosh; Intel Mac OS X 10_15_7"
    } else {
        "X11; Linux x86_64"
    };
    format!(
        "Mozilla/5.0 ({platform}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{version} Safari/537.36"
    )
}

fn detect_safari_ua() -> String {
    let version = detect_safari_version().unwrap_or_else(|| "17.4".to_string());
    format!(
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{version} Safari/605.1.15"
    )
}

// ── Version detection ──

fn detect_firefox_version() -> Option<String> {
    if let Ok(profile) = find_firefox_profile() {
        let compat = profile.join("compatibility.ini");
        if let Ok(contents) = std::fs::read_to_string(compat) {
            for line in contents.lines() {
                if let Some(rest) = line.strip_prefix("LastVersion=") {
                    let ver = rest.split('_').next()?;
                    return Some(ver.to_string());
                }
            }
        }
    }

    let cmd = if cfg!(target_os = "macos") {
        "/Applications/Firefox.app/Contents/MacOS/firefox"
    } else {
        "firefox"
    };
    if let Ok(output) = std::process::Command::new(cmd).arg("--version").output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            if let Some(ver) = s.trim().strip_prefix("Mozilla Firefox ") {
                return Some(ver.to_string());
            }
        }
    }
    None
}

fn detect_chrome_version() -> Option<String> {
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"]
    } else {
        &["google-chrome", "google-chrome-stable", "chromium-browser", "chromium"]
    };
    for cmd in candidates {
        if let Ok(output) = std::process::Command::new(cmd).arg("--version").output() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                // "Google Chrome 125.0.6422.60" or "Chromium 125.0.6422.60"
                let trimmed = s.trim();
                if let Some(ver) = trimmed.split_whitespace().last() {
                    if ver.contains('.') {
                        return Some(ver.to_string());
                    }
                }
            }
        }
    }
    None
}

fn detect_safari_version() -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    // Read Safari's Info.plist for CFBundleShortVersionString
    let plist = "/Applications/Safari.app/Contents/version.plist";
    if let Ok(contents) = std::fs::read_to_string(plist) {
        // Look for <key>CFBundleShortVersionString</key>\n<string>17.4</string>
        let mut found_key = false;
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.contains("CFBundleShortVersionString") {
                found_key = true;
            } else if found_key {
                if let Some(ver) = trimmed.strip_prefix("<string>") {
                    if let Some(ver) = ver.strip_suffix("</string>") {
                        return Some(ver.to_string());
                    }
                }
                found_key = false;
            }
        }
    }
    None
}

// ── Cookie import ──

/// Try importing Codeforces cookies from available browsers.
/// Tries Firefox first, then Chrome, then Safari.
/// Returns cookies and which browser they came from.
pub fn import_cf_cookies() -> Result<CookieImportResult, String> {
    let mut errors = Vec::new();

    // Try Firefox
    match import_firefox_cookies() {
        Ok(cookies) if !cookies.is_empty() => {
            // Accept if we have either JSESSIONID or cf_clearance — both indicate an active session
            if cookies.iter().any(|(n, _)| n == "JSESSIONID" || n == "cf_clearance") {
                return Ok(CookieImportResult {
                    cookies,
                    browser: Browser::Firefox,
                });
            }
            errors.push("Firefox: no active session cookies found".to_string());
        }
        Ok(_) => errors.push("Firefox: no Codeforces cookies found".to_string()),
        Err(e) => errors.push(format!("Firefox: {}", e)),
    }

    // Try Chrome
    match import_chrome_cookies() {
        Ok(cookies) if !cookies.is_empty() => {
            return Ok(CookieImportResult {
                cookies,
                browser: Browser::Chrome,
            });
        }
        Ok(_) => errors.push("Chrome: no Codeforces cookies found".to_string()),
        Err(e) => errors.push(format!("Chrome: {}", e)),
    }

    // Try Safari (macOS only)
    if cfg!(target_os = "macos") {
        match import_safari_cookies() {
            Ok(cookies) if !cookies.is_empty() => {
                return Ok(CookieImportResult {
                    cookies,
                    browser: Browser::Safari,
                });
            }
            Ok(_) => errors.push("Safari: no Codeforces cookies found".to_string()),
            Err(e) => errors.push(format!("Safari: {}", e)),
        }
    }

    Err(format!(
        "No Codeforces cookies found. Log into codeforces.com in any browser first.\n{}",
        errors.join("\n")
    ))
}

// ── Firefox ──

fn import_firefox_cookies() -> Result<Vec<(String, String)>, String> {
    let profile_dir = find_firefox_profile()?;
    let mut cookies: HashMap<String, String> = HashMap::new();

    // Persistent cookies from cookies.sqlite
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

    // Session cookies from recovery.jsonlz4
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

    Ok(cookies.into_iter().collect())
}

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

fn find_firefox_profile() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;

    let ff_dir = if cfg!(target_os = "macos") {
        home.join("Library")
            .join("Application Support")
            .join("Firefox")
            .join("Profiles")
    } else {
        home.join(".mozilla").join("firefox")
    };

    if !ff_dir.exists() {
        return Err(format!("Firefox profile directory not found ({})", ff_dir.display()));
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

// ── Chrome ──

fn import_chrome_cookies() -> Result<Vec<(String, String)>, String> {
    let cookies_db = find_chrome_cookies_db()?;

    let tmp = std::env::temp_dir().join("myro_chrome_cookies.sqlite");
    std::fs::copy(&cookies_db, &tmp)
        .map_err(|e| format!("Failed to copy Chrome cookies: {}", e))?;

    let conn = rusqlite::Connection::open_with_flags(
        &tmp,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| format!("Failed to open Chrome cookies database: {}", e))?;

    // Chrome encrypts cookie values on macOS/Linux. The `value` column has the
    // plaintext (often empty on newer Chrome), while `encrypted_value` holds the
    // encrypted blob. We try `value` first; if empty we skip (encrypted cookies
    // require keychain access which is complex). For many users the non-encrypted
    // value column still works, and session cookies set via JS are often plaintext.
    let mut stmt = conn
        .prepare(
            "SELECT name, value FROM cookies \
             WHERE host_key LIKE '%codeforces.com' \
             AND value != ''",
        )
        .map_err(|e| format!("Failed to query Chrome cookies: {}", e))?;

    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("Failed to read Chrome cookies: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let _ = std::fs::remove_file(&tmp);

    Ok(rows)
}

fn find_chrome_cookies_db() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;

    let candidates = if cfg!(target_os = "macos") {
        vec![
            home.join("Library/Application Support/Google/Chrome/Default/Cookies"),
            home.join("Library/Application Support/Google/Chrome/Profile 1/Cookies"),
        ]
    } else {
        vec![
            home.join(".config/google-chrome/Default/Cookies"),
            home.join(".config/google-chrome/Profile 1/Cookies"),
            home.join(".config/chromium/Default/Cookies"),
        ]
    };

    for path in candidates {
        if path.exists() {
            return Ok(path);
        }
    }

    Err("Chrome cookies database not found".into())
}

// ── Safari (macOS only) ──

fn import_safari_cookies() -> Result<Vec<(String, String)>, String> {
    if !cfg!(target_os = "macos") {
        return Err("Safari is only available on macOS".into());
    }

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let cookies_db = home.join("Library/Cookies/Cookies.binarycookies");

    if !cookies_db.exists() {
        return Err("Safari cookies file not found".into());
    }

    // Safari's Cookies.binarycookies is a proprietary binary format.
    // Use the `sqlite3` approach via Safari's websitedata instead.
    // On modern macOS, Safari stores cookies in a sqlite db too.
    let safari_data = home.join("Library/Containers/com.apple.Safari/Data/Library/Cookies/Cookies.binarycookies");
    let cookies_path = if safari_data.exists() {
        safari_data
    } else {
        cookies_db
    };

    // Parse Safari's binary cookie format
    parse_safari_binary_cookies(&cookies_path)
}

fn parse_safari_binary_cookies(path: &PathBuf) -> Result<Vec<(String, String)>, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read Safari cookies: {}", e))?;

    if data.len() < 4 || &data[0..4] != b"cook" {
        return Err("Invalid Safari cookies format".into());
    }

    let mut cookies = Vec::new();
    let num_pages = u32::from_be_bytes(data[4..8].try_into().map_err(|_| "truncated header")?);

    // Read page sizes
    let mut page_sizes = Vec::new();
    let mut offset = 8;
    for _ in 0..num_pages {
        if offset + 4 > data.len() {
            break;
        }
        let size = u32::from_be_bytes(data[offset..offset + 4].try_into().map_err(|_| "truncated")?);
        page_sizes.push(size as usize);
        offset += 4;
    }

    // Parse each page
    for page_size in &page_sizes {
        if offset + page_size > data.len() {
            break;
        }
        let page = &data[offset..offset + page_size];
        if let Ok(page_cookies) = parse_safari_cookie_page(page) {
            for (name, value, domain) in page_cookies {
                if domain.contains("codeforces.com") {
                    cookies.push((name, value));
                }
            }
        }
        offset += page_size;
    }

    Ok(cookies)
}

fn parse_safari_cookie_page(page: &[u8]) -> Result<Vec<(String, String, String)>, String> {
    if page.len() < 8 {
        return Err("page too short".into());
    }

    // Page header: 4-byte magic (0x00000100), then 4-byte cookie count
    let num_cookies =
        u32::from_le_bytes(page[4..8].try_into().map_err(|_| "bad count")?) as usize;

    // Cookie offsets follow (4 bytes each)
    let mut cookie_offsets = Vec::new();
    let mut off = 8;
    for _ in 0..num_cookies {
        if off + 4 > page.len() {
            break;
        }
        let cookie_off =
            u32::from_le_bytes(page[off..off + 4].try_into().map_err(|_| "bad offset")?) as usize;
        cookie_offsets.push(cookie_off);
        off += 4;
    }

    let mut result = Vec::new();
    for cookie_off in cookie_offsets {
        if cookie_off + 48 > page.len() {
            continue;
        }
        let c = &page[cookie_off..];
        if c.len() < 48 {
            continue;
        }

        // Cookie record: size(4), flags(4), padding(4),
        // url_offset(4), name_offset(4), path_offset(4), value_offset(4), ...
        let url_off =
            u32::from_le_bytes(c[16..20].try_into().map_err(|_| "bad url_off")?) as usize;
        let name_off =
            u32::from_le_bytes(c[20..24].try_into().map_err(|_| "bad name_off")?) as usize;
        let _path_off =
            u32::from_le_bytes(c[24..28].try_into().map_err(|_| "bad path_off")?) as usize;
        let value_off =
            u32::from_le_bytes(c[28..32].try_into().map_err(|_| "bad value_off")?) as usize;

        let read_cstr = |start: usize| -> String {
            if start >= c.len() {
                return String::new();
            }
            let slice = &c[start..];
            let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            String::from_utf8_lossy(&slice[..end]).to_string()
        };

        let domain = read_cstr(url_off);
        let name = read_cstr(name_off);
        let value = read_cstr(value_off);

        if !name.is_empty() && !value.is_empty() {
            result.push((name, value, domain));
        }
    }

    Ok(result)
}
