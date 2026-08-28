//! Exact KV-cache tensor sizing from observed model dimensions.
//!
//! This module contains no admission or quality threshold. It reports capacity
//! requirements; callers combine those measurements with explicit operator
//! constraints and measured process RSS.

use cynic_protocol::{ContextTier, KvQuantization};
use serde::{Deserialize, Serialize};

/// Attention architecture type affecting KV-cache scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionType {
    /// Multi-Head Attention: Q heads == KV heads (full uncompressed KV cache).
    Mha,
    /// Grouped-Query Attention: Q heads > KV heads (e.g. 8x compression).
    Gqa,
    /// Multi-Query Attention: single KV head shared across all Q heads.
    Mqa,
    /// Multi-Head Latent Attention (DeepSeek / MLA): compressed latent vector + decoupled RoPE keys.
    Mla {
        kv_lora_rank: u32,
        qk_rope_head_dim: u32,
    },
}

/// Tensor dimensions extracted from GGUF metadata or model architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactTensorGeometry {
    pub n_layers: u32,
    pub n_heads: u32,
    pub n_kv_heads: u32,
    pub head_dim: u32,
    pub embedding_dim: u32,
    pub vocab_size: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention_type: Option<AttentionType>,
}

impl ExactTensorGeometry {
    /// Returns the active or inferred attention architecture.
    pub fn resolved_attention_type(&self) -> AttentionType {
        if let Some(explicit) = self.attention_type {
            return explicit;
        }
        if self.n_kv_heads == 1 && self.n_heads > 1 {
            AttentionType::Mqa
        } else if self.n_kv_heads > 1 && self.n_kv_heads < self.n_heads {
            AttentionType::Gqa
        } else {
            AttentionType::Mha
        }
    }

    /// Returns KV storage bytes using the GGML block layouts of `kv_quant`.
    ///
    /// For standard MHA/GQA/MQA:
    /// `2 * layers * kv_heads * head_dim * context * slots * bytes/element`.
    ///
    /// For MLA:
    /// `layers * (kv_lora_rank + qk_rope_head_dim) * context * slots * bytes/element`.
    ///
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

        let elements = match self.resolved_attention_type() {
            AttentionType::Mla {
                kv_lora_rank,
                qk_rope_head_dim,
            } => {
                let latent_dim = u128::from(kv_lora_rank.checked_add(qk_rope_head_dim)?);
                u128::from(self.n_layers)
                    .checked_mul(latent_dim)?
                    .checked_mul(u128::from(context_length))?
                    .checked_mul(u128::from(parallel_slots))?
            }
            AttentionType::Mha | AttentionType::Gqa | AttentionType::Mqa => {
                u128::from(2_u32)
                    .checked_mul(u128::from(self.n_layers))?
                    .checked_mul(u128::from(self.n_kv_heads))?
                    .checked_mul(u128::from(self.head_dim))?
                    .checked_mul(u128::from(context_length))?
                    .checked_mul(u128::from(parallel_slots))?
            }
        };

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
            attention_type: None,
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

    #[test]
    fn attention_type_inference_and_mla_kv_calculation() {
        let mut g = qwen_geometry();
        assert_eq!(g.resolved_attention_type(), AttentionType::Gqa);

        g.n_kv_heads = 64;
        assert_eq!(g.resolved_attention_type(), AttentionType::Mha);

        g.n_kv_heads = 1;
        assert_eq!(g.resolved_attention_type(), AttentionType::Mqa);

        let mla_geom = ExactTensorGeometry {
            n_layers: 60,
            n_heads: 128,
            n_kv_heads: 128,
            head_dim: 128,
            embedding_dim: 7168,
            vocab_size: 102400,
            attention_type: Some(AttentionType::Mla {
                kv_lora_rank: 512,
                qk_rope_head_dim: 64,
            }),
        };
        assert!(matches!(
            mla_geom.resolved_attention_type(),
            AttentionType::Mla { .. }
        ));

        let mla_q4 = mla_geom
            .calculate_kv_cache_bytes(32_768, KvQuantization::Q4_0, 1)
            .expect("valid mla calculation");
        assert!(mla_q4 > 0);
    }
}
