fn validate_hardcoded_paths(project_root: &str) -> Vec<serde_json::Value> {
    let staged_files = staged_files_for_hardcoded_path_check(project_root);
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
    regex::Regex::new(&format!("{}|{}", regex::escape(&unix_home), regex::escape(&mac_home)))
        .expect("hardcoded path regex must compile")
}

fn staged_files_for_hardcoded_path_check(project_root: &str) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACM"])
        .current_dir(project_root)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
fn hardcoded_path_findings_for_files(
    project_root: &str,
    files: &[&str],
) -> Vec<serde_json::Value> {
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
