use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

use super::types::StandaloneCliInstallReport;
use cynic_protocol::build::{
    STANDALONE_CLI_INSTALL_REPORT_SCHEMA, is_user_local_target, standalone_cli_install_argv,
};
pub use cynic_protocol::build::is_user_local_target;
use cynic_types::paths::CynicPaths;

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
    let source_display = source.display().to_string();
    let command = standalone_cli_install_argv(&source_display, &target_display, user_local);

    if !apply {
        return Ok(StandaloneCliInstallReport {
            schema: STANDALONE_CLI_INSTALL_REPORT_SCHEMA.to_string(),
            status: "planned".to_string(),
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
            schema: STANDALONE_CLI_INSTALL_REPORT_SCHEMA.to_string(),
            status: "already_installed".to_string(),
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
            schema: STANDALONE_CLI_INSTALL_REPORT_SCHEMA.to_string(),
            status: "already_installed".to_string(),
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
        "installed".to_string()
    } else {
        "blocked".to_string()
    };

    Ok(StandaloneCliInstallReport {
        schema: STANDALONE_CLI_INSTALL_REPORT_SCHEMA.to_string(),
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
    CynicPaths::binary_dir().join("cynic")
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

fn standalone_cli_install_proof() -> Vec<String> {
    vec![
        "installs the Rust standalone cynic binary directly".to_string(),
        "does not install a shell wrapper".to_string(),
        "does not require CYNIC_PROJECT_ROOT".to_string(),
        "does not depend on Python organs".to_string(),
        "source binary can be injected explicitly by the build surface".to_string(),
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
mod tests;
