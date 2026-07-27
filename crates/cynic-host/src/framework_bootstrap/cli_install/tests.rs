use std::path::{Path, PathBuf};

use super::{
    discover_source_binary, discover_source_binary_from_path, install_output_indicates_same_file,
    install_standalone_cli, is_user_local_target,
};

#[test]
fn standalone_cli_install_is_idempotent_when_source_is_target() {
    let current = std::env::current_exe().expect("current exe");
    let report = install_standalone_cli(
        Some(current.to_str().expect("utf8 path")),
        Some(current.to_str().expect("utf8 path")),
        true,
    )
    .expect("install cli report");

    assert_eq!(report.status, "already_installed");
    assert!(!report.mutates);
    assert!(report.requested_apply);
}

#[test]
fn standalone_cli_source_discovered_from_env_override() {
    let root = std::env::temp_dir().join(format!(
        "cynic-host-cli-install-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let expected = root.join("target").join("debug").join("cynic");
    std::fs::create_dir_all(expected.parent().expect("artifact parent"))
        .expect("create workspace artifact dir");
    std::fs::write(&expected, b"\n").expect("write dummy artifact");

    let previous = std::env::var("CYNIC_STANDALONE_CLI_SOURCE");
    // SAFETY: the test restores the process environment before returning.
    unsafe { std::env::set_var("CYNIC_STANDALONE_CLI_SOURCE", &expected) };
    let discovered = discover_source_binary().expect("source should be discovered");
    assert_eq!(discovered, expected);

    if let Ok(value) = previous {
        // SAFETY: restores the value captured above.
        unsafe { std::env::set_var("CYNIC_STANDALONE_CLI_SOURCE", value) };
    } else {
        // SAFETY: restores the absence captured above.
        unsafe { std::env::remove_var("CYNIC_STANDALONE_CLI_SOURCE") };
    }

    std::fs::remove_file(&expected).expect("cleanup artifact");
    std::fs::remove_dir_all(root.join("target")).expect("cleanup target");
}

#[test]
fn install_output_detects_same_file() {
    assert!(install_output_indicates_same_file(
        "",
        "install: '/usr/local/bin/cynic' et '/usr/local/bin/cynic' identifient le même fichier"
    ));
    assert!(install_output_indicates_same_file(
        "",
        "install: '/usr/local/bin/cynic' et '/usr/local/bin/cynic' identifient le meme fichier"
    ));
    assert!(install_output_indicates_same_file(
        "install: '/usr/local/bin/cynic' and '/usr/local/bin/cynic' are the same file",
        ""
    ));
    assert!(!install_output_indicates_same_file(
        "install: missing operand",
        "install: failed to create temporary file"
    ));
}

#[test]
fn arbitrary_user_target_is_treated_as_user_local() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    assert!(is_user_local_target(Path::new("/tmp/cynic-ci-test/cynic")));
    assert!(is_user_local_target(Path::new(&format!(
        "{home}/.local/bin/cynic"
    ))));
    assert!(is_user_local_target(Path::new("/mnt/custom/bin/cynic")));
}

#[test]
fn standalone_cli_source_is_discovered_from_workspace_layout() {
    let root = std::env::temp_dir().join(format!(
        "cynic-host-ci-workspace-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let expected = root.join("target").join("debug").join("cynic");
    std::fs::create_dir_all(expected.parent().expect("artifact parent"))
        .expect("create workspace artifact dir");
    std::fs::write(&expected, b"\n").expect("write dummy artifact");

    let discovered =
        discover_source_binary_from_path(&root).expect("workspace source should be discovered");
    assert_eq!(discovered, expected);

    std::fs::remove_file(&expected).expect("cleanup artifact");
    std::fs::remove_dir_all(root.join("target")).expect("cleanup target");
}

#[test]
fn system_targets_remain_system_mode() {
    assert!(!is_user_local_target(Path::new("/usr/local/bin/cynic")));
    assert!(!is_user_local_target(Path::new("/usr/bin/cynic")));
    assert!(!is_user_local_target(Path::new("/sbin/reboot")));
}

#[test]
fn default_install_target_is_system_wide_by_default() {
    let previous = std::env::var("CYNIC_STANDALONE_CLI").ok();
    if previous.is_some() {
        // SAFETY: the test restores the process environment before returning.
        unsafe { std::env::remove_var("CYNIC_STANDALONE_CLI") };
    }

    assert_eq!(
        super::default_cli_install_target(),
        PathBuf::from("/usr/local/bin/cynic")
    );

    if let Some(value) = previous {
        // SAFETY: restores the value captured above.
        unsafe { std::env::set_var("CYNIC_STANDALONE_CLI", value) };
    }
}
