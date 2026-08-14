use std::process::ExitCode;

use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};

use crate::adapters::local_command_runner::LocalHostCommandRunner;
use crate::adapters::local_observation::LocalHostObservationAdapter;
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
    let spec = bridge::privilege_bridge_install_spec(bundle_dir, "local-host");
    emit_json(&bridge::privilege_bridge_plan(&spec), false)
}

pub(crate) fn execute_bridge_install(args: &[String]) -> Result<ExitCode, String> {
    let bundle_dir = host_exec_bundle_dir(args);
    let apply = has_flag(args, "--apply");
    let spec = bridge::privilege_bridge_install_spec(bundle_dir, "local-host");
    let authorization = ExecutionAuthorization::new(
        "host-exec-bootstrap-admin",
        PolicyAuthority::HumanAdmin,
        apply,
        crate::execution::privilege_bridge_preconditions(),
    );
    let report = bridge::privilege_bridge_apply(
        &spec,
        authorization,
        &LocalHostCommandRunner,
        &LocalHostObservationAdapter,
    );
    emit_json(&report, report.status != "completed")
}

pub(crate) fn execute_bridge_status() -> Result<ExitCode, String> {
    let report = bridge::bridge_status();
    emit_json(&report, report.status != "active")
}
