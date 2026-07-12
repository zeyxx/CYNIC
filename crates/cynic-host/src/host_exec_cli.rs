use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use crate::adapters::local_command_runner::LocalHostCommandRunner;
use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::bootstrap_chain;
use crate::bridge;
use crate::domain::bootstrap::{build_bootstrap_plan, build_bootstrap_status};
use crate::domain::shared::SurfaceStatus;
use crate::host_bootstrap_evidence;
use crate::host_exec_policy::{HostExecPolicyDocument, HostExecRequest};
use crate::ports::HostPrincipalPort;
use crate::profiles::HostProfile;
use crate::registry::{
    HostProfileResolution, resolve_current_host_profile, resolve_current_host_profile_resolution,
    resolve_host_profile, resolve_host_profile_resolution,
};
use crate::runtime_layout::current_runtime_layout;
use crate::transport;
use crate::usecases::{
    HostObservationReports, build_host_activation_plan_with_resolution,
    build_host_apply_report_with_resolution, build_host_bundle_with_resolution,
    observe_host_with_resolution,
};

pub fn execute(args: Vec<String>) -> Result<ExitCode, String> {
    let Some(profile) = args.first().map(String::as_str) else {
        return Err(usage());
    };

    match profile {
        "bridge-bundle" => execute_bridge_bundle(&args[1..]),
        "bridge-plan" => execute_bridge_plan(&args[1..]),
        "bridge-install" => execute_bridge_install(&args[1..]),
        "bridge-status" => {
            let report = bridge::bridge_status();
            emit_json(&report, report.status != "active")
        }
        "host-exec-validate-request" => execute_host_exec_validate_request(&args[1..]),
        "transport-matrix" => emit_json(&transport::build_transport_matrix_report(), false),
        "bootstrap-status" => execute_bootstrap_status(&args[1..]),
        "bootstrap-plan" => execute_bootstrap_plan(&args[1..]),
        "bootstrap-chain" => execute_bootstrap_chain(&args[1..]),
        "bootstrap-evidence" => execute_bootstrap_evidence(&args[1..]),
        "principal-prepare" => execute_principal_prepare(&args[1..]),
        "network-baseline" => execute_network_baseline(&args[1..]),
        _ => Err(format!("unknown profile: {profile}\n{}", usage())),
    }
}

fn execute_bridge_bundle(args: &[String]) -> Result<ExitCode, String> {
    let output_dir = flag_value(args, "--output-dir")
        .ok_or_else(|| "bridge-bundle requires --output-dir <path>".to_string())?;
    emit_json(
        &bridge::write_bridge_bundle(output_dir).map_err(|error| error.to_string())?,
        false,
    )
}

fn execute_bridge_plan(args: &[String]) -> Result<ExitCode, String> {
    let bundle_dir = host_exec_bundle_dir(args);
    emit_json(&bridge::bridge_activation_plan(bundle_dir), false)
}

fn execute_bridge_install(args: &[String]) -> Result<ExitCode, String> {
    let bundle_dir = host_exec_bundle_dir(args);
    let apply = has_flag(args, "--apply");
    let report = bridge::bridge_install_report(bundle_dir, apply);
    emit_json(&report, report.status != "completed")
}

fn execute_host_exec_validate_request(args: &[String]) -> Result<ExitCode, String> {
    let policy_path = flag_value(args, "--policy").unwrap_or_else(default_host_exec_policy_path);
    let declared_services = declared_services(args);
    let payload = read_stdin_json()?;
    let request: HostExecRequest = serde_json::from_value(payload)
        .map_err(|error| format!("invalid host exec request: {error}"))?;
    let policy = HostExecPolicyDocument::load(Path::new(&policy_path))?;
    let validation = policy.validate_request(&request, &declared_services);
    emit_json(&validation, !validation.ok)
}

fn execute_bootstrap_status(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let observed = observe_selected_host(&selected, args);
    let bridge_status = bridge::bridge_status();
    let report = build_bootstrap_status(
        bridge_status.status == "active",
        host_substrate_active(&observed),
    );
    emit_json(&report, report.status != SurfaceStatus::Active)
}

fn execute_bootstrap_plan(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let observed = observe_selected_host(&selected, args);
    let bridge_status = bridge::bridge_status();
    let status = build_bootstrap_status(
        bridge_status.status == "active",
        host_substrate_active(&observed),
    );
    emit_json(&build_bootstrap_plan(&status), false)
}

fn execute_bootstrap_chain(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let resolution = selected_resolution(args);
    let report = bootstrap_chain::build_bootstrap_chain_report(selected, resolution)
        .map_err(|error| error.to_string())?;
    emit_json(&report, report.status != SurfaceStatus::Active)
}

fn execute_bootstrap_evidence(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    let resolution = selected_resolution(args);
    let report = host_bootstrap_evidence::observe_host_bootstrap_evidence(selected, resolution)
        .map_err(|error| error.to_string())?;
    emit_json(&report, report.status != SurfaceStatus::Active)
}

fn execute_principal_prepare(args: &[String]) -> Result<ExitCode, String> {
    let selected = selected_profile(args)?;
    if !has_flag(args, "--apply") {
        return emit_json(
            &crate::principal_prepare::principal_prepare_plan(&selected.desired_principal()),
            false,
        );
    }

    let report = crate::principal_prepare::principal_prepare_apply(&selected.desired_principal());
    emit_json(&report, report.status != "completed")
}

fn execute_network_baseline(args: &[String]) -> Result<ExitCode, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage());
    };

    let profile_args = &args[1..];
    let selected = selected_profile(profile_args)?;
    let resolution = selected_resolution(profile_args);
    let observed =
        observe_host_with_resolution(&selected, &LocalHostObservationAdapter, resolution.clone());

    match command {
        "observe" => emit_json(&observed, observed_blocked(&observed)),
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

    let report =
        crate::usecases::execute_host_apply(selected, observed, dry_run, &LocalHostCommandRunner);
    let should_fail = !dry_run && report.status != SurfaceStatus::Active;
    emit_json(&report, should_fail)
}

fn observe_selected_host(selected: &HostProfile, args: &[String]) -> HostObservationReports {
    observe_host_with_resolution(
        selected,
        &LocalHostObservationAdapter,
        selected_resolution(args),
    )
}

fn host_substrate_active(observed: &HostObservationReports) -> bool {
    observed.principal.status == SurfaceStatus::Active
        && observed.privilege.status == SurfaceStatus::Active
        && observed.packages.status == SurfaceStatus::Active
        && observed.services.status == SurfaceStatus::Active
}

fn observed_blocked(observed: &HostObservationReports) -> bool {
    !host_substrate_active(observed)
}

fn emit_json(value: &impl serde::Serialize, should_fail: bool) -> Result<ExitCode, String> {
    let payload = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    println!("{payload}");
    Ok(if should_fail {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn read_stdin_json() -> Result<serde_json::Value, String> {
    use std::io::Read;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    serde_json::from_str(&input).map_err(|error| format!("failed to parse stdin json: {error}"))
}

fn usage() -> String {
    "usage: cynic-host-exec <bridge-bundle --output-dir <dir> | bridge-plan [--bundle-dir <dir>] | bridge-install [--bundle-dir <dir>] --apply | bridge-status | host-exec-validate-request [--policy <path>] [--declared-services svc1,svc2] < request.json | transport-matrix | bootstrap-status [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-plan [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-chain [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-evidence [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | principal-prepare [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] [--apply] | network-baseline <observe|bundle|plan|apply-report|apply> [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] [--dry-run|--apply]>".to_string()
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn host_exec_bundle_dir(args: &[String]) -> String {
    flag_value(args, "--bundle-dir").unwrap_or_else(default_host_exec_bundle_dir)
}

fn default_host_exec_bundle_dir() -> String {
    current_runtime_layout().host_exec_bundle_dir
}

fn default_host_exec_policy_path() -> String {
    current_runtime_layout().host_exec_policy_path()
}

fn declared_services(args: &[String]) -> BTreeSet<String> {
    flag_value(args, "--declared-services")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn selected_profile(args: &[String]) -> Result<HostProfile, String> {
    if let Some(name) = flag_value(args, "--profile") {
        return HostProfile::parse(&name);
    }
    if let Some(node_name) = flag_value(args, "--node") {
        let registry_path = flag_value(args, "--registry");
        return resolve_host_profile(&node_name, registry_path.as_deref());
    }
    let registry_path = flag_value(args, "--registry");
    match resolve_current_host_profile(registry_path.as_deref()) {
        Ok(profile) => Ok(profile),
        Err(_) => Ok(HostProfile::NetworkNode),
    }
}

fn selected_resolution(args: &[String]) -> Option<HostProfileResolution> {
    if let Some(name) = flag_value(args, "--profile") {
        return Some(HostProfileResolution {
            hostname_observed: std::env::var("HOSTNAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
            matched_node: None,
            matched_role: None,
            canonical_role: None,
            profile_applied: match HostProfile::parse(&name) {
                Ok(profile) => profile.name(),
                Err(_) => "unknown",
            },
            match_source: "explicit_profile",
            host_bootstrap_target_stage: None,
            host_bootstrap_required_domains: Vec::new(),
            registry_path: flag_value(args, "--registry"),
        });
    }
    if let Some(node_name) = flag_value(args, "--node") {
        let registry_path = flag_value(args, "--registry");
        return resolve_host_profile_resolution(&node_name, registry_path.as_deref()).ok();
    }
    let registry_path = flag_value(args, "--registry");
    resolve_current_host_profile_resolution(registry_path.as_deref()).ok()
}

#[cfg(test)]
mod tests {
    use super::{declared_services, flag_value, host_exec_bundle_dir};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn flag_value_reads_following_token() {
        let args = args(&[
            "--profile",
            "network-node",
            "--registry",
            "infra/registry.json",
        ]);

        assert_eq!(
            flag_value(&args, "--registry"),
            Some("infra/registry.json".to_string())
        );
    }

    #[test]
    fn host_exec_bundle_dir_has_single_default() {
        assert_eq!(host_exec_bundle_dir(&[]), "/tmp/cynic-host-exec");
    }

    #[test]
    fn declared_services_are_trimmed_and_empty_values_removed() {
        let args = args(&["--declared-services", "cynic-kernel, ,surrealdb.service"]);

        let services = declared_services(&args);

        assert!(services.contains("cynic-kernel"));
        assert!(services.contains("surrealdb.service"));
        assert_eq!(services.len(), 2);
    }
}
