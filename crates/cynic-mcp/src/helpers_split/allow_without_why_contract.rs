fn validate_allow_without_why(project_root: &str) -> Vec<serde_json::Value> {
    let kernel_path = PathBuf::from(project_root).join("crates/cynic-kernel/src");
    let mut findings = Vec::new();
    if !kernel_path.is_dir() {
        return findings;
    }
    let allow_regex = regex::Regex::new(r"#\[allow\(").expect("allow regex must compile");
    for file_path in walk_rust_files(&kernel_path) {
        let relative = relative_project_path(project_root, &file_path);
        if relative.contains("#[cfg(test)]") || relative.contains("#[cfg_attr(test") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&file_path) else {
            continue;
        };
        let lines = content.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if !allow_regex.is_match(line) {
                continue;
            }
            let start = index.saturating_sub(2);
            let end = (index + 3).min(lines.len());
            let has_why = lines[start..end].iter().any(|context| context.contains("WHY:"));
            if !has_why {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "allow_without_why",
                    "path": relative,
                    "detail": format!("{}:{}", index + 1, line.trim()),
                }));
            }
        }
    }
    findings
}
