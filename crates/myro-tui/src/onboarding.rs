use std::sync::mpsc;

use anyhow::Result;

use crate::{
    app::{AppState, OnboardingPhase},
    browser,
    config::AppConfig,
    recommender_state::RecommenderState,
    state::{self, UserState},
};

pub fn initial_app_state(app_config: &AppConfig) -> AppState {
    if app_config.codeforces.handle.is_some() && !app_config.codeforces.cookies.is_empty() {
        AppState::Home { selected: 0 }
    } else if let Some(handle) = &app_config.codeforces.handle {
        AppState::HandlePrompt {
            phase: OnboardingPhase::CookieImport,
            handle_input: handle.clone(),
            error: None,
            validating: false,
            validate_rx: None,
        }
    } else {
        AppState::HandlePrompt {
            phase: OnboardingPhase::Handle,
            handle_input: String::new(),
            error: None,
            validating: false,
            validate_rx: None,
        }
    }
}

pub fn poll_handle_validation(state: &mut AppState) {
    if let AppState::HandlePrompt {
        phase: OnboardingPhase::Handle,
        handle_input,
        validating,
        validate_rx,
        error,
        ..
    } = state
    {
        if *validating {
            if let Some(rx) = validate_rx {
                match rx.try_recv() {
                    Ok(Ok(_user)) => {
                        let handle = handle_input.clone();
                        *validating = false;
                        *validate_rx = None;
                        *error = None;
                        *state = AppState::HandlePrompt {
                            phase: OnboardingPhase::CookieImport,
                            handle_input: handle,
                            error: None,
                            validating: false,
                            validate_rx: None,
                        };
                    }
                    Ok(Err(msg)) => {
                        *error = Some(msg);
                        *validating = false;
                        *validate_rx = None;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        *error = Some("validation failed (connection lost)".into());
                        *validating = false;
                        *validate_rx = None;
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
        }
    }
}

pub fn start_handle_validation(state: &mut AppState) {
    let handle = match state {
        AppState::HandlePrompt {
            handle_input,
            error,
            ..
        } => {
            let handle = handle_input.trim().to_string();
            if handle.is_empty() {
                *error = Some("handle cannot be empty".into());
                return;
            }
            handle
        }
        _ => return,
    };

    let (tx, rx) = mpsc::channel();
    let handle_for_thread = handle.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(async {
            let client = myro_cf::CfClient::new();
            client.fetch_user_info(&handle_for_thread).await
        });
        let _ = tx.send(result.map_err(|e| format!("{}", e)));
    });

    if let AppState::HandlePrompt {
        validating,
        validate_rx,
        ..
    } = state
    {
        *validating = true;
        *validate_rx = Some(rx);
    }
}

pub struct CookieImportSuccess {
    pub status_message: String,
}

pub fn import_cookies(
    app_config: &mut AppConfig,
    user_state: &mut UserState,
    recommender: &mut RecommenderState,
    handle: String,
) -> Result<CookieImportSuccess, String> {
    let result = browser::import_cf_cookies()?;
    let count = result.cookies.len();
    let ua = browser::detect_browser_ua(result.browser);

    app_config.codeforces.handle = Some(handle.clone());
    app_config.codeforces.cookies = result.cookies;
    app_config.codeforces.user_agent = Some(ua);
    app_config
        .save()
        .map_err(|e| format!("failed to save config: {}", e))?;

    user_state.name = Some(handle.clone());
    let _ = state::save_state(user_state);

    recommender.skip_auto_recommend = true;
    recommender.send(crate::recommend::RecommendRequest::FetchAndFit {
        handle,
        model_path: app_config.recommender.model_path.clone(),
    });

    Ok(CookieImportSuccess {
        status_message: format!("\u{2713} Imported {} cookies from {}", count, result.browser),
    })
}

pub fn reimport_cookies(app_config: &mut AppConfig) -> Result<String, String> {
    let result = browser::import_cf_cookies()?;
    let count = result.cookies.len();
    let ua = browser::detect_browser_ua(result.browser);

    app_config.codeforces.cookies = result.cookies;
    app_config.codeforces.user_agent = Some(ua);
    app_config
        .save()
        .map_err(|e| format!("failed to save config: {}", e))?;

    Ok(format!("\u{2713} Imported {} cookies from {}", count, result.browser))
}

pub fn logout(
    app_config: &mut AppConfig,
    user_state: &mut UserState,
    recommender: &mut RecommenderState,
) -> AppState {
    app_config.codeforces.handle = None;
    app_config.codeforces.cookies.clear();
    app_config.codeforces.user_agent = None;
    let _ = app_config.save();

    user_state.name = None;
    recommender.clear_pending_problem(user_state);
    recommender.clear_profile_cache();
    recommender.clear_runtime_state();

    // Clear Supabase auth tokens
    myro_api::auth::clear_tokens();

    AppState::HandlePrompt {
        phase: OnboardingPhase::Handle,
        handle_input: String::new(),
        error: None,
        validating: false,
        validate_rx: None,
    }
}
