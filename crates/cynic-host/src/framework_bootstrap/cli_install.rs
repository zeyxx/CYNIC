use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

use super::types::StandaloneCliInstallReport;

pub fn install_standalone_cli(
    source: Option<&str>,
    target: Option<&str>,
    apply: bool,
) -> Result<StandaloneCliInstallReport, String> {
    let source = match source {
        Some(path) => PathBuf::from(path),
        None => discover_source_binary().ok_or_else(|| {
            "failed to discover a source binary for standalone CLI install".to_string()
        })?,
    };
    let target = target
        .map(PathBuf::from)
        .unwrap_or_else(default_cli_install_target);
    let target_display = target.display().to_string();
    let user_local = is_user_local_target(&target);
    let command = if user_local {
        // No root ownership: $HOME/.local/bin (or other user-writable prefix).
        vec![
            "install".to_string(),
            "-m".to_string(),
            "0755".to_string(),
            source.display().to_string(),
            target_display.clone(),
        ]
    } else {
        vec![
            "install".to_string(),
            "-o".to_string(),
            "root".to_string(),
            "-g".to_string(),
            "root".to_string(),
            "-m".to_string(),
            "0755".to_string(),
            source.display().to_string(),
            target_display.clone(),
        ]
    };
    let source_display = source.display().to_string();

    if !apply {
        return Ok(StandaloneCliInstallReport {
            schema: "cynic.framework_bootstrap.standalone_cli_install.v1",
            status: "planned",
            mutates: false,
            requested_apply: false,
            source: source_display,
            target: target_display,
            command,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            proof: standalone_cli_install_proof(),
        });
    }

    if same_existing_file(&source, &target) {
        return Ok(StandaloneCliInstallReport {
            schema: "cynic.framework_bootstrap.standalone_cli_install.v1",
            status: "already_installed",
            mutates: false,
            requested_apply: true,
            source: source_display,
            target: target_display,
            command,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            proof: standalone_cli_install_proof(),
        });
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create install directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let source_str = source
        .to_str()
        .ok_or_else(|| format!("source path is not valid utf-8: {}", source.display()))?;
    let output = if user_local {
        Command::new("install")
            .args(["-m", "0755", source_str, &target_display])
            .output()
    } else {
        Command::new("install")
            .args([
                "-o",
                "root",
                "-g",
                "root",
                "-m",
                "0755",
                source_str,
                &target_display,
            ])
            .output()
    }
    .map_err(|error| format!("failed to execute install: {error}"))?;

    let output_stdout = String::from_utf8_lossy(&output.stdout);
    let output_stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success()
        && install_output_indicates_same_file(output_stdout.as_ref(), output_stderr.as_ref())
    {
        return Ok(StandaloneCliInstallReport {
            schema: "cynic.framework_bootstrap.standalone_cli_install.v1",
            status: "already_installed",
            mutates: false,
            requested_apply: true,
            source: source_display,
            target: target_display,
            command,
            exit_code: None,
            stdout: output_stdout.trim().to_string(),
            stderr: output_stderr.trim().to_string(),
            proof: standalone_cli_install_proof(),
        });
    }

    let status = if output.status.success() {
        "installed"
    } else {
        "blocked"
    };

    Ok(StandaloneCliInstallReport {
        schema: "cynic.framework_bootstrap.standalone_cli_install.v1",
        status,
        mutates: true,
        requested_apply: true,
        source: source_display,
        target: target_display,
        command,
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        proof: standalone_cli_install_proof(),
    })
}

fn default_cli_install_target() -> PathBuf {
    if let Ok(p) = std::env::var("CYNIC_STANDALONE_CLI") {
        return PathBuf::from(p);
    }
    PathBuf::from("/usr/local/bin/cynic")
}

fn is_user_local_target(target: &Path) -> bool {
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if target.starts_with(home.join(".local/bin")) {
            return true;
        }
    }
    if let Ok(xdg) = std::env::var("XDG_BIN_HOME")
        && target.starts_with(PathBuf::from(xdg))
    {
        return true;
    }
    if target.starts_with("/usr/local")
        || target.starts_with("/usr/bin")
        || target.starts_with("/usr/sbin")
        || target.starts_with("/bin")
        || target.starts_with("/sbin")
    {
        return false;
    }

    true
}

fn discover_source_binary() -> Option<PathBuf> {
    if let Ok(source) = std::env::var("CYNIC_STANDALONE_CLI_SOURCE") {
        let source = PathBuf::from(source);
        if source.exists() {
            return Some(source);
        }
    }

    if let Ok(source) = std::env::var("CYNIC_CLI_ARTIFACT") {
        let source = PathBuf::from(source);
        if source.exists() {
            return Some(source);
        }
    }

    std::env::current_dir()
        .ok()
        .and_then(|cwd| discover_source_binary_from_path(&cwd))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|current| current.parent().map(Path::to_path_buf))
                .and_then(|parent| discover_source_binary_from_path(&parent))
        })
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|current| std::path::absolute(current).ok())
        })
}

fn discover_source_binary_from_path(start_dir: &Path) -> Option<PathBuf> {
    let mut cursor = Some(start_dir);
    while let Some(dir) = cursor {
        let debug = dir.join("target").join("debug").join("cynic");
        if debug.exists() {
            return Some(debug);
        }

        let release = dir.join("target").join("release").join("cynic");
        if release.exists() {
            return Some(release);
        }

        cursor = dir.parent();
    }
    None
}

fn same_existing_file(source: &Path, target: &Path) -> bool {
    if source == target {
        return true;
    }

    #[cfg(unix)]
    {
        if let (Ok(source_meta), Ok(target_meta)) = (source.metadata(), target.metadata())
            && source_meta.is_file()
            && target_meta.is_file()
            && source_meta.ino() == target_meta.ino()
            && source_meta.dev() == target_meta.dev()
        {
            return true;
        }
    }

    let Ok(source) = source.canonicalize() else {
        return false;
    };
    let Ok(target) = target.canonicalize() else {
        return false;
    };
    source == target
}

fn standalone_cli_install_proof() -> Vec<&'static str> {
    vec![
        "installs the Rust standalone cynic binary directly",
        "does not install a shell wrapper",
        "does not require CYNIC_PROJECT_ROOT",
        "does not depend on Python organs",
        "source binary can be injected explicitly by the build surface",
    ]
}

fn install_output_indicates_same_file(stdout: &str, stderr: &str) -> bool {
    let stdout = stdout.to_lowercase();
    let stderr = stderr.to_lowercase();
    let markers = [
        "same file",
        "are the same file",
        "the same file",
        "identifient le même fichier",
        "identifient le meme fichier",
        "identifie le même fichier",
        "identifie le meme fichier",
        "already installed",
    ];

    markers
        .iter()
        .any(|marker| stdout.contains(marker) || stderr.contains(marker))
}

#[cfg(test)]
<<<<<<< HEAD
mod tests;
=======
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        discover_source_binary, discover_source_binary_from_path,
        install_output_indicates_same_file, install_standalone_cli, is_user_local_target,
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

        // SAFETY: this test executes in single-threaded context and restores the
        // prior environment value to avoid cross-test contamination.
        unsafe { std::env::set_var("CYNIC_STANDALONE_CLI_SOURCE", &expected) };
        let discovered = discover_source_binary().expect("source should be discovered");
        assert_eq!(discovered, expected);

        if let Ok(value) = previous {
            // SAFETY: same reason as above; this test is local and restores env state.
            unsafe { std::env::set_var("CYNIC_STANDALONE_CLI_SOURCE", value) };
        } else {
            // SAFETY: same reason as above; this test is single-threaded and scoped.
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
            // SAFETY: this test is single-threaded and restores env.
            unsafe { std::env::remove_var("CYNIC_STANDALONE_CLI") };
        }

        assert_eq!(
            super::default_cli_install_target(),
            PathBuf::from("/usr/local/bin/cynic")
        );

        if let Some(value) = previous {
            // SAFETY: this test is single-threaded and restores env.
            unsafe { std::env::set_var("CYNIC_STANDALONE_CLI", value) };
        }
    }
}
>>>>>>> github/main
