//! Dynamic build-node resolution for a federation without single points of
//! failure.
//!
//! The build role is no longer a single hardcoded `build_runner`. It is a
//! *desired* assignment (declared in the registry seed) backed by observed
//! runtime reality: mesh reachability + OS-substrate health. When reality
//! diverges from the desired topology, that divergence is a first-class data
//! event (see `TopologyDriftSignal` in `health_signal`), never a panic.
//!
//! Selection rule (B3): mesh-live primes the registry seed; among reachable,
//! non-OS-critical candidates, prefer the lowest role priority (primary before
//! secondary), then the fewest active OS-health anomalies. Fail-closed: if no
//! candidate is reachable and healthy, return `None` and let the caller emit a
//! `TopologyDriftSignal` rather than silently degrading to a broken node.

use crate::health_signal::OsHealthAnomaly;

/// Priority of a build role. Lower number = higher priority. Two builders
/// minimum (primary + secondary) tolerate one node loss with distinct roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BuildRolePriority {
    Primary = 0,
    Secondary = 1,
    Tertiary = 2,
}

impl BuildRolePriority {
    /// Map a declared role string to its priority. Non-builder roles yield
    /// `None` so they are never selected for build dispatch.
    pub fn from_role_str(role: &str) -> Option<Self> {
        match role {
            "build_runner_primary" => Some(Self::Primary),
            "build_runner_secondary" => Some(Self::Secondary),
            "build_runner_tertiary" => Some(Self::Tertiary),
            _ => None,
        }
    }
}

/// Compact, transport-neutral view of a node's OS-substrate health, folded
/// from its active `OsHealthAnomaly` list so the resolver stays dependency-light.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodeHealthView {
    pub critical_active: bool,
    pub warning_active: bool,
    pub active_osha_codes: Vec<String>,
    /// Available memory in MiB (observed locally; 0 for remote peers without agent).
    pub available_mem_mib: u64,
}

impl NodeHealthView {
    pub fn from_anomalies(anomalies: &[OsHealthAnomaly], available_mem_mib: u64) -> Self {
        let mut view = NodeHealthView::default();
        view.available_mem_mib = available_mem_mib;
        for a in anomalies {
            let is_critical = a.severity.eq_ignore_ascii_case("critical");
            let is_warning = a.severity.eq_ignore_ascii_case("warning");
            if is_critical {
                view.critical_active = true;
            } else if is_warning {
                view.warning_active = true;
            }
            if let Some(code) = &a.error_code {
                if !view.active_osha_codes.contains(code) {
                    view.active_osha_codes.push(code.clone());
                }
            }
        }
        view
    }
}

/// A build-node candidate for dynamic selection. `reachable` comes from mesh
/// observation; `health` from the OS-substrate observer (OSH-001..008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildNodeCandidate {
    pub node_id: String,
    pub desired_role: String,
    pub reachable: bool,
    pub health: NodeHealthView,
}

/// Outcome of a successful selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedBuildNode {
    pub node_id: String,
    pub desired_role: String,
    pub role_priority: BuildRolePriority,
    pub active_osha_codes: Vec<String>,
}

/// Pure, fail-closed build-node resolver.
///
/// Returns the highest-priority reachable, non-OS-critical builder, breaking
/// ties by fewest active anomalies. Returns `None` when no candidate is both
/// reachable and free of critical OS-health anomalies — the caller MUST then
/// emit a `TopologyDriftSignal`; this function never falls back to a degraded
/// node.
pub fn resolve_build_node(candidates: &[BuildNodeCandidate]) -> Option<SelectedBuildNode> {
    candidates
        .iter()
        .filter(|c| c.reachable && !c.health.critical_active)
        .filter_map(|c| {
            let priority = BuildRolePriority::from_role_str(&c.desired_role)?;
            Some((c, priority))
        })
        .min_by(|(a, pa), (b, pb)| {
            pa.cmp(pb).then_with(|| {
                a.health
                    .active_osha_codes
                    .len()
                    .cmp(&b.health.active_osha_codes.len())
            })
        })
        .map(|(c, priority)| SelectedBuildNode {
            node_id: c.node_id.clone(),
            desired_role: c.desired_role.clone(),
            role_priority: priority,
            active_osha_codes: c.health.active_osha_codes.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(node_id: &str, role: &str, reachable: bool, critical: bool) -> BuildNodeCandidate {
        let health = if critical {
            NodeHealthView {
                critical_active: true,
                warning_active: false,
                active_osha_codes: vec!["CYNIC-OSH-001".to_string()],
                available_mem_mib: 0,
            }
        } else {
            NodeHealthView {
                critical_active: false,
                warning_active: false,
                active_osha_codes: vec![],
                available_mem_mib: 0,
            }
        };
        BuildNodeCandidate {
            node_id: node_id.to_string(),
            desired_role: role.to_string(),
            reachable,
            health,
        }
    }

    #[test]
    fn selects_primary_when_healthy() {
        let candidates = vec![
            candidate("builder-b", "build_runner_secondary", true, false),
            candidate("builder-a", "build_runner_primary", true, false),
        ];
        let sel = resolve_build_node(&candidates).expect("expected selection");
        assert_eq!(sel.node_id, "builder-a");
        assert_eq!(sel.role_priority, BuildRolePriority::Primary);
    }

    #[test]
    fn fails_over_to_secondary_when_primary_critical() {
        let candidates = vec![
            candidate("builder-a", "build_runner_primary", true, true),
            candidate("builder-b", "build_runner_secondary", true, false),
        ];
        let sel = resolve_build_node(&candidates).expect("expected failover");
        assert_eq!(sel.node_id, "builder-b");
        assert_eq!(sel.role_priority, BuildRolePriority::Secondary);
    }

    #[test]
    fn none_when_no_reachable_healthy_builder() {
        let candidates = vec![
            candidate("builder-a", "build_runner_primary", false, false),
            candidate("builder-b", "build_runner_secondary", true, true),
        ];
        assert!(resolve_build_node(&candidates).is_none());
    }

    #[test]
    fn warning_is_selectable_but_not_critical() {
        let mut c = candidate("builder-a", "build_runner_primary", true, false);
        c.health = NodeHealthView {
            critical_active: false,
            warning_active: true,
            active_osha_codes: vec!["CYNIC-OSH-003".to_string()],
            available_mem_mib: 0,
        };
        let sel = resolve_build_node(&[c]).expect("warning is selectable");
        assert_eq!(sel.node_id, "builder-a");
        assert!(sel.active_osha_codes.contains(&"CYNIC-OSH-003".to_string()));
    }

    #[test]
    fn non_builder_roles_ignored() {
        let candidates = vec![candidate("random-node", "storage_primary", true, false)];
        assert!(resolve_build_node(&candidates).is_none());
    }
}
