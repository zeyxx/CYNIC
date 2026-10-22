//! Node service unit commands — render and prove the declared systemd unit.
//!
//! The reconciler owns lifecycle (enable/start) but never authors unit
//! content. `service generate` renders the exact bytes from explicit
//! parameters and stages them; `service prove` compares the installed file
//! against the rendering. Installation itself stays an operator or
//! capability-mediated act.

use std::process::ExitCode;

use cynic_node_protocol::node_service_unit::{
    NodeServiceUnitParams, detect_service_unit_drift, render_service_unit, service_unit_sha256,
};
use serde::Serialize;

#[derive(Serialize)]
struct ServiceGenerateReport {
    schema: &'static str,
    status: &'static str,
    unit_name: String,
    digest: String,
    output_path: Option<String>,
}

#[derive(Serialize)]
struct ServiceProveReport {
    schema: &'static str,
    unit_name: String,
    expected_digest: String,
    installed_digest: Option<String>,
    installed_path: String,
    drift: String,
    active: Option<bool>,
}

pub(crate) fn execute_service_generate(
    unit_name: String,
    description: String,
    binary: String,
    config: String,
    user: String,
    output: Option<String>,
    json: bool,
) -> ExitCode {
    let params = NodeServiceUnitParams::new(unit_name, description, binary, config, user);
    let rendered = match render_service_unit(&params) {
        Ok(rendered) => rendered,
        Err(error) => {
            eprintln!("invalid service unit params: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(path) = output.clone() {
        if let Err(error) = std::fs::write(&path, rendered.as_bytes()) {
            eprintln!("cannot write unit to {path}: {error}");
            return ExitCode::FAILURE;
        }
    }
    let report = ServiceGenerateReport {
        schema: "cynic.node_service.generate.report.v1",
        status: "rendered",
        unit_name: params.unit_name.clone(),
        digest: service_unit_sha256(&rendered),
        output_path: output,
    };
    if json {
        crate::cli_json::emit_pretty_json(&report)
    } else {
        println!("RENDERED: {} ({})", report.unit_name, report.digest);
        ExitCode::SUCCESS
    }
}

fn probe_service_active(unit_name: &str) -> Option<bool> {
    let output = std::process::Command::new("systemctl")
        .arg("is-active")
        .arg("--quiet")
        .arg(unit_name)
        .output()
        .ok()?;
    Some(output.status.success())
}

pub(crate) fn execute_service_prove(
    unit_name: String,
    description: String,
    binary: String,
    config: String,
    user: String,
    unit_dir: String,
    json: bool,
) -> ExitCode {
    let params = NodeServiceUnitParams::new(unit_name, description, binary, config, user);
    let installed_path = format!("{}/{}", unit_dir.trim_end_matches('/'), params.unit_name);
    let installed = match std::fs::read_to_string(&installed_path) {
        Ok(content) => Some(content),
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!("cannot read installed unit {installed_path}: {error}");
                return ExitCode::FAILURE;
            }
            None
        }
    };
    let drift = match detect_service_unit_drift(&params, installed.as_deref()) {
        Ok(drift) => drift,
        Err(error) => {
            eprintln!("invalid service unit params: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = ServiceProveReport {
        schema: "cynic.node_service.prove.report.v1",
        unit_name: drift.unit_name.clone(),
        expected_digest: drift.expected_digest.clone(),
        installed_digest: drift.installed_digest.clone(),
        installed_path,
        drift: drift.status.clone(),
        active: probe_service_active(&drift.unit_name),
    };
    if json {
        crate::cli_json::emit_pretty_json(&report)
    } else {
        println!(
            "DRIFT: {} (installed: {})",
            report.drift,
            report.installed_digest.as_deref().unwrap_or("absent")
        );
        ExitCode::SUCCESS
    }
}
