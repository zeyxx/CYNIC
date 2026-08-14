use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use cynic_host::bounded_helper::{
    BoundedHelperInstallRuntime, bundle_complete as helper_bundle_complete,
};
use cynic_host::install_contract::{
    MODE_0440, MODE_0644, MODE_0755, ROOT_OWNERSHIP, mode_map, owner_string, root_owned_path,
};
use cynic_host::registry::DEFAULT_REGISTRY_PATH;

use crate::network_actuator::policy::build_policy_document;
use crate::network_actuator::types::{
    NetworkActuatorActivationPlan, NetworkActuatorBundleManifest,
};
use crate::network_actuator_contract::{
    NETWORK_ACTUATOR_HELPER_FILENAME, NETWORK_ACTUATOR_MANIFEST_FILENAME,
    NETWORK_ACTUATOR_POLICY_FILENAME, NETWORK_ACTUATOR_SUDOERS_FILENAME, bundle_required_files,
    sudoers_projection,
};
use crate::network_command_surface::{
    NetworkCommandSurface, expectation, from_host_command, install_dir_command,
    install_file_command, render_command,
};
use crate::observe::NodeObservation;

mod commands;
mod context;

use commands::{
    denial_checks, deny_route_command, postflight_commands, preflight_commands, success_criteria,
};
use context::{NetworkActuatorBundleContext, helper_install_runtime};

pub(crate) fn write_install_bundle(
    observation: &NodeObservation,
    output_dir: impl AsRef<Path>,
) -> Result<NetworkActuatorBundleManifest, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let helper_target = output_dir.join(NETWORK_ACTUATOR_HELPER_FILENAME);
    let policy_target = output_dir.join(NETWORK_ACTUATOR_POLICY_FILENAME);
    let sudoers_target = output_dir.join(NETWORK_ACTUATOR_SUDOERS_FILENAME);
    let manifest_target = output_dir.join(NETWORK_ACTUATOR_MANIFEST_FILENAME);

    prepare_bundle_target(&helper_target)?;
    prepare_bundle_target(&policy_target)?;
    prepare_bundle_target(&sudoers_target)?;

    let helper_source = helper_binary_source()?;
    fs::copy(&helper_source, &helper_target)?;
    fs::set_permissions(&helper_target, fs::Permissions::from_mode(MODE_0755.unix))?;

    let policy = build_policy_document(observation);
    fs::write(
        &policy_target,
        format!("{}\n", serde_json::to_string_pretty(&policy)?),
    )?;
    fs::set_permissions(&policy_target, fs::Permissions::from_mode(MODE_0644.unix))?;

    fs::write(&sudoers_target, sudoers_projection().as_bytes())?;
    fs::set_permissions(&sudoers_target, fs::Permissions::from_mode(MODE_0440.unix))?;

    let manifest = install_bundle_manifest_with_context(
        &helper_source.to_string_lossy(),
        &policy_target.to_string_lossy(),
        &output_dir.to_string_lossy(),
        &NetworkActuatorBundleContext::from_current_runtime(),
    );
    fs::write(
        &manifest_target,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    fs::set_permissions(&manifest_target, fs::Permissions::from_mode(MODE_0644.unix))?;
    Ok(manifest)
}

pub(crate) fn activation_plan(
    _observation: &NodeObservation,
    bundle_dir: impl AsRef<Path>,
) -> NetworkActuatorActivationPlan {
    let bundle_dir = bundle_dir.as_ref();
    let context = NetworkActuatorBundleContext::from_current_runtime();
    let install_plan = install_bundle_manifest_with_context(
        "<current_executable>",
        &bundle_dir
            .join("network-actuator.policy.json")
            .to_string_lossy(),
        &bundle_dir.to_string_lossy(),
        &context,
    );
    activation_plan_with_context(bundle_dir, install_plan, &context)
}

fn activation_plan_with_context(
    bundle_dir: &Path,
    install_plan: NetworkActuatorBundleManifest,
    context: &NetworkActuatorBundleContext,
) -> NetworkActuatorActivationPlan {
    let bundle_dir_str = bundle_dir.to_string_lossy().to_string();
    let bundle_files = install_plan.bundle_files.clone();
    let helper_runtime = helper_install_runtime(context);
    NetworkActuatorActivationPlan {
        schema: "cynic.network_actuator.activation_plan.v1",
        mutates: false,
        bundle_dir: bundle_dir_str.clone(),
        bundle_expected_complete: bundle_complete(bundle_dir),
        bundle_files,
        preflight: preflight_commands(&bundle_dir_str),
        install_commands: install_plan
            .prepare_directories
            .into_iter()
            .chain([
                install_dir_command(
                    "prepare_directory",
                    context.config_dir.clone(),
                    root_owned_path(MODE_0755),
                ),
                from_host_command(
                    helper_runtime
                        .install_helper_command(&bundle_dir_str, NETWORK_ACTUATOR_HELPER_FILENAME),
                ),
                install_file_command(
                    "install_policy",
                    format!("{bundle_dir_str}/{NETWORK_ACTUATOR_POLICY_FILENAME}"),
                    context.policy_target.clone(),
                    root_owned_path(MODE_0644),
                ),
                from_host_command(
                    helper_runtime.install_sudoers_command(
                        &bundle_dir_str,
                        NETWORK_ACTUATOR_SUDOERS_FILENAME,
                    ),
                ),
                from_host_command(helper_runtime.validate_sudoers_command()),
            ])
            .collect(),
        postflight: postflight_commands(context),
        denial_checks: denial_checks(context),
        success_criteria: success_criteria(),
    }
}

fn install_bundle_manifest_with_context(
    helper_source: &str,
    policy_source: &str,
    bundle_dir: &str,
    context: &NetworkActuatorBundleContext,
) -> NetworkActuatorBundleManifest {
    let helper_runtime = helper_install_runtime(context);
    let prepare_directories = prepare_directories(context);
    let required_modes = bundle_required_modes(&helper_runtime);
    let bundle_files = bundle_files(bundle_dir);
    NetworkActuatorBundleManifest {
        schema: "cynic.network_actuator.install_plan.v1",
        mutates: false,
        helper_source: helper_source.to_string(),
        helper_target: context.helper_target.clone(),
        policy_source: policy_source.to_string(),
        policy_target: context.policy_target.clone(),
        sudoers_target: context.sudoers_target.clone(),
        sudoers_rule: context.sudoers_rule.clone(),
        prepare_directories: prepare_directories.clone(),
        required_owner: owner_string(ROOT_OWNERSHIP),
        required_modes,
        activation_checks: activation_checks(prepare_directories, context),
        bundle_dir: bundle_dir.to_string(),
        bundle_files,
    }
}

fn bundle_required_modes(
    helper_runtime: &BoundedHelperInstallRuntime,
) -> BTreeMap<&'static str, String> {
    mode_map(&[("helper", MODE_0755), ("policy", MODE_0644)])
        .into_iter()
        .chain(helper_runtime.required_modes())
        .collect()
}

fn bundle_files(bundle_dir: &str) -> BTreeMap<&'static str, String> {
    BTreeMap::from([
        (
            "helper",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_HELPER_FILENAME}"),
        ),
        (
            "policy",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_POLICY_FILENAME}"),
        ),
        (
            "sudoers",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_SUDOERS_FILENAME}"),
        ),
        (
            "manifest",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_MANIFEST_FILENAME}"),
        ),
    ])
}

fn activation_checks(
    prepare_directories: Vec<NetworkCommandSurface>,
    context: &NetworkActuatorBundleContext,
) -> Vec<NetworkCommandSurface> {
    let deny_direct_route = deny_route_command(context);
    prepare_directories
        .into_iter()
        .chain([
            expectation(
                "install_helper",
                "install helper_source -> helper_target with root:root 0755",
            ),
            expectation(
                "install_policy",
                "install policy_source -> policy_target with root:root 0644",
            ),
            expectation(
                "install_sudoers",
                "install rendered_sudoers -> sudoers_target with root:root 0440",
            ),
            expectation("validate_sudoers", "validate sudoers_target with visudo -cf"),
            expectation(
                "prove_profile_apply",
                format!(
                    "sudo -n helper_target apply --profile synapse-route --registry {DEFAULT_REGISTRY_PATH} --apply"
                ),
            ),
            expectation(
                "deny_direct_route",
                format!(
                    "{} must remain unauthorized for the cynic user",
                    render_command(&deny_direct_route)
                ),
            ),
        ])
        .collect()
}

fn prepare_directories(context: &NetworkActuatorBundleContext) -> Vec<NetworkCommandSurface> {
    let mut commands = helper_install_runtime(context)
        .prepare_directories()
        .into_iter()
        .map(from_host_command)
        .collect::<Vec<_>>();
    commands.push(install_dir_command(
        "prepare_directory",
        Path::new(&context.policy_target)
            .parent()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| context.config_dir.clone()),
        root_owned_path(MODE_0755),
    ));
    commands
}

fn prepare_bundle_target(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn helper_binary_source() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    if current_exe.file_name().and_then(|name| name.to_str()) == Some("cynic-network-actuator") {
        return Ok(current_exe);
    }

    let sibling = current_exe
        .parent()
        .map(|parent| parent.join(NETWORK_ACTUATOR_HELPER_FILENAME))
        .ok_or("missing executable parent directory")?;
    if sibling.exists() {
        return Ok(sibling);
    }

    Err(format!(
        "missing helper binary at {}; build it first with `cargo build -p cynic-node --bin {NETWORK_ACTUATOR_HELPER_FILENAME}`",
        sibling.display()
    )
    .into())
}

fn bundle_complete(bundle_dir: &Path) -> bool {
    helper_bundle_complete(bundle_dir, &bundle_required_files())
}

#[cfg(test)]
mod tests {
    use super::{
        NetworkActuatorBundleContext, activation_plan_with_context,
        install_bundle_manifest_with_context,
    };
    use crate::network_command_surface::NetworkCommandInvocation;
    use std::path::Path;

    fn test_context() -> NetworkActuatorBundleContext {
        NetworkActuatorBundleContext {
            helper_target: "/portable/bin/cynic-network-actuator".to_string(),
            policy_target: "/portable/etc/cynic/network-actuator.policy.json".to_string(),
            sudoers_target: "/portable/etc/sudoers.d/cynic-network-actuator".to_string(),
            sudoers_rule: "cynic ALL=(root) NOPASSWD: /portable/bin/cynic-network-actuator"
                .to_string(),
            helper_dir: "/portable/bin".to_string(),
            config_dir: "/portable/etc/cynic".to_string(),
            sudoers_dir: "/portable/etc/sudoers.d".to_string(),
            sudo_bin: "/portable/bin/sudo".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            wireguard_bin: "/portable/bin/wg".to_string(),
            denied_shell_bin: "/portable/bin/sh".to_string(),
        }
    }

    #[test]
    fn install_manifest_uses_injected_targets() {
        let manifest = install_bundle_manifest_with_context(
            "/tmp/build/cynic-network-actuator",
            "/tmp/build/network-actuator.policy.json",
            "/tmp/bundle",
            &test_context(),
        );

        assert_eq!(
            manifest.helper_target,
            "/portable/bin/cynic-network-actuator"
        );
        assert_eq!(
            manifest.policy_target,
            "/portable/etc/cynic/network-actuator.policy.json"
        );
        assert_eq!(
            manifest.sudoers_target,
            "/portable/etc/sudoers.d/cynic-network-actuator"
        );
        assert!(
            manifest
                .activation_checks
                .iter()
                .any(|item| matches!(&item.invocation, NetworkCommandInvocation::Expectation { text } if text.contains("/portable/bin/sudo -n /portable/bin/ip route show")))
        );
    }

    #[test]
    fn activation_plan_uses_injected_install_context() {
        let manifest = install_bundle_manifest_with_context(
            "<current_executable>",
            "/tmp/bundle/network-actuator.policy.json",
            "/tmp/bundle",
            &test_context(),
        );
        let plan =
            activation_plan_with_context(Path::new("/tmp/bundle"), manifest, &test_context());

        assert_eq!(plan.bundle_dir, "/tmp/bundle");
        let helper_install =
            plan.install_commands
                .iter()
                .find_map(|command| match &command.invocation {
                    NetworkCommandInvocation::InstallFile { target, .. }
                        if target == "/portable/bin/cynic-network-actuator" =>
                    {
                        Some(target)
                    }
                    _ => None,
                });
        assert_eq!(
            helper_install,
            Some(&"/portable/bin/cynic-network-actuator".to_string())
        );
        match &plan.denial_checks[0].invocation {
            NetworkCommandInvocation::Argv { argv } => {
                assert_eq!(argv[0], "/portable/bin/sudo");
                assert_eq!(argv[2], "/portable/bin/ip");
            }
            _ => panic!("expected argv command"),
        }
        match &plan.denial_checks[2].invocation {
            NetworkCommandInvocation::Argv { argv } => {
                assert_eq!(argv[2], "/portable/bin/sh");
            }
            _ => panic!("expected argv command"),
        }
    }
}
