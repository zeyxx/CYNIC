use std::process::ExitCode;

use cynic_host::adapters::local_command_runner::LocalHostCommandRunner;
use cynic_host::adapters::local_observation::LocalHostObservationAdapter;
use cynic_host::bootstrap_chain;
use cynic_host::bridge;
use cynic_host::domain::bootstrap::{build_bootstrap_plan, build_bootstrap_status};
use cynic_host::domain::shared::SurfaceStatus;
use cynic_host::host_bootstrap_evidence;
use cynic_host::host_exec_policy::{HostExecPolicyDocument, HostExecRequest};
use cynic_host::ports::HostPrincipalPort;
use cynic_host::profiles::HostProfile;
use cynic_host::registry::{
    HostProfileResolution, resolve_current_host_profile, resolve_current_host_profile_resolution,
    resolve_host_profile, resolve_host_profile_resolution,
};
use cynic_host::transport;
use cynic_host::usecases::{
    build_host_activation_plan_with_resolution, build_host_apply_report_with_resolution,
    build_host_bundle_with_resolution, observe_host_with_resolution,
};

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match execute(args) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn execute(args: Vec<String>) -> Result<ExitCode, String> {
    let Some(profile) = args.first().map(String::as_str) else {
        return Err(usage());
    };
    if profile == "bridge-bundle" {
        let output_dir = flag_value(&args[1..], "--output-dir")
            .ok_or_else(|| "bridge-bundle requires --output-dir <path>".to_string())?;
        return emit_json(
            &bridge::write_bridge_bundle(output_dir).map_err(|error| error.to_string())?,
            false,
        );
    }
    if profile == "bridge-plan" {
        let bundle_dir = flag_value(&args[1..], "--bundle-dir")
            .unwrap_or_else(|| "/tmp/cynic-host-exec".to_string());
        return emit_json(&bridge::bridge_activation_plan(bundle_dir), false);
    }
    if profile == "bridge-install" {
        let bundle_dir = flag_value(&args[1..], "--bundle-dir")
            .unwrap_or_else(|| "/tmp/cynic-host-exec".to_string());
        let apply = args.iter().any(|arg| arg == "--apply");
        let report = bridge::bridge_install_report(bundle_dir, apply);
        return emit_json(&report, report.status != "completed");
    }
    if profile == "bridge-status" {
        let report = bridge::bridge_status();
        return emit_json(&report, report.status != "active");
    }
    if profile == "host-exec-validate-request" {
        let policy_path = flag_value(&args[1..], "--policy")
            .unwrap_or_else(|| "/etc/cynic/host-exec.policy.json".to_string());
        let declared_services = flag_value(&args[1..], "--declared-services")
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_string)
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        let payload = read_stdin_json()?;
        let request: HostExecRequest = serde_json::from_value(payload)
            .map_err(|error| format!("invalid host exec request: {error}"))?;
        let policy = HostExecPolicyDocument::load(std::path::Path::new(&policy_path))?;
        let validation = policy.validate_request(&request, &declared_services);
        return emit_json(&validation, !validation.ok);
    }
    if profile == "transport-matrix" {
        return emit_json(&transport::build_transport_matrix_report(), false);
    }
    if profile == "bootstrap-status" {
        let selected = selected_profile(&args[1..])?;
        let resolution = selected_resolution(&args[1..]);
        let observed =
            observe_host_with_resolution(&selected, &LocalHostObservationAdapter, resolution);
        let host_substrate_active = observed.principal.status == SurfaceStatus::Active
            && observed.privilege.status == SurfaceStatus::Active
            && observed.packages.status == SurfaceStatus::Active
            && observed.services.status == SurfaceStatus::Active;
        let bridge_status = bridge::bridge_status();
        let report =
            build_bootstrap_status(bridge_status.status == "active", host_substrate_active);
        return emit_json(&report, report.status != SurfaceStatus::Active);
    }
    if profile == "bootstrap-plan" {
        let selected = selected_profile(&args[1..])?;
        let resolution = selected_resolution(&args[1..]);
        let observed =
            observe_host_with_resolution(&selected, &LocalHostObservationAdapter, resolution);
        let host_substrate_active = observed.principal.status == SurfaceStatus::Active
            && observed.privilege.status == SurfaceStatus::Active
            && observed.packages.status == SurfaceStatus::Active
            && observed.services.status == SurfaceStatus::Active;
        let bridge_status = bridge::bridge_status();
        let status =
            build_bootstrap_status(bridge_status.status == "active", host_substrate_active);
        return emit_json(&build_bootstrap_plan(&status), false);
    }
    if profile == "bootstrap-chain" {
        let selected = selected_profile(&args[1..])?;
        let resolution = selected_resolution(&args[1..]);
        let report = bootstrap_chain::build_bootstrap_chain_report(selected, resolution)
            .map_err(|error| error.to_string())?;
        return emit_json(&report, report.status != SurfaceStatus::Active);
    }
    if profile == "bootstrap-evidence" {
        let selected = selected_profile(&args[1..])?;
        let resolution = selected_resolution(&args[1..]);
        let report = host_bootstrap_evidence::observe_host_bootstrap_evidence(selected, resolution)
            .map_err(|error| error.to_string())?;
        return emit_json(&report, report.status != SurfaceStatus::Active);
    }
    if profile == "principal-prepare" {
        let selected = selected_profile(&args[1..])?;
        let apply = args.iter().any(|arg| arg == "--apply");
        if !apply {
            return emit_json(
                &cynic_host::principal_prepare::principal_prepare_plan(
                    &selected.desired_principal(),
                ),
                false,
            );
        }
        let report =
            cynic_host::principal_prepare::principal_prepare_apply(&selected.desired_principal());
        return emit_json(&report, report.status != "completed");
    }
    if profile != "network-baseline" {
        return Err(format!("unknown profile: {profile}\n{}", usage()));
    }

    let Some(command) = args.get(1).map(String::as_str) else {
        return Err(usage());
    };

    let selected = selected_profile(&args[2..])?;
    let resolution = selected_resolution(&args[2..]);
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
        "apply" => {
            let dry_run = args.iter().any(|arg| arg == "--dry-run");
            let apply = args.iter().any(|arg| arg == "--apply");
            if dry_run == apply {
                return Err("apply requires exactly one of --dry-run or --apply".to_string());
            }
            let report = if dry_run {
                cynic_host::usecases::execute_host_apply(
                    &selected,
                    &observed,
                    true,
                    &LocalHostCommandRunner,
                )
            } else {
                cynic_host::usecases::execute_host_apply(
                    &selected,
                    &observed,
                    false,
                    &LocalHostCommandRunner,
                )
            };
            let should_fail =
                !dry_run && report.status != cynic_host::domain::shared::SurfaceStatus::Active;
            emit_json(&report, should_fail)
        }
        "apply-report" => emit_json(
            &build_host_apply_report_with_resolution(&selected, &observed, true, resolution),
            false,
        ),
        _ => Err(format!("unknown command: {command}\n{}", usage())),
    }
}

fn observed_blocked(observed: &cynic_host::usecases::HostObservationReports) -> bool {
    use cynic_host::domain::shared::SurfaceStatus;

    observed.principal.status != SurfaceStatus::Active
        || observed.privilege.status != SurfaceStatus::Active
        || observed.packages.status != SurfaceStatus::Active
        || observed.services.status != SurfaceStatus::Active
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
