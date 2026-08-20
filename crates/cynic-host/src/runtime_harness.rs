use serde::{Deserialize, Serialize};

pub use cynic_protocol::HarnessAuthority;

pub const RUNTIME_HARNESS_CONTEXT_SCHEMA: &str = "cynic.runtime_harness.context.v1";
pub const RUNTIME_HARNESS_CATALOG_SCHEMA: &str = "cynic.runtime_harness.catalog.v1";
pub const RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA: &str =
    "cynic.runtime_harness.target_attestation.v2";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeHarnessContext {
    pub schema: &'static str,
    pub kind: RuntimeHarnessKind,
    pub role: &'static str,
    pub authority: HarnessAuthority,
    pub isolation_boundary: &'static str,
    pub target_scope: &'static str,
    pub mutates_host_network: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeHarnessCatalogReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub selected_kind: Option<&'static str>,
    pub contexts: Vec<RuntimeHarnessContext>,
    pub progression: Vec<&'static str>,
    pub rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHarnessTargetAttestation {
    pub schema: String,
    pub harness_kind: RuntimeHarnessKind,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHarnessKind {
    ClosedLocalLab,
    Conformance,
    Staging,
    Canary,
    Production,
}



pub fn closed_local_lab_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::ClosedLocalLab,
        role: "ephemeral proof environment for bootstrap, network, and runtime evidence",
        authority: HarnessAuthority::TestProjection,
        isolation_boundary: "local qemu/libvirt isolated network",
        target_scope: "single-host closed lab first, then N guest nodes",
        mutates_host_network: false,
    }
}

pub fn conformance_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::Conformance,
        role: "portable node qualification suite for execution, bootstrap, security, and observation contracts",
        authority: HarnessAuthority::ConformanceGate,
        isolation_boundary: "bounded local or guest execution with no production authority",
        target_scope: "one candidate node first, then repeatable N-node qualification",
        mutates_host_network: false,
    }
}

pub fn staging_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::Staging,
        role: "production-like release candidate validation before fleet rollout",
        authority: HarnessAuthority::ReleaseCandidate,
        isolation_boundary: "declared staging VM or container boundary",
        target_scope: "release candidate node or staging constellation",
        mutates_host_network: false,
    }
}

pub fn canary_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::Canary,
        role: "bounded real-fleet rollout with observation and rollback requirements",
        authority: HarnessAuthority::FleetCanary,
        isolation_boundary: "declared canary node or fleet subset boundary",
        target_scope: "small declared subset of real runtime nodes",
        mutates_host_network: true,
    }
}

pub fn production_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::Production,
        role: "declared runtime authority with persistent evidence and SLO tracking",
        authority: HarnessAuthority::RuntimeAuthority,
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
    if attestation.harness_kind != context.kind {
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
    selected_kind: Option<&str>,
) -> Result<RuntimeHarnessCatalogReport, String> {
    let selected_kind = selected_kind
        .map(|kind| {
            RuntimeHarnessKind::canonical_name(kind)
                .ok_or_else(|| format!("unknown runtime harness kind: {kind}"))
        })
        .transpose()?;
    let contexts = match selected_kind {
        Some(kind) => {
            let context = runtime_harness_contexts()
                .into_iter()
                .find(|context| context.kind.as_str() == kind)
                .ok_or_else(|| format!("runtime harness kind is not modeled: {kind}"))?;
            vec![context]
        }
        None => runtime_harness_contexts(),
    };

    Ok(RuntimeHarnessCatalogReport {
        schema: RUNTIME_HARNESS_CATALOG_SCHEMA,
        status: "ok",
        selected_kind,
        contexts,
        progression: vec![
            "closed_local_lab",
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

impl RuntimeHarnessKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClosedLocalLab => "closed_local_lab",
            Self::Conformance => "conformance",
            Self::Staging => "staging",
            Self::Canary => "canary",
            Self::Production => "production",
        }
    }

    fn canonical_name(kind: &str) -> Option<&'static str> {
        match kind {
            "closed_local_lab" | "lab" => Some("closed_local_lab"),
            "conformance" => Some("conformance"),
            "staging" => Some("staging"),
            "canary" => Some("canary"),
            "production" => Some("production"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HarnessAuthority, RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
        RuntimeHarnessTargetAttestation, closed_local_lab_context, conformance_context,
        runtime_harness_catalog, validate_runtime_harness_target_attestation,
        validate_runtime_harness_target_attestation_at,
    };

    fn attested_lab_target() -> RuntimeHarnessTargetAttestation {
        let context = closed_local_lab_context();
        RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: RuntimeHarnessKind::ClosedLocalLab,
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
        assert_eq!(context.kind, RuntimeHarnessKind::ClosedLocalLab);
        assert_eq!(context.authority, HarnessAuthority::TestProjection);
        assert!(!context.mutates_host_network);
    }

    #[test]
    fn conformance_is_portable_and_not_production_authority() {
        let context = conformance_context();

        assert_eq!(context.kind, RuntimeHarnessKind::Conformance);
        assert_eq!(context.authority, HarnessAuthority::ConformanceGate);
        assert!(!context.mutates_host_network);
    }

    #[test]
    fn catalog_can_select_conformance() {
        let report = runtime_harness_catalog(Some("conformance")).expect("catalog");

        assert_eq!(report.schema, super::RUNTIME_HARNESS_CATALOG_SCHEMA);
        assert_eq!(report.status, "ok");
        assert_eq!(report.selected_kind, Some("conformance"));
        assert_eq!(report.contexts.len(), 1);
        assert_eq!(report.contexts[0].kind, RuntimeHarnessKind::Conformance);
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
}
