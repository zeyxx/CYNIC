//! audit_convergence.rs — Hexagonal Application Service for Model Audit Convergence.
//!
//! Orchestrates the decoupled ports (Substrate, Emission, Cognitive, Economics)
//! to produce a complete GeometricMatrixPoint without coupling to any specific runtime.

use std::sync::Arc;
use async_trait::async_trait;

use crate::domain::audit_geometry::{
    GeometricMatrixPoint, ModelMatrixCoordinate,
};
use crate::domain::generation_parameters::GenerationParameters;
use crate::ports::model_audit_port::{
    AuditPortError, CognitiveAuditPort, EconomicEvaluatorPort, EmissionProbePort,
    ModelAuditConvergencePort, SubstrateProbePort,
};

/// Hexagonal application service orchestrating model capability convergence across ports.
pub struct AuditConvergenceService {
    substrate_probe: Arc<dyn SubstrateProbePort>,
    emission_probe: Arc<dyn EmissionProbePort>,
    cognitive_audit: Arc<dyn CognitiveAuditPort>,
    economic_evaluator: Arc<dyn EconomicEvaluatorPort>,
}

impl AuditConvergenceService {
    pub fn new(
        substrate_probe: Arc<dyn SubstrateProbePort>,
        emission_probe: Arc<dyn EmissionProbePort>,
        cognitive_audit: Arc<dyn CognitiveAuditPort>,
        economic_evaluator: Arc<dyn EconomicEvaluatorPort>,
    ) -> Self {
        Self {
            substrate_probe,
            emission_probe,
            cognitive_audit,
            economic_evaluator,
        }
    }
}

#[async_trait]
impl ModelAuditConvergencePort for AuditConvergenceService {
    async fn converge_geometric_point(
        &self,
        coordinate: &ModelMatrixCoordinate,
        params: &GenerationParameters,
        benchmark_prompt: &str,
    ) -> Result<GeometricMatrixPoint, AuditPortError> {
        // 1. Layer 1: Probe physical substrate
        let substrate = self.substrate_probe.probe_substrate()?;

        // 2. Layer 2: Measure dynamic emission performance with GenerationParameters
        let emission = self
            .emission_probe
            .probe_emission(coordinate, params, benchmark_prompt)
            .await?;

        // 3. Layer 3: Audit cognitive, reasoning, and schema capabilities
        let capability = self
            .cognitive_audit
            .audit_capabilities(coordinate, params)
            .await?;

        // 4. Layer 4: Compute economic metrics & Pareto status
        let economics = self
            .economic_evaluator
            .evaluate_economics(coordinate, &substrate, &emission, &capability)?;

        // 5. Compose the unified, geometrically sound matrix point
        Ok(GeometricMatrixPoint {
            schema: GeometricMatrixPoint::SCHEMA,
            coordinate: coordinate.clone(),
            substrate,
            emission,
            capability,
            economics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::audit_geometry::{
        ModelCognitiveCapabilityAudit, ModelEconomicEvaluation, ModelEmissionDynamics,
        ModelSubstrateObservation, OffloadStrategy,
    };
    use crate::math::tensor_geometry::AttentionType;
    use cynic_protocol::{ContextTier, KvQuantization};

    struct MockSubstrate;
    impl SubstrateProbePort for MockSubstrate {
        fn probe_substrate(&self) -> Result<ModelSubstrateObservation, AuditPortError> {
            Ok(ModelSubstrateObservation {
                peak_rss_bytes: 1024 * 1024 * 1024,
                gtt_allocated_bytes: 4 * 1024 * 1024 * 1024,
                dedicated_vram_bytes: 0,
                power_watts_estimate: Some(40.0),
                peak_temperature_celsius: Some(65.0),
                throttling_observed: false,
            })
        }
    }

    struct MockEmission;
    #[async_trait]
    impl EmissionProbePort for MockEmission {
        async fn probe_emission(
            &self,
            _coord: &ModelMatrixCoordinate,
            _params: &GenerationParameters,
            _prompt: &str,
        ) -> Result<ModelEmissionDynamics, AuditPortError> {
            Ok(ModelEmissionDynamics {
                prefill_tps: 500.0,
                decode_tps: 40.0,
                ttft_ms: 500.0,
                jitter_variance_ms: 1.0,
                speculative_alpha: Some(0.85),
                effective_dram_bandwidth_gbps: 40.0,
            })
        }
    }

    struct MockCognitive;
    #[async_trait]
    impl CognitiveAuditPort for MockCognitive {
        async fn audit_capabilities(
            &self,
            _coord: &ModelMatrixCoordinate,
            _params: &GenerationParameters,
        ) -> Result<ModelCognitiveCapabilityAudit, AuditPortError> {
            Ok(ModelCognitiveCapabilityAudit {
                reasoning_pass1: 0.95,
                tool_schema_fidelity: 0.98,
                long_context_retention: 0.90,
                quantization_retention_score: 0.96,
            })
        }
    }

    struct MockEconomics;
    impl EconomicEvaluatorPort for MockEconomics {
        fn evaluate_economics(
            &self,
            _coord: &ModelMatrixCoordinate,
            _sub: &ModelSubstrateObservation,
            _em: &ModelEmissionDynamics,
            _cap: &ModelCognitiveCapabilityAudit,
        ) -> Result<ModelEconomicEvaluation, AuditPortError> {
            Ok(ModelEconomicEvaluation {
                ttcs_ms: 5000.0,
                tasks_per_hour: 720.0,
                tokens_per_successful_task: 200,
                wh_per_successful_task: Some(0.05),
                failure_recovery_rate: Some(0.80),
                is_pareto_optimal: true,
            })
        }
    }

    #[tokio::test]
    async fn test_audit_convergence_service_assembles_clean_geometric_point() {
        let service = AuditConvergenceService::new(
            Arc::new(MockSubstrate),
            Arc::new(MockEmission),
            Arc::new(MockCognitive),
            Arc::new(MockEconomics),
        );

        let coord = ModelMatrixCoordinate {
            model_alias: "test-model".to_string(),
            architecture: "llama".to_string(),
            total_params_b: 1.0,
            active_params_b: 1.0,
            weight_quant: "Q8_0".to_string(),
            kv_quant: KvQuantization::Q4_0,
            context_tier: ContextTier::Ctx4k,
            offload_strategy: OffloadStrategy::FullVulkan {
                device: "Vulkan0".to_string(),
                threads: 8,
            },
            parallel_slots: 1,
            attention_type: AttentionType::Mha,
        };

        let params = GenerationParameters::deterministic_reasoning(512);

        let point = service
            .converge_geometric_point(&coord, &params, "Test Prompt")
            .await
            .expect("convergence must succeed");

        assert_eq!(point.coordinate.model_alias, "test-model");
        assert_eq!(point.emission.decode_tps, 40.0);
        assert_eq!(point.capability.reasoning_pass1, 0.95);
        assert!(point.economics.is_pareto_optimal);
    }
}
