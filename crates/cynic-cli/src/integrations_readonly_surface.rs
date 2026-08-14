use std::process::ExitCode;

use crate::cli_json::emit_pretty_json;
use crate::integrations_surface_render::{
    print_integrations_plan_human, print_integrations_status_human,
};
use cynic_host::adapters::local_integration::{
    observe_local_integrations, plan_local_integrations,
};

pub(crate) fn execute_integrations_status(json: bool) -> ExitCode {
    let report = observe_local_integrations();
    if json {
        return emit_pretty_json(&report);
    } else {
        print_integrations_status_human(&report);
    }
    ExitCode::SUCCESS
}

pub(crate) fn execute_integrations_plan(json: bool) -> ExitCode {
    let report = plan_local_integrations();
    if json {
        return emit_pretty_json(&report);
    } else {
        print_integrations_plan_human(&report);
    }
    ExitCode::SUCCESS
}
