fn validate_secrets(project_root: &str) -> Vec<serde_json::Value> {
    let staged_content = staged_diff_content(project_root);
    secret_findings_from_diff(&staged_content)
}

fn staged_diff_content(project_root: &str) -> String {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached"])
        .current_dir(project_root)
        .output();
    let Ok(output) = output else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn secret_findings_from_diff(diff: &str) -> Vec<serde_json::Value> {
    let raw_secrets_regex =
        regex::Regex::new(r"AIza[\w-]{10,}|hf_[\w-]{10,}|sk-[\w-]{10,}")
            .expect("raw secrets regex must compile");
    let creds_regex = regex::Regex::new(
        r#"(api[_-]?key|token|password|secret)[^\w]*[:=]\s*['"]?[^\s$<'"]{6,}"#,
    )
    .expect("credential regex must compile");
    let creds_exclusion_regex = regex::Regex::new(
        r#"change-me|your-api-key-here|example|placeholder|dummy|fake|test-key|test-token|token-abc|redacted|already-present|prompt|CancellationToken|JUPYTER_TOKEN|PVEAPIToken|session_token|app_token|secretapikey|apikey|surreal_pass|token\?:\s*string|os\.environ\.get|os\.getenv|getpass\.getpass|args\.api_key|runtime\.API_KEY|globals\(\)\[\\\"API_KEY\\\"\]|get_api_key\(\)|get_x_credentials\(\)|password=password|password\s*= self\._build_password|"password": password|token = runtime\.authorize\(\)|_cf_credentials\(\)|result\.get|payload\.get|self\.api_key\s*=|api_key=api_key"#,
    )
    .expect("credential exclusion regex must compile");

    let mut findings = Vec::new();
    for line in diff.lines() {
        if !line.starts_with('+') || line.starts_with("+++") {
            continue;
        }
        let line_content = &line[1..];
        if raw_secrets_regex.is_match(line_content) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "raw_secret_exposed",
                "path": "git_diff_cached",
                "detail": line_content,
            }));
        }
        if creds_regex.is_match(line_content) && !creds_exclusion_regex.is_match(line_content) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "hardcoded_credential_exposed",
                "path": "git_diff_cached",
                "detail": line_content,
            }));
        }
    }
    findings
}
