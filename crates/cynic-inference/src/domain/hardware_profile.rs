//! Host hardware probe for model sizing and launch planning.
//!
//! Pure observation of /proc + DRM + optional plugin dirs — no vendor SDK.
//! Agnostic: no frozen machine names; all decisions derive from probed fields.
//!
//! Memory budget model (from Ahmad's "GPU Memory Math"):
//!   total ≈ weights + KV_cache + activations + framework_overhead + safety_margin
//!   VRAM ≈ parameters × (bits_per_weight ÷ 8)  [then add everything else]
//!   Rule: leave 10-30% headroom. Running at 99% VRAM is begging for OOM.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuClass {
    None,
    /// Integrated / APU (shared system RAM).
    Integrated,
    /// Discrete GPU (dedicated VRAM if known).
    Discrete,
    Unknown,
}

/// Hardware tier based on memory bandwidth.
///
/// From Ahmad's "Memory Bandwidth for Local AI Hardware":
///   - Edge: < 150 GB/s — small models, assistants
///   - ThinAndLight: 150-300 GB/s — thin-and-light AI PCs
///   - Workstation: 450-650 GB/s — serious workstation tier
///   - Server: 800+ GB/s — expensive, powerful
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareTier {
    Edge,
    ThinAndLight,
    Workstation,
    Server,
}

impl HardwareTier {
    pub fn from_bandwidth_gb_s(bandwidth: f64) -> Self {
        match bandwidth {
            b if b < 150.0 => Self::Edge,
            b if b < 300.0 => Self::ThinAndLight,
            b if b < 650.0 => Self::Workstation,
            _ => Self::Server,
        }
    }

    /// Multi-agent workloads need Workstation or Server tier.
    pub fn supports_multi_agent(&self) -> bool {
        matches!(self, Self::Workstation | Self::Server)
    }

    /// Whether this tier is suitable for MoE models (need high bandwidth).
    pub fn supports_moe(&self) -> bool {
        matches!(self, Self::Server)
    }
}

impl std::fmt::Display for HardwareTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Edge => write!(f, "edge"),
            Self::ThinAndLight => write!(f, "thin_and_light"),
            Self::Workstation => write!(f, "workstation"),
            Self::Server => write!(f, "server"),
        }
    }
}

/// Full memory budget for an inference workload.
///
/// From Ahmad: "if you only budget for weights, you're already out of memory."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBudget {
    /// Model weights in bytes (quantized).
    pub weights_bytes: u64,
    /// KV cache in bytes (grows with context).
    pub kv_cache_bytes: u64,
    /// Activations / temporary tensors in bytes.
    pub activations_bytes: u64,
    /// Framework overhead (CUDA, runtime, etc.).
    pub framework_overhead_bytes: u64,
    /// Safety margin (10-30% of subtotal).
    pub safety_margin_bytes: u64,
}

impl MemoryBudget {
    pub fn total(&self) -> u64 {
        self.weights_bytes
            + self.kv_cache_bytes
            + self.activations_bytes
            + self.framework_overhead_bytes
            + self.safety_margin_bytes
    }

    /// Check if this budget fits within available memory.
    pub fn fits_in(&self, available_bytes: u64) -> bool {
        self.total() <= available_bytes
    }

    /// Safety margin ratio (should be 0.10 - 0.30).
    pub fn safety_ratio(&self) -> f64 {
        let subtotal = self.weights_bytes
            + self.kv_cache_bytes
            + self.activations_bytes
            + self.framework_overhead_bytes;
        if subtotal == 0 {
            0.0
        } else {
            self.safety_margin_bytes as f64 / subtotal as f64
        }
    }

    /// Recommended: enforce at least 15% safety margin.
    pub fn has_adequate_safety(&self) -> bool {
        self.safety_ratio() >= 0.15
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub class: GpuClass,
    pub vendor_id: Option<String>,
    pub vendor_name: Option<String>,
    /// Memory bandwidth in GB/s (if known).
    pub bandwidth_gb_s: Option<f64>,
    /// Dedicated VRAM in bytes (if known).
    pub vram_bytes: Option<u64>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub schema: &'static str,
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub cpu_threads: u32,
    pub gpu: GpuInfo,
    /// Conservative GGUF weight budget (bytes) for comfortable load.
    pub recommended_max_weight_bytes: u64,
    /// Soft upper bound (may swap / tight).
    pub absolute_max_weight_bytes: u64,
    /// Human tier label for catalogs.
    pub tier: String,
    /// Structured hardware tier (from bandwidth).
    pub hardware_tier: HardwareTier,
    /// Estimated memory bandwidth in GB/s.
    pub memory_bandwidth_gb_s: Option<f64>,
    pub backends: Vec<String>,
    /// CPUID feature tokens from /proc/cpuinfo (avx2, fma, …).
    pub cpu_flags: Vec<String>,
    /// Current cpufreq governor if available (informational).
    pub cpu_governor: Option<String>,
    /// GTT total when exposed by DRM (APU shared graphics aperture).
    pub gtt_total_bytes: Option<u64>,
}

impl HardwareProfile {
    pub const SCHEMA: &'static str = "cynic.inference.hardware_profile.v1";

    /// Probe the current host (always live — never cached here).
    pub fn probe() -> Self {
        let (total, available) = read_meminfo();
        let cpu_threads = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
        let cpu_flags = read_cpu_flags();
        let cpu_governor = read_cpu_governor();
        let (gpu, gtt_total_bytes) = probe_gpu();

        // Budget: prefer available RAM; leave headroom for OS + other workloads.
        // APU/integrated: weights share system RAM — be stricter.
        let headroom = match gpu.class {
            GpuClass::Integrated => 0.45,
            GpuClass::Discrete => 0.55,
            _ => 0.40,
        };
        let recommended = ((available as f64) * headroom) as u64;
        let absolute = ((available as f64) * 0.75) as u64;

        let avail_gb = available as f64 / (1024.0 * 1024.0 * 1024.0);
        let tier = if avail_gb < 6.0 {
            "tiny"
        } else if avail_gb < 12.0 {
            "small"
        } else if avail_gb < 20.0 {
            "medium"
        } else if avail_gb < 40.0 {
            "large"
        } else {
            "xlarge"
        }
        .to_string();

        let mut backends = vec!["cpu".to_string()];
        if matches!(gpu.class, GpuClass::Integrated | GpuClass::Discrete) {
            backends.push("vulkan".to_string());
        }
        if gpu
            .vendor_name
            .as_deref()
            .is_some_and(|v| v.to_ascii_lowercase().contains("nvidia"))
        {
            backends.push("cuda".to_string());
        }

        Self {
            schema: Self::SCHEMA,
            total_ram_bytes: total,
            available_ram_bytes: available,
            cpu_threads,
            gpu: gpu.clone(),
            recommended_max_weight_bytes: recommended.max(512 * 1024 * 1024),
            absolute_max_weight_bytes: absolute.max(1024 * 1024 * 1024),
            tier,
            hardware_tier: HardwareTier::from_bandwidth_gb_s(gpu.bandwidth_gb_s.unwrap_or(0.0)),
            memory_bandwidth_gb_s: gpu.bandwidth_gb_s,
            backends,
            cpu_flags,
            cpu_governor,
            gtt_total_bytes,
        }
    }

    pub fn fits_recommended(&self, size_bytes: u64) -> bool {
        size_bytes <= self.recommended_max_weight_bytes
    }

    pub fn fits_absolute(&self, size_bytes: u64) -> bool {
        size_bytes <= self.absolute_max_weight_bytes
    }

    /// Feature-class label for ggml CPU plugin selection (not marketing arch).
    pub fn cpu_feature_class(&self) -> &'static str {
        let f = |s: &str| self.cpu_flags.iter().any(|x| x == s);
        // Order: most specific first. Zen3 has avx2 but not avx512.
        if f("avx512f") || f("avx512vnni") {
            "avx512"
        } else if f("avx2") && f("fma") {
            "avx2"
        } else if f("avx") {
            "avx"
        } else {
            "baseline"
        }
    }
}

fn read_meminfo() -> (u64, u64) {
    let Ok(raw) = std::fs::read_to_string("/proc/meminfo") else {
        return (8 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024);
    };
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb(v);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail_kb = parse_kb(v);
        }
    }
    (total_kb * 1024, avail_kb * 1024)
}

fn parse_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn read_cpu_flags() -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string("/proc/cpuinfo") else {
        return Vec::new();
    };
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("flags")
            && let Some(flags) = rest.split(':').nth(1)
        {
            return flags.split_whitespace().map(|s| s.to_string()).collect();
        }
    }
    Vec::new()
}

fn read_cpu_governor() -> Option<String> {
    std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        .ok()
        .map(|s| s.trim().to_string())
}

fn probe_gpu() -> (GpuInfo, Option<u64>) {
    let mut vendor_id = None;
    let mut notes = Vec::new();
    let mut gtt_total = None;
    let mut vram_total = None;

    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }
            let base = e.path().join("device");
            if let Ok(v) = std::fs::read_to_string(base.join("vendor")) {
                vendor_id = Some(v.trim().to_string());
                notes.push(format!("drm:{name} vendor={}", v.trim()));
            }
            if let Ok(v) = std::fs::read_to_string(base.join("mem_info_gtt_total")) {
                if let Ok(n) = v.trim().parse::<u64>() {
                    gtt_total = Some(n);
                    notes.push(format!("gtt_total={n}"));
                }
            }
            if let Ok(v) = std::fs::read_to_string(base.join("mem_info_vram_total")) {
                if let Ok(n) = v.trim().parse::<u64>() {
                    vram_total = Some(n);
                    notes.push(format!("vram_total={n}"));
                }
            }
        }
    }

    let (mut class, vendor_name) = match vendor_id.as_deref() {
        Some("0x1002") | Some("0x1022") => (GpuClass::Integrated, Some("AMD".to_string())),
        Some("0x10de") => (GpuClass::Discrete, Some("NVIDIA".to_string())),
        Some("0x8086") => (GpuClass::Integrated, Some("Intel".to_string())),
        Some(_) => (GpuClass::Unknown, None),
        None => (GpuClass::None, None),
    };

    if let (Some(vram), Some(_gtt)) = (vram_total, gtt_total) {
        if vram >= 6 * 1024 * 1024 * 1024 && vendor_name.as_deref() == Some("AMD") {
            notes.push("large vram aperture observed".into());
        }
    }
    if vendor_name.as_deref() == Some("NVIDIA") {
        class = GpuClass::Discrete;
    }

    // Bandwidth estimation: discrete GPU with VRAM > 8GB ≈ workstation tier
    let bandwidth = match (class.clone(), vram_total) {
        (GpuClass::Discrete, Some(vram)) if vram >= 8 * 1024 * 1024 * 1024 => Some(500.0),
        (GpuClass::Discrete, _) => Some(200.0),
        (GpuClass::Integrated, _) => Some(100.0),
        _ => None,
    };

    (
        GpuInfo {
            class,
            vendor_id,
            vendor_name,
            bandwidth_gb_s: bandwidth,
            vram_bytes: vram_total,
            notes,
        },
        gtt_total,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_positive_ram() {
        let hw = HardwareProfile::probe();
        assert!(hw.total_ram_bytes > 0);
        assert!(!hw.tier.is_empty());
        assert!(!hw.backends.is_empty());
    }

    #[test]
    fn feature_class_from_flags() {
        let mut hw = HardwareProfile::probe();
        hw.cpu_flags = vec!["avx".into(), "avx2".into(), "fma".into()];
        assert_eq!(hw.cpu_feature_class(), "avx2");
    }

    #[test]
    fn bandwidth_tier_from_value() {
        assert_eq!(HardwareTier::from_bandwidth_gb_s(100.0), HardwareTier::Edge);
        assert_eq!(HardwareTier::from_bandwidth_gb_s(200.0), HardwareTier::ThinAndLight);
        assert_eq!(HardwareTier::from_bandwidth_gb_s(500.0), HardwareTier::Workstation);
        assert_eq!(HardwareTier::from_bandwidth_gb_s(800.0), HardwareTier::Server);
    }

    #[test]
    fn edge_tier_does_not_support_multi_agent() {
        assert!(!HardwareTier::Edge.supports_multi_agent());
        assert!(!HardwareTier::ThinAndLight.supports_multi_agent());
    }

    #[test]
    fn workstation_and_server_support_multi_agent() {
        assert!(HardwareTier::Workstation.supports_multi_agent());
        assert!(HardwareTier::Server.supports_multi_agent());
    }

    #[test]
    fn only_server_supports_moe() {
        assert!(!HardwareTier::Edge.supports_moe());
        assert!(!HardwareTier::Workstation.supports_moe());
        assert!(HardwareTier::Server.supports_moe());
    }

    #[test]
    fn memory_budget_total_is_sum_of_parts() {
        let budget = MemoryBudget {
            weights_bytes: 4_000_000_000,
            kv_cache_bytes: 1_000_000_000,
            activations_bytes: 500_000_000,
            framework_overhead_bytes: 500_000_000,
            safety_margin_bytes: 1_500_000_000,
        };
        assert_eq!(budget.total(), 7_500_000_000);
    }

    #[test]
    fn memory_budget_fits_in() {
        let budget = MemoryBudget {
            weights_bytes: 4_000_000_000,
            kv_cache_bytes: 1_000_000_000,
            activations_bytes: 500_000_000,
            framework_overhead_bytes: 500_000_000,
            safety_margin_bytes: 1_000_000_000,
        };
        assert!(budget.fits_in(7_000_000_000));
        assert!(!budget.fits_in(6_999_999_999));
    }

    #[test]
    fn safety_ratio_computes_correctly() {
        let budget = MemoryBudget {
            weights_bytes: 8_000_000_000,
            kv_cache_bytes: 0,
            activations_bytes: 0,
            framework_overhead_bytes: 0,
            safety_margin_bytes: 2_000_000_000,
        };
        assert!((budget.safety_ratio() - 0.25).abs() < 0.01);
    }

    #[test]
    fn inadequate_safety_detected() {
        let budget = MemoryBudget {
            weights_bytes: 9_500_000_000,
            kv_cache_bytes: 0,
            activations_bytes: 0,
            framework_overhead_bytes: 0,
            safety_margin_bytes: 500_000_000,  // ~5% — too low
        };
        assert!(!budget.has_adequate_safety());
    }
}
