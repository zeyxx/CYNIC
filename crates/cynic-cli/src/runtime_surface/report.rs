use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::{emit_json, fail};

const DEFAULT_RUNTIME_REPORT_PATH: &str = "/var/lib/cynic/reports/runtime-report.json";

#[derive(Debug, Serialize)]
struct RuntimeReport {
    schema: &'static str,
    status: &'static str,
    generated_at_unix_ms: u128,
    purpose: &'static str,
    phases: Vec<RuntimeReportPhase>,
    inference: InferenceReportSummary,
    persistence: RuntimeReportPersistence,
    reports: Vec<RuntimeReportSurface>,
    rules: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct RuntimeReportPersistence {
    default_path: &'static str,
    written_path: Option<String>,
    mutates_filesystem: bool,
}

#[derive(Debug, Serialize)]
struct RuntimeReportPhase {
    phase: &'static str,
    environment: &'static str,
    authority: &'static str,
    isolation_boundary: &'static str,
    target_scope: &'static str,
    mutates_host_network: bool,
    required_reports: Vec<&'static str>,
    promotion_gate: &'static str,
}

#[derive(Debug, Serialize)]
struct InferenceReportSummary {
    schema: &'static str,
    status: &'static str,
    config_path: Option<String>,
    backend_count: usize,
    sovereign_count: usize,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct RuntimeReportSurface {
    id: &'static str,
    command: &'static str,
    phase: &'static str,
    mutates: bool,
    purpose: &'static str,
}

pub(crate) fn execute_report(json: bool, output: Option<String>, persist: bool) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }

    match build_runtime_report() {
        Ok(mut report) => {
            let output_path = output
                .map(PathBuf::from)
                .or_else(|| persist.then(|| PathBuf::from(DEFAULT_RUNTIME_REPORT_PATH)));
            if let Some(path) = output_path.as_ref()
                && let Err(error) = write_report(path, &mut report)
            {
                return fail(error);
            }
            emit_json(&report, false)
        }
        Err(error) => fail(error),
    }
}

fn build_runtime_report() -> Result<RuntimeReport, String> {
    let harness = cynic_host::runtime_harness::runtime_harness_catalog(None)?;
    let phases = harness
        .contexts
        .iter()
        .map(|context| RuntimeReportPhase {
            phase: context.kind.as_str(),
            environment: context.role,
            authority: match context.authority {
                cynic_host::runtime_harness::HarnessAuthority::TestProjection => "test_projection",
                cynic_host::runtime_harness::HarnessAuthority::ConformanceGate => {
                    "conformance_gate"
                }
                cynic_host::runtime_harness::HarnessAuthority::ReleaseCandidate => {
                    "release_candidate"
                }
                cynic_host::runtime_harness::HarnessAuthority::FleetCanary => "fleet_canary",
                cynic_host::runtime_harness::HarnessAuthority::RuntimeAuthority => {
                    "runtime_authority"
                }
            },
            isolation_boundary: context.isolation_boundary,
            target_scope: context.target_scope,
            mutates_host_network: context.mutates_host_network,
            required_reports: required_reports_for_phase(context.kind.as_str()),
            promotion_gate: promotion_gate_for_phase(context.kind.as_str()),
        })
        .collect();

    let inference = inference_summary();
    let status = if inference.status == "error" {
        "blocked"
    } else {
        "ok"
    };

    Ok(RuntimeReport {
        schema: "cynic.runtime.report.v1",
        status,
        generated_at_unix_ms: now_unix_ms(),
        purpose: "human-readable automation index for runtime phases, evidence commands, and inference SSOT state",
        phases,
        inference,
        persistence: RuntimeReportPersistence {
            default_path: DEFAULT_RUNTIME_REPORT_PATH,
            written_path: None,
            mutates_filesystem: false,
        },
        reports: report_surfaces(),
        rules: vec![
            "reports are read-only unless their command explicitly says otherwise",
            "lab and conformance are proof environments, not runtime authority",
            "canary and production may mutate real host/network state only under declared authority",
            "inference config reports must never emit API key values",
            "promotion requires phase-local proof plus residual unknowns made explicit by metrics",
            "persistent runtime reports belong under /var/lib/cynic/reports, not under a user home",
        ],
    })
}

fn write_report(path: &Path, report: &mut RuntimeReport) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "runtime report output path has no parent: {}",
            path.display()
        )
    })?;
    std::fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create report directory {}: {error}",
            parent.display()
        )
    })?;

    report.persistence.written_path = Some(path.display().to_string());
    report.persistence.mutates_filesystem = true;
    let payload = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("failed to serialize runtime report: {error}"))?;
    std::fs::write(path, payload)
        .map_err(|error| format!("failed to write runtime report {}: {error}", path.display()))
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn required_reports_for_phase(phase: &str) -> Vec<&'static str> {
    match phase {
        "closed_local_lab" => vec![
            "runtime_harness_catalog",
            "runtime_inference_config",
            "runtime_validate",
        ],
        "conformance" => vec![
            "runtime_harness_catalog",
            "host_conformance",
            "host_runtime_metrics",
            "runtime_inference_config",
        ],
        "staging" => vec![
            "host_conformance",
            "host_runtime_metrics",
            "runtime_inference_config",
            "runtime_proof_bundle",
        ],
        "canary" => vec![
            "host_runtime_metrics",
            "runtime_inference_config",
            "runtime_proof_bundle",
            "rollback_proof",
        ],
        "production" => vec![
            "host_runtime_metrics",
            "runtime_inference_config",
            "runtime_proof_bundle",
            "persistent_evidence",
        ],
        _ => vec!["runtime_harness_catalog"],
    }
}

fn promotion_gate_for_phase(phase: &str) -> &'static str {
    match phase {
        "closed_local_lab" => {
            "prove config, bootstrap, and inference wiring without host/network mutation"
        }
        "conformance" => "qualify one candidate node with conformance and metrics before staging",
        "staging" => "prove release candidate behavior under production-like isolation",
        "canary" => "prove bounded real-fleet behavior with rollback evidence",
        "production" => "maintain runtime authority with persistent metrics and evidence",
        _ => "unknown phase",
    }
}

fn inference_summary() -> InferenceReportSummary {
    match cynic_inference::load_inference_config() {
        Ok(loaded) => {
            let sovereign_count = loaded
                .backends
                .iter()
                .filter(|backend| backend.sovereign)
                .count();
            InferenceReportSummary {
                schema: "cynic.inference_config.v1",
                status: "ok",
                config_path: loaded.path.map(|path| path.to_string_lossy().to_string()),
                backend_count: loaded.backends.len(),
                sovereign_count,
                error: None,
            }
        }
        Err(cynic_inference::InferenceConfigError::NotFound { path }) => InferenceReportSummary {
            schema: "cynic.inference_config.v1",
            status: "not_found",
            config_path: Some(path.to_string_lossy().to_string()),
            backend_count: 0,
            sovereign_count: 0,
            error: None,
        },
        Err(error) => InferenceReportSummary {
            schema: "cynic.inference_config.v1",
            status: "error",
            config_path: None,
            backend_count: 0,
            sovereign_count: 0,
            error: Some(error.to_string()),
        },
    }
}

fn report_surfaces() -> Vec<RuntimeReportSurface> {
    vec![
        RuntimeReportSurface {
            id: "runtime_report",
            command: "cynic runtime report --json [--output <path> | --persist]",
            phase: "all",
            mutates: false,
            purpose: "aggregate phase and report index for humans and automation; --output/--persist write a JSON artifact",
        },
        RuntimeReportSurface {
            id: "runtime_harness_catalog",
            command: "cynic runtime harness --json",
            phase: "all",
            mutates: false,
            purpose: "list execution membranes and authority progression",
        },
        RuntimeReportSurface {
            id: "host_conformance",
            command: "cynic runtime conformance --json",
            phase: "conformance",
            mutates: false,
            purpose: "observe host qualification gates",
        },
        RuntimeReportSurface {
            id: "host_runtime_metrics",
            command: "cynic runtime metrics --json",
            phase: "conformance",
            mutates: false,
            purpose: "measure known coverage, maturity, and residual unknowns",
        },
        RuntimeReportSurface {
            id: "runtime_inference_config",
            command: "cynic runtime inference --json",
            phase: "all",
            mutates: false,
            purpose: "report sovereign inference SSOT without leaking secrets",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_RUNTIME_REPORT_PATH;

    #[test]
    fn default_runtime_report_path_is_installed_state_not_user_home() {
        assert_eq!(
            DEFAULT_RUNTIME_REPORT_PATH,
            "/var/lib/cynic/reports/runtime-report.json"
        );
        assert!(!DEFAULT_RUNTIME_REPORT_PATH.contains("/home/"));
        assert!(!DEFAULT_RUNTIME_REPORT_PATH.contains("~"));
    }
}
