//! One-shot admission for the first isolated build-runner checkout.
//!
//! This is deliberately narrower than a scheduler grant. It binds a staged
//! immutable source bundle to a clean build workspace while the scheduler and
//! its signer are not yet available. It never authorizes runtime activation.

use serde::{Deserialize, Serialize};

use crate::digest_json;
use crate::one_shot_capability::{ONE_SHOT_CAPABILITY_SCHEMA, OneShotCapability};

pub const BUILD_RUNNER_BOOTSTRAP_PREFLIGHT_SCHEMA: &str =
    "cynic.build_runner.bootstrap_preflight.v1";
pub const BUILD_RUNNER_BOOTSTRAP_ADMISSION_SCHEMA: &str =
    "cynic.build_runner.bootstrap_admission.v1";
pub const BUILD_RUNNER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA: &str =
    "cynic.build_runner.bootstrap_capability_binding.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerBootstrapPreflight {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub source_commit: String,
    pub source_bundle_digest: String,
    pub registry_path: String,
    pub registry_digest: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerBootstrapAdmission {
    pub schema: String,
    pub status: String,
    pub authority_class: String,
    pub node_id: String,
    pub source_commit: String,
    pub registry_path: String,
    pub registry_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_bundle_digest: Option<String>,
    pub target_phase: String,
    pub allowed_actions: Vec<String>,
    pub proofs: Vec<String>,
}

/// Binds the generic recovery capability to the exact first build workspace.
/// This is intentionally separate from the legacy receipt: a v1 receipt did
/// not carry a machine key or executor provenance and cannot be upgraded by
/// inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerBootstrapCapabilityBinding {
    pub schema: String,
    pub node_subject: String,
    pub source_commit: String,
    pub source_bundle_digest: String,
    pub registry_digest: String,
    pub executor_digest: String,
    pub capability: OneShotCapability,
}

pub fn build_runner_bootstrap_input_digest(
    binding: &BuildRunnerBootstrapCapabilityBinding,
) -> Result<String, serde_json::Error> {
    digest_json(&serde_json::json!({
        "schema": BUILD_RUNNER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA,
        "node_subject": binding.node_subject,
        "source_commit": binding.source_commit,
        "source_bundle_digest": binding.source_bundle_digest,
        "registry_digest": binding.registry_digest,
        "executor_digest": binding.executor_digest,
    }))
}

pub fn build_runner_bootstrap_action_digest(action: &str) -> Result<String, serde_json::Error> {
    digest_json(&serde_json::json!({
        "schema": BUILD_RUNNER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA,
        "action": action,
    }))
}

/// Schema + types for the 2N `runtime_publisher` bootstrap capability binding.
/// This is deliberately a SEPARATE binding from the build-runner one: the
/// kernel admits a `RuntimeMachineIdentity` to publish proof/receipt bundles
/// (runtime_publisher capability) without inheriting any build-runner authority.
pub const RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA: &str =
    "cynic.runtime_publisher.bootstrap_capability_binding.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePublisherCapabilityBinding {
    pub schema: String,
    pub node_subject: String,
    pub source_commit: String,
    pub registry_digest: String,
    pub executor_digest: String,
    pub capability: OneShotCapability,
}

pub fn runtime_publisher_bootstrap_input_digest(
    binding: &RuntimePublisherCapabilityBinding,
) -> Result<String, serde_json::Error> {
    digest_json(&serde_json::json!({
        "schema": RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA,
        "node_subject": binding.node_subject,
        "source_commit": binding.source_commit,
        "registry_digest": binding.registry_digest,
        "executor_digest": binding.executor_digest,
        "capability": binding.capability,
    }))
}

pub fn runtime_publisher_bootstrap_action_digest(
    action: &str,
) -> Result<String, serde_json::Error> {
    digest_json(&serde_json::json!({
        "schema": RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA,
        "action": action,
    }))
}

pub fn validate_runtime_publisher_bootstrap_capability_binding(
    binding: &RuntimePublisherCapabilityBinding,
    action: &str,
    now_unix_s: u64,
) -> Vec<String> {
    let mut blockers = Vec::new();
    let expected = RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA;
    if binding.schema != expected {
        blockers.push(format!(
            "schema mismatch: expected {expected}, got {}",
            binding.schema
        ));
    }
    if binding.capability.schema != ONE_SHOT_CAPABILITY_SCHEMA {
        blockers.push("capability schema mismatch".to_string());
    }
    // Scope must be the runtime publisher admission surface only.
    if binding.capability.scope != "runtime-publisher-admission" {
        blockers.push(format!(
            "capability scope must be runtime-publisher-admission, got {}",
            binding.capability.scope
        ));
    }
    if binding.capability.status != "admitted" {
        blockers.push("capability status must be admitted".to_string());
    }
    if binding.capability.expires_at_unix_s <= now_unix_s {
        blockers.push("capability expired".to_string());
    }
    let _ = action;
    blockers
}

pub fn validate_build_runner_bootstrap_capability_binding(
    binding: &BuildRunnerBootstrapCapabilityBinding,
    action: &str,
    now_unix_s: u64,
) -> Vec<String> {
    let mut blockers = Vec::new();
    let action_digest = build_runner_bootstrap_action_digest(action).ok();
    let input_digest = build_runner_bootstrap_input_digest(binding).ok();
    if binding.schema != BUILD_RUNNER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA
        || !valid_commit(&binding.source_commit)
        || !valid_sha256(&binding.source_bundle_digest)
        || !valid_sha256(&binding.registry_digest)
        || !valid_sha256(&binding.executor_digest)
        || binding.node_subject.trim().is_empty()
        || binding.capability.authority_class != "MachineEnrollment"
        || binding.capability.scope != "build-runner-bootstrap"
        || input_digest.as_deref() != Some(binding.capability.input_digest.as_str())
        || binding.capability.executor_digest != binding.executor_digest
    {
        blockers.push("build_runner_bootstrap_capability_binding_invalid".to_string());
    }
    let Some(action_digest) = action_digest else {
        blockers.push("build_runner_bootstrap_capability_action_invalid".to_string());
        return blockers;
    };
    blockers.extend(binding.capability.validate_for(
        &binding.node_subject,
        "build-runner-bootstrap",
        &action_digest,
        now_unix_s,
    ));
    blockers.sort();
    blockers.dedup();
    blockers
}

impl BuildRunnerBootstrapAdmission {
    /// Validates only intrinsic receipt claims. Node, checkout, registry bytes,
    /// ownership, and one-shot consumption remain observations at the host
    /// execution boundary.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != BUILD_RUNNER_BOOTSTRAP_ADMISSION_SCHEMA
            || self.status != "admitted"
            || self.authority_class != "MachineEnrollment"
        {
            blockers.push("build_runner_bootstrap_admission_identity_invalid".to_string());
        }
        if self.node_id.trim().is_empty() || !valid_commit(&self.source_commit) {
            blockers.push("build_runner_bootstrap_admission_binding_invalid".to_string());
        }
        if self.registry_path.trim().is_empty() || !valid_sha256(&self.registry_digest) {
            blockers.push("build_runner_bootstrap_admission_registry_invalid".to_string());
        }
        if self
            .source_bundle_digest
            .as_deref()
            .is_none_or(|digest| !valid_sha256(digest))
        {
            blockers.push("build_runner_bootstrap_admission_source_bundle_invalid".to_string());
        }
        let expected_actions: &[&str] = match self.target_phase.as_str() {
            "check" => &["make:check", "cargo:check", "cargo:run", "cargo:test"],
            "build" => &[
                "make:build-host-exec",
                "make:build",
                "make:build-release",
                "cargo:build",
            ],
            _ => &[],
        };
        if expected_actions.is_empty()
            || self.allowed_actions.is_empty()
            || self
                .allowed_actions
                .iter()
                .any(|action| !expected_actions.contains(&action.as_str()))
        {
            blockers.push("build_runner_bootstrap_admission_action_invalid".to_string());
        }
        if self.proofs.is_empty() {
            blockers.push("build_runner_bootstrap_admission_proofs_missing".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability_binding() -> BuildRunnerBootstrapCapabilityBinding {
        let mut binding = BuildRunnerBootstrapCapabilityBinding {
            schema: BUILD_RUNNER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA.to_string(),
            node_subject: "node-key:sha256:subject".to_string(),
            source_commit: "a".repeat(40),
            source_bundle_digest: format!("sha256:{}", "b".repeat(64)),
            registry_digest: format!("sha256:{}", "c".repeat(64)),
            executor_digest: format!("sha256:{}", "d".repeat(64)),
            capability: OneShotCapability {
                schema: crate::ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
                status: "admitted".to_string(),
                authority_class: "MachineEnrollment".to_string(),
                capability_id: "bootstrap_build_1".to_string(),
                subject: "node-key:sha256:subject".to_string(),
                scope: "build-runner-bootstrap".to_string(),
                input_digest: String::new(),
                executor_digest: format!("sha256:{}", "d".repeat(64)),
                issued_at_unix_s: 10,
                expires_at_unix_s: 20,
                allowed_action_digests: vec![
                    build_runner_bootstrap_action_digest("make:check").expect("action digest"),
                ],
                authority_key_id: format!("sha256:{}", "e".repeat(64)),
                signature_hex: "f".repeat(128),
                evidence_refs: vec!["enrollment:proof".to_string()],
                budget: None,
            },
        };
        binding.capability.input_digest =
            build_runner_bootstrap_input_digest(&binding).expect("input digest");
        binding
    }

    fn admission() -> BuildRunnerBootstrapAdmission {
        BuildRunnerBootstrapAdmission {
            schema: BUILD_RUNNER_BOOTSTRAP_ADMISSION_SCHEMA.to_string(),
            status: "admitted".to_string(),
            authority_class: "MachineEnrollment".to_string(),
            node_id: "cynic-builder".to_string(),
            source_commit: "a".repeat(40),
            registry_path: "/etc/cynic/registry.json".to_string(),
            registry_digest: format!("sha256:{}", "b".repeat(64)),
            source_bundle_digest: Some(format!("sha256:{}", "c".repeat(64))),
            target_phase: "check".to_string(),
            allowed_actions: vec!["make:check".to_string()],
            proofs: vec!["source_commit_read_back".to_string()],
        }
    }

    #[test]
    fn bootstrap_receipt_rejects_activation_and_missing_bundle_binding() {
        let mut receipt = admission();
        receipt.allowed_actions.push("activate".to_string());
        receipt.source_bundle_digest = None;
        let blockers = receipt.validate();
        assert!(blockers.contains(&"build_runner_bootstrap_admission_action_invalid".to_string()));
        assert!(
            blockers
                .contains(&"build_runner_bootstrap_admission_source_bundle_invalid".to_string())
        );
    }

    #[test]
    fn bootstrap_build_receipt_admits_the_isolated_host_exec_action() {
        let mut receipt = admission();
        receipt.target_phase = "build".to_string();
        receipt.allowed_actions = vec!["make:build-host-exec".to_string()];
        assert!(receipt.validate().is_empty());
    }

    #[test]
    fn capability_binding_rejects_unbound_executor_or_action() {
        let mut binding = capability_binding();
        assert!(
            validate_build_runner_bootstrap_capability_binding(&binding, "make:check", 11)
                .is_empty()
        );
        binding.executor_digest = format!("sha256:{}", "0".repeat(64));
        assert!(
            validate_build_runner_bootstrap_capability_binding(&binding, "make:check", 11)
                .contains(&"build_runner_bootstrap_capability_binding_invalid".to_string())
        );
    }
}
