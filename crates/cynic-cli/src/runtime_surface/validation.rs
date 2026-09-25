use std::process::ExitCode;

use cynic_host::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus};
use serde::Serialize;

use crate::{emit_json, fail, registry_path};

mod fs_assertions;
mod predicates;
mod proof_fixtures;
mod steps;

use fs_assertions::{registry_dns_zone, validate_directory_contains, validate_directory_excludes};
use predicates::{
    metrics_consumes_architecture_proof, plan_consumes_host_identity_proof,
    plan_consumes_network_runtime_proof, runtime_paths_explicit_registry_is_operator_projection,
    suite_covers_cloud_prisms,
};
use proof_fixtures::{
    validation_temp_dir, write_satisfied_architecture_boundary_proof,
    write_satisfied_host_and_network_proofs, write_satisfied_host_identity_proof,
};
use steps::{
    RuntimeValidationStep, run_json_validation_step, run_json_validation_step_with_predicate,
};

pub(crate) fn execute_validate(
    json: bool,
    registry: Option<String>,
    lab_registry: Option<String>,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let registry = registry_path(registry);
    let lab_registry = lab_registry.unwrap_or_else(|| "infra/registries/lab.json".to_string());
    match build_runtime_validation_report(&registry, &lab_registry) {
        Ok(report) => emit_json(&report, report.status != "ok"),
        Err(error) => fail(error),
    }
}

#[derive(Debug, Serialize)]
struct RuntimeValidationReport {
    schema: &'static str,
    status: &'static str,
    registry_path: String,
    lab_registry_path: String,
    rule: &'static str,
    lifecycle_status: LifecycleStatus,
    lifecycle_disposition: LifecycleDisposition,
    next_boundary_kind: BoundaryKind,
    next_boundary_reason: String,
    next_command: Option<String>,
    steps: Vec<RuntimeValidationStep>,
}

fn build_runtime_validation_report(
    registry_path: &str,
    lab_registry_path: &str,
) -> Result<RuntimeValidationReport, String> {
    let temp_root = validation_temp_dir()?;
    let lab_dns_dir = temp_root.join("lab-dns");
    let host_proof_path = temp_root.join("host-proof.json");
    let network_proof_path = temp_root.join("host-network-proof.json");
    let architecture_proof_path = temp_root.join("architecture-proof.json");
    write_satisfied_host_identity_proof(&host_proof_path)?;
    write_satisfied_host_and_network_proofs(&network_proof_path)?;
    write_satisfied_architecture_boundary_proof(&architecture_proof_path)?;
    let lab_zone = registry_dns_zone(lab_registry_path)?;
    let default_zone = "synapse.cynic";

    let steps = vec![
        run_json_validation_step_with_predicate(
            "runtime_paths",
            &["runtime", "paths", "--json", "--registry", registry_path],
            &[0],
            "runtime path resolution must execute and emit JSON",
            runtime_paths_explicit_registry_is_operator_projection,
        ),
        run_json_validation_step(
            "canonical_surface_audit",
            &["dev", "surface-audit", "--json"],
            &[0],
            "canonical surfaces must map to real adapters, owner modules, owner calls, and mutation policies",
        ),
        run_json_validation_step(
            "host_conformance_probe",
            &[
                "runtime",
                "conformance",
                "--json",
                "--registry",
                registry_path,
            ],
            &[0, 1],
            "host conformance must execute read-only; blocked is valid proof before bootstrap",
        ),
        run_json_validation_step(
            "host_runtime_metrics",
            &["runtime", "metrics", "--json", "--registry", registry_path],
            &[0],
            "runtime metrics must emit percent-based JSON and keep unknown metrics explicit",
        ),
        run_json_validation_step_with_predicate(
            "host_runtime_metrics_consumes_architecture_proof",
            &[
                "runtime",
                "metrics",
                "--json",
                "--registry",
                registry_path,
                "--proof",
                &architecture_proof_path.to_string_lossy(),
            ],
            &[0],
            "runtime metrics must consume external architecture proof without pretending host observation produced it",
            metrics_consumes_architecture_proof,
        ),
        run_json_validation_step(
            "fleet_constellation_projection",
            &[
                "bootstrap",
                "constellation",
                "--json",
                "--registry",
                registry_path,
            ],
            &[0, 1],
            "real fleet registry must execute; bootstrap_required is valid proof",
        ),
        run_json_validation_step(
            "fleet_network_runtime_probe",
            &["network", "runtime", "--registry", registry_path],
            &[0, 1],
            "real network runtime probe must execute; blocked is valid proof",
        ),
        run_json_validation_step(
            "lab_constellation_projection",
            &[
                "bootstrap",
                "constellation",
                "--json",
                "--registry",
                lab_registry_path,
            ],
            &[0, 1],
            "portable lab registry must execute with N nodes",
        ),
        run_json_validation_step(
            "lab_execution_suite",
            &[
                "runtime",
                "suite",
                "--json",
                "--registry",
                lab_registry_path,
                "--profile",
                "lab",
            ],
            &[0],
            "portable lab execution suite must be generated by the CLI",
        ),
        run_json_validation_step_with_predicate(
            "lab_execution_suite_cloud_prisms",
            &[
                "runtime",
                "suite",
                "--json",
                "--registry",
                lab_registry_path,
                "--profile",
                "lab",
            ],
            &[0],
            "execution suite must cover cloud characteristics, service models, and storage security controls",
            suite_covers_cloud_prisms,
        ),
        run_json_validation_step(
            "lab_execution_plan",
            &[
                "runtime",
                "plan",
                "--json",
                "--registry",
                lab_registry_path,
                "--profile",
                "lab",
            ],
            &[0],
            "portable lab execution plan must expose runnable probes and blocked gates",
        ),
        run_json_validation_step_with_predicate(
            "lab_execution_plan_consumes_host_proof",
            &[
                "runtime",
                "plan",
                "--json",
                "--registry",
                lab_registry_path,
                "--profile",
                "lab",
                "--proof",
                &host_proof_path.to_string_lossy(),
            ],
            &[0],
            "runtime plan must open L2 when host_identity_observed is supplied as a typed proof",
            plan_consumes_host_identity_proof,
        ),
        run_json_validation_step_with_predicate(
            "lab_execution_plan_consumes_network_proof",
            &[
                "runtime",
                "plan",
                "--json",
                "--registry",
                lab_registry_path,
                "--profile",
                "lab",
                "--proof",
                &network_proof_path.to_string_lossy(),
            ],
            &[0],
            "runtime plan must open L3 when network_runtime_observed is supplied as a typed proof",
            plan_consumes_network_runtime_proof,
        ),
        run_json_validation_step(
            "lab_network_runtime_probe",
            &["network", "runtime", "--registry", lab_registry_path],
            &[0, 1],
            "portable lab network runtime probe must execute without assuming real fleet",
        ),
        run_json_validation_step(
            "lab_dns_bundle_generation",
            &[
                "network",
                "dns",
                "bundle",
                "--registry",
                lab_registry_path,
                "--output-dir",
                &lab_dns_dir.to_string_lossy(),
            ],
            &[0],
            "portable lab DNS bundle must generate successfully",
        ),
        validate_directory_contains(
            "lab_dns_bundle_zone",
            &lab_dns_dir,
            &lab_zone,
            "generated DNS bundle must contain the lab zone",
        ),
        validate_directory_excludes(
            "lab_dns_bundle_no_default_zone_leak",
            &lab_dns_dir,
            default_zone,
            "generated DNS bundle must not leak the default real-fleet zone",
        ),
    ];

    let status = if steps.iter().all(|step| step.status == "ok") {
        "ok"
    } else {
        "blocked"
    };
    let first_blocked = steps.iter().find(|step| step.status != "ok");
    let (
        lifecycle_status,
        lifecycle_disposition,
        next_boundary_kind,
        next_boundary_reason,
        next_command,
    ) = if status == "ok" {
        (
            LifecycleStatus::Ready,
            LifecycleDisposition::ObservableCandidate,
            BoundaryKind::Observe,
            "runtime validation steps completed without blocked portability or proof failures"
                .to_string(),
            None,
        )
    } else if let Some(step) = first_blocked {
        let next_command = match step.name {
            "host_conformance_probe" => Some(format!(
                "cynic runtime conformance --json --registry {registry_path}"
            )),
            "fleet_constellation_projection" => Some(format!(
                "cynic bootstrap constellation --json --registry {registry_path}"
            )),
            "fleet_network_runtime_probe" => {
                Some(format!("cynic network runtime --registry {registry_path}"))
            }
            "lab_network_runtime_probe" => Some(format!(
                "cynic network runtime --registry {lab_registry_path}"
            )),
            _ => Some("cynic runtime validate --json".to_string()),
        };
        let boundary = if step.name.contains("network") {
            BoundaryKind::Connect
        } else {
            BoundaryKind::Adopt
        };
        let lifecycle_status = if step.name.contains("network") {
            LifecycleStatus::ConnectionRequired
        } else {
            LifecycleStatus::PrincipalMismatch
        };
        (
            lifecycle_status,
            lifecycle_status.default_disposition(),
            boundary,
            format!("runtime validation is blocked at step {}", step.name),
            next_command,
        )
    } else {
        (
            LifecycleStatus::Missing,
            LifecycleDisposition::NotDiscovered,
            BoundaryKind::Observe,
            "runtime validation reported blocked without a failing step".to_string(),
            Some("cynic runtime validate --json".to_string()),
        )
    };

    Ok(RuntimeValidationReport {
<<<<<<< HEAD
        schema: cynic_protocol::RUNTIME_VALIDATION_SCHEMA,
=======
        schema: "cynic.runtime_validation.v1",
>>>>>>> github/main
        status,
        registry_path: registry_path.to_string(),
        lab_registry_path: lab_registry_path.to_string(),
        rule: "validation must execute runtime surfaces and accept honest blocked states, but fail portability leaks",
        lifecycle_status,
        lifecycle_disposition,
        next_boundary_kind,
        next_boundary_reason,
        next_command,
        steps,
    })
}
