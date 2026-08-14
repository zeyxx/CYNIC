use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use crate::bridge::{PrivilegeBridgeInstallSpec, PrivilegeBridgeObservedState};
use crate::process_observation::{command_exit_success, command_stdout};
use crate::runtime_layout::current_runtime_layout;

pub(super) fn observe(spec: &PrivilegeBridgeInstallSpec) -> PrivilegeBridgeObservedState {
    let layout = current_runtime_layout();
    let subject_present =
        command_stdout(&layout.getent_bin(), &["passwd", &spec.subject]).is_some();
    let authority_uid = identity_id("passwd", &spec.authority_user, 2);
    let authority_gid = identity_id("group", &spec.authority_group, 2);
    let helper_metadata = fs::metadata(&spec.helper_target).ok();
    let projection_metadata = fs::metadata(&spec.projection_target).ok();
    let projection_content = fs::read_to_string(&spec.projection_target).ok();
    let provider_available = provider_available(&spec.provider);

    PrivilegeBridgeObservedState {
        subject_present,
        provider_available,
        helper_present: helper_metadata.is_some(),
        helper_executable: helper_metadata
            .as_ref()
            .is_some_and(|metadata| metadata.permissions().mode() & 0o111 != 0),
        helper_mode_matches: helper_metadata
            .as_ref()
            .is_some_and(|metadata| metadata.permissions().mode() & 0o7777 == spec.helper_mode),
        helper_authority_owned: metadata_owned_by(
            helper_metadata.as_ref(),
            authority_uid,
            authority_gid,
        ),
        projection_present: projection_metadata.is_some(),
        projection_mode_matches: projection_metadata
            .as_ref()
            .is_some_and(|metadata| metadata.permissions().mode() & 0o7777 == spec.projection_mode),
        projection_authority_owned: metadata_owned_by(
            projection_metadata.as_ref(),
            authority_uid,
            authority_gid,
        ),
        projection_valid: provider_available
            && validate_projection(&spec.provider, &spec.projection_target),
        invocation_policy_bounded: projection_content.as_deref()
            == Some(spec.expected_projection.as_str()),
        bounded_helper_noninteractive: prove_bounded_invocation(spec),
        unrestricted_shell_denied: prove_unrestricted_shell_denied(spec),
    }
}

fn identity_id(database: &str, name: &str, field: usize) -> Option<u32> {
    command_stdout(&current_runtime_layout().getent_bin(), &[database, name])
        .as_deref()
        .and_then(|line| line.split(':').nth(field))
        .and_then(|value| value.parse().ok())
}

fn metadata_owned_by(
    metadata: Option<&fs::Metadata>,
    expected_uid: Option<u32>,
    expected_gid: Option<u32>,
) -> bool {
    let (Some(metadata), Some(uid), Some(gid)) = (metadata, expected_uid, expected_gid) else {
        return false;
    };
    metadata.uid() == uid && metadata.gid() == gid
}

fn provider_available(provider: &str) -> bool {
    let layout = current_runtime_layout();
    match provider {
        "sudoers" => {
            command_exit_success(&layout.which_bin(), &[&layout.visudo_bin()]) == Some(true)
                && command_exit_success(&layout.which_bin(), &[&layout.sudo_bin()]) == Some(true)
        }
        _ => false,
    }
}

fn validate_projection(provider: &str, path: &str) -> bool {
    match provider {
        "sudoers" => {
            command_exit_success(&current_runtime_layout().visudo_bin(), &["-cf", path])
                == Some(true)
        }
        _ => false,
    }
}

fn prove_bounded_invocation(spec: &PrivilegeBridgeInstallSpec) -> bool {
    if spec.provider != "sudoers" || spec.proof_invocation.is_empty() {
        return false;
    }
    let layout = current_runtime_layout();
    let mut argv = vec![
        "-u".to_string(),
        spec.subject.clone(),
        layout.sudo_bin(),
        "-n".to_string(),
    ];
    argv.extend(spec.proof_invocation.clone());
    command_exit_success(
        &layout.sudo_bin(),
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    ) == Some(true)
}

fn prove_unrestricted_shell_denied(spec: &PrivilegeBridgeInstallSpec) -> bool {
    if spec.provider != "sudoers" {
        return false;
    }
    let layout = current_runtime_layout();
    command_exit_success(
        &layout.sudo_bin(),
        &[
            "-u",
            &spec.subject,
            &layout.sudo_bin(),
            "-n",
            "/bin/sh",
            "-c",
            "true",
        ],
    ) == Some(false)
}
