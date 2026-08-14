use crate::domain::routing::RoutingDecision;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub struct FleetNodeStat {
    pub node: String,
    pub avg_latency_ms: u64,
    pub success_rate: f64,
    pub age_secs: u64,
    pub failure_reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RankedInferenceCandidate {
    pub node: String,
    pub avg_latency_ms: u64,
    pub success_rate: f64,
    pub quality: String,
    pub failure_reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferenceRouteDecisionView {
    pub selected_node: String,
    pub reason: String,
    pub avg_latency_ms: u64,
    pub success_rate: f64,
    pub quality: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferenceCandidateView {
    pub node: String,
    pub avg_latency_ms: u64,
    pub success_rate: f64,
    pub quality: String,
    pub failure_reason: String,
}

pub fn node_quality(failure_reason: &str, age_secs: u64, success_rate: f64) -> &'static str {
    match failure_reason {
        "none" => match (age_secs, success_rate) {
            (0..=5, sr) if sr >= 0.95 => "excellent",
            (0..=60, sr) if sr >= 0.9 => "good",
            (_, sr) if sr >= 0.7 => "degraded",
            _ => "dead",
        },
        "port_conflict" | "config_error" | "firewall" | "unknown" => "degraded",
        _ => "dead",
    }
}

pub fn rank_healthy_candidates(nodes: Vec<FleetNodeStat>) -> Vec<RankedInferenceCandidate> {
    let mut candidates: Vec<_> = nodes
        .into_iter()
        .filter(|node| {
            if node.failure_reason == "process_crash" || node.failure_reason == "not_started" {
                return false;
            }
            node.success_rate >= 0.7
        })
        .map(|node| RankedInferenceCandidate {
            quality: node_quality(&node.failure_reason, node.age_secs, node.success_rate)
                .to_string(),
            node: node.node,
            avg_latency_ms: node.avg_latency_ms,
            success_rate: node.success_rate,
            failure_reason: node.failure_reason,
        })
        .collect();

    candidates.sort_by(
        |left, right| match right.success_rate.partial_cmp(&left.success_rate) {
            Some(std::cmp::Ordering::Equal) => left.avg_latency_ms.cmp(&right.avg_latency_ms),
            Some(ordering) => ordering,
            None => std::cmp::Ordering::Equal,
        },
    );

    candidates
}

pub fn select_best_candidate(
    nodes: Vec<FleetNodeStat>,
) -> Option<(RoutingDecision, RankedInferenceCandidate)> {
    let ranked = rank_healthy_candidates(nodes);
    let best = ranked.first()?.clone();
    Some((
        RoutingDecision {
            backend_name: best.node.clone(),
            reason: format!(
                "ranked #1 of {} healthy nodes: success_rate={:.1}%, avg_latency={}ms",
                ranked.len(),
                best.success_rate * 100.0,
                best.avg_latency_ms
            ),
        },
        best,
    ))
}

pub fn project_route_response(nodes: Vec<FleetNodeStat>) -> Option<InferenceRouteDecisionView> {
    let (decision, best) = select_best_candidate(nodes)?;
    Some(InferenceRouteDecisionView {
        selected_node: decision.backend_name,
        reason: decision.reason,
        avg_latency_ms: best.avg_latency_ms,
        success_rate: best.success_rate,
        quality: best.quality,
    })
}

pub fn project_candidate_views(
    nodes: Vec<FleetNodeStat>,
    limit: usize,
) -> Vec<InferenceCandidateView> {
    rank_healthy_candidates(nodes)
        .into_iter()
        .take(limit)
        .map(|candidate| InferenceCandidateView {
            node: candidate.node,
            avg_latency_ms: candidate.avg_latency_ms,
            success_rate: candidate.success_rate,
            quality: candidate.quality,
            failure_reason: candidate.failure_reason,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_route_response_from_best_candidate() {
        let projected = project_route_response(vec![
            stat("slow", 200, 0.95),
            stat("fast", 50, 0.95),
            stat("bad", 10, 0.2),
        ])
        .unwrap();

        assert_eq!(projected.selected_node, "fast");
        assert_eq!(projected.avg_latency_ms, 50);
        assert_eq!(projected.quality, "excellent");
    }

    #[test]
    fn projects_limited_candidate_views() {
        let candidates = project_candidate_views(
            vec![stat("a", 20, 0.9), stat("b", 10, 0.9), stat("c", 5, 0.8)],
            2,
        );

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].node, "b");
        assert_eq!(candidates[1].node, "a");
    }

    fn stat(node: &str, avg_latency_ms: u64, success_rate: f64) -> FleetNodeStat {
        FleetNodeStat {
            node: node.to_string(),
            avg_latency_ms,
            success_rate,
            age_secs: 1,
            failure_reason: "none".to_string(),
        }
    }
}
