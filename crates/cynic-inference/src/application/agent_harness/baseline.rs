use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::report::BenchmarkReport;

pub const BENCHMARK_BASELINE_SCHEMA: &str = "cynic.agent_harness.baseline.v1";
pub const BENCHMARK_BASELINE_ASSESSMENT_SCHEMA: &str = "cynic.agent_harness.baseline_assessment.v1";

/// An immutable, model-specific performance acceptance baseline.
///
/// The digest prevents a mutable alias such as `latest` from silently changing
/// what is being compared. Host and engine changes remain visible in the run
/// report and must be reviewed by the baseline owner before promotion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkBaseline {
    pub schema: String,
    pub baseline_id: String,
    pub plan_id: String,
    pub model_id: String,
    pub model_digest: String,
    pub min_success_percent: f64,
    #[serde(default)]
    pub min_mean_output_tokens_per_second: Option<f64>,
    #[serde(default)]
    pub max_mean_wall_time_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkBaselineAssessment {
    pub schema: String,
    pub baseline_id: String,
    pub status: String,
    pub comparable: bool,
    pub passed: Option<bool>,
    pub observed_model_ids: Vec<String>,
    pub observed_model_digests: Vec<String>,
    pub success_percent: f64,
    pub mean_output_tokens_per_second: Option<f64>,
    pub mean_wall_time_ms: Option<f64>,
    pub failed_checks: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

pub fn assess_baseline(
    report: &BenchmarkReport,
    baseline: &BenchmarkBaseline,
) -> BenchmarkBaselineAssessment {
    let observed_model_ids: BTreeSet<String> = report
        .reports
        .iter()
        .map(|run| run.scenario.model.parameters.model_id.clone())
        .collect();
    let observed_model_digests: BTreeSet<String> = report
        .reports
        .iter()
        .filter_map(|run| run.scenario.model.parameters.model_digest.clone())
        .collect();
    let mut residual_unknowns = Vec::new();
    let mut failed_checks = Vec::new();

    if baseline.schema != BENCHMARK_BASELINE_SCHEMA {
        residual_unknowns.push(format!("unsupported_baseline_schema:{}", baseline.schema));
    }
    if report.plan_id != baseline.plan_id {
        residual_unknowns.push(format!(
            "plan_id_mismatch:expected={} actual={}",
            baseline.plan_id, report.plan_id
        ));
    }
    if observed_model_ids.len() != 1 || !observed_model_ids.contains(&baseline.model_id) {
        residual_unknowns.push(format!(
            "model_id_mismatch:expected={} actual={}",
            baseline.model_id,
            observed_model_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if observed_model_digests.len() != 1 || !observed_model_digests.contains(&baseline.model_digest)
    {
        residual_unknowns.push(format!(
            "model_digest_mismatch_or_missing:expected={} actual={}",
            baseline.model_digest,
            observed_model_digests
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    let scenario_count = report.summary.scenario_count;
    let success_percent = if scenario_count == 0 {
        0.0
    } else {
        report.summary.completed_count as f64 * 100.0 / scenario_count as f64
    };
    let comparable = residual_unknowns.is_empty();
    if comparable {
        if success_percent < baseline.min_success_percent {
            failed_checks.push(format!(
                "success_percent_below_baseline:actual={success_percent:.2} minimum={:.2}",
                baseline.min_success_percent
            ));
        }
        if let Some(minimum) = baseline.min_mean_output_tokens_per_second {
            match report.summary.mean_output_tokens_per_second {
                Some(actual) if actual >= minimum => {}
                Some(actual) => failed_checks.push(format!(
                    "output_tokens_per_second_below_baseline:actual={actual:.2} minimum={minimum:.2}"
                )),
                None => residual_unknowns.push("output_tokens_per_second_unknown".into()),
            }
        }
        if let Some(maximum) = baseline.max_mean_wall_time_ms {
            match report.summary.mean_wall_time_ms {
                Some(actual) if actual <= maximum => {}
                Some(actual) => failed_checks.push(format!(
                    "wall_time_above_baseline:actual={actual:.2} maximum={maximum:.2}"
                )),
                None => residual_unknowns.push("mean_wall_time_unknown".into()),
            }
        }
    }
    let comparable = residual_unknowns.is_empty();
    let passed = comparable.then(|| failed_checks.is_empty());
    let status = if !comparable {
        "not_comparable"
    } else if passed == Some(true) {
        "passed"
    } else {
        "regressed"
    };

    BenchmarkBaselineAssessment {
        schema: BENCHMARK_BASELINE_ASSESSMENT_SCHEMA.into(),
        baseline_id: baseline.baseline_id.clone(),
        status: status.into(),
        comparable,
        passed,
        observed_model_ids: observed_model_ids.into_iter().collect(),
        observed_model_digests: observed_model_digests.into_iter().collect(),
        success_percent,
        mean_output_tokens_per_second: report.summary.mean_output_tokens_per_second,
        mean_wall_time_ms: report.summary.mean_wall_time_ms,
        failed_checks,
        residual_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent_harness::{AgentHarnessReport, BenchmarkSummary, ScenarioSpec};

    fn report(digest: Option<&str>) -> BenchmarkReport {
        let scenario: ScenarioSpec = serde_json::from_value(serde_json::json!({
            "scenario_id":"s", "model":{"role":"engine","parameters":{
                "model_id":"model-a", "model_path":"/model.gguf", "model_digest":digest,
                "context_requested":1,"parallel_slots":1
            }}, "tasks":[], "thresholds":{}
        }))
        .expect("scenario");
        BenchmarkReport {
            schema: "cynic.agent_harness.benchmark_report.v1".into(),
            plan_id: "plan-a".into(),
            reports: vec![AgentHarnessReport {
                schema: "cynic.agent_harness.run.v1".into(),
                run_id: "r".into(),
                scenario,
                samples: vec![],
                started_at: "now".into(),
                finished_at: Some("later".into()),
                status: "completed".into(),
                cycle: None,
                transition: None,
                runtime_transition: None,
                runtime_launch_plan: None,
            }],
            summary: BenchmarkSummary {
                scenario_count: 1,
                completed_count: 1,
                stopped_count: 0,
                failed_count: 0,
                sample_count: 1,
                mean_wall_time_ms: Some(100.0),
                mean_output_tokens_per_second: Some(20.0),
                max_rss_bytes: None,
                min_available_memory_percent: None,
                max_rss_percent: None,
                event_proof_count: 0,
            },
        }
    }

    fn baseline() -> BenchmarkBaseline {
        BenchmarkBaseline {
            schema: BENCHMARK_BASELINE_SCHEMA.into(),
            baseline_id: "b".into(),
            plan_id: "plan-a".into(),
            model_id: "model-a".into(),
            model_digest: "sha256:abc".into(),
            min_success_percent: 100.0,
            min_mean_output_tokens_per_second: Some(10.0),
            max_mean_wall_time_ms: Some(200.0),
        }
    }

    #[test]
    fn accepts_matching_immutable_model_baseline() {
        assert_eq!(
            assess_baseline(&report(Some("sha256:abc")), &baseline()).status,
            "passed"
        );
    }

    #[test]
    fn refuses_alias_only_comparison_without_digest() {
        let assessment = assess_baseline(&report(None), &baseline());
        assert_eq!(assessment.status, "not_comparable");
        assert!(assessment.residual_unknowns[0].contains("model_digest"));
    }
}
