fn validate_ips(project_root: &str, staged_content: &str) -> Vec<serde_json::Value> {
    let mut findings = ip_findings_from_diff(staged_content);
    findings.extend(ip_findings_from_backends(project_root));
    findings
}

fn ip_regex() -> regex::Regex {
    regex::Regex::new(r"100\.(74|75|119)\.\d{1,3}\.\d{1,3}").expect("IP regex must compile")
}

fn ip_findings_from_diff(diff: &str) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let ip_regex = ip_regex();
    for line in diff.lines() {
        if !line.starts_with('+') || line.starts_with("+++") {
            continue;
        }
        let line_content = &line[1..];
        if ip_regex.is_match(line_content) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "staged_hardcoded_ip",
                "path": "git_diff_cached",
                "detail": line_content,
            }));
        }
    }
    findings
}

fn ip_findings_from_backends(project_root: &str) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let backends_path = PathBuf::from(project_root).join("backends.toml");
    let Ok(content) = fs::read_to_string(&backends_path) else {
        return findings;
    };
    let ip_regex = ip_regex();
    for line in content.lines() {
        if ip_regex.is_match(line) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "backends_hardcoded_ip",
                "path": "backends.toml",
                "detail": line.trim(),
            }));
        }
    }
    findings
}
