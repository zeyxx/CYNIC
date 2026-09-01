//! Typed Builder actions for progressively scoped slice checks.
//!
//! This module contains no transport or execution. It makes the exact source,
//! scope, target, and proof identity explicit before a node-owned runner acts.

use serde::{Deserialize, Serialize};

pub const BUILD_SLICE_ACTION_SCHEMA: &str = "cynic.build.slice_action.v1";
pub const BUILD_SLICE_RECEIPT_SCHEMA: &str = "cynic.build.slice_receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSlicePhase {
    Check,
    Test,
    Build,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSliceScope {
    Unit,
    Module,
    Package,
    Crate,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSliceTarget {
    Lib,
    Bin,
    Test,
    Example,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSliceNetworkMode {
    Offline,
    Online,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSliceStatus {
    Passed,
    Failed,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildSliceAction {
    pub schema: String,
    pub source_commit: String,
    pub phase: BuildSlicePhase,
    pub scope: BuildSliceScope,
    pub package: String,
    pub target: BuildSliceTarget,
    pub test_filter: Option<String>,
    pub network_mode: BuildSliceNetworkMode,
    pub runner_ref: String,
    pub idempotency_key: String,
    pub timeout_secs: u32,
    pub max_jobs: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildSliceReceipt {
    pub schema: String,
    pub action_digest: String,
    pub source_commit: String,
    pub checkout_commit: Option<String>,
    pub runner_ref: String,
    pub toolchain: Option<String>,
    pub package: String,
    pub target: BuildSliceTarget,
    pub test_filter: Option<String>,
    pub phase: BuildSlicePhase,
    pub scope: BuildSliceScope,
    pub status: BuildSliceStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub output_digest: Option<String>,
    pub checkout_clean: Option<bool>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

impl BuildSliceAction {
    pub fn validate(&self) -> Vec<String> {
        let mut violations = Vec::new();
        if self.schema != BUILD_SLICE_ACTION_SCHEMA {
            violations.push("build_slice_action_schema_invalid".to_string());
        }
        if self.source_commit.len() != 40
            || !self
                .source_commit
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            violations.push("build_slice_source_commit_invalid".to_string());
        }
        if self.package.trim().is_empty()
            || self.package.contains('/')
            || self.package.contains('\\')
        {
            violations.push("build_slice_package_invalid".to_string());
        }
        if self.runner_ref.trim().is_empty() {
            violations.push("build_slice_runner_ref_missing".to_string());
        }
        if self.idempotency_key.trim().is_empty() {
            violations.push("build_slice_idempotency_key_missing".to_string());
        }
        if self.timeout_secs == 0 || self.timeout_secs > 3600 {
            violations.push("build_slice_timeout_out_of_bounds".to_string());
        }
        if self.max_jobs == 0 || self.max_jobs > 64 {
            violations.push("build_slice_max_jobs_out_of_bounds".to_string());
        }
        if !matches!(self.phase, BuildSlicePhase::Test) && self.test_filter.is_some() {
            violations.push("build_slice_test_filter_requires_test_phase".to_string());
        }
        if matches!(self.scope, BuildSliceScope::Unit | BuildSliceScope::Module)
            && self.test_filter.as_deref().is_none_or(str::is_empty)
        {
            violations.push("build_slice_fine_scope_requires_test_filter".to_string());
        }
        violations
    }
}

impl BuildSliceReceipt {
    pub fn validate(&self) -> Vec<String> {
        let mut violations = Vec::new();
        if self.schema != BUILD_SLICE_RECEIPT_SCHEMA {
            violations.push("build_slice_receipt_schema_invalid".to_string());
        }
        if self.source_commit.len() != 40
            || !self
                .source_commit
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            violations.push("build_slice_receipt_source_commit_invalid".to_string());
        }
        if self.status == BuildSliceStatus::Passed {
            if self.checkout_commit.as_deref() != Some(self.source_commit.as_str()) {
                violations.push("build_slice_passed_checkout_sha_mismatch".to_string());
            }
            if self.checkout_clean != Some(true) {
                violations.push("build_slice_passed_checkout_not_clean".to_string());
            }
            if self.evidence_refs.is_empty() {
                violations.push("build_slice_passed_evidence_missing".to_string());
            }
        }
        if self.status == BuildSliceStatus::Failed && self.exit_code == Some(0) {
            violations.push("build_slice_failed_with_zero_exit_code".to_string());
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action() -> BuildSliceAction {
        BuildSliceAction {
            schema: BUILD_SLICE_ACTION_SCHEMA.to_string(),
            source_commit: "a".repeat(40),
            phase: BuildSlicePhase::Test,
            scope: BuildSliceScope::Module,
            package: "cynic-node".to_string(),
            target: BuildSliceTarget::Lib,
            test_filter: Some("capability_context".to_string()),
            network_mode: BuildSliceNetworkMode::Offline,
            runner_ref: "role:build_runner".to_string(),
            idempotency_key: "digest:action".to_string(),
            timeout_secs: 300,
            max_jobs: 1,
        }
    }

    #[test]
    fn valid_module_test_action_passes() {
        assert!(action().validate().is_empty());
    }

    #[test]
    fn fine_scope_without_filter_is_rejected() {
        let mut value = action();
        value.test_filter = None;
        let violations = value.validate();
        assert!(violations.contains(&"build_slice_fine_scope_requires_test_filter".to_string()));
    }

    #[test]
    fn action_rejects_invalid_commit_and_package_path() {
        let mut value = action();
        value.source_commit = "short".to_string();
        value.package = "../cynic-node".to_string();
        let violations = value.validate();
        assert!(violations.contains(&"build_slice_source_commit_invalid".to_string()));
        assert!(violations.contains(&"build_slice_package_invalid".to_string()));
    }

    #[test]
    fn passed_receipt_requires_exact_clean_checkout_and_evidence() {
        let receipt = BuildSliceReceipt {
            schema: BUILD_SLICE_RECEIPT_SCHEMA.to_string(),
            action_digest: "digest:action".to_string(),
            source_commit: "a".repeat(40),
            checkout_commit: Some("b".repeat(40)),
            runner_ref: "role:build_runner".to_string(),
            toolchain: Some("rustc".to_string()),
            package: "cynic-node".to_string(),
            target: BuildSliceTarget::Lib,
            test_filter: Some("capability_context".to_string()),
            phase: BuildSlicePhase::Test,
            scope: BuildSliceScope::Module,
            status: BuildSliceStatus::Passed,
            exit_code: Some(0),
            duration_ms: Some(10),
            output_digest: Some("digest:output".to_string()),
            checkout_clean: Some(false),
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        };
        let violations = receipt.validate();
        assert!(violations.contains(&"build_slice_passed_checkout_sha_mismatch".to_string()));
        assert!(violations.contains(&"build_slice_passed_checkout_not_clean".to_string()));
        assert!(violations.contains(&"build_slice_passed_evidence_missing".to_string()));
    }
}
