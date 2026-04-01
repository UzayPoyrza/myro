use std::path::PathBuf;
use std::sync::mpsc;

/// Requests sent from main thread to recommender thread.
#[allow(dead_code)]
pub enum RecommendRequest {
    /// Fetch user history from CF and fit embedding.
    FetchAndFit {
        handle: String,
        model_path: PathBuf,
    },
    /// Pick a problem near target probability.
    Recommend {
        target_p: f64,
        solved_keys: Vec<String>,
    },
    /// Fetch a problem statement by contest_id and index.
    FetchProblem { contest_id: i64, index: String },
    /// Submit a solution.
    Submit {
        contest_id: i64,
        index: String,
        source_code: String,
        handle: String,
        cookies: Vec<(String, String)>,
        user_agent: String,
    },
    /// Poll for verdict.
    PollVerdict { contest_id: i64 },
    /// Record a solve/fail and refit.
    RecordAndRefit { problem_key: String, solved: bool },
    Quit,
}

/// Events sent from recommender thread back to main thread.
#[allow(dead_code)]
pub enum RecommendEvent {
    /// User embedding fitted successfully.
    EmbeddingReady {
        num_observations: usize,
        user_rating: Option<i32>,
    },
    /// A problem has been recommended.
    ProblemRecommended {
        contest_id: i64,
        index: String,
        predicted_p: f64,
        rating: Option<i32>,
        tags: Vec<String>,
    },
    /// Problem statement fetched and parsed.
    ProblemFetched {
        statement: myro_cf::ProblemStatement,
    },
    /// Solution submitted, waiting for verdict.
    Submitted,
    /// Verdict received.
    Verdict {
        verdict: String,
        problem_index: String,
    },
    /// Embedding refitted after recording a solve/fail.
    Refitted,
    /// Skill profile computed (with optional deltas from previous embedding).
    SkillProfile {
        profile: myro_predict::model::skills::SkillProfile,
        deltas: Vec<myro_predict::model::skills::SkillDelta>,
    },
    /// Error occurred.
    Error { message: String },
    /// Status update for display.
    Status { message: String },
}

pub struct RecommendHandle {
    pub request_tx: mpsc::Sender<RecommendRequest>,
    pub event_rx: mpsc::Receiver<RecommendEvent>,
}

/// Spawn the recommender background thread.
pub fn spawn_recommender() -> RecommendHandle {
    let (req_tx, req_rx) = mpsc::channel::<RecommendRequest>();
    let (evt_tx, evt_rx) = mpsc::channel::<RecommendEvent>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime for recommender");

        rt.block_on(async {
            recommender_loop(req_rx, evt_tx).await;
        });
    });

    RecommendHandle {
        request_tx: req_tx,
        event_rx: evt_rx,
    }
}

pub fn history_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("history.json")
}

async fn recommender_loop(
    req_rx: mpsc::Receiver<RecommendRequest>,
    evt_tx: mpsc::Sender<RecommendEvent>,
) {
    use myro_predict::model::inference::{
        build_observations_from_submissions, fit_user_weighted, predict_all,
        time_decay_weight, DEFAULT_HALF_LIFE_DAYS,
    };
    use myro_predict::model::types::{ProblemModel, UserParams, WeightedObservation};

    let mut model: Option<ProblemModel> = None;
    let mut user_params: Option<UserParams> = None;
    let mut predictions: Option<Vec<f64>> = None;
    let mut auth_client = myro_cf::auth::CfAuthClient::new();
    let cf_client = myro_cf::CfClient::new();
    let hist_path = history_path();
    let mut local_history =
        myro_predict::history::SolveHistory::load(&hist_path).unwrap_or_default();
    let mut solved_keys_set: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    // Persisted across requests so RecordAndRefit can refit with the full dataset
    let mut cf_observations: Vec<WeightedObservation> = Vec::new();
    let mut cf_problem_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut cf_submissions_hash = String::new();
    let cache_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("user_params.bin");

    while let Ok(req) = req_rx.recv() {

        match req {
            RecommendRequest::FetchAndFit { handle, model_path } => {
                // Load model once (skip if already loaded)
                if model.is_none() {
                    let _ = evt_tx.send(RecommendEvent::Status {
                        message: "Loading model...".into(),
                    });
                    match myro_predict::db::model_store::load_problem_model(&model_path) {
                        Ok(m) => model = Some(m),
                        Err(e) => {
                            let _ = evt_tx.send(RecommendEvent::Error {
                                message: format!("Failed to load model: {}", e),
                            });
                            continue;
                        }
                    }
                }
                let m = model.as_ref().unwrap();

                // Fetch submissions from CF
                let _ = evt_tx.send(RecommendEvent::Status {
                    message: format!("Fetching history for {}...", handle),
                });
                let submissions = match cf_client.fetch_user_status(&handle).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Failed to fetch submissions: {}", e),
                        });
                        continue;
                    }
                };

                // Fetch user info for rating
                let user_rating = cf_client
                    .fetch_user_info(&handle)
                    .await
                    .ok()
                    .and_then(|u| u.rating);

                // Build observations from CF submissions
                let now_ts = chrono::Utc::now().timestamp();
                let (cf_obs, sk) = build_observations_from_submissions(
                    m,
                    &submissions,
                    now_ts,
                    DEFAULT_HALF_LIFE_DAYS,
                );
                // Store CF observations for RecordAndRefit to reuse
                cf_observations = cf_obs;
                cf_problem_keys = sk.keys().cloned().collect();
                solved_keys_set = sk
                    .into_iter()
                    .filter(|(_, v)| *v)
                    .map(|(k, _)| k)
                    .collect();

                // Combine CF + local history observations
                let mut obs = cf_observations.clone();
                for entry in &local_history.entries {
                    if cf_problem_keys.contains(&entry.problem_id) {
                        continue;
                    }
                    if let Some(&idx) = m.problem_index.get(&entry.problem_id) {
                        let days_ago = (now_ts - entry.timestamp) as f64 / 86400.0;
                        let w = time_decay_weight(days_ago, DEFAULT_HALF_LIFE_DAYS);
                        obs.push(WeightedObservation {
                            problem_idx: idx,
                            solved: entry.solved,
                            weight: w,
                        });
                    }
                }

                // Cache key combines CF submissions + local history
                let max_id = submissions.iter().map(|s| s.id).max().unwrap_or(0);
                cf_submissions_hash = myro_predict::cache::compute_submissions_hash(
                    submissions.len(),
                    max_id,
                );
                let combined_hash = format!(
                    "{}+{}",
                    cf_submissions_hash,
                    local_history.content_hash()
                );

                // Check cache — skip fitting if hash matches
                let params = match myro_predict::cache::load_cached_params(&cache_path, &combined_hash) {
                    Ok(Some(cached)) => {
                        UserParams {
                            theta: cached.theta,
                            bias: cached.bias,
                        }
                    }
                    _ => {
                        let _ = evt_tx.send(RecommendEvent::Status {
                            message: "Fitting user embedding...".into(),
                        });
                        let p = fit_user_weighted(m, &obs, 0.01, 100, 0.01);
                        let _ = myro_predict::cache::save_cached_params(
                            &myro_predict::cache::CachedUserParams {
                                theta: p.theta.clone(),
                                bias: p.bias,
                                history_hash: combined_hash,
                            },
                            &cache_path,
                        );
                        p
                    }
                };
                let preds = predict_all(&params, m);

                // Compute and send skill profile
                let profile =
                    myro_predict::model::skills::compute_skill_profile(&params, m);
                let _ = evt_tx.send(RecommendEvent::SkillProfile {
                    profile,
                    deltas: vec![],
                });

                user_params = Some(params);
                predictions = Some(preds);

                let _ = evt_tx.send(RecommendEvent::EmbeddingReady {
                    num_observations: obs.len(),
                    user_rating,
                });
            }

            RecommendRequest::Recommend {
                target_p,
                solved_keys,
            } => {
                let m = match model.as_ref() {
                    Some(m) => m,
                    None => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: "Model not loaded".into(),
                        });
                        continue;
                    }
                };
                let preds = match predictions.as_ref() {
                    Some(p) => p,
                    None => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: "Predictions not computed".into(),
                        });
                        continue;
                    }
                };

                let solved_set: std::collections::HashSet<&str> =
                    solved_keys.iter().map(|s| s.as_str()).collect();

                // Build candidate list: unsolved, near target_p
                let margin = 0.1;
                let mut candidates: Vec<(usize, &str, f64, Option<i32>)> = Vec::new();
                for (key, &idx) in &m.problem_index {
                    if solved_set.contains(key.as_str())
                        || solved_keys_set.contains(key.as_str())
                    {
                        continue;
                    }
                    let p = preds[idx];
                    if (p - target_p).abs() <= margin {
                        let rating = m.problem_ratings.get(idx).and_then(|r| *r);
                        candidates.push((idx, key.as_str(), p, rating));
                    }
                }

                if candidates.is_empty() {
                    let _ = evt_tx.send(RecommendEvent::Error {
                        message: format!(
                            "No unsolved problems near P(solve)={:.0}% +/- 10%",
                            target_p * 100.0
                        ),
                    });
                    continue;
                }

                // Pick random candidate
                use rand::seq::SliceRandom;
                let (_idx, key, pred_p, rating) =
                    *candidates.choose(&mut rand::thread_rng()).unwrap();
                let parts: Vec<&str> = key.split(':').collect();
                let (contest_id, index) = if parts.len() == 2 {
                    (parts[0].parse::<i64>().unwrap_or(0), parts[1].to_string())
                } else {
                    (0, key.to_string())
                };

                let tags = m
                    .problem_tags
                    .get(_idx)
                    .cloned()
                    .unwrap_or_default();

                let _ = evt_tx.send(RecommendEvent::ProblemRecommended {
                    contest_id,
                    index,
                    predicted_p: pred_p,
                    rating,
                    tags,
                });
            }

            RecommendRequest::FetchProblem { contest_id, index } => {
                let _ = evt_tx.send(RecommendEvent::Status {
                    message: format!("Fetching problem {}{}...", contest_id, index),
                });
                match cf_client.fetch_problem_html(contest_id, &index).await {
                    Ok(html) => {
                        match myro_cf::parser::parse_problem(&html, contest_id, &index) {
                            Ok(statement) => {
                                let _ =
                                    evt_tx.send(RecommendEvent::ProblemFetched { statement });
                            }
                            Err(e) => {
                                let _ = evt_tx.send(RecommendEvent::Error {
                                    message: format!("Failed to parse problem: {}", e),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Failed to fetch problem: {}", e),
                        });
                    }
                }
            }

            RecommendRequest::Submit {
                contest_id,
                index,
                source_code,
                handle,
                cookies,
                user_agent,
            } => {
                // Load cookies if needed
                if !auth_client.is_logged_in() {
                    auth_client.load_cookies(&handle, &cookies, &user_agent);
                }

                let _ = evt_tx.send(RecommendEvent::Status {
                    message: "Submitting solution...".into(),
                });
                match auth_client
                    .submit_solution(
                        contest_id,
                        &index,
                        &source_code,
                        myro_cf::auth::LANG_PYPY3,
                    )
                    .await
                {
                    Ok(_) => {
                        let _ = evt_tx.send(RecommendEvent::Submitted);
                    }
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Submit failed: {}", e),
                        });
                    }
                }
            }

            RecommendRequest::PollVerdict { contest_id } => {
                match auth_client.poll_latest_verdict(contest_id).await {
                    Ok(Some((verdict, idx))) => {
                        let _ = evt_tx.send(RecommendEvent::Verdict {
                            verdict,
                            problem_index: idx,
                        });
                    }
                    Ok(None) => {
                        // Still judging — caller should poll again
                    }
                    Err(e) => {
                        let _ = evt_tx.send(RecommendEvent::Error {
                            message: format!("Verdict poll failed: {}", e),
                        });
                    }
                }
            }

            RecommendRequest::RecordAndRefit {
                problem_key,
                solved,
            } => {
                // Record in local history
                let now = chrono::Utc::now().timestamp();
                local_history.record(problem_key.clone(), solved, now);
                if let Err(e) = local_history.save(&hist_path) {
                    let _ = evt_tx.send(RecommendEvent::Error {
                        message: format!("Failed to save history: {}", e),
                    });
                }

                if solved {
                    solved_keys_set.insert(problem_key.clone());
                }

                // Refit embedding if model loaded
                if let Some(m) = model.as_ref() {
                    let old_params = user_params.clone();

                    // Start with CF observations, add local history entries not in CF
                    let mut obs = cf_observations.clone();
                    for entry in &local_history.entries {
                        if cf_problem_keys.contains(&entry.problem_id) {
                            continue;
                        }
                        if let Some(&idx) = m.problem_index.get(&entry.problem_id) {
                            let days_ago = (now - entry.timestamp) as f64 / 86400.0;
                            let w = time_decay_weight(days_ago, DEFAULT_HALF_LIFE_DAYS);
                            obs.push(WeightedObservation {
                                problem_idx: idx,
                                solved: entry.solved,
                                weight: w,
                            });
                        }
                    }

                    let params = fit_user_weighted(m, &obs, 0.01, 100, 0.01);
                    let preds = predict_all(&params, m);

                    // Update cache with combined hash
                    let combined_hash = format!(
                        "{}+{}",
                        cf_submissions_hash,
                        local_history.content_hash()
                    );
                    let _ = myro_predict::cache::save_cached_params(
                        &myro_predict::cache::CachedUserParams {
                            theta: params.theta.clone(),
                            bias: params.bias,
                            history_hash: combined_hash,
                        },
                        &cache_path,
                    );

                    // Compute skill profile and deltas
                    let profile =
                        myro_predict::model::skills::compute_skill_profile(&params, m);
                    let deltas = if let Some(old) = &old_params {
                        myro_predict::model::skills::compute_skill_deltas(old, &params, m)
                    } else {
                        vec![]
                    };

                    // Record skill snapshot
                    let trigger = if solved { "solved" } else { "failed" };
                    let snapshot = myro_predict::history::SkillSnapshot {
                        timestamp: now,
                        trigger: trigger.to_string(),
                        problem_key: Some(problem_key),
                        overall_rating: profile.overall_rating,
                        tag_ratings: profile
                            .tag_ratings
                            .iter()
                            .map(|t| (t.tag.clone(), t.effective_rating))
                            .collect(),
                    };
                    let skill_hist_path = dirs::data_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("myro")
                        .join("skill_history.json");
                    let mut skill_history =
                        myro_predict::history::SkillHistory::load(&skill_hist_path)
                            .unwrap_or_default();
                    skill_history.record(snapshot);
                    let _ = skill_history.save(&skill_hist_path);

                    let _ = evt_tx.send(RecommendEvent::SkillProfile {
                        profile,
                        deltas,
                    });

                    user_params = Some(params);
                    predictions = Some(preds);
                }

                let _ = evt_tx.send(RecommendEvent::Refitted);
            }

            RecommendRequest::Quit => break,
        }
    }

    // Suppress unused variable warning
    let _ = user_params;
}
