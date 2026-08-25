//! Pure Builder route-convergence decision.
//!
//! This contract separates temporary provider/SSH bootstrap paths from normal
//! fenced workload delivery. It owns no I/O and never turns an incomplete proof
//! into a remote mutation authority.

use serde::{Deserialize, Serialize};

use crate::SecurityEvidenceState;

use super::bootstrap_admission::{
    BuildRunnerBootstrapCapabilityBinding, build_runner_bootstrap_action_digest,
    build_runner_bootstrap_input_digest,
};
use super::{BUILD_RUNNER_OBSERVATION_MAX_AGE_MS, BuildRunnerObservation};

pub const BUILD_RUNNER_ROUTE_CONVERGENCE_SCHEMA: &str = "cynic.build_runner.route_convergence.v1";
pub const BUILD_RUNNER_SEED_ACTION_PLAN_SCHEMA: &str = "cynic.build_runner.seed_action_plan.v1";
pub const BUILD_RUNNER_SEED_READ_BACK_CONTRACT_SCHEMA: &str =
    "cynic.build_runner.seed_read_back_contract.v1";
pub const BUILD_RUNNER_ROUTE_EVIDENCE_MAX_AGE_MS: u64 = BUILD_RUNNER_OBSERVATION_MAX_AGE_MS;

/// Bounded action proposed for the `SshSeed` route only.
///
/// This is a *plan*, never an execution. A host adapter must still verify the
/// one-shot capability binding and obtain explicit `--yes`/admission before
/// running `argv`, and must perform post-action read-back. `RuntimeDelivery`
/// never consults this action — it requires `RuntimeFenced` and its own
/// scheduler grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerSeedActionPlan {
    pub schema: &'static str,
    pub route_state: BuildRunnerRouteState,
    /// The node id the seed targets. Must match `BuildRunnerRouteDesiredState::node_id`.
    pub node_id: String,
    /// Exact source commit the seed materializes.
    pub source_commit: String,
    /// Digest of the staged source bundle that will be fetched.
    pub source_bundle_digest: String,
    /// Digest of the bootstrap executor used by the bounded action.
    pub executor_digest: String,
    /// Bounded argv for the seed action only (never arbitrary shell).
    pub argv: Vec<Vec<String>>,
    /// Human-readable, machine-checkable post-action read-back contract.
    /// The adapter must refuse success unless all are satisfied.
    pub read_back_contract: BuildRunnerSeedReadBackContract,
    /// TTL the capability and its action are valid for, in seconds.
    pub ttl_seconds: u64,
    /// The one-shot capability binding that must validate before argv runs.
    pub capability_binding: BuildRunnerBootstrapCapabilityBinding,
}

/// Machine-checkable post-action read-back contract for `SshSeed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerSeedReadBackContract {
    pub schema: &'static str,
    /// The seed checkout must report HEAD equal to this commit.
    pub expected_commit: String,
    /// The fetched source-bundle digest must equal this value.
    pub expected_source_bundle_digest: String,
    /// The persisted bootstrap proof ref must be a regular file with this digest.
    pub bootstrap_proof_ref: String,
    /// The capability must be marked consumed in durable state.
    pub capability_consumed_proof_ref: String,
}

impl BuildRunnerRouteDesiredState {
    /// Propose the bounded `SshSeed` action plan for an admitted/proved seed
    /// state. Returns `None` for any non-seed state (including `RuntimeFenced`,
    /// which uses a scheduler grant instead) and for any blocker/mismatch.
    pub fn propose_seed_action(
        &self,
        decision: &BuildRunnerRouteDecision,
        capability_binding: BuildRunnerBootstrapCapabilityBinding,
        now_unix_s: u64,
    ) -> Option<BuildRunnerSeedActionPlan> {
        match decision.state {
            BuildRunnerRouteState::SeedAdmitted | BuildRunnerRouteState::SeedProved => {}
            _ => return None,
        }
        // Seed path must actually be SshSeed, and there must be no blockers or
        // residual unknowns that would have prevented reaching it cleanly.
        if decision.next_path != Some(BuildRunnerRoutePath::SshSeed)
            || !decision.blockers.is_empty()
            || !decision.residual_unknowns.is_empty()
        {
            return None;
        }
        // The capability must still be valid at the decision time.
        if capability_binding.capability.expires_at_unix_s <= now_unix_s {
            return None;
        }
        // The binding must target the exact desired source commit and node.
        if capability_binding.source_commit != self.source_commit
            || capability_binding.node_subject.is_empty()
        {
            return None;
        }
        // Capability must bind to the build-runner-bootstrap scope and be admitted.
        if capability_binding.capability.scope != "build-runner-bootstrap"
            || capability_binding.capability.status != "admitted"
        {
            return None;
        }
        let action_digest = build_runner_bootstrap_action_digest("bootstrap:materialize").ok()?;
        if !capability_binding
            .capability
            .allowed_action_digests
            .contains(&action_digest)
        {
            return None;
        }
        let input_digest = build_runner_bootstrap_input_digest(&capability_binding).ok()?;
        if input_digest != capability_binding.capability.input_digest {
            return None;
        }
        Some(BuildRunnerSeedActionPlan {
            schema: BUILD_RUNNER_SEED_ACTION_PLAN_SCHEMA,
            route_state: decision.state.clone(),
            node_id: self.node_id.clone(),
            source_commit: self.source_commit.clone(),
            source_bundle_digest: capability_binding.source_bundle_digest.clone(),
            executor_digest: capability_binding.executor_digest.clone(),
            argv: vec![vec![
                "cynic-host-exec".to_string(),
                "bootstrap-seed".to_string(),
            ]],
            read_back_contract: BuildRunnerSeedReadBackContract {
                schema: BUILD_RUNNER_SEED_READ_BACK_CONTRACT_SCHEMA,
                expected_commit: self.source_commit.clone(),
                expected_source_bundle_digest: capability_binding.source_bundle_digest.clone(),
                bootstrap_proof_ref: format!(
                    "bootstrap:proof:{}:{}",
                    self.node_id, self.source_commit
                ),
                capability_consumed_proof_ref: format!(
                    "capability:consumed:{}",
                    capability_binding.capability.capability_id
                ),
            },
            ttl_seconds: capability_binding
                .capability
                .expires_at_unix_s
                .saturating_sub(now_unix_s),
            capability_binding,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildRunnerRoutePath {
    ProxmoxRecovery,
    SshSeed,
    RuntimeDelivery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildRunnerRouteState {
    Unroutable,
    ProviderObserved,
    SshTrustVerified,
    SeedAdmitted,
    SeedProved,
    BuilderRuntimeObserved,
    RuntimeFenced,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerRouteDesiredState {
    pub node_id: String,
    pub provider: String,
    pub resource_id: String,
    pub role: String,
    pub source_commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerProviderRouteObservation {
    pub status: String,
    pub node_id: String,
    pub provider: String,
    pub resource_id: String,
    pub observed_at_unix_ms: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerSshTrustRouteObservation {
    /// `trusted_match`, `untrusted_observed`, `trust_refuted`, or `not_observed`.
    pub status: String,
    pub observed_at_unix_ms: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerSeedRouteObservation {
    /// `absent`, `admitted`, `proved`, `refuted`, or `unknown`.
    pub status: String,
    pub node_id: String,
    pub source_commit: String,
    pub observed_at_unix_ms: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerCoordinationRouteObservation {
    /// `stable`, `incompatible`, `unavailable`, or `unknown`.
    pub status: String,
    pub observed_at_unix_ms: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerRouteObservation {
    pub provider: BuildRunnerProviderRouteObservation,
    pub ssh_trust: BuildRunnerSshTrustRouteObservation,
    pub seed: BuildRunnerSeedRouteObservation,
    pub builder_runtime: Option<BuildRunnerObservation>,
    pub coordination: BuildRunnerCoordinationRouteObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerRouteDecision {
    pub schema: String,
    pub node_id: String,
    pub state: BuildRunnerRouteState,
    pub next_path: Option<BuildRunnerRoutePath>,
    /// A route decision never executes. Adapters require a distinct explicit
    /// admission before performing their bounded action.
    pub mutates: bool,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

pub fn reconcile_build_runner_route(
    desired: &BuildRunnerRouteDesiredState,
    observed: &BuildRunnerRouteObservation,
    now_unix_ms: u64,
) -> BuildRunnerRouteDecision {
    let mut blockers = provider_blockers(desired, &observed.provider, now_unix_ms);
    if !blockers.is_empty() {
        return decision(
            desired,
            BuildRunnerRouteState::Blocked,
            None,
            blockers,
            Vec::new(),
        );
    }

    if observed.ssh_trust.status == "trust_refuted" {
        return decision(
            desired,
            BuildRunnerRouteState::Blocked,
            None,
            vec!["build_runner_ssh_trust_refuted".to_string()],
            Vec::new(),
        );
    }
    if !fresh(&observed.ssh_trust.observed_at_unix_ms, now_unix_ms)
        || observed.ssh_trust.evidence_refs.is_empty()
        || observed.ssh_trust.status != "trusted_match"
    {
        return decision(
            desired,
            BuildRunnerRouteState::ProviderObserved,
            Some(BuildRunnerRoutePath::ProxmoxRecovery),
            Vec::new(),
            vec!["build_runner_ssh_trust_unproven_or_stale".to_string()],
        );
    }

    if observed.seed.status == "refuted" {
        return decision(
            desired,
            BuildRunnerRouteState::Blocked,
            None,
            vec!["build_runner_seed_refuted".to_string()],
            Vec::new(),
        );
    }
    if observed.seed.node_id != desired.node_id {
        return decision(
            desired,
            BuildRunnerRouteState::Blocked,
            None,
            vec!["build_runner_seed_node_mismatch".to_string()],
            Vec::new(),
        );
    }
    if observed.seed.source_commit != desired.source_commit {
        return decision(
            desired,
            BuildRunnerRouteState::Blocked,
            None,
            vec!["build_runner_seed_source_commit_mismatch".to_string()],
            Vec::new(),
        );
    }
    if observed.seed.status == "admitted" && fresh(&observed.seed.observed_at_unix_ms, now_unix_ms)
    {
        return decision(
            desired,
            BuildRunnerRouteState::SeedAdmitted,
            Some(BuildRunnerRoutePath::SshSeed),
            Vec::new(),
            Vec::new(),
        );
    }
    if observed.seed.status != "proved"
        || !fresh(&observed.seed.observed_at_unix_ms, now_unix_ms)
        || observed.seed.evidence_refs.is_empty()
    {
        return decision(
            desired,
            BuildRunnerRouteState::SshTrustVerified,
            Some(BuildRunnerRoutePath::SshSeed),
            Vec::new(),
            vec!["build_runner_seed_unproved_or_stale".to_string()],
        );
    }

    let Some(runtime) = observed.builder_runtime.as_ref() else {
        return decision(
            desired,
            BuildRunnerRouteState::SeedProved,
            Some(BuildRunnerRoutePath::SshSeed),
            Vec::new(),
            vec!["build_runner_runtime_unobserved".to_string()],
        );
    };
    blockers = runtime_blockers(desired, runtime, now_unix_ms);
    if !blockers.is_empty() {
        return decision(
            desired,
            BuildRunnerRouteState::Blocked,
            None,
            blockers,
            Vec::new(),
        );
    }

    if observed.coordination.status != "stable"
        || !fresh(&observed.coordination.observed_at_unix_ms, now_unix_ms)
        || observed.coordination.evidence_refs.is_empty()
    {
        return decision(
            desired,
            BuildRunnerRouteState::BuilderRuntimeObserved,
            None,
            Vec::new(),
            vec!["build_runner_coordination_unproven_or_unstable".to_string()],
        );
    }

    decision(
        desired,
        BuildRunnerRouteState::RuntimeFenced,
        Some(BuildRunnerRoutePath::RuntimeDelivery),
        Vec::new(),
        Vec::new(),
    )
}

fn provider_blockers(
    desired: &BuildRunnerRouteDesiredState,
    provider: &BuildRunnerProviderRouteObservation,
    now_unix_ms: u64,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if provider.status != "observed" {
        blockers.push("build_runner_provider_not_observed".to_string());
    }
    if provider.node_id != desired.node_id
        || provider.provider != desired.provider
        || provider.resource_id != desired.resource_id
    {
        blockers.push("build_runner_provider_identity_mismatch".to_string());
    }
    if !fresh(&provider.observed_at_unix_ms, now_unix_ms) {
        blockers.push("build_runner_provider_observation_stale".to_string());
    }
    if provider.evidence_refs.is_empty() {
        blockers.push("build_runner_provider_evidence_missing".to_string());
    }
    normalize(blockers)
}

fn runtime_blockers(
    desired: &BuildRunnerRouteDesiredState,
    runtime: &BuildRunnerObservation,
    now_unix_ms: u64,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if runtime.schema != super::BUILD_RUNNER_OBSERVATION_SCHEMA || runtime.status != "observed" {
        blockers.push("build_runner_runtime_observation_invalid".to_string());
    }
    if runtime.node_id != desired.node_id
        || runtime.declared_role.as_deref() != Some(desired.role.as_str())
        || runtime.identity_match != SecurityEvidenceState::Proven
    {
        blockers.push("build_runner_runtime_identity_or_role_unproven".to_string());
    }
    if !fresh(&runtime.observed_at_unix_ms, now_unix_ms) {
        blockers.push("build_runner_runtime_observation_stale".to_string());
    }
    if runtime.evidence_refs.is_empty() {
        blockers.push("build_runner_runtime_evidence_missing".to_string());
    }
    normalize(blockers)
}

fn fresh(observed_at_unix_ms: &u64, now_unix_ms: u64) -> bool {
    *observed_at_unix_ms > 0
        && now_unix_ms
            .checked_sub(*observed_at_unix_ms)
            .is_some_and(|age| age <= BUILD_RUNNER_ROUTE_EVIDENCE_MAX_AGE_MS)
}

fn decision(
    desired: &BuildRunnerRouteDesiredState,
    state: BuildRunnerRouteState,
    next_path: Option<BuildRunnerRoutePath>,
    blockers: Vec<String>,
    residual_unknowns: Vec<String>,
) -> BuildRunnerRouteDecision {
    BuildRunnerRouteDecision {
        schema: BUILD_RUNNER_ROUTE_CONVERGENCE_SCHEMA.to_string(),
        node_id: desired.node_id.clone(),
        state,
        next_path,
        mutates: false,
        blockers: normalize(blockers),
        residual_unknowns: normalize(residual_unknowns),
    }
}

fn normalize(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_000_000;

    fn desired() -> BuildRunnerRouteDesiredState {
        BuildRunnerRouteDesiredState {
            node_id: "cynic-builder".to_string(),
            provider: "proxmox".to_string(),
            resource_id: "lxc/105".to_string(),
            role: "build_runner".to_string(),
            source_commit: "a".repeat(40),
        }
    }

    fn observed() -> BuildRunnerRouteObservation {
        BuildRunnerRouteObservation {
            provider: BuildRunnerProviderRouteObservation {
                status: "observed".to_string(),
                node_id: "cynic-builder".to_string(),
                provider: "proxmox".to_string(),
                resource_id: "lxc/105".to_string(),
                observed_at_unix_ms: NOW,
                evidence_refs: vec!["provider:lxc/105".to_string()],
            },
            ssh_trust: BuildRunnerSshTrustRouteObservation {
                status: "trusted_match".to_string(),
                observed_at_unix_ms: NOW,
                evidence_refs: vec!["ssh:trusted_match".to_string()],
            },
            seed: BuildRunnerSeedRouteObservation {
                status: "proved".to_string(),
                node_id: "cynic-builder".to_string(),
                source_commit: "a".repeat(40),
                observed_at_unix_ms: NOW,
                evidence_refs: vec!["seed:proof".to_string()],
            },
            builder_runtime: Some(BuildRunnerObservation {
                schema: super::super::BUILD_RUNNER_OBSERVATION_SCHEMA.to_string(),
                status: "observed".to_string(),
                node_id: "cynic-builder".to_string(),
                hostname: "cynic-builder".to_string(),
                declared_role: Some("build_runner".to_string()),
                identity_match: SecurityEvidenceState::Proven,
                observed_at_unix_ms: NOW,
                registry_path: "/etc/cynic/registry.json".to_string(),
                evidence: serde_json::json!({"build_runner": true}),
                evidence_refs: vec!["builder:runtime".to_string()],
            }),
            coordination: BuildRunnerCoordinationRouteObservation {
                status: "stable".to_string(),
                observed_at_unix_ms: NOW,
                evidence_refs: vec!["coordination:lease".to_string()],
            },
        }
    }

    #[test]
    fn only_all_fresh_proofs_authorize_runtime_delivery() {
        let decision = reconcile_build_runner_route(&desired(), &observed(), NOW);
        assert_eq!(decision.state, BuildRunnerRouteState::RuntimeFenced);
        assert_eq!(
            decision.next_path,
            Some(BuildRunnerRoutePath::RuntimeDelivery)
        );
        assert!(!decision.mutates);
        assert!(decision.blockers.is_empty());
    }

    #[test]
    fn provider_proof_without_ssh_keeps_only_recovery_path() {
        let mut report = observed();
        report.ssh_trust.status = "not_observed".to_string();
        let decision = reconcile_build_runner_route(&desired(), &report, NOW);
        assert_eq!(decision.state, BuildRunnerRouteState::ProviderObserved);
        assert_eq!(
            decision.next_path,
            Some(BuildRunnerRoutePath::ProxmoxRecovery)
        );
        assert!(
            decision
                .residual_unknowns
                .contains(&"build_runner_ssh_trust_unproven_or_stale".to_string())
        );
    }

    #[test]
    fn admitted_seed_has_no_runtime_delivery_authority() {
        let mut report = observed();
        report.seed.status = "admitted".to_string();
        let decision = reconcile_build_runner_route(&desired(), &report, NOW);
        assert_eq!(decision.state, BuildRunnerRouteState::SeedAdmitted);
        assert_eq!(decision.next_path, Some(BuildRunnerRoutePath::SshSeed));
    }

    #[test]
    fn proved_seed_without_builder_runtime_remains_seed_only() {
        let mut report = observed();
        report.builder_runtime = None;
        let decision = reconcile_build_runner_route(&desired(), &report, NOW);
        assert_eq!(decision.state, BuildRunnerRouteState::SeedProved);
        assert_eq!(decision.next_path, Some(BuildRunnerRoutePath::SshSeed));
    }

    #[test]
    fn unstable_coordination_never_promotes_a_healthy_builder() {
        let mut report = observed();
        report.coordination.status = "incompatible".to_string();
        let decision = reconcile_build_runner_route(&desired(), &report, NOW);
        assert_eq!(
            decision.state,
            BuildRunnerRouteState::BuilderRuntimeObserved
        );
        assert_eq!(decision.next_path, None);
        assert!(
            decision
                .residual_unknowns
                .contains(&"build_runner_coordination_unproven_or_unstable".to_string())
        );
    }

    #[test]
    fn trust_refutation_blocks_all_paths() {
        let mut report = observed();
        report.ssh_trust.status = "trust_refuted".to_string();
        let decision = reconcile_build_runner_route(&desired(), &report, NOW);
        assert_eq!(decision.state, BuildRunnerRouteState::Blocked);
        assert_eq!(decision.next_path, None);
        assert_eq!(
            decision.blockers,
            vec!["build_runner_ssh_trust_refuted".to_string()]
        );
    }

    #[test]
    fn stale_ssh_proof_cannot_authorize_seed_or_delivery() {
        let mut report = observed();
        report.ssh_trust.observed_at_unix_ms = NOW - BUILD_RUNNER_ROUTE_EVIDENCE_MAX_AGE_MS - 1;
        let decision = reconcile_build_runner_route(&desired(), &report, NOW);
        assert_eq!(decision.state, BuildRunnerRouteState::ProviderObserved);
        assert_eq!(
            decision.next_path,
            Some(BuildRunnerRoutePath::ProxmoxRecovery)
        );
    }

    #[test]
    fn provider_identity_mismatch_is_a_hard_blocker() {
        let mut report = observed();
        report.provider.resource_id = "lxc/999".to_string();
        let decision = reconcile_build_runner_route(&desired(), &report, NOW);
        assert_eq!(decision.state, BuildRunnerRouteState::Blocked);
        assert!(
            decision
                .blockers
                .contains(&"build_runner_provider_identity_mismatch".to_string())
        );
    }

    #[test]
    fn seed_commit_mismatch_blocks_route_promotion() {
        let mut report = observed();
        report.seed.source_commit = "b".repeat(40);
        let decision = reconcile_build_runner_route(&desired(), &report, NOW);
        assert_eq!(decision.state, BuildRunnerRouteState::Blocked);
        assert!(
            decision
                .blockers
                .contains(&"build_runner_seed_source_commit_mismatch".to_string())
        );
    }
}
