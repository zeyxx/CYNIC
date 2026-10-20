use crate::command_surface::{
    HostCommandSurface, install_dir_command, install_file_command, validate_sudoers_command,
};
use crate::install_contract::{
    InstallMode, InstallOwnership, InstallPathPolicy, MODE_0440, MODE_0755,
};
use crate::process_observation::capture_argv;
use crate::runtime_layout::current_runtime_layout;
use std::borrow::Cow;
use std::fs;
use std::path::Path;

pub mod admitted;
pub mod bundle;
pub mod installer;
pub mod status;
pub mod types;

#[cfg(test)]
mod admitted_tests;
#[cfg(test)]
mod tests;

pub use admitted::{
    PrivilegeBridgeExecutionReport, PrivilegeBridgeOperationReport, PrivilegeBridgeOperationResult,
    privilege_bridge_apply, privilege_bridge_plan,
};
use installer::bridge_install_report_from_commands;
use status::bridge_membrane_status;
pub use types::{
    HostExecBridgeInstallReport, HostExecBridgeRootlessSmokeReport, HostExecBridgeStatus,
    PrivilegeBridgeInstallSpec, PrivilegeBridgeObservedState, PrivilegeMembraneBundleManifest,
};

const PRIVILEGE_MEMBRANE_FILENAME: &str = "privilege.sudoers";
const PRIVILEGE_SUBJECT: &str = "cynic";

pub fn privilege_bridge_install_spec(
    bundle_dir: impl AsRef<Path>,
    target: impl Into<String>,
) -> PrivilegeBridgeInstallSpec {
    let bundle_dir = bundle_dir.as_ref();
    let layout = current_runtime_layout();
    let executor = layout.privilege_executor_path();
    let membrane_source = bundle_dir
        .join(PRIVILEGE_MEMBRANE_FILENAME)
        .display()
        .to_string();
    let membrane_target = layout.privilege_sudoers_path();
    let expected_membrane = fs::read_to_string(&membrane_source).unwrap_or_default();
    PrivilegeBridgeInstallSpec {
        target: target.into(),
        bundle_complete: membrane_bundle_complete(bundle_dir),
        subject: PRIVILEGE_SUBJECT.to_string(),
        authority_user: "root".to_string(),
        authority_group: "root".to_string(),
        provider: "sudoers".to_string(),
        executor: executor.clone(),
        membrane_source,
        membrane_target,
        membrane_mode: MODE_0440.unix,
        expected_membrane,
        proof_invocation: vec![
            executor,
            "privilege".to_string(),
            "exec".to_string(),
            "principal-prepare".to_string(),
        ],
    }
}

pub fn bridge_rootless_smoke(
    sandbox_root: impl AsRef<Path>,
    apply: bool,
) -> Result<HostExecBridgeRootlessSmokeReport, Box<dyn std::error::Error>> {
    let sandbox_root = sandbox_root.as_ref();
    let bundle_dir = sandbox_root.join("bundle");
    let membrane_target = sandbox_root
        .join("sudoers.d")
        .join("cynic")
        .display()
        .to_string();
    // The executor is basket-managed: in the sandbox the running test
    // binary stands in for it (present + executable, which is the whole
    // contract the bridge holds on the executor).
    let executor = std::env::current_exe()?.display().to_string();
    let owner_policy = current_user_owned_policy(MODE_0755);
    let sudoers_policy = current_user_owned_policy(MODE_0440);
    let bundle_manifest = bundle::write_privilege_membrane_bundle(&bundle_dir)
        .map_err(|error| format!("membrane bundle: {error}"))?;
    let install_commands = membrane_install_commands_for_runtime(
        &bundle_dir.to_string_lossy(),
        &membrane_target,
        owner_policy,
        sudoers_policy,
    );
    let install_report =
        bridge_install_report_from_commands(&bundle_dir, apply, true, install_commands, false);
    let status_report = bridge_membrane_status(&executor, &membrane_target, PRIVILEGE_SUBJECT);
    let status = if install_report.status == "completed" && status_report.status == "active" {
        "completed"
    } else if install_report.status == "blocked" {
        "blocked"
    } else {
        "failed"
    };

    Ok(HostExecBridgeRootlessSmokeReport {
        schema: "cynic.host_exec_bridge.rootless_smoke.v1",
        status,
        mutates: true,
        requested_apply: apply,
        sandbox_root: sandbox_root.display().to_string(),
        bundle_manifest,
        install_report,
        status_report,
    })
}

pub fn bridge_status() -> HostExecBridgeStatus {
    let layout = current_runtime_layout();
    bridge_membrane_status(
        &layout.privilege_executor_path(),
        &layout.privilege_sudoers_path(),
        PRIVILEGE_SUBJECT,
    )
}

/// Prove the privilege membrane end to end: run `principal-prepare`
/// through `sudo -n` and the admitted `privilege exec` argv, then report
/// local bridge status. The capability is convergent: re-running it on a
/// prepared principal is a no-op, so the probe is safe. Observation itself
/// needs no privilege; this proves the sudo+admit chain works.
pub fn bridge_status_via_membrane() -> Result<HostExecBridgeStatus, String> {
    let layout = current_runtime_layout();
    let argv = vec![
        layout.sudo_bin(),
        "-n".to_string(),
        layout.privilege_executor_path(),
        "privilege".to_string(),
        "exec".to_string(),
        "principal-prepare".to_string(),
    ];
    let output = capture_argv(&argv)?;
    if !output.success {
        return Err(output.stderr);
    }
    let membrane_probe: serde_json::Value =
        serde_json::from_str(&output.stdout).map_err(|error| error.to_string())?;
    if membrane_probe
        .get("status")
        .and_then(serde_json::Value::as_str)
        != Some("executed")
    {
        return Err("privilege membrane did not execute principal-prepare".to_string());
    }
    let mut report = bridge_status();
    report.status = "active".to_string();
    report.checks.membrane_admits_executor = true;
    report
        .missing
        .retain(|check| check != "membrane_admits_executor");
    Ok(report)
}

pub(crate) fn membrane_install_commands_for_runtime(
    bundle_dir: &str,
    membrane_target: &str,
    directory_policy: InstallPathPolicy,
    sudoers_policy: InstallPathPolicy,
) -> Vec<HostCommandSurface> {
    // ORDER IS A SECURITY PROPERTY. `validate_sudoers` runs BEFORE
    // `install_sudoers` and targets the STAGED file, never the installed one.
    // A sudoers file that fails `visudo -c` must never land in
    // `/etc/sudoers.d/`: an unparseable file there can break `sudo` entirely,
    // including the ability to fix it. This is the same invariant
    // `pve_helper_install_commands` enforces.
    // The executor is basket-managed and never installed here: only the
    // staged membrane file moves.
    let staged_membrane = format!("{bundle_dir}/{PRIVILEGE_MEMBRANE_FILENAME}");
    prepare_directory_commands_for_membrane(membrane_target, directory_policy)
        .into_iter()
        .chain([
            validate_sudoers_command("validate_sudoers", staged_membrane.clone()),
            install_file_command(
                "install_sudoers",
                staged_membrane,
                membrane_target.to_string(),
                sudoers_policy,
            ),
        ])
        .collect()
}

fn prepare_directory_commands_for_membrane(
    membrane_target: &str,
    directory_policy: InstallPathPolicy,
) -> Vec<HostCommandSurface> {
    vec![install_dir_command(
        "prepare_directory",
        Path::new(membrane_target)
            .parent()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| current_runtime_layout().sudoers_dir),
        directory_policy,
    )]
}

fn membrane_bundle_complete(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let required = ["manifest.json", PRIVILEGE_MEMBRANE_FILENAME];
    path.exists() && required.iter().all(|name| path.join(name).exists())
}

fn current_user_owned_policy(mode: InstallMode) -> InstallPathPolicy {
    InstallPathPolicy {
        owner: InstallOwnership {
            user: Cow::Owned(observed_user()),
            group: Cow::Owned(observed_group()),
        },
        mode,
    }
}

fn observed_user() -> String {
    capture_argv(&["id".to_string(), "-un".to_string()])
        .ok()
        .filter(|output| output.success && !output.stdout.is_empty())
        .map(|output| output.stdout)
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| PRIVILEGE_SUBJECT.to_string())
}

fn observed_group() -> String {
    capture_argv(&["id".to_string(), "-gn".to_string()])
        .ok()
        .filter(|output| output.success && !output.stdout.is_empty())
        .map(|output| output.stdout)
        .unwrap_or_else(observed_user)
}
