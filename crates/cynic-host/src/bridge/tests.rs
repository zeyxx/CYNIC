use super::*;
use crate::command_surface::install_dir_command;
use crate::install_contract::root_owned_path;

#[test]
fn bridge_install_dir_surface_becomes_argv_without_shell() {
    let command = install_dir_command(
        "prepare_directory",
        "/tmp/cynic-test",
        root_owned_path(MODE_0755),
    );

    let argv = crate::execution::command_to_argv(&command).expect("install_dir is executable");

    assert_eq!(
        argv,
        vec![
            "install",
            "-d",
            "-o",
            "root",
            "-g",
            "root",
            "-m",
            "0755",
            "/tmp/cynic-test"
        ]
    );
}

#[test]
fn install_validates_the_staged_membrane_before_installing_it() {
    // Regression lock. The order used to be install-then-validate, and the
    // validation targeted the INSTALLED path. A sudoers file that fails
    // `visudo -c` would land in /etc/sudoers.d/ before being checked, which can
    // break `sudo` entirely — including the ability to fix it.
    // The membrane install moves no binary: the executor is basket-managed.
    let commands = membrane_install_commands_for_runtime(
        "/tmp/cynic-bridge-test/bundle",
        "/tmp/cynic-bridge-test/sudoers.d/cynic",
        root_owned_path(MODE_0755),
        root_owned_path(MODE_0440),
    );

    let roles: Vec<&str> = commands
        .iter()
        .map(|surface| surface.role.as_str())
        .collect();
    assert!(
        !roles.contains(&"install_helper"),
        "no binary is installed by the membrane flow, got roles {roles:?}"
    );
    let validate_at = roles
        .iter()
        .position(|role| *role == "validate_sudoers")
        .expect("validate_sudoers present");
    let install_at = roles
        .iter()
        .position(|role| *role == "install_sudoers")
        .expect("install_sudoers present");
    assert!(
        validate_at < install_at,
        "validate must run BEFORE install, got order {roles:?}"
    );

    // Validation targets the STAGED file, never the installed one.
    let validate = &commands[validate_at];
    let staged_path = match &validate.invocation {
        crate::command_surface::HostCommandInvocation::ValidateSudoers { path } => path.clone(),
        other => panic!("expected ValidateSudoers, got {other:?}"),
    };
    assert!(
        staged_path.contains("/tmp/cynic-bridge-test/bundle"),
        "must validate the staged path: {staged_path}"
    );
    assert!(
        staged_path.ends_with("privilege.sudoers"),
        "must validate the staged membrane: {staged_path}"
    );
    assert!(
        !staged_path.contains("/tmp/cynic-bridge-test/sudoers.d"),
        "must NOT validate the installed path: {staged_path}"
    );
}
