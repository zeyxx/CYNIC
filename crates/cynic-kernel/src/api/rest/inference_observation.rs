//! Observation adapter for inference REST operations.
//!
//! The router owns HTTP orchestration. This module owns the translation from
//! inference/node events into kernel storage observations.

use cynic_inference::application::model_catalog::ModelProbeObservation;

use crate::domain::storage::{Observation, StorageError, StoragePort};

pub async fn store_recovery_observation(
    storage: &dyn StoragePort,
    node: &str,
    status: &str,
) -> Result<(), StorageError> {
    let obs = Observation {
        project: "cynic".to_string(),
        agent_id: "kernel-remediation".to_string(),
        tool: "ts_exec".to_string(),
        target: node.to_string(),
        domain: "infrastructure".to_string(),
        status: status.to_string(),
        context: "Attempted llama-server restart on degraded node".to_string(),
        session_id: "".to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        tags: vec!["k15-recovery".to_string(), "remediation".to_string()],
        source_tier: "permanent".to_string(),
        value: None,
        confidence: None,
        consumer: None,
        action: None,
        depends_on: vec![],
        maturity: None,
        hash: String::new(),
        prev_hash: String::new(),
        observers: vec![],
        consensus_score: None,
    };

    storage.store_observation(&obs).await
}

pub async fn store_probe_observation(
    storage: &dyn StoragePort,
    dog: &str,
    node: &str,
    probe: &ModelProbeObservation,
) -> Result<(), StorageError> {
    if probe.failure_reason == "none" {
        return Ok(());
    }

    let obs = Observation {
        project: "cynic".to_string(),
        agent_id: "kernel-probe".to_string(),
        tool: "llama_server_probe".to_string(),
        target: format!("{node}/{dog}"),
        domain: "infrastructure".to_string(),
        status: if probe.reachable {
            "degraded"
        } else {
            "unreachable"
        }
        .to_string(),
        context: format!(
            "Dog probe failed: failure_type={}, latency_ms={}",
            probe.failure_reason, probe.latency_ms
        ),
        session_id: "".to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        tags: vec![
            "k15-observability".to_string(),
            format!("failure_type:{}", probe.failure_reason),
        ],
        source_tier: "permanent".to_string(),
        value: None,
        confidence: None,
        consumer: None,
        action: None,
        depends_on: vec![],
        maturity: None,
        hash: String::new(),
        prev_hash: String::new(),
        observers: vec![],
        consensus_score: None,
    };

    storage.store_observation(&obs).await
}
