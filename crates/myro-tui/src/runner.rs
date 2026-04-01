use myro_cf::TestExample;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub struct TestResult {
    pub test_num: usize,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
    pub runtime_ms: u64,
    pub error: Option<String>,
    pub is_custom: bool,
}

fn run_single(test_num: usize, solution_path: &Path, example: &TestExample, lang: &str) -> TestResult {
    let start = Instant::now();

    let (program, args): (&str, Vec<&str>) = match lang {
        "python3" | "python" => ("python3", vec![solution_path.to_str().unwrap_or("solution.py")]),
        _ => ("python3", vec![solution_path.to_str().unwrap_or("solution.py")]),
    };

    let result = Command::new(program)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = match result {
        Ok(c) => c,
        Err(e) => {
            return TestResult {
                test_num,
                passed: false,
                expected: example.output.clone(),
                actual: String::new(),
                runtime_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Failed to start process: {}", e)),
                is_custom: false,
            };
        }
    };

    // Write input to stdin
    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        let _ = stdin.write_all(example.input.as_bytes());
        let _ = stdin.write_all(b"\n");
        // stdin is dropped here, closing it
    }

    // Wait with timeout (5 seconds)
    let output = match wait_with_timeout(&mut child, std::time::Duration::from_secs(5)) {
        Ok(out) => out,
        Err(e) => {
            let _ = child.kill();
            return TestResult {
                test_num,
                passed: false,
                expected: example.output.clone(),
                actual: String::new(),
                runtime_ms: start.elapsed().as_millis() as u64,
                error: Some(e),
                is_custom: false,
            };
        }
    };

    let runtime_ms = start.elapsed().as_millis() as u64;
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        return TestResult {
            test_num,
            passed: false,
            expected: example.output.clone(),
            actual,
            runtime_ms,
            error: Some(if stderr.is_empty() {
                format!("Exit code: {}", output.status.code().unwrap_or(-1))
            } else {
                stderr
            }),
            is_custom: false,
        };
    }

    let expected_trimmed = example.output.trim();
    let passed = actual == expected_trimmed;

    TestResult {
        test_num,
        passed,
        expected: expected_trimmed.to_string(),
        actual,
        runtime_ms,
        error: if stderr.is_empty() { None } else { Some(stderr) },
        is_custom: false,
    }
}

pub fn run_tests_incremental(
    solution_path: &Path,
    examples: &[TestExample],
    lang: &str,
    tx: &std::sync::mpsc::Sender<TestResult>,
) {
    for (i, ex) in examples.iter().enumerate() {
        let result = run_single(i + 1, solution_path, ex, lang);
        if tx.send(result).is_err() {
            break;
        }
    }
}

pub fn run_custom(solution_path: &Path, input: &str) -> Vec<TestResult> {
    let example = TestExample { input: input.to_string(), output: String::new() };
    let mut result = run_single(1, solution_path, &example, "python3");
    result.is_custom = true;
    result.passed = result.error.is_none();
    vec![result]
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Process has exited, collect output
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    use std::io::Read;
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    use std::io::Read;
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output {
                    status: _status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    return Err("Time Limit Exceeded (5s)".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => return Err(format!("Error waiting for process: {}", e)),
        }
    }
}
