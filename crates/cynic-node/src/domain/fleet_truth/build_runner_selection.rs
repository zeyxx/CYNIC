//! Pure reconcile layer: select the build_runner node for an admitted build,
//! with explicit graceful degradation when the announced (preferred) node is
//! unavailable.
//!
//! This is a pure decision. It takes declared nodes + effective status
//! (already observed by `fleet_truth`) and returns which runner should own the
//! next build, plus a `degraded` flag and a machine-readable reason. It never
//! performs IO, never mutates, and emits no proof on its own — the caller is
//! responsible for turning `SelectedBuildRunner` into a read-back report.

use serde::{Deserialize, Serialize};

use super::{EffectiveNode, Node};

/// Which runner a build should target, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedBuildRunner {
    /// Node chosen to run the build. `None` only when `allow_local_fallback`
    /// was set and no healthy runner exists (degraded local mode).
    pub node: Option<String>,
    /// True when the preferred (announced) runner was unavailable and we fell
    /// back to another healthy runner or to local degraded mode.
    pub degraded: bool,
    /// Machine-readable reason: `preferred_healthy`, `preferred_absent`,
    /// `preferred_down`, `fallback_healthy`, `degraded_local`.
    pub reason: String,
    /// The preferred runner name that was requested (for read-back).
    pub preferred: String,
}

/// Build-runner candidates derived from declared nodes + observed status.
fn build_runner_candidates(nodes: &[Node], effective: &[EffectiveNode]) -> Vec<String> {
    let mut healthy = Vec::new();
    for node in nodes.iter().filter(|n| n.role == "build_runner") {
        let status = effective
            .iter()
            .find(|e| e.name == node.name)
            .map(|e| e.effective_status.as_str())
            .unwrap_or("unobserved");
        if status == "active" {
            healthy.push(node.name.clone());
        }
    }
    healthy
}

fn effective_status_of(effective: &[EffectiveNode], name: &str) -> String {
    effective
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.effective_status.clone())
        .unwrap_or_else(|| "absent".to_string())
}

/// Reconcile a build-runner selection.
///
/// `preferred` is the announced node (declared in registry, e.g. via
/// `preferred_build_runner`). When it is healthy, it wins. When it is missing
/// or down, we degrade: a different healthy `build_runner` if one exists, else
/// local degraded mode when `allow_local_fallback` is set.
pub fn select_build_runner(
    preferred: &str,
    nodes: &[Node],
    effective: &[EffectiveNode],
    allow_local_fallback: bool,
) -> SelectedBuildRunner {
    let candidates = build_runner_candidates(nodes, effective);
    let preferred_status = effective_status_of(effective, preferred);

    if preferred_status == "active" {
        return SelectedBuildRunner {
            node: Some(preferred.to_string()),
            degraded: false,
            reason: "preferred_healthy".to_string(),
            preferred: preferred.to_string(),
        };
    }

    if !candidates.is_empty() {
        // Prefer a healthy candidate other than the (down) preferred node.
        let fallback = candidates
            .iter()
            .find(|c| *c != preferred)
            .cloned()
            .or_else(|| candidates.first().cloned());
        return SelectedBuildRunner {
            node: fallback,
            degraded: true,
            reason: if preferred_status == "absent" {
                "preferred_absent".to_string()
            } else {
                "preferred_down".to_string()
            },
            preferred: preferred.to_string(),
        };
    }

    if allow_local_fallback {
        return SelectedBuildRunner {
            node: None,
            degraded: true,
            reason: "degraded_local".to_string(),
            preferred: preferred.to_string(),
        };
    }

    // No healthy runner and no fallback allowed: report the blocked state
    // through the selection itself rather than failing.
    SelectedBuildRunner {
        node: None,
        degraded: true,
        reason: "blocked_no_runner".to_string(),
        preferred: preferred.to_string(),
    }
}

/// Derive the announced (preferred) build_runner from the declared nodes.
///
/// Explicit `preferred` wins. Otherwise the preferred runner is the first
/// declared `build_runner` with `status == "active"`; if none is active, the
/// first declared `build_runner` (so the reconcile can still explain why it is
/// unavailable rather than failing blind).
pub fn preferred_build_runner_from_declared(preferred: Option<&str>, declared: &[Node]) -> String {
    if let Some(value) = preferred.filter(|v| !v.is_empty()) {
        return value.to_string();
    }
    declared
        .iter()
        .filter(|n| n.role == "build_runner")
        .find(|n| n.status == "active")
        .or_else(|| declared.iter().find(|n| n.role == "build_runner"))
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "none".to_string())
}

/// Render the canonical machine-readable selection report
/// (`cynic.build_runner_selection.v1`).
pub fn build_selection_report(
    selection: &SelectedBuildRunner,
    declared: &[Node],
    effective: &[EffectiveNode],
) -> serde_json::Value {
    let healthy_runners: Vec<String> = declared
        .iter()
        .filter(|n| n.role == "build_runner")
        .filter(|n| {
            effective
                .iter()
                .find(|e| e.name == n.name)
                .map(|e| e.effective_status == "active")
                .unwrap_or(false)
        })
        .map(|n| n.name.clone())
        .collect();

    serde_json::json!({
        "schema": "cynic.build_runner_selection.v1",
        "status": if selection.node.is_some() { "selected" } else { "blocked" },
        "selected": selection.node,
        "degraded": selection.degraded,
        "reason": selection.reason,
        "preferred": selection.preferred,
        "healthy_build_runners": healthy_runners,
        "allow_local_fallback": selection.node.is_none() && selection.reason == "degraded_local",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fleet_truth::model::{ConstellationSummary, EnergySummary, FleetSummary};
    use crate::domain::fleet_truth::{FleetReport, Node, ObservedResource};

    fn node(name: &str, role: &str, status: &str) -> Node {
        Node {
            name: name.to_string(),
            role: role.to_string(),
            status: status.to_string(),
            constellation: None,
            aliases: vec![],
            host: None,
            proxmox_id: None,
            energy_profile: Default::default(),
        }
    }

    fn effective(name: &str, role: &str, status: &str) -> EffectiveNode {
        EffectiveNode {
            name: name.to_string(),
            role: role.to_string(),
            constellation: None,
            declared_status: status.to_string(),
            observed_status: status.to_string(),
            effective_status: status.to_string(),
            provider: None,
            observed_constellation: None,
        }
    }

    #[test]
    fn preferred_healthy_wins_no_degradation() {
        let nodes = vec![node("lab-builder", "build_runner", "active")];
        let eff = vec![effective("lab-builder", "build_runner", "active")];
        let sel = select_build_runner("lab-builder", &nodes, &eff, true);
        assert_eq!(sel.node.as_deref(), Some("lab-builder"));
        assert!(!sel.degraded);
        assert_eq!(sel.reason, "preferred_healthy");
    }

    #[test]
    fn preferred_down_falls_back_to_other_healthy_runner() {
        let nodes = vec![
            node("lab-builder", "build_runner", "active"),
            node("backup-builder", "build_runner", "active"),
        ];
        // preferred is stale (down), backup is active.
        let eff = vec![
            effective("lab-builder", "build_runner", "stale"),
            effective("backup-builder", "build_runner", "active"),
        ];
        let sel = select_build_runner("lab-builder", &nodes, &eff, true);
        assert_eq!(sel.node.as_deref(), Some("backup-builder"));
        assert!(sel.degraded);
        assert_eq!(sel.reason, "preferred_down");
    }

    #[test]
    fn preferred_absent_falls_back_when_other_healthy_exists() {
        let nodes = vec![node("backup-builder", "build_runner", "active")];
        let eff = vec![effective("backup-builder", "build_runner", "active")];
        let sel = select_build_runner("lab-builder", &nodes, &eff, true);
        assert_eq!(sel.node.as_deref(), Some("backup-builder"));
        assert!(sel.degraded);
        assert_eq!(sel.reason, "preferred_absent");
    }

    #[test]
    fn no_healthy_runner_degrades_local_when_allowed() {
        let nodes = vec![node("lab-builder", "build_runner", "active")];
        let eff = vec![effective("lab-builder", "build_runner", "stale")];
        let sel = select_build_runner("lab-builder", &nodes, &eff, true);
        assert_eq!(sel.node, None);
        assert!(sel.degraded);
        assert_eq!(sel.reason, "degraded_local");
    }

    #[test]
    fn no_healthy_runner_blocked_when_fallback_disabled() {
        let nodes = vec![node("lab-builder", "build_runner", "active")];
        let eff = vec![effective("lab-builder", "build_runner", "stale")];
        let sel = select_build_runner("lab-builder", &nodes, &eff, false);
        assert_eq!(sel.node, None);
        assert!(sel.degraded);
        assert_eq!(sel.reason, "blocked_no_runner");
    }

    #[test]
    fn selection_plugs_into_fleet_report_shape() {
        // Ensure the reconcile consumes the real FleetReport types without
        // inventing a parallel node model.
        let report = FleetReport {
            summary: FleetSummary {
                status: "ok".to_string(),
                declared_nodes: 1,
                observed_resources: 1,
                constellations: ConstellationSummary {
                    declared: Default::default(),
                    observed: Default::default(),
                },
                anomaly_count: 0,
                critical_anomaly_count: 0,
                critical_targets: vec![],
            },
            desired: vec![node("lab-builder", "build_runner", "active")],
            declared: vec![node("lab-builder", "build_runner", "active")],
            observed: vec![ObservedResource {
                provider: "proxmox".to_string(),
                name: "lab-builder".to_string(),
                status: "running".to_string(),
                resource_id: None,
                kind: None,
                constellation: None,
            }],
            effective: vec![effective("lab-builder", "build_runner", "active")],
            anomalies: vec![],
            energy: EnergySummary {
                classes: Default::default(),
                missing_profiles: vec![],
            },
        };
        let sel = select_build_runner("lab-builder", &report.declared, &report.effective, true);
        assert_eq!(sel.reason, "preferred_healthy");
    }

    #[test]
    fn preferred_derivation_picks_first_active_then_first_declared() {
        // explicit wins
        let nodes = vec![node("lab-builder", "build_runner", "active")];
        assert_eq!(
            preferred_build_runner_from_declared(Some("custom"), &nodes),
            "custom"
        );
        // first active build_runner when no explicit
        let nodes = vec![
            node("old-builder", "build_runner", "stale"),
            node("lab-builder", "build_runner", "active"),
        ];
        assert_eq!(
            preferred_build_runner_from_declared(None, &nodes),
            "lab-builder"
        );
        // fall back to first declared build_runner if none active
        let nodes = vec![node("old-builder", "build_runner", "stale")];
        assert_eq!(
            preferred_build_runner_from_declared(None, &nodes),
            "old-builder"
        );
        // none declared -> "none"
        assert_eq!(preferred_build_runner_from_declared(None, &[]), "none");
    }

    #[test]
    fn selection_report_schema_is_machine_readable() {
        let nodes = vec![
            node("lab-builder", "build_runner", "active"),
            node("backup-builder", "build_runner", "active"),
        ];
        let eff = vec![
            effective("lab-builder", "build_runner", "stale"),
            effective("backup-builder", "build_runner", "active"),
        ];
        let sel = select_build_runner("lab-builder", &nodes, &eff, true);
        let report = build_selection_report(&sel, &nodes, &eff);
        assert_eq!(report["schema"], "cynic.build_runner_selection.v1");
        assert_eq!(report["status"], "selected");
        assert_eq!(report["selected"], "backup-builder");
        assert_eq!(report["degraded"], true);
        assert_eq!(report["reason"], "preferred_down");
        assert_eq!(report["preferred"], "lab-builder");
        assert_eq!(report["healthy_build_runners"][0], "backup-builder");
        assert_eq!(report["allow_local_fallback"], false);
    }
}
