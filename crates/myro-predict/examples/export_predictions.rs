/// Export predictions for a user as CSV with Elo comparison.
/// Usage: cargo run --release --example export_predictions -- <handle> <model_path> <output_csv>
///
/// Also computes Elo-based P(solve) = σ((user_rating - problem_rating) / 400)
/// and a divergence column showing where MF and Elo disagree most.
use anyhow::{Context, Result};
use myro_predict::db::model_store;
use myro_predict::model::inference::{
    build_observations_from_submissions, fit_user_weighted, predict_all, DEFAULT_HALF_LIFE_DAYS,
};

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <handle> <model_path> <output_csv>",
            args[0]
        );
        std::process::exit(1);
    }
    let handle = &args[1];
    let model_path = std::path::Path::new(&args[2]);
    let output_path = &args[3];

    let model = model_store::load_problem_model(model_path)?;
    let client = myro_cf::CfClient::new();

    // Fetch user's current rating for Elo baseline
    eprintln!("Fetching user info for {}...", handle);
    let user_info = client
        .fetch_user_info(handle)
        .await
        .context("Failed to fetch user info")?;
    let user_rating = user_info.rating.unwrap_or(1500) as f64;
    eprintln!("User rating: {}", user_rating as i32);

    eprintln!("Fetching submissions for {}...", handle);
    let submissions = client
        .fetch_user_status(handle)
        .await
        .context("Failed to fetch user submissions")?;

    let now_ts = chrono::Utc::now().timestamp();
    let (obs, solved_keys) =
        build_observations_from_submissions(&model, &submissions, now_ts, DEFAULT_HALF_LIFE_DAYS);

    let submitted = obs.len();
    let solved = obs.iter().filter(|o| o.solved).count();
    eprintln!(
        "Found {} submitted problems in model ({} solved)",
        submitted, solved
    );

    eprintln!("Fitting user parameters...");
    let user_params = fit_user_weighted(&model, &obs, 0.01, 100, 0.01);
    let all_preds = predict_all(&user_params, &model);

    // Build reverse index: idx -> key
    let mut idx_to_key: Vec<(&str, usize)> = model
        .problem_index
        .iter()
        .map(|(k, &v)| (k.as_str(), v))
        .collect();
    idx_to_key.sort_by_key(|&(_, idx)| idx);

    // Write CSV
    let mut wtr = std::fs::File::create(output_path)?;
    use std::io::Write;
    writeln!(
        wtr,
        "problem_key,problem_name,problem_link,rating,tags,p_solve_mf,p_solve_elo,divergence,solved"
    )?;

    for &(key, idx) in &idx_to_key {
        let p_mf = all_preds[idx];
        let rating_opt = model.problem_ratings.get(idx).and_then(|r| *r);
        let rating_str = rating_opt
            .map(|r| r.to_string())
            .unwrap_or_default();
        let tags = model
            .problem_tags
            .get(idx)
            .map(|t| t.join("; "))
            .unwrap_or_default();

        // Elo prediction: σ((user_rating - problem_rating) / 400)
        let p_elo = rating_opt
            .map(|pr| sigmoid((user_rating - pr as f64) / 400.0))
            .unwrap_or(0.5);

        // Divergence: MF - Elo (positive = MF thinks easier than Elo suggests)
        let divergence = p_mf - p_elo;

        // Whether user has solved this problem
        let solved = solved_keys.get(key).copied().unwrap_or(false);

        // Parse key "contestId:index" to build name and link
        let parts: Vec<&str> = key.split(':').collect();
        let (contest_id, prob_idx) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            (key, "")
        };
        let name = format!("{}{}", contest_id, prob_idx);
        let link = format!(
            "https://codeforces.com/contest/{}/problem/{}",
            contest_id, prob_idx
        );

        writeln!(
            wtr,
            "{},\"{}\",{},{},\"{}\",{:.4},{:.4},{:.4},{}",
            key, name, link, rating_str, tags, p_mf, p_elo, divergence, solved
        )?;
    }

    eprintln!("Wrote {} problems to {}", idx_to_key.len(), output_path);
    Ok(())
}
