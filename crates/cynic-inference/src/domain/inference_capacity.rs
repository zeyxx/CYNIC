//! Exact GGUF-derived KV capacity and conservative host admission.
//!
//! This module owns the pure capacity decision. It does not start a runtime,
//! select an engine, or claim measured RSS: those remain adapter/read-back work.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::gguf_meta::{GgufKvGeometry, GgufMeta};
use super::hardware_profile::HardwareProfile;

const GGML_BLOCK_ELEMENTS: u64 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KvCacheType {
    F16,
    Q8_0,
    Q4_0,
    Q4_1,
}

impl KvCacheType {
    fn bytes_per_block(self) -> u64 {
        match self {
            Self::F16 => 64,
            Self::Q8_0 => 34,
            Self::Q4_0 => 18,
            Self::Q4_1 => 20,
        }
    }
}

impl FromStr for KvCacheType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "f16" => Ok(Self::F16),
            "q8_0" | "q8" => Ok(Self::Q8_0),
            "q4_0" | "q4" => Ok(Self::Q4_0),
            "q4_1" => Ok(Self::Q4_1),
            other => Err(format!(
                "unsupported kv cache type '{other}', expected f16, q8_0, q4_0, or q4_1"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvCapacityRequest {
    pub context_tokens: u64,
    pub parallel_slots: u32,
    pub key_cache_type: KvCacheType,
    pub value_cache_type: KvCacheType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvCapacityBreakdown {
    pub key_elements: u64,
    pub value_elements: u64,
    pub key_bytes: u64,
    pub value_bytes: u64,
    pub total_bytes: u64,
    pub formula: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CapacityDecision {
    Admitted,
    Blocked { reasons: Vec<String> },
    Unknown { reasons: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceCapacityReport {
    pub schema: &'static str,
    pub model_path: String,
    pub model_size_bytes: u64,
    pub geometry: Option<GgufKvGeometry>,
    pub request: KvCapacityRequest,
    pub kv_cache: Option<KvCapacityBreakdown>,
    pub observed_available_ram_bytes: u64,
    pub system_reserve_bytes: u64,
    pub conservative_known_footprint_bytes: Option<u64>,
    pub allocatable_bytes: Option<u64>,
    pub decision: CapacityDecision,
    pub residual_unknowns: Vec<String>,
}

impl InferenceCapacityReport {
    pub const SCHEMA: &'static str = "cynic.inference.capacity.v1";
}

pub fn assess_capacity(
    meta: &GgufMeta,
    model_size_bytes: u64,
    hardware: &HardwareProfile,
    request: KvCapacityRequest,
) -> InferenceCapacityReport {
    let system_reserve_bytes = hardware.total_ram_bytes / 10;
    let geometry = meta.kv_geometry().ok();
    let mut residual_unknowns = vec![
        "engine allocator, scratch workspace, mmap/page-cache residency, and post-launch RSS are not included in known footprint".to_string(),
        "GPU/GTT placement is shared with host RAM and is not additional allocatable capacity".to_string(),
    ];

    let Some(geometry) = geometry else {
        return InferenceCapacityReport {
            schema: InferenceCapacityReport::SCHEMA,
            model_path: meta.path.clone(),
            model_size_bytes,
            geometry: None,
            request,
            kv_cache: None,
            observed_available_ram_bytes: hardware.available_ram_bytes,
            system_reserve_bytes,
            conservative_known_footprint_bytes: None,
            allocatable_bytes: None,
            decision: CapacityDecision::Unknown {
                reasons: vec![
                    "GGUF lacks the architecture-specific K/V geometry required for exact sizing"
                        .to_string(),
                ],
            },
            residual_unknowns,
        };
    };

    let breakdown = match exact_kv_bytes(&geometry, &request) {
        Some(value) => value,
        None => {
            return InferenceCapacityReport {
                schema: InferenceCapacityReport::SCHEMA,
                model_path: meta.path.clone(),
                model_size_bytes,
                geometry: Some(geometry),
                request,
                kv_cache: None,
                observed_available_ram_bytes: hardware.available_ram_bytes,
                system_reserve_bytes,
                conservative_known_footprint_bytes: None,
                allocatable_bytes: None,
                decision: CapacityDecision::Unknown {
                    reasons: vec![
                        "KV capacity arithmetic overflowed or received a zero dimension"
                            .to_string(),
                    ],
                },
                residual_unknowns,
            };
        }
    };

    let mut reasons = Vec::new();
    if request.context_tokens > geometry.native_context_tokens {
        reasons.push(format!(
            "requested context {} exceeds GGUF native context {}",
            request.context_tokens, geometry.native_context_tokens
        ));
    }
    let known_footprint = model_size_bytes.saturating_add(breakdown.total_bytes);
    let allocatable = hardware
        .available_ram_bytes
        .saturating_sub(system_reserve_bytes);
    if known_footprint > allocatable {
        reasons.push(format!(
            "known weights plus KV footprint {} bytes exceeds allocatable {} bytes after {} byte system reserve",
            known_footprint, allocatable, system_reserve_bytes
        ));
    }
    residual_unknowns.sort();
    residual_unknowns.dedup();

    InferenceCapacityReport {
        schema: InferenceCapacityReport::SCHEMA,
        model_path: meta.path.clone(),
        model_size_bytes,
        geometry: Some(geometry),
        request,
        kv_cache: Some(breakdown),
        observed_available_ram_bytes: hardware.available_ram_bytes,
        system_reserve_bytes,
        conservative_known_footprint_bytes: Some(known_footprint),
        allocatable_bytes: Some(allocatable),
        decision: if reasons.is_empty() {
            CapacityDecision::Admitted
        } else {
            CapacityDecision::Blocked { reasons }
        },
        residual_unknowns,
    }
}

pub fn exact_kv_bytes(
    geometry: &GgufKvGeometry,
    request: &KvCapacityRequest,
) -> Option<KvCapacityBreakdown> {
    let slots = u64::from(request.parallel_slots);
    let key_elements = geometry
        .block_count
        .checked_mul(geometry.kv_head_count)?
        .checked_mul(geometry.key_length)?
        .checked_mul(request.context_tokens)?
        .checked_mul(slots)?;
    let value_elements = geometry
        .block_count
        .checked_mul(geometry.kv_head_count)?
        .checked_mul(geometry.value_length)?
        .checked_mul(request.context_tokens)?
        .checked_mul(slots)?;
    if key_elements == 0 || value_elements == 0 {
        return None;
    }
    let key_bytes = key_elements
        .div_ceil(GGML_BLOCK_ELEMENTS)
        .checked_mul(request.key_cache_type.bytes_per_block())?;
    let value_bytes = value_elements
        .div_ceil(GGML_BLOCK_ELEMENTS)
        .checked_mul(request.value_cache_type.bytes_per_block())?;
    Some(KvCapacityBreakdown {
        key_elements,
        value_elements,
        key_bytes,
        value_bytes,
        total_bytes: key_bytes.checked_add(value_bytes)?,
        formula: "GGML block bytes: ceil(elements/32) * cache_type_block_bytes, independently for K and V",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gguf_meta::GgufKvGeometry;

    fn tiel_geometry() -> GgufKvGeometry {
        GgufKvGeometry {
            architecture: "qwen35moe".to_string(),
            native_context_tokens: 262_144,
            block_count: 41,
            kv_head_count: 2,
            key_length: 256,
            value_length: 256,
        }
    }

    #[test]
    fn tiel_256k_q4_cache_is_exactly_sized_from_observed_geometry() {
        let request = KvCapacityRequest {
            context_tokens: 262_144,
            parallel_slots: 1,
            key_cache_type: KvCacheType::Q4_0,
            value_cache_type: KvCacheType::Q4_0,
        };
        let measured = exact_kv_bytes(&tiel_geometry(), &request).expect("nonzero geometry");
        assert_eq!(measured.key_bytes, 3_095_396_352);
        assert_eq!(measured.value_bytes, 3_095_396_352);
        assert_eq!(measured.total_bytes, 6_190_792_704);
    }

    #[test]
    fn independent_key_and_value_quantizations_are_not_collapsed() {
        let request = KvCapacityRequest {
            context_tokens: 262_144,
            parallel_slots: 1,
            key_cache_type: KvCacheType::Q8_0,
            value_cache_type: KvCacheType::Q4_0,
        };
        let measured = exact_kv_bytes(&tiel_geometry(), &request).expect("nonzero geometry");
        assert_eq!(measured.key_bytes, 5_846_859_776);
        assert_eq!(measured.value_bytes, 3_095_396_352);
        assert_eq!(measured.total_bytes, 8_942_256_128);
    }

    #[test]
    fn native_context_limit_blocks_before_launch() {
        let geometry = tiel_geometry();
        assert!(262_145 > geometry.native_context_tokens);
    }
}
