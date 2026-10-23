//! Bounded enrollment health check for the `node enrollment check` arm.
//!
//! Read-only: file read-back plus a short TCP probe of the renewal
//! endpoint (`kernel_url`). The probe never hangs the arm — 2s timeout,
//! IP literals only (no DNS), failure is a reported blocker, not an error.
//! Replaces the unbounded legacy check path that could stall for minutes.

use std::net::{SocketAddr, TcpStream};
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use cynic_node_protocol::summarize_node_enrollment_check;

use crate::adapters::local_node_enrollment::LocalNodeEnrollmentReader;
use crate::config;

const RENEWAL_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

/// Short TCP probe of the renewal endpoint. Returns
/// `(reachable, parse_unknowns)`: an unparsable URL is reported, never
/// panicked on, and counts as unreachable.
fn probe_renewal_endpoint(kernel_url: &str) -> (bool, Vec<String>) {
    let without_scheme = kernel_url
        .strip_prefix("https://")
        .or_else(|| kernel_url.strip_prefix("http://"))
        .unwrap_or(kernel_url)
        .split('/')
        .next()
        .unwrap_or(kernel_url)
        .trim_end_matches('/');
    match without_scheme.parse::<SocketAddr>() {
        Ok(addr) => (
            TcpStream::connect_timeout(&addr, RENEWAL_PROBE_TIMEOUT).is_ok(),
            Vec::new(),
        ),
        Err(_) => (
            false,
            vec!["node_enrollment_renewal_endpoint_not_ip_literal".to_string()],
        ),
    }
}

pub fn execute_node_enrollment_check(config_path: &str, json: bool) -> ExitCode {
    let control = match config::load_node_control(config_path) {
        Ok(control) => control,
        Err(error) => {
            let report = serde_json::json!({
                "schema": "cynic.node_enrollment.check.v1",
                "status": "blocked",
                "config_path": config_path,
                "blockers": [error],
            });
            if json {
                return emit(&report, false);
            }
            eprintln!("BLOCKED: config error");
            return ExitCode::FAILURE;
        }
    };
    let reader = LocalNodeEnrollmentReader::new(
        control.enrollment_request_path.clone(),
        control.enrollment_grant_path.clone(),
        control.enrollment_authority_public_key_path.clone(),
        control.signing_key_path.clone(),
    );
    let read_back = reader.read_back(&control.node_id);
    let (reachable, mut unknowns) = probe_renewal_endpoint(&control.kernel_url);
    unknowns.extend(read_back.unknowns.iter().cloned());
    let summary = summarize_node_enrollment_check(
        &read_back.state,
        read_back.grant_id.clone(),
        read_back.expires_at_unix_s,
        &unknowns,
        reachable,
        control.build_scheduler.is_some(),
        now_unix_s(),
    );
    let report = serde_json::json!({
        "schema": "cynic.node_enrollment.check.v1",
        "status": summary.status,
        "config_path": config_path,
        "node_id": control.node_id,
        "enrollment_proven": summary.enrollment_proven,
        "grant_id": summary.grant_id,
        "grant_expires_at_unix_s": summary.grant_expires_at_unix_s,
        "renewal_endpoint": control.kernel_url,
        "renewal_endpoint_reachable": summary.renewal_endpoint_reachable,
        "blockers": summary.blockers,
    });
    if json {
        return emit(&report, summary.status == "ready");
    }
    if summary.status == "ready" {
        println!("READY: enrollment proven for {}", control.node_id);
    } else {
        println!(
            "BLOCKED: {} ({})",
            control.node_id,
            summary.blockers.join(", ")
        );
    }
    if summary.status == "ready" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn emit(value: &impl Serialize, success: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("node enrollment check serialization failed: {error}");
            return ExitCode::FAILURE;
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
