use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// A problem file deserialized from JSON
#[derive(Debug, Clone, Deserialize)]
pub struct ProblemFile {
    pub contest_id: i64,
    pub index: String,
    pub title: String,
    pub difficulty: i32,
    pub tags: Vec<String>,
    pub time_limit: Option<String>,
    pub memory_limit: Option<String>,
    pub description: String,
    pub input_spec: String,
    pub output_spec: String,
    pub examples: Vec<ExampleFile>,
    pub routes: Vec<RouteFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExampleFile {
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteFile {
    pub name: String,
    pub description: String,
    pub skill_tags: Vec<String>,
    pub observations: Vec<ObservationFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObservationFile {
    pub title: String,
    pub description: String,
    pub hints: HintsFile,
    pub skill_tag: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HintsFile {
    pub nudge: String,
    pub question: String,
    pub formal: String,
}

impl ProblemFile {
    /// Derive the problem ID from content: "cf:{contest_id}{index}"
    pub fn id(&self) -> String {
        format!("cf:{}{}", self.contest_id, self.index)
    }

    /// Total number of observations across all routes
    pub fn total_observations(&self) -> usize {
        self.routes.iter().map(|r| r.observations.len()).sum()
    }
}

/// Load a single problem from a JSON file
pub fn load_problem_file(path: &Path) -> Result<ProblemFile> {
    let content = std::fs::read_to_string(path)?;
    let problem: ProblemFile = serde_json::from_str(&content)?;
    Ok(problem)
}

/// Load all problem JSON files from a directory
pub fn load_problem_set(dir: &Path) -> Result<Vec<ProblemFile>> {
    let mut problems = Vec::new();
    if !dir.exists() {
        return Ok(problems);
    }
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "json")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let problem = load_problem_file(&entry.path())?;
        problems.push(problem);
    }
    // Sort by difficulty
    problems.sort_by_key(|p| p.difficulty);
    Ok(problems)
}

/// Default path for the problem set directory (repo root `test-problem-set/`)
pub fn default_problem_set_dir() -> PathBuf {
    // Try to find relative to the executable or current dir
    let candidates = [
        PathBuf::from("test-problem-set"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-problem-set"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }
    // Fallback
    candidates[0].clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_problem_set() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-problem-set");
        let problems = load_problem_set(&dir).unwrap();
        assert!(problems.len() >= 10, "Expected at least 10 problems, got {}", problems.len());

        for p in &problems {
            assert!(!p.title.is_empty());
            assert!(!p.routes.is_empty(), "Problem {} has no routes", p.id());
            for r in &p.routes {
                assert!(
                    r.observations.len() >= 3,
                    "Route '{}' in {} has only {} observations",
                    r.name,
                    p.id(),
                    r.observations.len()
                );
                for o in &r.observations {
                    assert!(!o.hints.nudge.is_empty(), "Empty nudge in {} - {}", p.id(), o.title);
                    assert!(!o.hints.question.is_empty(), "Empty question in {} - {}", p.id(), o.title);
                    assert!(!o.hints.formal.is_empty(), "Empty formal in {} - {}", p.id(), o.title);
                }
            }
        }
    }

    #[test]
    fn test_problem_id_derivation() {
        let json = r#"{
            "contest_id": 4,
            "index": "A",
            "title": "Watermelon",
            "difficulty": 800,
            "tags": ["math"],
            "description": "test",
            "input_spec": "test",
            "output_spec": "test",
            "examples": [],
            "routes": []
        }"#;
        let p: ProblemFile = serde_json::from_str(json).unwrap();
        assert_eq!(p.id(), "cf:4A");
    }
}
