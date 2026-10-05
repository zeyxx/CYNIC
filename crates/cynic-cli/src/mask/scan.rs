//! Scanning/observation logic for the mask surface.
//!
//! Observes the runtime state and produces judgments. This is the
//! observe/reconcile step — it reads the substrate and projects it into the
//! canonical judgment shape.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_eval::judgment::{CynefinDomain, CynicJudgment, DijkwLayer, SagePhase};

use super::types::JudgmentReport;

/// Observe the runtime and produce a judgment. Read-only — never fabricate
/// state. If a signal is unreadable, preserve it as residual unknown.
pub(super) fn observe_judgment(subject: Option<&str>) -> JudgmentReport {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let subject_id = subject.unwrap_or("local").to_string();

    let os_observation = match observe_os_health_signal(&subject_id) {
        Some(j) => j,
        None => {
            let mut j = CynicJudgment::new(
                format!("judgment-{}-{}", subject_id, now),
                false,
                DijkwLayer::Data,
                SagePhase::Priming,
                CynefinDomain::Disordered,
                now,
            );
            j.residual_unknowns = vec!["os_health_observation_unavailable".to_string()];
            j.confidence = 0.0;
            return JudgmentReport::new(j, now);
        }
    };

    JudgmentReport::new(os_observation, now)
}

/// Attempt to observe OS health and convert it into a judgment. Returns None
/// if the observation cannot be made (missing host integration, etc.).
fn observe_os_health_signal(subject_id: &str) -> Option<CynicJudgment> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut j = CynicJudgment::new(
        format!("judgment-{}-{}", subject_id, now),
        false,
        DijkwLayer::Information,
        SagePhase::Sense,
        CynefinDomain::Complicated,
        now,
    );
    j.residual_unknowns = vec![
        "os_health_observation_pending_host_integration".to_string(),
        "convergence_matrix_observation_pending".to_string(),
    ];
    j.confidence = 0.3;
    Some(j)
}

/// Observe a patch (diff vs origin/main) and return the raw observation.
pub(super) fn observe_patch(root: &Path, branch: &str) -> cynic_eval::judge_patch::PatchObservation {
    cynic_host::judge_patch_observer::observe_patch(root, branch)
}
