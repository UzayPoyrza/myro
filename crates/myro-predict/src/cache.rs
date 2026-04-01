use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Cached user parameters with hash-based invalidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUserParams {
    pub theta: Vec<f64>,
    pub bias: f64,
    pub history_hash: String,
}

/// Save cached user params to a bincode file.
pub fn save_cached_params(params: &CachedUserParams, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    let encoded = bincode::serialize(params).context("Failed to serialize cached params")?;
    std::fs::write(path, encoded)
        .with_context(|| format!("Failed to write cache to {}", path.display()))?;
    Ok(())
}

/// Load cached user params, returning None if the file doesn't exist
/// or the hash doesn't match the expected value.
pub fn load_cached_params(path: &Path, expected_hash: &str) -> Result<Option<CachedUserParams>> {
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read(path)
        .with_context(|| format!("Failed to read cache from {}", path.display()))?;
    let cached: CachedUserParams = match bincode::deserialize(&data) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    if cached.history_hash != expected_hash {
        return Ok(None);
    }
    Ok(Some(cached))
}

/// Compute a hash from submission count and latest submission ID.
/// Invalidates when new submissions appear.
pub fn compute_submissions_hash(count: usize, latest_id: i64) -> String {
    let mut hasher = DefaultHasher::new();
    count.hash(&mut hasher);
    latest_id.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("user_params.bin");

        let params = CachedUserParams {
            theta: vec![1.0, 2.0, 3.0],
            bias: 0.5,
            history_hash: "abc123".to_string(),
        };
        save_cached_params(&params, &path).unwrap();

        let loaded = load_cached_params(&path, "abc123").unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.theta, vec![1.0, 2.0, 3.0]);
        assert_eq!(loaded.bias, 0.5);
    }

    #[test]
    fn test_cache_miss_wrong_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("user_params.bin");

        let params = CachedUserParams {
            theta: vec![1.0],
            bias: 0.0,
            history_hash: "abc123".to_string(),
        };
        save_cached_params(&params, &path).unwrap();

        let loaded = load_cached_params(&path, "different_hash").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_cache_miss_no_file() {
        let loaded =
            load_cached_params(Path::new("/tmp/nonexistent_myro_cache.bin"), "any").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_submissions_hash_stable() {
        let h1 = compute_submissions_hash(100, 12345);
        let h2 = compute_submissions_hash(100, 12345);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_submissions_hash_changes() {
        let h1 = compute_submissions_hash(100, 12345);
        let h2 = compute_submissions_hash(101, 12345);
        let h3 = compute_submissions_hash(100, 12346);
        assert_ne!(h1, h2);
        assert_ne!(h1, h3);
    }
}
