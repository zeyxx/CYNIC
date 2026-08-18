use crate::domain::hermes_runtime::{HermesGatewayMutation, HermesGatewayStatus};

#[derive(Debug, thiserror::Error)]
pub enum HermesGatewayRuntimeError {
    #[error("Hermes gateway control failed: {0}")]
    Control(String),
    #[error("Hermes gateway readiness failed: {0}")]
    Readiness(String),
}

/// Lifecycle boundary for the Hermes layer above model inference.
///
/// The port deliberately does not expose a provider-specific command line;
/// process/systemd/container adapters own that detail.
pub trait HermesGatewayRuntimePort: Send + Sync {
    fn status(&self) -> Result<HermesGatewayStatus, HermesGatewayRuntimeError>;
    fn start(
        &self,
        wait_for_ready: bool,
    ) -> Result<HermesGatewayMutation, HermesGatewayRuntimeError>;
    fn stop(&self) -> Result<HermesGatewayMutation, HermesGatewayRuntimeError>;
}
