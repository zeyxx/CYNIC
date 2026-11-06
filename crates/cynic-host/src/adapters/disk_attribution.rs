//! Pure per-class disk volume attribution probe.
//!
//! Addresses P3 / measurement gap #4 from substrate construction priority:
//! attributing disk weight to distinct governed classes (checkouts, targets, sccache, workspaces).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskClass {
    /// Ephemeral evidence checkouts (`checkouts/`)
    Checkouts,
    /// Canonical shared warm cargo cache (`shared-target/`)
    SharedTarget,
    /// Deployment build targets (`build-targets/`)
    BuildTargets,
    /// Agent workload workspaces (`workspaces/` or `workloads/`)
    WorkloadWorkspaces,
    /// Compiler content cache (`sccache/`)
    CompilerCache,
    /// Temporary files (`/tmp` or ephemeral scratch)
    Temporary,
    /// Unclassified or uncategorized paths
    Other,
}

impl DiskClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Checkouts => "checkouts",
            Self::SharedTarget => "shared_target",
            Self::BuildTargets => "build_targets",
            Self::WorkloadWorkspaces => "workload_workspaces",
            Self::CompilerCache => "compiler_cache",
            Self::Temporary => "temporary",
            Self::Other => "other",
        }
    }
}

/// Classifies a relative or absolute path into its governed disk class.
pub fn classify_disk_path(path: &str) -> DiskClass {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_matches('/');

    if trimmed.starts_with("tmp") || trimmed.contains("/tmp/") || trimmed.contains("/tmp") {
        DiskClass::Temporary
    } else if trimmed.contains("checkouts") {
        DiskClass::Checkouts
    } else if trimmed.contains("shared-target") {
        DiskClass::SharedTarget
    } else if trimmed.contains("build-targets") {
        DiskClass::BuildTargets
    } else if trimmed.contains("workspaces") || trimmed.contains("workloads") {
        DiskClass::WorkloadWorkspaces
    } else if trimmed.contains("sccache") {
        DiskClass::CompilerCache
    } else {
        DiskClass::Other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskClassAttribution {
    pub class: DiskClass,
    pub class_name: String,
    pub total_bytes: u64,
    pub entry_count: usize,
}

/// Aggregates an inventory of (path, bytes) into per-class totals, sorted descending by size.
pub fn aggregate_disk_classes(inventory: &[(String, u64)]) -> Vec<DiskClassAttribution> {
    use std::collections::HashMap;

    let mut totals: HashMap<DiskClass, (u64, usize)> = HashMap::new();
    for (path, bytes) in inventory {
        let class = classify_disk_path(path);
        let entry = totals.entry(class).or_insert((0, 0));
        entry.0 += bytes;
        entry.1 += 1;
    }

    let mut result: Vec<DiskClassAttribution> = totals
        .into_iter()
        .map(|(class, (total_bytes, entry_count))| DiskClassAttribution {
            class,
            class_name: class.as_str().to_string(),
            total_bytes,
            entry_count,
        })
        .collect();

    result.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_disk_path() {
        assert_eq!(
            classify_disk_path("/var/lib/cynic/checkouts/cynic-abc123456789"),
            DiskClass::Checkouts
        );
        assert_eq!(
            classify_disk_path("/var/lib/cynic/build-targets/fgate"),
            DiskClass::BuildTargets
        );
        assert_eq!(
            classify_disk_path("/var/lib/cynic/shared-target/debug"),
            DiskClass::SharedTarget
        );
        assert_eq!(
            classify_disk_path("/var/lib/cynic/source/workspaces/build-workload-01"),
            DiskClass::WorkloadWorkspaces
        );
        assert_eq!(
            classify_disk_path("/var/lib/cynic/sccache/a/b/c"),
            DiskClass::CompilerCache
        );
        assert_eq!(
            classify_disk_path("/tmp/cynic-build-scratch"),
            DiskClass::Temporary
        );
        assert_eq!(
            classify_disk_path("/var/lib/cynic/random_unregistered"),
            DiskClass::Other
        );
    }

    #[test]
    fn test_aggregate_disk_classes() {
        let inventory = vec![
            ("/var/lib/cynic/checkouts/c1".to_string(), 1000),
            ("/var/lib/cynic/checkouts/c2".to_string(), 2000),
            ("/var/lib/cynic/build-targets/t1".to_string(), 5000),
            ("/tmp/scratch".to_string(), 500),
        ];

        let agg = aggregate_disk_classes(&inventory);
        assert_eq!(agg[0].class, DiskClass::BuildTargets);
        assert_eq!(agg[0].total_bytes, 5000);
        assert_eq!(agg[0].entry_count, 1);

        assert_eq!(agg[1].class, DiskClass::Checkouts);
        assert_eq!(agg[1].total_bytes, 3000);
        assert_eq!(agg[1].entry_count, 2);

        assert_eq!(agg[2].class, DiskClass::Temporary);
        assert_eq!(agg[2].total_bytes, 500);
        assert_eq!(agg[2].entry_count, 1);
    }
}
