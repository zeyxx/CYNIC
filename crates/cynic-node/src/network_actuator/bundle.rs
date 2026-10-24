use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use cynic_protocol::install_contract::{
    MODE_0644, MODE_0755, ROOT_OWNERSHIP, mode_map, owner_string, root_owned_path,
};

use crate::network_actuator::policy::build_policy_document;
use crate::network_actuator::types::{
    NetworkActuatorActivationPlan, NetworkActuatorBundleManifest,
};
use crate::network_actuator_contract::{
    NETWORK_ACTUATOR_MANIFEST_FILENAME, NETWORK_ACTUATOR_POLICY_FILENAME, bundle_required_files,
};
use crate::network_command_surface::{
    NetworkCommandSurface, expectation, install_dir_command, install_file_command,
};
use crate::observe::NodeObservation;

pub mod commands;
pub mod context;

use commands::{denial_checks, postflight_commands, preflight_commands, success_criteria};
use context::NetworkActuatorBundleContext;

pub fn write_install_bundle(
    observation: &NodeObservation,
    output_dir: impl AsRef<Path>,
) -> Result<NetworkActuatorBundleManifest, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    // No helper binary is staged: the executor is the basket-managed `cynic`
    // binary. No sudoers file is staged either: the bridge owns the single
    // membrane and the node only verifies it admits network-actuate.
    let policy_target = output_dir.join(NETWORK_ACTUATOR_POLICY_FILENAME);
    let manifest_target = output_dir.join(NETWORK_ACTUATOR_MANIFEST_FILENAME);

    let policy = build_policy_document(observation);
    fs::write(
        &policy_target,
        format!("{}\n", serde_json::to_string_pretty(&policy)?),
    )?;
    fs::set_permissions(&policy_target, fs::Permissions::from_mode(MODE_0644.unix))?;

    let manifest = install_bundle_manifest_with_context(
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

pub fn activation_plan(
    _observation: &NodeObservation,
    bundle_dir: impl AsRef<Path>,
) -> NetworkActuatorActivationPlan {
    let bundle_dir = bundle_dir.as_ref();
    let context = NetworkActuatorBundleContext::from_current_runtime();
    let install_plan = install_bundle_manifest_with_context(
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
                install_file_command(
                    "install_policy",
                    format!("{bundle_dir_str}/{NETWORK_ACTUATOR_POLICY_FILENAME}"),
                    context.policy_target.clone(),
                    root_owned_path(MODE_0644),
                ),
                expectation(
                    "verify_membrane_admits_network_actuate",
                    format!(
                        "membrane {} contains the admission {} with root:root 0440",
                        context.membrane_target, context.membrane_admission
                    ),
                ),
            ])
            .collect(),
        postflight: postflight_commands(context),
        denial_checks: denial_checks(context),
        success_criteria: success_criteria(),
    }
}

fn install_bundle_manifest_with_context(
    policy_source: &str,
    bundle_dir: &str,
    context: &NetworkActuatorBundleContext,
) -> NetworkActuatorBundleManifest {
    let prepare_directories = prepare_directories(context);
    let bundle_files = bundle_files(bundle_dir);
    NetworkActuatorBundleManifest {
        schema: "cynic.network_actuator.install_plan.v1",
        mutates: false,
        executor: context.executor.clone(),
        policy_source: policy_source.to_string(),
        policy_target: context.policy_target.clone(),
        membrane_target: context.membrane_target.clone(),
        membrane_admission: context.membrane_admission.clone(),
        prepare_directories: prepare_directories.clone(),
        required_owner: owner_string(ROOT_OWNERSHIP),
        required_modes: bundle_required_modes(),
        activation_checks: activation_checks(prepare_directories, context),
        bundle_dir: bundle_dir.to_string(),
        bundle_files,
    }
}

fn bundle_required_modes() -> std::collections::BTreeMap<&'static str, String> {
    mode_map(&[("policy", MODE_0644)])
}

fn bundle_files(bundle_dir: &str) -> std::collections::BTreeMap<&'static str, String> {
    std::collections::BTreeMap::from([
        (
            "policy",
            format!("{bundle_dir}/{NETWORK_ACTUATOR_POLICY_FILENAME}"),
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
    prepare_directories
        .into_iter()
        .chain([
            expectation(
                "install_policy",
                "install policy_source -> policy_target with root:root 0644",
            ),
            expectation(
                "verify_membrane_admits_network_actuate",
                format!(
                    "membrane {} contains the admission {}",
                    context.membrane_target, context.membrane_admission
                ),
            ),
            expectation(
                "deny_direct_route",
                "direct route invocation must remain unauthorized for the cynic user",
            ),
        ])
        .collect()
}

fn prepare_directories(context: &NetworkActuatorBundleContext) -> Vec<NetworkCommandSurface> {
    vec![install_dir_command(
        "prepare_directory",
        Path::new(&context.policy_target)
            .parent()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| context.config_dir.clone()),
        root_owned_path(MODE_0755),
    )]
}

fn bundle_complete(bundle_dir: &Path) -> bool {
    bundle_required_files()
        .iter()
        .all(|file| bundle_dir.join(file).is_file())
}

#[cfg(test)]
mod tests {
    use super::{activation_plan_with_context, install_bundle_manifest_with_context};
    use crate::network_actuator::bundle::context::NetworkActuatorBundleContext;
    use crate::network_command_surface::NetworkCommandInvocation;
    use std::path::Path;

    fn test_context() -> NetworkActuatorBundleContext {
        NetworkActuatorBundleContext {
            executor: "/portable/bin/cynic".to_string(),
            policy_target: "/portable/etc/cynic/network-actuator.policy.json".to_string(),
            membrane_target: "/portable/etc/sudoers.d/cynic".to_string(),
            membrane_admission:
                "/portable/bin/cynic privilege exec network-actuate".to_string(),
            config_dir: "/portable/etc/cynic".to_string(),
            sudo_bin: "/portable/bin/sudo".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            wireguard_bin: "/portable/bin/wg".to_string(),
            denied_shell_bin: "/portable/bin/sh".to_string(),
        }
    }

    #[test]
    fn install_manifest_uses_injected_targets() {
        let manifest = install_bundle_manifest_with_context(
            "/tmp/bundle/network-actuator.policy.json",
            "/tmp/bundle",
            &test_context(),
        );

        assert_eq!(manifest.executor, "/portable/bin/cynic");
        assert_eq!(
            manifest.policy_target,
            "/portable/etc/cynic/network-actuator.policy.json"
        );
        assert_eq!(
            manifest.membrane_target,
            "/portable/etc/sudoers.d/cynic"
        );
        assert!(
            !manifest
                .bundle_files
                .values()
                .any(|file| file.contains("cynic-network-actuator")),
            "no helper binary is staged: {:?}",
            manifest.bundle_files
        );
    }

    #[test]
    fn activation_plan_installs_policy_but_no_helper_or_sudoers() {
        let manifest = install_bundle_manifest_with_context(
            "/tmp/bundle/network-actuator.policy.json",
            "/tmp/bundle",
            &test_context(),
        );
        let plan =
            activation_plan_with_context(Path::new("/tmp/bundle"), manifest, &test_context());

        assert_eq!(plan.bundle_dir, "/tmp/bundle");
        let installed: Vec<&str> = plan
            .install_commands
            .iter()
            .filter_map(|command| match &command.invocation {
                NetworkCommandInvocation::InstallFile { target, .. } => Some(target.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(installed, ["/portable/etc/cynic/network-actuator.policy.json"]);
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
