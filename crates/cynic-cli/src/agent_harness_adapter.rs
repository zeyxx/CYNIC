//! CLI-level wiring between host substrate observation and inference harness.

use cynic_inference::{HarnessObserver, MemorySample};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct HostResourceObserver;

impl HarnessObserver for HostResourceObserver {
    fn observe(&self) -> MemorySample {
        let observed = cynic_host::inference_observation::observe_inference_resources();
        MemorySample {
            rss_bytes: observed.rss_bytes,
            available_memory_bytes: observed.available_memory_bytes,
            total_memory_bytes: observed.total_memory_bytes,
            available_memory_percent: observed.available_memory_percent,
            rss_percent: observed.rss_percent,
            vram_bytes: observed.vram_bytes,
            gtt_bytes: observed.gtt_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_host_observation_without_recomputing_policy() {
        let sample = HostResourceObserver.observe();
        assert!(sample.rss_bytes.is_some());
        assert!(sample.available_memory_bytes.is_some());
    }
}
