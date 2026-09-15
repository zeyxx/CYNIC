//! Canonical workload read-back validation shared by CLI and MCP projections.

use crate::ports::build_workload_observation_port::{
    BuildWorkloadObservationPort, BuildWorkloadObservationReadBack,
};

pub struct BuildWorkloadObservationResult {
    pub read_back: BuildWorkloadObservationReadBack,
    pub delivery: Option<cynic_workload::AgentWorkloadDelivery>,
    pub read_back_violations: Vec<String>,
}

pub async fn observe_build_workload(
    port: &impl BuildWorkloadObservationPort,
    workload_id: &str,
) -> Result<BuildWorkloadObservationResult, String> {
    let read_back = port.observe_workload(workload_id).await?;
    let mut read_back_violations =
        match cynic_workload::parse_agent_workload_observation_read_back(&read_back.body) {
            Ok(delivery) if delivery.envelope.workload_id == workload_id => {
                return Ok(BuildWorkloadObservationResult {
                    read_back,
                    delivery: Some(delivery),
                    read_back_violations: Vec::new(),
                });
            }
            Ok(_) => vec!["agent_workload_observation_workload_id_mismatch".to_string()],
            Err(violations) => violations,
        };
    read_back_violations.sort();
    read_back_violations.dedup();
    Ok(BuildWorkloadObservationResult {
        read_back,
        delivery: None,
        read_back_violations,
    })
}
