use std::process::ExitCode;

use crate::bridge;
use crate::host_exec_cli::args::{flag_value, has_flag, host_exec_bundle_dir};
use crate::host_exec_cli::io::emit_json;

pub(crate) fn execute_bridge_bundle(args: &[String]) -> Result<ExitCode, String> {
    let output_dir = flag_value(args, "--output-dir")
        .ok_or_else(|| "bridge-bundle requires --output-dir <path>".to_string())?;
    emit_json(
        &bridge::write_bridge_bundle(output_dir).map_err(|error| error.to_string())?,
        false,
    )
}

pub(crate) fn execute_bridge_plan(args: &[String]) -> Result<ExitCode, String> {
    let bundle_dir = host_exec_bundle_dir(args);
    emit_json(&bridge::bridge_activation_plan(bundle_dir), false)
}

pub(crate) fn execute_bridge_install(args: &[String]) -> Result<ExitCode, String> {
    let bundle_dir = host_exec_bundle_dir(args);
    let apply = has_flag(args, "--apply");
    let report = bridge::bridge_install_report(bundle_dir, apply);
    emit_json(&report, report.status != "completed")
}

pub(crate) fn execute_bridge_status() -> Result<ExitCode, String> {
    let report = bridge::bridge_status();
    emit_json(&report, report.status != "active")
}
