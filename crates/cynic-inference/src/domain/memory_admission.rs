//! Sovereign Memory Admission Guard for LLM Inference.
//!
//! Enforces physical memory constraints according to "GPU Memory Math for LLMs (2026 Edition)"
//! by Ahmad Osman (https://x.com/TheAhmadOsman):
//! VRAM/RAM required = Weights + KV Cache + Activations + Safety Margin (20%).
//! On Unified Memory Architecture (UMA / APU), VRAM is shared with host RAM,
//! meaning insufficient host RAM will trigger an OS kernel OOM-Killer.

use cynic_protocol::KvQuantization;
use serde::{Deserialize, Serialize};

/// Detailed memory footprint breakdown for an LLM launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryFootprintEstimate {
    pub weights_bytes: u64,
    pub kv_cache_bytes: u64,
    pub activation_bytes: u64,
    pub safety_margin_bytes: u64,
    pub total_required_bytes: u64,
}

impl MemoryFootprintEstimate {
    /// Compute memory footprint from model size, context size and slots.
    ///
    /// KV Cache formula: 2 * layers * kv_heads * head_dim * context * slots * bytes_per_element
    /// Standard baseline defaults for ~2B-3B models if tensor geometry is not probed:
    /// - 32 layers, 8 kv_heads, 128 head_dim.
    pub fn estimate(
        weights_bytes: u64,
        ctx_size: u32,
        parallel_slots: u32,
        kv_quant: KvQuantization,
    ) -> Self {
        let layers = 32u128;
        let kv_heads = 8u128;
        let head_dim = 128u128;
        let slots = u128::from(parallel_slots.max(1));
        let ctx = u128::from(ctx_size);

        let elements = 2u128 * layers * kv_heads * head_dim * ctx * slots;
        let bytes_per_element = match kv_quant {
            KvQuantization::Fp16 => 2u128,
            KvQuantization::Q8_0 => 1u128,
            KvQuantization::Q4_0 | KvQuantization::Q4_1 => 1u128, // approx half byte + scale
        };

        let kv_cache_bytes = (elements * bytes_per_element) as u64;
        let activation_bytes = 256 * 1024 * 1024; // 256 MB baseline activation buffer

        let subtotal = weights_bytes
            .saturating_add(kv_cache_bytes)
            .saturating_add(activation_bytes);

        // 20% safety margin for runtime framework overhead (llama.cpp allocations, page tables)
        let safety_margin_bytes = subtotal / 5;
        let total_required_bytes = subtotal.saturating_add(safety_margin_bytes);

        Self {
            weights_bytes,
            kv_cache_bytes,
            activation_bytes,
            safety_margin_bytes,
            total_required_bytes,
        }
    }
}

/// Verdict of preflight memory admission check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAdmissionVerdict {
    pub admitted: bool,
    pub available_ram_bytes: u64,
    pub required_bytes: u64,
    pub headroom_bytes: i64,
    pub reason: Option<String>,
}

/// Evaluates whether a model can be safely admitted given physical available RAM.
///
/// Refuses admission if available RAM does not cover total required footprint plus
/// a non-negotiable host OS reserve buffer (e.g. 512 MB for desktop/daemons).
pub fn evaluate_memory_admission(
    available_ram_bytes: u64,
    footprint: &MemoryFootprintEstimate,
    host_os_reserve_bytes: u64,
) -> MemoryAdmissionVerdict {
    let effective_available = available_ram_bytes.saturating_sub(host_os_reserve_bytes);

    if effective_available >= footprint.total_required_bytes {
        let headroom = (effective_available - footprint.total_required_bytes) as i64;
        MemoryAdmissionVerdict {
            admitted: true,
            available_ram_bytes,
            required_bytes: footprint.total_required_bytes,
            headroom_bytes: headroom,
            reason: None,
        }
    } else {
        let deficit = footprint
            .total_required_bytes
            .saturating_sub(effective_available);
        let deficit_mb = deficit / (1024 * 1024);
        let avail_mb = available_ram_bytes / (1024 * 1024);
        let req_mb = footprint.total_required_bytes / (1024 * 1024);
        let weights_mb = footprint.weights_bytes / (1024 * 1024);
        let kv_mb = footprint.kv_cache_bytes / (1024 * 1024);

        let reason = format!(
            "Insufficient physical memory (OOM risk): available {} MB < required {} MB \
             (Weights: {} MB, KV Cache: {} MB, Deficit: {} MB). Refusing admission to protect host OS.",
            avail_mb, req_mb, weights_mb, kv_mb, deficit_mb
        );

        MemoryAdmissionVerdict {
            admitted: false,
            available_ram_bytes,
            required_bytes: footprint.total_required_bytes,
            headroom_bytes: -(deficit as i64),
            reason: Some(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_footprint_calculation() {
        let weights = 1_600 * 1024 * 1024; // 1.6 GB
        let footprint = MemoryFootprintEstimate::estimate(weights, 8192, 4, KvQuantization::Fp16);

        assert_eq!(footprint.weights_bytes, weights);
        assert!(footprint.kv_cache_bytes > 0);
        assert!(footprint.total_required_bytes > weights);
        // Subtotal + 20% margin
        let subtotal = weights + footprint.kv_cache_bytes + footprint.activation_bytes;
        assert_eq!(footprint.total_required_bytes, subtotal + subtotal / 5);
    }

    #[test]
    fn test_admission_granted_when_enough_ram() {
        let weights = 1_600 * 1024 * 1024;
        let footprint = MemoryFootprintEstimate::estimate(weights, 4096, 1, KvQuantization::Fp16);

        let available = footprint.total_required_bytes + 1_000 * 1024 * 1024;
        let verdict = evaluate_memory_admission(available, &footprint, 512 * 1024 * 1024);

        assert!(verdict.admitted);
        assert!(verdict.headroom_bytes > 0);
        assert!(verdict.reason.is_none());
    }

    #[test]
    fn test_admission_rejected_when_insufficient_ram() {
        let weights = 1_600 * 1024 * 1024;
        let footprint = MemoryFootprintEstimate::estimate(weights, 8192, 4, KvQuantization::Fp16);

        // Only 1.1 GB available (the real situation observed on cortex!)
        let available = 1_100 * 1024 * 1024;
        let verdict = evaluate_memory_admission(available, &footprint, 512 * 1024 * 1024);

        assert!(!verdict.admitted);
        assert!(verdict.headroom_bytes < 0);
        assert!(verdict.reason.is_some());
        assert!(verdict.reason.unwrap().contains("OOM risk"));
    }
}
