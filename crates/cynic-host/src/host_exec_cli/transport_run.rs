//! `cynic-host-exec transport-run` — execute a declared transport capability
//! (scp/ssh/git) with registry-resolved endpoints and exact-argv rendering.
//!
//! Thin glue over `crate::privilege::transport_caps`. The agent never writes
//! an ad-hoc scp/ssh line again: it names a capability id and passes typed
//! params. Deny-by-default: unknown capability, missing/extra param, unsafe
//! value, unresolved endpoint, or HumanAdmin class without consent → blocked.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::host_exec_cli::args;
use crate::privilege::{
    AuthorityClass, DEFAULT_TRANSPORT_POLICY_PATH, TransportCapabilities, TransportPolicyError,
};

const TRANSPORT_RUN_REPORT_SCHEMA: &str = "cynic.host.transport_run_report.v1";

#[derive(Debug, Serialize)]
struct TransportRunReport {
    schema: &'static str,
    capability: String,
    status: String,
    argv: Vec<String>,
    endpoint_resolved: Option<String>,
    authority_class: String,
    blockers: Vec<String>,
}

/// Build the node → ssh-target view from the fleet registry
/// (`nodes[].transports.ssh.target`). Falls back to the documented ssh config
/// aliases when the registry entry lacks a target — recorded as override, not
/// silently.
fn registry_ssh_view(
    registry_path: &Path,
) -> Result<BTreeMap<String, String>, TransportPolicyError> {
    let raw = std::fs::read_to_string(registry_path).map_err(|error| {
        TransportPolicyError(format!(
            "read registry {}: {error}",
            registry_path.display()
        ))
    })?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| TransportPolicyError(format!("parse registry: {error}")))?;
    let mut view = BTreeMap::new();
    if let Some(nodes) = value.get("nodes").and_then(serde_json::Value::as_array) {
        for node in nodes {
            let Some(id) = node.get("id").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if let Some(target) = node
                .pointer("/transports/ssh/target")
                .and_then(serde_json::Value::as_str)
            {
                view.insert(id.to_string(), target.to_string());
            }
        }
    }
    Ok(view)
}

pub(crate) fn execute_transport_run(
    args_slice: &[String],
) -> Result<std::process::ExitCode, String> {
    let mut report = TransportRunReport {
        schema: TRANSPORT_RUN_REPORT_SCHEMA,
        capability: String::new(),
        status: "blocked".to_string(),
        argv: Vec::new(),
        endpoint_resolved: None,
        authority_class: String::new(),
        blockers: Vec::new(),
    };

    // Shape: transport-run --capability <id> [--param name=value]... [--apply]
    //        [--policy <path>] [--registry <path>] [name=value ...]
    let Some(capability_id) = args::flag_value(args_slice, "--capability") else {
        return Err("transport-run requires --capability <id>".to_string());
    };
    report.capability = capability_id.clone();
    let apply = args::has_flag(args_slice, "--apply");
    let policy_path = args::flag_value(args_slice, "--policy")
        .unwrap_or_else(|| DEFAULT_TRANSPORT_POLICY_PATH.to_string());

    let Ok(policy) = TransportCapabilities::load(&policy_path) else {
        report
            .blockers
            .push(format!("policy_unreadable_or_invalid: {policy_path}"));
        return finish(report);
    };

    let Ok(capability) = policy.capability(&capability_id) else {
        report
            .blockers
            .push(format!("unknown_capability: {capability_id}"));
        return finish(report);
    };
    report.authority_class = format!("{:?}", capability.authority_class);

    // HumanAdmin bootstrap capabilities need explicit per-run consent.
    if capability.authority_class == AuthorityClass::HumanAdminBootstrap {
        let consent = std::env::var("CYNIC_HUMAN_ADMIN_CONSENT").ok();
        if consent.as_deref() != Some(capability.id.as_str()) {
            report.blockers.push(format!(
                "human_admin_consent_missing: set CYNIC_HUMAN_ADMIN_CONSENT={} \
                 in the same run after the human approved this exact command",
                capability.id
            ));
            return finish(report);
        }
    }

    // Typed params: --param k=v and bare k=v positionals.
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    let mut iterator = args_slice.iter().peekable();
    while let Some(arg) = iterator.next() {
        if arg == "--param" {
            if let Some(pair) = iterator.next() {
                if let Some((name, value)) = pair.split_once('=') {
                    params.insert(name.to_string(), value.to_string());
                }
            }
        } else if arg.contains('=') && !arg.starts_with('-') {
            if let Some((name, value)) = arg.split_once('=') {
                params.insert(name.to_string(), value.to_string());
            }
        }
    }

    // Endpoint resolution: overrides live in the policy; else the registry.
    let registry_path = args::flag_value(args_slice, "--registry")
        .map(PathBuf::from)
        .unwrap_or_else(default_registry_path);
    let view = match registry_ssh_view(&registry_path) {
        Ok(view) => view,
        Err(error) => {
            report.blockers.push(error.0);
            return finish(report);
        }
    };
    let endpoint = match policy.resolve_endpoint(capability, &view) {
        Ok(endpoint) => {
            report.endpoint_resolved = Some(endpoint.clone());
            endpoint
        }
        Err(error) => {
            report.blockers.push(error.0);
            return finish(report);
        }
    };

    let argv = match policy.render(capability, &params, &endpoint) {
        Ok(argv) => argv,
        Err(error) => {
            report.blockers.push(error.0);
            return finish(report);
        }
    };
    report.argv = argv.clone();

    if !apply {
        report.status = "planned".to_string();
        return finish(report);
    }

    // Execute the rendered argv verbatim; capture read-back proof.
    let (program, rest) = argv.split_first().expect("rendered argv non-empty");
    let output = Command::new(program).args(rest).output();
    match output {
        Ok(output) => {
            let success = output.status.success();
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            print!("{}", String::from_utf8_lossy(&output.stdout));
            report.status = if success { "completed" } else { "failed" }.to_string();
            if !success {
                report
                    .blockers
                    .push(format!("exit_status: {:?}", output.status.code()));
            }
        }
        Err(error) => {
            report.blockers.push(format!("spawn_failed: {error}"));
        }
    }
    finish(report)
}

fn default_registry_path() -> PathBuf {
    PathBuf::from("infra/registry.json")
}

fn finish(report: TransportRunReport) -> Result<std::process::ExitCode, String> {
    let failed = report.status != "completed" && report.status != "planned";
    crate::host_exec_cli::io::emit_json(&report, failed)
}
