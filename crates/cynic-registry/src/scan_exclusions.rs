//! Reader for `infra/scan-exclusions.json` — the single source for the directory
//! sets repository scanners must skip.
//!
//! Authority: `infra/scan-exclusions.json` (`cynic.infra.scan_exclusions.v1`,
//! normative). An absent or unparsable registry degrades to the compiled-in
//! list; it never degrades to scanning everything. Scanner identity is
//! explicit so adding a scanner cannot silently inherit another scanner's
//! exclusions. (Same rule as `cynic-release` secret_lint; this crate cannot
//! depend on it, so the 20-line read pattern lives here too.)

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

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

#[derive(Debug, Clone, Default, Deserialize)]
struct ScanExclusionSets {
    #[serde(default)]
    excluded_dirs: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ScanExclusions {
    #[serde(default)]
    scanners: BTreeMap<String, ScanExclusionSets>,
    #[serde(default)]
    fallback: ScanExclusionSets,
}

/// Find the repository root by walking up from `start` for
/// `infra/scan-exclusions.json`. Returns `None` outside a worktree.
pub fn discover_registry_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if dir.join("infra").join("scan-exclusions.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Excluded directory names for one scanner. Registry first, compiled fallback
/// second, never an empty set.
pub fn excluded_dirs(repo_root: Option<&Path>, scanner: &str) -> BTreeSet<String> {
    let from_registry = repo_root
        .and_then(|root| std::fs::read_to_string(root.join("infra").join("scan-exclusions.json")).ok())
        .and_then(|raw| serde_json::from_str::<ScanExclusions>(&raw).ok())
        .and_then(|registry| registry.scanners.get(scanner).cloned())
        .map(|sets| sets.excluded_dirs.into_iter().collect::<BTreeSet<_>>());
    match from_registry {
        Some(dirs) if !dirs.is_empty() => dirs,
        _ => FALLBACK_EXCLUDED_DIRS.iter().map(|s| s.to_string()).collect(),
    }
}

/// Excluded directory names for one scanner, discovering the worktree from the
/// current directory. Outside a worktree this is the compiled fallback.
pub fn excluded_dirs_for_cwd(scanner: &str) -> BTreeSet<String> {
    let root = std::env::current_dir()
        .ok()
        .and_then(|cwd| discover_registry_root(&cwd));
    excluded_dirs(root.as_deref(), scanner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_compiled_list_outside_a_worktree() {
        let dirs = excluded_dirs(None, "space_gc");
        assert!(dirs.contains(".venv_data"));
        assert!(dirs.contains("target"));
        assert!(dirs.contains(".git"));
    }

    #[test]
    fn reads_scanner_identity_from_the_registry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let infra = tmp.path().join("infra");
        std::fs::create_dir(&infra).expect("infra dir");
        std::fs::write(
            infra.join("scan-exclusions.json"),
            r#"{"scanners": {"space_gc": {"excluded_dirs": ["custom-skip"]}}}"#,
        )
        .expect("registry");
        let dirs = excluded_dirs(Some(tmp.path()), "space_gc");
        assert!(dirs.contains("custom-skip"));
        assert!(!dirs.contains("target"));
    }

    #[test]
    fn discovers_the_registry_by_walking_up() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let infra = tmp.path().join("infra");
        std::fs::create_dir(&infra).expect("infra dir");
        std::fs::write(infra.join("scan-exclusions.json"), r#"{"scanners": {}}"#)
            .expect("registry");
        let nested = tmp.path().join("crates").join("x");
        std::fs::create_dir_all(&nested).expect("nested");
        assert_eq!(discover_registry_root(&nested).as_deref(), Some(tmp.path()));
    }
}
