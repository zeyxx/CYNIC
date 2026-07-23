use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::conformance::HostConformanceReport;
use crate::domain::shared::SurfaceStatus;
use crate::forgejo_control::current_forgejo_plan;
use cynic_protocol::RuntimeProofBundle;

pub const HOST_RUNTIME_METRICS_SCHEMA: &str = "cynic.host.runtime_metrics.v1";
const DEFAULT_PROOF_FRESHNESS_MAX_AGE_MS: u128 = 5 * 60 * 1000;
const MIN_KNOWN_COVERAGE_PERCENT: u8 = 60;
const MIN_MATURITY_PERCENT: u8 = 80;

#[derive(Debug, Serialize)]
pub struct HostRuntimeMetricsReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub source_schema: &'static str,
    pub maturity_percent: u8,
    pub measured_percent: u8,
    pub known_coverage_percent: u8,
    pub residual_unknown_percent: u8,
    pub measured_metric_count: usize,
    pub unknown_metric_count: usize,
    pub total_metric_count: usize,
    pub minimum_known_coverage_percent: u8,
    pub minimum_maturity_percent: u8,
    pub proof_freshness_max_age_ms: u128,
    pub external_proof_gates: Vec<String>,
    pub metrics: Vec<RuntimeMetric>,
    pub rules: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct RuntimeMetric {
    pub id: &'static str,
    pub label: &'static str,
    pub percent: Option<u8>,
    pub target_percent: u8,
    pub critical: bool,
    pub status: &'static str,
    pub numerator: Option<usize>,
    pub denominator: Option<usize>,
    pub source: &'static str,
    pub detail: String,
}

pub fn host_runtime_metrics(report: &HostConformanceReport) -> HostRuntimeMetricsReport {
    host_runtime_metrics_with_external_proof(report, None)
}

pub fn host_runtime_metrics_with_external_proof(
    report: &HostConformanceReport,
    external_proof: Option<&RuntimeProofBundle>,
) -> HostRuntimeMetricsReport {
    host_runtime_metrics_with_now(
        report,
        external_proof,
        now_unix_ms(),
        DEFAULT_PROOF_FRESHNESS_MAX_AGE_MS,
    )
}

fn host_runtime_metrics_with_now(
    report: &HostConformanceReport,
    external_proof: Option<&RuntimeProofBundle>,
    now_unix_ms: u128,
    proof_freshness_max_age_ms: u128,
) -> HostRuntimeMetricsReport {
    let metrics = vec![
        conformance_metric(report),
        bootstrap_stage_metric(report),
        runtime_uptime_metric(),
        proof_freshness_metric(report, now_unix_ms, proof_freshness_max_age_ms),
        desired_vs_observed_metric(report),
        apply_success_metric(),
        rollback_coverage_metric(),
        portability_metric(),
        distributed_formation_metric(external_proof),
        architecture_boundary_metric(external_proof),
        sovereign_git_metric(),
        security_posture_metric(report),
        observability_coverage_metric(report),
    ];
    let total_metric_count = metrics.len();
    let measured = metrics
        .iter()
        .filter_map(|metric| metric.percent)
        .collect::<Vec<_>>();
    let measured_metric_count = measured.len();
    let unknown_metric_count = total_metric_count.saturating_sub(measured_metric_count);
    let known_coverage_percent = percent(measured_metric_count, total_metric_count);
    let residual_unknown_percent = 100u8.saturating_sub(known_coverage_percent);
    let measured_percent = if measured.is_empty() {
        0
    } else {
        (measured.iter().map(|value| *value as usize).sum::<usize>() / measured.len()) as u8
    };
    let maturity_percent =
        ((measured_percent as usize * known_coverage_percent as usize) / 100) as u8;
    let critical_below_target = metrics
        .iter()
        .any(|metric| metric.critical && metric.status == "below_target");
    let status = if critical_below_target {
        "below_target"
    } else if known_coverage_percent < MIN_KNOWN_COVERAGE_PERCENT {
        "insufficient_evidence"
    } else if maturity_percent >= MIN_MATURITY_PERCENT {
        "operationally_acceptable"
    } else if metrics.iter().any(|metric| metric.percent.is_none()) {
        "partial"
    } else {
        "below_target"
    };

    HostRuntimeMetricsReport {
        schema: HOST_RUNTIME_METRICS_SCHEMA,
        status,
        source_schema: report.schema,
        maturity_percent,
        measured_percent,
        known_coverage_percent,
        residual_unknown_percent,
        measured_metric_count,
        unknown_metric_count,
        total_metric_count,
        minimum_known_coverage_percent: MIN_KNOWN_COVERAGE_PERCENT,
        minimum_maturity_percent: MIN_MATURITY_PERCENT,
        proof_freshness_max_age_ms,
        external_proof_gates: external_proof
            .map(RuntimeProofBundle::observed_gates)
            .unwrap_or_default(),
        metrics,
        rules: vec![
            "100 percent is not a global requirement; each metric has an explicit target threshold",
            "unknown metrics are null, never coerced to zero",
            "percent zero means measured and failing",
            "maturity is measured_percent weighted by known coverage, so blind spots reduce confidence",
            "runtime uptime requires historical probes, not a single observation",
            "external runtime proof bundles may satisfy metrics that cannot be observed by host conformance alone",
        ],
    }
}

fn conformance_metric(report: &HostConformanceReport) -> RuntimeMetric {
    let denominator = report.gates.len();
    let numerator = report
        .gates
        .iter()
        .filter(|gate| gate.status == "satisfied")
        .count();
    measured_metric(MeasuredMetricSpec {
        id: "host_conformance_percent",
        label: "Host conformance",
        numerator,
        denominator,
        source: "cynic.host.conformance.v1/gates",
        target_percent: 95,
        critical: true,
        detail: format!("satisfied_gates={numerator}/{denominator}"),
    })
}

fn bootstrap_stage_metric(report: &HostConformanceReport) -> RuntimeMetric {
    let target_index = bootstrap_stage_index(&report.bootstrap_evidence.target_stage);
    let reached_index = bootstrap_stage_index(&report.bootstrap_evidence.reached_stage);
    match (target_index, reached_index) {
        (Some(target), Some(reached)) => {
            let denominator = target + 1;
            let numerator = if report.bootstrap_evidence.reached_stage == "none" {
                0
            } else {
                usize::min(reached + 1, denominator)
            };
            measured_metric(MeasuredMetricSpec {
                id: "bootstrap_stage_percent",
                label: "Bootstrap stage",
                numerator,
                denominator,
                source: "cynic.host_bootstrap_evidence.v1",
                target_percent: 90,
                critical: true,
                detail: format!(
                    "target_stage={} reached_stage={}",
                    report.bootstrap_evidence.target_stage, report.bootstrap_evidence.reached_stage
                ),
            })
        }
        _ => unknown_metric(
            "bootstrap_stage_percent",
            "Bootstrap stage",
            "cynic.host_bootstrap_evidence.v1",
            90,
            true,
            format!(
                "unrecognized target_stage={} reached_stage={}",
                report.bootstrap_evidence.target_stage, report.bootstrap_evidence.reached_stage
            ),
        ),
    }
}

fn proof_freshness_metric(
    report: &HostConformanceReport,
    now_unix_ms: u128,
    proof_freshness_max_age_ms: u128,
) -> RuntimeMetric {
    let denominator = report.proof_bundle.proofs.len();
    let numerator = report
        .proof_bundle
        .proofs
        .iter()
        .filter(|proof| {
            now_unix_ms.saturating_sub(proof.observed_at_unix_ms) <= proof_freshness_max_age_ms
        })
        .count();
    measured_metric(MeasuredMetricSpec {
        id: "proof_freshness_percent",
        label: "Proof freshness",
        numerator,
        denominator,
        source: "cynic.runtime_proof_bundle.v1",
        target_percent: 95,
        critical: true,
        detail: format!("fresh_proofs={numerator}/{denominator}"),
    })
}

fn desired_vs_observed_metric(report: &HostConformanceReport) -> RuntimeMetric {
    let statuses = [
        report.observed.principal.status,
        report.observed.privilege.status,
        report.observed.packages.status,
        report.observed.services.status,
    ];
    let denominator = statuses.len();
    let numerator = statuses
        .iter()
        .filter(|status| **status == SurfaceStatus::Active)
        .count();
    measured_metric(MeasuredMetricSpec {
        id: "desired_vs_observed_percent",
        label: "Desired vs observed",
        numerator,
        denominator,
        source: "cynic-host/usecases/HostObservationReports",
        target_percent: 95,
        critical: true,
        detail: format!("active_host_domains={numerator}/{denominator}"),
    })
}

fn security_posture_metric(report: &HostConformanceReport) -> RuntimeMetric {
    let statuses = [
        report.observed.principal.status,
        report.observed.privilege.status,
    ];
    let denominator = statuses.len();
    let numerator = statuses
        .iter()
        .filter(|status| **status == SurfaceStatus::Active)
        .count();
    measured_metric(MeasuredMetricSpec {
        id: "security_posture_percent",
        label: "Security posture",
        numerator,
        denominator,
        source: "principal+privilege observation",
        target_percent: 95,
        critical: true,
        detail: format!("active_security_domains={numerator}/{denominator}"),
    })
}

fn observability_coverage_metric(report: &HostConformanceReport) -> RuntimeMetric {
    let denominator = 6;
    let mut numerator = 0;
    numerator += 1; // principal observation emitted
    numerator += 1; // privilege observation emitted
    numerator += 1; // packages observation emitted
    numerator += 1; // services observation emitted
    numerator += usize::from(!report.bootstrap_evidence.stages.is_empty());
    numerator += usize::from(!report.proof_bundle.proofs.is_empty());
    measured_metric(MeasuredMetricSpec {
        id: "observability_coverage_percent",
        label: "Observability coverage",
        numerator,
        denominator,
        source: "host observation + bootstrap evidence + proof bundle",
        target_percent: 80,
        critical: false,
        detail: format!("observable_surfaces={numerator}/{denominator}"),
    })
}

fn runtime_uptime_metric() -> RuntimeMetric {
    unknown_metric(
        "runtime_uptime_percent",
        "Runtime uptime",
        "historical health probes",
        99,
        true,
        "requires time-series probes; one observation is not uptime".to_string(),
    )
}

fn apply_success_metric() -> RuntimeMetric {
    unknown_metric(
        "apply_success_percent",
        "Apply success",
        "apply execution history",
        95,
        true,
        "requires recorded apply attempts and outcomes".to_string(),
    )
}

fn rollback_coverage_metric() -> RuntimeMetric {
    unknown_metric(
        "rollback_coverage_percent",
        "Rollback coverage",
        "execution plans",
        90,
        true,
        "requires mutation catalog with rollback or compensation hints".to_string(),
    )
}

fn portability_metric() -> RuntimeMetric {
    unknown_metric(
        "portability_percent",
        "Portability",
        "multi-environment conformance matrix",
        80,
        false,
        "requires conformance runs across declared OS/provider targets".to_string(),
    )
}

fn distributed_formation_metric(external_proof: Option<&RuntimeProofBundle>) -> RuntimeMetric {
    proof_gate_metric(
        external_proof,
        &ProofGateMetricSpec {
            id: "distributed_formation_percent",
            label: "Distributed formation",
            gate: "distributed_formation_observed",
            source: "network/constellation runtime proof",
            target_percent: 90,
            critical: true,
            missing_detail: "requires N-node reachability and service-formation evidence",
        },
    )
}

fn architecture_boundary_metric(external_proof: Option<&RuntimeProofBundle>) -> RuntimeMetric {
    proof_gate_metric(
        external_proof,
        &ProofGateMetricSpec {
            id: "architecture_boundary_percent",
            label: "Architecture boundary",
            gate: "architecture_boundary_verified",
            source: "architecture tests",
            target_percent: 95,
            critical: true,
            missing_detail: "requires test execution evidence, not host runtime observation",
        },
    )
}

fn sovereign_git_metric() -> RuntimeMetric {
    match current_forgejo_plan() {
        Ok(plan) => measured_metric(MeasuredMetricSpec {
            id: "sovereign_git_percent",
            label: "Sovereign git",
            numerator: usize::from(plan.status == "ready" && plan.actions.is_empty()),
            denominator: 1,
            source: "cynic.forgejo.plan.v1",
            target_percent: 95,
            critical: true,
            detail: format!(
                "forgejo status={} pending_actions={} blockers={}",
                plan.status,
                plan.actions.len(),
                plan.blockers.join(",")
            ),
        }),
        Err(error) => unknown_metric(
            "sovereign_git_percent",
            "Sovereign git",
            "cynic.forgejo.plan.v1",
            95,
            true,
            error,
        ),
    }
}

#[derive(Debug)]
struct ProofGateMetricSpec {
    id: &'static str,
    label: &'static str,
    gate: &'static str,
    source: &'static str,
    target_percent: u8,
    critical: bool,
    missing_detail: &'static str,
}

#[derive(Debug)]
struct MeasuredMetricSpec {
    id: &'static str,
    label: &'static str,
    numerator: usize,
    denominator: usize,
    source: &'static str,
    target_percent: u8,
    critical: bool,
    detail: String,
}

fn proof_gate_metric(
    external_proof: Option<&RuntimeProofBundle>,
    spec: &ProofGateMetricSpec,
) -> RuntimeMetric {
    match external_proof.and_then(|bundle| proof_gate_state(bundle, spec.gate)) {
        Some(ProofGateState::Satisfied) => measured_metric(MeasuredMetricSpec {
            id: spec.id,
            label: spec.label,
            numerator: 1,
            denominator: 1,
            source: spec.source,
            target_percent: spec.target_percent,
            critical: spec.critical,
            detail: format!("external proof gate {} is satisfied", spec.gate),
        }),
        Some(ProofGateState::Blocked) => measured_metric(MeasuredMetricSpec {
            id: spec.id,
            label: spec.label,
            numerator: 0,
            denominator: 1,
            source: spec.source,
            target_percent: spec.target_percent,
            critical: spec.critical,
            detail: format!(
                "external proof gate {} is present but not satisfied",
                spec.gate
            ),
        }),
        None => unknown_metric(
            spec.id,
            spec.label,
            spec.source,
            spec.target_percent,
            spec.critical,
            spec.missing_detail.to_string(),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProofGateState {
    Satisfied,
    Blocked,
}

fn proof_gate_state(bundle: &RuntimeProofBundle, gate: &str) -> Option<ProofGateState> {
    bundle
        .proofs
        .iter()
        .find(|proof| proof.gate == gate)
        .map(|proof| {
            if proof.gate_satisfied() {
                ProofGateState::Satisfied
            } else {
                ProofGateState::Blocked
            }
        })
}

fn measured_metric(spec: MeasuredMetricSpec) -> RuntimeMetric {
    let percent = percent(spec.numerator, spec.denominator);
    let status = if percent >= spec.target_percent {
        "within_target"
    } else if spec.critical {
        "below_target"
    } else {
        "watch"
    };
    RuntimeMetric {
        id: spec.id,
        label: spec.label,
        percent: Some(percent),
        target_percent: spec.target_percent,
        critical: spec.critical,
        status,
        numerator: Some(spec.numerator),
        denominator: Some(spec.denominator),
        source: spec.source,
        detail: spec.detail,
    }
}

fn unknown_metric(
    id: &'static str,
    label: &'static str,
    source: &'static str,
    target_percent: u8,
    critical: bool,
    detail: String,
) -> RuntimeMetric {
    RuntimeMetric {
        id,
        label,
        percent: None,
        target_percent,
        critical,
        status: "unknown",
        numerator: None,
        denominator: None,
        source,
        detail,
    }
}

fn percent(numerator: usize, denominator: usize) -> u8 {
    let percent = match numerator.checked_mul(100) {
        Some(numerator_x100) => numerator_x100.checked_div(denominator).unwrap_or(0),
        None => u8::MAX as usize,
    };
    let percent = percent.min(100);
    u8::try_from(percent).unwrap_or(u8::MAX)
}

fn bootstrap_stage_index(stage: &str) -> Option<usize> {
    Some(match stage {
        "h0_host_identity_ready" => 0,
        "h1_principal_materialized" => 1,
        "h2_filesystem_projected" => 2,
        "h3_packages_ready" => 3,
        "h4_privilege_helpers_ready" => 4,
        "h5_services_projected" => 5,
        "h6_host_actionable" => 6,
        "none" => 0,
        _ => return None,
    })
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conformance::observe_host_conformance_with_observer;
    use crate::domain::packages::PackageObservedState;
    use crate::domain::principal::PrincipalObservedState;
    use crate::domain::privilege::PrivilegeObservedState;
    use crate::domain::services::{ServiceObservedState, ServiceObservedUnit};
    use crate::ports::{
        HostPackageObservationPort, HostPrincipalObservationPort, HostPrivilegeObservationPort,
        HostServiceObservationPort,
    };
    use crate::profiles::HostProfile;
    use crate::registry::HostProfileResolution;

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

    impl crate::ports::HostHardwareObservationPort for ReadyObserver {
        fn observe_hardware(&self) -> crate::hardware_conformance::HardwareObservation {
            let inventory = crate::hardware_conformance::HardwareInventory {
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
            };
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

    impl crate::ports::PreOsPlatformObservationPort for ReadyObserver {
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

    #[test]
    fn metrics_keep_unknown_separate_from_zero() {
        let report = observe_host_conformance_with_observer(
            HostProfile::NetworkNode,
            Some(test_resolution()),
            &ReadyObserver,
        )
        .expect("conformance");

        let metrics = host_runtime_metrics_with_now(&report, None, now_unix_ms(), 60_000);

        assert_eq!(metrics.schema, HOST_RUNTIME_METRICS_SCHEMA);
        assert_eq!(
            metric(&metrics, "host_conformance_percent").percent,
            Some(100)
        );
        assert_eq!(
            metric(&metrics, "host_conformance_percent").target_percent,
            95
        );
        assert_eq!(
            metric(&metrics, "host_conformance_percent").status,
            "within_target"
        );
        assert!(metrics.known_coverage_percent < 100);
        assert!(metrics.residual_unknown_percent > 0);
        assert_eq!(metric(&metrics, "runtime_uptime_percent").percent, None);
        assert_eq!(
            metric(&metrics, "runtime_uptime_percent").target_percent,
            99
        );
        assert_eq!(metric(&metrics, "runtime_uptime_percent").status, "unknown");
    }

    #[test]
    fn metrics_can_consume_architecture_boundary_runtime_proof() {
        let report = observe_host_conformance_with_observer(
            HostProfile::NetworkNode,
            Some(test_resolution()),
            &ReadyObserver,
        )
        .expect("conformance");
        let proof = RuntimeProofBundle {
            schema: cynic_protocol::RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
            status: "satisfied".to_string(),
            proofs: vec![cynic_protocol::RuntimeProof {
                schema: cynic_protocol::RUNTIME_PROOF_SCHEMA.to_string(),
                gate: "architecture_boundary_verified".to_string(),
                status: "satisfied".to_string(),
                producer: "cargo-test".to_string(),
                subject: "cynic-cli/tests/architecture_boundary.rs".to_string(),
                observed_at_unix_ms: 1,
                evidence: Vec::new(),
            }],
        };

        let metrics = host_runtime_metrics_with_now(&report, Some(&proof), now_unix_ms(), 60_000);

        assert_eq!(
            metrics.external_proof_gates,
            vec!["architecture_boundary_verified".to_string()]
        );
        assert_eq!(
            metric(&metrics, "architecture_boundary_percent").percent,
            Some(100)
        );
        assert_eq!(
            metric(&metrics, "architecture_boundary_percent").status,
            "within_target"
        );
        assert_eq!(
            metric(&metrics, "distributed_formation_percent").percent,
            None
        );
    }

    fn metric<'a>(report: &'a HostRuntimeMetricsReport, id: &str) -> &'a RuntimeMetric {
        report
            .metrics
            .iter()
            .find(|metric| metric.id == id)
            .expect("metric")
    }

    fn test_resolution() -> HostProfileResolution {
        HostProfileResolution {
            hostname_observed: "candidate-node".to_string(),
            matched_node: Some("candidate-node".to_string()),
            matched_role: Some("network_node".to_string()),
            canonical_role: None,
            profile_applied: "network-node",
            match_source: "node_name",
            host_bootstrap_target_stage: Some("h5_services_projected".to_string()),
            host_bootstrap_required_domains: vec!["services".to_string()],
            registry_path: Some("infra/registry.json".to_string()),
        }
    }
}
