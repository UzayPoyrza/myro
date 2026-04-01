use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// App-level config stored in ~/.config/myro/config.toml
/// Separate from CoachConfig — this covers codeforces auth and recommender settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub codeforces: CfConfig,
    #[serde(default)]
    pub recommender: RecommenderConfig,
    #[serde(default)]
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    #[serde(default = "default_release_url")]
    pub release_url: String,
    #[serde(default = "default_auto_check")]
    pub auto_check: bool,
}

fn default_release_url() -> String {
    "https://server.taild22ffc.ts.net:3030/api/v1/repos/kalpturer/myro".to_string()
}

fn default_auto_check() -> bool {
    true
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            release_url: default_release_url(),
            auto_check: default_auto_check(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CfConfig {
    pub handle: Option<String>,
    /// Imported browser cookies for CF session.
    #[serde(default)]
    pub cookies: Vec<(String, String)>,
    /// Firefox user-agent string (detected at cookie import time).
    #[serde(default)]
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommenderConfig {
    /// Target solve probability for recommendations (0.1-0.9).
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
    // Try repo-bundled model first (same pattern as myro-coach's default_problem_set_dir)
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("crates/myro-predict/problem_model.bin.gz"));
    if let Some(ref p) = repo_path {
        if p.exists() {
            return p.clone();
        }
    }

    // Fall back to XDG data dir
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
    /// Preserves unknown sections (e.g. [coach]) that AppConfig doesn't own.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        // Read existing file to preserve sections we don't own (e.g. [coach])
        let mut doc: toml::map::Map<String, toml::Value> =
            if let Ok(existing) = std::fs::read_to_string(&path) {
                toml::from_str(&existing).unwrap_or_default()
            } else {
                toml::map::Map::new()
            };

        // Serialize our sections and merge them into the existing doc
        let ours: toml::Value =
            toml::Value::try_from(self).context("Failed to serialize config")?;
        if let toml::Value::Table(our_table) = ours {
            for (key, value) in our_table {
                doc.insert(key, value);
            }
        }

        let contents =
            toml::to_string_pretty(&doc).context("Failed to serialize merged config")?;
        std::fs::write(&path, contents).context("Failed to write config file")?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("myro")
            .join("config.toml")
    }
}
