use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CfApiResponse<T> {
    pub status: String,
    pub result: Option<T>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfContest {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub contest_type: String,
    pub phase: String,
    pub start_time_seconds: Option<i64>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CfStandingsResult {
    pub contest: CfContest,
    pub problems: Vec<CfProblem>,
    pub rows: Vec<CfRanklistRow>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfProblem {
    pub contest_id: Option<i64>,
    pub index: String,
    pub name: String,
    pub rating: Option<i32>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfRanklistRow {
    pub party: CfParty,
    pub rank: Option<i32>,
    pub problem_results: Vec<CfProblemResult>,
    #[serde(default)]
    pub participant_type: Option<String>,
}

impl CfRanklistRow {
    pub fn participant_type_str(&self) -> &str {
        self.participant_type.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfProblemResult {
    pub points: f64,
    pub rejected_attempt_count: i32,
    pub best_submission_time_seconds: Option<i64>,
    #[serde(rename = "type")]
    pub result_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfParty {
    pub members: Vec<CfMember>,
    pub participant_type: Option<String>,
    pub team_id: Option<i64>,
    pub team_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CfMember {
    pub handle: String,
    pub rating: Option<i32>,
}

/// A Codeforces user profile from user.info.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfUser {
    pub handle: String,
    pub rating: Option<i32>,
    pub max_rating: Option<i32>,
    pub rank: Option<String>,
    pub max_rank: Option<String>,
}

/// A rating change entry from contest.ratingChanges.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfRatingChange {
    pub handle: String,
    pub old_rating: i32,
    pub new_rating: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfSubmission {
    pub id: i64,
    pub contest_id: Option<i64>,
    pub problem: CfProblem,
    pub verdict: Option<String>,
    pub creation_time_seconds: i64,
}

/// A parsed problem statement from the CF problem page HTML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemStatement {
    pub contest_id: i64,
    pub index: String,
    pub title: String,
    pub time_limit: String,
    pub memory_limit: String,
    pub description: String,
    pub input_spec: String,
    pub output_spec: String,
    pub examples: Vec<TestExample>,
    pub note: Option<String>,
}

/// A single example test case (input/output pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExample {
    pub input: String,
    pub output: String,
}

/// Result from problemset.problems endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CfProblemsetResult {
    pub problems: Vec<CfProblem>,
    #[serde(rename = "problemStatistics")]
    pub problem_statistics: Vec<CfProblemStatistics>,
}

/// Statistics for a problem from the problemset endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfProblemStatistics {
    pub contest_id: Option<i64>,
    pub index: String,
    pub solved_count: i64,
}
