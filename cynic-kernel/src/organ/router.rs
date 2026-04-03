// organ/router.rs — Profile-based backend selection.
//
// Maps InferenceProfile → requirements, then selects a healthy backend
// from a cluster that meets those requirements.
// Phase 1: simple first-eligible selection. Phase 2: cursor-based round-robin.

use crate::domain::chat::InferenceProfile;
use crate::organ::registry::*;

/// What a profile requires from a backend.
#[derive(Debug, Clone)]
pub struct ProfileRequirements {
    pub requires_json: bool,
    pub requires_thinking: bool,
    pub requires_agent_reasoning: bool,
}

/// Derive requirements from a profile.
pub fn profile_requirements(profile: &InferenceProfile) -> ProfileRequirements {
    match profile {
        InferenceProfile::Scoring => ProfileRequirements {
            requires_json: true,
            requires_thinking: false,
            requires_agent_reasoning: false,
        },
        InferenceProfile::Agent => ProfileRequirements {
            requires_json: false,
            requires_thinking: true,
            requires_agent_reasoning: true,
        },
        InferenceProfile::Summary => ProfileRequirements {
            requires_json: true,
            requires_thinking: false,
            requires_agent_reasoning: false,
        },
        InferenceProfile::Infer => ProfileRequirements {
            requires_json: true,
            requires_thinking: false,
            requires_agent_reasoning: false,
        },
    }
}

/// Select the first healthy backend from a cluster that meets capability thresholds.
/// Returns None when all backends are dead or below threshold.
///
/// Phase 1: first-eligible (Failover). Phase 2 adds cursor for RoundRobin.
pub fn select_backend<'a>(
    cluster: &Cluster,
    backends: &'a [Backend],
    threshold: &CapabilityThreshold,
) -> Option<&'a Backend> {
    cluster
        .backends
        .iter()
        .filter_map(|bid| backends.iter().find(|b| &b.id == bid))
        .find(|b| {
            matches!(
                b.health,
                BackendHealth::Healthy | BackendHealth::Degraded { .. }
            ) && (!b.declared.json || b.measured.json_valid_rate >= threshold.min_json_valid_rate)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn make_backend(id: &str, health: BackendHealth) -> Backend {
        Backend {
            id: BackendId(id.into()),
            node_id: NodeId("node".into()),
            endpoint: "http://localhost:8080/v1".into(),
            model: "test-model".into(),
            declared: DeclaredCapabilities {
                json: true,
                scoring: true,
                ..Default::default()
            },
            measured: MeasuredCapabilities {
                json_valid_rate: 0.9,
                ..Default::default()
            },
            health,
            timeout_secs: 30,
            remediation: None,
        }
    }

    fn make_cluster(id: &str, backend_ids: Vec<&str>) -> Cluster {
        Cluster {
            id: ClusterId(id.into()),
            required_json_rate: 0.7,
            backends: backend_ids
                .into_iter()
                .map(|s| BackendId(s.into()))
                .collect(),
            strategy: ClusterStrategy::RoundRobin,
        }
    }

    #[test]
    fn scoring_profile_requires_json() {
        let req = profile_requirements(&InferenceProfile::Scoring);
        assert!(req.requires_json);
        assert!(!req.requires_thinking);
    }

    #[test]
    fn agent_profile_requires_thinking() {
        let req = profile_requirements(&InferenceProfile::Agent);
        assert!(!req.requires_json);
        assert!(req.requires_thinking);
        assert!(req.requires_agent_reasoning);
    }

    #[test]
    fn selects_healthy_backend_from_cluster() {
        let backends = vec![
            make_backend(
                "b1",
                BackendHealth::Dead {
                    reason: "down".into(),
                    since: Instant::now(),
                },
            ),
            make_backend("b2", BackendHealth::Healthy),
        ];
        let cluster = make_cluster("c1", vec!["b1", "b2"]);
        let selected = select_backend(&cluster, &backends, &CapabilityThreshold::default());
        assert_eq!(selected.unwrap().id.0, "b2");
    }

    #[test]
    fn returns_none_when_all_dead() {
        let backends = vec![make_backend(
            "b1",
            BackendHealth::Dead {
                reason: "down".into(),
                since: Instant::now(),
            },
        )];
        let cluster = make_cluster("c1", vec!["b1"]);
        let selected = select_backend(&cluster, &backends, &CapabilityThreshold::default());
        assert!(selected.is_none());
    }

    #[test]
    fn degraded_backend_is_eligible() {
        let backends = vec![make_backend(
            "b1",
            BackendHealth::Degraded {
                reason: "slow".into(),
                since: Instant::now(),
            },
        )];
        let cluster = make_cluster("c1", vec!["b1"]);
        let selected = select_backend(&cluster, &backends, &CapabilityThreshold::default());
        assert!(selected.is_some());
    }

    #[test]
    fn below_json_threshold_excluded() {
        let mut backend = make_backend("b1", BackendHealth::Healthy);
        backend.measured.json_valid_rate = 0.3; // below 0.7 threshold
        let backends = vec![backend];
        let cluster = make_cluster("c1", vec!["b1"]);
        let selected = select_backend(&cluster, &backends, &CapabilityThreshold::default());
        assert!(selected.is_none());
    }

    #[test]
    fn non_json_backend_skips_rate_check() {
        let mut backend = make_backend("b1", BackendHealth::Healthy);
        backend.declared.json = false;
        backend.measured.json_valid_rate = 0.0; // low rate but json not declared
        let backends = vec![backend];
        let cluster = make_cluster("c1", vec!["b1"]);
        let selected = select_backend(&cluster, &backends, &CapabilityThreshold::default());
        assert!(selected.is_some());
    }
}
