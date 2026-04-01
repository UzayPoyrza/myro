use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// A single entry in the solve history.
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct HistoryEntry {
    pub problem_id: String,
    pub solved: bool,
    pub timestamp: i64,
}

/// Local solve history for a user, persisted as JSON.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SolveHistory {
    pub entries: Vec<HistoryEntry>,
}

impl SolveHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Record a solve/attempt. Updates existing entry if problem already recorded.
    pub fn record(&mut self, problem_id: String, solved: bool, timestamp: i64) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.problem_id == problem_id) {
            // Upgrade to solved if newly solved, update timestamp
            if solved && !entry.solved {
                entry.solved = true;
            }
            if timestamp > entry.timestamp {
                entry.timestamp = timestamp;
            }
        } else {
            self.entries.push(HistoryEntry {
                problem_id,
                solved,
                timestamp,
            });
        }
    }

    /// Compute a content hash for cache invalidation (not cryptographic).
    pub fn content_hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.entries.len().hash(&mut hasher);
        for entry in &self.entries {
            entry.hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }

    /// Save to a JSON file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("Failed to serialize history")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }
        std::fs::write(path, json)
            .with_context(|| format!("Failed to write history to {}", path.display()))?;
        Ok(())
    }

    /// Load from a JSON file. Returns empty history if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read history from {}", path.display()))?;
        let history: Self =
            serde_json::from_str(&json).context("Failed to deserialize history")?;
        Ok(history)
    }
}

/// A snapshot of skill ratings at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSnapshot {
    pub timestamp: i64,
    /// What triggered this snapshot: "solved", "failed", "gave_up", "initial"
    pub trigger: String,
    pub problem_key: Option<String>,
    pub overall_rating: i32,
    pub tag_ratings: HashMap<String, i32>,
}

/// History of skill rating snapshots, persisted as JSON.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillHistory {
    pub snapshots: Vec<SkillSnapshot>,
}

impl SkillHistory {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    /// Record a new skill snapshot. Caps at 500 entries by downsampling older half.
    pub fn record(&mut self, snapshot: SkillSnapshot) {
        self.snapshots.push(snapshot);
        if self.snapshots.len() > 500 {
            // Keep the newer half as-is, downsample the older half by 2x
            let mid = self.snapshots.len() / 2;
            let older: Vec<SkillSnapshot> = self.snapshots[..mid]
                .iter()
                .step_by(2)
                .cloned()
                .collect();
            let newer = self.snapshots[mid..].to_vec();
            self.snapshots = older;
            self.snapshots.extend(newer);
        }
    }

    /// Get rating history for a specific tag.
    pub fn tag_history(&self, tag: &str) -> Vec<(i64, i32)> {
        self.snapshots
            .iter()
            .filter_map(|s| s.tag_ratings.get(tag).map(|&r| (s.timestamp, r)))
            .collect()
    }

    /// Save to a JSON file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize skill history")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }
        std::fs::write(path, json)
            .with_context(|| format!("Failed to write skill history to {}", path.display()))?;
        Ok(())
    }

    /// Load from a JSON file. Returns empty history if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read skill history from {}", path.display()))?;
        let history: Self =
            serde_json::from_str(&json).context("Failed to deserialize skill history")?;
        Ok(history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");

        let mut history = SolveHistory::new();
        history.record("cf:1800A".into(), true, 1000);
        history.record("cf:1801B".into(), false, 2000);
        history.save(&path).unwrap();

        let loaded = SolveHistory::load(&path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert!(loaded.entries[0].solved);
        assert!(!loaded.entries[1].solved);
    }

    #[test]
    fn test_hash_changes_on_update() {
        let mut history = SolveHistory::new();
        history.record("cf:1800A".into(), true, 1000);
        let hash1 = history.content_hash();

        history.record("cf:1801B".into(), false, 2000);
        let hash2 = history.content_hash();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_record_upgrade() {
        let mut history = SolveHistory::new();
        history.record("cf:1800A".into(), false, 1000);
        assert!(!history.entries[0].solved);

        history.record("cf:1800A".into(), true, 2000);
        assert!(history.entries[0].solved);
        assert_eq!(history.entries[0].timestamp, 2000);
        assert_eq!(history.entries.len(), 1);
    }

    #[test]
    fn test_load_missing_file() {
        let history = SolveHistory::load(Path::new("/tmp/nonexistent_myro_history.json")).unwrap();
        assert!(history.entries.is_empty());
    }
}
