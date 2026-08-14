fn validate_unlogged_ok(project_root: &str) -> Vec<serde_json::Value> {
    let rust_files: Vec<String> = staged_files_for_hardcoded_path_check(project_root)
        .into_iter()
        .filter(|file| file.ends_with(".rs"))
        .collect();
    unlogged_ok_findings_for_files(project_root, &rust_files)
}

fn unlogged_ok_findings_for_files(
    project_root: &str,
    rust_files: &[String],
) -> Vec<serde_json::Value> {
    let exclusion_patterns = [
        r"//",
        r"#\[cfg\(test\)\]",
        r"mod tests",
        r"parse\(\)\.ok\(\)",
        r"env::var.*\.ok\(\)",
        r"inspect_err",
        r"to_str\(\)\.ok\(\)",
        r"from_utf8.*\.ok\(\)",
        r"filter_map.*\.ok\(\)",
        r"create_dir_all.*\.ok\(\)",
        r"remove_file.*\.ok\(\)",
        r"\.lock\(\)\.ok\(\)",
        r"\.read\(\)\.ok\(\)",
        r"\.output\(\)",
        r"read_to_string.*\.ok\(\)",
    ]
    .into_iter()
    .map(|pattern| regex::Regex::new(pattern).expect("exclusion regex must compile"))
    .collect::<Vec<_>>();
    let logging_keywords = regex::Regex::new(r"inspect_err|tracing::|eprintln!|warn!|env::var|WHY:|K14|poison")
        .expect("logging regex must compile");

    let mut findings = Vec::new();
    for file_path in rust_files {
        if file_path.contains("probe/") || file_path.contains("probes/") {
            continue;
        }
        let absolute_path = PathBuf::from(project_root).join(file_path);
        let Ok(content) = fs::read_to_string(&absolute_path) else {
            continue;
        };
        let lines = content.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains(".ok()") {
                continue;
            }
            if exclusion_patterns.iter().any(|pattern| pattern.is_match(line)) {
                continue;
            }
            let start = index.saturating_sub(8);
            let end = (index + 4).min(lines.len());
            let vicinity = lines[start..end].join("\n");
            if !logging_keywords.is_match(&vicinity) {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "unlogged_ok_call",
                    "path": file_path,
                    "detail": format!("{}:{}", index + 1, line.trim()),
                }));
            }
        }
    }
    findings
}
