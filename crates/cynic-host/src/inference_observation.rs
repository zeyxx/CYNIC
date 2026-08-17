//! Live resource observation for bounded inference workloads.
//! No policy or inference knowledge belongs here; this is substrate evidence.

use serde::Serialize;
use std::path::Path;

pub const INFERENCE_OBSERVATION_SCHEMA: &str = "cynic.host.inference_observation.v1";

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct InferenceResourceObservation {
    pub schema: &'static str,
    pub rss_bytes: Option<u64>,
    pub available_memory_bytes: Option<u64>,
    pub total_memory_bytes: Option<u64>,
    pub available_memory_percent: Option<f32>,
    pub rss_percent: Option<f32>,
    pub vram_bytes: Option<u64>,
    pub gtt_bytes: Option<u64>,
}

/// Observe the current process and host memory without mutation or shelling out.
pub fn observe_inference_resources() -> InferenceResourceObservation {
    let total_memory_bytes = read_meminfo_kib("MemTotal:").map(|v| v * 1024);
    let available_memory_bytes = read_meminfo_kib("MemAvailable:").map(|v| v * 1024);
    let rss_bytes = read_status_kib("VmRSS:").map(|v| v * 1024);
    InferenceResourceObservation {
        schema: INFERENCE_OBSERVATION_SCHEMA,
        rss_bytes,
        available_memory_bytes,
        total_memory_bytes,
        available_memory_percent: percent(available_memory_bytes, total_memory_bytes),
        rss_percent: percent(rss_bytes, total_memory_bytes),
        vram_bytes: drm_total("mem_info_vram_total"),
        gtt_bytes: drm_total("mem_info_gtt_total"),
    }
}

fn percent(value: Option<u64>, total: Option<u64>) -> Option<f32> {
    value
        .zip(total)
        .and_then(|(value, total)| (total > 0).then(|| value as f32 * 100.0 / total as f32))
}

fn read_status_kib(field: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix(field)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
}

fn read_meminfo_kib(field: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/meminfo")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix(field)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
}

fn drm_total(name: &str) -> Option<u64> {
    let root = Path::new("/sys/class/drm");
    let entries = std::fs::read_dir(root).ok()?;
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_to_string(entry.path().join("device").join(name)).ok())
        .find_map(|raw| raw.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_is_read_only_and_schemaed() {
        let report = observe_inference_resources();
        assert_eq!(report.schema, INFERENCE_OBSERVATION_SCHEMA);
        assert!(report.available_memory_bytes.is_some());
        assert!(report.rss_bytes.is_some());
        assert!(report.total_memory_bytes.is_some());
        assert!(report.available_memory_percent.is_some());
    }
}
