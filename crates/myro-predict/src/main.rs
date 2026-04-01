mod collect;

use anyhow::Result;
use myro_predict::{db, model};
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "myro-predict")]
#[command(about = "CF solve probability prediction using logistic matrix factorization")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch Codeforces contest standings into the local database
    Collect {
        /// SQLite database path
        #[arg(long, default_value = "predict.db")]
        db_path: PathBuf,

        /// Stop after N contests (for testing)
        #[arg(long)]
        max_contests: Option<usize>,

        /// Only fetch contests with ID >= this value
        #[arg(long)]
        since_contest: Option<i64>,

        /// Re-attempt previously failed contest fetches
        #[arg(long, default_value_t = false)]
        retry_failed: bool,
    },

    /// Train the logistic matrix factorization model
    Train {
        /// SQLite database path
        #[arg(long, default_value = "predict.db")]
        db_path: PathBuf,

        /// Temporal cutoff date (YYYY-MM-DD). Contests before this date are training data.
        #[arg(long)]
        cutoff: String,

        /// Number of latent dimensions
        #[arg(long, default_value_t = 30)]
        latent_dim: usize,

        /// Number of training epochs
        #[arg(long, default_value_t = 50)]
        epochs: usize,

        /// Learning rate
        #[arg(long, default_value_t = 0.01)]
        lr: f64,

        /// L2 regularization strength
        #[arg(long, default_value_t = 0.01)]
        lambda: f64,

        /// Enable tag-informed initialization
        #[arg(long, default_value_t = true)]
        tag_init: bool,

        /// Negative sample ratio (0.0 = no downsampling)
        #[arg(long, default_value_t = 0.0)]
        neg_ratio: f64,

        /// Minimum contests per user to include
        #[arg(long, default_value_t = 10)]
        min_contests: usize,

        /// Output model path
        #[arg(long, default_value = "model.bin.gz")]
        output: PathBuf,

        /// Verbose training output
        #[arg(long, default_value_t = false)]
        verbose: bool,

        /// Comma-separated user handles to exclude from training data
        #[arg(long)]
        exclude_users: Option<String>,
    },

    /// Evaluate model via temporal walk-forward: fit user embedding from prior
    /// history, predict current contest outcomes
    Eval {
        /// SQLite database path
        #[arg(long, default_value = "predict.db")]
        db_path: PathBuf,

        /// Minimum contests per user to include in dataset
        #[arg(long, default_value_t = 10)]
        min_contests: usize,

        /// Minimum prior contests before evaluating a user
        #[arg(long, default_value_t = 5)]
        min_history: usize,

        /// Problem model path
        #[arg(long, default_value = "problem_model.bin.gz")]
        model_path: PathBuf,

        /// Optional date ceiling (YYYY-MM-DD)
        #[arg(long)]
        cutoff: Option<String>,

        /// Verbose output
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Backfill user ratings from contest.ratingChanges for existing contests
    BackfillRatings {
        /// SQLite database path
        #[arg(long, default_value = "predict.db")]
        db_path: PathBuf,
    },

    /// Export model parameters to CSV for external analysis
    ExportAnalysis {
        /// Trained model path
        #[arg(long, default_value = "model.bin.gz")]
        model_path: PathBuf,

        /// Output directory for CSV files
        #[arg(long, default_value = ".")]
        output_dir: PathBuf,
    },

    /// Export problem-only model from a full trained model
    ExportModel {
        /// Full trained model path
        #[arg(long, default_value = "model.bin.gz")]
        model_path: PathBuf,

        /// Output problem model path
        #[arg(long, default_value = "problem_model.bin.gz")]
        output: PathBuf,
    },

    /// Query solve probabilities for a specific user
    Query {
        /// Codeforces handle
        #[arg(long)]
        handle: String,

        /// Problem model path (use export-model to create from full model)
        #[arg(long, default_value = "problem_model.bin.gz")]
        model_path: PathBuf,

        /// Comma-separated problem IDs (e.g., "1800A,1801B")
        #[arg(long)]
        problems: Option<String>,

        /// Show top-N problems ranked by predicted difficulty
        #[arg(long)]
        top_n: Option<usize>,
    },

    /// Show per-tag skill ratings for a user
    TagSkills {
        /// Codeforces handle
        #[arg(long)]
        handle: String,

        /// Problem model path
        #[arg(long, default_value = "problem_model.bin.gz")]
        model_path: PathBuf,

        /// Export results to CSV file
        #[arg(long)]
        csv: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Collect {
            db_path,
            max_contests,
            since_contest,
            retry_failed,
        } => {
            collect::run_collect(&db_path, max_contests, since_contest, retry_failed).await?;
        }
        Commands::Train {
            db_path,
            cutoff,
            latent_dim,
            epochs,
            lr,
            lambda,
            tag_init,
            neg_ratio,
            min_contests,
            output,
            verbose,
            exclude_users,
        } => {
            let cutoff_ts = chrono::NaiveDate::parse_from_str(&cutoff, "%Y-%m-%d")
                .map_err(|e| anyhow::anyhow!("Invalid cutoff date '{}': {}", cutoff, e))?
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp();

            let config = model::types::ModelConfig {
                latent_dim,
                epochs,
                learning_rate: lr,
                lambda,
                tag_init,
                negative_sample_ratio: neg_ratio,
                min_contests,
                cutoff_timestamp: cutoff_ts,
                verbose,
            };

            let conn = db::schema::open_db(&db_path)?;
            let excluded: Vec<String> = exclude_users
                .map(|s| s.split(',').map(|h| h.trim().to_string()).collect())
                .unwrap_or_default();
            if !excluded.is_empty() {
                println!("Excluding {} users from training: {:?}", excluded.len(), excluded);
            }
            let dataset = db::contest_data::load_observations_filtered(
                &conn, cutoff_ts, min_contests, true, &excluded,
            )?;

            println!(
                "Training set: {} users, {} problems, {} observations",
                dataset.num_users, dataset.num_problems, dataset.observations.len()
            );

            let (trained_model, curve) = model::train::train_with_curve(&dataset, &config)?;

            db::model_store::save_model(&trained_model, &output)?;
            println!("Model saved to {}", output.display());

            // Save training curve
            let curve_path = output.with_file_name("analysis_training_curve.csv");
            {
                let mut f = std::fs::File::create(&curve_path)?;
                writeln!(f, "epoch,loss,train_auc")?;
                for m in &curve {
                    writeln!(f, "{},{:.6},{:.6}", m.epoch, m.loss, m.train_auc)?;
                }
            }
            println!("Training curve saved to {}", curve_path.display());
        }
        Commands::Eval {
            db_path,
            min_contests,
            min_history,
            model_path,
            cutoff,
            verbose,
        } => {
            let cutoff_ts = match cutoff {
                Some(ref date_str) => chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .map_err(|e| anyhow::anyhow!("Invalid cutoff date '{}': {}", date_str, e))?
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp(),
                None => i64::MAX,
            };

            let conn = db::schema::open_db(&db_path)?;
            let dataset =
                db::contest_data::load_observations(&conn, cutoff_ts, min_contests, true)?;

            println!(
                "Temporal eval: {} users, {} problems, {} observations",
                dataset.num_users, dataset.num_problems, dataset.observations.len()
            );

            let problem_model = db::model_store::load_problem_model(&model_path)?;
            println!(
                "Loaded problem model: {} problems, k={}",
                problem_model.problem_params.len(),
                problem_model.latent_dim
            );

            model::eval::run_temporal_eval(&problem_model, &dataset, min_history, verbose)?;
        }
        Commands::BackfillRatings { db_path } => {
            let conn = db::schema::open_db(&db_path)?;
            let client = myro_cf::CfClient::new();

            let missing = db::contest_data::contests_missing_ratings(&conn)?;
            println!(
                "Found {} contests missing user ratings, fetching ratingChanges...",
                missing.len()
            );

            let progress = indicatif::ProgressBar::new(missing.len() as u64);
            progress.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40} {pos}/{len} | {msg}")
                    .unwrap(),
            );

            let mut filled = 0usize;
            let mut unrated = 0usize;

            for contest_id in &missing {
                progress.set_message(format!("contest #{}", contest_id));
                match client.fetch_rating_changes(*contest_id).await {
                    Ok(changes) => {
                        if changes.is_empty() {
                            unrated += 1;
                        } else {
                            db::contest_data::backfill_user_ratings(&conn, *contest_id, &changes)?;
                            filled += 1;
                        }
                    }
                    Err(_) => {
                        // Unrated contests return an error — that's fine
                        unrated += 1;
                    }
                }
                progress.inc(1);
            }

            progress.finish_with_message("done");
            println!(
                "\nBackfill complete: {} contests got ratings, {} were unrated",
                filled, unrated
            );

            // Stats
            let total: i64 = conn.query_row(
                "SELECT COUNT(*) FROM cf_contest_results",
                [],
                |row| row.get(0),
            )?;
            let with_rating: i64 = conn.query_row(
                "SELECT COUNT(*) FROM cf_contest_results WHERE user_rating IS NOT NULL",
                [],
                |row| row.get(0),
            )?;
            println!(
                "User rating coverage: {}/{} rows ({:.1}%)",
                with_rating,
                total,
                with_rating as f64 / total as f64 * 100.0
            );
        }
        Commands::ExportAnalysis {
            model_path,
            output_dir,
        } => {
            let model = db::model_store::load_model(&model_path)?;
            let k = model.config.latent_dim;

            // Build reverse index: idx -> key
            let mut problem_keys: Vec<(&str, usize)> =
                model.problem_index.iter().map(|(k, &v)| (k.as_str(), v)).collect();
            problem_keys.sort_by_key(|&(_, idx)| idx);

            let mut user_keys: Vec<(&str, usize)> =
                model.user_index.iter().map(|(k, &v)| (k.as_str(), v)).collect();
            user_keys.sort_by_key(|&(_, idx)| idx);

            // --- Write analysis_problem_params.csv ---
            let problem_path = output_dir.join("analysis_problem_params.csv");
            {
                let mut f = std::fs::File::create(&problem_path)?;

                // Header
                write!(f, "problem_key,rating,tags,difficulty")?;
                for d in 0..k {
                    write!(f, ",alpha_{}", d)?;
                }
                writeln!(f)?;

                // Rows
                for &(key, idx) in &problem_keys {
                    let params = &model.problem_params[idx];
                    let rating = model
                        .problem_ratings
                        .get(idx)
                        .and_then(|r| *r)
                        .map(|r| r.to_string())
                        .unwrap_or_default();
                    let tags = model
                        .problem_tags
                        .get(idx)
                        .map(|t| t.join(";"))
                        .unwrap_or_default();

                    // Escape key and tags for CSV (wrap in quotes if they contain commas/quotes)
                    write!(f, "{},{},\"{}\",{:.6}", key, rating, tags, params.difficulty)?;
                    for d in 0..k {
                        write!(f, ",{:.6}", params.alpha[d])?;
                    }
                    writeln!(f)?;
                }
            }
            println!(
                "Wrote {} problem rows to {}",
                problem_keys.len(),
                problem_path.display()
            );

            // --- Write analysis_user_params.csv ---
            let user_path = output_dir.join("analysis_user_params.csv");
            {
                let mut f = std::fs::File::create(&user_path)?;

                // Header
                write!(f, "handle,bias")?;
                for d in 0..k {
                    write!(f, ",theta_{}", d)?;
                }
                writeln!(f)?;

                // Rows
                for &(handle, idx) in &user_keys {
                    let params = &model.user_params[idx];
                    write!(f, "{},{:.6}", handle, params.bias)?;
                    for d in 0..k {
                        write!(f, ",{:.6}", params.theta[d])?;
                    }
                    writeln!(f)?;
                }
            }
            println!(
                "Wrote {} user rows to {}",
                user_keys.len(),
                user_path.display()
            );

            // --- Write analysis_tag_dim_map.csv (bonus: tag-to-dimension mapping) ---
            if !model.tag_dim_map.is_empty() {
                let tag_map_path = output_dir.join("analysis_tag_dim_map.csv");
                let mut f = std::fs::File::create(&tag_map_path)?;
                writeln!(f, "tag,dimension")?;
                let mut tag_dims: Vec<(&str, usize)> =
                    model.tag_dim_map.iter().map(|(k, &v)| (k.as_str(), v)).collect();
                tag_dims.sort_by_key(|&(_, d)| d);
                for (tag, dim) in &tag_dims {
                    writeln!(f, "{},{}", tag, dim)?;
                }
                println!(
                    "Wrote {} tag mappings to {}",
                    tag_dims.len(),
                    tag_map_path.display()
                );
            }

            println!(
                "\nModel has {} latent dimensions, {} users, {} problems",
                k,
                model.user_params.len(),
                model.problem_params.len()
            );
        }
        Commands::ExportModel {
            model_path,
            output,
        } => {
            let full_model = db::model_store::load_model(&model_path)?;
            let problem_model: model::types::ProblemModel = full_model.into();
            db::model_store::save_problem_model(&problem_model, &output)?;
            println!(
                "Exported problem model ({} problems, k={}) to {}",
                problem_model.problem_params.len(),
                problem_model.latent_dim,
                output.display()
            );
        }
        Commands::Query {
            handle,
            model_path,
            problems,
            top_n,
        } => {
            model::inference::run_query(&handle, &model_path, problems, top_n).await?;
        }
        Commands::TagSkills {
            handle,
            model_path,
            csv,
        } => {
            run_tag_skills(&handle, &model_path, csv.as_deref()).await?;
        }
    }

    Ok(())
}

async fn run_tag_skills(handle: &str, model_path: &std::path::Path, csv: Option<&std::path::Path>) -> Result<()> {
    use model::inference::{
        build_observations_from_submissions, fit_user_weighted, DEFAULT_HALF_LIFE_DAYS,
    };
    use model::skills::compute_skill_profile;

    let problem_model = db::model_store::load_problem_model(model_path)?;
    let client = myro_cf::CfClient::new();

    println!("Fetching submissions for {}...", handle);
    let submissions = client
        .fetch_user_status(handle)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch submissions: {}", e))?;

    let now_ts = chrono::Utc::now().timestamp();
    let (obs, _) =
        build_observations_from_submissions(&problem_model, &submissions, now_ts, DEFAULT_HALF_LIFE_DAYS);

    if obs.is_empty() {
        anyhow::bail!("No overlap between user's submissions and model problems");
    }

    println!("Fitting user embedding from {} observations...", obs.len());
    let user_params = fit_user_weighted(&problem_model, &obs, 0.01, 100, 0.01);

    let profile = compute_skill_profile(&user_params, &problem_model);

    println!();
    println!(
        "{:<30} {:>8} {:>10} {:>6}",
        "Tag", "Rating", "P(solve)", "N"
    );
    println!("{}", "-".repeat(58));

    for tag_rating in &profile.tag_ratings {
        println!(
            "{:<30} {:>8} {:>9.1}% {:>6}",
            tag_rating.tag,
            tag_rating.effective_rating,
            tag_rating.avg_p_solve * 100.0,
            tag_rating.num_problems,
        );
    }

    println!("{}", "-".repeat(58));
    println!(
        "{:<30} {:>8}",
        "OVERALL", profile.overall_rating,
    );

    if let Some(csv_path) = csv {
        let mut f = std::fs::File::create(csv_path)?;
        writeln!(f, "tag,effective_rating,strength,avg_p_solve,num_problems")?;
        for tr in &profile.tag_ratings {
            writeln!(
                f,
                "{},{},{:.4},{:.4},{}",
                tr.tag, tr.effective_rating, tr.strength, tr.avg_p_solve, tr.num_problems,
            )?;
        }
        println!("\nExported to {}", csv_path.display());
    }

    Ok(())
}
