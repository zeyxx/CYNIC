//! Event Source Ports & Adapters — Multi-Domain Ingestion
//!
//! Defines the universal ingestion port and pluggable adapters for
//! all 5 observation dimensions:
//! - Blockchain
//! - Fleet
//! - System
//! - Inference
//! - Social/Offchain

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::metabolism::{LoadProbe, OsLoadProbe};
use async_trait::async_trait;

use crate::domain::UniversalObserverEvent;

/// Universal trait for any observation source in the organism.
#[async_trait]
pub trait EventSourcePort: Send + Sync {
    /// Human-readable identifier of the source.
    fn name(&self) -> &str;

    /// Primary observation domain category.
    fn domain_name(&self) -> &str;

    /// Polls or collects the latest batch of observation events.
    async fn poll_events(&mut self) -> Result<Vec<UniversalObserverEvent>, String>;
}

pub struct SystemMetabolismSource {
    host_name: String,
    metabolism: OsLoadProbe,
}

impl std::fmt::Debug for SystemMetabolismSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemMetabolismSource")
            .field("host_name", &self.host_name)
            .finish()
    }
}

impl SystemMetabolismSource {
    pub fn new(host_name: impl Into<String>) -> Self {
        Self {
            host_name: host_name.into(),
            metabolism: OsLoadProbe::new(),
        }
    }
}

#[async_trait]
impl EventSourcePort for SystemMetabolismSource {
    fn name(&self) -> &str {
        "system_metabolism_source"
    }

    fn domain_name(&self) -> &str {
        "system"
    }

    async fn poll_events(&mut self) -> Result<Vec<UniversalObserverEvent>, String> {
        let state = self.metabolism.current_load();
        let payload = serde_json::json!({
            "load_tier": format!("{state:?}"),
            "status": "observed",
        });

        let event = UniversalObserverEvent::system_event(
            &self.host_name,
            "metabolism",
            "state_snapshot",
            payload,
        );

        Ok(vec![event])
    }
}

/// Fleet & P2P mesh source adapter (peer count, discovery).
#[derive(Debug)]
pub struct FleetMeshSource {
    node_id: String,
    role: String,
    connected_peers: Arc<AtomicU64>,
}

impl FleetMeshSource {
    pub fn new(
        node_id: impl Into<String>,
        role: impl Into<String>,
        connected_peers: Arc<AtomicU64>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            role: role.into(),
            connected_peers,
        }
    }
}

#[async_trait]
impl EventSourcePort for FleetMeshSource {
    fn name(&self) -> &str {
        "fleet_mesh_source"
    }

    fn domain_name(&self) -> &str {
        "fleet"
    }

    async fn poll_events(&mut self) -> Result<Vec<UniversalObserverEvent>, String> {
        let peers = self.connected_peers.load(Ordering::Relaxed);
        let payload = serde_json::json!({
            "connected_peers": peers,
            "node_role": &self.role,
        });

        let event = UniversalObserverEvent::fleet_event(
            &self.node_id,
            &self.role,
            "peer_count_sample",
            peers,
            payload,
        );

        Ok(vec![event])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ObservationDomain;

    #[tokio::test]
    async fn system_metabolism_source_polls_events() {
        let mut source = SystemMetabolismSource::new("argo-node-01");
        let events = source.poll_events().await.expect("poll events");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].domain, ObservationDomain::System { .. }));
    }

    #[tokio::test]
    async fn fleet_mesh_source_reflects_peer_count() {
        let peers = Arc::new(AtomicU64::new(8));
        let mut source = FleetMeshSource::new("argo-01", "observer", peers.clone());
        let events = source.poll_events().await.expect("poll events");
        assert_eq!(events.len(), 1);
        match events[0].domain {
            ObservationDomain::Fleet { peer_count, .. } => {
                assert_eq!(peer_count, Some(8));
            }
            _ => panic!("expected fleet domain"),
        }
    }
}
