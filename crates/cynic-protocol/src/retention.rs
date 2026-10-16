//! Unified retention trio: Observer → Planner → Executor.
//!
//! Three traits and shared types for GC/retention/cleanup surfaces.
//! The build_artefact domain is the first consumer; the trio is general.
//!
//! Pattern: observe (read-only) → plan (pure) → execute (effect).

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const RETENTION_OBSERVATION_SCHEMA: &str = "cynic.retention_observation.v1";
pub const RETENTION_PLAN_SCHEMA: &str = "cynic.retention_plan.v1";
pub const RETENTION_RESULT_SCHEMA: &str = "cynic.retention_result.v1";
pub const BUILD_ARTEFACT_REPORT_SCHEMA: &str = "cynic.build_artefact_report.v1";
pub const BUILD_ARTEFACT_GC_PLAN_SCHEMA: &str = "cynic.build_artefact_gc_plan.v1";
pub const BUILD_ARTEFACT_GC_RESULT_SCHEMA: &str = "cynic.build_artefact_gc_result.v1";
pub const FLEET_BUILD_ARTEFACT_SCHEMA: &str = "cynic.fleet_build_artefact.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionKind {
    DiskCache,
    BuildArtefact,
    BuildCache,
    Release,
    Worktree,
    KnowledgeCache,
    ModelArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionScope {
    pub root: String,
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PressureSignal {
    pub use_pct: u8,
    pub avail_bytes: u64,
    pub is_pressured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PressureTrigger {
    pub use_pct_threshold: u8,
    pub avail_bytes_threshold: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub kind: RetentionKind,
    pub max_age: Option<Duration>,
    pub pressure_trigger: Option<PressureTrigger>,
    pub preserve_count: usize,
    pub preserve_profiles: Vec<String>,
    pub grace_period: Option<Duration>,
    pub scopes: Vec<RetentionScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionItem {
    pub id: String,
    pub path: String,
    pub size_bytes: u64,
    pub age: Option<Duration>,
    pub reason: String,
    pub requires_privilege: bool,
    pub argv: Vec<String>,
    // Domain-specific context (used by planner)
    pub is_orphan: Option<bool>,
    pub is_root_owned: Option<bool>,
    pub remote_exists: Option<bool>,
    pub sha: Option<String>,
    pub branch: Option<String>,
    pub owner_uid: Option<u32>,
    pub owner_gid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionObservation {
    pub schema: String,
    pub timestamp: String,
    pub items: Vec<RetentionItem>,
    pub total_reclaimable_bytes: u64,
    pub pressure: Option<PressureSignal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPlan {
    pub schema: String,
    pub items: Vec<RetentionItem>,
    pub total_reclaimable_bytes: u64,
    pub policy_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionItemExecution {
    pub item_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionResult {
    pub schema: String,
    pub timestamp: String,
    pub success: bool,
    pub items_executed: Vec<RetentionItemExecution>,
    pub total_reclaimed_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionError {
    pub message: String,
}

impl RetentionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RetentionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RetentionError {}

impl From<String> for RetentionError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for RetentionError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// Phase 1: read-only observation.
pub trait RetentionObserver {
    fn observe(&self) -> Result<RetentionObservation, RetentionError>;
}

/// Phase 2: pure decision from observation + policy.
pub trait RetentionPlanner {
    fn plan(&self, obs: &RetentionObservation, policy: &RetentionPolicy) -> RetentionPlan;
}

/// Phase 3: effect with proof.
pub trait RetentionExecutor {
    fn execute(&self, plan: &RetentionPlan) -> Result<RetentionResult, RetentionError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference implementation of the retired build_gc_plan logic.
    // This is the equivalence target: the trio planner must produce the same
    // plan for the same observation input.
    fn reference_gc_plan(
        checkouts: &[(String, String, u64, u32, u64, bool, bool)], // (path, sha, size, uid, age_secs, is_orphan, is_root_owned)
    ) -> Vec<(String, String, u64, String, bool, Vec<String>)> {
        // (id, path, size_bytes, reason, requires_privilege, argv)
        let mut items = Vec::new();
        for (path, sha, size_bytes, uid, age_secs, is_orphan, is_root_owned) in checkouts {
            let mut reasons = Vec::new();
            if *is_orphan {
                reasons.push("orphan(no remote)".to_string());
            }
            let is_old = *age_secs > 48 * 3600;
            if is_old {
                reasons.push(format!("age {}h", *age_secs / 3600));
            }
            if reasons.is_empty() {
                continue;
            }
            if *is_root_owned {
                reasons.push("root-owned".to_string());
            }
            let reason = reasons.join("+");
            let id = format!("checkout:{}", sha.chars().take(8).collect::<String>());
            let requires_privilege = *is_root_owned;
            let argv = vec!["rm".to_string(), "-rf".to_string(), path.clone()];
            items.push((id, path.clone(), *size_bytes, reason, requires_privilege, argv));
        }
        items
    }

    fn make_observation(
        checkouts: &[(String, String, u64, u32, u64, bool, bool)],
    ) -> RetentionObservation {
        let items: Vec<RetentionItem> = checkouts
            .iter()
            .map(|(path, sha, size_bytes, uid, age_secs, is_orphan, is_root_owned)| {
                RetentionItem {
                    id: format!("checkout:{}", sha.chars().take(8).collect::<String>()),
                    path: path.clone(),
                    size_bytes: *size_bytes,
                    age: Some(Duration::from_secs(*age_secs)),
                    reason: String::new(),
                    requires_privilege: *is_root_owned,
                    argv: vec![],
                    is_orphan: Some(*is_orphan),
                    is_root_owned: Some(*is_root_owned),
                    remote_exists: Some(!*is_orphan),
                    sha: Some(sha.clone()),
                    branch: Some(path.rsplit('/').next().unwrap_or(path).to_string()),
                    owner_uid: Some(*uid),
                    owner_gid: Some(990),
                }
            })
            .collect();
        RetentionObservation {
            schema: RETENTION_OBSERVATION_SCHEMA.to_string(),
            timestamp: "2026-10-16T00:00:00Z".to_string(),
            items,
            total_reclaimable_bytes: 0,
            pressure: None,
        }
    }

    fn make_policy() -> RetentionPolicy {
        RetentionPolicy {
            kind: RetentionKind::BuildArtefact,
            max_age: Some(Duration::from_secs(48 * 3600)),
            pressure_trigger: None,
            preserve_count: 0,
            preserve_profiles: vec![],
            grace_period: None,
            scopes: vec![],
        }
    }

    // The trio planner implementation under test.
    struct BuildArtefactRetentionPlanner;

    impl RetentionPlanner for BuildArtefactRetentionPlanner {
        fn plan(&self, obs: &RetentionObservation, policy: &RetentionPolicy) -> RetentionPlan {
            let mut items = Vec::new();
            let mut total = 0u64;
            for item in &obs.items {
                let mut reasons = Vec::new();
                if item.is_orphan == Some(true) {
                    reasons.push("orphan(no remote)".to_string());
                }
                let is_old = item
                    .age
                    .and_then(|a| policy.max_age.map(|max| a > max))
                    .unwrap_or(false);
                if is_old {
                    if let Some(age) = item.age {
                        reasons.push(format!("age {}h", age.as_secs() / 3600));
                    }
                }
                if reasons.is_empty() {
                    continue;
                }
                if item.is_root_owned == Some(true) {
                    reasons.push("root-owned".to_string());
                }
                let reason = reasons.join("+");
                total += item.size_bytes;
                items.push(RetentionItem {
                    id: item.id.clone(),
                    path: item.path.clone(),
                    size_bytes: item.size_bytes,
                    age: item.age,
                    reason,
                    requires_privilege: item.requires_privilege,
                    argv: vec![
                        "rm".to_string(),
                        "-rf".to_string(),
                        item.path.clone(),
                    ],
                    is_orphan: item.is_orphan,
                    is_root_owned: item.is_root_owned,
                    remote_exists: item.remote_exists,
                    sha: item.sha.clone(),
                    branch: item.branch.clone(),
                    owner_uid: item.owner_uid,
                    owner_gid: item.owner_gid,
                });
            }
            RetentionPlan {
                schema: RETENTION_PLAN_SCHEMA.to_string(),
                items,
                total_reclaimable_bytes: total,
                policy_version: "v1".to_string(),
            }
        }
    }

    #[test]
    fn trio_plan_matches_retired_implementation_orphan_and_age() {
        let checkouts = vec![
            (
                "/var/lib/cynic/checkouts/old-orphan".to_string(),
                "abc123".to_string(),
                5000u64,
                999u32,
                200_000u64,
                true,
                false,
            ),
            (
                "/var/lib/cynic/checkouts/live".to_string(),
                "live123".to_string(),
                5000u64,
                999u32,
                1000u64,
                false,
                false,
            ),
            (
                "/var/lib/cynic/checkouts/aged-root".to_string(),
                "aged01".to_string(),
                10000u64,
                0u32,
                100_000u64,
                false,
                true,
            ),
        ];
        let expected = reference_gc_plan(&checkouts);
        let obs = make_observation(&checkouts);
        let policy = make_policy();
        let planner = BuildArtefactRetentionPlanner;
        let plan = planner.plan(&obs, &policy);
        assert_eq!(plan.items.len(), expected.len());
        for (item, (id, path, size_bytes, reason, requires_privilege, argv)) in
            plan.items.iter().zip(expected.iter())
        {
            assert_eq!(item.id, *id);
            assert_eq!(item.path, *path);
            assert_eq!(item.size_bytes, *size_bytes);
            assert_eq!(item.reason, *reason);
            assert_eq!(item.requires_privilege, *requires_privilege);
            assert_eq!(item.argv, *argv);
        }
    }

    #[test]
    fn trio_plan_matches_retired_implementation_empty() {
        let checkouts: Vec<(String, String, u64, u32, u64, bool, bool)> = vec![];
        let expected = reference_gc_plan(&checkouts);
        let obs = make_observation(&checkouts);
        let policy = make_policy();
        let planner = BuildArtefactRetentionPlanner;
        let plan = planner.plan(&obs, &policy);
        assert_eq!(plan.items.len(), 0);
        assert_eq!(plan.items.len(), expected.len());
    }

    #[test]
    fn trio_plan_matches_retired_implementation_all_reclaimable() {
        let checkouts = vec![
            (
                "/var/lib/cynic/checkouts/orphan1".to_string(),
                "orphan1".to_string(),
                1000u64,
                999u32,
                100u64,
                true,
                false,
            ),
            (
                "/var/lib/cynic/checkouts/aged1".to_string(),
                "aged001".to_string(),
                2000u64,
                999u32,
                200_000u64,
                false,
                false,
            ),
        ];
        let expected = reference_gc_plan(&checkouts);
        let obs = make_observation(&checkouts);
        let policy = make_policy();
        let planner = BuildArtefactRetentionPlanner;
        let plan = planner.plan(&obs, &policy);
        assert_eq!(plan.items.len(), 2);
        assert_eq!(plan.items.len(), expected.len());
        assert_eq!(plan.total_reclaimable_bytes, 3000);
    }

    #[test]
    fn retention_trio_traits_exist_and_are_grepable() {
        // This test verifies the trio traits exist and can be referenced.
        // It will fail to compile if the trio is missing.
        fn assert_observer<T: RetentionObserver>() {}
        fn assert_planner<T: RetentionPlanner>() {}
        fn assert_executor<T: RetentionExecutor>() {}
        assert_observer::<BuildArtefactRetentionObserver>();
        assert_planner::<BuildArtefactRetentionPlanner>();
        assert_executor::<BuildArtefactRetentionExecutor>();
    }

    // Stub implementations for the trait existence test.
    struct BuildArtefactRetentionObserver;
    struct BuildArtefactRetentionExecutor;

    impl RetentionObserver for BuildArtefactRetentionObserver {
        fn observe(&self) -> Result<RetentionObservation, RetentionError> {
            Ok(RetentionObservation {
                schema: RETENTION_OBSERVATION_SCHEMA.to_string(),
                timestamp: String::new(),
                items: vec![],
                total_reclaimable_bytes: 0,
                pressure: None,
            })
        }
    }

    impl RetentionExecutor for BuildArtefactRetentionExecutor {
        fn execute(&self, _plan: &RetentionPlan) -> Result<RetentionResult, RetentionError> {
            Ok(RetentionResult {
                schema: RETENTION_RESULT_SCHEMA.to_string(),
                timestamp: String::new(),
                success: true,
                items_executed: vec![],
                total_reclaimed_bytes: 0,
            })
        }
    }
}
