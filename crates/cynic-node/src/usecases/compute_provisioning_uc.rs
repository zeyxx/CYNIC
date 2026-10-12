//! Sovereign compute workload provisioning and execution use case.
//!
//! Replaces legacy orphan `cynic-cloud-api` with a typed, bounded FCIS use case.
//! Orchestrates pure placement decisions and dispatches execution to either
//! local Bubblewrap sandboxes or remote Proxmox LXC containers.

use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{
    CommandExecutionResult, ComputeRequirements, ComputeSubstratePort,
    SubstrateObservation, decide_compute_placement,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const COMPUTE_PROVISIONING_REPORT_SCHEMA: &str =
    "cynic.node.compute_provisioning_report.v1";

/// Declarative intent to provision and execute a compute workload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeProvisioningIntent {
    pub workload_id: String,
    pub requirements: ComputeRequirements,
    pub command: Vec<String>,
    pub timeout_ms: u64,
}

/// Verifiable report emitted after compute provisioning attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeProvisioningReport {
    pub schema: String,
    pub workload_id: String,
    pub admitted: bool,
    pub status: String,
    pub substrate_selected: Option<String>,
    pub strategy_selected: Option<SandboxStrategy>,
    pub execution_result: Option<CommandExecutionResult>,
    pub reason: String,
    pub duration_ms: u64,
}

/// Orchestrates pure admission/placement and drives execution via the injected substrate port.
pub async fn execute_compute_provisioning<P: ComputeSubstratePort>(
    intent: &ComputeProvisioningIntent,
    observed_substrates: &[SubstrateObservation],
    substrate_port: &P,
) -> Result<ComputeProvisioningReport, String> {
    let started = Instant::now();

    // 1. Pure placement predicate (Zero-I/O)
    let decision = decide_compute_placement(&intent.requirements, observed_substrates);

    if !decision.admitted {
        return Ok(ComputeProvisioningReport {
            schema: COMPUTE_PROVISIONING_REPORT_SCHEMA.to_string(),
            workload_id: intent.workload_id.clone(),
            admitted: false,
            status: "rejected".to_string(),
            substrate_selected: None,
            strategy_selected: None,
            execution_result: None,
            reason: decision.reason,
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    // 2. Resource allocation on chosen substrate
    let allocation = substrate_port
        .allocate_runner(&intent.workload_id, &intent.requirements)
        .await
        .map_err(|e| format!("substrate_allocation_failed: {e}"))?;

    // 3. Execution inside allocated runner
    let exec_res = substrate_port
        .execute_command(&allocation.allocation_id, &intent.command, intent.timeout_ms)
        .await;

    // 4. Guaranteed cleanup
    let release_res = substrate_port
        .release_runner(&allocation.allocation_id)
        .await;

    let execution_result = exec_res.map_err(|e| format!("substrate_execution_failed: {e}"))?;
    if let Err(e) = release_res {
        eprintln!(
            "warning: failed to release runner {}: {e}",
            allocation.allocation_id
        );
    }

    let status = if execution_result.exit_code == 0 {
        "succeeded".to_string()
    } else {
        "failed".to_string()
    };

    Ok(ComputeProvisioningReport {
        schema: COMPUTE_PROVISIONING_REPORT_SCHEMA.to_string(),
        workload_id: intent.workload_id.clone(),
        admitted: true,
        status,
        substrate_selected: decision.selected_substrate,
        strategy_selected: decision.selected_strategy,
        execution_result: Some(execution_result),
        reason: decision.reason,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}
