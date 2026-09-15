use cynic_host::ports::evidence_store::ContentAddressedEvidenceStorePort;
use cynic_workload::{
    AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA,
    AgentWorkloadCompletionPublication,
};

use crate::ports::{
    agent_workload_completion_receipt_port::AgentWorkloadCompletionReceiptPort,
    agent_workload_delivery_lease_port::AgentWorkloadDeliveryLeasePort,
    agent_workload_delivery_port::AgentWorkloadDeliveryPort,
    agent_workload_store_port::AgentWorkloadStorePort,
};

pub(super) async fn reconcile<D>(
    node_id: &str,
    delivery_port: &D,
    store: &dyn AgentWorkloadStorePort,
    leases: &dyn AgentWorkloadDeliveryLeasePort,
    evidence_store: &dyn ContentAddressedEvidenceStorePort,
    receipts: &dyn AgentWorkloadCompletionReceiptPort,
) -> serde_json::Value
where
    D: AgentWorkloadDeliveryPort,
{
    let records = match store.durable_records() {
        Ok(records) => records,
        Err(error) => {
            return report(
                node_id,
                Vec::new(),
                Vec::new(),
                vec![format!("durable_workload_scan_failed:{}", error.code)],
            );
        }
    };
    let mut published = Vec::new();
    let mut blockers = Vec::new();
    let mut unknowns = Vec::new();
    for durable in records
        .into_iter()
        .filter(|record| record.record.state.terminal())
    {
        let workload_id = durable.record.envelope.workload_id.clone();
        if durable.record.envelope.delegation.node_id != node_id {
            blockers.push(format!("terminal_workload_node_mismatch:{workload_id}"));
            continue;
        }
        match receipts.contains(&durable.record_digest) {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => {
                blockers.push(format!("completion_receipt_invalid:{workload_id}:{error}"));
                continue;
            }
        }
        let Some(completion) = durable.record.completion.as_ref() else {
            blockers.push(format!(
                "terminal_workload_completion_missing:{workload_id}"
            ));
            continue;
        };
        let delivery_lease = match leases.read(&workload_id) {
            Ok(Some(lease)) => lease,
            Ok(None) => {
                blockers.push(format!("workload_delivery_lease_missing:{workload_id}"));
                continue;
            }
            Err(error) => {
                blockers.push(format!(
                    "workload_delivery_lease_invalid:{workload_id}:{error}"
                ));
                continue;
            }
        };
        let mut evidence = Vec::new();
        let mut evidence_missing = false;
        for reference in &completion.evidence_refs {
            match evidence_store.read(&reference.digest) {
                Ok(Some(envelope)) => evidence.push(envelope),
                Ok(None) => {
                    blockers.push(format!(
                        "completion_evidence_missing:{workload_id}:{}",
                        reference.digest
                    ));
                    evidence_missing = true;
                }
                Err(error) => {
                    blockers.push(format!(
                        "completion_evidence_invalid:{workload_id}:{}:{error}",
                        reference.digest
                    ));
                    evidence_missing = true;
                }
            }
        }
        if evidence_missing {
            continue;
        }
        match store.read(&workload_id) {
            Ok(Some(read_back)) if read_back == durable => {}
            Ok(Some(read_back)) => {
                blockers.push(format!(
                    "terminal_workload_read_back_mismatch:{workload_id}:{}",
                    read_back.record_digest
                ));
                continue;
            }
            Ok(None) => {
                blockers.push(format!("terminal_workload_read_back_missing:{workload_id}"));
                continue;
            }
            Err(error) => {
                blockers.push(format!(
                    "terminal_workload_read_back_invalid:{workload_id}:{}",
                    error.code
                ));
                continue;
            }
        }
        let publication = AgentWorkloadCompletionPublication {
            schema: AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA.to_string(),
            node_id: node_id.to_string(),
            workload_id: workload_id.clone(),
            published_at_ms: super::now_unix_ms(),
            delivery_lease,
            durable_record: durable,
            evidence,
        };
        match delivery_port.publish_completion(&publication).await {
            Ok(read_back) => match receipts.persist(&read_back) {
                Ok(()) => published.push(workload_id),
                Err(error) => unknowns.push(format!(
                    "completion_receipt_persistence_failed:{workload_id}:{error}"
                )),
            },
            Err(error) => unknowns.push(format!(
                "workload_completion_publication_failed:{workload_id}:{error}"
            )),
        }
    }
    report(node_id, published, blockers, unknowns)
}

fn report(
    node_id: &str,
    published: Vec<String>,
    blockers: Vec<String>,
    unknowns: Vec<String>,
) -> serde_json::Value {
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else if published.is_empty() {
        "idle"
    } else {
        "published"
    };
    serde_json::json!({
        "schema": "cynic.node.agent_workload_completion_tick.v1",
        "status": status,
        "node_id": node_id,
        "published": published,
        "blockers": blockers,
        "unknowns": unknowns,
    })
}
