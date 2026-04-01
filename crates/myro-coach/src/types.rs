use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HintLevel {
    None = 0,
    Nudge = 1,
    Question = 2,
    Formal = 3,
}

impl HintLevel {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Nudge,
            2 => Self::Question,
            3 => Self::Formal,
            _ => Self::None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Nudge,
            Self::Nudge => Self::Question,
            Self::Question => Self::Formal,
            Self::Formal => Self::Formal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationState {
    Locked,
    Approaching,
    Found,
}

impl ObservationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Locked => "locked",
            Self::Approaching => "approaching",
            Self::Found => "found",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "approaching" => Self::Approaching,
            "found" => Self::Found,
            _ => Self::Locked,
        }
    }
}

/// Confidence indicator for coach state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    Observing,
    Concerned,
    ReadyToHelp,
    Intervening,
}

/// Format for ghost text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GhostFormat {
    Code,
    Natural,
}

/// Structured response from the coaching LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachResponse {
    pub state: String,
    pub confidence: f64,
    pub matched_observation_id: Option<String>,
    pub coach_message: String,
    pub ghost_text: Option<String>,
    pub ghost_format: Option<GhostFormat>,
    pub next_action: Option<String>,
}

/// Trigger that caused a coaching intervention
#[derive(Debug, Clone)]
pub enum InterventionTrigger {
    Stall { idle_secs: u64 },
    UserRequested,
}

impl InterventionTrigger {
    pub fn description(&self) -> String {
        match self {
            Self::Stall { idle_secs } => format!("user idle for {}s", idle_secs),
            Self::UserRequested => "user requested help".to_string(),
        }
    }
}
