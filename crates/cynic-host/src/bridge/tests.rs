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
