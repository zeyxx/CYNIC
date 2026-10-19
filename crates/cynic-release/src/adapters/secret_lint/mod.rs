use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use walkdir::WalkDir;

const SUSPICIOUS_PATTERNS: &[&str] = &[
    r#"(api[_-]?key|token|secret|password)\s*=\s*"[A-Za-z0-9_./+:-]{24,}""#,
    r#""--(api-key|token|secret|password)"\s*,\s*"[A-Za-z0-9_./+:-]{24,}""#,
    r#"Bearer\s+[A-Za-z0-9_./+:-]{32,}"#,
    r#"ghp_[A-Za-z0-9]{36}"#,
    r#"gho_[A-Za-z0-9]{36}"#,
];

/// Compiled-in fallback, kept identical to the `fallback` block of
/// `infra/scan-exclusions.json`. A release or builder run can execute outside the
/// repository root, where infra/ is absent; degrading to today's behaviour keeps the
/// gate fail-closed on a real finding instead of fail-open by scanning a vendored venv.
const FALLBACK_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".cargo-home",
    ".venv",
    ".venv_data",
    "__pycache__",
    "node_modules",
    "target",
    "vendor",
];

const FALLBACK_EXCLUDED_EXTENSIONS: &[&str] = &[
    "bin", "gif", "gz", "ico", "jpeg", "jpg", "pdf", "png", "ttf", "woff", "woff2", "zip",
];

#[derive(Debug, Clone, Deserialize)]
struct ScanExclusionSets {
    #[serde(default)]
    excluded_dirs: Vec<String>,
    #[serde(default)]
    excluded_extensions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ScanExclusions {
    #[serde(default)]
    scanners: BTreeMap<String, ScanExclusionSets>,
    fallback: ScanExclusionSets,
}

/// Resolve the skip sets for one scanner, preferring the registry over the fallback.
///
/// An absent or unparsable registry degrades to the compiled-in list; it never degrades
/// to scanning everything. Scanner identity is explicit so adding a scanner cannot
/// silently inherit another scanner's exclusions.
fn exclusion_sets(root: &Path, scanner: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let registry = root.join("infra").join("scan-exclusions.json");
    let sets = std::fs::read_to_string(&registry)
        .ok()
        .and_then(|raw| serde_json::from_str::<ScanExclusions>(&raw).ok())
        .and_then(|registry| registry.scanners.get(scanner).cloned())
        .unwrap_or_else(|| ScanExclusionSets {
            excluded_dirs: FALLBACK_EXCLUDED_DIRS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            excluded_extensions: FALLBACK_EXCLUDED_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        });

    (
        sets.excluded_dirs.into_iter().collect(),
        sets.excluded_extensions.into_iter().collect(),
    )
}

/// Match any path component, so nested `target/` directories are skipped, not just a top-level
/// `target/`. The previous `relative.starts_with(d)` compared whole components
/// against one-element prefixes and missed every nested build directory.
fn is_excluded(relative: &Path, dirs: &BTreeSet<String>, extensions: &BTreeSet<String>) -> bool {
    if relative
        .components()
        .any(|component| dirs.contains(component.as_os_str().to_string_lossy().as_ref()))
    {
        return true;
    }
    relative
        .extension()
        .map(|ext| extensions.contains(ext.to_string_lossy().as_ref()))
        .unwrap_or(false)
}

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
    let (excluded_dirs, excluded_extensions) = exclusion_sets(root, "secret_lint");

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

            if is_excluded(relative, &excluded_dirs, &excluded_extensions) {
                continue;
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
        let fixture = format!("{}=\"{}\"", "api_key", "abcdef1234567890abcdef1234567890");
        let result = redact_secret(&fixture);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("abcdef"));
    }

    fn sets(dirs: &[&str], extensions: &[&str]) -> (BTreeSet<String>, BTreeSet<String>) {
        (
            dirs.iter().map(|s| s.to_string()).collect(),
            extensions.iter().map(|s| s.to_string()).collect(),
        )
    }

    /// Regression: the release gate failed on a third-party `verify_token=` literal
    /// inside a 1.3G `.venv_data` tree. The walker must skip a Python venv wherever it
    /// sits, at the root or nested under a crate.
    #[test]
    fn excludes_python_venv_at_root_and_nested() {
        let (dirs, extensions) = sets(FALLBACK_EXCLUDED_DIRS, &[]);
        assert!(is_excluded(Path::new(".venv_data"), &dirs, &extensions));
        assert!(is_excluded(
            Path::new("infra/scripts/data_system/.venv_data"),
            &dirs,
            &extensions
        ));
        assert!(is_excluded(
            Path::new("crates/cynic-host/.venv"),
            &dirs,
            &extensions
        ));
    }

    /// End-to-end proof on the real walker: a credential literal inside a Python venv
    /// is not reported, and the same literal in tracked source is. `tempfile` gives an
    /// out-of-tree root, so this also covers the registry being absent and the
    /// compiled-in fallback taking over -- the degraded mode, not the happy path.
    #[test]
    fn venv_finding_is_suppressed_but_source_finding_is_not() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let venv = tmp.path().join(".venv_data").join("site-packages");
        std::fs::create_dir_all(&venv).expect("create venv");
        std::fs::write(
            venv.join("_eval_results.py"),
            "verify_token=\"abcdef1234567890abcdef1234567890\"\n",
        )
        .expect("write venv file");
        std::fs::write(
            tmp.path().join("config.py"),
            "verify_token=\"abcdef1234567890abcdef1234567890\"\n",
        )
        .expect("write source file");

        let observation = scan_for_secrets(tmp.path());

        assert_eq!(
            observation.files_scanned,
            1,
            "only the tracked-source file may be scanned, got: {:?}",
            observation
                .findings
                .iter()
                .map(|f| f.file_path.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            observation.findings.len(),
            1,
            "one real leak, one false positive"
        );
        assert!(
            observation.findings[0].file_path.contains("config.py"),
            "the finding must be the tracked source, not the vendored package"
        );
    }

    /// The previous `relative.starts_with(d)` compared whole path components, so a
    /// nested `crates/x/target/` build directory was scanned in full.
    #[test]
    fn excludes_nested_build_directory() {
        let (dirs, extensions) = sets(FALLBACK_EXCLUDED_DIRS, &[]);
        assert!(is_excluded(
            Path::new("crates/cynic-release/target/debug/deps/libx.rlib"),
            &dirs,
            &extensions
        ));
    }

    /// A directory whose name merely starts with an excluded name must still be
    /// scanned; otherwise `vendor/` exclusion would silently blind the gate to
    /// `vendored-config/`.
    #[test]
    fn does_not_exclude_prefix_only_match() {
        let (dirs, extensions) = sets(FALLBACK_EXCLUDED_DIRS, &[]);
        assert!(!is_excluded(
            Path::new("vendored-config/settings.toml"),
            &dirs,
            &extensions
        ));
        assert!(!is_excluded(
            Path::new("crates/cynic-release/src/lib.rs"),
            &dirs,
            &extensions
        ));
    }

    #[test]
    fn excludes_by_extension() {
        let (dirs, extensions) = sets(&[], FALLBACK_EXCLUDED_EXTENSIONS);
        assert!(is_excluded(
            Path::new("docs/diagram.png"),
            &dirs,
            &extensions
        ));
        assert!(!is_excluded(Path::new("docs/notes.md"), &dirs, &extensions));
    }

    /// The registry is the source; the compiled-in list is only a degraded mode. They
    /// must not drift, or the fallback would silently diverge from the contract.
    #[test]
    fn registry_matches_compiled_in_fallback() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("crate is at <repo>/crates/<name>");
        let registry_path = repo_root.join("infra").join("scan-exclusions.json");
        let raw = std::fs::read_to_string(&registry_path)
            .unwrap_or_else(|e| panic!("registry must exist at {}: {e}", registry_path.display()));
        let registry: ScanExclusions = serde_json::from_str(&raw).expect("registry must parse");

        let declared: Vec<&str> = registry
            .fallback
            .excluded_dirs
            .iter()
            .map(String::as_str)
            .collect();
        for dir in FALLBACK_EXCLUDED_DIRS {
            assert!(
                declared.contains(dir),
                "{dir} is compiled in but absent from infra/scan-exclusions.json"
            );
        }
        for dir in &registry.fallback.excluded_dirs {
            assert!(
                FALLBACK_EXCLUDED_DIRS.contains(&dir.as_str()),
                "{dir} is in the registry but not compiled in"
            );
        }

        let declared_ext: Vec<&str> = registry
            .fallback
            .excluded_extensions
            .iter()
            .map(String::as_str)
            .collect();
        for ext in FALLBACK_EXCLUDED_EXTENSIONS {
            assert!(
                declared_ext.contains(ext),
                "{ext} is compiled in but absent from the registry"
            );
        }

        assert!(
            registry.scanners.contains_key("secret_lint"),
            "the consumer of this registry must declare itself"
        );
    }
}
