use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};
use std::process::ExitCode;

use crate::adapters::local_command_runner::LocalHostCommandRunner;
use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::domain::shared::SurfaceStatus;
use crate::host_exec_cli::args::{has_flag, usage};
use crate::host_exec_cli::bootstrap::host_substrate_active;
use crate::host_exec_cli::io::emit_json;
use crate::host_exec_cli::profile_selection::{selected_profile, selected_resolution};
use crate::profiles::HostProfile;
use crate::usecases::{
    HostObservationReports, build_host_activation_plan_with_resolution,
    build_host_apply_report_with_resolution, build_host_bundle_with_resolution,
    execute_admitted_service_apply, execute_host_apply, observe_host_with_resolution,
    service_apply_satisfied_preconditions,
};

pub(crate) fn execute_network_baseline(args: &[String]) -> Result<ExitCode, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage());
    };

    let profile_args = &args[1..];
    let selected = selected_profile(profile_args)?;
    let resolution = selected_resolution(profile_args);
    let observed =
        observe_host_with_resolution(&selected, &LocalHostObservationAdapter, resolution.clone());

    match command {
        "observe" => emit_json(&observed, !host_substrate_active(&observed)),
        "bundle" => emit_json(
            &build_host_bundle_with_resolution(&selected, resolution.clone()),
            false,
        ),
        "plan" => emit_json(
            &build_host_activation_plan_with_resolution(&selected, &observed, resolution.clone()),
            false,
        ),
        "apply" => execute_network_apply(args, &selected, &observed),
        "apply-report" => emit_json(
            &build_host_apply_report_with_resolution(&selected, &observed, true, resolution),
            false,
        ),
        _ => Err(format!("unknown command: {command}\n{}", usage())),
    }
}

fn execute_network_apply(
    args: &[String],
    selected: &HostProfile,
    observed: &HostObservationReports,
) -> Result<ExitCode, String> {
    let dry_run = has_flag(args, "--dry-run");
    let apply = has_flag(args, "--apply");
    if dry_run == apply {
        return Err("apply requires exactly one of --dry-run or --apply".to_string());
    }

    let report = if dry_run {
        execute_host_apply(selected, observed, true, &LocalHostCommandRunner)
    } else {
        let authorization = ExecutionAuthorization::new(
            "host-exec-human-operator",
            PolicyAuthority::HumanOperator,
            true,
            service_apply_satisfied_preconditions(observed),
        );
        execute_admitted_service_apply(
            selected,
            observed,
            authorization,
            &LocalHostCommandRunner,
            &LocalHostObservationAdapter,
        )
    };
    let should_fail = !dry_run && report.status != SurfaceStatus::Active;
    emit_json(&report, should_fail)
}
