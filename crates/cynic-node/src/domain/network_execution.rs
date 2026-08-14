use cynic_host::runtime_harness::{
    HarnessAuthority, RuntimeHarnessContext, RuntimeHarnessTargetAttestation,
    validate_runtime_harness_target_attestation,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NetworkExecutionBoundary {
    UnverifiedLocalProcess,
    IsolatedHarnessTarget,
    RuntimeHost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct NetworkExecutionViolation {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct NetworkExecutionAdmission {
    pub(crate) schema: &'static str,
    pub(crate) admitted: bool,
    pub(crate) profile: String,
    pub(crate) mutates_network: bool,
    pub(crate) boundary: NetworkExecutionBoundary,
    pub(crate) supported_boundaries: Vec<NetworkExecutionBoundary>,
    pub(crate) harness_kind: Option<&'static str>,
    pub(crate) harness_mutates_host_network: Option<bool>,
    pub(crate) target_attestation: Option<RuntimeHarnessTargetAttestation>,
    pub(crate) violations: Vec<NetworkExecutionViolation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct HarnessTargetAttestationReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) context: RuntimeHarnessContext,
    pub(crate) attestation: RuntimeHarnessTargetAttestation,
    pub(crate) violations: Vec<NetworkExecutionViolation>,
}

pub(crate) fn build_harness_target_attestation_report(
    context: &RuntimeHarnessContext,
    attestation: RuntimeHarnessTargetAttestation,
) -> HarnessTargetAttestationReport {
    let violations = validate_runtime_harness_target_attestation(context, &attestation)
        .into_iter()
        .map(|violation| NetworkExecutionViolation {
            code: violation.code,
            message: violation.message,
        })
        .collect::<Vec<_>>();
    HarnessTargetAttestationReport {
        schema: "cynic.network.harness_target_attestation.v1",
        status: if violations.is_empty() {
            "attested"
        } else {
            "refused"
        },
        mutates: false,
        context: context.clone(),
        attestation,
        violations,
    }
}

pub(crate) fn admit_network_execution(
    profile: &str,
    mutates_network: bool,
    boundary: NetworkExecutionBoundary,
    harness: Option<&RuntimeHarnessContext>,
    target_attestation: Option<&RuntimeHarnessTargetAttestation>,
) -> NetworkExecutionAdmission {
    let mut violations = Vec::new();

    if mutates_network {
        match harness {
            None => violations.push(violation(
                "runtime_harness_required",
                "mutating network execution requires a declared runtime harness",
            )),
            Some(context) => match boundary {
                NetworkExecutionBoundary::UnverifiedLocalProcess => violations.push(violation(
                    "network_execution_boundary_unverified",
                    "a local process cannot prove that it targets the harness rather than the operator host",
                )),
                NetworkExecutionBoundary::IsolatedHarnessTarget => match target_attestation {
                    Some(attestation) => violations.extend(
                        validate_runtime_harness_target_attestation(context, attestation)
                            .into_iter()
                            .map(|violation| NetworkExecutionViolation {
                                code: violation.code,
                                message: violation.message,
                            }),
                    ),
                    None => violations.push(violation(
                        "harness_target_attestation_required",
                        "isolated harness target execution requires a verified target attestation",
                    )),
                },
                NetworkExecutionBoundary::RuntimeHost => {
                    if !context.mutates_host_network {
                        violations.push(violation(
                            "harness_forbids_host_network_mutation",
                            "the selected harness forbids mutation of the runtime host network",
                        ));
                    }
                    if !matches!(
                        context.authority,
                        HarnessAuthority::FleetCanary | HarnessAuthority::RuntimeAuthority
                    ) {
                        violations.push(violation(
                            "runtime_network_authority_required",
                            "runtime host network mutation requires canary or runtime authority",
                        ));
                    }
                }
            },
        }
    }

    NetworkExecutionAdmission {
        schema: "cynic.network_execution.admission.v1",
        admitted: violations.is_empty(),
        profile: profile.to_string(),
        mutates_network,
        boundary,
        supported_boundaries: vec![
            NetworkExecutionBoundary::UnverifiedLocalProcess,
            NetworkExecutionBoundary::IsolatedHarnessTarget,
            NetworkExecutionBoundary::RuntimeHost,
        ],
        harness_kind: harness.map(|context| context.kind.as_str()),
        harness_mutates_host_network: harness.map(|context| context.mutates_host_network),
        target_attestation: target_attestation.cloned(),
        violations,
    }
}

fn violation(code: &'static str, message: &'static str) -> NetworkExecutionViolation {
    NetworkExecutionViolation { code, message }
}

#[cfg(test)]
mod tests {
    use cynic_host::runtime_harness::{
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
        RuntimeHarnessTargetAttestation, closed_local_lab_context, production_context,
    };

    use super::{NetworkExecutionBoundary, admit_network_execution};

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
            proof_refs: vec!["runtime-proof://lab/target/lab-kernel".to_string()],
        }
    }

    #[test]
    fn boolean_intent_cannot_make_a_local_process_an_isolated_target() {
        let harness = closed_local_lab_context();
        let admission = admit_network_execution(
            "mesh-route-install",
            true,
            NetworkExecutionBoundary::UnverifiedLocalProcess,
            Some(&harness),
            None,
        );

        assert!(!admission.admitted);
        assert!(
            admission
                .violations
                .iter()
                .any(|violation| { violation.code == "network_execution_boundary_unverified" })
        );
    }

    #[test]
    fn closed_lab_can_mutate_an_attested_harness_target_not_the_operator_host() {
        let harness = closed_local_lab_context();
        let target_admission = admit_network_execution(
            "mesh-route-install",
            true,
            NetworkExecutionBoundary::IsolatedHarnessTarget,
            Some(&harness),
            Some(&attested_lab_target()),
        );
        let host_admission = admit_network_execution(
            "mesh-route-install",
            true,
            NetworkExecutionBoundary::RuntimeHost,
            Some(&harness),
            None,
        );

        assert!(target_admission.admitted);
        assert!(!host_admission.admitted);
        assert!(
            host_admission
                .violations
                .iter()
                .any(|violation| { violation.code == "harness_forbids_host_network_mutation" })
        );
    }

    #[test]
    fn runtime_host_requires_runtime_authority() {
        let harness = production_context();
        let admission = admit_network_execution(
            "mesh-route-install",
            true,
            NetworkExecutionBoundary::RuntimeHost,
            Some(&harness),
            None,
        );

        assert!(admission.admitted);
    }

    #[test]
    fn observation_does_not_require_mutation_authority() {
        let admission = admit_network_execution(
            "mesh-observe",
            false,
            NetworkExecutionBoundary::UnverifiedLocalProcess,
            None,
            None,
        );

        assert!(admission.admitted);
    }

    #[test]
    fn isolated_boundary_without_attestation_is_refused() {
        let harness = closed_local_lab_context();
        let admission = admit_network_execution(
            "mesh-route-install",
            true,
            NetworkExecutionBoundary::IsolatedHarnessTarget,
            Some(&harness),
            None,
        );

        assert!(!admission.admitted);
        assert!(
            admission
                .violations
                .iter()
                .any(|violation| { violation.code == "harness_target_attestation_required" })
        );
    }
}
