use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct CoachConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub mock: bool,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    #[serde(default = "default_stall_threshold")]
    pub stall_threshold_secs: u64,
    #[serde(default = "default_max_interventions")]
    pub max_interventions: u32,
    #[serde(default = "default_max_auto_interventions")]
    pub max_auto_interventions: u32,
    #[serde(default = "default_true")]
    pub ghost_text_enabled: bool,
}

fn default_true() -> bool {
    true
}
fn default_stall_threshold() -> u64 {
    90
}
fn default_max_interventions() -> u32 {
    8
}
fn default_max_auto_interventions() -> u32 {
    3
}

impl Default for CoachConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mock: false,
            base_url: Some("https://mega4090.taild22ffc.ts.net:8081/v1".to_string()),
            api_key: None,
            model: None,
            stall_threshold_secs: 90,
            max_interventions: 8,
            max_auto_interventions: 3,
            ghost_text_enabled: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    coach: Option<CoachConfig>,
}

impl CoachConfig {
    /// Load config from TOML file, then apply env var overrides.
    /// Returns default config if file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = config_file_path();
        let mut config = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let file: ConfigFile = toml::from_str(&content)?;
            file.coach.unwrap_or_default()
        } else {
            Self::default()
        };

        // Env var overrides
        if std::env::var("MYRO_COACH_MOCK").is_ok() {
            config.mock = true;
        }
        if let Ok(url) = std::env::var("MYRO_LLM_BASE_URL") {
            config.base_url = Some(url);
        }
        if let Ok(key) = std::env::var("MYRO_LLM_API_KEY") {
            config.api_key = Some(key);
        }
        if let Ok(model) = std::env::var("MYRO_LLM_MODEL") {
            config.model = Some(model);
        }

        Ok(config)
    }

    /// Check if the coach can actually function (needs a base_url or mock mode)
    pub fn is_available(&self) -> bool {
        self.enabled && (self.mock || self.base_url.is_some())
    }
}

fn config_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("config.toml")
}
