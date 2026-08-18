//! Exact KV-cache tensor sizing from observed model dimensions.
//!
//! This module contains no admission or quality threshold. It reports capacity
//! requirements; callers combine those measurements with explicit operator
//! constraints and measured process RSS.

use cynic_protocol::{ContextTier, KvQuantization};
use serde::{Deserialize, Serialize};

/// Tensor dimensions extracted from GGUF metadata or model architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactTensorGeometry {
    pub n_layers: u32,
    pub n_heads: u32,
    pub n_kv_heads: u32,
    pub head_dim: u32,
    pub embedding_dim: u32,
    pub vocab_size: u32,
}

impl ExactTensorGeometry {
    /// Returns KV storage bytes using the GGML block layouts of `kv_quant`.
    ///
    /// `2 * layers * kv_heads * head_dim * context * slots * bytes/element`.
    /// Returns `None` on a zero-sized shape or integer overflow rather than
    /// fabricating a capacity estimate.
    pub fn calculate_kv_cache_bytes(
        &self,
        context_length: u32,
        kv_quant: KvQuantization,
        parallel_slots: u32,
    ) -> Option<u64> {
        let bytes_per_block = match kv_quant {
            KvQuantization::Fp16 => 64_u128,
            KvQuantization::Q8_0 => 34,
            KvQuantization::Q4_0 => 18,
            KvQuantization::Q4_1 => 20,
        };
        let elements_per_block = match kv_quant {
            KvQuantization::Fp16 => 32_u128,
            _ => 32,
        };

        let elements = u128::from(2_u32)
            .checked_mul(u128::from(self.n_layers))?
            .checked_mul(u128::from(self.n_kv_heads))?
            .checked_mul(u128::from(self.head_dim))?
            .checked_mul(u128::from(context_length))?
            .checked_mul(u128::from(parallel_slots))?;
        if elements == 0 {
            return None;
        }
        let blocks = elements.div_ceil(elements_per_block);
        u64::try_from(blocks.checked_mul(bytes_per_block)?).ok()
    }

    /// Returns the known execution allocations: model weights, KV storage, and
    /// an explicitly documented activation lower bound. Runtime RSS remains the
    /// authoritative capacity observation.
    pub fn calculate_total_execution_footprint(
        &self,
        weight_bytes: u64,
        context_tier: ContextTier,
        kv_quant: KvQuantization,
        batch_size: u32,
        parallel_slots: u32,
    ) -> Option<u64> {
        let kv_bytes =
            self.calculate_kv_cache_bytes(context_tier.token_count(), kv_quant, parallel_slots)?;
        let activation_bytes = u64::from(batch_size)
            .checked_mul(u64::from(self.embedding_dim))?
            .checked_mul(8)?;

        weight_bytes
            .checked_add(kv_bytes)?
            .checked_add(activation_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qwen_geometry() -> ExactTensorGeometry {
        ExactTensorGeometry {
            n_layers: 64,
            n_heads: 64,
            n_kv_heads: 8,
            head_dim: 128,
            embedding_dim: 5120,
            vocab_size: 152064,
        }
    }

    #[test]
    fn quantized_kv_storage_uses_ggml_block_sizes() {
        let geometry = qwen_geometry();
        let fp16 = geometry
            .calculate_kv_cache_bytes(65_536, KvQuantization::Fp16, 1)
            .expect("non-zero dimensions");
        let q4 = geometry
            .calculate_kv_cache_bytes(65_536, KvQuantization::Q4_0, 1)
            .expect("non-zero dimensions");

        assert_eq!(fp16, 17_179_869_184);
        assert_eq!(q4, 4_831_838_208);
        assert_eq!(fp16 * 9 / 32, q4);
    }

    #[test]
    fn zero_sized_shape_is_unknown() {
        assert_eq!(
            qwen_geometry().calculate_kv_cache_bytes(0, KvQuantization::Fp16, 1),
            None
        );
    }
}
