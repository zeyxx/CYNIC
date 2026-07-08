fn validate_reqwest_usage(project_root: &str) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let search_paths = [
        "crates/cynic-kernel/src/domain",
        "crates/cynic-kernel/src/api",
        "crates/cynic-kernel/src/main.rs",
    ];
    let exclusion_file = "crates/cynic-kernel/src/api/mcp/proxy.rs";

    for relative_path in search_paths {
        let absolute_path = PathBuf::from(project_root).join(relative_path);
        if absolute_path.is_file() {
            scan_reqwest_usage_file(
                project_root,
                &absolute_path,
                exclusion_file,
                &mut findings,
            );
            continue;
        }
        if absolute_path.is_dir() {
            for entry in walk_rust_files(&absolute_path) {
                scan_reqwest_usage_file(project_root, &entry, exclusion_file, &mut findings);
            }
        }
    }

    findings
}

fn scan_reqwest_usage_file(
    project_root: &str,
    file_path: &std::path::Path,
    exclusion_file: &str,
    findings: &mut Vec<serde_json::Value>,
) {
    let relative = relative_project_path(project_root, file_path);
    if relative.ends_with(exclusion_file) {
        return;
    }
    let Ok(content) = fs::read_to_string(file_path) else {
        return;
    };
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.contains("reqwest") && !trimmed.starts_with("//") && !trimmed.contains("K2-exempt") {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "misplaced_reqwest_usage",
                "path": relative,
                "detail": format!("{}:{}", index + 1, line.trim()),
            }));
        }
    }
}

fn walk_rust_files(root: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_rust_files(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files
}
