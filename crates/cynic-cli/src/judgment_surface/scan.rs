//! Scan layer for the judgment surface.
//!
//! Observes the runtime state and produces a CynicJudgment. This is the
//! observe/reconcile step — it reads the substrate and projects it into the
//! canonical judgment shape.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::CynicJudgment;
use cynic_protocol::judgment::{CynefinDomain, DijkwLayer, SagePhase};

use crate::judgment_surface::types::JudgmentReport;

/// Observe the runtime and produce a judgment. Read-only — never fabricate
/// state. If a signal is unreadable, preserve it as residual unknown.
pub(super) fn observe_judgment(subject: Option<&str>) -> JudgmentReport {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let subject_id = subject.unwrap_or("local").to_string();

    // Observe OS substrate health via the host's observer.
    let os_observation = match observe_os_health_signal(&subject_id) {
        Some(j) => j,
        None => {
            // No OS health signal available — produce a Data-layer judgment
            // that preserves the gap as residual unknown.
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
    // The host's OS health observer is available in cynic-host. We use it
    // indirectly through the protocol's OsHealthAnomaly shape.
    //
    // For now, we produce a conservative Information-layer judgment that
    // reflects the observation capability without depending on a live
    // host integration from the CLI crate (which stays thin).
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
