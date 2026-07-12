use serde::Serialize;

use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::domain::shared::SurfaceStatus;
use crate::host_bootstrap_evidence::{
    HostBootstrapEvidenceReport, observe_host_bootstrap_evidence_with_observer,
};
use crate::ports::{
    HostPackageObservationPort, HostPrincipalObservationPort, HostPrivilegeObservationPort,
    HostServiceObservationPort,
};
use crate::profiles::HostProfile;
use crate::registry::HostProfileResolution;
use crate::runtime_harness::{RuntimeHarnessContext, conformance_context};
use crate::runtime_proof::host_identity_runtime_proof_bundle;
use crate::usecases::{HostObservationReports, observe_host_with_resolution};

#[derive(Debug, Serialize)]
pub struct HostConformanceReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub context: RuntimeHarnessContext,
    pub host_profile: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub gates: Vec<HostConformanceGate>,
    pub observed: HostObservationReports,
    pub bootstrap_evidence: HostBootstrapEvidenceReport,
    pub proof_bundle: cynic_protocol::RuntimeProofBundle,
    pub rules: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct HostConformanceGate {
    pub id: &'static str,
    pub status: &'static str,
    pub detail: String,
}

pub fn observe_host_conformance(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
) -> Result<HostConformanceReport, Box<dyn std::error::Error>> {
    observe_host_conformance_with_observer(profile, resolution, &LocalHostObservationAdapter)
}

pub fn observe_host_conformance_with_observer(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    observed_port: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
     ),
) -> Result<HostConformanceReport, Box<dyn std::error::Error>> {
    let observed = observe_host_with_resolution(&profile, observed_port, resolution.clone());
    let bootstrap_evidence =
        observe_host_bootstrap_evidence_with_observer(profile, resolution.clone(), observed_port)?;
    let proof_bundle = host_identity_runtime_proof_bundle(&observed);
    let context = conformance_context();
    let gates = conformance_gates(&context, &observed, &bootstrap_evidence, &proof_bundle);
    let status = if gates.iter().all(|gate| gate.status == "satisfied") {
        "qualified"
    } else {
        "blocked"
    };

    Ok(HostConformanceReport {
        schema: "cynic.host.conformance.v1",
        status,
        mutates: false,
        context,
        host_profile: profile.name(),
        host_resolution: resolution,
        gates,
        observed,
        bootstrap_evidence,
        proof_bundle,
        rules: vec![
            "conformance observes host state and emits proof; it must not apply mutations",
            "a node is qualified only when host identity proof is satisfied",
            "a node is qualified only when its declared bootstrap target stage is reached",
            "network conformance must be layered separately after host conformance",
        ],
        next_actions: next_actions(status),
    })
}

fn conformance_gates(
    context: &RuntimeHarnessContext,
    observed: &HostObservationReports,
    bootstrap_evidence: &HostBootstrapEvidenceReport,
    proof_bundle: &cynic_protocol::RuntimeProofBundle,
) -> Vec<HostConformanceGate> {
    vec![
        HostConformanceGate {
            id: "harness_is_conformance_gate",
            status: if context.mutates_host_network {
                "blocked"
            } else {
                "satisfied"
            },
            detail: format!(
                "authority={:?} mutates_host_network={}",
                context.authority, context.mutates_host_network
            ),
        },
        HostConformanceGate {
            id: "host_identity_observed",
            status: if proof_bundle.status == "satisfied" {
                "satisfied"
            } else {
                "blocked"
            },
            detail: format!("runtime_proof_bundle_status={}", proof_bundle.status),
        },
        HostConformanceGate {
            id: "bootstrap_target_reached",
            status: if bootstrap_evidence.target_satisfied {
                "satisfied"
            } else {
                "blocked"
            },
            detail: format!(
                "target_stage={} reached_stage={}",
                bootstrap_evidence.target_stage, bootstrap_evidence.reached_stage
            ),
        },
        HostConformanceGate {
            id: "service_surface_observed",
            status: surface_gate_status(observed.services.status),
            detail: format!(
                "missing_units={} disabled_units={} inactive_units={}",
                observed.services.missing_units.len(),
                observed.services.disabled_units.len(),
                observed.services.inactive_units.len()
            ),
        },
    ]
}

fn surface_gate_status(status: SurfaceStatus) -> &'static str {
    if status == SurfaceStatus::Active {
        "satisfied"
    } else {
        "blocked"
    }
}

fn next_actions(status: &str) -> Vec<&'static str> {
    if status == "qualified" {
        vec!["promote candidate node to network conformance"]
    } else {
        vec![
            "inspect blocked gates",
            "run cynic bootstrap host plan for non-mutating remediation plan",
            "do not promote to staging/canary until conformance is qualified",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::packages::PackageObservedState;
    use crate::domain::principal::PrincipalObservedState;
    use crate::domain::privilege::PrivilegeObservedState;
    use crate::domain::services::{ServiceObservedState, ServiceObservedUnit};

    struct ReadyObserver;

    impl HostPrincipalObservationPort for ReadyObserver {
        fn observe_principal(
            &self,
            policy: &crate::domain::principal::PrincipalPolicy,
        ) -> PrincipalObservedState {
            PrincipalObservedState {
                user_present: true,
                group_present: true,
                observed_home_dir: Some(policy.home_dir.clone()),
                present_state_dirs: policy.state_dirs.clone(),
            }
        }
    }

    impl HostPrivilegeObservationPort for ReadyObserver {
        fn observe_privilege(
            &self,
            _policy: &crate::domain::privilege::PrivilegePolicy,
        ) -> PrivilegeObservedState {
            PrivilegeObservedState {
                helper_dir_present: true,
                sudoers_projection_present: true,
                denial_proof_ready: true,
            }
        }
    }

    impl HostPackageObservationPort for ReadyObserver {
        fn observe_packages(
            &self,
            policy: &crate::domain::packages::PackagePolicy,
        ) -> PackageObservedState {
            PackageObservedState {
                installed_packages: policy.bootstrap_packages.clone(),
            }
        }
    }

    impl HostServiceObservationPort for ReadyObserver {
        fn observe_services(
            &self,
            policy: &crate::domain::services::ServicePolicy,
        ) -> ServiceObservedState {
            ServiceObservedState {
                units: policy
                    .units
                    .iter()
                    .map(|unit| ServiceObservedUnit {
                        unit_name: unit.unit_name.clone(),
                        present: true,
                        enabled: true,
                        active: true,
                    })
                    .collect(),
            }
        }
    }

    struct MissingServiceObserver;

    impl HostPrincipalObservationPort for MissingServiceObserver {
        fn observe_principal(
            &self,
            policy: &crate::domain::principal::PrincipalPolicy,
        ) -> PrincipalObservedState {
            ReadyObserver.observe_principal(policy)
        }
    }

    impl HostPrivilegeObservationPort for MissingServiceObserver {
        fn observe_privilege(
            &self,
            policy: &crate::domain::privilege::PrivilegePolicy,
        ) -> PrivilegeObservedState {
            ReadyObserver.observe_privilege(policy)
        }
    }

    impl HostPackageObservationPort for MissingServiceObserver {
        fn observe_packages(
            &self,
            policy: &crate::domain::packages::PackagePolicy,
        ) -> PackageObservedState {
            ReadyObserver.observe_packages(policy)
        }
    }

    impl HostServiceObservationPort for MissingServiceObserver {
        fn observe_services(
            &self,
            policy: &crate::domain::services::ServicePolicy,
        ) -> ServiceObservedState {
            ServiceObservedState {
                units: policy
                    .units
                    .iter()
                    .map(|unit| ServiceObservedUnit {
                        unit_name: unit.unit_name.clone(),
                        present: false,
                        enabled: false,
                        active: false,
                    })
                    .collect(),
            }
        }
    }

    #[test]
    fn host_conformance_qualifies_ready_node_without_mutation() {
        let resolution = HostProfileResolution {
            hostname_observed: "candidate-node".to_string(),
            matched_node: Some("candidate-node".to_string()),
            matched_role: Some("network_node".to_string()),
            canonical_role: None,
            profile_applied: "network-node",
            match_source: "node_name",
            host_bootstrap_target_stage: Some("h5_services_projected".to_string()),
            host_bootstrap_required_domains: vec!["services".to_string()],
            registry_path: Some("infra/registry.json".to_string()),
        };
        let report = observe_host_conformance_with_observer(
            HostProfile::NetworkNode,
            Some(resolution),
            &ReadyObserver,
        )
        .expect("conformance");

        assert_eq!(report.schema, "cynic.host.conformance.v1");
        assert_eq!(report.status, "qualified");
        assert!(!report.mutates);
        assert!(!report.context.mutates_host_network);
        assert_eq!(report.proof_bundle.status, "satisfied");
    }

    #[test]
    fn host_conformance_blocks_when_service_surface_is_missing() {
        let report = observe_host_conformance_with_observer(
            HostProfile::NetworkNode,
            None,
            &MissingServiceObserver,
        )
        .expect("conformance");

        assert_eq!(report.status, "blocked");
        assert!(
            report
                .gates
                .iter()
                .any(|gate| { gate.id == "service_surface_observed" && gate.status == "blocked" })
        );
    }
}
