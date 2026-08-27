//! Streaming metrics, latency jitter, and emission stability analysis.

use super::types::StreamDynamicsReport;

/// Computes streaming dynamics from a series of chunk timestamps (in milliseconds).
pub fn compute_stream_dynamics(
    chunk_timestamps_ms: &[f64],
    prompt_tokens: u32,
    completion_tokens: u32,
    total_elapsed_ms: f64,
) -> StreamDynamicsReport {
    if chunk_timestamps_ms.is_empty() {
        return StreamDynamicsReport {
            ttft_ms: total_elapsed_ms as f32,
            mean_inter_token_ms: 0.0,
            jitter_variance_ms: 0.0,
            predicted_tokens_per_second: (completion_tokens as f64
                / (total_elapsed_ms / 1000.0).max(0.001))
                as f32,
            prompt_tokens_per_second: 0.0,
        };
    }

    let ttft_ms = chunk_timestamps_ms[0] as f32;

    // Calculate intervals between successive chunks
    let mut intervals = Vec::new();
    for i in 1..chunk_timestamps_ms.len() {
        let diff = (chunk_timestamps_ms[i] - chunk_timestamps_ms[i - 1]).max(0.0);
        intervals.push(diff);
    }

    let mean_inter_token_ms = if !intervals.is_empty() {
        intervals.iter().sum::<f64>() / intervals.len() as f64
    } else {
        0.0
    };

    let jitter_variance_ms = if intervals.len() > 1 {
        let variance = intervals
            .iter()
            .map(|&val| (val - mean_inter_token_ms).powi(2))
            .sum::<f64>()
            / (intervals.len() - 1) as f64;
        variance.sqrt()
    } else {
        0.0
    };

    let gen_time_secs = (total_elapsed_ms - ttft_ms as f64) / 1000.0;
    let predicted_tokens_per_second = if gen_time_secs > 0.001 && completion_tokens > 0 {
        (completion_tokens as f64 / gen_time_secs) as f32
    } else {
        0.0
    };

    let prompt_time_secs = (ttft_ms as f64) / 1000.0;
    let prompt_tokens_per_second = if prompt_time_secs > 0.001 && prompt_tokens > 0 {
        (prompt_tokens as f64 / prompt_time_secs) as f32
    } else {
        0.0
    };

    StreamDynamicsReport {
        ttft_ms,
        mean_inter_token_ms: mean_inter_token_ms as f32,
        jitter_variance_ms: jitter_variance_ms as f32,
        predicted_tokens_per_second,
        prompt_tokens_per_second,
    }
}

/// Calculates the Time To Correct Solution (TTCS) in milliseconds.
/// Returns None if the task did not reach a validated correct solution.
pub fn calculate_ttcs_ms(
    total_elapsed_ms: f64,
    retries: u32,
    validation_passed: bool,
) -> Option<f64> {
    if validation_passed && total_elapsed_ms.is_finite() && total_elapsed_ms > 0.0 {
        let retry_factor = 1.0 + (retries as f64 * 0.05);
        Some(total_elapsed_ms * retry_factor)
    } else {
        None
    }
}

/// Calculates successful tasks per hour based on TTCS and success rate.
///
/// Formula: `(3600 * 1000 / ttcs_ms) * success_rate`.
pub fn calculate_tasks_per_hour(ttcs_ms: f64, success_rate: f64) -> f64 {
    if ttcs_ms <= 0.0 || !ttcs_ms.is_finite() || !success_rate.is_finite() {
        return 0.0;
    }
    let clamped_success = success_rate.clamp(0.0, 1.0);
    let ms_per_hour = 3_600_000.0;
    (ms_per_hour / ttcs_ms) * clamped_success
}

/// Calculates energy efficiency in Wh per successful task.
pub fn calculate_wh_per_successful_task(power_watts: f64, ttcs_ms: f64) -> f64 {
    if power_watts <= 0.0 || ttcs_ms <= 0.0 {
        return 0.0;
    }
    let hours = ttcs_ms / 3_600_000.0;
    power_watts * hours
}

/// Calculates Failure Recovery Rate (FRR): P(success B | failure A).
pub fn calculate_failure_recovery_rate(
    recovered_by_b_count: u32,
    failed_by_a_count: u32,
) -> Option<f64> {
    if failed_by_a_count == 0 {
        None
    } else {
        let rate = recovered_by_b_count as f64 / failed_by_a_count as f64;
        Some(rate.clamp(0.0, 1.0))
    }
}

/// Comparative reasoning telemetry contrasting thinking vs visible token emissions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReasoningTelemetry {
    pub thinking_tokens: u32,
    pub visible_output_tokens: u32,
    /// Ratio of thinking tokens to final answer tokens (overthinking factor).
    pub overthinking_ratio: f32,
    pub total_ttcs_ms: f64,
    pub quality_pass1_score: f32,
    /// Quality-Adjusted Tasks Per Hour: (3600000 / TTCS) * Pass@1.
    pub quality_adjusted_tph: f64,
}

/// Calculates the overthinking ratio: thinking_tokens / visible_output_tokens.
pub fn calculate_overthinking_ratio(thinking_tokens: u32, visible_output_tokens: u32) -> f32 {
    if visible_output_tokens == 0 {
        thinking_tokens as f32
    } else {
        thinking_tokens as f32 / visible_output_tokens as f32
    }
}

/// Calculates Quality-Adjusted Tasks Per Hour: (3600000 / TTCS_ms) * pass1_score.
pub fn calculate_quality_adjusted_tph(ttcs_ms: f64, pass1_score: f32) -> f64 {
    if ttcs_ms <= 0.0 || !ttcs_ms.is_finite() {
        return 0.0;
    }
    let clamped_score = (pass1_score.clamp(0.0, 1.0)) as f64;
    let ms_per_hour = 3_600_000.0;
    (ms_per_hour / ttcs_ms) * clamped_score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_accurate_stream_jitter() {
        let timestamps = vec![150.0, 180.0, 215.0, 245.0, 280.0];
        let report = compute_stream_dynamics(&timestamps, 40, 5, 290.0);

        assert_eq!(report.ttft_ms, 150.0);
        assert!(report.mean_inter_token_ms > 30.0 && report.mean_inter_token_ms < 35.0);
        assert!(report.predicted_tokens_per_second > 0.0);
    }

    #[test]
    fn test_ttcs_and_tasks_per_hour_calculations() {
        let ttcs = calculate_ttcs_ms(10_000.0, 0, true).expect("should calculate TTCS");
        assert_eq!(ttcs, 10_000.0);

        // 10s per task, 100% success -> 360 tasks/hour
        let tph = calculate_tasks_per_hour(ttcs, 1.0);
        assert_eq!(tph, 360.0);

        // 10s per task, 50% success -> 180 tasks/hour
        let tph_50 = calculate_tasks_per_hour(ttcs, 0.5);
        assert_eq!(tph_50, 180.0);

        let failed = calculate_ttcs_ms(10_000.0, 2, false);
        assert_eq!(failed, None);
    }

    #[test]
    fn test_failure_recovery_rate_calculation() {
        let frr = calculate_failure_recovery_rate(4, 10).expect("should calculate FRR");
        assert_eq!(frr, 0.4);

        let zero_failures = calculate_failure_recovery_rate(0, 0);
        assert_eq!(zero_failures, None);
    }

    #[test]
    fn test_reasoning_telemetry_metrics() {
        // 200 thinking tokens, 50 output tokens -> overthinking ratio = 4.0
        let ratio = calculate_overthinking_ratio(200, 50);
        assert_eq!(ratio, 4.0);

        // TTCS = 5000ms (5s), Pass@1 = 0.90 -> 720 base tph * 0.90 = 648.0 quality-adjusted tph
        let qa_tph = calculate_quality_adjusted_tph(5000.0, 0.90);
        assert_eq!(qa_tph, 648.0);
    }
}
