//! Evaluation of the immutable vertical contract catalog against local proof.

use serde::Serialize;

use cynic_protocol::{
    ConvergenceCell, ConvergenceMatrixReport, ConvergenceStatus, assess_convergence_matrix,
    framework_spec,
};

use crate::conformance::HostConformanceGate;
use crate::build::BuildProof;
use crate::repo_access::current_repo_access_status;
use crate::runtime_harness::RuntimeHarnessContext;
use crate::source_control::observe_source_worktree;

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
}

pub fn observe_vertical_evidence(build_proof: Option<&BuildProof>) -> VerticalEvidenceInput {
    let repo_access = current_repo_access_status();
    let (source_status, source_blockers) = repo_access.canonical_project_root.as_deref().map_or(
        ("unknown", vec!["canonical_project_root_missing".to_string()]),
        |root| {
            let worktree = observe_source_worktree(
                std::path::Path::new(root),
                Some("cynic"),
            );
            if worktree.status == "ready" {
                ("partial", worktree.blockers)
            } else {
                ("blocked", worktree.blockers)
            }
        },
    );
    let (build_status, build_blockers) = match build_proof {
        Some(proof) if proof.status == "proved" => ("complete", Vec::new()),
        Some(proof) if proof.status == "failed" => ("blocked", proof.blockers.clone()),
        Some(proof) => ("unknown", proof.blockers.clone()),
        None => ("unknown", vec!["build_proof_not_consumed".to_string()]),
    };
    VerticalEvidenceInput {
        source_status,
        source_blockers,
        build_status,
        build_blockers,
    }
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
        report.verticals.iter().map(|entry| entry.id.clone()).collect(),
        cells,
    )
}

pub fn evaluate_vertical_conformance(
    context: &RuntimeHarnessContext,
    gates: &[HostConformanceGate],
    evidence: &VerticalEvidenceInput,
) -> VerticalConformanceReport {
    let catalog = framework_spec();
    let identity = gate_status(gates, "host_identity_observed");
    let services = gate_status(gates, "service_surface_observed");
    let host = gate_status(gates, "harness_is_conformance_gate");
    let bootstrap = gate_status(gates, "bootstrap_target_reached");
    let hardware = gate_status(gates, "hardware_stability_qualified");
    let mut entries = Vec::new();
    let mut unknowns = Vec::new();
    let mut blockers = Vec::new();
    for contract in catalog.vertical_contracts {
        let (status, mut entry_blockers, refs) = match contract.id {
            "process" | "principal" => (identity, vec![], vec!["host_identity_observed".to_string()]),
            "filesystem" => (bootstrap, vec![], vec!["bootstrap_target_reached".to_string()]),
            "source" => (evidence.source_status, evidence.source_blockers.clone(), vec!["source_worktree_observation".to_string()]),
            "build" => (evidence.build_status, evidence.build_blockers.clone(), vec![]),
            "artifact" => (bootstrap, vec![], vec!["bootstrap_target_reached".to_string()]),
            "service" => (services, vec![], vec!["service_surface_observed".to_string()]),
            "host_os" => (host, vec![], vec!["harness_is_conformance_gate".to_string()]),
            "network" => ("unknown", vec!["network_vertical_not_consumed_by_host_conformance".to_string()], vec![]),
            "node" => (identity, vec![], vec!["host_identity_observed".to_string()]),
            "provider" | "orchestrator" | "federation" => ("unknown", vec!["adapter_not_observed".to_string()], vec![]),
            "project" => (evidence.source_status, evidence.source_blockers.clone(), vec!["source_worktree_observation".to_string()]),
            "fleet" => ("unknown", vec!["fleet_vertical_requires_multi_node_proof".to_string()], vec![]),
            "human_boundary" => (hardware, vec![], vec!["hardware_stability_qualified".to_string()]),
            _ => ("unknown", vec!["catalog_vertical_not_mapped".to_string()], vec![]),
        };
        if status == "unknown" { unknowns.push(contract.id.to_string()); }
        if !entry_blockers.is_empty() { blockers.extend(entry_blockers.clone()); }
        entries.push(VerticalConformanceEntry {
            id: contract.id.to_string(),
            declared_coverage: contract.coverage.to_string(),
            status,
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
        catalog_status: "consumed",
        verticals: entries,
        residual_unknowns: unknowns,
        blockers,
    }
}

fn gate_status(gates: &[HostConformanceGate], id: &str) -> &'static str {
    match gates.iter().find(|gate| gate.id == id).map(|gate| gate.status) {
        Some("satisfied") => "partial",
        Some("blocked") => "blocked",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_harness::conformance_context;

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
            },
        );
        assert_eq!(report.catalog_status, "consumed");
        assert!(report.residual_unknowns.contains(&"provider".to_string()));
        assert!(report.residual_unknowns.contains(&"orchestrator".to_string()));
    }
}
