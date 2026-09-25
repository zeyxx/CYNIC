use crate::bounded_helper::{BoundedHelperInstallRuntime, BoundedHelperProjection};
use crate::command_surface::{
    HostCommandSurface, install_dir_command, install_file_command, validate_sudoers_command,
};
use crate::install_contract::{
    InstallMode, InstallOwnership, InstallPathPolicy, MODE_0440, MODE_0755,
};
use crate::process_observation::capture_argv;
use crate::runtime_layout::current_runtime_layout;
use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

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
use bundle::{BridgeBundleContext, write_bridge_bundle_with_context};
use installer::bridge_install_report_from_commands;
use status::bridge_status_for_projection;
pub use types::{
    HostExecBridgeBundleManifest, HostExecBridgeInstallReport, HostExecBridgeRootlessSmokeReport,
    HostExecBridgeStatus, PrivilegeBridgeInstallSpec, PrivilegeBridgeObservedState,
};

const HOST_EXEC_HELPER_FILENAME: &str = "cynic-host-exec";
const HOST_EXEC_SUDOERS_FILENAME: &str = "cynic-host-exec.sudoers";
const HOST_EXEC_SUDOERS_SUBJECT: &str = "cynic";

pub fn write_bridge_bundle(
    output_dir: impl AsRef<Path>,
) -> Result<HostExecBridgeBundleManifest, Box<dyn std::error::Error>> {
    let helper_source = helper_binary_source()?;
    write_bridge_bundle_from_binary(output_dir, helper_source)
}

pub fn write_bridge_bundle_from_binary(
    output_dir: impl AsRef<Path>,
    helper_source: impl AsRef<Path>,
) -> Result<HostExecBridgeBundleManifest, Box<dyn std::error::Error>> {
    write_bridge_bundle_with_context(
        output_dir,
        helper_source.as_ref(),
        bridge_bundle_context(helper_projection(), helper_install_runtime()),
        None,
    )
}

fn write_bridge_bundle_with_runtime(
    output_dir: impl AsRef<Path>,
    helper_source: &Path,
    projection: BoundedHelperProjection,
    helper_runtime: BoundedHelperInstallRuntime,
    prepare_directories: Option<Vec<HostCommandSurface>>,
) -> Result<HostExecBridgeBundleManifest, Box<dyn std::error::Error>> {
    write_bridge_bundle_with_context(
        output_dir,
        helper_source,
        bridge_bundle_context(projection, helper_runtime),
        prepare_directories,
    )
}

pub fn bridge_install_report(
    bundle_dir: impl AsRef<Path>,
    apply: bool,
) -> PrivilegeBridgeExecutionReport {
    let spec = privilege_bridge_install_spec(bundle_dir, "local-host");
    privilege_bridge_apply(
        &spec,
        ExecutionAuthorization::new(
            "bridge-install-compatibility",
            PolicyAuthority::HumanAdmin,
            apply,
            crate::execution::privilege_bridge_preconditions(),
        ),
        &crate::adapters::local_command_runner::LocalHostCommandRunner,
        &crate::adapters::local_observation::LocalHostObservationAdapter,
    )
}

pub fn privilege_bridge_install_spec(
    bundle_dir: impl AsRef<Path>,
    target: impl Into<String>,
) -> PrivilegeBridgeInstallSpec {
    let bundle_dir = bundle_dir.as_ref();
    let runtime = helper_install_runtime();
    let projection = helper_projection();
    PrivilegeBridgeInstallSpec {
        target: target.into(),
        bundle_complete: bundle_complete(bundle_dir),
        subject: projection.subject.clone(),
        authority_user: "root".to_string(),
        authority_group: "root".to_string(),
        provider: "sudoers".to_string(),
        helper_source: bundle_dir
            .join(HOST_EXEC_HELPER_FILENAME)
            .display()
            .to_string(),
        helper_target: runtime.helper_target,
        helper_mode: MODE_0755.unix,
        projection_source: bundle_dir
            .join(HOST_EXEC_SUDOERS_FILENAME)
            .display()
            .to_string(),
        projection_target: runtime.sudoers_target,
        projection_mode: MODE_0440.unix,
        expected_projection: projection.sudoers_projection(),
        proof_invocation: vec![projection.helper_path, "principal-prepare".to_string()],
    }
}

pub fn bridge_rootless_smoke(
    sandbox_root: impl AsRef<Path>,
    apply: bool,
) -> Result<HostExecBridgeRootlessSmokeReport, Box<dyn std::error::Error>> {
    let sandbox_root = sandbox_root.as_ref();
    let bundle_dir = sandbox_root.join("bundle");
    let helper_runtime = BoundedHelperInstallRuntime {
        helper_target: sandbox_root
            .join("libexec")
            .join(HOST_EXEC_HELPER_FILENAME)
            .display()
            .to_string(),
        sudoers_target: sandbox_root
            .join("sudoers.d")
            .join("cynic-host-exec")
            .display()
            .to_string(),
        helper_dir: sandbox_root.join("libexec").display().to_string(),
        sudoers_dir: sandbox_root.join("sudoers.d").display().to_string(),
    };
    let projection = helper_projection_for_paths(
        helper_runtime.helper_target.clone(),
        helper_runtime.sudoers_target.clone(),
    );
    let owner_policy = current_user_owned_policy(MODE_0755);
    let sudoers_policy = current_user_owned_policy(MODE_0440);
    let prepare_directories =
        prepare_directory_commands_for_runtime(&helper_runtime, owner_policy.clone());
    let helper_source = helper_binary_source()?;
    let bundle_manifest = write_bridge_bundle_with_runtime(
        &bundle_dir,
        &helper_source,
        projection.clone(),
        helper_runtime.clone(),
        Some(prepare_directories),
    )?;
    let install_commands = install_commands_for_runtime(
        &helper_runtime,
        &bundle_dir.to_string_lossy(),
        owner_policy,
        sudoers_policy,
    );
    let install_report =
        bridge_install_report_from_commands(&bundle_dir, apply, true, install_commands, false);
    let status_report = bridge_status_for_projection(&projection);
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
    bridge_status_for_projection(&helper_projection())
}

pub fn bridge_status_via_helper() -> Result<HostExecBridgeStatus, String> {
    let layout = current_runtime_layout();
    let argv = vec![
        layout.sudo_bin(),
        "-n".to_string(),
        host_exec_helper_path(),
        "bridge-status".to_string(),
    ];
    let output = capture_argv(&argv)?;
    if !output.success {
        return Err(output.stderr);
    }
    let helper_status: serde_json::Value =
        serde_json::from_str(&output.stdout).map_err(|error| error.to_string())?;
    if helper_status
        .get("status")
        .and_then(serde_json::Value::as_str)
        != Some("active")
    {
        return Err("bounded helper did not report an active bridge".to_string());
    }
    let mut report = bridge_status();
    report.status = "active".to_string();
    report.checks.sudoers_rule_matches = true;
    report
        .missing
        .retain(|check| check != "sudoers_rule_matches");
    Ok(report)
}

fn install_commands_for_runtime(
    helper_runtime: &BoundedHelperInstallRuntime,
    bundle_dir: &str,
    directory_policy: InstallPathPolicy,
    sudoers_policy: InstallPathPolicy,
) -> Vec<HostCommandSurface> {
    prepare_directory_commands_for_runtime(helper_runtime, directory_policy)
        .into_iter()
        .chain([
            install_file_command(
                "install_helper",
                format!("{bundle_dir}/{HOST_EXEC_HELPER_FILENAME}"),
                helper_runtime.helper_target.clone(),
                current_user_owned_policy(MODE_0755),
            ),
            install_file_command(
                "install_sudoers",
                format!("{bundle_dir}/{HOST_EXEC_SUDOERS_FILENAME}"),
                helper_runtime.sudoers_target.clone(),
                sudoers_policy,
            ),
            validate_sudoers_command("validate_sudoers", helper_runtime.sudoers_target.clone()),
        ])
        .collect()
}

fn prepare_directory_commands_for_runtime(
    helper_runtime: &BoundedHelperInstallRuntime,
    directory_policy: InstallPathPolicy,
) -> Vec<HostCommandSurface> {
    vec![
        install_dir_command(
            "prepare_directory",
            Path::new(&helper_runtime.helper_target)
                .parent()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| helper_runtime.helper_dir.clone()),
            directory_policy.clone(),
        ),
        install_dir_command(
            "prepare_directory",
            Path::new(&helper_runtime.sudoers_target)
                .parent()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| helper_runtime.sudoers_dir.clone()),
            directory_policy,
        ),
    ]
}

fn helper_binary_source() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let source = std::env::current_exe()?;
<<<<<<< HEAD
    resolve_helper_binary_source(
        &source,
        Path::new(&current_runtime_layout().host_exec_helper_path()),
    )
}

fn resolve_helper_binary_source(
    current_executable: &Path,
    installed_seed: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut candidates = vec![current_executable.to_path_buf()];
    if !current_executable.ends_with(HOST_EXEC_HELPER_FILENAME) {
        candidates.push(current_executable.with_file_name(HOST_EXEC_HELPER_FILENAME));
        if let Some(parent) = current_executable.parent().and_then(Path::parent) {
            candidates.push(parent.join(HOST_EXEC_HELPER_FILENAME));
        }
    }
    // The runtime seed is independently governed and intentionally excluded
    // from mutable baskets. It is the only valid fallback when a CLI is
    // present without its sibling helper, the n-1 condition this bridge
    // exists to repair.
    candidates.push(installed_seed.to_path_buf());

    candidates
        .into_iter()
        .find(|candidate| valid_helper_binary(candidate))
        .ok_or_else(|| {
            format!(
                "unable to resolve executable cynic-host-exec from current executable {} or runtime seed {}",
                current_executable.display(),
                installed_seed.display()
            )
            .into()
        })
}

fn valid_helper_binary(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .ok()
        .is_some_and(|metadata| {
            metadata.file_type().is_file()
                && !metadata.file_type().is_symlink()
                && std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o111 != 0
        })
=======
    if !source.ends_with("cynic-host-exec") {
        let sibling = source.with_file_name(HOST_EXEC_HELPER_FILENAME);
        if sibling.exists() {
            return Ok(sibling);
        }
        if let Some(parent) = source.parent().and_then(Path::parent) {
            let target_debug = parent.join(HOST_EXEC_HELPER_FILENAME);
            if target_debug.exists() {
                return Ok(target_debug);
            }
        }
        return Err(format!(
            "unable to resolve cynic-host-exec near current executable: {}",
            source.display()
        )
        .into());
    }
    Ok(source)
>>>>>>> github/main
}

fn bundle_complete(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let required = [
        "manifest.json",
        HOST_EXEC_HELPER_FILENAME,
        HOST_EXEC_SUDOERS_FILENAME,
    ];
    path.exists() && required.iter().all(|name| path.join(name).exists())
}

fn bridge_bundle_context(
    projection: BoundedHelperProjection,
    helper_runtime: BoundedHelperInstallRuntime,
) -> BridgeBundleContext {
    BridgeBundleContext {
        helper_filename: HOST_EXEC_HELPER_FILENAME,
        sudoers_filename: HOST_EXEC_SUDOERS_FILENAME,
        helper_runtime,
        projection,
    }
}

fn host_exec_helper_path() -> String {
    current_runtime_layout().host_exec_helper_path()
}

fn host_exec_sudoers_path() -> String {
    current_runtime_layout().host_exec_sudoers_path()
}

fn helper_projection() -> BoundedHelperProjection {
    helper_projection_for_paths(host_exec_helper_path(), host_exec_sudoers_path())
}

fn helper_projection_for_paths(
    helper_path: impl Into<String>,
    sudoers_path: impl Into<String>,
) -> BoundedHelperProjection {
    let helper_path = helper_path.into();
    BoundedHelperProjection::new(
        HOST_EXEC_SUDOERS_SUBJECT,
        helper_path.clone(),
        sudoers_path,
        "Allows the cynic service user to invoke only bounded host-exec subprofiles.",
    )
    .with_allowed_invocations(vec![
        vec![helper_path.clone(), "bridge-status".to_string()],
        vec![helper_path.clone(), "principal-prepare".to_string()],
        vec![
            helper_path.clone(),
            "hardware-baseline-enroll".to_string(),
            "--apply".to_string(),
        ],
        vec![
            helper_path.clone(),
            "service-reconcile".to_string(),
            "--apply".to_string(),
        ],
        vec![
            helper_path.clone(),
            "network-baseline".to_string(),
            "observe".to_string(),
        ],
        vec![helper_path.clone(), "network-actuate".to_string()],
    ])
}

fn helper_install_runtime() -> BoundedHelperInstallRuntime {
    BoundedHelperInstallRuntime {
        helper_target: host_exec_helper_path(),
        sudoers_target: host_exec_sudoers_path(),
        helper_dir: current_runtime_layout().helper_dir,
        sudoers_dir: current_runtime_layout().sudoers_dir,
    }
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
        .unwrap_or_else(|| HOST_EXEC_SUDOERS_SUBJECT.to_string())
}

fn observed_group() -> String {
    capture_argv(&["id".to_string(), "-gn".to_string()])
        .ok()
        .filter(|output| output.success && !output.stdout.is_empty())
        .map(|output| output.stdout)
        .unwrap_or_else(observed_user)
}
