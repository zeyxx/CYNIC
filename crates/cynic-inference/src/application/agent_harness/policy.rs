use super::types::{HarnessSample, Thresholds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopDecision {
    Continue,
    StopMemory,
    StopWallTime,
    StopFailed,
}

pub fn evaluate_sample(sample: &HarnessSample, thresholds: &Thresholds) -> StopDecision {
    if sample.status != "ok" {
        return StopDecision::StopFailed;
    }
    if thresholds
        .min_available_memory_bytes
        .zip(sample.memory.available_memory_bytes)
        .is_some_and(|(min, available)| available < min)
    {
        return StopDecision::StopMemory;
    }
    if thresholds
        .min_available_memory_percent
        .zip(sample.memory.available_memory_percent)
        .is_some_and(|(min, available)| available < min)
    {
        return StopDecision::StopMemory;
    }
    if thresholds
        .max_rss_bytes
        .zip(sample.memory.rss_bytes)
        .is_some_and(|(max, rss)| rss > max)
    {
        return StopDecision::StopMemory;
    }
    if thresholds
        .max_rss_percent
        .zip(sample.memory.rss_percent)
        .is_some_and(|(max, rss)| rss > max)
    {
        return StopDecision::StopMemory;
    }
    if thresholds
        .max_wall_time_ms
        .zip(sample.wall_time_ms)
        .is_some_and(|(max, elapsed)| elapsed > max)
    {
        return StopDecision::StopWallTime;
    }
    StopDecision::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent_harness::types::MemorySample;

    fn sample() -> HarnessSample {
        HarnessSample {
            task_id: "t".into(),
            runtime_run_id: None,
            http_status: Some(200),
            prompt_tokens: Some(1),
            output_tokens: Some(1),
            prompt_tokens_per_second: None,
            output_tokens_per_second: None,
            wall_time_ms: Some(10),
            event_count: None,
            event_kinds: Vec::new(),
            event_stream_error: None,
            memory: MemorySample {
                available_memory_bytes: Some(100),
                ..Default::default()
            },
            effective_context_per_slot: Some(64),
            status: "ok".into(),
            error: None,
        }
    }

    #[test]
    fn stops_before_memory_threshold() {
        let mut thresholds = Thresholds {
            min_available_memory_bytes: Some(101),
            min_available_memory_percent: None,
            max_rss_bytes: None,
            max_rss_percent: None,
            max_wall_time_ms: None,
        };
        assert_eq!(
            evaluate_sample(&sample(), &thresholds),
            StopDecision::StopMemory
        );
        thresholds.min_available_memory_bytes = Some(99);
        assert_eq!(
            evaluate_sample(&sample(), &thresholds),
            StopDecision::Continue
        );
    }

    #[test]
    fn stops_below_available_memory_percentage() {
        let mut sample = sample();
        sample.memory.available_memory_percent = Some(8.0);
        let thresholds = Thresholds {
            min_available_memory_bytes: None,
            min_available_memory_percent: Some(10.0),
            max_rss_bytes: None,
            max_rss_percent: None,
            max_wall_time_ms: None,
        };
        assert_eq!(
            evaluate_sample(&sample, &thresholds),
            StopDecision::StopMemory
        );
    }
}
