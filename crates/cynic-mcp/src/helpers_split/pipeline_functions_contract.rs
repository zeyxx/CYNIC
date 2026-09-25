fn validate_pipeline_functions(project_root: &str) -> Vec<serde_json::Value> {
    let api_path = PathBuf::from(project_root).join("crates/cynic-kernel/src/api");
    if !api_path.is_dir() {
        return Vec::new();
    }
    pipeline_function_findings(&api_path, project_root)
}

fn pipeline_function_findings(
    api_path: &std::path::Path,
    project_root: &str,
) -> Vec<serde_json::Value> {
    let pipeline_functions = ["format_crystal_context", "compute_qscore", "trimmed_mean"];
    let function_patterns = pipeline_functions
        .iter()
        .map(|name| {
            (
                *name,
                regex::Regex::new(&format!(r"fn\s+{}", regex::escape(name)))
                    .expect("pipeline regex must compile"),
            )
        })
        .collect::<Vec<_>>();

    let mut findings = Vec::new();
    for file_path in walk_rust_files(api_path) {
        let Ok(content) = fs::read_to_string(&file_path) else {
            continue;
        };
        for (index, line) in content.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for (function_name, pattern) in &function_patterns {
                if pattern.is_match(line) {
                    findings.push(serde_json::json!({
                        "severity": "critical",
                        "kind": "reimplemented_pipeline_function",
                        "path": relative_project_path(project_root, &file_path),
                        "detail": format!("{}:{} ({})", index + 1, line.trim(), function_name),
                    }));
                }
            }
        }
    }
    findings
}
