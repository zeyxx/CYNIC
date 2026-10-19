//! Driven and driving ports for sovereign compute substrates.

use super::types::{CommandExecutionResult, ComputeRequirements, RunnerAllocation};

/// Asynchronous port driving compute runner allocation and execution.
pub trait ComputeSubstratePort: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Allocate a compute runner matching workload requirements.
    fn allocate_runner(
        &self,
        workload_id: &str,
        requirements: &ComputeRequirements,
    ) -> impl std::future::Future<Output = Result<RunnerAllocation, Self::Error>> + Send;

    /// Execute a command in the allocated runner.
    fn execute_command(
        &self,
        allocation_id: &str,
        argv: &[String],
        timeout_ms: u64,
    ) -> impl std::future::Future<Output = Result<CommandExecutionResult, Self::Error>> + Send;

    /// Terminate and release the allocated runner.
    fn release_runner(
        &self,
        allocation_id: &str,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}
