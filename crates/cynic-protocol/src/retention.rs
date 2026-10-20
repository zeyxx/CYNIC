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

/// Single source for the build-artefact GC age threshold (48h).
/// All call sites must use this constructor — no inline `48 * 3600`.
pub const BUILD_ARTEFACT_MAX_AGE_SECS: u64 = 48 * 3600;

/// Canonical build-artefact retention policy. Single construction site;
/// the planner (`plan_build_artefacts`) reads `max_age` from here.
pub fn default_build_artefact_policy() -> RetentionPolicy {
    RetentionPolicy {
        kind: RetentionKind::BuildArtefact,
        max_age: Some(Duration::from_secs(BUILD_ARTEFACT_MAX_AGE_SECS)),
        pressure_trigger: None,
        preserve_count: 0,
        preserve_profiles: vec![],
        grace_period: None,
        scopes: vec![RetentionScope {
            root: cynic_types::paths::CynicPaths::state_dir()
                .join("checkouts")
                .display()
                .to_string(),
            patterns: vec![],
        }],
    }
}

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


    #[test]
    fn retention_trio_traits_exist_and_are_grepable() {
        // This test verifies the trio traits exist and can be referenced.
        // It will fail to compile if the trio is missing.
        fn assert_observer<T: RetentionObserver>() {}
        fn assert_planner<T: RetentionPlanner>() {}
        fn assert_executor<T: RetentionExecutor>() {}
        assert_observer::<BuildArtefactRetentionObserver>();
        assert_planner::<BuildArtefactRetentionPlannerStub>();
        assert_executor::<BuildArtefactRetentionExecutor>();
    }

    // Stub implementations for the trait existence test.
    struct BuildArtefactRetentionObserver;
    struct BuildArtefactRetentionPlannerStub;
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

    impl RetentionPlanner for BuildArtefactRetentionPlannerStub {
        fn plan(&self, _obs: &RetentionObservation, _policy: &RetentionPolicy) -> RetentionPlan {
            RetentionPlan {
                schema: RETENTION_PLAN_SCHEMA.to_string(),
                items: vec![],
                total_reclaimable_bytes: 0,
                policy_version: String::new(),
            }
        }
    }

    #[test]
    fn default_policy_pins_single_48h_source() {
        let policy = default_build_artefact_policy();
        assert_eq!(policy.kind, RetentionKind::BuildArtefact);
        assert_eq!(
            policy.max_age,
            Some(Duration::from_secs(BUILD_ARTEFACT_MAX_AGE_SECS))
        );
        assert_eq!(BUILD_ARTEFACT_MAX_AGE_SECS, 48 * 3600);
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
