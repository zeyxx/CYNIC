use serde::Serialize;

use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::build::BuildProof;
use crate::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus, SurfaceStatus};
use crate::hardware_baseline::{HardwareBaselinePlan, plan_hardware_baseline};
use crate::hardware_conformance::{HardwareConformanceReport, observe_hardware_conformance};
use crate::host_bootstrap_evidence::{
    HostBootstrapEvidenceReport, observe_host_bootstrap_evidence_with_observer,
};
use crate::ports::{
    HostHardwareObservationPort, HostPackageObservationPort, HostPrincipalObservationPort,
    HostPrivilegeObservationPort, HostServiceObservationPort, PreOsPlatformObservationPort,
};
use crate::profiles::HostProfile;
use crate::registry::HostProfileResolution;
use crate::repo_access::current_repo_access_status;
use crate::runtime_harness::{RuntimeHarnessContext, conformance_context};
use crate::runtime_proof::{
    assemble_runtime_proof_bundle, assemble_runtime_proof_persistence_record,
    host_identity_runtime_proof,
};
use crate::usecases::{HostObservationReports, observe_host_with_resolution};
use crate::vertical_conformance::{
    VerticalConformanceReport, evaluate_vertical_conformance, observe_vertical_evidence,
};

#[derive(Debug, Serialize)]
pub struct HostConformanceReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub context: RuntimeHarnessContext,
    pub host_profile: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub gates: Vec<HostConformanceGate>,
    pub verticals: VerticalConformanceReport,
    pub convergence: cynic_protocol::ConvergenceMatrixReport,
    pub observed: HostObservationReports,
    pub bootstrap_evidence: HostBootstrapEvidenceReport,
    pub hardware: HardwareConformanceReport,
    pub hardware_baseline_plan: HardwareBaselinePlan,
    pub pre_os: cynic_protocol::PreOsPlatformObservation,
    pub proof_bundle: cynic_protocol::RuntimeProofBundle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_proof: Option<BuildProof>,
    pub persistence_record: cynic_protocol::RuntimeProofPersistenceRecord,
    pub rules: Vec<&'static str>,
    pub next_actions: Vec<String>,
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
    observe_host_conformance_with_observer_and_build_proof(
        profile,
        resolution,
        &LocalHostObservationAdapter,
        None,
    )
}

pub fn observe_host_conformance_with_observer(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    observed_port: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
         + HostHardwareObservationPort
         + PreOsPlatformObservationPort
     ),
) -> Result<HostConformanceReport, Box<dyn std::error::Error>> {
    observe_host_conformance_with_observer_and_build_proof(profile, resolution, observed_port, None)
}

pub fn observe_host_conformance_with_build_proof(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    build_proof: Option<&BuildProof>,
) -> Result<HostConformanceReport, Box<dyn std::error::Error>> {
    observe_host_conformance_with_observer_and_build_proof(
        profile,
        resolution,
        &LocalHostObservationAdapter,
        build_proof,
    )
}

fn observe_host_conformance_with_observer_and_build_proof(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    observed_port: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
         + HostHardwareObservationPort
         + PreOsPlatformObservationPort
     ),
    build_proof: Option<&BuildProof>,
) -> Result<HostConformanceReport, Box<dyn std::error::Error>> {
    let observed = observe_host_with_resolution(&profile, observed_port, resolution.clone());
    let bootstrap_evidence =
        observe_host_bootstrap_evidence_with_observer(profile, resolution.clone(), observed_port)?;
    let identity_proof = host_identity_runtime_proof(&observed);
    let context = conformance_context();
    let hardware = observe_hardware_conformance(observed_port);
    let hardware_subject = resolution
        .as_ref()
        .map(|value| value.hostname_observed.as_str())
        .unwrap_or("unresolved-host");
    let hardware_baseline_path =
        crate::runtime_layout::current_runtime_layout().hardware_baseline_path();
    let hardware_baseline_plan = plan_hardware_baseline(
        hardware_subject,
        "cynic-conformance",
        &hardware_baseline_path,
        &hardware,
    );
    let pre_os = observed_port.observe_pre_os_platform();
    let gates = conformance_gates(
        &context,
        &observed,
        &bootstrap_evidence,
        &cynic_protocol::RuntimeProofBundle {
            schema: cynic_protocol::RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
            status: identity_proof.status.clone(),
            proofs: vec![identity_proof.clone()],
        },
        &hardware,
    );
    let vertical_evidence = observe_vertical_evidence(build_proof);
    let verticals = evaluate_vertical_conformance(&context, &gates, &vertical_evidence);
    let subject = identity_proof.subject.clone();
    let convergence =
        crate::vertical_conformance::convergence_matrix_for_subject(&subject, &verticals);
    let proof_bundle = assemble_runtime_proof_bundle(identity_proof, &verticals);
    let persistence_record =
        assemble_runtime_proof_persistence_record(&proof_bundle, &verticals, build_proof);
    let status = if gates.iter().all(|gate| gate.status == "satisfied") {
        "qualified"
    } else {
        "blocked"
    };
    let next_command = bootstrap_evidence.next_command.clone();

    Ok(HostConformanceReport {
        schema: "cynic.host.conformance.v1",
        status,
        mutates: false,
        context,
        host_profile: profile.name(),
        host_resolution: resolution,
        lifecycle_status: bootstrap_evidence.lifecycle_status,
        lifecycle_disposition: bootstrap_evidence.lifecycle_disposition,
        next_boundary_kind: bootstrap_evidence.next_boundary_kind,
        next_boundary_reason: bootstrap_evidence.next_boundary_reason.clone(),
        next_command: next_command.clone(),
        gates,
        verticals,
        convergence,
        observed,
        bootstrap_evidence,
        hardware,
        hardware_baseline_plan,
        pre_os,
        proof_bundle,
        build_proof: build_proof.cloned(),
        persistence_record,
        rules: vec![
            "conformance observes host state and emits proof; it must not apply mutations",
            "a node is qualified only when host identity proof is satisfied",
            "a node is qualified only when its declared bootstrap target stage is reached",
            "network conformance must be layered separately after host conformance",
            "hardware conformance requires offline evidence where the running OS cannot observe firmware truth",
            "pre-OS control is selected from observed capabilities; no operating system or firmware provider is canonical",
        ],
        next_actions: next_actions(status, next_command.as_deref()),
    })
}

fn conformance_gates(
    context: &RuntimeHarnessContext,
    observed: &HostObservationReports,
    bootstrap_evidence: &HostBootstrapEvidenceReport,
    proof_bundle: &cynic_protocol::RuntimeProofBundle,
    hardware: &HardwareConformanceReport,
) -> Vec<HostConformanceGate> {
    let repo_access = current_repo_access_status();
    let forgejo = repo_access
        .sovereign_remotes
        .iter()
        .find(|remote| remote.id == "forgejo");
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
        HostConformanceGate {
            id: "hardware_stability_qualified",
            status: if hardware.status == "qualified" {
                "satisfied"
            } else {
                "blocked"
            },
            detail: format!(
                "hardware_status={} residual_unknowns={:?}",
                hardware.status, hardware.residual_unknowns
            ),
        },
        HostConformanceGate {
            id: "sovereign_git_service_observed",
            status: if forgejo.is_some_and(|remote| remote.service_health_status == "reachable") {
                "satisfied"
            } else {
                "blocked"
            },
            detail: match forgejo {
                Some(remote) => format!(
                    "forgejo service_health={} resolution={} tcp={} repo_auth={}",
                    remote.service_health_status,
                    remote.host_resolution_status,
                    remote.tcp_reachability_status,
                    remote.repo_auth_readiness
                ),
                None => "forgejo sovereign remote missing from repo-access authority".to_string(),
            },
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

fn next_actions(status: &str, canonical_next_command: Option<&str>) -> Vec<String> {
    if status == "qualified" {
        vec!["promote candidate node to network conformance".to_string()]
    } else {
        let mut actions = vec![
            "inspect blocked gates".to_string(),
            "run cynic bootstrap host plan for non-mutating remediation plan".to_string(),
            "do not promote to staging/canary until conformance is qualified".to_string(),
        ];
        if let Some(command) = canonical_next_command {
            actions.push(command.to_string());
        }
        actions.sort();
        actions.dedup();
        actions
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
                owned_paths: policy.managed_paths(),
                ownership_unknown_paths: Vec::new(),
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

    impl HostHardwareObservationPort for ReadyObserver {
        fn observe_hardware(&self) -> crate::hardware_conformance::HardwareObservation {
            let inventory = ready_hardware_inventory();
            crate::hardware_conformance::HardwareObservation {
                baseline_digest: cynic_protocol::digest_json(&inventory).ok(),
                inventory,
                firmware_version: Some("test-firmware".to_string()),
                max_temperature_millicelsius: Some(40_000),
                corrected_memory_errors: Some(0),
                uncorrected_memory_errors: Some(0),
                storage_device_count: 1,
                offline_memory_test: crate::hardware_conformance::EvidenceState::Satisfied,
                memory_tuning: crate::hardware_conformance::EvidenceState::Satisfied,
                runtime_tuning_overrides: Vec::new(),
            }
        }
    }

    fn ready_hardware_inventory() -> crate::hardware_conformance::HardwareInventory {
        crate::hardware_conformance::HardwareInventory {
            board_name: Some("fixture".to_string()),
            cpu_model: Some("fixture".to_string()),
            devices: vec![crate::hardware_conformance::HardwareDevice {
                bus: "fixture".to_string(),
                address: "0".to_string(),
                class: None,
                vendor_id: None,
                device_id: None,
                model: None,
                firmware: None,
                driver: None,
            }],
            ..Default::default()
        }
    }

    impl PreOsPlatformObservationPort for ReadyObserver {
        fn observe_pre_os_platform(&self) -> cynic_protocol::PreOsPlatformObservation {
            cynic_protocol::PreOsPlatformObservation {
                schema: cynic_protocol::PRE_OS_PLATFORM_OBSERVATION_SCHEMA.to_string(),
                platform: "test".to_string(),
                controls: vec![cynic_protocol::PreOsControlKind::Hypervisor],
                boot_once: cynic_protocol::SecurityEvidenceState::Proven,
                artifact_discovery: cynic_protocol::SecurityEvidenceState::Proven,
                attestation_sink: cynic_protocol::SecurityEvidenceState::Proven,
                evidence_refs: vec!["fixture:pre-os".to_string()],
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

    impl HostHardwareObservationPort for MissingServiceObserver {
        fn observe_hardware(&self) -> crate::hardware_conformance::HardwareObservation {
            ReadyObserver.observe_hardware()
        }
    }

    impl PreOsPlatformObservationPort for MissingServiceObserver {
        fn observe_pre_os_platform(&self) -> cynic_protocol::PreOsPlatformObservation {
            ReadyObserver.observe_pre_os_platform()
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
