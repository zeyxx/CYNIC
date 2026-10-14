use std::collections::BTreeSet;
use std::path::Path;

use walkdir::WalkDir;

use super::types::{GcScope, SpaceGcObservation};
use crate::scan_exclusions::excluded_dirs;

pub fn observe_space_usage(root: &Path, scope: GcScope) -> SpaceGcObservation {
    let mut total_bytes = 0u64;
    let mut file_count = 0u64;
    let mut targets = Vec::new();

    // Registry-declared skip set (infra/scan-exclusions.json, scanner
    // "space_gc"; compiled fallback outside a worktree). Dependency caches
    // are regenerable and must never be reported as reclaimable user space.
    let skipped: BTreeSet<String> = excluded_dirs(Some(root), "space_gc");

    let patterns = match scope {
        GcScope::Repo => vec![
            "target",
            "target-cynic-ci",
            "target-cynic-mcp-test",
            ".cargo-home/registry/cache",
            ".cargo-home/registry/src",
            ".venv_data",
        ],
        GcScope::Devops => vec!["target", ".cargo-home/registry/cache"],
        GcScope::All => vec![
            "target",
            "target-cynic-ci",
            "target-cynic-mcp-test",
            ".cargo-home/registry/cache",
            ".cargo-home/registry/src",
            ".cargo-home/git/db",
            "node_modules",
            ".venv_data",
        ],
    };

    for pattern in patterns {
        let dir = root.join(pattern);
        if dir.exists() {
            let mut dir_bytes = 0u64;
            let mut dir_count = 0u64;
            // Prune registry-excluded names below the pattern root. Depth 0
            // is exempt: an explicit pattern is explicit intent, so a
            // top-level `.venv_data` or `target` is measured even though the
            // same names are skipped when nested (no double counting).
            let walker = WalkDir::new(&dir)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    e.depth() == 0
                        || !e
                            .file_name()
                            .to_str()
                            .is_some_and(|name| skipped.contains(name))
                });
            for entry in walker {
                if let Ok(e) = entry {
                    if e.file_type().is_file() {
                        if let Ok(meta) = e.metadata() {
                            dir_bytes += meta.len();
                            dir_count += 1;
                        }
                    }
                }
            }
            if dir_bytes > 0 {
                targets.push(super::types::GcTarget {
                    path: pattern.to_string(),
                    bytes: dir_bytes,
                    file_count: dir_count,
                });
                total_bytes += dir_bytes;
                file_count += dir_count;
            }
        }
    }

    SpaceGcObservation {
        schema: "cynic.host.space_gc_observation.v1",
        scope,
        total_bytes,
        file_count,
        targets,
        timestamp_unix_s: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // A fake venv: one lib blob plus a nested target/ that must be
        // pruned (registry-excluded), never double-counted.
        let venv_lib = root.join(".venv_data").join("lib");
        std::fs::create_dir_all(venv_lib.join("target")).expect("venv dirs");
        std::fs::write(venv_lib.join("pkg.bin"), vec![7u8; 1024]).expect("blob");
        std::fs::write(venv_lib.join("target").join("nested.bin"), vec![7u8; 2048])
            .expect("nested blob");
        // A fake cargo target dir with a nested venv (pruned as well).
        let target_debug = root.join("target").join("debug");
        std::fs::create_dir_all(target_debug.join(".venv_data")).expect("target dirs");
        std::fs::write(target_debug.join("bin.bin"), vec![7u8; 512]).expect("blob");
        std::fs::write(
            target_debug.join(".venv_data").join("nested-venv.bin"),
            vec![7u8; 4096],
        )
        .expect("nested venv blob");
        tmp
    }

    #[test]
    fn all_scope_lists_the_venv_without_nested_double_counting() {
        let tmp = fixture_root();
        let obs = observe_space_usage(tmp.path(), GcScope::All);
        let venv = obs
            .targets
            .iter()
            .find(|t| t.path == ".venv_data")
            .expect("venv listed");
        assert_eq!(venv.bytes, 1024, "nested target/ pruned, not counted");
        let target = obs
            .targets
            .iter()
            .find(|t| t.path == "target")
            .expect("target listed");
        assert_eq!(target.bytes, 512, "nested .venv_data pruned, not counted");
        assert_eq!(obs.total_bytes, 1024 + 512);
    }

    #[test]
    fn repo_scope_lists_the_venv_but_devops_does_not() {
        let tmp = fixture_root();
        let repo = observe_space_usage(tmp.path(), GcScope::Repo);
        assert!(repo.targets.iter().any(|t| t.path == ".venv_data"));
        let devops = observe_space_usage(tmp.path(), GcScope::Devops);
        assert!(!devops.targets.iter().any(|t| t.path == ".venv_data"));
    }
}
