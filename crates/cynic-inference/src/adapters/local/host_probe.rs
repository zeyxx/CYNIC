//! Filesystem-backed host hardware probe (Linux `/proc` + DRM sysfs).
//!
//! Impure reads live here; all parsing and budget decisions stay pure in
//! `domain::hardware_profile` and are unit-tested without touching the host.

use crate::domain::hardware_profile::{
    DrmCardReadings, HardwareProfile, classify_gpu, parse_cpu_flags, parse_cpu_governor,
    parse_meminfo,
};
use crate::ports::hardware_probe_port::HardwareProbePort;

/// `/proc` + sysfs probe with conservative fallbacks when unreadable.
#[derive(Debug, Clone, Copy, Default)]
pub struct FsHardwareProbe;

const FALLBACK_TOTAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const FALLBACK_AVAIL_BYTES: u64 = 4 * 1024 * 1024 * 1024;

impl HardwareProbePort for FsHardwareProbe {
    fn probe_hardware(&self) -> HardwareProfile {
        let (total, available) = std::fs::read_to_string("/proc/meminfo")
            .map(|raw| parse_meminfo(&raw))
            .unwrap_or((FALLBACK_TOTAL_BYTES, FALLBACK_AVAIL_BYTES));
        let cpu_threads = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
        let cpu_flags = std::fs::read_to_string("/proc/cpuinfo")
            .map(|raw| parse_cpu_flags(&raw))
            .unwrap_or_default();
        let cpu_governor =
            std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
                .ok()
                .and_then(|raw| parse_cpu_governor(&raw));
        let cards = read_drm_cards();
        let (gpu, gtt_total_bytes) = classify_gpu(&cards);
        let mem_bw_gbps = std::env::var("CYNIC_ESTIMATED_MEM_BW_GBPS")
            .ok()
            .and_then(|v| v.trim().parse::<f64>().ok());

        HardwareProfile::from_observed(
            total,
            available,
            cpu_threads,
            cpu_flags,
            cpu_governor,
            gpu,
            gtt_total_bytes,
            mem_bw_gbps,
        )
    }
}

/// Best-effort pre-read of DRM card sysfs values (pure classification happens
/// in the domain).
fn read_drm_cards() -> Vec<DrmCardReadings> {
    let mut cards = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return cards;
    };
    for entry in entries.flatten() {
        let card_name = entry.file_name().to_string_lossy().to_string();
        if !card_name.starts_with("card") || card_name.contains('-') {
            continue;
        }
        let base = entry.path().join("device");
        let read_trimmed = |name: &str| {
            std::fs::read_to_string(base.join(name))
                .ok()
                .map(|v| v.trim().to_string())
        };
        let parse_u64 = |name: &str| {
            read_trimmed(name).and_then(|v| v.parse::<u64>().ok())
        };
        cards.push(DrmCardReadings {
            card_name,
            vendor: read_trimmed("vendor"),
            gtt_total: parse_u64("mem_info_gtt_total"),
            vram_total: parse_u64("mem_info_vram_total"),
        });
    }
    cards
}

/// Convenience free function for call sites that do not inject the port yet.
pub fn probe_hardware() -> HardwareProfile {
    FsHardwareProbe.probe_hardware()
}
