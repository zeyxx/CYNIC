//! Measured-input Roofline analysis for inference decoding.
//!
//! Enforces physical throughput ceilings according to:
//! "Memory Bandwidth for Local AI Hardware (2026 Edition)"
//! by Ahmad Osman (https://x.com/TheAhmadOsman):
//! "Capacity decides what fits. Bandwidth decides how hard the box can breathe.
//! Software stack decides how much you actually cash out."
//!
//! The model produces a physical ceiling only from an observed sustained memory
//! bandwidth and an explicit decode traffic estimate. It never infers bandwidth
//! from a CPU, GPU, or marketing name.

use serde::{Deserialize, Serialize};

/// 2026 Hardware Bandwidth Tiers (Ahmad Osman classification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BandwidthTier {
    /// 1.8 TB/s class: RTX 5090, RTX PRO 6000 Blackwell. Unrestricted multi-agent throughput.
    Tier1800,
    /// 800 GB/s class: Mac Studio M3 Ultra, RTX 4090. Big one-box / serious workstation.
    Tier800,
    /// 500 GB/s class: M4/M5 Max, Radeon PRO W7900, Tenstorrent Blackhole p150.
    Tier500,
    /// 250-300 GB/s unified memory: DGX Spark, Mac mini M4 Pro, Strix Halo / Ryzen AI Max.
    Tier250,
    /// 130-160 GB/s thin-and-light: MacBook Air M5, Intel Lunar Lake, Snapdragon X. Small models.
    Tier150,
    /// Under 100 GB/s: Standard dual-channel DDR4/DDR5 desktop/APU (e.g. Cortex Cezanne/Renoir).
    Sub100LegacyDdr,
}

impl BandwidthTier {
    /// Classify sustained bandwidth in GB/s into normative hardware tier.
    pub fn from_bandwidth_gbps(gbps: f64) -> Self {
        if gbps >= 1400.0 {
            Self::Tier1800
        } else if gbps >= 700.0 {
            Self::Tier800
        } else if gbps >= 400.0 {
            Self::Tier500
        } else if gbps >= 220.0 {
            Self::Tier250
        } else if gbps >= 110.0 {
            Self::Tier150
        } else {
            Self::Sub100LegacyDdr
        }
    }
}

/// Measured quantities required for a reproducible Roofline assessment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RooflineInputs {
    /// Sustained bandwidth measured for the selected execution path, in bytes/s.
    pub sustained_bandwidth_bytes_per_sec: f64,
    /// Floating-point operations required by one decoded token.
    pub decode_flops_per_token: f64,
    /// Bytes transferred by one decoded token, including weights, active KV
    /// reads, and any measured copies.
    pub decode_bytes_per_token: u64,
    /// Observed decode throughput, in tokens/s.
    pub observed_decode_tps: f32,
    /// Optional peak FLOP/s measured on the same execution path.
    pub peak_flops_per_sec: Option<f64>,
}

/// Physical classification derived only when the relevant measurements exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RooflineBottleneck {
    Memory,
    Compute,
}

/// Result of a Roofline calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RooflineAssessment {
    /// Memory-bandwidth ceiling in tokens/s.
    pub memory_bound_tps: f32,
    /// Optional compute ceiling in tokens/s; absent without peak FLOP/s data.
    pub compute_bound_tps: Option<f32>,
    /// The minimum observed ceiling.
    pub theoretical_max_decode_tps: f32,
    /// FLOPs/byte at the decode point.
    pub decode_arithmetic_intensity: f64,
    /// Measured throughput divided by the theoretical ceiling. Values above one
    /// are retained as an inconsistent-observation signal; they are not clamped.
    pub efficiency_ratio: f32,
    /// Bound that selects the minimum ceiling, or `None` without a compute observation.
    pub bottleneck: Option<RooflineBottleneck>,
}

/// Calculates the memory-bandwidth ceiling:
/// `tokens/s = sustained bytes/s / decode bytes/token`.
pub fn calculate_theoretical_max_tps(
    decode_bytes_per_token: u64,
    sustained_bandwidth_bytes_per_sec: f64,
) -> Option<f32> {
    (decode_bytes_per_token > 0
        && sustained_bandwidth_bytes_per_sec.is_finite()
        && sustained_bandwidth_bytes_per_sec > 0.0)
        .then(|| (sustained_bandwidth_bytes_per_sec / decode_bytes_per_token as f64) as f32)
}

/// Derives an assessment from measured input. Missing peak compute data is
/// represented as `None`; it is never substituted with a host-specific guess.
pub fn assess_roofline(input: RooflineInputs) -> Option<RooflineAssessment> {
    if !input.decode_flops_per_token.is_finite() || input.decode_flops_per_token <= 0.0 {
        return None;
    }

    let memory_bound_tps = calculate_theoretical_max_tps(
        input.decode_bytes_per_token,
        input.sustained_bandwidth_bytes_per_sec,
    )?;

    let decode_arithmetic_intensity =
        input.decode_flops_per_token / input.decode_bytes_per_token as f64;
    let compute_bound_tps = input.peak_flops_per_sec.and_then(|peak| {
        (peak.is_finite() && peak > 0.0).then(|| (peak / input.decode_flops_per_token) as f32)
    });

    let (theoretical_max_decode_tps, bottleneck) = match compute_bound_tps {
        Some(compute_bound_tps) if memory_bound_tps <= compute_bound_tps => {
            (memory_bound_tps, Some(RooflineBottleneck::Memory))
        }
        Some(compute_bound_tps) => (compute_bound_tps, Some(RooflineBottleneck::Compute)),
        None => (memory_bound_tps, None),
    };

    Some(RooflineAssessment {
        memory_bound_tps,
        compute_bound_tps,
        theoretical_max_decode_tps,
        decode_arithmetic_intensity,
        efficiency_ratio: input.observed_decode_tps / theoretical_max_decode_tps,
        bottleneck,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_a_measured_memory_ceiling() {
        let assessment = assess_roofline(RooflineInputs {
            sustained_bandwidth_bytes_per_sec: 45_000_000_000.0,
            decode_flops_per_token: 6_000_000_000.0,
            decode_bytes_per_token: 1_800_000_000,
            observed_decode_tps: 20.86,
            peak_flops_per_sec: Some(500_000_000_000.0),
        })
        .expect("valid measured inputs");

        assert!((assessment.memory_bound_tps - 25.0).abs() < 0.01);
        assert_eq!(assessment.bottleneck, Some(RooflineBottleneck::Memory));
        assert!(assessment.efficiency_ratio > 0.83 && assessment.efficiency_ratio < 0.84);
    }

    #[test]
    fn preserves_unknown_compute_ceiling() {
        let assessment = assess_roofline(RooflineInputs {
            sustained_bandwidth_bytes_per_sec: 1_000_000.0,
            decode_flops_per_token: 100.0,
            decode_bytes_per_token: 100,
            observed_decode_tps: 1.0,
            peak_flops_per_sec: None,
        })
        .expect("memory inputs are sufficient");

        assert_eq!(assessment.compute_bound_tps, None);
        assert_eq!(assessment.bottleneck, None);
    }

    #[test]
    fn test_bandwidth_tier_classification() {
        assert_eq!(
            BandwidthTier::from_bandwidth_gbps(1792.0),
            BandwidthTier::Tier1800
        );
        assert_eq!(
            BandwidthTier::from_bandwidth_gbps(819.0),
            BandwidthTier::Tier800
        );
        assert_eq!(
            BandwidthTier::from_bandwidth_gbps(512.0),
            BandwidthTier::Tier500
        );
        assert_eq!(
            BandwidthTier::from_bandwidth_gbps(256.0),
            BandwidthTier::Tier250
        );
        assert_eq!(
            BandwidthTier::from_bandwidth_gbps(136.0),
            BandwidthTier::Tier150
        );
        assert_eq!(
            BandwidthTier::from_bandwidth_gbps(38.0),
            BandwidthTier::Sub100LegacyDdr
        );
    }
}
