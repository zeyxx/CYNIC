use std::time::{SystemTime, UNIX_EPOCH};

use cynic_host::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus};

use crate::runtime_surface::report::catalog::{
    promotion_gate_for_phase, report_surfaces, required_reports_for_phase,
};
use crate::runtime_surface::report::summary::{inference_summary, sovereign_git_summary};
use crate::runtime_surface::report::types::{
    RuntimeReport, RuntimeReportPersistence, RuntimeReportPhase,
};

pub(crate) fn build_runtime_report(
    default_runtime_report_path: &'static str,
) -> Result<RuntimeReport, String> {
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
    let sovereign_git = sovereign_git_summary();
    let status = if inference.status == "error" || sovereign_git.status == "blocked" {
        "blocked"
    } else {
        "ok"
    };
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
            "runtime report index is readable; inference SSOT and sovereign git authority are not in a blocked state"
                .to_string(),
            None,
        )
    } else {
        (
            LifecycleStatus::PrincipalMismatch,
            LifecycleDisposition::RequiresPrincipalAdoption,
            BoundaryKind::Adopt,
            if inference.status == "error" {
                "runtime report index is blocked by inference SSOT loading or projection errors"
                    .to_string()
            } else {
                "runtime report index is blocked by sovereign git service reachability or authority projection gaps"
                    .to_string()
            },
            if inference.status == "error" {
                Some("cynic runtime inference --json".to_string())
            } else {
                Some("cynic runtime repo-access --json".to_string())
            },
        )
    };

    Ok(RuntimeReport {
        schema: "cynic.runtime.report.v1",
        status,
        lifecycle_status,
        lifecycle_disposition,
        next_boundary_kind,
        next_boundary_reason,
        next_command,
        generated_at_unix_ms: now_unix_ms(),
        purpose: "human-readable automation index for runtime phases, evidence commands, and inference SSOT state",
        phases,
        inference,
        sovereign_git,
        persistence: RuntimeReportPersistence {
            default_path: default_runtime_report_path,
            written_path: None,
            mutates_filesystem: false,
        },
        reports: report_surfaces(),
        rules: vec![
            "reports are read-only unless their command explicitly says otherwise",
            "lab and conformance are proof environments, not runtime authority",
            "canary and production may mutate real host/network state only under declared authority",
            "inference config reports must never emit API key values",
            "sovereign git health is runtime authority infrastructure, not an external integration",
            "promotion requires phase-local proof plus residual unknowns made explicit by metrics",
            "persistent runtime reports belong under /var/lib/cynic/reports, not under a user home",
        ],
    })
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
