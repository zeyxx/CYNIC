//! Inference engine selector.
//!
//! From Ahmad's "Inference Engines for LLMs & Local AI Hardware":
//!   "You don't pick an engine first. You pick a hardware strategy, a workload
//!   shape, and a serving model. The engine follows."
//!
//! This module implements that decision as a pure function.

use crate::domain::hardware_profile::HardwareTier;

/// Inference engine families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceEngine {
    /// Portable local runtime (llama.cpp, MLC LLM, ONNX Runtime GenAI).
    Portable,
    /// Apple Silicon native (MLX, MLX-LM).
    AppleNative,
    /// Consumer CUDA single-GPU (ExLlamaV2).
    ConsumerCuda,
    /// Multi-GPU quantized (ExLlamaV3, vLLM).
    MultiCuda,
    /// Production serving (vLLM, SGLang).
    Production,
    /// NVIDIA max performance (TensorRT-LLM).
    NvidiaMax,
    /// Cluster orchestration layer.
    Cluster,
}

impl std::fmt::Display for InferenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Portable => write!(f, "portable"),
            Self::AppleNative => write!(f, "apple_native"),
            Self::ConsumerCuda => write!(f, "consumer_cuda"),
            Self::MultiCuda => write!(f, "multi_cuda"),
            Self::Production => write!(f, "production"),
            Self::NvidiaMax => write!(f, "nvidia_max"),
            Self::Cluster => write!(f, "cluster"),
        }
    }
}

/// Workload shape for engine selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadShape {
    /// Single-user local, offline.
    SingleUserLocal,
    /// Team / private API.
    TeamPrivate,
    /// Production serving, many users.
    ProductionServing,
    /// Long context workloads (32K+).
    LongContext,
    /// Mixture-of-Experts model.
    MixtureOfExperts,
    /// Multi-node fleet.
    MultiNode,
}

/// Engine recommendation with reasoning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineRecommendation {
    pub engine: InferenceEngine,
    pub reason: String,
    /// Alternative if primary is unavailable.
    pub fallback: Option<InferenceEngine>,
}

/// Select an engine based on hardware tier and workload shape.
///
/// Implements Ahmad's "one-page decision guide":
///   Laptop / edge / odd hardware → llama.cpp (Portable)
///   Mac-first workflows → MLX / MLX-LM (AppleNative)
///   Single RTX local inference → ExLlamaV2 (ConsumerCuda)
///   2-4+ NVIDIA / CUDA GPUs → ExLlamaV3 (MultiCuda)
///   General production serving → vLLM (Production)
///   Long-context / MoE / routing → SGLang (Production w/ features)
///   NVIDIA max performance → TensorRT-LLM (NvidiaMax)
///   Cluster orchestration → NVIDIA Dynamo (Cluster)
pub fn select_engine(
    hardware_tier: HardwareTier,
    has_apple_silicon: bool,
    has_nvidia_gpu: bool,
    workload: &WorkloadShape,
) -> EngineRecommendation {
    // Mac-first → Apple Native
    if has_apple_silicon && matches!(workload, WorkloadShape::SingleUserLocal) {
        return EngineRecommendation {
            engine: InferenceEngine::AppleNative,
            reason: "Mac-first workflow → MLX / MLX-LM for unified memory.".into(),
            fallback: Some(InferenceEngine::Portable),
        };
    }

    // Multi-node → Cluster
    if matches!(workload, WorkloadShape::MultiNode) {
        return EngineRecommendation {
            engine: InferenceEngine::Cluster,
            reason: "Multi-node fleet → orchestration layer (Dynamo) above engine.".into(),
            fallback: Some(InferenceEngine::Production),
        };
    }

    // MoE workloads need production features
    if matches!(workload, WorkloadShape::MixtureOfExperts) {
        return if has_nvidia_gpu && matches!(hardware_tier, HardwareTier::Server) {
            EngineRecommendation {
                engine: InferenceEngine::NvidiaMax,
                reason: "MoE on NVIDIA server → TensorRT-LLM expert parallelism.".into(),
                fallback: Some(InferenceEngine::Production),
            }
        } else {
            EngineRecommendation {
                engine: InferenceEngine::Production,
                reason: "MoE workload → vLLM or SGLang expert routing.".into(),
                fallback: Some(InferenceEngine::MultiCuda),
            }
        };
    }

    // Long context → production with features
    if matches!(workload, WorkloadShape::LongContext) {
        return EngineRecommendation {
            engine: InferenceEngine::Production,
            reason: "Long context → vLLM / SGLang paged attention + prefix caching.".into(),
            fallback: Some(InferenceEngine::MultiCuda),
        };
    }

    // Hardware tier based selection
    match hardware_tier {
        HardwareTier::Edge => EngineRecommendation {
            engine: InferenceEngine::Portable,
            reason: "Edge tier → llama.cpp for CPU / low-VRAM portability.".into(),
            fallback: None,
        },
        HardwareTier::ThinAndLight => EngineRecommendation {
            engine: InferenceEngine::Portable,
            reason: "Thin-and-light → llama.cpp for portability and low overhead.".into(),
            fallback: Some(InferenceEngine::AppleNative),
        },
        HardwareTier::Workstation => {
            if has_nvidia_gpu {
                EngineRecommendation {
                    engine: InferenceEngine::ConsumerCuda,
                    reason:
                        "Single NVIDIA workstation → ExLlamaV2 for fast quantized local inference."
                            .into(),
                    fallback: Some(InferenceEngine::Production),
                }
            } else {
                EngineRecommendation {
                    engine: InferenceEngine::Portable,
                    reason: "Non-NVIDIA workstation → llama.cpp with Vulkan/OpenCL.".into(),
                    fallback: None,
                }
            }
        }
        HardwareTier::Server => match workload {
            WorkloadShape::ProductionServing => EngineRecommendation {
                engine: InferenceEngine::Production,
                reason: "Server production → vLLM / SGLang continuous batching.".into(),
                fallback: Some(InferenceEngine::NvidiaMax),
            },
            WorkloadShape::TeamPrivate => EngineRecommendation {
                engine: InferenceEngine::Production,
                reason: "Team private API → vLLM OpenAI-compatible endpoint.".into(),
                fallback: Some(InferenceEngine::MultiCuda),
            },
            _ => EngineRecommendation {
                engine: InferenceEngine::MultiCuda,
                reason: "Server single-user → ExLlamaV3 multi-GPU quantized.".into(),
                fallback: Some(InferenceEngine::Production),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_single_user_selects_apple_native() {
        let rec = select_engine(
            HardwareTier::Workstation,
            true,  // apple silicon
            false, // no nvidia
            &WorkloadShape::SingleUserLocal,
        );
        assert_eq!(rec.engine, InferenceEngine::AppleNative);
    }

    #[test]
    fn multi_node_selects_cluster() {
        let rec = select_engine(HardwareTier::Server, false, true, &WorkloadShape::MultiNode);
        assert_eq!(rec.engine, InferenceEngine::Cluster);
    }

    #[test]
    fn moe_server_selects_nvidia_max() {
        let rec = select_engine(
            HardwareTier::Server,
            false,
            true,
            &WorkloadShape::MixtureOfExperts,
        );
        assert_eq!(rec.engine, InferenceEngine::NvidiaMax);
    }

    #[test]
    fn edge_selects_portable() {
        let rec = select_engine(
            HardwareTier::Edge,
            false,
            false,
            &WorkloadShape::SingleUserLocal,
        );
        assert_eq!(rec.engine, InferenceEngine::Portable);
    }

    #[test]
    fn workstation_nvidia_selects_consumer_cuda() {
        let rec = select_engine(
            HardwareTier::Workstation,
            false,
            true,
            &WorkloadShape::SingleUserLocal,
        );
        assert_eq!(rec.engine, InferenceEngine::ConsumerCuda);
    }

    #[test]
    fn server_production_selects_production() {
        let rec = select_engine(
            HardwareTier::Server,
            false,
            true,
            &WorkloadShape::ProductionServing,
        );
        assert_eq!(rec.engine, InferenceEngine::Production);
    }
}
