//! Fleet capacity alerts as notification outbox entries.
//!
//! WHY: a muted or degraded runner must reach the operator, not sit in a
//! registry file. This module turns supervised usability into durable,
//! idempotent outbox entries. Detection stays automatic; delivery rides the
//! existing notify drain, remediation stays signed.
//!
//! HOW: pure function (no I/O). The caller supplies per-runner usability
//! with reasons; this module returns one entry per non-usable runner.
//!
//! NEGATIVE BOUNDARY: does not read or write files, does not perform network
//! I/O, does not send anything. Entries only.

use cynic_protocol::runner_capacity::RunnerUsability;
use cynic_protocol::{
    NOTIFICATION_OUTBOX_ENTRY_SCHEMA, NotificationChannel, NotificationContentClassification,
    NotificationDeliveryStatus, NotificationOutboxEntry, NotificationReferencedEvent,
    NotificationReferencedEventKind,
};

/// Reconcile fleet usability into outbox entries: one per muted/degraded runner.
///
/// `usability` pairs runner ids with their decided usability and the
/// observed time in milliseconds. Usable runners produce nothing.
pub fn reconcile_fleet_outbox(
    usability: &[(String, RunnerUsability)],
    observed_at_unix_ms: u64,
) -> Vec<NotificationOutboxEntry> {
    let mut entries = Vec::new();

    for (runner_id, state) in usability {
        let (level, reason) = match state {
            RunnerUsability::Usable => continue,
            RunnerUsability::Degraded(reason) => ("WARNING", reason.clone()),
            RunnerUsability::Muted(reason) => ("CRITICAL", reason.clone()),
        };

        let idempotency_key = format!("fleet_capacity:{runner_id}:{level}");
        let summary = format!(
            "[{level}] Runner '{runner_id}' {level_str}: {reason}. Recovery: run cynic fleet observe",
            level_str = level.to_lowercase(),
        );

        entries.push(NotificationOutboxEntry {
            schema: NOTIFICATION_OUTBOX_ENTRY_SCHEMA.to_string(),
            notification_id: format!("notif:fleet:{runner_id}:{observed_at_unix_ms}"),
            referenced_event: NotificationReferencedEvent {
                kind: NotificationReferencedEventKind::WorkloadBlocked,
                event_id: format!("fleet_event:{runner_id}:{observed_at_unix_ms}"),
                workload_id: "fleet_supervisor".to_string(),
                commit_sha: None,
                build_verdict: None,
                proof_ref: Some(format!("fleet://capacity/{runner_id}")),
            },
            channel: NotificationChannel::Telegram,
            classification: NotificationContentClassification::OperatorPrivate,
            idempotency_key,
            summary,
            delivery_status: NotificationDeliveryStatus::Pending,
            delivery_receipt: None,
            created_at_unix_ms: observed_at_unix_ms,
        });
    }

    entries
}

#[cfg(test)]
mod fleet_outbox_tests {
    use super::*;

    #[test]
    fn one_entry_per_muted_runner() {
        let usability = vec![
            (
                "builder".to_string(),
                RunnerUsability::Muted("stale snapshot".to_string()),
            ),
            (
                "compute-01".to_string(),
                RunnerUsability::Muted("unmeasured disk".to_string()),
            ),
            ("spare".to_string(), RunnerUsability::Usable),
        ];
        let entries = reconcile_fleet_outbox(&usability, 1_000_000_000);
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .all(|e| e.delivery_status == NotificationDeliveryStatus::Pending)
        );
        assert!(
            entries
                .iter()
                .all(|e| e.channel == NotificationChannel::Telegram)
        );
    }

    #[test]
    fn degraded_is_warning_usable_is_silent() {
        let usability = vec![
            (
                "builder".to_string(),
                RunnerUsability::Degraded("below floor".to_string()),
            ),
            ("spare".to_string(), RunnerUsability::Usable),
        ];
        let entries = reconcile_fleet_outbox(&usability, 1_000_000_000);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].summary.contains("[WARNING]"));
        assert!(entries[0].summary.contains("cynic fleet observe"));
    }
}
