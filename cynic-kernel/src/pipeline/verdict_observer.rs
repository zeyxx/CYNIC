//! Verdict observation posting — the forward loop (verdict → observation → CCM).
//!
//! Critical K15 consumer: every verdict that exits /judge must POST back to /observe
//! so CCM intake can process it. Without this, verdicts accumulate locally but never
//! feed crystals. This completes the cycle: observations → judge → verdicts → observations.

use crate::domain::dog::Verdict;
use crate::domain::storage::Observation;
use chrono::Utc;

use super::PipelineDeps;

/// Post verdict as observation back to CCM intake.
///
/// Called after every verdict is stored (whether cache hit or fresh evaluation).
/// Non-blocking: all errors logged and swallowed (doesn't fail the verdict).
///
/// This is the FORWARD flow completing the compound loop:
///   Observation → Judge → Verdict → Observation (here) → CCM intake → Crystals
pub(crate) async fn post_verdict_observation(
    verdict: &Verdict,
    stimulus_domain: Option<&str>,
    deps: &PipelineDeps<'_>,
) {
    // Domain gate: skip verdicts without explicit domain (noise, not knowledge)
    let domain = stimulus_domain.unwrap_or("general");
    if domain == "general" {
        tracing::debug!(
            phase = "verdict_observation",
            verdict_id = %verdict.id,
            "domain='general' — verdict observation skipped (noise)"
        );
        return;
    }

    // Build observation from verdict
    let observation = Observation {
        project: "cynic".to_string(),
        agent_id: "kernel".to_string(), // K15 producer: kernel itself
        tool: "verdict".to_string(),
        target: "ccm_intake".to_string(), // Consumer: CCM's intake module
        domain: domain.to_string(),
        status: "ok".to_string(),
        // Context: verdict summary (verdict_id + kind + q_score)
        context: format!(
            "verdict_id={} kind={:?} q_score={:?}",
            verdict.id, verdict.kind, verdict.q_score
        ),
        session_id: "kernel-auto".to_string(), // Not from a Claude session
        timestamp: Utc::now().to_rfc3339(),
        tags: vec!["verdict".to_string(), "compound-loop".to_string()],
    };

    // Attempt to store observation
    if let Err(e) = deps.storage.store_observation(&observation).await {
        tracing::warn!(
            phase = "verdict_observation",
            verdict_id = %verdict.id,
            error = %e,
            "failed to post verdict observation (K15 forward loop broken)"
        );
        deps.metrics.inc_observation_post_failed();
        return;
    }

    tracing::info!(
        phase = "verdict_observation",
        verdict_id = %verdict.id,
        domain = %domain,
        "verdict observation posted (forward loop: verdict → observation → CCM)"
    );
    deps.metrics.inc_observation_post_success();
}
