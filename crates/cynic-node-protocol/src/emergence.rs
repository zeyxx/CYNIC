//! Emergence — coordination without a central coordinator.
//!
//! Each node runs an autonomy loop (see `local_autonomy`). When the kernel is
//! dead, nodes must still coordinate: who builds? who stores? who is the forge?
//! This layer answers that question without a central scheduler.
//!
//! The mechanism is simple: each node floods its local state to the mesh, and
//! each node independently runs the same deterministic election rule over the
//! shared state. Because the rule is pure and the state is shared, every node
//! converges to the same answer — no consensus protocol, no leader election,
//! just deterministic emergence from local observations.

use serde::{Deserialize, Serialize};

// Re-exported so downstream crates can reference `emergence::AutonomyMode`
// alongside the emergence types that embed it.
pub use crate::local_autonomy::AutonomyMode;

pub const EMERGENCE_STATE_SCHEMA: &str = "cynic.emergence_state.v1";
pub const EMERGENCE_DECISION_SCHEMA: &str = "cynic.emergence_decision.v1";

/// One node's view of the mesh (what it observes about others).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerState {
    pub node_id: String,
    pub role: String,
    pub mode: AutonomyMode,
    pub health_score: u8, // 0-100, higher = healthier
    pub last_seen_unix_s: u64,
    pub capabilities: Vec<String>,
}

/// The shared mesh state (gossiped between nodes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmergenceState {
    pub schema: String,
    pub observed_at_unix_s: u64,
    pub local_node_id: String,
    pub peers: Vec<PeerState>,
    /// What this node observes about itself.
    pub local: PeerState,
}

/// The emergence decision — what role should this node play?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmergenceDecision {
    pub schema: String,
    pub local_node_id: String,
    /// The role this node should assume (if any).
    pub assumed_role: Option<String>,
    /// The priority of this node for the assumed role.
    pub role_priority: Option<RolePriority>,
    /// All nodes that could fill this role, ordered by priority.
    pub candidates: Vec<String>,
    /// Why this decision was made.
    pub reasoning: String,
    /// What remains unknown.
    pub residual_unknowns: Vec<String>,
}

/// Priority for a federated role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RolePriority {
    Primary = 0,
    Secondary = 1,
    Tertiary = 2,
    Standby = 3,
}

impl RolePriority {
    pub fn from_role_str(role: &str) -> Option<Self> {
        match role {
            r if r.ends_with("_primary") => Some(Self::Primary),
            r if r.ends_with("_secondary") => Some(Self::Secondary),
            r if r.ends_with("_tertiary") => Some(Self::Tertiary),
            r if r.ends_with("_standby") => Some(Self::Standby),
            _ => None,
        }
    }
}

/// Pure rule: elect a node for a role from the shared mesh state.
///
/// Rules (deterministic, same on every node):
/// 1. Filter peers that are alive (AutonomyMode != Degraded) and have the role.
/// 2. Sort by role priority (primary > secondary > tertiary > standby).
/// 3. Then by health score (higher = better).
/// 4. Then by node_id (deterministic tiebreaker).
/// 5. The first candidate is the elected node.
pub fn elect_for_role(state: &EmergenceState, role: &str) -> EmergenceDecision {
    let mut candidates: Vec<&PeerState> = state
        .peers
        .iter()
        .filter(|p| p.mode != AutonomyMode::Degraded)
        .filter(|p| p.role.contains(role))
        .filter(|p| p.capabilities.iter().any(|c| c.contains(role)))
        .collect();

    // Sort by priority, then health, then node_id (deterministic)
    candidates.sort_by(|a, b| {
        let pa = RolePriority::from_role_str(&a.role).unwrap_or(RolePriority::Standby);
        let pb = RolePriority::from_role_str(&b.role).unwrap_or(RolePriority::Standby);
        pa.cmp(&pb)
            .then_with(|| b.health_score.cmp(&a.health_score))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });

    let elected = candidates.first().map(|p| p.node_id.clone());
    let elected_priority = candidates
        .first()
        .and_then(|p| RolePriority::from_role_str(&p.role));

    let reasoning = match &elected {
        Some(node) => format!("elected {node} for {role}"),
        None => format!("no viable candidate for {role}"),
    };

    let mut residual_unknowns = vec![];
    if candidates.is_empty() {
        residual_unknowns.push(format!("no_candidates_for_{role}"));
    }

    EmergenceDecision {
        schema: EMERGENCE_DECISION_SCHEMA.to_string(),
        local_node_id: state.local_node_id.clone(),
        assumed_role: elected.clone().map(|_| role.to_string()),
        role_priority: elected_priority,
        candidates: candidates.iter().map(|p| p.node_id.clone()).collect(),
        reasoning,
        residual_unknowns,
    }
}

/// Pure rule: should this node assume a role?
///
/// A node assumes a role only if it is the elected candidate AND it is alive.
pub fn should_assume_role(state: &EmergenceState, role: &str) -> bool {
    let decision = elect_for_role(state, role);
    match &decision.assumed_role {
        Some(r) if r == role => {
            decision.candidates.first().map(|s| s.as_str()) == Some(state.local_node_id.as_str())
        }
        _ => false,
    }
}

/// Compute health score from OS health signals (0-100).
pub fn health_score_from_os(
    dmesg_softlockup_count: u64,
    dmesg_oom_count: u64,
    available_mem_mib: u64,
) -> u8 {
    let mut score: i32 = 100;
    score -= (dmesg_softlockup_count as i32) * 10;
    score -= (dmesg_oom_count as i32) * 20;
    if available_mem_mib < 2048 {
        score -= 15;
    }
    if available_mem_mib < 1024 {
        score -= 25;
    }
    score.clamp(0, 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(node_id: &str, role: &str, mode: AutonomyMode, health: u8) -> PeerState {
        PeerState {
            node_id: node_id.to_string(),
            role: role.to_string(),
            mode,
            health_score: health,
            last_seen_unix_s: 1,
            capabilities: vec![role.to_string()],
        }
    }

    #[test]
    fn elect_primary_when_healthy() {
        let state = EmergenceState {
            schema: EMERGENCE_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            local_node_id: "node-a".to_string(),
            peers: vec![
                peer(
                    "node-a",
                    "build_runner_secondary",
                    AutonomyMode::Autonomous,
                    80,
                ),
                peer(
                    "node-b",
                    "build_runner_primary",
                    AutonomyMode::Autonomous,
                    90,
                ),
            ],
            local: peer(
                "node-a",
                "build_runner_secondary",
                AutonomyMode::Autonomous,
                80,
            ),
        };
        let decision = elect_for_role(&state, "build_runner");
        assert_eq!(decision.candidates.first().unwrap(), "node-b");
    }

    #[test]
    fn elect_secondary_when_primary_dead() {
        let state = EmergenceState {
            schema: EMERGENCE_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            local_node_id: "node-a".to_string(),
            peers: vec![
                peer(
                    "node-a",
                    "build_runner_secondary",
                    AutonomyMode::Autonomous,
                    80,
                ),
                peer("node-b", "build_runner_primary", AutonomyMode::Degraded, 90),
            ],
            local: peer(
                "node-a",
                "build_runner_secondary",
                AutonomyMode::Autonomous,
                80,
            ),
        };
        let decision = elect_for_role(&state, "build_runner");
        assert_eq!(decision.candidates.first().unwrap(), "node-a");
    }

    #[test]
    fn no_candidate_when_all_degraded() {
        let state = EmergenceState {
            schema: EMERGENCE_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            local_node_id: "node-a".to_string(),
            peers: vec![
                peer("node-a", "build_runner", AutonomyMode::Degraded, 80),
                peer("node-b", "build_runner", AutonomyMode::Degraded, 90),
            ],
            local: peer("node-a", "build_runner", AutonomyMode::Degraded, 80),
        };
        let decision = elect_for_role(&state, "build_runner");
        assert!(decision.candidates.is_empty());
        assert!(!decision.residual_unknowns.is_empty());
    }

    #[test]
    fn health_score_penalizes_oom() {
        let score = health_score_from_os(0, 1, 8000);
        assert!(score <= 80);
    }

    #[test]
    fn health_score_penalizes_low_memory() {
        let score = health_score_from_os(0, 0, 512);
        assert!(score <= 75);
    }

    #[test]
    fn schema_is_stable() {
        assert_eq!(EMERGENCE_STATE_SCHEMA, "cynic.emergence_state.v1");
        assert_eq!(EMERGENCE_DECISION_SCHEMA, "cynic.emergence_decision.v1");
    }
}
