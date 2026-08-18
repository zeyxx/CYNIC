//! Evaluation of the immutable vertical contract catalog against local proof.

use serde::Serialize;

use cynic_protocol::{
    ConvergenceCell, ConvergenceMatrixReport, ConvergenceStatus, FrameworkSpec,
    assess_convergence_matrix, framework_spec,
};

use crate::build::BuildProof;
use crate::conformance::HostConformanceGate;
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

pub fn observe_vertical_evidence(build_proof: Option<&BuildProof>) -> VerticalEvidenceInput {
    let repo_access = current_repo_access_status();
    let (source_status, source_blockers) = repo_access.canonical_project_root.as_deref().map_or(
        (
            "unknown",
            vec!["canonical_project_root_missing".to_string()],
        ),
        |root| {
            let worktree = observe_source_worktree(std::path::Path::new(root), Some("cynic"));
            if worktree.status == "ready" {
                ("partial", worktree.blockers)
            } else {
                ("blocked", worktree.blockers)
            }
        },
    );
    let (build_status, build_blockers) = match build_proof {
        Some(proof) if proof.status == "proved" || proof.status == "completed" => {
            ("complete", Vec::new())
        }
        Some(proof) if proof.status == "failed" => ("blocked", proof.blockers.clone()),
        Some(proof) => ("unknown", proof.blockers.clone()),
        None => {
            if std::path::Path::new("/var/lib/cynic/reports/build-proof.json").exists()
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
    VerticalEvidenceInput {
        source_status,
        source_blockers,
        build_status,
        build_blockers,
        network_status,
        network_blockers,
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
}
