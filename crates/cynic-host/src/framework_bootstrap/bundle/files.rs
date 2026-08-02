use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(super) fn copy_standalone_cli_if_available(
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<bool, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to locate current executable: {error}"))?;
    if current_exe.file_name().and_then(|name| name.to_str()) != Some("cynic") {
        return Ok(false);
    }

    let target = output_dir.join("bin/cynic");
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create standalone cli dir: {error}"))?;
    }
    fs::copy(&current_exe, &target).map_err(|error| {
        format!(
            "failed to copy standalone cynic binary from {} to {}: {error}",
            current_exe.display(),
            target.display()
        )
    })?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("failed to chmod {}: {error}", target.display()))?;
    files.push("bin/cynic".to_string());
    Ok(true)
}

pub(super) fn copy_runtime_artifact(
    artifact_dir: &Path,
    binary: &str,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    let source = artifact_dir.join(binary);
    let target = output_dir.join("bin").join(binary);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create runtime artifact dir: {error}"))?;
    }
    fs::copy(&source, &target).map_err(|error| {
        format!(
            "failed to copy runtime artifact from {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("failed to chmod {}: {error}", target.display()))?;
    files.push(format!("bin/{binary}"));
    Ok(())
}

pub(super) fn copy_registry(
    source: &Path,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    let target = output_dir.join("etc/registry.json");
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create registry projection dir: {error}"))?;
    }
    fs::copy(source, &target).map_err(|error| {
        format!(
            "failed to copy runtime registry from {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    files.push("etc/registry.json".to_string());
    Ok(())
}

pub(super) fn write_file(
    path: PathBuf,
    content: String,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    let relative = path
        .strip_prefix(output_dir)
        .map_err(|error| format!("failed to relativize {}: {error}", path.display()))?;
    files.push(relative.display().to_string());
    Ok(())
}

pub(super) fn write_executable_file(
    path: PathBuf,
    content: String,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    write_file(path.clone(), content, output_dir, files)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("failed to chmod {}: {error}", path.display()))
}
