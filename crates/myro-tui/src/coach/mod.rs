pub mod bridge;
pub mod ghost;
pub mod panel;

use std::sync::mpsc;
use std::time::Instant;

use myro_coach::intervention::InterventionEngine;
use myro_coach::types::{ConfidenceLevel, CoachResponse, GhostFormat};

/// A line in the coach panel
#[derive(Debug, Clone)]
pub struct CoachLine {
    pub text: String,
    #[allow(dead_code)]
    pub is_header: bool,
}

/// Ghost text to display below the cursor
#[derive(Debug, Clone)]
pub struct GhostTextState {
    pub text: String,
    pub format: GhostFormat,
    pub appeared_at: u64, // tick when it appeared (for fade-in)
}

/// Request sent from main thread to coach background thread
pub enum CoachRequest {
    Analyze {
        code: String,
        trigger: String,
        elapsed_secs: u64,
    },
    UserMessage {
        message: String,
        code: String,
        elapsed_secs: u64,
    },
    RequestHint {
        code: String,
        elapsed_secs: u64,
        hint_count: usize,
    },
    Quit,
}

/// Event received from coach background thread
pub enum CoachEvent {
    CoachMessage { response: CoachResponse },
    Error { message: String },
    Debug { message: String },
}

pub struct CoachState {
    pub request_tx: mpsc::Sender<CoachRequest>,
    pub event_rx: mpsc::Receiver<CoachEvent>,
    pub panel_visible: bool,
    pub panel_lines: Vec<CoachLine>,
    pub ghost_text: Option<GhostTextState>,
    pub confidence: ConfidenceLevel,
    pub hint_count: usize,
    #[allow(dead_code)]
    pub session_id: String,
    pub intervention_engine: InterventionEngine,
    pub last_line_count: usize,
    pub started_at: Instant,
    pub last_snapshot_tick: u64,
    /// True while waiting for an LLM response
    pub thinking: bool,
    /// When the current thinking started (for elapsed display)
    pub thinking_started: Option<Instant>,
    /// True when the last message was a hint response
    pub is_hint: bool,
}

impl CoachState {
    pub fn new(
        request_tx: mpsc::Sender<CoachRequest>,
        event_rx: mpsc::Receiver<CoachEvent>,
        session_id: String,
        stall_threshold: u64,
        max_interventions: u32,
    ) -> Self {
        Self {
            request_tx,
            event_rx,
            panel_visible: false,
            panel_lines: vec![],
            ghost_text: None,
            confidence: ConfidenceLevel::Observing,
            hint_count: 0,
            session_id,
            intervention_engine: InterventionEngine::new(stall_threshold, max_interventions),
            last_line_count: 0,
            started_at: Instant::now(),
            last_snapshot_tick: 0,
            thinking: false,
            thinking_started: None,
            is_hint: false,
        }
    }

    /// Send a request, but skip if already thinking (dedup).
    /// Returns a short description of what happened (for debug log).
    pub fn send_request(&mut self, request: CoachRequest) -> Option<String> {
        let label = match &request {
            CoachRequest::Analyze { trigger, .. } => format!("analyze({})", trigger),
            CoachRequest::UserMessage { message, .. } => {
                format!("user_msg(\"{}\")", &message[..message.len().min(30)])
            }
            CoachRequest::RequestHint { .. } => "hint".to_string(),
            CoachRequest::Quit => "quit".to_string(),
        };
        if self.thinking {
            return Some(format!("request skipped (thinking): {}", label));
        }
        let is_hint = matches!(request, CoachRequest::RequestHint { .. });
        if self.request_tx.send(request).is_ok() {
            self.thinking = true;
            self.thinking_started = Some(Instant::now());
            self.is_hint = is_hint;
            if is_hint {
                self.hint_count += 1;
            }
            Some(format!("request sent: {}", label))
        } else {
            Some(format!("request send failed: {}", label))
        }
    }

    /// Process a coach response, updating panel and ghost text
    pub fn apply_response(&mut self, response: &CoachResponse, current_tick: u64) {
        self.thinking = false;
        self.thinking_started = None;

        // Truncate to first 2 sentences for display
        let display_msg = truncate_to_sentences(&response.coach_message, 2);

        // Update panel lines
        self.panel_lines.clear();
        self.panel_lines.push(CoachLine {
            text: display_msg,
            is_header: false,
        });

        // Update ghost text
        if let Some(text) = &response.ghost_text {
            if !text.is_empty() {
                self.ghost_text = Some(GhostTextState {
                    text: text.clone(),
                    format: response.ghost_format.unwrap_or(GhostFormat::Natural),
                    appeared_at: current_tick,
                });
            }
        }

        // Update confidence based on state
        self.confidence = match response.state.as_str() {
            "found" | "approaching" => ConfidenceLevel::Observing,
            "moving_away" => ConfidenceLevel::Concerned,
            _ => ConfidenceLevel::ReadyToHelp,
        };
    }

    /// Handle an error from the coach thread
    pub fn apply_error(&mut self, message: &str) {
        self.thinking = false;
        self.thinking_started = None;
        self.panel_lines = vec![CoachLine {
            text: format!("Error: {}", message),
            is_header: false,
        }];
    }

    /// Dismiss ghost text (on any keypress)
    pub fn dismiss_ghost_text(&mut self) {
        self.ghost_text = None;
    }
}

/// Truncate a message to the first N sentences.
/// Splits on `. `, `? `, `! ` boundaries. If no sentence boundary found, returns as-is.
fn truncate_to_sentences(text: &str, max_sentences: usize) -> String {
    let mut count = 0;
    let bytes = text.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if (bytes[i] == b'.' || bytes[i] == b'?' || bytes[i] == b'!') && bytes[i + 1] == b' ' {
            count += 1;
            if count >= max_sentences {
                return text[..=i].to_string();
            }
        }
    }
    text.to_string()
}
