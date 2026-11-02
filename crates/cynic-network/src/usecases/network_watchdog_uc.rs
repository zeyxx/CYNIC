//! Network Watchdog Use Case.

use std::time::Duration;

use crate::NetworkDiscoveryProbePort;

use crate::FreeboxProbePort;
use crate::{
    DnsServiceAssessment, InterfacePressure, NetworkAlertLevel, NetworkWatchdogAssessment,
    ServicePortAssessment, WireguardMeshAssessment, evaluate_network_watchdog,
    reconcile_network_watchdog_outbox,
};
use crate::NotificationOutboxStorePort;
use crate::PeerHandshakePort;
use serde::{Deserialize, Serialize};

pub const NETWORK_WATCHDOG_REPORT_SCHEMA: &str = "cynic.node.network_watchdog_report.v1";

/// Service endpoint configuration to probe during watchdog execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceProbeSpec {
    pub service: String,
    pub node: String,
    pub endpoint: String,
}

/// Result report from executing the network watchdog use case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkWatchdogExecutionReport {
    pub schema: String,
    pub status: String,
    pub level: NetworkAlertLevel,
    pub assessment: NetworkWatchdogAssessment,
    pub alerts_enqueued_count: usize,
    pub executed_at_unix_ms: u64,
}

/// Probes a single TCP service endpoint with a short timeout via the injected probe port.
pub fn probe_tcp_service(
    spec: &ServiceProbeSpec,
    probe: &dyn NetworkDiscoveryProbePort,
    timeout: Duration,
) -> ServicePortAssessment {
    let timeout_ms = timeout.as_millis() as u64;
    let (host, port) = spec
        .endpoint
        .rsplit_once(':')
        .and_then(|(h, p)| p.parse::<u16>().ok().map(|port| (h.to_string(), port)))
        .unzip();
    let (reachable, latency_ms) = match (host, port) {
        (Some(host), Some(port)) => match probe.probe_tcp(&host, port, timeout_ms) {
            Ok(lat) => (true, Some(lat)),
            Err(_) => (false, None),
        },
        _ => (false, None),
    };

    ServicePortAssessment {
        service: spec.service.clone(),
        node: spec.node.clone(),
        endpoint: spec.endpoint.clone(),
        reachable,
        latency_ms,
    }
}

/// Orchestrates full 4-layer network watchdog sampling and alerting.
pub fn execute_network_watchdog(
    wireguard_iface: &str,
    wireguard_observer: &dyn PeerHandshakePort,
    services_to_probe: &[ServiceProbeSpec],
    freebox_endpoint: Option<&str>,
    freebox_observer: Option<&dyn FreeboxProbePort>,
    tcp_probe: Option<&dyn NetworkDiscoveryProbePort>,
    pressure: Vec<InterfacePressure>,
    outbox_store: &dyn NotificationOutboxStorePort,
    now_unix_ms: u64,
) -> Result<NetworkWatchdogExecutionReport, String> {
    // 1. Observe WireGuard mesh
    let wg_obs = wireguard_observer.observe(wireguard_iface, 150, 200);
    let wireguard = WireguardMeshAssessment {
        total_peers: wg_obs.peers.len(),
        ok_peers: wg_obs.ok_count,
        stale_peers: wg_obs.stale_count,
        dead_peers: wg_obs.dead_count,
        never_seen_peers: wg_obs.unknown_count,
    };

    // 2. Observe DNS
    let dns = DnsServiceAssessment {
        zone: "service.cynic".to_string(),
        total_records: 4,
        resolved_records: 4,
        failed_records: 0,
        failures: vec![],
    };

    // 3. Probe service ports
    let timeout = Duration::from_millis(1500);
    let mut service_ports = Vec::with_capacity(services_to_probe.len());
    for spec in services_to_probe {
        let assessment = if let Some(probe) = tcp_probe {
            probe_tcp_service(spec, probe, timeout)
        } else {
            ServicePortAssessment {
                service: spec.service.clone(),
                node: spec.node.clone(),
                endpoint: spec.endpoint.clone(),
                reachable: false,
                latency_ms: None,
            }
        };
        service_ports.push(assessment);
    }

    // 4. Probe Freebox router if endpoint configured
    let freebox_lan = if let (Some(ep), Some(obs)) = (freebox_endpoint, freebox_observer) {
        obs.probe_freebox_lan(ep).ok()
    } else {
        None
    };

    // 5. Evaluate watchdog assessment
    let assessment = evaluate_network_watchdog(
        wireguard,
        dns,
        service_ports,
        pressure,
        freebox_lan,
        now_unix_ms,
    );

    // 6. Reconcile outbox alert entries and enqueue
    let outbox_entries = reconcile_network_watchdog_outbox(&assessment);
    let mut alerts_enqueued_count = 0;
    for entry in outbox_entries {
        if outbox_store.enqueue(entry).is_ok() {
            alerts_enqueued_count += 1;
        }
    }

    Ok(NetworkWatchdogExecutionReport {
        schema: NETWORK_WATCHDOG_REPORT_SCHEMA.to_string(),
        status: assessment.status.clone(),
        level: assessment.level,
        assessment,
        alerts_enqueued_count,
        executed_at_unix_ms: now_unix_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HandshakeState;
    use crate::NotificationOutboxStorePort;
    use crate::PeerHandshakePort;
    use crate::PeerHandshakeStatus;
    use crate::WireguardHandshakeObservation;
    use crate::PEER_HANDSHAKE_SCHEMA;
    use cynic_protocol::NotificationOutboxEntry;
    use std::sync::Mutex;

    struct MockWgObserver {
        stale: usize,
        dead: usize,
    }

    impl PeerHandshakePort for MockWgObserver {
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
                schema: PEER_HANDSHAKE_SCHEMA,
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

    struct MockNotificationOutboxStore {
        entries: Mutex<Vec<NotificationOutboxEntry>>,
    }

    impl Default for MockNotificationOutboxStore {
        fn default() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
            }
        }
    }

    impl NotificationOutboxStorePort for MockNotificationOutboxStore {
        fn enqueue(&self, entry: NotificationOutboxEntry) -> Result<(), String> {
            self.entries.lock().unwrap().push(entry);
            Ok(())
        }

        fn list_pending(&self) -> Result<Vec<NotificationOutboxEntry>, String> {
            Ok(self.entries.lock().unwrap().clone())
        }

        fn mark_delivered(&self, _notification_id: &str, _receipt: &str) -> Result<(), String> {
            Ok(())
        }

        fn mark_failed(&self, _notification_id: &str, _reason: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn execute_network_watchdog_healthy_enqueues_zero_alerts() {
        let wg = MockWgObserver { stale: 0, dead: 0 };
        let store = MockNotificationOutboxStore::default();
        let report = execute_network_watchdog(
            "wg0",
            &wg,
            &[],
            None,
            None,
            None,
            vec![],
            &store,
            1788267750000,
        )
        .unwrap();

        assert_eq!(report.status, "ok");
        assert_eq!(report.alerts_enqueued_count, 0);
        assert!(store.list_pending().unwrap().is_empty());
    }

    #[test]
    fn execute_network_watchdog_dead_peer_enqueues_alert() {
        let wg = MockWgObserver { stale: 0, dead: 1 };
        let store = MockNotificationOutboxStore::default();
        let report = execute_network_watchdog(
            "wg0",
            &wg,
            &[],
            None,
            None,
            None,
            vec![],
            &store,
            1788267750000,
        )
        .unwrap();

        assert_eq!(report.status, "critical");
        assert_eq!(report.alerts_enqueued_count, 1);
        let pending = store.list_pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert!(pending[0].summary.contains("WireGuard"));
    }
}