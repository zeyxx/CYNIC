//! Fleet heartbeat port — the interface between the pure decision and
//! the runtime substrate.

use cynic_protocol::fleet_heartbeat::FleetHeartbeat;

/// Port for receiving and storing heartbeats.
pub trait FleetHeartbeatPort {
    /// Receive a heartbeat from a node.
    fn receive_heartbeat(&mut self, heartbeat: FleetHeartbeat);
    /// Get all stored heartbeats.
    fn heartbeats(&self) -> &[FleetHeartbeat];
    /// Clear all stored heartbeats.
    fn clear(&mut self);
}

/// In-memory heartbeat store — runtime implementation.
pub struct InMemoryHeartbeatStore {
    heartbeats: Vec<FleetHeartbeat>,
    capacity: usize,
}

impl InMemoryHeartbeatStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            heartbeats: Vec::new(),
            capacity,
        }
    }
}

impl FleetHeartbeatPort for InMemoryHeartbeatStore {
    fn receive_heartbeat(&mut self, heartbeat: FleetHeartbeat) {
        // Replace any existing heartbeat from the same node.
        self.heartbeats.retain(|h| h.node_id != heartbeat.node_id);
        self.heartbeats.push(heartbeat);
        // Enforce capacity — drop oldest.
        if self.heartbeats.len() > self.capacity {
            self.heartbeats.remove(0);
        }
    }

    fn heartbeats(&self) -> &[FleetHeartbeat] {
        &self.heartbeats
    }

    fn clear(&mut self) {
        self.heartbeats.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet_heartbeat::FleetNodeStatus;

    fn hb(id: &str, at: u64) -> FleetHeartbeat {
        FleetHeartbeat {
            node_id: id.into(),
            emitted_at_unix_s: at,
            self_status: FleetNodeStatus::Alive,
            uptime_s: 100,
            unknowns: vec![],
        }
    }

    #[test]
    fn store_keeps_latest_per_node() {
        let mut store = InMemoryHeartbeatStore::new(10);
        store.receive_heartbeat(hb("a", 100));
        store.receive_heartbeat(hb("a", 200));
        assert_eq!(store.heartbeats().len(), 1);
        assert_eq!(store.heartbeats()[0].emitted_at_unix_s, 200);
    }

    #[test]
    fn store_enforces_capacity() {
        let mut store = InMemoryHeartbeatStore::new(2);
        store.receive_heartbeat(hb("a", 100));
        store.receive_heartbeat(hb("b", 200));
        store.receive_heartbeat(hb("c", 300));
        assert_eq!(store.heartbeats().len(), 2);
        assert_eq!(store.heartbeats()[0].node_id, "b");
    }
}
