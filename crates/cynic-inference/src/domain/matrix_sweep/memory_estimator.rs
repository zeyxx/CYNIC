//! Pure mathematical model for predicting RAM, KV-cache scaling, and context pressure.

use cynic_protocol::{ContextTier, KvQuantization};

/// Architecture parameters extracted from GGUF metadata or model tier heuristics.
#[derive(Debug, Clone, Copy)]
pub struct ModelArchDescriptor {
    pub n_layers: u32,
    pub n_kv_heads: u32,
    pub head_dim: u32,
    pub hidden_dim: u32,
}

impl ModelArchDescriptor {
    /// Heuristics for well-known open-weight model architectures.
    pub fn for_parameter_count(approx_params_b: f32) -> Self {
        if approx_params_b <= 2.0 {
            // e.g. MiniCPM-1B / Qwen-2B
            Self {
                n_layers: 24,
                n_kv_heads: 4,
                head_dim: 64,
                hidden_dim: 1536,
            }
        } else if approx_params_b <= 4.0 {
            // e.g. LFM-2.6B / VibeThinker-3B / Shieldstral-3B
            Self {
                n_layers: 32,
                n_kv_heads: 8,
                head_dim: 128,
                hidden_dim: 2560,
            }
        } else if approx_params_b <= 12.0 {
            // e.g. Qwythos-9B / Qwen-7B/9B
            Self {
                n_layers: 36,
                n_kv_heads: 8,
                head_dim: 128,
                hidden_dim: 4096,
            }
        } else if approx_params_b <= 16.0 {
            // e.g. Qwen-14B
            Self {
                n_layers: 48,
                n_kv_heads: 8,
                head_dim: 128,
                hidden_dim: 5120,
            }
        } else {
            // e.g. Qwen-27B / Qwen-32B / 35B
            Self {
                n_layers: 64,
                n_kv_heads: 8,
                head_dim: 128,
                hidden_dim: 5120,
            }
        }
    }

    /// Calculate KV cache size in bytes for a given context tier, quantization, and parallel slot count.
    pub fn estimate_kv_cache_bytes(
        self,
        context: ContextTier,
        kv_quant: KvQuantization,
        parallel_slots: u32,
    ) -> u64 {
        let n_tokens = context.token_count() as u64;
        let bytes_per_elt = kv_quant.bytes_per_element();
        
        // KV cache = 2 (Key + Value) * n_layers * n_kv_heads * head_dim * context * bytes_per_elt * slots
        let elements_per_token = 2.0 * (self.n_layers as f64) * (self.n_kv_heads as f64) * (self.head_dim as f64);
        let total_bytes = elements_per_token * (n_tokens as f64) * (bytes_per_elt as f64) * (parallel_slots as f64);
        
        total_bytes as u64
    }

    /// Estimate total memory footprint (Weights + KV cache + Compute scratchpad).
    pub fn estimate_total_footprint(
        self,
        weight_bytes: u64,
        context: ContextTier,
        kv_quant: KvQuantization,
        batch_size: u32,
        parallel_slots: u32,
    ) -> u64 {
        let kv_bytes = self.estimate_kv_cache_bytes(context, kv_quant, parallel_slots);
        
        // Scratchpad overhead: proportional to batch_size * hidden_dim
        let scratchpad_bytes = (batch_size as u64) * (self.hidden_dim as u64) * 4 * (self.n_layers as u64 / 4);
        
        weight_bytes + kv_bytes + scratchpad_bytes
    }
}

/// Dynamic memory admission verification against live observed hardware.
pub fn can_admit_model_configuration(
    total_estimated_bytes: u64,
    hardware_available_bytes: u64,
    system_reserve_bytes: u64,
) -> bool {
    if hardware_available_bytes <= system_reserve_bytes {
        return false;
    }
    let allocatable = hardware_available_bytes - system_reserve_bytes;
    total_estimated_bytes <= allocatable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_64k_kv_cache_for_27b_model() {
        let arch = ModelArchDescriptor::for_parameter_count(27.0);
        
        // FP16 KV Cache at 64k
        let kv_fp16 = arch.estimate_kv_cache_bytes(ContextTier::Ctx64k, KvQuantization::Fp16, 1);
        let kv_fp16_gb = kv_fp16 as f64 / (1024.0 * 1024.0 * 1024.0);
        assert!((kv_fp16_gb - 16.0).abs() < 0.5, "FP16 64k KV cache should be ~16 GB, got {kv_fp16_gb:.2}");

        // Q8_0 KV Cache at 64k (50% reduction)
        let kv_q8 = arch.estimate_kv_cache_bytes(ContextTier::Ctx64k, KvQuantization::Q8_0, 1);
        let kv_q8_gb = kv_q8 as f64 / (1024.0 * 1024.0 * 1024.0);
        assert!((kv_q8_gb - 8.0).abs() < 0.5, "Q8 64k KV cache should be ~8 GB, got {kv_q8_gb:.2}");

        // Q4_0 KV Cache at 64k (75% reduction)
        let kv_q4 = arch.estimate_kv_cache_bytes(ContextTier::Ctx64k, KvQuantization::Q4_0, 1);
        let kv_q4_gb = kv_q4 as f64 / (1024.0 * 1024.0 * 1024.0);
        assert!((kv_q4_gb - 4.0).abs() < 0.5, "Q4 64k KV cache should be ~4 GB, got {kv_q4_gb:.2}");
    }

    #[test]
    fn verifies_memory_admission_safety() {
        let total_footprint = 20 * 1024 * 1024 * 1024; // 20 GB
        let available_ram = 27 * 1024 * 1024 * 1024;   // 27 GB
        let system_reserve = 4 * 1024 * 1024 * 1024;   // 4 GB

        assert!(can_admit_model_configuration(total_footprint, available_ram, system_reserve));
        assert!(!can_admit_model_configuration(24 * 1024 * 1024 * 1024, available_ram, system_reserve));
    }
}
