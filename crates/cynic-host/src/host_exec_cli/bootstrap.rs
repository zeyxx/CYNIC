use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};
use std::process::ExitCode;

use crate::adapters::local_command_runner::LocalHostCommandRunner;
use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::bootstrap_chain;
use crate::bridge;
use crate::domain::bootstrap::{build_bootstrap_plan, build_bootstrap_status};
use crate::domain::shared::SurfaceStatus;
use crate::host_bootstrap_evidence;
use crate::host_exec_cli::args::has_flag;
use crate::host_exec_cli::io::emit_json;
use crate::host_exec_cli::profile_selection::{selected_profile, selected_resolution};
use crate::ports::HostPrincipalPort;
use crate::profiles::HostProfile;
use crate::usecases::{HostObservationReports, observe_host_with_resolution};

pub(crate) fn execute_bootstrap_status(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let observed = observe_selected_host(&selected, args);
    let bridge_status = bridge::bridge_status();
    let report = build_bootstrap_status(
        bridge_status.status == "active",
        host_substrate_active(&observed),
    );
    emit_json(&report, report.status != SurfaceStatus::Active)
}

pub(crate) fn execute_bootstrap_plan(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let observed = observe_selected_host(&selected, args);
    let bridge_status = bridge::bridge_status();
    let status = build_bootstrap_status(
        bridge_status.status == "active",
        host_substrate_active(&observed),
    );
    emit_json(&build_bootstrap_plan(&status), false)
}

pub(crate) fn execute_bootstrap_chain(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let resolution = selected_resolution(args);
    let report = bootstrap_chain::build_bootstrap_chain_report(selected, resolution)
        .map_err(|error| error.to_string())?;
    emit_json(&report, report.status != SurfaceStatus::Active)
}

pub(crate) fn execute_bootstrap_evidence(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let resolution = selected_resolution(args);
    let report = host_bootstrap_evidence::observe_host_bootstrap_evidence(selected, resolution)
        .map_err(|error| error.to_string())?;
    emit_json(&report, report.status != SurfaceStatus::Active)
}

pub(crate) fn execute_principal_prepare(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let resolution = selected_resolution(args);
    if !has_flag(args, "--apply") {
        return emit_json(
            &crate::principal_prepare::principal_prepare_plan_with_resolution(
                &selected.desired_principal(),
                resolution.as_ref(),
            ),
            false,
        );
    }

    let authorization = ExecutionAuthorization::new(
        "host-exec-bootstrap-admin",
        PolicyAuthority::HumanAdmin,
        true,
        crate::principal_prepare::principal_prepare_preconditions(),
    );
    let report = crate::principal_prepare::principal_prepare_apply(
        &selected.desired_principal(),
        resolution.as_ref(),
        authorization,
        &LocalHostCommandRunner,
        &LocalHostObservationAdapter,
    );
    emit_json(&report, report.status != "completed")
}

pub(crate) fn observe_selected_host(
    selected: &HostProfile,
    args: &[String],
) -> HostObservationReports {
    observe_host_with_resolution(
        selected,
        &crate::adapters::local_observation::LocalHostObservationAdapter,
        selected_resolution(args),
    )
}

pub(crate) fn host_substrate_active(observed: &HostObservationReports) -> bool {
    observed.principal.status == SurfaceStatus::Active
        && observed.privilege.status == SurfaceStatus::Active
        && observed.packages.status == SurfaceStatus::Active
        && observed.services.status == SurfaceStatus::Active
}
