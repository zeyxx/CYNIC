use std::fs;

use super::*;
use crate::command_surface::install_dir_command;
use crate::install_contract::root_owned_path;

#[test]
fn bridge_install_requires_apply() {
    let bundle_dir = std::env::temp_dir().join(format!("cynic-bridge-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&bundle_dir);
    fs::create_dir_all(&bundle_dir).expect("create bundle dir");
    fs::write(bundle_dir.join("manifest.json"), "{}").expect("manifest");
    fs::write(bundle_dir.join(HOST_EXEC_HELPER_FILENAME), "").expect("helper");
    fs::write(bundle_dir.join(HOST_EXEC_SUDOERS_FILENAME), "").expect("sudoers");

    let report = bridge_install_report(&bundle_dir, false);

    assert_eq!(report.status, "blocked");
    assert!(
        report
            .blockers
            .contains(&"explicit_apply_required".to_string())
    );
    let _ = fs::remove_dir_all(&bundle_dir);
}

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
fn runtime_projection_never_delegates_human_admin_principal_apply() {
    let projection =
        helper_projection_for_paths("/portable/cynic-host-exec", "/portable/privilege-policy");
    let rule = projection.sudoers_rule();

    assert!(rule.contains("principal-prepare"));
    assert!(!rule.contains("principal-prepare --apply"));
    assert!(rule.contains(
        "cynic ALL=(root) NOPASSWD: /portable/cynic-host-exec service-reconcile --apply"
    ));
    assert!(!rule.contains("service-reconcile *"));
    assert!(rule.contains("cynic ALL=(root) NOPASSWD: /portable/cynic-host-exec network-actuate"));
}

#[test]
fn install_validates_the_staged_sudoers_before_installing_it() {
    // Regression lock. The order used to be install-then-validate, and the
    // validation targeted the INSTALLED path. A sudoers file that fails
    // `visudo -c` would land in /etc/sudoers.d/ before being checked, which can
    // break `sudo` entirely — including the ability to fix it.
    let runtime = BoundedHelperInstallRuntime {
        helper_target: "/tmp/cynic-bridge-test/libexec/cynic-host-exec".to_string(),
        sudoers_target: "/tmp/cynic-bridge-test/sudoers.d/cynic-host-exec".to_string(),
        helper_dir: "/tmp/cynic-bridge-test/libexec".to_string(),
        sudoers_dir: "/tmp/cynic-bridge-test/sudoers.d".to_string(),
    };
    let commands = install_commands_for_runtime(
        &runtime,
        "/tmp/cynic-bridge-test/bundle",
        root_owned_path(MODE_0755),
        root_owned_path(MODE_0440),
    );

    let roles: Vec<&str> = commands.iter().map(|surface| surface.role.as_str()).collect();
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
        !staged_path.contains("/tmp/cynic-bridge-test/sudoers.d"),
        "must NOT validate the installed path: {staged_path}"
    );
}

#[test]
fn bridge_bundle_can_use_the_independently_installed_runtime_seed() {
    let root = std::env::temp_dir().join(format!("cynic-bridge-seed-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create seed root");
    let cli = root.join("cynic");
    let seed = root.join(HOST_EXEC_HELPER_FILENAME);
    fs::write(&cli, "cli").expect("write cli");
    fs::write(&seed, "seed").expect("write seed");
    let mut permissions = fs::metadata(&seed).expect("seed metadata").permissions();
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(0o755);
    fs::set_permissions(&seed, permissions).expect("make seed executable");

    assert_eq!(
        resolve_helper_binary_source(&cli, &seed).expect("resolve seed"),
        seed
    );
    let _ = fs::remove_dir_all(&root);
}
