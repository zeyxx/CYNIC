use std::process::ExitCode;

use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};

use crate::adapters::local_command_runner::LocalHostCommandRunner;
use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::host_exec_cli::io::emit_json;
use crate::ports::HostServicePort;
use crate::registry::{
    resolve_current_host_profile, resolve_current_host_profile_resolution,
    resolve_registry_retired_units,
};
use crate::usecases::{
    execute_runtime_service_reconcile_with_policy, observe_host_with_resolution,
    service_apply_satisfied_preconditions,
};

const EXACT_APPLY_ARGS: [&str; 1] = ["--apply"];

pub(crate) fn execute(args: &[String]) -> Result<ExitCode, String> {
    validate_exact_args(args)?;
    let profile = resolve_current_host_profile(None).map_err(|error| {
        format!("service-reconcile requires a registry-resolved current host: {error}")
    })?;
    let resolution = resolve_current_host_profile_resolution(None).map_err(|error| {
        format!("service-reconcile requires a registry-resolved target: {error}")
    })?;
    let observed =
        observe_host_with_resolution(&profile, &LocalHostObservationAdapter, Some(resolution));
    let authorization = ExecutionAuthorization::new(
        "cynic-runtime-host-helper",
        PolicyAuthority::RuntimeAuthority,
        true,
        service_apply_satisfied_preconditions(&observed),
    );
    // The retirement list is registry-derived: it names the units this
    // specific node must no longer run, and the registry is the single
    // authority for what a host runs. A node that declares none retires
    // nothing, and the plan is identical to the previous behaviour.
    let hostname = observed
        .host_resolution
        .as_ref()
        .map(|resolution| resolution.hostname_observed.clone())
        .unwrap_or_default();
    let retired_units = resolve_registry_retired_units(&hostname, None)?;
    let mut policy = HostServicePort::desired_services(&profile);
    policy.retired_units = retired_units;
    let report = execute_runtime_service_reconcile_with_policy(
        &policy,
        &observed,
        authorization,
        &LocalHostCommandRunner,
        &LocalHostObservationAdapter,
    );
    emit_json(
        &report,
        report.status != crate::domain::shared::SurfaceStatus::Active,
    )
}

fn validate_exact_args(args: &[String]) -> Result<(), String> {
    let exact = args.len() == EXACT_APPLY_ARGS.len()
        && args
            .iter()
            .map(String::as_str)
            .eq(EXACT_APPLY_ARGS.iter().copied());
    if exact {
        Ok(())
    } else {
        Err(
            "service-reconcile accepts exactly `--apply`; target and policy are registry-derived"
                .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::validate_exact_args;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn accepts_only_the_exact_runtime_reconcile_envelope() {
        assert!(validate_exact_args(&args(&["--apply"])).is_ok());
        for rejected in [
            args(&[]),
            args(&["--apply", "cynic-node.service"]),
            args(&["--node", "node-a", "--apply"]),
            args(&["--registry", "/tmp/foreign.json", "--apply"]),
        ] {
            assert!(validate_exact_args(&rejected).is_err());
        }
    }
}
