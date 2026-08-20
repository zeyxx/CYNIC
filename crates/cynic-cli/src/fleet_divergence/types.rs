//! Pure read-back shapes for the fleet-divergence surface.
//!
//! Serialization only: no filesystem, no process, no policy. The verdicts these
//! carry were decided in `cynic_protocol::registry_claim_audit`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use cynic_protocol::registry_claim_audit::ClaimAuditSummary;

pub(crate) const FLEET_DIVERGENCE_SCHEMA: &str = "cynic.cli.fleet_divergence.v1";

/// One audited registry claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ClaimRow {
    pub node_id: String,
    pub claim_kind: String,
    pub declared: String,
    /// The registry marks this entry unproven, so a contradiction is stale
    /// intent rather than an asserted falsehood.
    pub declared_as_candidate: bool,
    pub verdict: String,
    pub detail: String,
    pub error_codes: Vec<String>,
}

/// One audited name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NameRow {
    pub name: String,
    pub answers: Vec<String>,
    /// Vantage points that could not resolve — unknown, never counted as
    /// agreement.
    pub unresolved_from: Vec<String>,
    pub verdict: String,
    pub detail: String,
    pub error_codes: Vec<String>,
}

/// The read-back report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FleetDivergenceReport {
    pub schema: String,
    /// `observed` when claims were probed, `blocked` when the registry could not
    /// be read. Never `ok` on an unread registry.
    pub status: String,
    pub mutates: bool,
    pub registry_path: String,
    pub summary: Option<ClaimAuditSummary>,
    pub claims: Vec<ClaimRow>,
    pub names: Vec<NameRow>,
    /// What this audit could not establish. Present even on success: coverage is
    /// partial by construction and hiding that would overstate the result.
    pub residual_unknowns: Vec<String>,
}

impl FleetDivergenceReport {
    pub(crate) fn blocked(registry_path: &Path, error: String) -> Self {
        Self {
            schema: FLEET_DIVERGENCE_SCHEMA.to_string(),
            status: "blocked".to_string(),
            mutates: false,
            registry_path: registry_path.display().to_string(),
            summary: None,
            claims: Vec::new(),
            names: Vec::new(),
            residual_unknowns: vec![
                error,
                "no claim was audited, so this is not evidence the fleet agrees with its registry"
                    .to_string(),
            ],
        }
    }

    pub(crate) fn observed(
        registry_path: &Path,
        summary: ClaimAuditSummary,
        claims: Vec<ClaimRow>,
        names: Vec<NameRow>,
    ) -> Self {
        let mut residual_unknowns = Vec::new();

        if summary.unobservable > 0 {
            residual_unknowns.push(format!(
                "{} claim(s) could not be checked; unknown is not agreement",
                summary.unobservable
            ));
        }
        if names.iter().any(|row| row.verdict == "undecidable") {
            residual_unknowns.push(
                "name consistency needs at least 2 vantage points; only this host was asked, \
                 so a name every peer resolves differently still reads as undecidable"
                    .to_string(),
            );
        }
        residual_unknowns.push(
            "only synapse_address claims are probed; role_host and service_binding need a \
             credential this read-back surface does not hold"
                .to_string(),
        );

        Self {
            schema: FLEET_DIVERGENCE_SCHEMA.to_string(),
            status: "observed".to_string(),
            mutates: false,
            registry_path: registry_path.display().to_string(),
            summary: Some(summary),
            claims,
            names,
            residual_unknowns,
        }
    }

    pub(crate) fn summary_refuted(&self) -> usize {
        self.summary.as_ref().map(|s| s.refuted).unwrap_or(0)
    }

    pub(crate) fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "fleet divergence — {} ({})\n",
            self.status, self.registry_path
        ));

        if let Some(summary) = &self.summary {
            out.push_str(&format!(
                "claims: {} total | {} confirmed | {} refuted | {} unobservable\n",
                summary.total, summary.confirmed, summary.refuted, summary.unobservable
            ));
            if summary.asserted_refutations > 0 {
                out.push_str(&format!(
                    "  {} refutation(s) are asserted, not candidate — one side is wrong\n",
                    summary.asserted_refutations
                ));
            }
        }

        for row in &self.claims {
            let candidate = if row.declared_as_candidate {
                " [candidate]"
            } else {
                ""
            };
            out.push_str(&format!(
                "  {:<12} {:<16} {}{}\n      {} — {}\n",
                row.verdict, row.claim_kind, row.node_id, candidate, row.declared, row.detail
            ));
        }

        for row in &self.names {
            out.push_str(&format!(
                "  {:<12} name {}\n      {}\n",
                row.verdict, row.name, row.detail
            ));
            for unresolved in &row.unresolved_from {
                out.push_str(&format!("      unresolved: {unresolved}\n"));
            }
        }

        if !self.residual_unknowns.is_empty() {
            out.push_str("residual unknowns:\n");
            for unknown in &self.residual_unknowns {
                out.push_str(&format!("  - {unknown}\n"));
            }
        }

        out
    }
}
