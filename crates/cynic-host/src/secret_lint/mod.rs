use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

const SUSPICIOUS_PATTERNS: &[&str] = &[
    r#"(api[_-]?key|token|secret|password)\s*=\s*"[A-Za-z0-9_./+:-]{24,}""#,
    r#""--(api-key|token|secret|password)"\s*,\s*"[A-Za-z0-9_./+:-]{24,}""#,
    r#"Bearer\s+[A-Za-z0-9_./+:-]{32,}"#,
    r#"ghp_[A-Za-z0-9]{36}"#,
    r#"gho_[A-Za-z0-9]{36}"#,
];

const EXCLUDED_DIRS: &[&str] = &[".git", "target", ".cargo-home", "node_modules", "vendor"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretLintFinding {
    pub file_path: String,
    pub line_number: usize,
    pub redacted_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretLintObservation {
    pub schema: &'static str,
    pub files_scanned: u64,
    pub findings: Vec<SecretLintFinding>,
    pub timestamp_unix_s: u64,
}

impl SecretLintObservation {
    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }

    pub fn summary(&self) -> String {
        if self.findings.is_empty() {
            format!("{} files scanned, no secrets found", self.files_scanned)
        } else {
            format!(
                "{} files scanned, {} suspicious lines found",
                self.files_scanned,
                self.findings.len()
            )
        }
    }
}

pub fn scan_for_secrets(root: &Path) -> SecretLintObservation {
    let mut findings = Vec::new();
    let mut files_scanned = 0u64;

    let patterns: Vec<Regex> = SUSPICIOUS_PATTERNS
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    for entry in WalkDir::new(root).follow_links(false) {
        if let Ok(e) = entry {
            if !e.file_type().is_file() {
                continue;
            }

            let path = e.path();
            let relative = path.strip_prefix(root).unwrap_or(path);

            if EXCLUDED_DIRS.iter().any(|d| relative.starts_with(d)) {
                continue;
            }

            if let Some(ext) = relative.extension() {
                let ext = ext.to_string_lossy();
                if [
                    "png", "jpg", "jpeg", "gif", "ico", "woff", "woff2", "ttf", "eot", "pdf",
                    "zip", "gz", "bin",
                ]
                .contains(&ext.as_ref())
                {
                    continue;
                }
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                files_scanned += 1;
                for (line_num, line) in content.lines().enumerate() {
                    for pattern in &patterns {
                        if pattern.is_match(line) {
                            findings.push(SecretLintFinding {
                                file_path: relative.to_string_lossy().to_string(),
                                line_number: line_num + 1,
                                redacted_line: redact_secret(line),
                            });
                        }
                    }
                }
            }
        }
    }

    SecretLintObservation {
        schema: "cynic.host.secret_lint_observation.v1",
        files_scanned,
        findings,
        timestamp_unix_s: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

fn redact_secret(line: &str) -> String {
    let re = Regex::new(r#"[A-Za-z0-9_./+:-]{24,}"#).unwrap();
    re.replace_all(line, "[REDACTED]").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_replaces_long_alphanumeric() {
        let result = redact_secret(r#"api_key = "abcdef1234567890abcdef1234567890""#);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("abcdef"));
    }
}
