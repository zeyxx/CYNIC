//! `cynic network notify` — evaluate the watchdog and deliver its alerts.
//!
//! WHY THIS EXISTS. `cynic network watchdog` evaluates the mesh and enqueues one
//! alert entry per degraded component into an `InMemoryNotificationOutboxStore`.
//! That store lives for the duration of one process and is then dropped, and
//! `process_notification_outbox` — the function that actually hands entries to
//! `HttpTelegramNotificationTransport` — had no caller anywhere in the workspace.
//!
//! The consequence was silent, which is the worst kind: the watchdog reported
//! `alerts_enqueued_count: N`, the operator read N, and nothing was ever sent.
//! Every part of the chain existed except the wire between its two halves. From
//! the operator's seat, a computed-and-discarded alert is indistinguishable from
//! a healthy mesh.
//!
//! WHAT THIS SURFACE DOES. In ONE process: evaluate the same assessment, enqueue
//! the resulting alerts, then drain them through the transport. The in-memory
//! store is sufficient precisely because all three steps are here — persistence
//! would only be needed to hand entries between processes, and nothing should.
//!
//! WHAT IT DOES NOT DO. It does not decide what is alertable — that stays in
//! `evaluate_network_watchdog` / `reconcile_network_watchdog_outbox`. It does not
//! hold or echo a transport token; the credential is read from the environment at
//! send time and the transport's own Debug impl redacts it.
//!
//! AN UNCONFIGURED TRANSPORT IS NOT AN ERROR. A host without Telegram credentials
//! is a declared posture, not a failure; reporting red here would make every such
//! host permanently red and the alert meaningless. The surface reports
//! `transport_configured: false` and exits 0.

use std::process::ExitCode;

use cynic_node::usecases::network_watchdog_uc::execute_network_watchdog;
use cynic_node::usecases::notification_outbox_uc::{
    HttpTelegramNotificationTransport, InMemoryNotificationOutboxStore, process_notification_outbox,
};
use cynic_node::watchdog::peer_handshake::LocalPeerHandshakeObserver;
use serde::Serialize;

use crate::emit_json;
use crate::network_surface::declared_service_probe_specs;

#[derive(Debug, Serialize)]
struct NetworkNotifyReport {
    schema: &'static str,
    watchdog_status: String,
    alerts_enqueued: usize,
    pending_before_drain: usize,
    delivered: usize,
    failed: usize,
    dry_run: bool,
    /// False when no transport credential is present; delivery was skipped.
    transport_configured: bool,
    /// True when delivery was attempted and at least one alert failed.
    delivery_failed: bool,
}

/// Resolve the Telegram transport from the environment.
///
/// Returns `None` when either half of the pair is absent or empty. An
/// unconfigured transport is reported, never guessed at and never faked.
fn resolve_transport() -> Option<HttpTelegramNotificationTransport> {
    let token = std::env::var("CYNIC_TELEGRAM_BOT_TOKEN").ok()?;
    let chat_id = std::env::var("CYNIC_TELEGRAM_CHAT_ID").ok()?;
    if token.trim().is_empty() || chat_id.trim().is_empty() {
        return None;
    }
    HttpTelegramNotificationTransport::new(
        token,
        chat_id,
        std::time::Duration::from_secs(5),
    )
    .ok()
}

pub(crate) fn execute_network_notify(
    interface: &str,
    dry_run: bool,
    json: bool,
) -> ExitCode {
    let store = InMemoryNotificationOutboxStore::default();

    let wg_observer = LocalPeerHandshakeObserver;
    let freebox_probe = cynic_host::adapters::http_freebox_probe_adapter::HttpFreeboxProbeAdapter::default();
    let services = declared_service_probe_specs();
    let adapter = cynic_host::adapters::local_network::LocalNetworkAdapter;
    let pressure = {
        use cynic_host::ports::network::NetworkObservationPort as _;
        adapter.observe_network().pressure
    };

    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let report = match execute_network_watchdog(
        interface,
        &wg_observer,
        &services,
        None,
        Some(&freebox_probe),
        pressure,
        &store,
        now_unix_ms,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("network notify: watchdog evaluation failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let pending_before_drain = store.list_pending().map(|p| p.len()).unwrap_or(0);

    // The dry run stops after enqueue and before delivery, so the exact payload
    // and the exact count are observable without sending anything.
    if dry_run {
        let out = NetworkNotifyReport {
            schema: "cynic.network_notify.v1",
            watchdog_status: report.status.clone(),
            alerts_enqueued: report.alerts_enqueued_count,
            pending_before_drain,
            delivered: 0,
            failed: 0,
            dry_run: true,
            transport_configured: resolve_transport().is_some(),
            delivery_failed: false,
        };
        if json {
            return emit_json(&out, false);
        }
        println!(
            "DRY RUN — watchdog={} alerts_enqueued={} pending={} (nothing delivered)",
            out.watchdog_status, out.alerts_enqueued, pending_before_drain
        );
        for alert in &report.assessment.alerts {
            println!(
                "  [{:?}] {}: {} -> {}",
                alert.level, alert.component, alert.message, alert.recovery_hint
            );
        }
        return ExitCode::SUCCESS;
    }

    let Some(transport) = resolve_transport() else {
        let out = NetworkNotifyReport {
            schema: "cynic.network_notify.v1",
            watchdog_status: report.status.clone(),
            alerts_enqueued: report.alerts_enqueued_count,
            pending_before_drain,
            delivered: 0,
            failed: 0,
            dry_run: false,
            transport_configured: false,
            delivery_failed: false,
        };
        if json {
            return emit_json(&out, false);
        }
        println!(
            "watchdog={} alerts_enqueued={} — transport not configured, nothing sent",
            out.watchdog_status, out.alerts_enqueued
        );
        println!("set CYNIC_TELEGRAM_BOT_TOKEN and CYNIC_TELEGRAM_CHAT_ID to deliver");
        return ExitCode::SUCCESS;
    };

    let drain = match process_notification_outbox(&store, &transport) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("network notify: outbox delivery failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let out = NetworkNotifyReport {
        schema: "cynic.network_notify.v1",
        watchdog_status: report.status.clone(),
        alerts_enqueued: report.alerts_enqueued_count,
        pending_before_drain,
        delivered: drain.delivered_count,
        failed: drain.failed_count,
        dry_run: false,
        transport_configured: true,
        delivery_failed: drain.failed_count > 0,
    };

    if json {
        emit_json(&out, out.delivery_failed)
    } else {
        println!(
            "watchdog={} alerts_enqueued={} delivered={} failed={}",
            out.watchdog_status, out.alerts_enqueued, out.delivered, out.failed
        );
        if out.delivery_failed {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}
