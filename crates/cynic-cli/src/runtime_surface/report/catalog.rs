use crate::runtime_surface::report::types::RuntimeReportSurface;

pub(crate) fn required_reports_for_phase(phase: &str) -> Vec<&'static str> {
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

pub(crate) fn promotion_gate_for_phase(phase: &str) -> &'static str {
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

pub(crate) fn report_surfaces() -> Vec<RuntimeReportSurface> {
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
            id: "storage_capabilities",
            command: "cynic runtime storage-capabilities --json",
            phase: "conformance",
            mutates: false,
            purpose: "observe storage version, engine, transport, capability evidence, and residual risk",
        },
        RuntimeReportSurface {
            id: "runtime_repo_access",
            command: "cynic runtime repo-access --json",
            phase: "all",
            mutates: false,
            purpose: "observe sovereign git remote authority, Forgejo reachability, and repo-access readiness",
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
