use anyhow::Result;

use crate::client::SupabaseClient;
use crate::types::*;

/// Push solved problem IDs. Upserts on (user_id, problem_id).
pub fn push_solved(client: &SupabaseClient, problems: &[SolvedProblemRow]) -> Result<()> {
    if problems.is_empty() {
        return Ok(());
    }
    client.post("solved_problems", problems, true)
}

/// Pull all solved problems for the authenticated user.
pub fn pull_solved(client: &SupabaseClient) -> Result<Vec<SolvedProblemRow>> {
    client.get(
        "solved_problems",
        &format!("user_id=eq.{}&select=*", client.user_id),
    )
}

/// Push past entries. Upserts on (user_id, contest_id, index).
pub fn push_past_entries(client: &SupabaseClient, entries: &[PastEntryRow]) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    client.post("past_entries", entries, true)
}

/// Pull all past entries for the authenticated user.
pub fn pull_past_entries(client: &SupabaseClient) -> Result<Vec<PastEntryRow>> {
    client.get(
        "past_entries",
        &format!("user_id=eq.{}&select=*", client.user_id),
    )
}

/// Push skill snapshots (append-only, no upsert).
pub fn push_skill_snapshots(
    client: &SupabaseClient,
    snapshots: &[SkillSnapshotRow],
) -> Result<()> {
    if snapshots.is_empty() {
        return Ok(());
    }
    client.post("skill_snapshots", snapshots, false)
}

/// Push a coaching session record.
pub fn push_coaching_session(client: &SupabaseClient, session: &CoachingSessionRow) -> Result<()> {
    client.post("coaching_sessions", &[session], false)
}

/// Push a solution record.
pub fn push_solution(client: &SupabaseClient, solution: &SolutionRow) -> Result<()> {
    client.post("solutions", &[solution], false)
}

/// Upsert profile data (cf_handle, display_name, target_probability).
pub fn sync_profile(
    client: &SupabaseClient,
    cf_handle: Option<&str>,
    display_name: Option<&str>,
    target_probability: Option<f64>,
) -> Result<()> {
    let profile = Profile {
        user_id: client.user_id.clone(),
        cf_handle: cf_handle.map(String::from),
        display_name: display_name.map(String::from),
        target_probability,
        created_at: None,
    };
    client.post("profiles", &[profile], true)
}

/// Pull profile for the authenticated user.
pub fn pull_profile(client: &SupabaseClient) -> Result<Option<Profile>> {
    let rows: Vec<Profile> = client.get(
        "profiles",
        &format!("user_id=eq.{}&select=*", client.user_id),
    )?;
    Ok(rows.into_iter().next())
}
