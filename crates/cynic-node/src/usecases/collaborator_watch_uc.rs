//! Collaborator watcher: pure diff between observed and declared access.
//!
//! WHY: the declared registries (`collaborator_ids` per project, fiches in
//! `infra/collaborators.json`) drift silently from the forges. The watcher
//! names the drift before any gesture is proposed. Detection is automatic,
//! admission stays signed.
//!
//! HOW: pure logic in this module (no I/O). The caller provides the observed
//! handles (from `ExternalSourcePort::list_collaborators`), the declared
//! project collaborator ids, and the known fiches; this module returns the
//! drift classes with the proposed gesture for each.
//!
//! NEGATIVE BOUNDARY: does not read or write files, does not perform network
//! I/O, does not spawn processes, never admits anyone. Proposals only.

use serde::{Deserialize, Serialize};

/// Schema identifier for collaborator watch reports.
pub const COLLABORATOR_WATCH_SCHEMA: &str = "cynic.node.collaborator_watch.v1";

/// One drift item: observed access that the registries do not describe,
/// or declared access the forge no longer shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchDrift {
    /// Observed handle (forge login) or declared id, depending on class.
    pub subject: String,
    /// unknown_handle | stale_fiche | unlisted_handle.
    pub class: String,
    /// Proposed signed gesture: onboard | review | correct.
    pub proposed_gesture: String,
    /// Human-readable reason.
    pub reason: String,
}

/// Pure watch report for one project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchReport {
    pub schema: String,
    pub project_id: String,
    /// True when observed and declared agree.
    pub clean: bool,
    pub drift: Vec<WatchDrift>,
}

/// Diff observed forge handles against declared project ids and known fiches.
///
/// - `observed`: handles with access on the forge.
/// - `declared_ids`: `collaborator_ids` from `infra/external-projects.json`.
/// - `fiche_ids`: `collaborator_id` values from `infra/collaborators.json`.
/// - `fiche_handles`: `github_handle` values from `infra/collaborators.json`.
pub fn diff_watch(
    project_id: &str,
    observed: &[String],
    declared_ids: &[String],
    fiche_ids: &[String],
    fiche_handles: &[String],
) -> WatchReport {
    let mut drift = Vec::new();

    for handle in observed {
        if !fiche_handles.iter().any(|h| h == handle) {
            drift.push(WatchDrift {
                subject: handle.clone(),
                class: "unknown_handle".to_string(),
                proposed_gesture: "onboard".to_string(),
                reason: format!(
                    "handle '{handle}' has access to '{project_id}' with no fiche; propose onboard at observed"
                ),
            });
        } else if !declared_ids.iter().any(|id| {
            fiche_ids
                .iter()
                .position(|f| f == id)
                .map(|pos| fiche_handles.get(pos) == Some(&handle))
                .unwrap_or(false)
        }) {
            drift.push(WatchDrift {
                subject: handle.clone(),
                class: "unlisted_handle".to_string(),
                proposed_gesture: "correct".to_string(),
                reason: format!(
                    "handle '{handle}' is fiched but '{project_id}' does not declare it; propose registry correction"
                ),
            });
        }
    }

    for id in declared_ids {
        let handle = fiche_ids
            .iter()
            .position(|f| f == id)
            .and_then(|pos| fiche_handles.get(pos));
        match handle {
            Some(h) if observed.iter().any(|o| o == h) => {}
            _ => drift.push(WatchDrift {
                subject: id.clone(),
                class: "stale_fiche".to_string(),
                proposed_gesture: "review".to_string(),
                reason: format!(
                    "declared collaborator '{id}' has no observed access to '{project_id}'; propose review"
                ),
            }),
        }
    }

    let clean = drift.is_empty();
    WatchReport {
        schema: COLLABORATOR_WATCH_SCHEMA.to_string(),
        project_id: project_id.to_string(),
        clean,
        drift,
    }
}

#[cfg(test)]
mod watch_tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn agreement_is_clean() {
        let r = diff_watch("p", &s(&["amy"]), &s(&["amy-id"]), &s(&["amy-id"]), &s(&["amy"]));
        assert!(r.clean && r.drift.is_empty());
    }

    #[test]
    fn unknown_handle_proposes_onboard() {
        let r = diff_watch("p", &s(&["amy", "bob"]), &s(&["amy-id"]), &s(&["amy-id"]), &s(&["amy"]));
        assert!(!r.clean);
        let d = r.drift.iter().find(|d| d.subject == "bob").expect("bob drift");
        assert_eq!(d.class, "unknown_handle");
        assert_eq!(d.proposed_gesture, "onboard");
    }

    #[test]
    fn stale_declared_proposes_review() {
        let r = diff_watch(
            "p",
            &s(&["amy"]),
            &s(&["amy-id", "zed-id"]),
            &s(&["amy-id", "zed-id"]),
            &s(&["amy", "zed"]),
        );
        let d = r.drift.iter().find(|d| d.subject == "zed-id").expect("zed drift");
        assert_eq!(d.class, "stale_fiche");
        assert_eq!(d.proposed_gesture, "review");
    }

    #[test]
    fn fiched_but_unlisted_proposes_correct() {
        let r = diff_watch("p", &s(&["amy"]), &s(&[]), &s(&["amy-id"]), &s(&["amy"]));
        let d = r.drift.iter().find(|d| d.subject == "amy").expect("amy drift");
        assert_eq!(d.class, "unlisted_handle");
        assert_eq!(d.proposed_gesture, "correct");
    }
}
