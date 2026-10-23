//! Build placement: least-cost capable runner for a task class.
//!
//! Pure policy over an observed view. The caller assembles the view from
//! live probes (free RAM/disk), the proof journal (proven durations per
//! runner and phase), and the admission queue (queued time per runner).
//! Unknowns stay explicit: an unmeasurable runner or a phase with fewer
//! than MIN_SAMPLES proofs is *incapable-or-explore*, never zero.
//!
//! Score (minimize):
//!   S = queued_ms + duration_ms + lambda * pressure + mu * agent_in_flight
//! where `duration_ms` is the median of recent proofs (heavy-tailed link
//! times forbid the mean), `queued_ms` the queued durations ahead on that
//! runner, `pressure` the used-fraction past the hot threshold, and
//! `agent_in_flight` the requesting agent's running slots (fair share
//! without a central scheduler).

use serde::{Deserialize, Serialize};

use cynic_protocol::runner_capacity::{
    RUNNER_STALENESS_SECS, RunnerCapacitySnapshot, RunnerUsability, decide_runner_usability,
};

pub const BUILD_PLACEMENT_SCHEMA: &str = "cynic.build_placement.v1";
/// Minimum proof samples before a median is trusted. Below this the
/// runner is ranked by headroom only and the outcome is recorded.
pub const MIN_SAMPLES: usize = 5;
/// Used-fraction past which a runner counts as hot.
pub const HOT_THRESHOLD: f64 = 0.8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunnerSnapshot {
    pub node_id: String,
    /// None = unmeasurable (unknown, never zero).
    pub free_ram_bytes: Option<u64>,
    pub total_ram_bytes: Option<u64>,
    /// Sums of proven durations queued ahead on this runner, ms.
    pub queued_ms: u64,
    /// Proven median duration for the task class, ms. None = unexplored.
    pub median_ms: Option<u64>,
    /// How many proof samples back the median. Below MIN_SAMPLES the
    /// median is distrusted and the runner is explore-ranked.
    pub samples: usize,
    /// Requesting agent's slots already running anywhere.
    pub agent_in_flight: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementRank {
    pub node_id: String,
    pub capable: bool,
    pub score_ms: Option<u64>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementDecision {
    pub schema: String,
    pub task_class: String,
    pub ranked: Vec<PlacementRank>,
    pub chosen: Option<String>,
}

/// Rank candidates for `task_class`. `floor_ram_bytes` is the class floor
/// from compute-policy; `lambda` trades velocity against pressure (0 =
/// pure velocity); `mu_ms` is the fair-share cost of one held slot.
///
/// `capacity` is the supervised registry state: only runners present there
/// are eligible, and their RAM figures come from the snapshot, never from
/// a cold probe. An absent runner is incapable with the observe command,
/// never silently skipped.
pub fn place_build(
    task_class: &str,
    floor_ram_bytes: u64,
    lambda: f64,
    mu_ms: u64,
    runners: Vec<RunnerSnapshot>,
    capacity: &[RunnerCapacitySnapshot],
    now_unix_s: u64,
) -> PlacementDecision {
    let mut ranked: Vec<PlacementRank> = runners
        .into_iter()
        .map(|runner| {
            let Some(snap) = capacity.iter().find(|s| s.runner_id == runner.node_id) else {
                return PlacementRank {
                    node_id: runner.node_id,
                    capable: false,
                    score_ms: None,
                    reasons: vec!["unobserved: run cynic fleet observe".to_string()],
                };
            };
            match decide_runner_usability(snap, now_unix_s, floor_ram_bytes, RUNNER_STALENESS_SECS)
            {
                RunnerUsability::Muted(reason) | RunnerUsability::Degraded(reason) => {
                    return PlacementRank {
                        node_id: runner.node_id,
                        capable: false,
                        score_ms: None,
                        reasons: vec![reason],
                    };
                }
                RunnerUsability::Usable => {}
            }
            let Some(free) = snap.free_ram_bytes else {
                return PlacementRank {
                    node_id: runner.node_id,
                    capable: false,
                    score_ms: None,
                    reasons: vec!["unmeasurable_capacity".to_string()],
                };
            };
            if free < floor_ram_bytes {
                return PlacementRank {
                    node_id: runner.node_id,
                    capable: false,
                    score_ms: None,
                    reasons: vec![format!(
                        "below_floor: free {free} < floor {floor_ram_bytes}"
                    )],
                };
            }
            let total = snap.total_ram_bytes.or(runner.total_ram_bytes);
            let mut reasons = Vec::new();
            let duration_ms = match (runner.median_ms, runner.samples) {
                (Some(median), samples) if samples >= MIN_SAMPLES => median,
                _ => {
                    reasons.push("exploring_unproven_duration".to_string());
                    0
                }
            };
            let pressure_ms = match (total, lambda > 0.0) {
                (Some(total), true) if total > 0 => {
                    let used = 1.0 - (free as f64 / total as f64);
                    let hot = (used - HOT_THRESHOLD).max(0.0) / (1.0 - HOT_THRESHOLD);
                    reasons.push(format!("pressure {:.2}", (hot * 100.0).round() / 100.0));
                    (hot * lambda) as u64
                }
                _ => 0,
            };
            let fair_ms = runner.agent_in_flight.saturating_mul(mu_ms);
            if fair_ms > 0 {
                reasons.push(format!("agent_holds {}", runner.agent_in_flight));
            }
            let score = runner
                .queued_ms
                .saturating_add(duration_ms)
                .saturating_add(pressure_ms)
                .saturating_add(fair_ms);
            if runner.queued_ms > 0 {
                reasons.push(format!("queued_ahead_ms {}", runner.queued_ms));
            }
            PlacementRank {
                node_id: runner.node_id,
                capable: true,
                score_ms: Some(score),
                reasons,
            }
        })
        .collect();
    // Capable first by score; exploring (0-duration) runners with headroom
    // sort ahead of proven-slow ones only when their queue is empty — the
    // saturating sum already encodes that. Unknowns sink last.
    ranked.sort_by(|left, right| match (left.score_ms, right.score_ms) {
        (Some(a), Some(b)) => a.cmp(&b).then_with(|| left.node_id.cmp(&right.node_id)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.node_id.cmp(&right.node_id),
    });
    let chosen = ranked
        .iter()
        .find(|rank| rank.capable)
        .map(|rank| rank.node_id.clone());
    PlacementDecision {
        schema: BUILD_PLACEMENT_SCHEMA.to_string(),
        task_class: task_class.to_string(),
        ranked,
        chosen,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runner(node: &str, free_gb: Option<u64>, median: Option<u64>, samples: usize) -> RunnerSnapshot {
        RunnerSnapshot {
            node_id: node.to_string(),
            free_ram_bytes: free_gb.map(|gb| gb * 1024 * 1024 * 1024),
            total_ram_bytes: Some(16 * 1024 * 1024 * 1024),
            queued_ms: 0,
            median_ms: median,
            samples,
            agent_in_flight: 0,
        }
    }

    const NOW: u64 = 1_000_000;

    fn snap(node: &str, free_gb: Option<u64>, observed: u64) -> RunnerCapacitySnapshot {
        RunnerCapacitySnapshot {
            runner_id: node.to_string(),
            observed_unix_s: observed,
            free_ram_bytes: free_gb.map(|gb| gb * 1024 * 1024 * 1024),
            total_ram_bytes: Some(16 * 1024 * 1024 * 1024),
            disk_avail_bytes: Some(10_000_000_000),
            state_root: Some("/srv/cynic/runs".to_string()),
        }
    }

    fn capacity_for(entries: &[(&str, Option<u64>, u64)]) -> Vec<RunnerCapacitySnapshot> {
        entries
            .iter()
            .map(|(node, free_gb, observed)| snap(node, *free_gb, *observed))
            .collect()
    }

    #[test]
    fn unmeasurable_is_incapable_never_zero() {
        let capacity = capacity_for(&[("blind", None, NOW), ("seen", Some(8), NOW)]);
        let decision = place_build(
            "rust_build",
            4 * 1024 * 1024 * 1024,
            0.0,
            0,
            vec![
                runner("blind", None, Some(10_000), 10),
                runner("seen", Some(8), Some(200_000), 10),
            ],
            &capacity,
            NOW,
        );
        assert_eq!(decision.chosen.as_deref(), Some("seen"));
        assert!(!decision.ranked[1].capable);
    }

    #[test]
    fn below_floor_refuses() {
        let capacity = capacity_for(&[("thin", Some(2), NOW), ("fat", Some(8), NOW)]);
        let decision = place_build(
            "rust_build",
            4 * 1024 * 1024 * 1024,
            0.0,
            0,
            vec![
                runner("thin", Some(2), Some(1_000), 10),
                runner("fat", Some(8), Some(300_000), 10),
            ],
            &capacity,
            NOW,
        );
        assert_eq!(decision.chosen.as_deref(), Some("fat"));
    }

    #[test]
    fn queue_wait_beats_faster_runner() {
        let mut busy = runner("fast", Some(8), Some(30_000), 10);
        busy.queued_ms = 300_000;
        let capacity = capacity_for(&[("fast", Some(8), NOW), ("slow", Some(8), NOW)]);
        let decision = place_build(
            "rust_check_light",
            1024 * 1024 * 1024,
            0.0,
            0,
            vec![busy, runner("slow", Some(8), Some(60_000), 10)],
            &capacity,
            NOW,
        );
        assert_eq!(decision.chosen.as_deref(), Some("slow"));
    }

    #[test]
    fn distrusted_median_explores_by_headroom_order() {
        let capacity = capacity_for(&[("new", Some(12), NOW), ("old", Some(8), NOW)]);
        let decision = place_build(
            "rust_build",
            4 * 1024 * 1024 * 1024,
            0.0,
            0,
            vec![
                runner("new", Some(12), Some(5_000), 2),
                runner("old", Some(8), Some(200_000), 40),
            ],
            &capacity,
            NOW,
        );
        assert_eq!(decision.chosen.as_deref(), Some("new"));
        assert!(decision.ranked[0]
            .reasons
            .iter()
            .any(|r| r == "exploring_unproven_duration"));
    }

    #[test]
    fn agent_holding_slots_yields() {
        let mut holder = runner("held", Some(12), Some(30_000), 10);
        holder.agent_in_flight = 2;
        let capacity = capacity_for(&[("held", Some(12), NOW), ("free", Some(8), NOW)]);
        let decision = place_build(
            "rust_check_light",
            1024 * 1024 * 1024,
            0.0,
            60_000,
            vec![holder, runner("free", Some(8), Some(60_000), 10)],
            &capacity,
            NOW,
        );
        assert_eq!(decision.chosen.as_deref(), Some("free"));
    }

    #[test]
    fn unknown_runner_names_observe_command() {
        let capacity = capacity_for(&[("known", Some(8), NOW)]);
        let decision = place_build(
            "rust_build",
            4 * 1024 * 1024 * 1024,
            0.0,
            0,
            vec![
                runner("ghost", Some(8), Some(10_000), 10),
                runner("known", Some(8), Some(200_000), 10),
            ],
            &capacity,
            NOW,
        );
        assert_eq!(decision.chosen.as_deref(), Some("known"));
        let ghost = decision
            .ranked
            .iter()
            .find(|r| r.node_id == "ghost")
            .expect("ghost ranked");
        assert!(!ghost.capable);
        assert!(ghost.reasons.iter().any(|r| r.contains("fleet observe")));
    }

    #[test]
    fn stale_snapshot_mutes() {
        let stale = NOW - RUNNER_STALENESS_SECS - 1;
        let capacity = capacity_for(&[("old", Some(8), stale), ("fresh", Some(8), NOW)]);
        let decision = place_build(
            "rust_build",
            4 * 1024 * 1024 * 1024,
            0.0,
            0,
            vec![
                runner("old", Some(8), Some(10_000), 10),
                runner("fresh", Some(8), Some(200_000), 10),
            ],
            &capacity,
            NOW,
        );
        assert_eq!(decision.chosen.as_deref(), Some("fresh"));
    }

    #[test]
    fn snapshot_below_floor_degrades() {
        let capacity = capacity_for(&[("thin", Some(2), NOW), ("fat", Some(8), NOW)]);
        let decision = place_build(
            "rust_build",
            4 * 1024 * 1024 * 1024,
            0.0,
            0,
            vec![
                runner("thin", Some(8), Some(10_000), 10),
                runner("fat", Some(8), Some(200_000), 10),
            ],
            &capacity,
            NOW,
        );
        assert_eq!(decision.chosen.as_deref(), Some("fat"));
        let thin = decision
            .ranked
            .iter()
            .find(|r| r.node_id == "thin")
            .expect("thin ranked");
        assert!(!thin.capable);
        assert!(thin.reasons.iter().any(|r| r.contains("below")));
    }
}
