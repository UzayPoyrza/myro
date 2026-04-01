use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;

use myro_cf::{CfContest, CfRatingChange, CfStandingsResult};
use crate::model::types::{Observation, TrainingDataset};

/// Insert a full contest's standings into the database.
pub fn insert_contest(conn: &Connection, standings: &CfStandingsResult) -> Result<()> {
    let contest = &standings.contest;
    let now = chrono::Utc::now().timestamp();

    let tx = conn.unchecked_transaction()?;

    tx.execute(
        "INSERT OR REPLACE INTO cf_contests (contest_id, name, contest_type, start_time, duration, fetched_at, fetch_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'ok')",
        params![
            contest.id,
            contest.name,
            contest.contest_type,
            contest.start_time_seconds,
            contest.duration_seconds,
            now,
        ],
    )?;

    for problem in &standings.problems {
        let tags_json = serde_json::to_string(&problem.tags)?;
        tx.execute(
            "INSERT OR REPLACE INTO cf_contest_problems (contest_id, problem_idx, name, rating, tags)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                contest.id,
                problem.index,
                problem.name,
                problem.rating,
                tags_json,
            ],
        )?;
    }

    for row in &standings.rows {
        // Skip teams (multi-member parties)
        if row.party.members.len() != 1 {
            continue;
        }
        // Only include CONTESTANT (not virtual/practice/out-of-competition)
        let ptype = row.party.participant_type.as_deref().unwrap_or("");
        if ptype != "CONTESTANT" {
            continue;
        }

        let handle = &row.party.members[0].handle;
        let user_rating = row.party.members[0].rating;

        for (i, pr) in row.problem_results.iter().enumerate() {
            if i >= standings.problems.len() {
                break;
            }
            let problem_idx = &standings.problems[i].index;
            let solved = if pr.points > 0.0 { 1 } else { 0 };

            tx.execute(
                "INSERT OR REPLACE INTO cf_contest_results
                 (contest_id, handle, problem_idx, solved, rejected_count, solve_time, user_rating)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    contest.id,
                    handle,
                    problem_idx,
                    solved,
                    pr.rejected_attempt_count,
                    pr.best_submission_time_seconds,
                    user_rating,
                ],
            )?;
        }
    }

    tx.commit()?;
    Ok(())
}

/// Record a failed fetch attempt.
pub fn mark_contest_failed(conn: &Connection, contest: &CfContest, error: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO cf_contests (contest_id, name, contest_type, start_time, duration, fetched_at, fetch_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            contest.id,
            contest.name,
            contest.contest_type,
            contest.start_time_seconds,
            contest.duration_seconds,
            now,
            format!("error: {}", error),
        ],
    )?;
    Ok(())
}

/// Check if a contest has already been successfully fetched.
pub fn contest_is_fetched(conn: &Connection, contest_id: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cf_contests WHERE contest_id = ?1 AND fetch_status = 'ok'",
        params![contest_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Check if a contest was previously attempted but failed.
pub fn contest_fetch_failed(conn: &Connection, contest_id: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cf_contests WHERE contest_id = ?1 AND fetch_status LIKE 'error:%'",
        params![contest_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Backfill user_rating from contest.ratingChanges data.
/// Uses oldRating (the rating at the time of the contest).
pub fn backfill_user_ratings(
    conn: &Connection,
    contest_id: i64,
    changes: &[CfRatingChange],
) -> Result<()> {
    let mut stmt = conn.prepare(
        "UPDATE cf_contest_results SET user_rating = ?1
         WHERE contest_id = ?2 AND handle = ?3",
    )?;

    for change in changes {
        stmt.execute(params![change.old_rating, contest_id, change.handle])?;
    }

    Ok(())
}

/// Get contest IDs that have been fetched but have no user_rating data.
pub fn contests_missing_ratings(conn: &Connection) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT c.contest_id
         FROM cf_contests c
         JOIN cf_contest_results r ON r.contest_id = c.contest_id
         WHERE c.fetch_status = 'ok'
         GROUP BY c.contest_id
         HAVING SUM(CASE WHEN r.user_rating IS NOT NULL THEN 1 ELSE 0 END) = 0
         ORDER BY c.contest_id",
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

/// Load observations for training or testing.
///
/// If `is_train` is true, loads contests with start_time < cutoff_timestamp.
/// If `is_train` is false, loads contests with start_time >= cutoff_timestamp.
///
/// Filters to users with at least `min_contests` participations (counted across ALL data,
/// not just the split being loaded — so train/test use the same user set).
///
/// `exclude_users` allows excluding specific handles from the dataset (e.g., for cold-start testing).
pub fn load_observations(
    conn: &Connection,
    cutoff_timestamp: i64,
    min_contests: usize,
    is_train: bool,
) -> Result<TrainingDataset> {
    load_observations_filtered(conn, cutoff_timestamp, min_contests, is_train, &[])
}

/// Like `load_observations` but allows excluding specific user handles.
pub fn load_observations_filtered(
    conn: &Connection,
    cutoff_timestamp: i64,
    min_contests: usize,
    is_train: bool,
    exclude_users: &[String],
) -> Result<TrainingDataset> {
    // First, find users with enough contest participations (across all data).
    let mut user_counts: HashMap<String, usize> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT r.handle, r.contest_id
             FROM cf_contest_results r
             JOIN cf_contests c ON c.contest_id = r.contest_id
             WHERE c.fetch_status = 'ok'",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        // Count distinct contests per user
        let mut user_contests: HashMap<String, std::collections::HashSet<i64>> = HashMap::new();
        for row in rows {
            let (handle, contest_id) = row?;
            user_contests.entry(handle).or_default().insert(contest_id);
        }
        for (handle, contests) in user_contests {
            user_counts.insert(handle, contests.len());
        }
    }

    let exclude_set: std::collections::HashSet<&str> =
        exclude_users.iter().map(|s| s.as_str()).collect();

    let qualified_users: std::collections::HashSet<String> = user_counts
        .into_iter()
        .filter(|(handle, count)| *count >= min_contests && !exclude_set.contains(handle.as_str()))
        .map(|(handle, _)| handle)
        .collect();

    // Now load observations for the right split
    let op = if is_train { "<" } else { ">=" };
    let query = format!(
        "SELECT r.handle, r.contest_id, r.problem_idx, r.solved, r.user_rating,
                p.rating as problem_rating, p.tags, c.start_time
         FROM cf_contest_results r
         JOIN cf_contests c ON c.contest_id = r.contest_id
         JOIN cf_contest_problems p ON p.contest_id = r.contest_id AND p.problem_idx = r.problem_idx
         WHERE c.fetch_status = 'ok'
           AND c.start_time {} ?1
           AND p.rating IS NOT NULL",
        op
    );

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map(params![cutoff_timestamp], |row| {
        Ok((
            row.get::<_, String>(0)?,  // handle
            row.get::<_, i64>(1)?,     // contest_id
            row.get::<_, String>(2)?,  // problem_idx
            row.get::<_, bool>(3)?,    // solved
            row.get::<_, Option<i32>>(4)?, // user_rating
            row.get::<_, Option<i32>>(5)?, // problem_rating
            row.get::<_, String>(6)?,  // tags json
            row.get::<_, Option<i64>>(7)?, // start_time
        ))
    })?;

    let mut user_to_idx: HashMap<String, usize> = HashMap::new();
    let mut problem_to_idx: HashMap<String, usize> = HashMap::new();
    let mut problem_ratings: Vec<Option<i32>> = Vec::new();
    let mut problem_tags: Vec<Vec<String>> = Vec::new();
    let mut observations = Vec::new();

    for row in rows {
        let (handle, contest_id, problem_idx, solved, user_rating, problem_rating, tags_json, start_time) =
            row.context("Failed to read observation row")?;

        if !qualified_users.contains(&handle) {
            continue;
        }

        let user_idx = {
            let len = user_to_idx.len();
            *user_to_idx.entry(handle).or_insert(len)
        };

        let problem_key = format!("{}:{}", contest_id, problem_idx);
        let problem_idx_val = {
            let len = problem_to_idx.len();
            let idx = *problem_to_idx.entry(problem_key).or_insert(len);
            if idx == len {
                // New problem
                problem_ratings.push(problem_rating);
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                problem_tags.push(tags);
            }
            idx
        };

        observations.push(Observation {
            user_idx,
            problem_idx: problem_idx_val,
            solved,
            user_rating,
            problem_rating,
            contest_timestamp: start_time.unwrap_or(0),
        });
    }

    Ok(TrainingDataset {
        observations,
        num_users: user_to_idx.len(),
        num_problems: problem_to_idx.len(),
        user_index: user_to_idx,
        problem_index: problem_to_idx,
        problem_ratings,
        problem_tags,
    })
}
