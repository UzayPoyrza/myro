use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use myro_cf::CfClient;
use crate::db::{contest_data, schema};

pub async fn run_collect(
    db_path: &Path,
    max_contests: Option<usize>,
    since_contest: Option<i64>,
    retry_failed: bool,
) -> Result<()> {
    let conn = schema::open_db(db_path)?;
    let client = CfClient::new();

    println!("Fetching contest list...");
    let all_contests = client
        .fetch_contest_list()
        .await
        .context("Failed to fetch contest list")?;

    // Filter to finished CF/ICPC contests
    let mut contests: Vec<_> = all_contests
        .into_iter()
        .filter(|c| c.phase == "FINISHED")
        .filter(|c| c.contest_type == "CF" || c.contest_type == "ICPC")
        .filter(|c| {
            if let Some(since) = since_contest {
                c.id >= since
            } else {
                true
            }
        })
        .collect();

    // Sort by ID ascending (oldest first)
    contests.sort_by_key(|c| c.id);

    // Filter out already-fetched contests
    let mut to_fetch = Vec::new();
    for c in &contests {
        let already_ok = contest_data::contest_is_fetched(&conn, c.id)?;
        if already_ok {
            continue;
        }
        let previously_failed = contest_data::contest_fetch_failed(&conn, c.id)?;
        if previously_failed && !retry_failed {
            continue;
        }
        to_fetch.push(c);
    }

    if let Some(max) = max_contests {
        to_fetch.truncate(max);
    }

    println!(
        "Found {} total finished contests, {} to fetch",
        contests.len(),
        to_fetch.len()
    );

    if to_fetch.is_empty() {
        println!("Nothing to fetch. Database is up to date.");
        return Ok(());
    }

    let progress = ProgressBar::new(to_fetch.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos}/{len} contests | {msg}")
            .unwrap(),
    );

    let mut fetched = 0usize;
    let mut failed = 0usize;

    for contest in &to_fetch {
        progress.set_message(format!("#{} {}", contest.id, &contest.name[..contest.name.len().min(30)]));

        match client.fetch_contest_standings(contest.id).await {
            Ok(standings) => {
                let num_rows = standings.rows.len();
                let num_problems = standings.problems.len();

                if let Err(e) = contest_data::insert_contest(&conn, &standings) {
                    eprintln!(
                        "\nFailed to insert contest {}: {}",
                        contest.id, e
                    );
                    contest_data::mark_contest_failed(&conn, contest, &e.to_string())?;
                    failed += 1;
                } else {
                    // Fetch rating changes and backfill user_rating
                    match client.fetch_rating_changes(contest.id).await {
                        Ok(changes) => {
                            if let Err(e) = contest_data::backfill_user_ratings(&conn, contest.id, &changes) {
                                eprintln!("\nFailed to backfill ratings for contest {}: {}", contest.id, e);
                            }
                        }
                        Err(_) => {
                            // Some contests (e.g. unrated) don't have rating changes — that's OK
                        }
                    }

                    fetched += 1;
                    if num_rows > 0 {
                        progress.set_message(format!(
                            "#{} — {} participants, {} problems",
                            contest.id, num_rows, num_problems
                        ));
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "\nFailed to fetch standings for contest {} ({}): {}",
                    contest.id, contest.name, e
                );
                contest_data::mark_contest_failed(&conn, contest, &e.to_string())?;
                failed += 1;
            }
        }

        progress.inc(1);
    }

    progress.finish_with_message("done");

    println!();
    println!(
        "Collection complete: {} fetched, {} failed, {} already in DB",
        fetched,
        failed,
        contests.len() - to_fetch.len()
    );

    // Print DB stats
    let total_contests: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cf_contests WHERE fetch_status = 'ok'",
        [],
        |row| row.get(0),
    )?;
    let total_results: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cf_contest_results",
        [],
        |row| row.get(0),
    )?;
    let total_problems: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cf_contest_problems",
        [],
        |row| row.get(0),
    )?;

    println!(
        "Database: {} contests, {} problems, {} result rows",
        total_contests, total_problems, total_results
    );

    Ok(())
}
