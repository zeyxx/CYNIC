//! Pure data-centric projection of Source Sync observations and decisions into canonical CoherenceVector.
//!
//! Grounded in cynic-eval::convergence (Foundation §481).

use cynic_eval::convergence::{CoherenceVector, Compass, ConvergenceStatus, Vector};
use cynic_vcs::sync::{SyncDecision, SyncObservation};

pub const SOURCE_SYNC_REPORT_SCHEMA: &str = "cynic.source_sync.report.v1";

/// Projects a SourceSync observation and decision into a canonical CoherenceVector.
pub(crate) fn project_sync_coherence_vector(
    observation: &SyncObservation,
    decision: &SyncDecision,
    read_back_digest: Option<&str>,
) -> CoherenceVector {
    // 1. Determine subject_id (repository + branch)
    let branch = observation
        .cortex
        .branch
        .as_deref()
        .or(observation.forgejo.branch.as_deref())
        .unwrap_or("main");
    let subject_id = format!("source:CYNIC:{branch}");

    // 2. Identify the desired state digest (SSOT is Forgejo, then Cortex)
    let desired_sha = observation
        .forgejo
        .head_sha
        .as_deref()
        .or(observation.cortex.head_sha.as_deref())
        .unwrap_or("0000000000000000000000000000000000000000");

    let compass = Compass {
        schema: "cynic.compass.v1".to_string(),
        desired_state_digest: desired_sha.to_string(),
        subject_id,
        vertical_id: "source".to_string(),
    };

    // 3. Collect blockers & unknowns
    let mut blockers = Vec::new();
    let mut residual_unknowns = Vec::new();

    // Check dirty files or unmerged conflicts
    if !observation.cortex.dirty_files.is_empty() {
        blockers.push(format!(
            "cortex has {} dirty files: {}",
            observation.cortex.dirty_files.len(),
            observation.cortex.dirty_files.join(", ")
        ));
    }
    for b in &observation.cortex.blockers {
        if !blockers.contains(b) {
            blockers.push(b.clone());
        }
    }
    for b in &observation.forgejo.blockers {
        if !blockers.contains(b) {
            blockers.push(b.clone());
        }
    }

    // Residual unknowns (e.g. unreachable remote relays)
    for u in &decision.unknowns {
        residual_unknowns.push(u.clone());
    }

    // 4. Derive ConvergenceStatus and CoherenceDecision
    let (status, decision_label, action_desc) = if !blockers.is_empty() {
        (
            ConvergenceStatus::Blocked,
            "blocked",
            Some(format!("Resolve blockers: {}", blockers.join("; "))),
        )
    } else if !decision.actions.is_empty() {
        let action_str = decision
            .actions
            .iter()
            .map(|a| format!("{a:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        (ConvergenceStatus::Partial, "planned", Some(action_str))
    } else if !residual_unknowns.is_empty()
        && (!observation.forgejo.reachable || !observation.cortex.reachable)
    {
        (
            ConvergenceStatus::Unknown,
            "unknown",
            Some("Essential source planes unreachable".to_string()),
        )
    } else {
        (ConvergenceStatus::Complete, "converged", None)
    };

    let vector = Vector {
        decision: decision_label.to_string(),
        action: action_desc,
        read_back_digest: read_back_digest.map(|s| s.to_string()),
    };

    CoherenceVector {
        schema: "cynic.coherence_vector.v1".to_string(),
        compass,
        vector,
        status,
        blockers,
        residual_unknowns,
    }
}

/// A structured, data-centric report packaging the CoherenceVector with execution artifacts.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SourceSyncReport {
    pub schema: &'static str,
    pub coherence: CoherenceVector,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<cynic_vcs::sync::SyncPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<cynic_vcs::sync::SyncProof>,
}
