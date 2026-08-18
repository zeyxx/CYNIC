use super::types::{AgentHarnessReport, HarnessSample};

pub const BENCHMARK_MATRIX_SCHEMA: &str = "cynic.agent_harness.benchmark_report.v1";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BenchmarkSummary {
    pub scenario_count: usize,
    pub completed_count: usize,
    pub stopped_count: usize,
    pub failed_count: usize,
    pub sample_count: usize,
    pub mean_wall_time_ms: Option<f64>,
    pub mean_output_tokens_per_second: Option<f64>,
    pub max_rss_bytes: Option<u64>,
    pub min_available_memory_percent: Option<f32>,
    pub max_rss_percent: Option<f32>,
    pub event_proof_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BenchmarkReport {
    pub schema: String,
    pub plan_id: String,
    pub reports: Vec<AgentHarnessReport>,
    pub summary: BenchmarkSummary,
}

pub fn aggregate_reports(
    plan_id: impl Into<String>,
    reports: Vec<AgentHarnessReport>,
) -> BenchmarkReport {
    let samples: Vec<&HarnessSample> = reports
        .iter()
        .flat_map(|report| report.samples.iter())
        .collect();
    let wall_times: Vec<f64> = samples
        .iter()
        .filter_map(|sample| sample.wall_time_ms.map(|value| value as f64))
        .collect();
    let output_rates: Vec<f64> = samples
        .iter()
        .filter_map(|sample| sample.output_tokens_per_second.map(|value| value as f64))
        .collect();
    let max_rss_bytes = samples
        .iter()
        .filter_map(|sample| sample.memory.rss_bytes)
        .max();
    let min_available_memory_percent = samples
        .iter()
        .filter_map(|sample| sample.memory.available_memory_percent)
        .min_by(|left, right| left.total_cmp(right));
    let max_rss_percent = samples
        .iter()
        .filter_map(|sample| sample.memory.rss_percent)
        .max_by(|left, right| left.total_cmp(right));
    let summary = BenchmarkSummary {
        scenario_count: reports.len(),
        completed_count: reports
            .iter()
            .filter(|report| report.status == "completed")
            .count(),
        stopped_count: reports
            .iter()
            .filter(|report| {
                report.status.starts_with("stopped_") && !report.status.contains("failed")
            })
            .count(),
        failed_count: reports
            .iter()
            .filter(|report| {
                report.status.starts_with("blocked")
                    || report.status == "running"
                    || report.status.contains("failed")
            })
            .count(),
        sample_count: samples.len(),
        mean_wall_time_ms: mean(&wall_times),
        mean_output_tokens_per_second: mean(&output_rates),
        max_rss_bytes,
        min_available_memory_percent,
        max_rss_percent,
        event_proof_count: samples
            .iter()
            .filter(|sample| sample.event_count.is_some() && sample.event_stream_error.is_none())
            .count(),
    };
    BenchmarkReport {
        schema: BENCHMARK_MATRIX_SCHEMA.into(),
        plan_id: plan_id.into(),
        reports,
        summary,
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

/// Stable machine-readable projection; the caller owns filesystem persistence.
pub fn encode_report(report: &AgentHarnessReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report).map(|json| format!("{json}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent_harness::{AgentHarnessReport, BENCHMARK_SCHEMA};

    #[test]
    fn report_preserves_schema() {
        let report = AgentHarnessReport {
            schema: BENCHMARK_SCHEMA.into(),
            run_id: "r1".into(),
            scenario: serde_json::from_value(serde_json::json!({"scenario_id":"s","model":{"role":"worker","parameters":{"model_id":"m","model_path":"/m.gguf","architecture":"qwen","quantization":"q4_k_m","context_requested":131072,"parallel_slots":2,"context_per_slot":65536,"kv_cache_type_k":"q4_0","kv_cache_type_v":"q4_0","flash_attention":true,"gpu_layers":-1,"fit_mode":"off","threads":16,"batch_size":512,"ubatch_size":128,"temperature":0.2,"top_p":0.95,"reasoning_budget":128}},"tasks":[],"thresholds":{}})).unwrap(),
            samples: vec![],
            started_at: "now".into(),
            finished_at: None,
            status: "planned".into(),
            cycle: None,
            transition: None,
            runtime_transition: None,
            runtime_launch_plan: None,
        };
        assert!(encode_report(&report).unwrap().contains(BENCHMARK_SCHEMA));
    }

    #[test]
    fn aggregate_reports_exposes_global_metrics() {
        let report = AgentHarnessReport {
            schema: BENCHMARK_SCHEMA.into(),
            run_id: "r1".into(),
            scenario: serde_json::from_value(serde_json::json!({"scenario_id":"s","model":{"role":"worker","parameters":{"model_id":"m","model_path":"/m.gguf","architecture":null,"quantization":null,"context_requested":1,"parallel_slots":1,"context_per_slot":null,"kv_cache_type_k":null,"kv_cache_type_v":null,"flash_attention":null,"gpu_layers":null,"fit_mode":null,"threads":null,"batch_size":null,"ubatch_size":null,"temperature":null,"top_p":null,"reasoning_budget":null}},"tasks":[],"thresholds":{"min_available_memory_bytes":null,"max_rss_bytes":null,"max_wall_time_ms":null}})).unwrap(),
            samples: vec![],
            started_at: "now".into(),
            finished_at: None,
            status: "completed".into(),
            cycle: None,
            transition: None,
            runtime_transition: None,
            runtime_launch_plan: None,
        };
        let matrix = aggregate_reports("p1", vec![report]);
        assert_eq!(matrix.schema, BENCHMARK_MATRIX_SCHEMA);
        assert_eq!(matrix.summary.completed_count, 1);
    }

    #[test]
    fn aggregate_reports_retains_memory_and_event_proof_metrics() {
        let report = AgentHarnessReport {
            schema: BENCHMARK_SCHEMA.into(),
            run_id: "r2".into(),
            scenario: serde_json::from_value(serde_json::json!({
                "scenario_id":"s2",
                "model":{"role":"worker","parameters":{"model_id":"m","model_path":"/m.gguf","context_requested":1,"parallel_slots":1}},
                "tasks":[], "thresholds":{}
            })).unwrap(),
            samples: vec![HarnessSample {
                task_id: "t1".into(),
                runtime_run_id: Some("run_1".into()),
                http_status: Some(200),
                prompt_tokens: Some(4),
                output_tokens: Some(8),
                prompt_tokens_per_second: None,
                output_tokens_per_second: Some(12.5),
                wall_time_ms: Some(640),
                event_count: Some(3),
                event_kinds: vec!["message.delta".into()],
                event_stream_error: None,
                memory: crate::application::agent_harness::MemorySample {
                    rss_bytes: Some(100),
                    available_memory_bytes: Some(900),
                    total_memory_bytes: Some(1_000),
                    available_memory_percent: Some(90.0),
                    rss_percent: Some(10.0),
                    vram_bytes: None,
                    gtt_bytes: None,
                },
                effective_context_per_slot: Some(1),
                status: "ok".into(),
                error: None,
            }],
            started_at: "now".into(),
            finished_at: Some("later".into()),
            status: "completed".into(),
            cycle: None,
            transition: None,
            runtime_transition: None,
            runtime_launch_plan: None,
        };
        let summary = aggregate_reports("p2", vec![report]).summary;
        assert_eq!(summary.sample_count, 1);
        assert_eq!(summary.max_rss_bytes, Some(100));
        assert_eq!(summary.min_available_memory_percent, Some(90.0));
        assert_eq!(summary.max_rss_percent, Some(10.0));
        assert_eq!(summary.event_proof_count, 1);
        assert_eq!(summary.mean_output_tokens_per_second, Some(12.5));
    }
}
