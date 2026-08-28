//! model_audit_port.rs — Hexagonal Ports for Granular Model Probing, Audits, and Convergence.
//!
//! Provides decoupled trait boundaries separating:
//! 1. SubstrateProbePort (physical host observation)
//! 2. EmissionProbePort (dynamic execution with GenerationParameters)
//! 3. CognitiveAuditPort (epistemic reasoning & tool calling evaluation)
//! 4. EconomicEvaluatorPort (cost, TTCS, and Pareto convergence)

use async_trait::async_trait;

use crate::domain::audit_geometry::{
    GeometricMatrixPoint, ModelCognitiveCapabilityAudit, ModelEconomicEvaluation,
    ModelEmissionDynamics, ModelMatrixCoordinate, ModelSubstrateObservation,
};
use crate::domain::generation_parameters::GenerationParameters;

#[derive(Debug, thiserror::Error)]
pub enum AuditPortError {
    #[error("Substrate probe failed: {0}")]
    Substrate(String),
    #[error("Emission probe execution failed: {0}")]
    Emission(String),
    #[error("Cognitive capability evaluation failed: {0}")]
    Cognitive(String),
    #[error("Economic evaluation calculation error: {0}")]
    Economic(String),
}

/// Port for probing physical hardware substrate state (RSS, VRAM/GTT, temperature, power).
pub trait SubstrateProbePort: Send + Sync {
    fn probe_substrate(&self) -> Result<ModelSubstrateObservation, AuditPortError>;
}

/// Port for measuring dynamic emission performance with granular GenerationParameters.
#[async_trait]
pub trait EmissionProbePort: Send + Sync {
    async fn probe_emission(
        &self,
        coordinate: &ModelMatrixCoordinate,
        params: &GenerationParameters,
        prompt: &str,
    ) -> Result<ModelEmissionDynamics, AuditPortError>;
}

/// Port for auditing epistemic, reasoning, and tool-calling capabilities.
#[async_trait]
pub trait CognitiveAuditPort: Send + Sync {
    async fn audit_capabilities(
        &self,
        coordinate: &ModelMatrixCoordinate,
        params: &GenerationParameters,
    ) -> Result<ModelCognitiveCapabilityAudit, AuditPortError>;
}

/// Port for synthesizing economic decision metrics and Pareto optimality.
pub trait EconomicEvaluatorPort: Send + Sync {
    fn evaluate_economics(
        &self,
        coordinate: &ModelMatrixCoordinate,
        substrate: &ModelSubstrateObservation,
        emission: &ModelEmissionDynamics,
        capability: &ModelCognitiveCapabilityAudit,
    ) -> Result<ModelEconomicEvaluation, AuditPortError>;
}

/// Hexagonal orchestrator trait that coordinates all ports to produce a GeometricMatrixPoint.
#[async_trait]
pub trait ModelAuditConvergencePort: Send + Sync {
    async fn converge_geometric_point(
        &self,
        coordinate: &ModelMatrixCoordinate,
        params: &GenerationParameters,
        benchmark_prompt: &str,
    ) -> Result<GeometricMatrixPoint, AuditPortError>;
}
