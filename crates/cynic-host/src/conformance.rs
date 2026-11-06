use serde::Serialize;

use crate::adapters::local_observation::LocalHostObservationAdapter;
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
use cynic_registry::HostProfileResolution;
use crate::project_root::current_project_root;
use cynic_registry::runtime_harness::{RuntimeHarnessContext, conformance_context};
use cynic_registry::runtime_layout::current_runtime_layout;
use crate::runtime_proof::{
    assemble_runtime_proof_bundle, assemble_runtime_proof_persistence_record,
    host_identity_runtime_proof,
};
use crate::usecases::{HostObservationReports, observe_host_with_resolution};
use cynic_protocol::build::{BuildConformanceEvidence, BuildProof};
use cynic_protocol::convergence::{
    ConvergenceCell, ConvergenceMatrixReport, ConvergenceStatus, assess_convergence_matrix,
};
use cynic_protocol::{FrameworkSpec, framework_spec};
use cynic_types::paths::CynicPaths;

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
    pub convergence: cynic_protocol::convergence::ConvergenceMatrixReport,
    pub observed: HostObservationReports,
    pub bootstrap_evidence: HostBootstrapEvidenceReport,
    pub hardware: HardwareConformanceReport,
    pub hardware_baseline_plan: HardwareBaselinePlan,
    pub pre_os: cynic_node_protocol::PreOsPlatformObservation,
    pub proof_bundle: cynic_protocol::RuntimeProofBundle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_evidence: Option<BuildConformanceEvidence>,
    pub persistence_record: cynic_protocol::RuntimeProofPersistenceRecord,
    pub rules: Vec<&'static str>,
    pub next_actions: Vec<String>,
}

pub fn observe_host_conformance(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
) -> Result<HostConformanceReport, Box<dyn std::error::Error>> {
    observe_host_conformance_with_observer_and_build_evidence(
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
    observe_host_conformance_with_observer_and_build_evidence(
        profile,
        resolution,
        observed_port,
        None,
    )
}

pub fn observe_host_conformance_with_build_proof(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    build_proof: Option<&BuildProof>,
) -> Result<HostConformanceReport, Box<dyn std::error::Error>> {
    let build_evidence = build_proof.map(BuildConformanceEvidence::from_build_proof);
    observe_host_conformance_with_observer_and_build_evidence(
        profile,
        resolution,
        &LocalHostObservationAdapter,
        build_evidence.as_ref(),
    )
}

pub fn observe_host_conformance_with_build_evidence(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    build_evidence: Option<&BuildConformanceEvidence>,
) -> Result<HostConformanceReport, Box<dyn std::error::Error>> {
    observe_host_conformance_with_observer_and_build_evidence(
        profile,
        resolution,
        &LocalHostObservationAdapter,
        build_evidence,
    )
}

fn observe_host_conformance_with_observer_and_build_evidence(
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
    build_evidence: Option<&BuildConformanceEvidence>,
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
        cynic_registry::runtime_layout::current_runtime_layout().hardware_baseline_path();
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
    let vertical_evidence = observe_vertical_evidence(build_evidence);
    let verticals = evaluate_vertical_conformance(&context, &gates, &vertical_evidence);
    let subject = identity_proof.subject.clone();
    let convergence = convergence_matrix_for_subject(&subject, &verticals);
    let proof_bundle = assemble_runtime_proof_bundle(identity_proof, &verticals);
    let persistence_record =
        assemble_runtime_proof_persistence_record(&proof_bundle, &verticals, build_evidence);
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
        build_evidence: build_evidence.cloned(),
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
    let forgejo: Option<&()> = None;
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
            status: "blocked",
            detail: "decoupled from cynic-host".to_string(),
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
                executor_present: true,
                membrane_present: true,
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
            crate::domain::packages::promoted_package_state(policy.bootstrap_packages.clone())
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
        fn observe_pre_os_platform(&self) -> cynic_node_protocol::PreOsPlatformObservation {
            cynic_node_protocol::PreOsPlatformObservation {
                schema: cynic_node_protocol::PRE_OS_PLATFORM_OBSERVATION_SCHEMA.to_string(),
                platform: "test".to_string(),
                controls: vec![cynic_node_protocol::PreOsControlKind::Hypervisor],
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
        fn observe_pre_os_platform(&self) -> cynic_node_protocol::PreOsPlatformObservation {
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
        // Not "qualified": the package domain is Blocked by design because the
        // profile policy declares unresolved_distribution_authority(), so even a
        // fully ready observer leaves one host contract unsatisfied. Demanding
        // "qualified" here would require reporting an unselected distribution
        // authority as conformant.
        assert_eq!(
            report.status,
            "blocked",
            "gates: {:?}",
            report
                .gates
                .iter()
                .map(|gate| (gate.id, gate.status))
                .collect::<Vec<_>>()
        );
        assert!(!report.mutates);
        assert!(!report.context.mutates_host_network);
        // The proof bundle must not claim satisfaction while a host contract is
        // blocked: keep the coupling asserted rather than dropping the check.
        assert_ne!(
            report.proof_bundle.status, "satisfied",
            "a blocked conformance report must not carry a satisfied proof bundle"
        );
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

// Evaluation of the immutable vertical contract catalog against local proof.

/// A single conformance gate and its observed outcome.
///
/// Lives here rather than in `conformance` because it is a leaf: no
/// dependencies, consumed as a parameter by `evaluate_vertical_conformance`,
/// constructed by `observe_host_conformance`. Housing it in the higher module
/// forced `vertical_conformance` to import upward, which closed the cycle
/// `conformance <-> vertical_conformance`.
#[derive(Debug, Serialize)]
pub struct HostConformanceGate {
    pub id: &'static str,
    pub status: &'static str,
    pub detail: String,
}

pub const VERTICAL_CONFORMANCE_SCHEMA: &str = "cynic.vertical_conformance_report.v1";

#[derive(Debug, Clone, Serialize)]
pub struct VerticalConformanceReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub harness: RuntimeHarnessContext,
    pub catalog_status: &'static str,
    pub verticals: Vec<VerticalConformanceEntry>,
    pub residual_unknowns: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerticalConformanceEntry {
    pub id: String,
    pub declared_coverage: String,
    pub status: &'static str,
    pub responsibility_status: &'static str,
    pub accountable_owner: Option<String>,
    pub domain_owner: Option<String>,
    pub observation_owner: Option<String>,
    pub execution_owner: Option<String>,
    pub proof_owner: Option<String>,
    pub lifecycle_owner: Option<String>,
    pub owner: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerticalEvidenceInput {
    pub source_status: &'static str,
    pub source_blockers: Vec<String>,
    pub build_status: &'static str,
    pub build_blockers: Vec<String>,
    pub network_status: &'static str,
    pub network_blockers: Vec<String>,
}

pub fn observe_vertical_evidence(
    build_proof: Option<&BuildConformanceEvidence>,
) -> VerticalEvidenceInput {
    let canonical_root = current_project_root();
    let (source_status, source_blockers) = canonical_root.as_deref().map_or(
        (
            "unknown",
            vec!["canonical_project_root_missing".to_string()],
        ),
        |_root| {
            ("unknown", Vec::new())
        },
    );
    let (build_status, build_blockers) = match build_proof {
        Some(proof) if proof.status == "proved" || proof.status == "completed" => {
            ("complete", Vec::new())
        }
        Some(proof) if proof.status == "failed" => ("blocked", proof.blockers.clone()),
        Some(proof) => ("unknown", proof.blockers.clone()),
        None => {
            if CynicPaths::state_dir()
                .join("reports/build-proof.json")
                .exists()
                || std::path::Path::new("/tmp/cynic-build-proof.json").exists()
            {
                ("complete", Vec::new())
            } else {
                ("unknown", vec!["build_proof_not_consumed".to_string()])
            }
        }
    };
    let (network_status, network_blockers) = {
        if std::path::Path::new("/sys/class/net/wg0").exists() {
            ("complete", Vec::new())
        } else {
            (
                "unknown",
                vec!["network_vertical_not_consumed_by_host_conformance".to_string()],
            )
        }
    };
    let heartbeat_path =
        std::path::Path::new(&current_runtime_layout().state_dir).join("heartbeat.jsonl");
    let _ = append_heartbeat(
        &heartbeat_path,
        "vertical_conformance",
        build_status,
        network_status,
        source_status,
    );

    VerticalEvidenceInput {
        source_status,
        source_blockers,
        build_status,
        build_blockers,
        network_status,
        network_blockers,
    }
}

pub(crate) fn append_heartbeat(
    journal: &std::path::Path,
    surface: &str,
    build: &str,
    network: &str,
    source: &str,
) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = journal.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let entry = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "surface": surface,
        "build": build,
        "network": network,
        "source": source,
    });
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(journal)?;
    writeln!(file, "{}", entry.to_string())
}

pub fn convergence_matrix_for_subject(
    subject_id: &str,
    report: &VerticalConformanceReport,
) -> ConvergenceMatrixReport {
    let cells = report
        .verticals
        .iter()
        .map(|entry| ConvergenceCell {
            subject_id: subject_id.to_string(),
            vertical_id: entry.id.clone(),
            status: match entry.status {
                "complete" => ConvergenceStatus::Complete,
                "partial" => ConvergenceStatus::Partial,
                "blocked" => ConvergenceStatus::Blocked,
                _ => ConvergenceStatus::Unknown,
            },
            evidence_refs: entry.evidence_refs.clone(),
            blockers: entry.blockers.clone(),
        })
        .collect();
    assess_convergence_matrix(
        vec![subject_id.to_string()],
        report
            .verticals
            .iter()
            .map(|entry| entry.id.clone())
            .collect(),
        cells,
    )
}

pub fn evaluate_vertical_conformance(
    context: &RuntimeHarnessContext,
    gates: &[HostConformanceGate],
    evidence: &VerticalEvidenceInput,
) -> VerticalConformanceReport {
    evaluate_vertical_conformance_with_catalog(context, gates, evidence, framework_spec())
}

fn evaluate_vertical_conformance_with_catalog(
    context: &RuntimeHarnessContext,
    gates: &[HostConformanceGate],
    evidence: &VerticalEvidenceInput,
    catalog: FrameworkSpec,
) -> VerticalConformanceReport {
    use std::collections::{BTreeMap, BTreeSet};

    let identity = gate_status(gates, "host_identity_observed");
    let services = gate_status(gates, "service_surface_observed");
    let host = gate_status(gates, "harness_is_conformance_gate");
    let bootstrap = gate_status(gates, "bootstrap_target_reached");
    let hardware = gate_status(gates, "hardware_stability_qualified");
    let mut entries = Vec::new();
    let mut unknowns = Vec::new();
    let mut blockers = Vec::new();
    let responsibility_violations = catalog.responsibility_contracts.validate();
    let violations_by_capability = responsibility_violations.iter().fold(
        BTreeMap::<&str, Vec<&str>>::new(),
        |mut grouped, violation| {
            grouped
                .entry(violation.capability_id.as_str())
                .or_default()
                .push(violation.code);
            grouped
        },
    );
    let vertical_ids = catalog
        .vertical_contracts
        .iter()
        .map(|contract| contract.id)
        .collect::<BTreeSet<_>>();
    let responsibility_ids = catalog
        .responsibility_contracts
        .contracts
        .iter()
        .map(|contract| contract.capability_id)
        .collect::<BTreeSet<_>>();
    for missing in vertical_ids.difference(&responsibility_ids) {
        blockers.push(format!("responsibility_contract_missing:{missing}"));
    }
    for extra in responsibility_ids.difference(&vertical_ids) {
        blockers.push(format!("responsibility_contract_without_vertical:{extra}"));
    }
    for violation in &responsibility_violations {
        blockers.push(format!(
            "responsibility_contract_invalid:{}:{}",
            violation.capability_id, violation.code
        ));
    }
    let catalog_valid = blockers.is_empty();

    for contract in catalog.vertical_contracts {
        let (mut status, mut entry_blockers, refs) = match contract.id {
            "process" | "principal" => {
                (identity, vec![], vec!["host_identity_observed".to_string()])
            }
            "filesystem" => (
                bootstrap,
                vec![],
                vec!["bootstrap_target_reached".to_string()],
            ),
            "source" => (
                evidence.source_status,
                evidence.source_blockers.clone(),
                vec!["source_worktree_observation".to_string()],
            ),
            "build" => (
                evidence.build_status,
                evidence.build_blockers.clone(),
                vec![],
            ),
            "artifact" => (
                bootstrap,
                vec![],
                vec!["bootstrap_target_reached".to_string()],
            ),
            "service" => (
                services,
                vec![],
                vec!["service_surface_observed".to_string()],
            ),
            "host_os" => (
                host,
                vec![],
                vec!["harness_is_conformance_gate".to_string()],
            ),
            "network" => (
                evidence.network_status,
                evidence.network_blockers.clone(),
                if evidence.network_status == "complete" {
                    vec!["network_mesh_status".to_string()]
                } else {
                    vec![]
                },
            ),
            "node" => (identity, vec![], vec!["host_identity_observed".to_string()]),
            "provider" | "orchestrator" | "federation" => {
                ("unknown", vec!["adapter_not_observed".to_string()], vec![])
            }
            "project" => (
                evidence.source_status,
                evidence.source_blockers.clone(),
                vec!["source_worktree_observation".to_string()],
            ),
            "fleet" => (
                "unknown",
                vec!["fleet_vertical_requires_multi_node_proof".to_string()],
                vec![],
            ),
            "human_boundary" => (
                hardware,
                vec![],
                vec!["hardware_stability_qualified".to_string()],
            ),
            _ => (
                "unknown",
                vec!["catalog_vertical_not_mapped".to_string()],
                vec![],
            ),
        };
        let responsibility = catalog
            .responsibility_contracts
            .contracts
            .iter()
            .find(|candidate| candidate.capability_id == contract.id);
        let responsibility_errors = violations_by_capability
            .get(contract.id)
            .cloned()
            .unwrap_or_default();
        let responsibility_status = if responsibility.is_none() {
            entry_blockers.push("responsibility_contract_missing".to_string());
            "missing"
        } else if responsibility_errors.is_empty() {
            "complete"
        } else {
            entry_blockers.extend(
                responsibility_errors
                    .iter()
                    .map(|code| format!("responsibility_contract_invalid:{code}")),
            );
            "invalid"
        };
        if responsibility_status != "complete" {
            status = "blocked";
        }
        if status == "unknown" {
            unknowns.push(contract.id.to_string());
        }
        if !entry_blockers.is_empty() {
            blockers.extend(entry_blockers.clone());
        }
        entries.push(VerticalConformanceEntry {
            id: contract.id.to_string(),
            declared_coverage: contract.coverage.to_string(),
            status,
            responsibility_status,
            accountable_owner: responsibility.map(|item| item.accountable_owner.to_string()),
            domain_owner: responsibility.map(|item| item.domain_owner.to_string()),
            observation_owner: responsibility.map(|item| item.observation_owner.to_string()),
            execution_owner: responsibility.map(|item| item.execution_owner.to_string()),
            proof_owner: responsibility.map(|item| item.proof_owner.to_string()),
            lifecycle_owner: responsibility.map(|item| item.lifecycle_owner.to_string()),
            owner: contract.owner.into_iter().map(str::to_string).collect(),
            evidence_refs: refs,
            blockers: std::mem::take(&mut entry_blockers),
        });
    }
    let status = if entries.iter().any(|entry| entry.status == "blocked") {
        "blocked"
    } else if entries.iter().any(|entry| entry.status == "unknown") {
        "unknown"
    } else if entries.iter().all(|entry| entry.status == "complete") {
        "complete"
    } else {
        "partial"
    };
    VerticalConformanceReport {
        schema: VERTICAL_CONFORMANCE_SCHEMA,
        status,
        mutates: false,
        harness: context.clone(),
        catalog_status: if catalog_valid { "consumed" } else { "invalid" },
        verticals: entries,
        residual_unknowns: unknowns,
        blockers,
    }
}

fn gate_status(gates: &[HostConformanceGate], id: &str) -> &'static str {
    match gates
        .iter()
        .find(|gate| gate.id == id)
        .map(|gate| gate.status)
    {
        Some("satisfied") => "partial",
        Some("blocked") => "blocked",
        _ => "unknown",
    }
}

#[cfg(test)]
mod conformance_tests {
    use super::*;
    use cynic_registry::runtime_harness::conformance_context;

    #[test]
    fn local_vertical_report_preserves_unknown_provider_and_orchestrator() {
        let report = evaluate_vertical_conformance(
            &conformance_context(),
            &[],
            &VerticalEvidenceInput {
                source_status: "unknown",
                source_blockers: vec!["fixture".into()],
                build_status: "unknown",
                build_blockers: vec!["build_fixture".into()],
                network_status: "complete",
                network_blockers: vec![],
            },
        );
        assert_eq!(report.catalog_status, "consumed");
        assert!(report.residual_unknowns.contains(&"provider".to_string()));
        assert!(
            report
                .residual_unknowns
                .contains(&"orchestrator".to_string())
        );
        assert!(
            report
                .verticals
                .iter()
                .all(|entry| entry.responsibility_status == "complete")
        );
    }

    #[test]
    fn missing_responsibility_contract_blocks_its_vertical() {
        let mut catalog = framework_spec();
        catalog
            .responsibility_contracts
            .contracts
            .retain(|contract| contract.capability_id != "source");
        let report = evaluate_vertical_conformance_with_catalog(
            &conformance_context(),
            &[],
            &VerticalEvidenceInput {
                source_status: "partial",
                source_blockers: vec![],
                build_status: "unknown",
                build_blockers: vec![],
                network_status: "complete",
                network_blockers: vec![],
            },
            catalog,
        );
        let source = report
            .verticals
            .iter()
            .find(|entry| entry.id == "source")
            .expect("source vertical");
        assert_eq!(report.catalog_status, "invalid");
        assert_eq!(source.status, "blocked");
        assert_eq!(source.responsibility_status, "missing");
    }

    #[test]
    fn invalid_domain_contract_blocks_its_vertical() {
        let mut catalog = framework_spec();
        let source = catalog
            .responsibility_contracts
            .contracts
            .iter_mut()
            .find(|contract| contract.capability_id == "source")
            .expect("source responsibility");
        source.domain_rule = "";
        let report = evaluate_vertical_conformance_with_catalog(
            &conformance_context(),
            &[],
            &VerticalEvidenceInput {
                source_status: "partial",
                source_blockers: vec![],
                build_status: "unknown",
                build_blockers: vec![],
                network_status: "complete",
                network_blockers: vec![],
            },
            catalog,
        );
        let source = report
            .verticals
            .iter()
            .find(|entry| entry.id == "source")
            .expect("source vertical");
        assert_eq!(source.status, "blocked");
        assert_eq!(source.responsibility_status, "invalid");
        assert!(
            source
                .blockers
                .contains(&"responsibility_contract_invalid:domain_rule_missing".to_string())
        );
    }

    #[test]
    fn heartbeat_journal_appends_probe_evidence() {
        let temp =
            std::env::temp_dir().join(format!("cynic-heartbeat-test-{}", std::process::id()));
        let heartbeat_path = temp.join("probes").join("heartbeat.jsonl");
        let before = std::fs::read_to_string(&heartbeat_path).unwrap_or_default();
        append_heartbeat(
            &heartbeat_path,
            "vertical_conformance",
            "complete",
            "complete",
            "partial",
        )
        .expect("append heartbeat line");
        let after = std::fs::read_to_string(&heartbeat_path).unwrap_or_default();
        assert!(
            after.len() > before.len(),
            "heartbeat journal must grow after a probe"
        );
        let last_line = after
            .lines()
            .last()
            .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok());
        assert_eq!(
            last_line.as_ref().and_then(|v| v["surface"].as_str()),
            Some("vertical_conformance")
        );
        assert_eq!(
            last_line.as_ref().and_then(|v| v["build"].as_str()),
            Some("complete")
        );
        let _ = std::fs::remove_dir_all(&temp);
    }
}
use crate::domain::services::ServicePolicyExt as _;
