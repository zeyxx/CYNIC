use std::path::Path;
use std::process::ExitCode;

use cynic_host::adapters::local_observation::LocalHostObservationAdapter;
use cynic_host::bridge;
use cynic_host::ports::HostPrincipalPort;
use cynic_host::usecases::observe_host_with_resolution;

use crate::bootstrap_surface::{
    ProfileArgs, observed_blocked, selected_profile, selected_resolution,
};
use crate::{emit_json, fail};

pub(crate) fn maybe_run_host_exec_alias() -> Option<ExitCode> {
    let argv0 = std::env::args().next()?;
    let executable_name = Path::new(&argv0).file_name()?.to_string_lossy();
    if executable_name.as_ref() != "cynic-host-exec" {
        return None;
    }

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    Some(match execute_host_exec_alias(args) {
        Ok(exit_code) => exit_code,
        Err(error) => fail(error),
    })
}

fn execute_host_exec_alias(args: Vec<String>) -> Result<ExitCode, String> {
    let Some(profile) = args.first().map(String::as_str) else {
        return Err("usage: cynic-host-exec <bridge-status | principal-prepare [--apply] | network-baseline observe>".to_string());
    };

    match profile {
        "bridge-status" => {
            let report = bridge::bridge_status();
            Ok(emit_json(&report, report.status != "active"))
        }
        "principal-prepare" => {
            let profile_args = parse_profile_args(&args[1..]);
            let selected = selected_profile(&profile_args)?;
            let apply = args.iter().any(|arg| arg == "--apply");
            if apply {
                let report = cynic_host::principal_prepare::principal_prepare_apply(
                    &selected.desired_principal(),
                );
                Ok(emit_json(&report, report.status != "completed"))
            } else {
                Ok(emit_json(
                    &cynic_host::principal_prepare::principal_prepare_plan(
                        &selected.desired_principal(),
                    ),
                    false,
                ))
            }
        }
        "network-baseline" => {
            let Some(command) = args.get(1).map(String::as_str) else {
                return Err("usage: cynic-host-exec network-baseline observe".to_string());
            };
            if command != "observe" {
                return Err(format!(
                    "unsupported bounded network-baseline command: {command}"
                ));
            }
            let profile_args = parse_profile_args(&args[2..]);
            let selected = selected_profile(&profile_args)?;
            let resolution = selected_resolution(&profile_args);
            let observed =
                observe_host_with_resolution(&selected, &LocalHostObservationAdapter, resolution);
            Ok(emit_json(&observed, observed_blocked(&observed)))
        }
        other => Err(format!("unsupported bounded host-exec profile: {other}")),
    }
}

fn parse_profile_args(args: &[String]) -> ProfileArgs {
    ProfileArgs {
        profile: flag_value(args, "--profile"),
        node: flag_value(args, "--node"),
        registry: flag_value(args, "--registry"),
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}
