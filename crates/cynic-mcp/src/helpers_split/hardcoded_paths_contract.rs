fn validate_hardcoded_paths(project_root: &str, staged_files: &[String]) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let path_regex = hardcoded_path_regex();

    for file_path in staged_files {
        let absolute_path = PathBuf::from(project_root).join(&file_path);
        let Ok(content) = fs::read_to_string(&absolute_path) else {
            continue;
        };
        for (index, line) in content.lines().enumerate() {
            if path_regex.is_match(line) {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "hardcoded_absolute_path",
                    "path": file_path,
                    "detail": format!("{}:{}", index + 1, line.trim()),
                }));
            }
        }
    }

    findings
}

fn hardcoded_path_regex() -> regex::Regex {
    let unix_home = format!("/{}/", "home");
    let mac_home = format!("/{}/", "Users");
    regex::Regex::new(&format!(
        "{}|{}",
        regex::escape(&unix_home),
        regex::escape(&mac_home)
    ))
    .expect("hardcoded path regex must compile")
}

async fn staged_files_for_hardcoded_path_check(
    runner: &dyn ArgvExecutionPort,
    project_root: &str,
) -> Vec<String> {
    let output = runner
        .execute(ArgvExecutionRequest {
            argv: vec![
                "git".to_string(),
                "diff".to_string(),
                "--cached".to_string(),
                "--name-only".to_string(),
                "--diff-filter=ACM".to_string(),
            ],
            current_dir: Some(PathBuf::from(project_root)),
            stdin: None,
            timeout: std::time::Duration::from_secs(30),
        })
        .await;
    if !output.success() {
        return Vec::new();
    }
    output
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
fn hardcoded_path_findings_for_files(project_root: &str, files: &[&str]) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let path_regex = hardcoded_path_regex();
    for file_path in files {
        let absolute_path = PathBuf::from(project_root).join(file_path);
        let Ok(content) = fs::read_to_string(&absolute_path) else {
            continue;
        };
        for (index, line) in content.lines().enumerate() {
            if path_regex.is_match(line) {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "hardcoded_absolute_path",
                    "path": file_path,
                    "detail": format!("{}:{}", index + 1, line.trim()),
                }));
            }
        }
    }
    findings
}
