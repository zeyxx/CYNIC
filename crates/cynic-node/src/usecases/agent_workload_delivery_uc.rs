mod completion;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cynic_host::ports::evidence_store::ContentAddressedEvidenceStorePort;
use cynic_workload::{
    AGENT_WORKLOAD_RECORD_SCHEMA,
    AgentWorkloadDecision,
    AgentWorkloadDelivery,
    AgentWorkloadDeliveryAcceptance,
    AgentWorkloadRecord,
    AgentWorkloadState,
    admit_agent_workload_dispatch,
    admit_aggregate_sibling_budget,
};
use tokio_util::sync::CancellationToken;

use crate::ports::{
    agent_workload_completion_receipt_port::AgentWorkloadCompletionReceiptPort,
    agent_workload_delivery_lease_port::AgentWorkloadDeliveryLeasePort,
    agent_workload_delivery_port::AgentWorkloadDeliveryPort,
    agent_workload_store_port::AgentWorkloadStorePort,
};

pub async fn reconcile_deliveries<D>(
    node_id: &str,
    consumer_instance_id: &str,
    lease_ttl_ms: u64,
    delivery_port: &D,
    store: &dyn AgentWorkloadStorePort,
    leases: &dyn AgentWorkloadDeliveryLeasePort,
    receipts: &dyn AgentWorkloadCompletionReceiptPort,
) -> serde_json::Value
where
    D: AgentWorkloadDeliveryPort,
{
    let mut renewed = Vec::new();
    let mut blockers = Vec::new();
    let mut unknowns = Vec::new();
    match store.durable_records() {
        Ok(records) => {
            for durable in records {
                let workload_id = durable.record.envelope.workload_id.clone();
                if durable.record.state.terminal() {
                    match receipts.contains(&durable.record_digest) {
                        Ok(true) => continue,
                        Ok(false) => {}
                        Err(error) => {
                            blockers
                                .push(format!("completion_receipt_invalid:{workload_id}:{error}"));
                            continue;
                        }
                    }
                }
                let lease = match leases.read(&workload_id) {
                    Ok(Some(lease)) => lease,
                    Ok(None) => continue,
                    Err(error) => {
                        blockers.push(format!(
                            "workload_delivery_lease_read_failed:{workload_id}:{error}"
                        ));
                        continue;
                    }
                };
                let acceptance = AgentWorkloadDeliveryAcceptance {
                    consumer_instance_id: consumer_instance_id.to_string(),
                    record_digest: durable.record_digest.clone(),
                    requested_at_ms: now_unix_ms(),
                    lease_ttl_ms,
                    prior_lease_id: Some(lease.lease_id),
                    prior_fencing_token: Some(lease.fencing_token),
                };
                match delivery_port
                    .accept_delivery(node_id, &workload_id, &acceptance)
                    .await
                {
                    Ok(read_back) => match read_back.acceptance_lease.as_ref() {
                        Some(lease) => match leases.persist(&workload_id, lease) {
                            Ok(()) => renewed.push(workload_id),
                            Err(error) => unknowns.push(format!(
                                "workload_delivery_lease_persistence_failed:{workload_id}:{error}"
                            )),
                        },
                        None => unknowns.push(format!(
                            "workload_delivery_renewal_read_back_missing:{workload_id}"
                        )),
                    },
                    Err(error) => unknowns.push(format!(
                        "workload_delivery_renewal_failed:{workload_id}:{error}"
                    )),
                }
            }
        }
        Err(error) => blockers.push(format!("durable_workload_scan_failed:{}", error.code)),
    }
    let deliveries = match delivery_port.pending_deliveries(node_id).await {
        Ok(deliveries) => deliveries,
        Err(error) => {
            return serde_json::json!({
                "schema": "cynic.node.agent_workload_delivery_tick.v1",
                "status": "unknown",
                "node_id": node_id,
                "accepted": [],
                "blockers": [],
                "unknowns": [format!("workload_delivery_pull_failed:{error}")]
            });
        }
    };
    let mut accepted = Vec::new();
    for delivery in deliveries {
        match persist_delivery(store, &delivery) {
            Ok(durable) => {
                let acceptance = AgentWorkloadDeliveryAcceptance {
                    consumer_instance_id: consumer_instance_id.to_string(),
                    record_digest: durable.record_digest.clone(),
                    requested_at_ms: now_unix_ms(),
                    lease_ttl_ms,
                    prior_lease_id: None,
                    prior_fencing_token: None,
                };
                match delivery_port
                    .accept_delivery(node_id, &delivery.envelope.workload_id, &acceptance)
                    .await
                {
                    Ok(read_back) => match read_back.acceptance_lease.as_ref() {
                        Some(lease) => {
                            match leases.persist(&delivery.envelope.workload_id, lease) {
                                Ok(()) => accepted.push(serde_json::json!({
                                    "workload_id": delivery.envelope.workload_id,
                                    "record_digest": durable.record_digest,
                                    "delivery": read_back,
                                })),
                                Err(error) => unknowns.push(format!(
                                    "workload_delivery_lease_persistence_failed:{}:{error}",
                                    delivery.envelope.workload_id
                                )),
                            }
                        }
                        None => unknowns.push(format!(
                            "workload_delivery_acceptance_read_back_missing:{}",
                            delivery.envelope.workload_id
                        )),
                    },
                    Err(error) => unknowns.push(format!(
                        "workload_delivery_acceptance_failed:{}:{error}",
                        delivery.envelope.workload_id
                    )),
                }
            }
            Err(error) => blockers.push(format!(
                "workload_delivery_persistence_blocked:{}:{error}",
                delivery.envelope.workload_id
            )),
        }
    }
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else if accepted.is_empty() && renewed.is_empty() {
        "idle"
    } else {
        "accepted"
    };
    serde_json::json!({
        "schema": "cynic.node.agent_workload_delivery_tick.v1",
        "status": status,
        "node_id": node_id,
        "accepted": accepted,
        "renewed": renewed,
        "blockers": blockers,
        "unknowns": unknowns
    })
}

pub async fn run_delivery_loop<D>(
    node_id: &str,
    consumer_instance_id: &str,
    lease_ttl_ms: u64,
    interval: Duration,
    delivery_port: &D,
    store: &dyn AgentWorkloadStorePort,
    leases: &dyn AgentWorkloadDeliveryLeasePort,
    evidence_store: &dyn ContentAddressedEvidenceStorePort,
    receipts: &dyn AgentWorkloadCompletionReceiptPort,
    shutdown: &CancellationToken,
) where
    D: AgentWorkloadDeliveryPort,
{
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let report = reconcile_deliveries(
                    node_id,
                    consumer_instance_id,
                    lease_ttl_ms,
                    delivery_port,
                    store,
                    leases,
                    receipts,
                ).await;
                match report.get("status").and_then(serde_json::Value::as_str) {
                    Some("blocked" | "unknown") => tracing::warn!(report = %report, "agent workload delivery degraded"),
                    Some("accepted") => tracing::info!(report = %report, "agent workload delivered and read back"),
                    _ => {}
                }
                let report = completion::reconcile(
                    node_id,
                    delivery_port,
                    store,
                    leases,
                    evidence_store,
                    receipts,
                ).await;
                match report.get("status").and_then(serde_json::Value::as_str) {
                    Some("blocked" | "unknown") => tracing::warn!(report = %report, "agent workload completion publication degraded"),
                    Some("published") => tracing::info!(report = %report, "agent workload completion published and retained"),
                    _ => {}
                }
            }
            _ = shutdown.cancelled() => break,
        }
    }
}

fn persist_delivery(
    store: &dyn AgentWorkloadStorePort,
    delivery: &AgentWorkloadDelivery,
) -> Result<cynic_workload::DurableAgentWorkloadRecord, String> {
    if let Some(existing) = store
        .read(&delivery.envelope.workload_id)
        .map_err(|failure| failure.code)?
    {
        return (existing.record.envelope == delivery.envelope)
            .then_some(existing)
            .ok_or_else(|| "durable_workload_envelope_conflict".to_string());
    }
    let active_idempotency_keys = store
        .active_idempotency_keys()
        .map_err(|failure| failure.code)?;
    let admission = admit_agent_workload_dispatch(&delivery.envelope, &active_idempotency_keys);
    if !admission.admitted() {
        return Err(match admission.decision {
            AgentWorkloadDecision::Deny { violations } => violations
                .into_iter()
                .map(|violation| violation.code)
                .collect::<Vec<_>>()
                .join(","),
            AgentWorkloadDecision::Unknown { missing_evidence } => missing_evidence.join(","),
            AgentWorkloadDecision::Allow => "workload_delivery_admission_unknown".to_string(),
        });
    }
    let active = store.active_records().map_err(|failure| failure.code)?;
    admit_aggregate_sibling_budget(&delivery.envelope, &active)?;
    let record = AgentWorkloadRecord {
        schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        envelope: delivery.envelope.clone(),
        state: AgentWorkloadState::Dispatched,
        claim: None,
        last_fencing_token: 0,
        cancellation_generation: 0,
        consumed_actions: 0,
        completion: None,
    };
    store
        .compare_and_swap(&delivery.envelope.workload_id, None, &record, now_unix_ms())
        .map_err(|failure| failure.code)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::local_agent_workload_completion_receipt::LocalAgentWorkloadCompletionReceipt;
    use crate::adapters::local_agent_workload_delivery_lease::LocalAgentWorkloadDeliveryLeaseStore;
    use crate::adapters::local_agent_workload_store::LocalAgentWorkloadStore;
    use cynic_build::{
    BUILD_AGENT_REQUEST_SCHEMA,
    BuildAgentRequest,
    compose_build_agent_workload,
};
use cynic_workload::{
    AGENT_WORKLOAD_DELIVERY_SCHEMA,
    AgentWorkloadDeliveryStatus,
    accept_agent_workload_delivery,
};
    use std::sync::Mutex;

    struct DeliveryStub {
        pending: Vec<AgentWorkloadDelivery>,
        accepted: Mutex<Vec<String>>,
    }

    impl AgentWorkloadDeliveryPort for DeliveryStub {
        async fn pending_deliveries(
            &self,
            _node_id: &str,
        ) -> Result<Vec<AgentWorkloadDelivery>, String> {
            Ok(self.pending.clone())
        }

        async fn accept_delivery(
            &self,
            _node_id: &str,
            workload_id: &str,
            acceptance: &AgentWorkloadDeliveryAcceptance,
        ) -> Result<AgentWorkloadDelivery, String> {
            self.accepted.lock().unwrap().push(workload_id.to_string());
            accept_agent_workload_delivery(&self.pending[0], acceptance)
                .map_err(|violations| violations.join(","))
        }

        async fn publish_completion(
            &self,
            _publication: &cynic_workload::AgentWorkloadCompletionPublication,
        ) -> Result<AgentWorkloadDelivery, String> {
            Err("not exercised".to_string())
        }
    }

    fn delivery() -> AgentWorkloadDelivery {
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "validate-test".to_string(),
            requested_by: "agent-test".to_string(),
            source_commit: "a".repeat(40),
            source: cynic_build::BuildSourceReference {
                schema: cynic_build::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".into(),
                transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".into(),
                revision: "a".repeat(40),
                authority_ref: "infra/repo-remotes.json:forgejo".into(),
                snapshot: None,
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec!["canonical_gate_report".to_string()],
            release_publication: None,
            evidence_refs: vec!["git:commit:test".to_string()],
        };
        let envelope = compose_build_agent_workload(
            &request,
            "cynic-builder",
            "/var/lib/cynic/source/workspaces",
        )
        .unwrap();
        AgentWorkloadDelivery {
            schema: AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
            envelope,
            target_node_id: "cynic-builder".to_string(),
            submitted_by: "agent-test".to_string(),
            submitted_at_ms: 1,
            status: AgentWorkloadDeliveryStatus::Pending,
            accepted_at_ms: None,
            accepted_record_digest: None,
            acceptance_lease: None,
            terminal_at_ms: None,
            terminal_record_digest: None,
            terminal_record: None,
            completion: None,
            completion_evidence: Vec::new(),
            evidence_refs: vec!["git:commit:test".to_string()],
        }
    }

    #[tokio::test]
    async fn persists_before_acknowledging_delivery() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalAgentWorkloadStore::new(root.path());
        let leases = LocalAgentWorkloadDeliveryLeaseStore::new(root.path().join("leases"));
        let receipts = LocalAgentWorkloadCompletionReceipt::new(root.path().join("receipts"));
        let delivery = delivery();
        let workload_id = delivery.envelope.workload_id.clone();
        let port = DeliveryStub {
            pending: vec![delivery],
            accepted: Mutex::new(Vec::new()),
        };

        let report = reconcile_deliveries(
            "cynic-builder",
            "builder-incarnation-a",
            10_000,
            &port,
            &store,
            &leases,
            &receipts,
        )
        .await;

        assert_eq!(report["status"], "accepted");
        assert!(store.read(&workload_id).unwrap().is_some());
        assert_eq!(port.accepted.lock().unwrap().as_slice(), &[workload_id]);
    }
}
