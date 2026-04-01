use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::runner::RunResult;
use crate::scorer::{score_result, score_trajectory, ScoreResult, TrajectoryScore};
use crate::scenario::ScenarioExpect;

struct ResultEntry {
    run: RunResult,
    score: ScoreResult,
    #[allow(dead_code)]
    expect: ScenarioExpect,
    is_multi_turn: bool,
    turn_scores: Vec<ScoreResult>,
    trajectory_score: Option<TrajectoryScore>,
}

pub fn generate_report(results_dir: &Path, scenarios_dir: &Path, problem_set_dir: &Path) -> Result<String> {
    // Load all result JSON files
    let mut entries: Vec<ResultEntry> = Vec::new();

    let scenarios = crate::scenario::load_scenarios(scenarios_dir, problem_set_dir)
        .context("loading scenarios for report")?;
    let expect_map: BTreeMap<String, ScenarioExpect> = scenarios
        .iter()
        .filter_map(|s| s.expect.as_ref().map(|e| (s.file_name.clone(), e.clone())))
        .collect();

    // Map from scenario file_name to turn expects
    let turn_expect_map: BTreeMap<String, Vec<ScenarioExpect>> = scenarios
        .iter()
        .filter(|s| s.is_multi_turn())
        .map(|s| {
            (
                s.file_name.clone(),
                s.turns.iter().map(|t| t.expect.clone()).collect(),
            )
        })
        .collect();

    let is_mt_map: BTreeMap<String, bool> = scenarios
        .iter()
        .map(|s| (s.file_name.clone(), s.is_multi_turn()))
        .collect();

    let mut result_files: Vec<_> = std::fs::read_dir(results_dir)
        .with_context(|| format!("reading results dir: {}", results_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    result_files.sort_by_key(|e| e.file_name());

    for entry in result_files {
        let content = std::fs::read_to_string(entry.path())?;
        let run: RunResult = serde_json::from_str(&content)
            .with_context(|| format!("parsing result: {}", entry.path().display()))?;

        let is_mt = is_mt_map.get(&run.scenario_file).copied().unwrap_or(false);

        if is_mt {
            // Multi-turn: score each turn + trajectory
            let turn_expects = turn_expect_map.get(&run.scenario_file).cloned().unwrap_or_default();
            let turn_results = run.turns.as_ref().map(|t| t.as_slice()).unwrap_or(&[]);
            let mut turn_scores = Vec::new();
            let mut total_score: u32 = 0;
            let mut total_max: u32 = 0;

            for (idx, tr) in turn_results.iter().enumerate() {
                let expect = turn_expects.get(idx).cloned().unwrap_or_else(default_expect);
                let fake_run = RunResult {
                    scenario_name: run.scenario_name.clone(),
                    scenario_file: run.scenario_file.clone(),
                    model: run.model.clone(),
                    system_prompt: tr.system_prompt.clone(),
                    user_message: tr.user_message.clone(),
                    raw_response: tr.raw_response.clone(),
                    parsed: tr.parsed.clone(),
                    latency_ms: tr.latency_ms,
                    timestamp: String::new(),
                    turns: None,
                };
                let sc = score_result(&fake_run, &expect);
                total_score += sc.score;
                total_max += sc.max_score;
                turn_scores.push(sc);
            }

            let traj = score_trajectory(turn_results);
            total_score += traj.score;
            total_max += traj.max_score;

            let combined_score = ScoreResult {
                score: total_score,
                max_score: total_max,
                issues: turn_scores
                    .iter()
                    .enumerate()
                    .flat_map(|(i, sc)| {
                        sc.issues.iter().map(move |iss| crate::scorer::Issue {
                            check: format!("turn{}_{}", i + 1, iss.check),
                            message: iss.message.clone(),
                            points_lost: iss.points_lost,
                        })
                    })
                    .chain(traj.issues.iter().cloned())
                    .collect(),
            };

            entries.push(ResultEntry {
                run,
                score: combined_score,
                expect: default_expect(),
                is_multi_turn: true,
                turn_scores,
                trajectory_score: Some(traj),
            });
        } else {
            // Single-turn
            let expect = expect_map
                .get(&run.scenario_file)
                .cloned()
                .unwrap_or_else(default_expect);
            let score = score_result(&run, &expect);
            entries.push(ResultEntry {
                run,
                score,
                expect,
                is_multi_turn: false,
                turn_scores: Vec::new(),
                trajectory_score: None,
            });
        }
    }

    if entries.is_empty() {
        return Ok("# Coach Evaluation Report\n\nNo results found.\n".to_string());
    }

    let mut out = String::new();
    out.push_str("# Coach Evaluation Report\n\n");
    out.push_str(&format!(
        "Generated: {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    ));

    // Group by model
    let mut by_model: BTreeMap<String, Vec<&ResultEntry>> = BTreeMap::new();
    for e in &entries {
        by_model.entry(e.run.model.clone()).or_default().push(e);
    }

    // Summary table
    out.push_str("## Summary\n\n");
    out.push_str("| Scenario | ");
    let models: Vec<&String> = by_model.keys().collect();
    for m in &models {
        out.push_str(&format!("{} | ", m));
    }
    out.push('\n');
    out.push_str("|---|");
    for _ in &models {
        out.push_str("---|");
    }
    out.push('\n');

    // Collect all scenario names in order
    let mut scenario_names: Vec<String> = Vec::new();
    for e in &entries {
        if !scenario_names.contains(&e.run.scenario_file) {
            scenario_names.push(e.run.scenario_file.clone());
        }
    }

    for sname in &scenario_names {
        let is_mt = entries.iter().any(|e| &e.run.scenario_file == sname && e.is_multi_turn);
        let label = if is_mt {
            format!("{} (mt)", sname)
        } else {
            sname.clone()
        };
        out.push_str(&format!("| {} | ", label));
        for model in &models {
            let result = by_model[*model]
                .iter()
                .find(|e| &e.run.scenario_file == sname);
            if let Some(e) = result {
                out.push_str(&format!("{}/{} | ", e.score.score, e.score.max_score));
            } else {
                out.push_str("- | ");
            }
        }
        out.push('\n');
    }

    // Per-model totals
    out.push_str("\n### Totals\n\n");
    out.push_str("| Model | Score | Max | Pct | Avg Latency |\n");
    out.push_str("|---|---|---|---|---|\n");
    for (model, results) in &by_model {
        let total_score: u32 = results.iter().map(|e| e.score.score).sum();
        let total_max: u32 = results.iter().map(|e| e.score.max_score).sum();
        let pct = if total_max > 0 {
            (total_score as f64 / total_max as f64) * 100.0
        } else {
            0.0
        };
        let avg_latency: u64 = if results.is_empty() {
            0
        } else {
            results.iter().map(|e| e.run.latency_ms).sum::<u64>() / results.len() as u64
        };
        out.push_str(&format!(
            "| {} | {} | {} | {:.0}% | {}ms |\n",
            model, total_score, total_max, pct, avg_latency,
        ));
    }

    // Bar chart
    out.push_str("\n### Score Distribution\n\n");
    out.push_str("```\n");
    for (model, results) in &by_model {
        let total_score: u32 = results.iter().map(|e| e.score.score).sum();
        let total_max: u32 = results.iter().map(|e| e.score.max_score).sum();
        let bar_len = if total_max > 0 {
            (total_score as f64 / total_max as f64 * 40.0) as usize
        } else {
            0
        };
        out.push_str(&format!(
            "{:>20} |{}{} {}/{}\n",
            truncate(model, 20),
            "#".repeat(bar_len),
            " ".repeat(40 - bar_len),
            total_score,
            total_max,
        ));
    }
    out.push_str("```\n");

    // Issue frequency
    out.push_str("\n## Issue Frequency\n\n");
    let mut issue_counts: BTreeMap<String, u32> = BTreeMap::new();
    for e in &entries {
        for issue in &e.score.issues {
            *issue_counts.entry(issue.check.clone()).or_insert(0) += 1;
        }
    }
    if issue_counts.is_empty() {
        out.push_str("No issues detected.\n");
    } else {
        out.push_str("| Check | Count |\n");
        out.push_str("|---|---|\n");
        for (check, count) in &issue_counts {
            out.push_str(&format!("| {} | {} |\n", check, count));
        }
    }

    // Detailed results
    out.push_str("\n## Detailed Results\n\n");
    for e in &entries {
        if e.is_multi_turn {
            out.push_str(&format!(
                "### {} — {} ({}/{}) [multi-turn]\n\n",
                e.run.scenario_file, e.run.model, e.score.score, e.score.max_score,
            ));

            // Per-turn details
            if let Some(ref turns) = e.run.turns {
                for (idx, tr) in turns.iter().enumerate() {
                    let turn_sc = e.turn_scores.get(idx);
                    out.push_str(&format!(
                        "**Turn {}**: state={}, conf={:.2}, obs={}, {}ms",
                        idx + 1,
                        tr.parsed.state,
                        tr.parsed.confidence,
                        tr.parsed
                            .matched_observation_id
                            .as_deref()
                            .unwrap_or("none"),
                        tr.latency_ms,
                    ));
                    if let Some(sc) = turn_sc {
                        out.push_str(&format!(" — {}/{}", sc.score, sc.max_score));
                    }
                    out.push('\n');

                    // Show obs_states progression
                    let states: Vec<String> = tr
                        .obs_states_after
                        .iter()
                        .map(|(k, v)| format!("obs{}={}", k, v))
                        .collect();
                    if !states.is_empty() {
                        out.push_str(&format!("  obs: {}\n", states.join(", ")));
                    }
                }
            }

            // Trajectory score
            if let Some(ref traj) = e.trajectory_score {
                out.push_str(&format!(
                    "\n**Trajectory**: {}/{}\n",
                    traj.score, traj.max_score,
                ));
                for issue in &traj.issues {
                    out.push_str(&format!("  - `{}`: {}\n", issue.check, issue.message));
                }
            }

            if !e.score.issues.is_empty() {
                out.push_str("- **All Issues**:\n");
                for issue in &e.score.issues {
                    out.push_str(&format!(
                        "  - `{}`: {} (-{})\n",
                        issue.check, issue.message, issue.points_lost
                    ));
                }
            }
            out.push('\n');
        } else {
            out.push_str(&format!(
                "### {} — {} ({}/{})\n\n",
                e.run.scenario_file, e.run.model, e.score.score, e.score.max_score,
            ));
            out.push_str(&format!("- **State**: {}\n", e.run.parsed.state));
            out.push_str(&format!("- **Confidence**: {:.2}\n", e.run.parsed.confidence));
            out.push_str(&format!(
                "- **Matched obs**: {}\n",
                e.run.parsed
                    .matched_observation_id
                    .as_deref()
                    .unwrap_or("none"),
            ));
            out.push_str(&format!("- **Latency**: {}ms\n", e.run.latency_ms));

            if !e.score.issues.is_empty() {
                out.push_str("- **Issues**:\n");
                for issue in &e.score.issues {
                    out.push_str(&format!(
                        "  - `{}`: {} (-{})\n",
                        issue.check, issue.message, issue.points_lost
                    ));
                }
            }

            // Truncated coach message
            let msg = &e.run.parsed.coach_message;
            let display = if msg.len() > 200 {
                format!(
                    "{}...",
                    &msg[..msg
                        .char_indices()
                        .take_while(|(i, _)| *i < 200)
                        .last()
                        .map_or(0, |(i, c)| i + c.len_utf8())]
                )
            } else {
                msg.clone()
            };
            out.push_str(&format!("\n> {}\n\n", display));
        }
    }

    Ok(out)
}

fn default_expect() -> ScenarioExpect {
    ScenarioExpect {
        state: None,
        valid_states: Vec::new(),
        banned_states: Vec::new(),
        obs_id: None,
        min_confidence: None,
        max_confidence: None,
        must_be_socratic: true,
        banned_patterns: Vec::new(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
