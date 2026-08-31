//! Source worktree opportunity observer.
//!
//! Observes repository worktrees to surface unfinished work slices, detached
//! worktrees, and layout anomalies that block operator capacity.

use std::path::{Path, PathBuf};

use cynic_protocol::organic_opportunity::{
    EnergyEstimate, ORGANIC_OPPORTUNITY_SCHEMA, OpportunityClass, OpportunitySource,
    OpportunitySourceKind, OrganicOpportunity, SmallestNextStep,
};
use cynic_protocol::{
    SourceWorktreeInventoryEntry, SourceWorktreeInventoryEntryState,
    SourceWorktreeInventoryPathBasis,
};

use super::port::{OrganicOpportunityObserver, SourceObservationStatus};
use crate::source_worktree_inventory::{
    LocalSourceWorktreeInventoryRunner, SourceWorktreeInventoryRunner,
};

/// Configuration for source worktree observation.
#[derive(Debug, Clone)]
pub struct WorktreeObserverConfig {
    pub repository_root: PathBuf,
    pub workspace_store: PathBuf,
}

impl Default for WorktreeObserverConfig {
    fn default() -> Self {
        Self {
            repository_root: PathBuf::from("."),
            workspace_store: PathBuf::from(".workspace"),
        }
    }
}

/// Host adapter observing repository worktrees.
pub struct SourceWorktreeObserver<R: SourceWorktreeInventoryRunner> {
    config: WorktreeObserverConfig,
    runner: R,
}

impl SourceWorktreeObserver<LocalSourceWorktreeInventoryRunner> {
    pub fn new(config: WorktreeObserverConfig) -> Self {
        Self {
            config,
            runner: LocalSourceWorktreeInventoryRunner,
        }
    }
}

impl<R: SourceWorktreeInventoryRunner> SourceWorktreeObserver<R> {
    pub fn with_runner(config: WorktreeObserverConfig, runner: R) -> Self {
        Self { config, runner }
    }

    /// Pure analysis: turns inventory entries into opportunities.
    pub fn analyze_entries(
        entries: &[SourceWorktreeInventoryEntry],
        _now_unix_s: u64,
    ) -> Vec<OrganicOpportunity> {
        let mut opportunities = Vec::new();

        for entry in entries {
            // Unfinished agent slice detection
            if let Some(branch) = &entry.branch {
                if branch.starts_with("agent/") || branch.starts_with("feature/") {
                    // Agent branch in auxiliary worktree => candidate for unfinished consolidation
                    if entry.path_basis == SourceWorktreeInventoryPathBasis::WorkspaceLayout {
                        let rel = entry
                            .relative_path
                            .as_deref()
                            .unwrap_or_else(|| branch.as_str());
                        let opp = OrganicOpportunity {
                            schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
                            opportunity_id: format!(
                                "opp:unfinished_worktree:{}",
                                &entry.evidence_digest[..16.min(entry.evidence_digest.len())]
                            ),
                            scope: format!("worktree/{}", rel),
                            sources: vec![OpportunitySource {
                                kind: OpportunitySourceKind::SourceWorktree,
                                ref_id: entry.evidence_digest.clone(),
                                label: Some(format!("branch: {}", branch)),
                            }],
                            class: OpportunityClass::Unfinished,
                            expected_unlock: format!(
                                "Consolidate or merge active agent branch '{}' in worktree '{}'",
                                branch, rel
                            ),
                            estimated_energy: EnergyEstimate::Range {
                                method: "git worktree merge and cleanup".to_string(),
                                low_joules: 50.0,
                                high_joules: 500.0,
                            },
                            freshness_seconds: 0,
                            confidence: 0.85,
                            smallest_next_step: SmallestNextStep::Consolidate,
                            residual_unknowns: vec![
                                format!("head: {:?}", entry.head),
                                format!("locked: {:?}", entry.locked),
                            ],
                            operator_disposition: Default::default(),
                        };
                        opportunities.push(opp);
                    }
                }
            }

            // Malformed or outside layout worktree => plumbing anomaly
            if entry.state == SourceWorktreeInventoryEntryState::OutsideDeclaredLayout
                || entry.state == SourceWorktreeInventoryEntryState::Malformed
            {
                let opp = OrganicOpportunity {
                    schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
                    opportunity_id: format!(
                        "opp:worktree_anomaly:{}",
                        &entry.evidence_digest[..16.min(entry.evidence_digest.len())]
                    ),
                    scope: "infra/worktree_layout".to_string(),
                    sources: vec![OpportunitySource {
                        kind: OpportunitySourceKind::SourceWorktree,
                        ref_id: entry.evidence_digest.clone(),
                        label: Some(format!("state: {:?}", entry.state)),
                    }],
                    class: OpportunityClass::Plumbing,
                    expected_unlock: "Reconcile undeclared or malformed git worktree layout"
                        .to_string(),
                    estimated_energy: EnergyEstimate::Unknown {
                        reason: "layout repair requires inspecting unmanaged worktree directory"
                            .to_string(),
                    },
                    freshness_seconds: 0,
                    confidence: 0.90,
                    smallest_next_step: SmallestNextStep::ProposeRepair,
                    residual_unknowns: vec![format!("entry_state: {:?}", entry.state)],
                    operator_disposition: Default::default(),
                };
                opportunities.push(opp);
            }
        }

        opportunities
    }
}

impl<R: SourceWorktreeInventoryRunner> OrganicOpportunityObserver for SourceWorktreeObserver<R> {
    fn observer_name(&self) -> &'static str {
        "source_worktree_observer"
    }

    fn source_kind(&self) -> OpportunitySourceKind {
        OpportunitySourceKind::SourceWorktree
    }

    fn observe(&self) -> SourceObservationStatus<Vec<OrganicOpportunity>> {
        let output = self
            .runner
            .run(&["git", "worktree", "list", "--porcelain", "-z"]);

        if let Some(err) = output.spawn_error {
            return SourceObservationStatus::Unavailable {
                kind: OpportunitySourceKind::SourceWorktree,
                reason: err,
            };
        }

        if output.exit_code != Some(0) {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return SourceObservationStatus::Unavailable {
                kind: OpportunitySourceKind::SourceWorktree,
                reason: format!("git worktree list failed: {}", stderr),
            };
        }

        let raw_text = match std::str::from_utf8(&output.stdout) {
            Ok(s) => s,
            Err(_) => {
                return SourceObservationStatus::Unavailable {
                    kind: OpportunitySourceKind::SourceWorktree,
                    reason: "non-utf8 porcelain output".to_string(),
                };
            }
        };

        let (entries, _exceeded, _malformed) = crate::source_worktree_inventory::parse_porcelain(
            raw_text,
            &self.config.repository_root,
            &self.config.workspace_store,
        );

        if entries.is_empty() {
            return SourceObservationStatus::Empty;
        }

        let opps = Self::analyze_entries(&entries, 0);
        if opps.is_empty() {
            SourceObservationStatus::Empty
        } else {
            SourceObservationStatus::Observed { data: opps }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_entry(
        digest: &str,
        branch: Option<&str>,
        basis: SourceWorktreeInventoryPathBasis,
        state: SourceWorktreeInventoryEntryState,
        rel: Option<&str>,
    ) -> SourceWorktreeInventoryEntry {
        SourceWorktreeInventoryEntry {
            candidate_id: digest.to_string(),
            evidence_digest: digest.to_string(),
            state,
            unknown_attribute_count: 0,
            path_basis: basis,
            relative_path: rel.map(String::from),
            head: Some("abc1234".to_string()),
            branch: branch.map(String::from),
            observed_principal: None,
            path_is_symlink: None,
            git_registered: Some(true),
            git_prunable: None,
            detached_head: None,
            clean: None,
            locked: None,
            bare: None,
            lock_reason: None,
            prunable_reason: None,
            unknowns: vec![],
        }
    }

    #[test]
    fn agent_branch_in_workspace_creates_unfinished_opportunity() {
        let entry = mk_entry(
            "digest1234567890abcdef",
            Some("agent/hermes/v1-debt-purge"),
            SourceWorktreeInventoryPathBasis::WorkspaceLayout,
            SourceWorktreeInventoryEntryState::Observed,
            Some("CYNIC-agent-v1"),
        );

        let opps = SourceWorktreeObserver::<LocalSourceWorktreeInventoryRunner>::analyze_entries(
            &[entry],
            0,
        );
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].class, OpportunityClass::Unfinished);
        assert_eq!(opps[0].scope, "worktree/CYNIC-agent-v1");
        assert_eq!(opps[0].smallest_next_step, SmallestNextStep::Consolidate);
    }

    #[test]
    fn outside_layout_worktree_creates_plumbing_opportunity() {
        let entry = mk_entry(
            "digest_anomaly_12345678",
            Some("main"),
            SourceWorktreeInventoryPathBasis::Unknown,
            SourceWorktreeInventoryEntryState::OutsideDeclaredLayout,
            None,
        );

        let opps = SourceWorktreeObserver::<LocalSourceWorktreeInventoryRunner>::analyze_entries(
            &[entry],
            0,
        );
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].class, OpportunityClass::Plumbing);
        assert_eq!(opps[0].scope, "infra/worktree_layout");
    }
}
