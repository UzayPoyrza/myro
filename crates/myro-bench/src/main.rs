mod report;
mod runner;
mod scenario;
mod scorer;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use myro_coach::config::CoachConfig;
use myro_coach::llm::openai_compat::OpenAiCompatibleProvider;
use myro_coach::prompt::coaching::build_coaching_prompt;

#[derive(Parser)]
#[command(name = "myro-bench", about = "Benchmark LLM coaching quality")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run benchmark scenarios against an LLM
    Run {
        /// Directory containing scenario TOML files
        #[arg(long, default_value = "bench/scenarios")]
        scenarios: String,

        /// Directory containing problem JSON files
        #[arg(long, default_value = "test-problem-set")]
        problem_set: String,

        /// Directory containing prompt templates
        #[arg(long, default_value = "prompts")]
        prompts: String,

        /// LLM model name (overrides config)
        #[arg(long)]
        model: Option<String>,

        /// LLM base URL (overrides config)
        #[arg(long)]
        base_url: Option<String>,

        /// LLM API key (overrides config)
        #[arg(long)]
        api_key: Option<String>,

        /// Output directory for result JSON files
        #[arg(long, default_value = "bench/results")]
        output: String,

        /// Print prompts without calling LLM
        #[arg(long)]
        dry_run: bool,

        /// Run only scenarios matching this prefix
        #[arg(long)]
        filter: Option<String>,
    },
    /// Generate a markdown report from result files
    Report {
        /// Directory containing result JSON files
        #[arg(long, default_value = "bench/results")]
        results: String,

        /// Directory containing scenario TOML files (for expect data)
        #[arg(long, default_value = "bench/scenarios")]
        scenarios: String,

        /// Directory containing problem JSON files
        #[arg(long, default_value = "test-problem-set")]
        problem_set: String,

        /// Output markdown file
        #[arg(long, default_value = "docs/coach-eval-report.md")]
        output: String,
    },
}

fn find_project_root() -> PathBuf {
    // Walk up from CWD or CARGO_MANIFEST_DIR to find the workspace root
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest);
        if let Some(parent) = p.parent().and_then(|p| p.parent()) {
            if parent.join("Cargo.toml").exists() {
                return parent.to_path_buf();
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = find_project_root();

    match cli.command {
        Command::Run {
            scenarios,
            problem_set,
            prompts,
            model,
            base_url,
            api_key,
            output,
            dry_run,
            filter,
        } => {
            let scenarios_dir = resolve_path(&root, &scenarios);
            let problem_set_dir = resolve_path(&root, &problem_set);
            let prompts_dir = resolve_path(&root, &prompts);
            let output_dir = resolve_path(&root, &output);

            std::fs::create_dir_all(&output_dir)
                .with_context(|| format!("creating output dir: {}", output_dir.display()))?;

            let mut loaded = scenario::load_scenarios(&scenarios_dir, &problem_set_dir)?;
            if let Some(ref prefix) = filter {
                loaded.retain(|s| s.file_name.starts_with(prefix));
            }

            eprintln!("Loaded {} scenarios", loaded.len());

            if dry_run {
                for s in &loaded {
                    if s.is_multi_turn() {
                        // Multi-turn dry-run: print prompts for each turn
                        println!("=== {} [multi-turn, {} turns] ===", s.name, s.turns.len());
                        let obs_states = s.def.obs_states.clone();
                        let mut conversation_history: Vec<(String, String)> = s
                            .def
                            .recent_messages
                            .iter()
                            .map(|m| (m.role.clone(), m.content.clone()))
                            .collect();

                        for (idx, turn) in s.turns.iter().enumerate() {
                            if let Some(ref msg) = turn.user_message {
                                conversation_history
                                    .push(("user".to_string(), msg.clone()));
                            }

                            let ctx = scenario::build_context_dynamic(
                                s,
                                &obs_states,
                                &conversation_history,
                                &turn.trigger,
                                turn.elapsed_secs,
                                &turn.code.text,
                            );
                            let (system, user) =
                                build_coaching_prompt(&ctx, Some(&prompts_dir));

                            println!("--- TURN {} ---", idx + 1);
                            println!("--- SYSTEM PROMPT ---");
                            println!("{}", system);
                            println!("--- USER MESSAGE ---");
                            println!("{}", user);
                            println!("[LLM RESPONSE PLACEHOLDER]");

                            // Simulate advancing obs_states for dry-run display
                            // (can't know actual response, just show prompt progression)
                            conversation_history
                                .push(("assistant".to_string(), "[placeholder]".to_string()));
                            // Keep rolling 20
                            if conversation_history.len() > 20 {
                                let excess = conversation_history.len() - 20;
                                conversation_history.drain(..excess);
                            }
                            let _ = &obs_states; // obs_states unchanged in dry-run
                        }
                        println!();
                    } else {
                        let (system, user) =
                            runner::build_prompts(s, Some(&prompts_dir));
                        println!("=== {} ===", s.name);
                        println!("--- SYSTEM PROMPT ---");
                        println!("{}", system);
                        println!("--- USER MESSAGE ---");
                        println!("{}", user);
                        println!();
                    }
                }
                return Ok(());
            }

            // Build provider
            let config = CoachConfig::load().unwrap_or_default();
            let effective_base_url = base_url
                .or(config.base_url.clone())
                .context("no LLM base_url: set --base-url or configure ~/.config/myro/config.toml")?;
            let effective_model = model
                .or(config.model.clone())
                .unwrap_or_else(|| "default".to_string());
            let effective_api_key = api_key.or(config.api_key.clone());

            let provider =
                OpenAiCompatibleProvider::new(effective_base_url, effective_api_key, effective_model.clone());

            eprintln!("Model: {}", effective_model);
            eprintln!();

            let total = loaded.len();
            for (i, s) in loaded.iter().enumerate() {
                if s.is_multi_turn() {
                    eprint!("[{}/{}] {} [multi-turn, {} turns] ... ", i + 1, total, s.name, s.turns.len());

                    match runner::run_multi_turn_scenario(&provider, s, &effective_model, Some(&prompts_dir)).await {
                        Ok(result) => {
                            // Score each turn
                            let turn_results = result.turns.as_ref().unwrap();
                            let mut total_score: u32 = 0;
                            let mut total_max: u32 = 0;
                            for (idx, tr) in turn_results.iter().enumerate() {
                                let expect = &s.turns[idx].expect;
                                // Build a RunResult-like for scoring
                                let fake_run = runner::RunResult {
                                    scenario_name: s.name.clone(),
                                    scenario_file: s.file_name.clone(),
                                    model: effective_model.clone(),
                                    system_prompt: tr.system_prompt.clone(),
                                    user_message: tr.user_message.clone(),
                                    raw_response: tr.raw_response.clone(),
                                    parsed: tr.parsed.clone(),
                                    latency_ms: tr.latency_ms,
                                    timestamp: String::new(),
                                    turns: None,
                                };
                                let sc = scorer::score_result(&fake_run, expect);
                                total_score += sc.score;
                                total_max += sc.max_score;
                                if !sc.issues.is_empty() {
                                    for issue in &sc.issues {
                                        eprintln!("  turn {}: ! {}: {}", idx + 1, issue.check, issue.message);
                                    }
                                }
                            }

                            // Trajectory score
                            let traj = scorer::score_trajectory(turn_results);
                            total_score += traj.score;
                            total_max += traj.max_score;

                            eprintln!(
                                "{}/{} ({}ms total)",
                                total_score, total_max, result.latency_ms,
                            );
                            for issue in &traj.issues {
                                eprintln!("  trajectory: ! {}: {}", issue.check, issue.message);
                            }

                            // Write result JSON
                            let filename = format!("{}_{}.json", s.file_name, sanitize_model(&effective_model));
                            let path = output_dir.join(filename);
                            let json = serde_json::to_string_pretty(&result)?;
                            std::fs::write(&path, json)?;
                        }
                        Err(e) => {
                            eprintln!("ERROR: {:#}", e);
                        }
                    }
                } else {
                    eprint!("[{}/{}] {} ... ", i + 1, total, s.name);

                    let expect = s.expect.as_ref().expect("single-turn scenario missing [expect]");
                    match runner::run_scenario(&provider, s, &effective_model, Some(&prompts_dir)).await {
                        Ok(result) => {
                            let score = scorer::score_result(&result, expect);
                            eprintln!(
                                "{}/{} ({:.0}ms) state={} conf={:.2}",
                                score.score,
                                score.max_score,
                                result.latency_ms,
                                result.parsed.state,
                                result.parsed.confidence,
                            );
                            for issue in &score.issues {
                                eprintln!("  ! {}: {}", issue.check, issue.message);
                            }

                            // Write result JSON
                            let filename = format!("{}_{}.json", s.file_name, sanitize_model(&effective_model));
                            let path = output_dir.join(filename);
                            let json = serde_json::to_string_pretty(&result)?;
                            std::fs::write(&path, json)?;
                        }
                        Err(e) => {
                            eprintln!("ERROR: {:#}", e);
                        }
                    }
                }
            }

            eprintln!("\nResults written to {}", output_dir.display());
        }
        Command::Report {
            results,
            scenarios,
            problem_set,
            output,
        } => {
            let results_dir = resolve_path(&root, &results);
            let scenarios_dir = resolve_path(&root, &scenarios);
            let problem_set_dir = resolve_path(&root, &problem_set);
            let output_path = resolve_path(&root, &output);

            let report = report::generate_report(&results_dir, &scenarios_dir, &problem_set_dir)?;

            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&output_path, &report)?;
            eprintln!("Report written to {}", output_path.display());
        }
    }

    Ok(())
}

fn resolve_path(root: &Path, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn sanitize_model(model: &str) -> String {
    model
        .replace(['/', ':', ' '], "_")
}
