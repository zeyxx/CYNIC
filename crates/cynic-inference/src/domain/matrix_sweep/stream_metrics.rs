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
            predicted_tokens_per_second: (completion_tokens as f64 / (total_elapsed_ms / 1000.0).max(0.001)) as f32,
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
}
