use std::path::Path;
use walkdir::WalkDir;

use super::types::{GcScope, GcSummary, SpaceGcObservation};

pub fn observe_space_usage(root: &Path, scope: GcScope) -> SpaceGcObservation {
    let mut total_bytes = 0u64;
    let mut file_count = 0u64;
    let mut targets = Vec::new();

    let patterns = match scope {
        GcScope::Repo => vec![
            "target",
            "target-cynic-ci",
            "target-cynic-mcp-test",
            ".cargo-home/registry/cache",
            ".cargo-home/registry/src",
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
        ],
    };

    for pattern in patterns {
        let dir = root.join(pattern);
        if dir.exists() {
            let mut dir_bytes = 0u64;
            let mut dir_count = 0u64;
            for entry in WalkDir::new(&dir).follow_links(false) {
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
