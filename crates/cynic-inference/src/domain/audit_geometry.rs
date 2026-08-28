//! audit_geometry.rs — Granular, Decoupled Model Capability & Geometric Audit Framework.
//!
//! Provides orthogonal 5-layer geometric coordinates for inference evaluations:
//! - Layer 0 (Coordinate): Model architecture, quant, KV quant, context, offload, slots.
//! - Layer 1 (Substrate): Physical RAM RSS, GTT/VRAM aperture, temperature, power/RAPL.
//! - Layer 2 (Emission): Prefill speed, decode speed, TTFT, jitter, speculative \alpha, DRAM bandwidth.
//! - Layer 3 (Cognitive): Reasoning accuracy, tool-calling schema fidelity, long-context retention.
//! - Layer 4 (Economic): TTCS, tasks/hour, Wh/task, failure recovery rate (FRR), Pareto frontier tier.

use cynic_protocol::{ContextTier, KvQuantization};
use serde::{Deserialize, Serialize};

use crate::math::pareto::ObjectiveVector;
use crate::math::tensor_geometry::AttentionType;

pub const GEOMETRIC_AUDIT_MATRIX_SCHEMA: &str = "cynic.inference.audit_geometry.v1";

/// Layer 0: Pure multidimensional coordinate in parameter space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelMatrixCoordinate {
    pub model_alias: String,
    pub architecture: String,
    pub total_params_b: f32,
    pub active_params_b: f32,
    pub weight_quant: String,
    pub kv_quant: KvQuantization,
    pub context_tier: ContextTier,
    pub offload_strategy: OffloadStrategy,
    pub parallel_slots: u32,
    pub attention_type: AttentionType,
}

/// Offload strategy options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OffloadStrategy {
    CpuOnly { threads: u32 },
    FullVulkan { device: String, threads: u32 },
    Hybrid { gpu_layers: u32, device: String, threads: u32 },
}

/// Layer 1: Substrate physical resource and thermal observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSubstrateObservation {
    pub peak_rss_bytes: u64,
    pub gtt_allocated_bytes: u64,
    pub dedicated_vram_bytes: u64,
    pub power_watts_estimate: Option<f32>,
    pub peak_temperature_celsius: Option<f32>,
    pub throttling_observed: bool,
}

/// Layer 2: Emission and streaming dynamics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEmissionDynamics {
    pub prefill_tps: f32,
    pub decode_tps: f32,
    pub ttft_ms: f32,
    pub jitter_variance_ms: f32,
    pub speculative_alpha: Option<f32>,
    pub effective_dram_bandwidth_gbps: f32,
}

/// Layer 3: Epistemic & cognitive capability audit scores (in [0.0, 1.0]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCognitiveCapabilityAudit {
    /// Pass@1 score on domain reasoning benchmarks.
    pub reasoning_pass1: f32,
    /// Accuracy in selecting and strictly obeying JSON tool-calling schemas.
    pub tool_schema_fidelity: f32,
    /// Retrieval and needle-in-a-haystack retention across extended context.
    pub long_context_retention: f32,
    /// Ratio of quantized score vs unquantized baseline (quantization cliff detector).
    pub quantization_retention_score: f32,
}

/// Layer 4: Final economic decision metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEconomicEvaluation {
    /// Time To Correct Solution in milliseconds (includes tools + retries).
    pub ttcs_ms: f64,
    /// Validated useful tasks produced per hour.
    pub tasks_per_hour: f64,
    /// Mean tokens consumed per successfully completed task.
    pub tokens_per_successful_task: u64,
    /// Energy cost per completed task in Wh.
    pub wh_per_successful_task: Option<f64>,
    /// Failure Recovery Rate: probability of recovering if previous step failed.
    pub failure_recovery_rate: Option<f64>,
    /// Whether this point is on the non-dominated Pareto frontier.
    pub is_pareto_optimal: bool,
}

/// Geometrically complete matrix evaluation point composed across all 5 orthogonal layers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometricMatrixPoint {
    pub schema: &'static str,
    pub coordinate: ModelMatrixCoordinate,
    pub substrate: ModelSubstrateObservation,
    pub emission: ModelEmissionDynamics,
    pub capability: ModelCognitiveCapabilityAudit,
    pub economics: ModelEconomicEvaluation,
}

impl GeometricMatrixPoint {
    pub const SCHEMA: &'static str = GEOMETRIC_AUDIT_MATRIX_SCHEMA;

    /// Projects this point into an ObjectiveVector for mathematical Pareto analysis.
    pub fn to_objective_vector(&self) -> ObjectiveVector {
        ObjectiveVector {
            throughput_tps: self.emission.decode_tps,
            ttft_ms: self.emission.ttft_ms,
            jitter_variance_ms: self.emission.jitter_variance_ms,
            context_tokens: self.coordinate.context_tier.token_count(),
            quality_score: Some(self.capability.reasoning_pass1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometric_matrix_point_projection() {
        let point = GeometricMatrixPoint {
            schema: GeometricMatrixPoint::SCHEMA,
            coordinate: ModelMatrixCoordinate {
                model_alias: "qwen36-35b-iq2".to_string(),
                architecture: "qwen35moe".to_string(),
                total_params_b: 35.0,
                active_params_b: 3.0,
                weight_quant: "IQ2_M".to_string(),
                kv_quant: KvQuantization::Q4_0,
                context_tier: ContextTier::Ctx64k,
                offload_strategy: OffloadStrategy::FullVulkan {
                    device: "Vulkan0".to_string(),
                    threads: 8,
                },
                parallel_slots: 2,
                attention_type: AttentionType::Gqa,
            },
            substrate: ModelSubstrateObservation {
                peak_rss_bytes: 1_200_000_000,
                gtt_allocated_bytes: 10_500_000_000,
                dedicated_vram_bytes: 0,
                power_watts_estimate: Some(45.0),
                peak_temperature_celsius: Some(68.5),
                throttling_observed: false,
            },
            emission: ModelEmissionDynamics {
                prefill_tps: 280.0,
                decode_tps: 34.5,
                ttft_ms: 2200.0,
                jitter_variance_ms: 1.2,
                speculative_alpha: Some(0.78),
                effective_dram_bandwidth_gbps: 38.6,
            },
            capability: ModelCognitiveCapabilityAudit {
                reasoning_pass1: 0.88,
                tool_schema_fidelity: 0.96,
                long_context_retention: 0.92,
                quantization_retention_score: 0.94,
            },
            economics: ModelEconomicEvaluation {
                ttcs_ms: 8500.0,
                tasks_per_hour: 423.5,
                tokens_per_successful_task: 1450,
                wh_per_successful_task: Some(0.106),
                failure_recovery_rate: Some(0.72),
                is_pareto_optimal: true,
            },
        };

        let vector = point.to_objective_vector();
        assert_eq!(vector.throughput_tps, 34.5);
        assert_eq!(vector.ttft_ms, 2200.0);
        assert_eq!(vector.context_tokens, 65536);
        assert_eq!(vector.quality_score, Some(0.88));
    }
}
