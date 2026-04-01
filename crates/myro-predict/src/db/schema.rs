use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

pub fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)
        .with_context(|| format!("Failed to open database at {}", path.display()))?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    create_tables(&conn)?;

    Ok(conn)
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS cf_contests (
            contest_id    INTEGER PRIMARY KEY,
            name          TEXT NOT NULL,
            contest_type  TEXT NOT NULL,
            start_time    INTEGER,
            duration      INTEGER,
            fetched_at    INTEGER NOT NULL,
            fetch_status  TEXT NOT NULL DEFAULT 'ok'
        );

        CREATE TABLE IF NOT EXISTS cf_contest_problems (
            contest_id    INTEGER NOT NULL,
            problem_idx   TEXT NOT NULL,
            name          TEXT NOT NULL,
            rating        INTEGER,
            tags          TEXT NOT NULL DEFAULT '[]',
            PRIMARY KEY (contest_id, problem_idx),
            FOREIGN KEY (contest_id) REFERENCES cf_contests(contest_id)
        );

        CREATE TABLE IF NOT EXISTS cf_contest_results (
            contest_id    INTEGER NOT NULL,
            handle        TEXT NOT NULL,
            problem_idx   TEXT NOT NULL,
            solved        INTEGER NOT NULL,
            rejected_count INTEGER NOT NULL DEFAULT 0,
            solve_time    INTEGER,
            user_rating   INTEGER,
            PRIMARY KEY (contest_id, handle, problem_idx),
            FOREIGN KEY (contest_id, problem_idx) REFERENCES cf_contest_problems(contest_id, problem_idx)
        );

        CREATE TABLE IF NOT EXISTS prediction_models (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            name          TEXT NOT NULL,
            created_at    INTEGER NOT NULL,
            config_json   TEXT NOT NULL,
            model_blob    BLOB NOT NULL,
            metrics       TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_results_handle ON cf_contest_results(handle);
        CREATE INDEX IF NOT EXISTS idx_results_contest ON cf_contest_results(contest_id);
        CREATE INDEX IF NOT EXISTS idx_contests_start ON cf_contests(start_time);
        ",
    )
    .context("Failed to create tables")?;

    Ok(())
}
