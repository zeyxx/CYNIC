use cynic_workload::{
    admit_source_change_submission,
    AgentWorkloadDelivery,
    AgentWorkloadDeliveryStatus,
    AgentWorkloadEnvelope,
    SourceChangeSubmission,
};

use crate::ports::agent_workload_submission_port::{
    AgentWorkloadSubmissionPort, AgentWorkloadSubmissionReadBack,
};

pub fn plan_source_change_submission(
    workload: AgentWorkloadEnvelope,
    submission: SourceChangeSubmission,
    submitted_by: String,
    submitted_at_ms: u64,
) -> Result<AgentWorkloadDelivery, Vec<String>> {
    let admission = admit_source_change_submission(&submission, &workload);
    if admission.status != "admitted" {
        return Err(admission.blockers);
    }
    if workload.delegation.node_id.trim().is_empty() || submitted_by.trim().is_empty() {
        return Err(vec![
            "source_change_submission_delivery_identity_missing".into()
        ]);
    }
    Ok(AgentWorkloadDelivery {
        schema: cynic_workload::AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
        target_node_id: workload.delegation.node_id.clone(),
        submitted_by,
        submitted_at_ms,
        status: AgentWorkloadDeliveryStatus::Pending,
        accepted_at_ms: None,
        accepted_record_digest: None,
        acceptance_lease: None,
        terminal_at_ms: None,
        terminal_record_digest: None,
        terminal_record: None,
        completion: None,
        completion_evidence: Vec::new(),
        evidence_refs: submission.evidence_refs.clone(),
        envelope: workload,
    })
}

pub async fn submit_source_change_submission(
    port: &dyn AgentWorkloadSubmissionPort,
    delivery: &AgentWorkloadDelivery,
) -> Result<AgentWorkloadSubmissionReadBack, String> {
    let read_back = port.submit(delivery).await?;
    if !(200..300).contains(&read_back.http_status) {
        return Err("source_change_submission_kernel_http_status_not_success".into());
    }
    let violations =
        cynic_workload::validate_agent_workload_submission_read_back(delivery, &read_back.body);
    if violations.is_empty() {
        Ok(read_back)
    } else {
        Err(violations.join(","))
    }
}
