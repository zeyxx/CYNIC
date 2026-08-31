//! Read-back surface: registry claims versus observed substrate.
//!
//! Projection only. Every verdict comes from
//! `cynic_protocol::registry_claim_audit` and every observation from
//! `cynic_host::registry_divergence_probe`; this module extracts claims from the
//! registry document and formats what those layers decided.
//!
//! Read-only end to end. The probes issue queries (`ping`, `getent`, `curl -o
//! /dev/null`) and nothing here writes to the registry, so an audit can never
//! mutate the fleet it audits.

use std::path::PathBuf;
use std::process::ExitCode;

use cynic_host::registry_divergence_probe::{
    assemble_name_observation, probe_address_reachable, resolve_name_here,
};
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::registry_claim_audit::{
    ClaimKind, ClaimVerdict, NameVerdict, claim_to_signal_payload, compare_declarative_sources,
    judge_claim, judge_name, name_to_signal_payload, summarize_claims,
};

use crate::cli_json::emit_pretty_json;

mod scan;
mod types;

pub(crate) use types::{ClaimRow, FleetDivergenceReport, NameRow};

/// Default registry location, repo-relative so no operator path is compiled in.
const DEFAULT_REGISTRY: &str = "infra/registry.json";

/// The seed table the mesh resolver reads in preference to its compiled const.
/// Resolved beside the registry so a caller auditing a copied registry compares
/// against that copy's seed, not the repo's.
const DEFAULT_SEED: &str = "synapse-nodes.seed.json";

pub(crate) fn run(json: bool, registry: Option<PathBuf>, names: Vec<String>) -> ExitCode {
    let registry_path = registry.unwrap_or_else(|| PathBuf::from(DEFAULT_REGISTRY));

    // A missing registry is a blocked read-back with the path named, never an
    // empty "all clear": zero claims audited must not look like zero problems.
    let claims = match scan::extract_claims(&registry_path) {
        Ok(claims) => claims,
        Err(error) => {
            let report = FleetDivergenceReport::blocked(&registry_path, error);
            return emit(json, &report);
        }
    };

    let runner = LocalArgvRunner;
    let mut rows = Vec::new();
    let mut verdicts = Vec::new();

    for claim in &claims {
        let observation = probe_address_reachable(&runner, claim);
        let verdict = judge_claim(&observation);
        let signal = claim_to_signal_payload(&observation, &verdict);

        rows.push(ClaimRow {
            node_id: claim.node_id.clone(),
            claim_kind: claim.kind.as_str().to_string(),
            declared: claim.declared.clone(),
            declared_as_candidate: claim.declared_as_candidate,
            verdict: verdict_label(&verdict).to_string(),
            detail: verdict_detail(&verdict),
            error_codes: signal
                .map(|payload| {
                    payload
                        .error_codes()
                        .into_iter()
                        .map(|code| code.code().to_string())
                        .collect()
                })
                .unwrap_or_default(),
        });
        verdicts.push(verdict);
    }

    // Declarative divergence needs no probe: it is a disagreement between two
    // files, and every address involved answers. A seed table that contradicts
    // the registry is a competing authority, so it is audited here rather than
    // trusted as a fallback.
    let seed_path = registry_path
        .parent()
        .map(|dir| dir.join(DEFAULT_SEED))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SEED));

    if let Ok(registry_views) = scan::extract_registry_views(&registry_path) {
        match scan::extract_seed_views(&seed_path) {
            // No seed file: the resolver uses its compiled const. Absence of a
            // second source is not a divergence.
            Ok(None) => {}
            Ok(Some(seed_views)) => {
                for observation in compare_declarative_sources(&registry_views, &seed_views) {
                    let verdict = judge_claim(&observation);
                    let signal = claim_to_signal_payload(&observation, &verdict);

                    rows.push(ClaimRow {
                        node_id: observation.node_id.clone(),
                        claim_kind: observation.kind.as_str().to_string(),
                        declared: observation.declared.clone(),
                        declared_as_candidate: observation.declared_as_candidate,
                        verdict: verdict_label(&verdict).to_string(),
                        detail: verdict_detail(&verdict),
                        error_codes: signal
                            .map(|payload| {
                                payload
                                    .error_codes()
                                    .into_iter()
                                    .map(|code| code.code().to_string())
                                    .collect()
                            })
                            .unwrap_or_default(),
                    });
                    verdicts.push(verdict);
                }
            }
            // A corrupt seed is reported as an unobservable claim, not skipped:
            // the resolver will still consume something, and silence here would
            // hide which source it used.
            Err(error) => {
                let verdict = ClaimVerdict::Unobservable {
                    reason: error.clone(),
                };
                rows.push(ClaimRow {
                    node_id: "-".to_string(),
                    claim_kind: ClaimKind::DeclarativeAgreement.as_str().to_string(),
                    declared: seed_path.display().to_string(),
                    declared_as_candidate: false,
                    verdict: verdict_label(&verdict).to_string(),
                    detail: verdict_detail(&verdict),
                    error_codes: Vec::new(),
                });
                verdicts.push(verdict);
            }
        }
    }

    // Names are audited from this host only unless the caller supplies more
    // vantage points, so a single-observer answer stays Undecidable rather than
    // certifying the fleet from one resolver.
    let mut name_rows = Vec::new();
    for name in &names {
        let local = resolve_name_here(&runner, "local", name);
        let observation = assemble_name_observation(name, vec![local]);
        let verdict = judge_name(&observation);
        let signal = name_to_signal_payload(&observation, &verdict);

        name_rows.push(NameRow {
            name: name.clone(),
            answers: observation
                .answers
                .iter()
                .map(|(observer, value)| format!("{observer}={value}"))
                .collect(),
            unresolved_from: observation
                .unresolved
                .iter()
                .map(|(observer, reason)| format!("{observer}: {reason}"))
                .collect(),
            verdict: name_verdict_label(&verdict).to_string(),
            detail: name_verdict_detail(&verdict),
            error_codes: signal
                .map(|payload| {
                    payload
                        .error_codes()
                        .into_iter()
                        .map(|code| code.code().to_string())
                        .collect()
                })
                .unwrap_or_default(),
        });
    }

    let report = FleetDivergenceReport::observed(
        &registry_path,
        summarize_claims(&verdicts),
        rows,
        name_rows,
    );
    emit(json, &report)
}

fn emit(json: bool, report: &FleetDivergenceReport) -> ExitCode {
    if json {
        emit_pretty_json(report)
    } else {
        print!("{}", report.render_text());
        // A refuted claim is a finding the operator must act on, so the exit
        // code carries it. Unobservable does not fail: unknown is not a fault.
        if report.summary_refuted() > 0 {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}

fn verdict_label(verdict: &ClaimVerdict) -> &'static str {
    match verdict {
        ClaimVerdict::Confirmed => "confirmed",
        ClaimVerdict::Refuted { .. } => "refuted",
        ClaimVerdict::Unobservable { .. } => "unobservable",
    }
}

fn verdict_detail(verdict: &ClaimVerdict) -> String {
    match verdict {
        ClaimVerdict::Confirmed => "substrate agrees with the registry".to_string(),
        ClaimVerdict::Refuted {
            declared, observed, ..
        } => format!("registry says {declared}, substrate reports {observed}"),
        ClaimVerdict::Unobservable { reason } => reason.clone(),
    }
}

fn name_verdict_label(verdict: &NameVerdict) -> &'static str {
    match verdict {
        NameVerdict::Consistent { .. } => "consistent",
        NameVerdict::Inconsistent { .. } => "inconsistent",
        NameVerdict::Undecidable { .. } => "undecidable",
    }
}

fn name_verdict_detail(verdict: &NameVerdict) -> String {
    match verdict {
        NameVerdict::Consistent { value, observers } => {
            format!("{observers} vantage point(s) agree on {value}")
        }
        NameVerdict::Inconsistent { distinct_values } => {
            format!(
                "{} distinct answers: {}",
                distinct_values.len(),
                distinct_values.join(", ")
            )
        }
        NameVerdict::Undecidable { reason } => reason.clone(),
    }
}
