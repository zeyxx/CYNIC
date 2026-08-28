//! runtime_admission.rs — Real-Time Memory Admission & Hardware Preflight Gate.
//!
//! Invariant: No inference workload or benchmark may execute without a real-time observation
//! of live hardware state (/proc/meminfo MemAvailable and GPU GTT pool).
//!
//! If `weights + kv_cache(ctx, quant, slots) + activations + headroom (20%) > MemAvailable`,
//! the admission gate immediately returns `AdmissionDecision::Blocked { ... }`
//! preventing Linux OOM killer invocations and swap degradation before any process spawn.

use cynic_protocol::{ContextTier, KvQuantization};
use serde::{Deserialize, Serialize};

use crate::domain::audit_geometry::{ModelMatrixCoordinate, OffloadStrategy};
use crate::domain::hardware_profile::{HardwareProfile, MemoryBudget};
use crate::math::tensor_geometry::ExactTensorGeometry;

pub const ADMISSION_REPORT_SCHEMA: &str = "cynic.inference.admission_decision.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionStatus {
    Admitted,
    BlockedOomRisk,
    BlockedGttApertureExceeded,
    BlockedUnsupportedArchitecture,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmissionDecision {
    pub schema: &'static str,
    pub status: AdmissionStatus,
    pub is_admitted: bool,
    pub model_alias: String,
    pub required_memory_bytes: u64,
    pub available_ram_bytes: u64,
    pub memory_budget: MemoryBudget,
    pub reasons: Vec<String>,
    pub suggested_remediation: Option<String>,
}

impl AdmissionDecision {
    pub const SCHEMA: &'static str = ADMISSION_REPORT_SCHEMA;
}

/// Evaluates real-time admission of a model configuration against live hardware state.
pub fn admit_inference_workload(
    coordinate: &ModelMatrixCoordinate,
    geometry: &ExactTensorGeometry,
    hardware: &HardwareProfile,
    model_weight_bytes: u64,
) -> AdmissionDecision {
    let kv_bytes = geometry
        .calculate_kv_cache_bytes(
            coordinate.context_tier.token_count(),
            coordinate.kv_quant,
            coordinate.parallel_slots,
        )
        .unwrap_or(4 * 1024 * 1024 * 1024);

    let activation_bytes = u64::from(coordinate.parallel_slots) * 512 * 1024 * 1024; // 512MB per slot
    let runtime_overhead_bytes = 512 * 1024 * 1024; // 512MB llama-server / OS buffer

    let budget = MemoryBudget::calculate(
        model_weight_bytes,
        kv_bytes,
        activation_bytes,
        runtime_overhead_bytes,
        0.20, // strict 20% safety headroom
        hardware.available_ram_bytes,
    );

    let mut reasons = Vec::new();
    let mut status = AdmissionStatus::Admitted;
    let mut remediation = None;

    // 1. Check RAM physical capacity
    if !budget.fits {
        status = AdmissionStatus::BlockedOomRisk;
        let req_gb = budget.total_required_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let avail_gb = budget.available_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        reasons.push(format!(
            "Total required memory ({req_gb:.2} GB) exceeds available RAM ({avail_gb:.2} GB)"
        ));
        remediation = Some(
            "Reduce context tier, quantize KV cache to q4_0, or downscale model to a lighter quantization."
                .to_string(),
        );
    }

    // 2. Check GTT aperture for full Vulkan offload
    if let OffloadStrategy::FullVulkan { .. } = &coordinate.offload_strategy {
        if let Some(gtt_bytes) = hardware.gtt_total_bytes {
            let vulkan_required = model_weight_bytes.saturating_add(kv_bytes);
            if vulkan_required > gtt_bytes {
                status = AdmissionStatus::BlockedGttApertureExceeded;
                let v_req_gb = vulkan_required as f64 / (1024.0 * 1024.0 * 1024.0);
                let gtt_gb = gtt_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                reasons.push(format!(
                    "Vulkan allocated footprint ({v_req_gb:.2} GB) exceeds GTT aperture limit ({gtt_gb:.2} GB)"
                ));
                remediation = Some(
                    "Switch from Full Vulkan to Hybrid offload (ngl 16 / 24) to keep expert layers in CPU RAM."
                        .to_string(),
                );
            }
        }
    }

    let is_admitted = matches!(status, AdmissionStatus::Admitted);

    AdmissionDecision {
        schema: AdmissionDecision::SCHEMA,
        status,
        is_admitted,
        model_alias: coordinate.model_alias.clone(),
        required_memory_bytes: budget.total_required_bytes,
        available_ram_bytes: budget.available_ram_bytes,
        memory_budget: budget,
        reasons,
        suggested_remediation: remediation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::hardware_profile::GpuClass;
    use crate::math::tensor_geometry::AttentionType;

    fn sample_hardware(avail_gb: u64, gtt_gb: u64) -> HardwareProfile {
        HardwareProfile {
            schema: HardwareProfile::SCHEMA,
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            available_ram_bytes: avail_gb * 1024 * 1024 * 1024,
            cpu_threads: 16,
            gpu: crate::domain::hardware_profile::GpuInfo {
                class: GpuClass::Integrated,
                vendor_id: Some("0x1002".to_string()),
                vendor_name: Some("AMD".to_string()),
                notes: vec![],
            },
            recommended_max_weight_bytes: 10 * 1024 * 1024 * 1024,
            absolute_max_weight_bytes: 16 * 1024 * 1024 * 1024,
            tier: "large".to_string(),
            backends: vec!["vulkan".to_string()],
            cpu_flags: vec!["avx2".to_string()],
            cpu_governor: Some("powersave".to_string()),
            gtt_total_bytes: Some(gtt_gb * 1024 * 1024 * 1024),
            unified_gpu_memory_bytes: Some(gtt_gb * 1024 * 1024 * 1024),
            memory_bandwidth_bytes_per_sec_estimate: Some(40_000_000_000),
        }
    }

    fn sample_geometry() -> ExactTensorGeometry {
        ExactTensorGeometry {
            n_layers: 64,
            n_heads: 64,
            n_kv_heads: 8,
            head_dim: 128,
            embedding_dim: 5120,
            vocab_size: 152064,
            attention_type: Some(AttentionType::Gqa),
        }
    }

    #[test]
    fn test_admit_workload_admits_fitting_configuration() {
        let hw = sample_hardware(24, 19); // 24 GB RAM, 19 GB GTT
        let geom = sample_geometry();
        let coord = ModelMatrixCoordinate {
            model_alias: "qwen36-35b-iq2".to_string(),
            architecture: "qwen35moe".to_string(),
            total_params_b: 35.0,
            active_params_b: 3.0,
            weight_quant: "IQ2_XXS".to_string(),
            kv_quant: KvQuantization::Q4_0,
            context_tier: ContextTier::Ctx32k,
            offload_strategy: OffloadStrategy::FullVulkan {
                device: "Vulkan0".to_string(),
                threads: 8,
            },
            parallel_slots: 1,
            attention_type: AttentionType::Gqa,
        };

        let decision = admit_inference_workload(&coord, &geom, &hw, 10 * 1024 * 1024 * 1024);
        assert!(decision.is_admitted);
        assert_eq!(decision.status, AdmissionStatus::Admitted);
    }

    #[test]
    fn test_admit_workload_blocks_oom_oversized_configuration() {
        let hw = sample_hardware(12, 19); // only 12 GB RAM available
        let geom = sample_geometry();
        let coord = ModelMatrixCoordinate {
            model_alias: "oversized-model".to_string(),
            architecture: "llama".to_string(),
            total_params_b: 70.0,
            active_params_b: 70.0,
            weight_quant: "Q4_K_M".to_string(),
            kv_quant: KvQuantization::Fp16,
            context_tier: ContextTier::Ctx128k,
            offload_strategy: OffloadStrategy::CpuOnly { threads: 16 },
            parallel_slots: 1,
            attention_type: AttentionType::Gqa,
        };

        // 40 GB weights
        let decision = admit_inference_workload(&coord, &geom, &hw, 40 * 1024 * 1024 * 1024);
        assert!(!decision.is_admitted);
        assert_eq!(decision.status, AdmissionStatus::BlockedOomRisk);
        assert!(decision.suggested_remediation.is_some());
    }

    #[test]
    fn test_admit_workload_blocks_gtt_aperture_overflow() {
        let hw = sample_hardware(30, 8); // 30 GB RAM, but only 8 GB GTT
        let geom = sample_geometry();
        let coord = ModelMatrixCoordinate {
            model_alias: "vulkan-overflow".to_string(),
            architecture: "qwen2".to_string(),
            total_params_b: 9.0,
            active_params_b: 9.0,
            weight_quant: "Q4_K_M".to_string(),
            kv_quant: KvQuantization::Q8_0,
            context_tier: ContextTier::Ctx64k,
            offload_strategy: OffloadStrategy::FullVulkan {
                device: "Vulkan0".to_string(),
                threads: 8,
            },
            parallel_slots: 1,
            attention_type: AttentionType::Gqa,
        };

        // 6 GB weights + 5 GB KV = 11 GB > 8 GB GTT
        let decision = admit_inference_workload(&coord, &geom, &hw, 6 * 1024 * 1024 * 1024);
        assert!(!decision.is_admitted);
        assert_eq!(decision.status, AdmissionStatus::BlockedGttApertureExceeded);
        assert!(decision.reasons[0].contains("GTT aperture limit"));
    }
}
