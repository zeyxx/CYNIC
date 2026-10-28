pub use cynic_network::usecases::network_watchdog_uc;
pub use cynic_network::usecases::network_watchdog_uc::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecases::notification_outbox_uc::InMemoryNotificationOutboxStore;
    use crate::watchdog::peer_handshake::{
        HandshakeState, PeerHandshakeStatus, WireguardHandshakeObservation,
    };
    use cynic_network::NotificationOutboxStorePort;

    struct MockWgObserver {
        stale: usize,
        dead: usize,
    }

    impl crate::watchdog::peer_handshake::PeerHandshakePort for MockWgObserver {
        fn observe(
            &self,
            interface: &str,
            _stale_s: u64,
            _dead_s: u64,
        ) -> WireguardHandshakeObservation {
            let mut peers = Vec::new();
            if self.dead > 0 {
                peers.push(PeerHandshakeStatus {
                    public_key_prefix: "deadpeer123".to_string(),
                    last_handshake_unix_s: 0,
                    age_s: 300,
                    state: HandshakeState::Dead,
                });
            }
            WireguardHandshakeObservation {
                schema: crate::watchdog::peer_handshake::PEER_HANDSHAKE_SCHEMA,
                interface: interface.to_string(),
                observed_at_unix_s: 1788267750,
                peers,
                stale_count: self.stale,
                dead_count: self.dead,
                ok_count: 3,
                unknown_count: 0,
            }
        }
    }

    #[test]
    fn execute_network_watchdog_healthy_enqueues_zero_alerts() {
        let wg = MockWgObserver { stale: 0, dead: 0 };
        let store = InMemoryNotificationOutboxStore::default();
        let report =
            execute_network_watchdog("wg0", &wg, &[], None, None, vec![], &store, 1788267750000)
                .unwrap();

        assert_eq!(report.status, "ok");
        assert_eq!(report.alerts_enqueued_count, 0);
        assert!(store.list_pending().unwrap().is_empty());
    }

    #[test]
    fn execute_network_watchdog_dead_peer_enqueues_alert() {
        let wg = MockWgObserver { stale: 0, dead: 1 };
        let store = InMemoryNotificationOutboxStore::default();
        let report =
            execute_network_watchdog("wg0", &wg, &[], None, None, vec![], &store, 1788267750000)
                .unwrap();

        assert_eq!(report.status, "critical");
        assert_eq!(report.alerts_enqueued_count, 1);
        let pending = store.list_pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert!(pending[0].summary.contains("WireGuard"));
    }
}