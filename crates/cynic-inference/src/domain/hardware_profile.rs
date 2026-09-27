//! Host hardware probe for model sizing and launch planning.
//!
//! Pure observation of /proc + DRM + optional plugin dirs — no vendor SDK.
//! Agnostic: no frozen machine names; all decisions derive from probed fields.

use serde::{Deserialize, Serialize};

/// Coarse hardware capability tier used by engine selection policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareTier {
    Edge,
    ThinAndLight,
    Workstation,
    Server,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub class: GpuClass,
    pub vendor_id: Option<String>,
    pub vendor_name: Option<String>,
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
    pub backends: Vec<String>,
    /// CPUID feature tokens from /proc/cpuinfo (avx2, fma, …).
    pub cpu_flags: Vec<String>,
    /// Current cpufreq governor if available (informational).
    pub cpu_governor: Option<String>,
    /// GTT total when exposed by DRM (APU shared graphics aperture).
    pub gtt_total_bytes: Option<u64>,
    /// Unified GPU memory budget for weight offload (APU shared aperture).
    ///
    /// On APU / integrated GPUs the "VRAM" exposed by `mem_info_vram_total` is
    /// only a small carve-out; the real graphics memory is the GTT (GART
    /// Translation Table) which lives in the same physical RAM pool as the
    /// CPU. Vulkan allocates from the GTT, so it is legitimate VRAM for
    /// `llama.cpp` `-ngl` offload. We surface it as the offload budget so the
    /// launch planner does not wrongly fall back to CPU-only on APUs.
    pub unified_gpu_memory_bytes: Option<u64>,
    /// Best-effort sustained memory bandwidth estimate (bytes/s) used by the
    /// roofline decode ceiling.
    ///
    /// NOT auto-derived from a marketing name or frozen host value (per
    /// AGENTS.md: no hardcoding of machine specifics). It is read from the
    /// operator-supplied `CYNIC_ESTIMATED_MEM_BW_GBPS` override when present,
    /// otherwise `None` — the planner then omits the roofline warning rather
    /// than guessing a number. A future discovery path (e.g. dmidecode /
    /// libmemclk) may populate this; until then the operator is the source.
    pub memory_bandwidth_bytes_per_sec_estimate: Option<u64>,
}

impl HardwareProfile {
    pub const SCHEMA: &'static str = "cynic.inference.hardware_profile.v1";

    /// Assemble a profile from already-observed inputs (pure — no I/O, no env).
    ///
    /// All host reads (meminfo, cpuinfo, DRM sysfs, thread count, bandwidth
    /// override) are performed by the `HardwareProbePort` adapter; this
    /// function only decides budgets, tiers, and backends from data.
    #[allow(clippy::too_many_arguments)]
    pub fn from_observed(
        total: u64,
        available: u64,
        cpu_threads: u32,
        cpu_flags: Vec<String>,
        cpu_governor: Option<String>,
        gpu: GpuInfo,
        gtt_total_bytes: Option<u64>,
        mem_bw_gbps: Option<f64>,
    ) -> Self {

        // Unified GPU memory (GTT on APU) is legitimate VRAM for offload.
        // It lives in the same physical pool as the CPU but is allocated by
        // Vulkan for `-ngl`, so it must not be double-counted as CPU RAM.
        let unified_gpu_memory_bytes = if gpu.class == GpuClass::Integrated {
            gtt_total_bytes
        } else {
            None
        };

        // Budget: prefer available RAM; leave headroom for OS + other workloads.
        // APU/integrated: weights share system RAM — be stricter on CPU RAM.
        let headroom = match gpu.class {
            GpuClass::Integrated => 0.45,
            GpuClass::Discrete => 0.55,
            _ => 0.40,
        };
        let recommended = ((available as f64) * headroom) as u64;
        let absolute = ((available as f64) * 0.75) as u64;

        // GPU offload budget: how much weight `-ngl 99` may place on the
        // device. On APU this is the unified GTT aperture (not the tiny
        // "dedicated" VRAM carve-out). On discrete we currently have no
        // reliable VRAM probe, so it stays 0 and the planner keeps weights on
        // CPU RAM. On APU the GTT is a separate graphics pool, so a model that
        // fits there does not compete with the CPU-RAM budget above.
        let gpu_offload_budget = unified_gpu_memory_bytes
            .map(|g| ((g as f64) * 0.90) as u64)
            .unwrap_or(0);
        // For weight-sizing, the comfortable / hard ceilings are the larger of
        // the CPU-RAM headroom and the GPU offload budget (APU can host the
        // model entirely in graphics memory).
        let recommended_max = recommended.max(gpu_offload_budget).max(512 * 1024 * 1024);
        let absolute_max = absolute.max(gpu_offload_budget).max(1024 * 1024 * 1024);

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

        // Roofline decode ceiling needs a sustained memory bandwidth, supplied
        // by the operator through the probe adapter (never guessed here).
        let memory_bandwidth_bytes_per_sec_estimate = mem_bw_gbps
            .filter(|gbps| gbps.is_finite() && *gbps > 0.0)
            .map(|gbps| (gbps * 1_000_000_000.0) as u64);

        Self {
            schema: Self::SCHEMA,
            total_ram_bytes: total,
            available_ram_bytes: available,
            cpu_threads,
            gpu,
            recommended_max_weight_bytes: recommended_max,
            absolute_max_weight_bytes: absolute_max,
            tier,
            backends,
            cpu_flags,
            cpu_governor,
            gtt_total_bytes,
            unified_gpu_memory_bytes,
            memory_bandwidth_bytes_per_sec_estimate,
        }
    }

    /// Bytes of model weights that can be fully offloaded to the GPU via
    /// `-ngl 99` without spilling to CPU RAM.
    ///
    /// On APU / integrated GPUs this is the unified GTT aperture (≈90% of it),
    /// because Vulkan allocates weights there. On discrete GPUs we currently
    /// have no reliable VRAM probe, so we return 0 and let the planner fall
    /// back to partial offload / CPU. Callers must not assume this equals
    /// free VRAM — it is a conservative *weight* budget.
    pub fn gpu_offload_budget_bytes(&self) -> u64 {
        self.unified_gpu_memory_bytes
            .map(|g| ((g as f64) * 0.90) as u64)
            .unwrap_or(0)
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

/// One DRM card's pre-read sysfs values (collected by the probe adapter).
#[derive(Debug, Clone, Default)]
pub struct DrmCardReadings {
    pub card_name: String,
    pub vendor: Option<String>,
    pub gtt_total: Option<u64>,
    pub vram_total: Option<u64>,
}

/// Pure classifier over pre-read DRM card values.
pub fn classify_gpu(cards: &[DrmCardReadings]) -> (GpuInfo, Option<u64>) {
    let mut vendor_id = None;
    let mut notes = Vec::new();
    let mut gtt_total = None;
    let mut vram_total = None;

    for card in cards {
        if !card.card_name.starts_with("card") || card.card_name.contains('-') {
            continue;
        }
        if let Some(v) = &card.vendor {
            vendor_id = Some(v.clone());
            notes.push(format!("drm:{} vendor={v}", card.card_name));
        }
        if let Some(n) = card.gtt_total {
            gtt_total = Some(n);
            notes.push(format!("gtt_total={n}"));
        }
        if let Some(n) = card.vram_total {
            vram_total = Some(n);
            notes.push(format!("vram_total={n}"));
        }
    }

    let (mut class, vendor_name) = match vendor_id.as_deref() {
        Some("0x1002") | Some("0x1022") => (GpuClass::Integrated, Some("AMD".to_string())),
        Some("0x10de") => (GpuClass::Discrete, Some("NVIDIA".to_string())),
        Some("0x8086") => (GpuClass::Integrated, Some("Intel".to_string())),
        Some(_) => (GpuClass::Unknown, None),
        None => (GpuClass::None, None),
    };

    // Heuristic: large discrete VRAM vs shared GTT
    if let (Some(vram), Some(gtt)) = (vram_total, gtt_total) {
        if vram > 2 * 1024 * 1024 * 1024 && vram < gtt {
            // common APU: vram aperture small/equal and gtt large
        }
        if vram >= 6 * 1024 * 1024 * 1024 && vendor_name.as_deref() == Some("AMD") {
            // could still be APU with large aperture; keep integrated unless PCI discrete
            notes.push("large vram aperture observed".into());
        }
    }
    if vendor_name.as_deref() == Some("NVIDIA") {
        class = GpuClass::Discrete;
    }

    (
        GpuInfo {
            class,
            vendor_id,
            vendor_name,
            notes,
        },
        gtt_total,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_hw() -> HardwareProfile {
        HardwareProfile::from_observed(
            32 * 1024 * 1024 * 1024,
            24 * 1024 * 1024 * 1024,
            16,
            vec!["avx".into(), "avx2".into(), "fma".into()],
            Some("schedutil".into()),
            GpuInfo {
                class: GpuClass::Integrated,
                vendor_id: Some("0x1002".into()),
                vendor_name: Some("AMD".into()),
                notes: Vec::new(),
            },
            Some(8 * 1024 * 1024 * 1024),
            None,
        )
    }

    #[test]
    fn assembled_profile_has_positive_ram() {
        let hw = synthetic_hw();
        assert!(hw.total_ram_bytes > 0);
        assert!(!hw.tier.is_empty());
        assert!(!hw.backends.is_empty());
    }

    #[test]
    fn feature_class_from_flags() {
        let hw = synthetic_hw();
        assert_eq!(hw.cpu_feature_class(), "avx2");
    }

    #[test]
    fn meminfo_parser_reads_kb_lines() {
        use cynic_types::proc_text::parse_meminfo_bytes;

        let raw = "MemTotal:       16384000 kB\nMemAvailable:    8192000 kB\n";
        let (total, avail) = parse_meminfo_bytes(raw);
        assert_eq!(total, 16384000 * 1024);
        assert_eq!(avail, 8192000 * 1024);
    }

    #[test]
    fn classifier_marks_amd_apu_integrated() {
        let cards = vec![DrmCardReadings {
            card_name: "card0".into(),
            vendor: Some("0x1002".into()),
            gtt_total: Some(8 * 1024 * 1024 * 1024),
            vram_total: Some(512 * 1024 * 1024),
        }];
        let (gpu, gtt) = classify_gpu(&cards);
        assert_eq!(gpu.class, GpuClass::Integrated);
        assert_eq!(gtt, Some(8 * 1024 * 1024 * 1024));
    }
}
