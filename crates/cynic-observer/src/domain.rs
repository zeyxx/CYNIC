//! Observation Domain Models — Multi-Scale Universe of CYNIC Observation
//!
//! Extends observation beyond a single chain (Solana) to the 5 canonical dimensions:
//! 1. Blockchain (Solana, EVM, L2s)
//! 2. Fleet & Mesh (Nodes, P2P discovery, WireGuard, coordination leases)
//! 3. System & Metabolism (CPU, RAM, swap, WAL, disk, kernel metrics)
//! 4. Inference & Models (Local models, latency, quality, token usage, drift)
//! 5. Offchain & Social (Forgejo, Git, X/Twitter, Web resources)

#[allow(unused_imports)]
use cynic_protocol::{EvidenceEnvelope, build_evidence_envelope};
use serde::{Deserialize, Serialize};

pub const OBSERVATION_EVENT_SCHEMA: &str = "cynic.observation.event.v1";

/// The 5 fundamental domains of CYNIC observation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "domain_type", rename_all = "snake_case")]
pub enum ObservationDomain {
    /// Onchain events, transactions, liquidity flows, and program interactions.
    Blockchain {
        chain: String,
        block_or_slot: Option<u64>,
        identifier: String,
        program_or_contract: Option<String>,
    },
    /// Fleet topology, P2P gossip, lease fencing, and node health.
    Fleet {
        node_id: String,
        role: String,
        event_kind: String,
        peer_count: Option<u64>,
    },
    /// Host OS, hardware metrics, memory pressure, and WAL integrity.
    System {
        host: String,
        component: String,
        metric_name: String,
    },
    /// Sovereign inference backends, local execution, quality, and latency.
    Inference {
        model: String,
        backend: String,
        latency_ms: Option<u64>,
    },
    /// Offchain feeds, sovereign Git repos, web storefronts, and social signals.
    OffchainSocial {
        platform: String,
        channel_or_user: String,
        resource_locator: Option<String>,
    },
}

/// Universal observation envelope carrying multi-domain events with provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UniversalObserverEvent {
    pub schema: String,
    pub id: String,
    pub domain: ObservationDomain,
    pub timestamp_unix_ms: u128,
    pub payload: serde_json::Value,
    pub provenance: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

impl UniversalObserverEvent {
    pub fn new(
        id: impl Into<String>,
        domain: ObservationDomain,
        payload: serde_json::Value,
        provenance: Vec<String>,
    ) -> Self {
        let timestamp_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        Self {
            schema: OBSERVATION_EVENT_SCHEMA.to_string(),
            id: id.into(),
            domain,
            timestamp_unix_ms,
            payload,
            provenance,
            metadata: None,
        }
    }

    /// Converts this universal event into a cryptographic EvidenceEnvelope.
    pub fn to_evidence_envelope(&self, producer: &str) -> Result<EvidenceEnvelope, String> {
        let subject = format!("{:?}:{}", self.domain, self.id);
        build_evidence_envelope(
            &subject,
            producer,
            self.timestamp_unix_ms,
            self.provenance.clone(),
            &self.payload,
        )
    }

    /// Creates a blockchain event from legacy ObserverEvent.
    pub fn from_legacy_observer_event(event: &crate::ObserverEvent, chain: &str) -> Self {
        let domain = ObservationDomain::Blockchain {
            chain: chain.to_string(),
            block_or_slot: Some(event.slot),
            identifier: event.signature.clone(),
            program_or_contract: Some(event.program_id.clone()),
        };
        let payload = serde_json::json!({
            "signature": event.signature,
            "slot": event.slot,
            "program_id": event.program_id,
            "payload_bytes": event.payload.len(),
        });
        Self::new(
            event.signature.clone(),
            domain,
            payload,
            vec![format!("rpc:{chain}")],
        )
    }

    /// Creates a fleet observation event.
    pub fn fleet_event(
        node_id: &str,
        role: &str,
        event_kind: &str,
        peer_count: u64,
        payload: serde_json::Value,
    ) -> Self {
        let id = format!("{node_id}:{event_kind}:{}", Self::now_ms());
        let domain = ObservationDomain::Fleet {
            node_id: node_id.to_string(),
            role: role.to_string(),
            event_kind: event_kind.to_string(),
            peer_count: Some(peer_count),
        };
        Self::new(id, domain, payload, vec![format!("node:{node_id}")])
    }

    /// Creates a system / metabolism observation event.
    pub fn system_event(
        host: &str,
        component: &str,
        metric_name: &str,
        payload: serde_json::Value,
    ) -> Self {
        let id = format!("{host}:{component}:{metric_name}:{}", Self::now_ms());
        let domain = ObservationDomain::System {
            host: host.to_string(),
            component: component.to_string(),
            metric_name: metric_name.to_string(),
        };
        Self::new(id, domain, payload, vec![format!("host:{host}")])
    }

    /// Creates an inference observation event.
    pub fn inference_event(
        model: &str,
        backend: &str,
        latency_ms: u64,
        payload: serde_json::Value,
    ) -> Self {
        let id = format!("{model}:{backend}:{}", Self::now_ms());
        let domain = ObservationDomain::Inference {
            model: model.to_string(),
            backend: backend.to_string(),
            latency_ms: Some(latency_ms),
        };
        Self::new(id, domain, payload, vec![format!("backend:{backend}")])
    }

    fn now_ms() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universal_event_roundtrips_and_converts_to_evidence() {
        let event = UniversalObserverEvent::system_event(
            "node-cynic-01",
            "memory",
            "usage_bytes",
            serde_json::json!({"used_mb": 4096, "total_mb": 16384}),
        );
        assert_eq!(event.schema, OBSERVATION_EVENT_SCHEMA);
        let evidence = event
            .to_evidence_envelope("cynic-observer")
            .expect("evidence envelope");
        assert_eq!(evidence.schema, cynic_protocol::EVIDENCE_ENVELOPE_SCHEMA);
        assert!(!evidence.digest.is_empty());
    }

    #[test]
    fn fleet_event_carries_peer_count() {
        let event = UniversalObserverEvent::fleet_event(
            "argo-01",
            "observer",
            "peer_joined",
            5,
            serde_json::json!({"peer_id": "12D3KooW..."}),
        );
        match event.domain {
            ObservationDomain::Fleet { peer_count, .. } => {
                assert_eq!(peer_count, Some(5));
            }
            _ => panic!("expected fleet domain"),
        }
    }
}
