//! Build-vertical reliability contract: metric identities, targets, and the
//! pure computation from retained evidence counts.
//!
//! Measurement first: every metric carries numerator/denominator evidence.
//! A zero denominator yields `percent: None` (unknown), never a fabricated
//! zero or hundred. Targets are per-metric; 100% is not the global goal.
//! Collectors that assemble inputs from proof bundles live outside this
//! module; this contract only fixes what is measured and how.

use serde::{Deserialize, Serialize};

pub const BUILD_RELIABILITY_REPORT_SCHEMA: &str = "cynic.build_reliability_report.v1";

pub const TRIGGER_CONVERGED_TARGET_PERCENT: u8 = 90;
pub const SLICE_FIRST_GREEN_TARGET_PERCENT: u8 = 70;
pub const DEFECT_ESCAPE_TARGET_PERCENT: u8 = 0;
pub const BUILDER_REACHABLE_TARGET_PERCENT: u8 = 95;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReliabilityMetric {
    pub id: String,
    pub target_percent: u8,
    pub numerator: usize,
    pub denominator: usize,
    /// Rounded percentage, or `None` when nothing was observed.
    pub percent: Option<u8>,
    /// `true` when observed evidence meets the target.
    pub meets_target: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildReliabilityInput {
    /// Observation window label, e.g. a date range. Informational only.
    pub window: String,
    pub triggers_total: usize,
    pub triggers_converged: usize,
    pub slices_run: usize,
    pub slices_green_first_try: usize,
    pub defects_caught_before_merge: usize,
    pub defects_escaped_to_merge: usize,
    pub builder_health_checks: usize,
    pub builder_reachable: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildReliabilityReport {
    pub schema: String,
    pub window: String,
    pub metrics: Vec<ReliabilityMetric>,
}

fn metric(id: &str, target_percent: u8, numerator: usize, denominator: usize) -> ReliabilityMetric {
    let percent = if denominator == 0 {
        None
    } else {
        Some((numerator * 100 / denominator).min(100) as u8)
    };
    let meets_target = percent.is_some_and(|observed| observed >= target_percent);
    ReliabilityMetric {
        id: id.to_string(),
        target_percent,
        numerator,
        denominator,
        percent,
        meets_target,
    }
}

/// Escape metric: any escape fails the target, including the first.
fn escape_metric(defects_escaped_to_merge: usize, total_defects: usize) -> ReliabilityMetric {
    let mut metric = metric(
        "defect_escape_ratio",
        DEFECT_ESCAPE_TARGET_PERCENT,
        defects_escaped_to_merge,
        total_defects,
    );
    metric.meets_target = defects_escaped_to_merge == 0 && total_defects > 0;
    metric
}

/// Compute the reliability report from retained evidence counts.
pub fn compute_build_reliability(input: &BuildReliabilityInput) -> BuildReliabilityReport {
    let total_defects = input.defects_caught_before_merge + input.defects_escaped_to_merge;
    BuildReliabilityReport {
        schema: BUILD_RELIABILITY_REPORT_SCHEMA.to_string(),
        window: input.window.clone(),
        metrics: vec![
            metric(
                "trigger_converged_ratio",
                TRIGGER_CONVERGED_TARGET_PERCENT,
                input.triggers_converged,
                input.triggers_total,
            ),
            metric(
                "slice_first_green_ratio",
                SLICE_FIRST_GREEN_TARGET_PERCENT,
                input.slices_green_first_try,
                input.slices_run,
            ),
            escape_metric(input.defects_escaped_to_merge, total_defects),
            metric(
                "builder_reachable_ratio",
                BUILDER_REACHABLE_TARGET_PERCENT,
                input.builder_reachable,
                input.builder_health_checks,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> BuildReliabilityInput {
        BuildReliabilityInput {
            window: "2026-09-03/04".to_string(),
            triggers_total: 2,
            triggers_converged: 1,
            slices_run: 14,
            slices_green_first_try: 6,
            defects_caught_before_merge: 5,
            defects_escaped_to_merge: 0,
            builder_health_checks: 12,
            builder_reachable: 12,
        }
    }

    #[test]
    fn ratios_follow_retained_evidence() {
        let report = compute_build_reliability(&input());

        assert_eq!(report.schema, BUILD_RELIABILITY_REPORT_SCHEMA);
        let converged = &report.metrics[0];
        assert_eq!(converged.id, "trigger_converged_ratio");
        assert_eq!(converged.percent, Some(50));
        assert!(!converged.meets_target);
        let first_green = &report.metrics[1];
        assert_eq!(first_green.percent, Some(42));
        let reachable = &report.metrics[3];
        assert_eq!(reachable.percent, Some(100));
        assert!(reachable.meets_target);
    }

    #[test]
    fn zero_escape_meets_zero_target_only_with_evidence() {
        let report = compute_build_reliability(&input());
        assert!(report.metrics[2].meets_target);

        let mut escaped = input();
        escaped.defects_escaped_to_merge = 1;
        let report = compute_build_reliability(&escaped);
        assert!(!report.metrics[2].meets_target);
        assert_eq!(report.metrics[2].percent, Some(16));
    }

    #[test]
    fn empty_window_stays_unknown_not_zero() {
        let empty = BuildReliabilityInput {
            window: "empty".to_string(),
            triggers_total: 0,
            triggers_converged: 0,
            slices_run: 0,
            slices_green_first_try: 0,
            defects_caught_before_merge: 0,
            defects_escaped_to_merge: 0,
            builder_health_checks: 0,
            builder_reachable: 0,
        };
        let report = compute_build_reliability(&empty);

        assert!(report.metrics.iter().all(|metric| metric.percent.is_none()));
        assert!(report.metrics.iter().all(|metric| !metric.meets_target));
    }
}
