//! Network watchdog alert reconciliation to NotificationOutbox.

use crate::types::{NetworkAlertLevel, NetworkWatchdogAssessment};
use cynic_protocol::{
    NOTIFICATION_OUTBOX_ENTRY_SCHEMA, 
    NotificationChannel, NotificationContentClassification, NotificationDeliveryStatus,
    NotificationOutboxEntry, NotificationReferencedEvent, NotificationReferencedEventKind,
};

/// Pure function reconciling a watchdog assessment into durable notification outbox entries.
pub fn reconcile_network_watchdog_outbox(
    assessment: &NetworkWatchdogAssessment,
) -> Vec<NotificationOutboxEntry> {
    let mut entries = Vec::new();

    let minute_epoch = assessment.observed_at_unix_ms / 60_000;

    for (idx, alert) in assessment.alerts.iter().enumerate() {
        if alert.level == NetworkAlertLevel::Ok {
            continue;
        }

        let level_str = match alert.level {
            NetworkAlertLevel::Ok => "OK",
            NetworkAlertLevel::Warning => "WARNING",
            NetworkAlertLevel::Critical => "CRITICAL",
        };

        let notification_id = format!("notif:net:{}:{}:{}", alert.component, minute_epoch, idx);

        let idempotency_key = format!(
            "net_alert:{}:{}:{}",
            alert.component, level_str, minute_epoch
        );

        let summary = format!(
            "[{level_str}] Network alert on '{}': {}. Recovery: {}",
            alert.component, alert.message, alert.recovery_hint
        );

        let entry = NotificationOutboxEntry {
            schema: NOTIFICATION_OUTBOX_ENTRY_SCHEMA.to_string(),
            notification_id,
            referenced_event: NotificationReferencedEvent {
                kind: NotificationReferencedEventKind::WorkloadBlocked,
                event_id: format!(
                    "net_event:{}:{}",
                    alert.component, assessment.observed_at_unix_ms
                ),
                workload_id: "network_watchdog".to_string(),
                commit_sha: None,
                build_verdict: None,
                proof_ref: Some(format!(
                    "watchdog://assessment/{}",
                    assessment.observed_at_unix_ms
                )),
            },
            channel: NotificationChannel::Telegram,
            classification: NotificationContentClassification::OperatorPrivate,
            idempotency_key,
            summary,
            delivery_status: NotificationDeliveryStatus::Pending,
            delivery_receipt: None,
            created_at_unix_ms: assessment.observed_at_unix_ms,
        };

        entries.push(entry);
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        DnsSynapseAssessment, NETWORK_WATCHDOG_ASSESSMENT_SCHEMA, NetworkAlert,
        NetworkAlertLevel, NetworkWatchdogAssessment, WireguardMeshAssessment,
    };

    #[test]
    fn assessment_with_alerts_generates_valid_outbox_entries() {
        let assessment = NetworkWatchdogAssessment {
            schema: NETWORK_WATCHDOG_ASSESSMENT_SCHEMA.to_string(),
            status: "critical".to_string(),
            level: NetworkAlertLevel::Critical,
            wireguard_mesh: WireguardMeshAssessment {
                total_peers: 4,
                ok_peers: 3,
                stale_peers: 0,
                dead_peers: 1,
                never_seen_peers: 0,
            },
            dns_synapse: DnsSynapseAssessment {
                zone: "synapse.cynic".to_string(),
                total_records: 6,
                resolved_records: 6,
                failed_records: 0,
                failures: vec![],
            },
            service_ports: vec![],
            pressure: vec![],
            freebox_lan: None,
            alerts: vec![NetworkAlert {
                level: NetworkAlertLevel::Critical,
                component: "wireguard_mesh".to_string(),
                message: "1 WireGuard peer dead".to_string(),
                recovery_hint: "Check node status".to_string(),
            }],
            observed_at_unix_ms: 1788267750000,
        };

        let outbox = reconcile_network_watchdog_outbox(&assessment);
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].channel, NotificationChannel::Telegram);
        assert!(
            outbox[0]
                .summary
                .contains("[CRITICAL] Network alert on 'wireguard_mesh'")
        );
        assert!(cynic_protocol::validate_notification_outbox_entry(&outbox[0]).is_empty());
    }

    #[test]
    fn assessment_without_alerts_generates_empty_outbox() {
        let assessment = NetworkWatchdogAssessment {
            schema: NETWORK_WATCHDOG_ASSESSMENT_SCHEMA.to_string(),
            status: "ok".to_string(),
            level: NetworkAlertLevel::Ok,
            wireguard_mesh: WireguardMeshAssessment {
                total_peers: 4,
                ok_peers: 4,
                stale_peers: 0,
                dead_peers: 0,
                never_seen_peers: 0,
            },
            dns_synapse: DnsSynapseAssessment {
                zone: "synapse.cynic".to_string(),
                total_records: 6,
                resolved_records: 6,
                failed_records: 0,
                failures: vec![],
            },
            service_ports: vec![],
            pressure: vec![],
            freebox_lan: None,
            alerts: vec![],
            observed_at_unix_ms: 1788267750000,
        };

        let outbox = reconcile_network_watchdog_outbox(&assessment);
        assert!(outbox.is_empty());
    }
}
