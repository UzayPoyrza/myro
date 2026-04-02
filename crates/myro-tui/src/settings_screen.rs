use crossterm::event::{KeyCode, KeyEvent};
use std::sync::mpsc;

use crate::app::{App, AppState, ConfirmPopup, LoginPhase, SettingsItem, SETTINGS_ITEMS, models_for_provider};
use crate::onboarding;

impl App {
    pub(crate) fn handle_settings_key(&mut self, key: KeyEvent) {
        // Handle dropdown mode first
        if self.settings_in_dropdown_mode() {
            self.handle_settings_dropdown_key(key);
            return;
        }

        if self.settings_in_edit_mode() {
            if self.handle_settings_edit_key(key) {
                return;
            }
        }

        self.handle_settings_navigation_key(key);
    }

    fn settings_in_edit_mode(&self) -> bool {
        matches!(&self.state, AppState::Settings { editing: Some(_), .. })
    }

    fn settings_in_dropdown_mode(&self) -> bool {
        matches!(&self.state, AppState::Settings { dropdown: Some(_), .. })
    }

    fn handle_settings_dropdown_key(&mut self, key: KeyEvent) {
        let (selected, dropdown_idx) = match &self.state {
            AppState::Settings { selected, dropdown: Some(idx), .. } => (*selected, *idx),
            _ => return,
        };

        let field = match &SETTINGS_ITEMS[selected] {
            SettingsItem::Dropdown { field, .. } => *field,
            _ => return,
        };

        let options = self.dropdown_options(field);

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let AppState::Settings { dropdown, .. } = &mut self.state {
                    *dropdown = Some((dropdown_idx + 1) % options.len());
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let AppState::Settings { dropdown, .. } = &mut self.state {
                    *dropdown = Some(if dropdown_idx == 0 { options.len() - 1 } else { dropdown_idx - 1 });
                }
            }
            KeyCode::Enter => {
                if field == "coach.model" {
                    self.apply_model(options[dropdown_idx]);
                } else {
                    self.apply_provider(options[dropdown_idx]);
                }
                if let AppState::Settings { dropdown, .. } = &mut self.state {
                    *dropdown = None;
                }
            }
            KeyCode::Esc => {
                if let AppState::Settings { dropdown, .. } = &mut self.state {
                    *dropdown = None;
                }
            }
            _ => {}
        }
    }

    /// Get the correct dropdown options, using dynamic model list for coach.model.
    pub fn dropdown_options(&self, field: &str) -> &'static [&'static str] {
        if field == "coach.model" {
            let provider = detect_provider(self.coach_config.base_url.as_deref())
                .unwrap_or_else(|| "OpenRouter".into());
            models_for_provider(&provider)
        } else {
            match SETTINGS_ITEMS.iter().find(|item| matches!(item, SettingsItem::Dropdown { field: f, .. } if *f == field)) {
                Some(SettingsItem::Dropdown { options, .. }) => options,
                _ => &[],
            }
        }
    }

    fn handle_settings_edit_key(&mut self, key: KeyEvent) -> bool {
        let (selected, field_value) = match &mut self.state {
            AppState::Settings {
                selected,
                editing: Some(buf),
                ..
            } => match key.code {
                KeyCode::Char(c) => {
                    buf.push(c);
                    return true;
                }
                KeyCode::Backspace => {
                    buf.pop();
                    return true;
                }
                KeyCode::Esc => (*selected, None),
                KeyCode::Enter => (*selected, Some(buf.clone())),
                _ => return true,
            },
            _ => return false,
        };

        if let Some(value) = field_value {
            match &SETTINGS_ITEMS[selected] {
                SettingsItem::Editable { field, .. }
                | SettingsItem::EditableSensitive { field, .. } => {
                    if self.apply_setting(field, &value) {
                        self.save_coach_config();
                        let _ = self.app_config.save();
                    }
                }
                _ => {}
            }
        }

        if let AppState::Settings { editing, .. } = &mut self.state {
            *editing = None;
        }

        true
    }

    fn handle_settings_navigation_key(&mut self, key: KeyEvent) {
        let selected = match &self.state {
            AppState::Settings { selected, .. } => *selected,
            _ => return,
        };

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let AppState::Settings { selected, .. } = &mut self.state {
                    *selected = next_selectable(*selected, 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let AppState::Settings { selected, .. } = &mut self.state {
                    *selected = next_selectable(*selected, -1);
                }
            }
            KeyCode::Enter => match &SETTINGS_ITEMS[selected] {
                SettingsItem::Section { .. } => {}
                SettingsItem::Display { .. } => {}
                SettingsItem::Editable { field, .. }
                | SettingsItem::EditableSensitive { field, .. } => {
                    let current = self.read_setting(field);
                    if let AppState::Settings { editing, .. } = &mut self.state {
                        *editing = Some(current);
                    }
                }
                SettingsItem::Dropdown { field, .. } => {
                    let options = self.dropdown_options(field);
                    let display = self.read_setting_display(field);
                    let current_idx = options
                        .iter()
                        .position(|&o| o == display)
                        .unwrap_or(0);
                    if let AppState::Settings { dropdown, .. } = &mut self.state {
                        *dropdown = Some(current_idx);
                    }
                }
                SettingsItem::Action { action, .. } => {
                    self.execute_settings_action(action);
                }
            },
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state = AppState::Home { selected: 3 };
            }
            _ => {}
        }
    }

    fn execute_settings_action(&mut self, action: &str) {
        match action {
            "reimport_cookies" => match onboarding::reimport_cookies(&mut self.app_config) {
                Ok(message) => self.set_status(message),
                Err(e) => self.set_status(format!("cookie import failed: {}", e)),
            },
            "reset_history" => {
                self.confirm_popup = Some(ConfirmPopup {
                    title: "reset history",
                    message: "this will clear all solve history, cached predictions, and /isuck state. continue?",
                    action: "reset_history",
                });
            }
            "sign_in" => {
                if self.api.is_some() {
                    self.set_status("already signed in");
                } else {
                    self.state = AppState::Login {
                        phase: LoginPhase::ChooseMethod,
                        selected: 0,
                        auth_rx: None,
                    };
                }
            }
            "logout" => self.logout(),
            _ => {}
        }
    }

    fn logout(&mut self) {
        // Flush any pending events before clearing
        if let Some(ref events) = self.events {
            events.track("session_end", serde_json::json!({}));
            let _ = events.flush();
        }
        self.api = None;
        self.events = None;

        self.state = onboarding::logout(
            &mut self.app_config,
            &mut self.user_state,
            &mut self.recommender,
        );
    }

    fn read_setting(&self, field: &str) -> String {
        match field {
            "codeforces.handle" => self.app_config.codeforces.handle.clone().unwrap_or_default(),
            "recommender.target_probability" => {
                format!("{}", self.app_config.recommender.target_probability)
            }
            "coach.base_url" => self
                .coach_config
                .base_url
                .clone()
                .unwrap_or_default(),
            "coach.api_key" => self
                .coach_config
                .api_key
                .clone()
                .unwrap_or_default(),
            "coach.model" => self
                .coach_config
                .model
                .clone()
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    pub fn read_setting_display(&self, field: &str) -> String {
        match field {
            "codeforces.handle" => self
                .app_config
                .codeforces
                .handle
                .clone()
                .unwrap_or_else(|| "(not set)".into()),
            "recommender.target_probability" => {
                format!("{}", self.app_config.recommender.target_probability)
            }
            "coach.provider" => detect_provider(self.coach_config.base_url.as_deref())
                .unwrap_or_else(|| "(not set)".into()),
            "coach.api_key" => {
                match &self.coach_config.api_key {
                    Some(key) if !key.is_empty() => {
                        let visible: String = key.chars().take(4).collect();
                        format!("{}...", visible)
                    }
                    _ => "(not set)".into(),
                }
            }
            "coach.model" => self
                .coach_config
                .model
                .clone()
                .unwrap_or_else(|| "(auto)".into()),
            _ => String::new(),
        }
    }

    /// Whether we show "sign in" or "sign out" for the account action.
    pub fn is_signed_in(&self) -> bool {
        self.api.is_some()
    }

    fn apply_provider(&mut self, provider: &str) {
        let (url, model) = provider_config(provider);
        self.coach_config.base_url = Some(url.to_string());
        self.coach_config.model = Some(model.to_string());
        self.save_coach_config();
        self.set_status(format!("\u{2713} provider set to {}", provider));
        self.spawn_connection_test();
    }

    fn apply_model(&mut self, model: &str) {
        self.coach_config.model = Some(model.to_string());
        self.save_coach_config();
        self.set_status(format!("\u{2713} model set to {}", model));
        self.spawn_connection_test();
    }

    fn spawn_connection_test(&mut self) {
        let base_url = match &self.coach_config.base_url {
            Some(url) => url.clone(),
            None => return,
        };
        let api_key = self.coach_config.api_key.clone();
        let model = self.coach_config.model.clone().unwrap_or_default();

        if api_key.is_none() || model.is_empty() {
            return;
        }

        self.set_status("testing connection...");

        let (tx, rx) = mpsc::channel();
        self.connection_test_rx = Some(rx);

        std::thread::spawn(move || {
            let result = myro_coach::llm::openai_compat::test_connection(
                &base_url,
                api_key.as_deref(),
                &model,
            );
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
    }

    fn apply_setting(&mut self, field: &str, value: &str) -> bool {
        match field {
            "recommender.target_probability" => {
                if let Ok(p) = value.trim().parse::<f64>() {
                    if (0.1..=0.9).contains(&p) {
                        self.app_config.recommender.target_probability = p;
                        return true;
                    }
                }
                self.set_status("target probability must be 0.1-0.9");
                false
            }
            "coach.api_key" => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    self.coach_config.api_key = None;
                    self.set_status("api key cleared");
                } else {
                    self.coach_config.api_key = Some(trimmed.to_string());
                    self.set_status(format!("\u{2713} api key set"));
                    self.spawn_connection_test();
                }
                true
            }
            _ => false,
        }
    }

    /// Save coach config fields back to ~/.config/myro/config.toml
    /// Merges into existing file to preserve other sections.
    fn save_coach_config(&self) {
        let path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from(".config"))
            .join("myro")
            .join("config.toml");

        let mut doc: toml::map::Map<String, toml::Value> =
            if let Ok(existing) = std::fs::read_to_string(&path) {
                toml::from_str(&existing).unwrap_or_default()
            } else {
                toml::map::Map::new()
            };

        // Build coach section
        let mut coach = toml::map::Map::new();
        coach.insert(
            "enabled".into(),
            toml::Value::Boolean(self.coach_config.enabled),
        );
        if self.coach_config.mock {
            coach.insert("mock".into(), toml::Value::Boolean(true));
        }
        if let Some(ref url) = self.coach_config.base_url {
            coach.insert("base_url".into(), toml::Value::String(url.clone()));
        }
        if let Some(ref key) = self.coach_config.api_key {
            coach.insert("api_key".into(), toml::Value::String(key.clone()));
        }
        if let Some(ref model) = self.coach_config.model {
            coach.insert("model".into(), toml::Value::String(model.clone()));
        }
        doc.insert("coach".into(), toml::Value::Table(coach));

        if let Ok(contents) = toml::to_string_pretty(&doc) {
            let _ = std::fs::write(&path, contents);
        }
    }
}

/// Map provider name to (base_url, default_model).
fn provider_config(provider: &str) -> (&'static str, &'static str) {
    match provider {
        "OpenRouter" => ("https://openrouter.ai/api/v1", "anthropic/claude-sonnet-4"),
        "Anthropic" => ("https://api.anthropic.com/v1", "claude-sonnet-4-20250514"),
        "OpenAI" => ("https://api.openai.com/v1", "gpt-4o"),
        "Google" => ("https://generativelanguage.googleapis.com/v1beta/openai", "gemini-2.0-flash"),
        _ => ("https://openrouter.ai/api/v1", "anthropic/claude-sonnet-4"),
    }
}

/// Detect which provider is currently configured from the base_url.
fn detect_provider(base_url: Option<&str>) -> Option<String> {
    let url = base_url?;
    if url.contains("openrouter.ai") {
        Some("OpenRouter".into())
    } else if url.contains("api.anthropic.com") {
        Some("Anthropic".into())
    } else if url.contains("api.openai.com") {
        Some("OpenAI".into())
    } else if url.contains("generativelanguage.googleapis.com") {
        Some("Google".into())
    } else {
        Some(url.to_string())
    }
}

/// Navigate to the next/prev selectable settings item, skipping Section headers.
fn next_selectable(current: usize, direction: i32) -> usize {
    let len = SETTINGS_ITEMS.len();
    let mut idx = current;
    loop {
        if direction > 0 {
            idx = (idx + 1) % len;
        } else {
            idx = if idx == 0 { len - 1 } else { idx - 1 };
        }
        if SETTINGS_ITEMS[idx].is_selectable() || idx == current {
            return idx;
        }
    }
}
