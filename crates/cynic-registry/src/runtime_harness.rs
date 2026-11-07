use cynic_protocol::build::{
    BuildRunnerRouteDecision, BuildRunnerRouteDesiredState, BuildRunnerRouteObservation,
    BuildRunnerRouteState, reconcile_build_runner_route,
};
use serde::{Deserialize, Serialize};

pub use cynic_protocol::build::Authority;
pub use cynic_protocol::deployment_tier::DeploymentTier;

pub const RUNTIME_HARNESS_CONTEXT_SCHEMA: &str = "cynic.runtime_harness.context.v1";
pub const RUNTIME_HARNESS_CATALOG_SCHEMA: &str = "cynic.runtime_harness.catalog.v1";
pub const RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA: &str =
    "cynic.runtime_harness.target_attestation.v2";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeHarnessContext {
    pub schema: &'static str,
    pub tier: DeploymentTier,
    pub role: &'static str,
    pub authority: Authority,
    pub isolation_boundary: &'static str,
    pub target_scope: &'static str,
    pub mutates_host_network: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeHarnessCatalogReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub selected_tier: Option<&'static str>,
    pub contexts: Vec<RuntimeHarnessContext>,
    pub progression: Vec<&'static str>,
    pub rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHarnessTargetAttestation {
    pub schema: String,
    pub tier: DeploymentTier,
    pub observed_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub target_id: String,
    pub provider: String,
    pub transport: String,
    pub transport_target: String,
    pub declared_isolation_boundary: String,
    pub observed_target_boundary: String,
    pub target_identity_verified: bool,
    #[serde(default)]
    pub transport_verified: bool,
    pub isolated_network_verified: bool,
    pub operator_host_network_unchanged: bool,
    pub proof_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeHarnessTargetAttestationViolation {
    pub code: &'static str,
    pub message: &'static str,
}

pub fn closed_local_lab_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        tier: DeploymentTier::LocalLab,
        role: "ephemeral proof environment for bootstrap, network, and runtime evidence",
        authority: Authority::TestProjection,
        isolation_boundary: "local qemu/libvirt isolated network",
        target_scope: "single-host closed lab first, then N guest nodes",
        mutates_host_network: false,
    }
}

pub fn conformance_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        tier: DeploymentTier::Conformance,
        role: "portable node qualification suite for execution, bootstrap, security, and observation contracts",
        authority: Authority::ConformanceGate,
        isolation_boundary: "bounded local or guest execution with no production authority",
        target_scope: "one candidate node first, then repeatable N-node qualification",
        mutates_host_network: false,
    }
}

pub fn staging_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        tier: DeploymentTier::Staging,
        role: "production-like release candidate validation before fleet rollout",
        authority: Authority::ReleaseCandidate,
        isolation_boundary: "declared staging VM or container boundary",
        target_scope: "release candidate node or staging constellation",
        mutates_host_network: false,
    }
}

pub fn canary_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        tier: DeploymentTier::Canary,
        role: "bounded real-fleet rollout with observation and rollback requirements",
        authority: Authority::FleetCanary,
        isolation_boundary: "declared canary node or fleet subset boundary",
        target_scope: "small declared subset of real runtime nodes",
        mutates_host_network: true,
    }
}

pub fn production_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        tier: DeploymentTier::Production,
        role: "declared runtime authority with persistent evidence and SLO tracking",
        authority: Authority::RuntimeAuthority,
        isolation_boundary: "production fleet boundary declared by runtime authority",
        target_scope: "declared active organism runtime",
        mutates_host_network: true,
    }
}

pub fn runtime_harness_contexts() -> Vec<RuntimeHarnessContext> {
    vec![
        closed_local_lab_context(),
        conformance_context(),
        staging_context(),
        canary_context(),
        production_context(),
    ]
}

pub fn validate_runtime_harness_target_attestation(
    context: &RuntimeHarnessContext,
    attestation: &RuntimeHarnessTargetAttestation,
) -> Vec<RuntimeHarnessTargetAttestationViolation> {
    let now_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    validate_runtime_harness_target_attestation_at(context, attestation, now_unix_ms)
}

pub fn validate_runtime_harness_target_attestation_at(
    context: &RuntimeHarnessContext,
    attestation: &RuntimeHarnessTargetAttestation,
    now_unix_ms: u64,
) -> Vec<RuntimeHarnessTargetAttestationViolation> {
    let mut violations = Vec::new();
    if attestation.schema != RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA {
        violations.push(target_violation(
            "harness_target_attestation_schema_mismatch",
            "target attestation schema does not match the runtime harness contract",
        ));
    }
    if attestation.tier != context.tier {
        violations.push(target_violation(
            "harness_target_kind_mismatch",
            "target attestation was produced for a different runtime harness kind",
        ));
    }
    if attestation.observed_at_unix_ms == 0
        || attestation.expires_at_unix_ms <= attestation.observed_at_unix_ms
    {
        violations.push(target_violation(
            "harness_target_attestation_window_invalid",
            "target attestation must declare a bounded observation and expiry window",
        ));
    } else if now_unix_ms > attestation.expires_at_unix_ms {
        violations.push(target_violation(
            "harness_target_attestation_expired",
            "target attestation has expired and must be observed again before mutation",
        ));
    }
    if attestation.target_id.trim().is_empty() {
        violations.push(target_violation(
            "harness_target_identity_missing",
            "target attestation must identify the execution target",
        ));
    }
    if attestation.provider.trim().is_empty()
        || attestation.transport.trim().is_empty()
        || attestation.transport_target.trim().is_empty()
    {
        violations.push(target_violation(
            "harness_target_adapter_missing",
            "target attestation must identify its provider, transport, and transport target",
        ));
    }
    if attestation.declared_isolation_boundary != context.isolation_boundary {
        violations.push(target_violation(
            "harness_target_isolation_declaration_mismatch",
            "target attestation is not bound to the selected harness isolation declaration",
        ));
    }
    if attestation.observed_target_boundary.trim().is_empty()
        || !attestation.isolated_network_verified
    {
        violations.push(target_violation(
            "harness_target_isolation_unverified",
            "target adapter did not prove an isolated target network boundary",
        ));
    }
    if !attestation.target_identity_verified {
        violations.push(target_violation(
            "harness_target_identity_unverified",
            "target adapter did not prove the expected guest identity",
        ));
    }
    if !attestation.transport_verified {
        violations.push(target_violation(
            "harness_target_transport_unverified",
            "target adapter did not prove the declared execution transport",
        ));
    }
    if !attestation.operator_host_network_unchanged {
        violations.push(target_violation(
            "operator_host_network_safety_unverified",
            "target adapter did not prove that operator host network state remained unchanged",
        ));
    }
    if attestation.proof_refs.is_empty()
        || attestation
            .proof_refs
            .iter()
            .any(|proof_ref| proof_ref.trim().is_empty())
    {
        violations.push(target_violation(
            "harness_target_proof_missing",
            "target attestation must reference non-empty machine-readable proof evidence",
        ));
    }
    violations
}

fn target_violation(
    code: &'static str,
    message: &'static str,
) -> RuntimeHarnessTargetAttestationViolation {
    RuntimeHarnessTargetAttestationViolation { code, message }
}

pub fn runtime_harness_catalog(
    selected_tier: Option<&str>,
) -> Result<RuntimeHarnessCatalogReport, String> {
    let selected_tier = selected_tier
        .map(|tier| {
            DeploymentTier::parse_flexible(tier)
                .map(|t| t.as_str())
                .ok_or_else(|| format!("unknown runtime harness tier: {tier}"))
        })
        .transpose()?;
    let contexts = match selected_tier {
        Some(tier) => {
            let context = runtime_harness_contexts()
                .into_iter()
                .find(|context| context.tier.as_str() == tier)
                .ok_or_else(|| format!("runtime harness tier is not modeled: {tier}"))?;
            vec![context]
        }
        None => runtime_harness_contexts(),
    };

    Ok(RuntimeHarnessCatalogReport {
        schema: RUNTIME_HARNESS_CATALOG_SCHEMA,
        status: "ok",
        selected_tier,
        contexts,
        progression: vec![
            "local_lab",
            "conformance",
            "staging",
            "canary",
            "production",
        ],
        rules: vec![
            "a harness is an execution membrane, not the capability itself",
            "lab must remain a closed local test projection",
            "conformance is the portable node qualification boundary",
            "canary and production may mutate real host/network state only after explicit authority",
        ],
    })
}

/// Project the canonical BuildRunner route decision into a runtime harness
/// observation report.
///
/// This is a pure projection over `reconcile_build_runner_route`. It never
/// performs I/O, never executes argv, and never mutates the substrate. It
/// consumes only already-observed evidence and returns a machine-readable
/// read-back so operators/agents can inspect the route without invoking the
/// builder transport or submission surface.
///
/// The report carries the canonical [`BuildRunnerRouteDecision`] verbatim: it
/// adds no second mutation verdict. `BuildRunnerRouteDecision::mutates` is
/// `false` by construction ("a route decision never executes"), and this
/// projection preserves that single source of decision truth instead of
/// recomputing a parallel `mutates` flag from `next_path`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeHarnessBuildRunnerRouteReport {
    pub schema: &'static str,
    /// Verbatim projection of the route state — no second decision layer.
    pub status: &'static str,
    pub route: BuildRunnerRouteDecision,
    pub tier: &'static str,
}

pub const RUNTIME_HARNESS_BUILD_RUNNER_ROUTE_SCHEMA: &str =
    "cynic.runtime_harness.build_runner_route.v1";

pub fn observe_build_runner_route(
    desired: &BuildRunnerRouteDesiredState,
    observed: &BuildRunnerRouteObservation,
    now_unix_ms: u64,
    tier: DeploymentTier,
) -> RuntimeHarnessBuildRunnerRouteReport {
    let route = reconcile_build_runner_route(desired, observed, now_unix_ms);
    let status = match route.state {
        BuildRunnerRouteState::RuntimeFenced => "runtime_fenced",
        BuildRunnerRouteState::SeedAdmitted | BuildRunnerRouteState::SeedProved => "seed_ready",
        BuildRunnerRouteState::Blocked => "blocked_hard",
        BuildRunnerRouteState::Unroutable
        | BuildRunnerRouteState::ProviderObserved
        | BuildRunnerRouteState::SshTrustVerified
        | BuildRunnerRouteState::BuilderRuntimeObserved => "converging",
    };
    RuntimeHarnessBuildRunnerRouteReport {
        schema: RUNTIME_HARNESS_BUILD_RUNNER_ROUTE_SCHEMA,
        status,
        route,
        tier: tier.as_str(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessTargetAttestationRequest {
    pub target_id: String,
    pub provider: String,
    pub transport: String,
    pub transport_target: String,
    pub expected_network_boundary: String,
}

pub trait HarnessTargetAttestationPort {
    fn attest_target(
        &self,
        context: &RuntimeHarnessContext,
        request: &HarnessTargetAttestationRequest,
    ) -> Result<RuntimeHarnessTargetAttestation, String>;
}

pub trait HarnessTargetAttestationReaderPort {
    fn read_attestation(&self, path: &std::path::Path) -> Result<RuntimeHarnessTargetAttestation, String>;
}

pub struct LocalHarnessTargetAttestationReader;

#[derive(serde::Deserialize)]
struct AttestationEnvelope {
    attestation: RuntimeHarnessTargetAttestation,
}

impl HarnessTargetAttestationReaderPort for LocalHarnessTargetAttestationReader {
    fn read_attestation(&self, path: &std::path::Path) -> Result<RuntimeHarnessTargetAttestation, String> {
        let bytes = std::fs::read(path).map_err(|error| {
            format!(
                "failed to read harness target attestation {}: {error}",
                path.display()
            )
        })?;
        if let Ok(attestation) = serde_json::from_slice(&bytes) {
            return Ok(attestation);
        }
        serde_json::from_slice::<AttestationEnvelope>(&bytes)
            .map(|envelope| envelope.attestation)
            .map_err(|error| {
                format!(
                    "invalid harness target attestation JSON {}: {error}",
                    path.display()
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Authority, RUNTIME_HARNESS_BUILD_RUNNER_ROUTE_SCHEMA,
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, DeploymentTier,
        RuntimeHarnessTargetAttestation, closed_local_lab_context, conformance_context,
        observe_build_runner_route, runtime_harness_catalog,
        validate_runtime_harness_target_attestation,
        validate_runtime_harness_target_attestation_at,
        HarnessTargetAttestationReaderPort, LocalHarnessTargetAttestationReader,
    };
    use cynic_protocol::build::BuildRunnerObservation;
    use cynic_protocol::build::{
        BUILD_RUNNER_OBSERVATION_SCHEMA, BuildRunnerCoordinationRouteObservation,
        BuildRunnerProviderRouteObservation, BuildRunnerRouteObservation, BuildRunnerRoutePath,
        BuildRunnerRouteState, BuildRunnerSeedRouteObservation,
        BuildRunnerSshTrustRouteObservation,
    };
    use cynic_protocol::SecurityEvidenceState;

    fn attested_lab_target() -> RuntimeHarnessTargetAttestation {
        let context = closed_local_lab_context();
        RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            tier: DeploymentTier::LocalLab,
            observed_at_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            target_id: "lab-kernel".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "ssh".to_string(),
            transport_target: "cynic@172.30.41.10".to_string(),
            declared_isolation_boundary: context.isolation_boundary.to_string(),
            observed_target_boundary: "libvirt-network:cynic-lab-isolated".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec![
                "runtime-proof://lab/network/cynic-lab-isolated".to_string(),
                "runtime-proof://lab/guest/lab-kernel/identity".to_string(),
            ],
        }
    }

    #[test]
    fn closed_local_lab_is_not_runtime_authority() {
        let context = closed_local_lab_context();

        assert_eq!(context.schema, "cynic.runtime_harness.context.v1");
        assert_eq!(context.tier, DeploymentTier::LocalLab);
        assert_eq!(context.authority, Authority::TestProjection);
        assert!(!context.mutates_host_network);
    }

    #[test]
    fn conformance_is_portable_and_not_production_authority() {
        let context = conformance_context();

        assert_eq!(context.tier, DeploymentTier::Conformance);
        assert_eq!(context.authority, Authority::ConformanceGate);
        assert!(!context.mutates_host_network);
    }

    #[test]
    fn catalog_can_select_conformance() {
        let report = runtime_harness_catalog(Some("conformance")).expect("catalog");

        assert_eq!(report.schema, super::RUNTIME_HARNESS_CATALOG_SCHEMA);
        assert_eq!(report.status, "ok");
        assert_eq!(report.selected_tier, Some("conformance"));
        assert_eq!(report.contexts.len(), 1);
        assert_eq!(report.contexts[0].tier, DeploymentTier::Conformance);
    }

    #[test]
    fn target_attestation_binds_guest_identity_and_isolation_to_harness() {
        let context = closed_local_lab_context();
        let attestation = attested_lab_target();

        assert!(validate_runtime_harness_target_attestation(&context, &attestation).is_empty());
    }

    #[test]
    fn target_attestation_fails_closed_without_operator_network_proof() {
        let context = closed_local_lab_context();
        let mut attestation = attested_lab_target();
        attestation.operator_host_network_unchanged = false;

        let violations = validate_runtime_harness_target_attestation(&context, &attestation);

        assert!(
            violations
                .iter()
                .any(|violation| { violation.code == "operator_host_network_safety_unverified" })
        );
    }

    #[test]
    fn target_attestation_expires_before_it_can_authorize_mutation() {
        let context = closed_local_lab_context();
        let mut attestation = attested_lab_target();
        attestation.observed_at_unix_ms = 1_000;
        attestation.expires_at_unix_ms = 2_000;

        let violations =
            validate_runtime_harness_target_attestation_at(&context, &attestation, 2_001);

        assert!(
            violations
                .iter()
                .any(|violation| violation.code == "harness_target_attestation_expired")
        );
    }

    #[test]
    fn declared_transport_without_observation_cannot_authorize_mutation() {
        let context = closed_local_lab_context();
        let mut attestation = attested_lab_target();
        attestation.transport_verified = false;

        let violations = validate_runtime_harness_target_attestation(&context, &attestation);

        assert!(
            violations
                .iter()
                .any(|violation| violation.code == "harness_target_transport_unverified")
        );
    }

    // --- BuildRunner route projection (pure, no I/O) -------------------------

    const ROUTE_NOW: u64 = 1_000_000;

    fn route_desired() -> cynic_protocol::build::BuildRunnerRouteDesiredState {
        cynic_protocol::build::BuildRunnerRouteDesiredState {
            node_id: "cynic-builder".to_string(),
            provider: "proxmox".to_string(),
            resource_id: "lxc/105".to_string(),
            role: "build_runner".to_string(),
            source_commit: "a".repeat(40),
        }
    }

    fn fenced_runtime() -> BuildRunnerObservation {
        BuildRunnerObservation {
            schema: BUILD_RUNNER_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: "cynic-builder".to_string(),
            hostname: "cynic-builder".to_string(),
            declared_role: Some("build_runner".to_string()),
            identity_match: SecurityEvidenceState::Proven,
            observed_at_unix_ms: ROUTE_NOW,
            registry_path: "/etc/cynic/registry.json".to_string(),
            evidence: serde_json::json!({}),
            evidence_refs: vec!["node-proof".to_string()],
        }
    }

    fn fully_fenced_observation() -> BuildRunnerRouteObservation {
        BuildRunnerRouteObservation {
            provider: BuildRunnerProviderRouteObservation {
                status: "observed".to_string(),
                node_id: "cynic-builder".to_string(),
                provider: "proxmox".to_string(),
                resource_id: "lxc/105".to_string(),
                observed_at_unix_ms: ROUTE_NOW,
                evidence_refs: vec!["proxmox:lxc/105".to_string()],
            },
            ssh_trust: BuildRunnerSshTrustRouteObservation {
                status: "trusted_match".to_string(),
                observed_at_unix_ms: ROUTE_NOW,
                evidence_refs: vec!["ssh-trust:cynic-builder:trusted_match".to_string()],
            },
            seed: BuildRunnerSeedRouteObservation {
                status: "proved".to_string(),
                node_id: "cynic-builder".to_string(),
                source_commit: "a".repeat(40),
                observed_at_unix_ms: ROUTE_NOW,
                evidence_refs: vec!["seed-proof".to_string()],
            },
            builder_runtime: Some(fenced_runtime()),
            coordination: BuildRunnerCoordinationRouteObservation {
                status: "stable".to_string(),
                observed_at_unix_ms: ROUTE_NOW,
                evidence_refs: vec!["coordination-proof".to_string()],
            },
        }
    }

    #[test]
    fn full_fresh_evidence_projects_runtime_fenced_with_delivery_path() {
        let report = observe_build_runner_route(
            &route_desired(),
            &fully_fenced_observation(),
            ROUTE_NOW,
            DeploymentTier::LocalLab,
        );

        assert_eq!(report.schema, RUNTIME_HARNESS_BUILD_RUNNER_ROUTE_SCHEMA);
        assert_eq!(report.status, "runtime_fenced");
        assert_eq!(report.route.state, BuildRunnerRouteState::RuntimeFenced);
        assert_eq!(
            report.route.next_path,
            Some(BuildRunnerRoutePath::RuntimeDelivery)
        );
        assert!(report.route.blockers.is_empty());
        assert!(report.route.residual_unknowns.is_empty());
        // The canonical decision never mutates; the harness projection must not
        // invent a second mutation verdict.
        assert!(!report.route.mutates);
    }

    #[test]
    fn provider_only_projects_converging_with_recovery_path() {
        let observed = BuildRunnerRouteObservation {
            provider: BuildRunnerProviderRouteObservation {
                status: "observed".to_string(),
                node_id: "cynic-builder".to_string(),
                provider: "proxmox".to_string(),
                resource_id: "lxc/105".to_string(),
                observed_at_unix_ms: ROUTE_NOW,
                evidence_refs: vec!["proxmox:lxc/105".to_string()],
            },
            ssh_trust: BuildRunnerSshTrustRouteObservation {
                status: "not_observed".to_string(),
                observed_at_unix_ms: 0,
                evidence_refs: Vec::new(),
            },
            seed: BuildRunnerSeedRouteObservation {
                status: "absent".to_string(),
                node_id: "cynic-builder".to_string(),
                source_commit: "a".repeat(40),
                observed_at_unix_ms: 0,
                evidence_refs: Vec::new(),
            },
            builder_runtime: None,
            coordination: BuildRunnerCoordinationRouteObservation {
                status: "unknown".to_string(),
                observed_at_unix_ms: 0,
                evidence_refs: Vec::new(),
            },
        };

        let report = observe_build_runner_route(
            &route_desired(),
            &observed,
            ROUTE_NOW,
            DeploymentTier::LocalLab,
        );

        assert_eq!(report.status, "converging");
        assert_eq!(report.route.state, BuildRunnerRouteState::ProviderObserved);
        assert_eq!(
            report.route.next_path,
            Some(BuildRunnerRoutePath::ProxmoxRecovery)
        );
        assert_eq!(report.route.residual_unknowns.len(), 1);
    }

    #[test]
    fn refuted_ssh_trust_downgrades_to_blocked_hard() {
        let mut observed = fully_fenced_observation();
        observed.ssh_trust.status = "trust_refuted".to_string();

        let report = observe_build_runner_route(
            &route_desired(),
            &observed,
            ROUTE_NOW,
            DeploymentTier::LocalLab,
        );

        assert_eq!(report.status, "blocked_hard");
        assert_eq!(report.route.state, BuildRunnerRouteState::Blocked);
        assert!(report.route.next_path.is_none());
        assert!(
            report
                .route
                .blockers
                .contains(&"build_runner_ssh_trust_refuted".to_string())
        );
    }

    #[test]
    fn stale_provider_evidence_blocks_the_projection() {
        let mut observed = fully_fenced_observation();
        observed.provider.observed_at_unix_ms = ROUTE_NOW - 301_000;

        let report = observe_build_runner_route(
            &route_desired(),
            &observed,
            ROUTE_NOW,
            DeploymentTier::LocalLab,
        );

        assert_eq!(report.status, "blocked_hard");
        assert!(
            report
                .route
                .blockers
                .contains(&"build_runner_provider_observation_stale".to_string())
        );
    }

    #[test]
    fn stale_seed_keeps_seed_path_but_reports_residual_unknown() {
        let mut observed = fully_fenced_observation();
        // 301 s > BUILD_RUNNER_ROUTE_EVIDENCE_MAX_AGE_MS (300 s): stale.
        observed.seed.observed_at_unix_ms = ROUTE_NOW - 301_000;
        observed.seed.status = "admitted".to_string();
        observed.builder_runtime = None;
        observed.coordination.status = "unknown".to_string();

        let report = observe_build_runner_route(
            &route_desired(),
            &observed,
            ROUTE_NOW,
            DeploymentTier::Conformance,
        );

        assert_eq!(report.tier, "conformance");
        assert_eq!(report.status, "converging");
        assert_eq!(report.route.state, BuildRunnerRouteState::SshTrustVerified);
        assert_eq!(report.route.next_path, Some(BuildRunnerRoutePath::SshSeed));
        assert!(
            report
                .route
                .residual_unknowns
                .contains(&"build_runner_seed_unproved_or_stale".to_string())
        );
    }

    #[test]
    fn reads_direct_attestation_json() {
        let temp = tempfile::NamedTempFile::new().expect("tempfile");
        std::fs::write(
            temp.path(),
            serde_json::to_vec(&attestation()).expect("serialize"),
        )
        .expect("write");

        let loaded = LocalHarnessTargetAttestationReader
            .read_attestation(temp.path())
            .expect("attestation");

        assert_eq!(loaded.target_id, "lab-kernel");
    }

    #[test]
    fn reads_attestation_report_envelope() {
        let temp = tempfile::NamedTempFile::new().expect("tempfile");
        let envelope = serde_json::json!({
            "schema": "cynic.network.harness_target_attestation.v1",
            "status": "attested",
            "attestation": attestation(),
        });
        std::fs::write(
            temp.path(),
            serde_json::to_vec(&envelope).expect("serialize"),
        )
        .expect("write");

        let loaded = LocalHarnessTargetAttestationReader
            .read_attestation(temp.path())
            .expect("attestation");

        assert_eq!(loaded.transport, "qemu-guest-agent");
    }

    fn attestation() -> RuntimeHarnessTargetAttestation {
        RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            tier: DeploymentTier::LocalLab,
            observed_at_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            target_id: "lab-kernel".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "qemu-guest-agent".to_string(),
            transport_target: "lab-kernel".to_string(),
            declared_isolation_boundary: closed_local_lab_context().isolation_boundary.to_string(),
            observed_target_boundary: "libvirt-network:cynic-lab-isolated".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec!["sha256:test:proof".to_string()],
        }
    }
}
