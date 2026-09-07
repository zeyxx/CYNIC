use super::assemble;
use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use serde_json::Value;
use std::ffi::CString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::AsFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

pub(crate) fn read_json_file(file: &File, path: &Path) -> Result<Value, String> {
    let mut text = String::new();
    let mut reader = file
        .try_clone()
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    reader
        .read_to_string(&mut text)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid JSON in {}: {e}", path.display()))
}

fn read_json_anchored(
    parent: &File,
    name: &std::ffi::OsStr,
    path: &Path,
    expected: TargetIdentity,
) -> Result<Value, String> {
    let c_name = c_name(name, path)?;
    // c_name is NUL-free and resolution is relative to the verified
    // parent descriptor; these flags reject links and special blocking files.
    let owned = rustix::fs::openat(
        parent.as_fd(),
        &c_name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file = File::from(owned);
    let metadata = file
        .metadata()
        .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    let opened = TargetIdentity {
        dev: metadata.dev(),
        ino: metadata.ino(),
    };
    if opened != expected {
        return Err(format!(
            "output path/descriptor inode/device identity mismatch: {}",
            path.display()
        ));
    }
    let mut text = String::new();
    let mut reader = file;
    reader
        .read_to_string(&mut text)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let after = reader
        .metadata()
        .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
    if after.dev() != opened.dev || after.ino() != opened.ino || !after.is_file() {
        return Err(format!(
            "output descriptor identity changed while reading: {}",
            path.display()
        ));
    }
    serde_json::from_str(&text).map_err(|e| format!("invalid JSON in {}: {e}", path.display()))
}

pub(crate) fn canonical_json(value: &Value) -> String {
    fn sort(value: &Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), sort(v)))
                    .collect::<std::collections::BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            Value::Array(a) => Value::Array(a.iter().map(sort).collect()),
            other => other.clone(),
        }
    }
    serde_json::to_string_pretty(&sort(value)).expect("JSON serialization") + "\n"
}

pub(crate) fn check(manifest: &Path, output: &Path) -> Result<(), String> {
    let expected = assemble(manifest)?;
    let parent_path = output
        .parent()
        .ok_or_else(|| format!("output has no parent: {}", output.display()))?;
    let target_name = output
        .file_name()
        .ok_or_else(|| format!("output has no file name: {}", output.display()))?;
    let parent = open_verified_parent(parent_path)?;
    let identity = inspect_target(&parent, target_name, output)?
        .ok_or_else(|| format!("output does not exist: {}", output.display()))?;
    let actual = read_json_anchored(&parent, target_name, output, identity)?;
    if canonical_json(&expected) != canonical_json(&actual) {
        return Err("assembled output does not match manifest fragments".into());
    }
    Ok(())
}

pub(crate) fn write(manifest: &Path, output: &Path) -> Result<(), String> {
    let expected = assemble(manifest)?;
    let payload = canonical_json(&expected);
    let parent_path = output
        .parent()
        .ok_or_else(|| format!("output has no parent: {}", output.display()))?;
    let target_name = output
        .file_name()
        .ok_or_else(|| format!("output has no file name: {}", output.display()))?;
    let parent = open_verified_parent(parent_path)?;
    let initial_target = inspect_target(&parent, target_name, output)?;
    let mut temporary = create_temporary_output(&parent, target_name, output)?;

    let mut renamed = false;
    let result = (|| {
        temporary
            .file
            .write_all(payload.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
        temporary
            .file
            .sync_all()
            .map_err(|e| format!("cannot fsync {}: {e}", output.display()))?;

        // Re-check through the anchored directory immediately before rename.
        // A target appearing, disappearing, or changing inode/device is rejected.
        let current_target = inspect_target(&parent, target_name, output)?;
        if current_target != initial_target {
            return Err(format!(
                "output target identity changed during write: {}",
                output.display()
            ));
        }
        rename_at(&parent, &temporary.name, target_name, output)?;
        renamed = true;
        parent.sync_all().map_err(|e| {
            format!(
                "cannot fsync output directory {}: {e}",
                parent_path.display()
            )
        })
    })();

    if cleanup_temporary(result.is_err(), renamed) {
        let _ = unlink_at(&parent, &temporary.name);
    }
    result
}

fn cleanup_temporary(operation_failed: bool, renamed: bool) -> bool {
    operation_failed && !renamed
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TargetIdentity {
    dev: u64,
    ino: u64,
}

fn inspect_target(
    parent: &File,
    name: &std::ffi::OsStr,
    path: &Path,
) -> Result<Option<TargetIdentity>, String> {
    let name = c_name(name, path)?;
    // statat with SYMLINK_NOFOLLOW observes the directory entry itself.
    let stat = match rustix::fs::statat(parent.as_fd(), &name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => stat,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(format!("cannot inspect output {}: {error}", path.display()));
        }
    };
    if matches!(FileType::from_raw_mode(stat.st_mode), FileType::Symlink) {
        return Err(format!("output target is a symlink: {}", path.display()));
    }
    if !matches!(FileType::from_raw_mode(stat.st_mode), FileType::RegularFile) {
        return Err(format!(
            "output target is not a regular file: {}",
            path.display()
        ));
    }
    Ok(Some(TargetIdentity {
        dev: stat.st_dev,
        ino: stat.st_ino,
    }))
}

fn open_verified_parent(path: &Path) -> Result<File, String> {
    let expected = fs::symlink_metadata(path)
        .map_err(|e| format!("output parent is missing or unsafe {}: {e}", path.display()))?;
    if !expected.is_dir() || expected.file_type().is_symlink() {
        return Err(format!(
            "output parent is missing or unsafe: {}",
            path.display()
        ));
    }

    let mut current = File::open(if path.is_absolute() { "/" } else { "." }).map_err(|e| {
        format!(
            "cannot open output directory anchor {}: {e}",
            path.display()
        )
    })?;
    for component in path.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        current = open_directory(&current, name, path)?;
    }
    let actual = current
        .metadata()
        .map_err(|e| format!("cannot stat output parent {}: {e}", path.display()))?;
    if actual.dev() != expected.dev() || actual.ino() != expected.ino() {
        return Err(format!(
            "output parent inode/device identity changed: {}",
            path.display()
        ));
    }
    Ok(current)
}

fn open_directory(base: &File, child: &std::ffi::OsStr, full: &Path) -> Result<File, String> {
    let c = c_name(child, full)?;
    // Child is NUL-free; every component is opened relative to the
    // previous descriptor with NOFOLLOW|DIRECTORY.
    let owned = rustix::fs::openat(
        base.as_fd(),
        &c,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| format!("cannot open safe output parent {}: {e}", full.display()))?;
    Ok(File::from(owned))
}

fn c_name(name: &std::ffi::OsStr, path: &Path) -> Result<CString, String> {
    CString::new(name.as_bytes()).map_err(|_| format!("unsafe output name: {}", path.display()))
}

struct TemporaryOutput {
    name: CString,
    file: File,
}

fn create_temporary_output(
    parent: &File,
    target: &std::ffi::OsStr,
    output: &Path,
) -> Result<TemporaryOutput, String> {
    let name = target.to_string_lossy();
    let pid = std::process::id();
    for attempt in 0..100u32 {
        let candidate = CString::new(format!(".{name}.cynic-tmp-{pid}-{attempt}")).unwrap();
        // Candidate is NUL-free; parent is an anchored directory FD; O_EXCL
        // prevents races.
        match rustix::fs::openat(
            parent.as_fd(),
            &candidate,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_bits_truncate(0o600),
        ) {
            Ok(owned) => {
                return Ok(TemporaryOutput {
                    name: candidate,
                    file: File::from(owned),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "cannot create temporary output beside {}: {error}",
                    output.display()
                ));
            }
        }
    }
    Err(format!(
        "cannot allocate a unique temporary output beside {}",
        output.display()
    ))
}

fn rename_at(
    parent: &File,
    temporary: &CString,
    target: &std::ffi::OsStr,
    output: &Path,
) -> Result<(), String> {
    let target = c_name(target, output)?;
    // Both names are NUL-free and are resolved only in the anchored FD.
    rustix::fs::renameat(parent.as_fd(), temporary, parent.as_fd(), &target)
        .map_err(|e| format!("cannot atomically replace {}: {e}", output.display()))
}

fn unlink_at(parent: &File, name: &CString) -> Result<(), std::io::Error> {
    rustix::fs::unlinkat(parent.as_fd(), name, AtFlags::empty()).map_err(std::io::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_identity_changes_are_detected_relative_to_parent_fd() {
        let root = std::env::temp_dir().join(format!(
            "surface-registry-identity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let output = root.join("out.json");
        fs::write(&output, "first").unwrap();
        let parent = File::open(&root).unwrap();
        let initial = inspect_target(&parent, output.file_name().unwrap(), &output).unwrap();
        fs::rename(&output, root.join("old.json")).unwrap();
        fs::write(&output, "second").unwrap();
        let current = inspect_target(&parent, output.file_name().unwrap(), &output).unwrap();
        assert_ne!(initial, current);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn anchored_output_observation_rejects_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "surface-registry-symlink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let real = root.join("real.json");
        let link = root.join("out.json");
        fs::write(&real, "{}\n").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let parent = File::open(&root).unwrap();
        assert!(inspect_target(&parent, link.file_name().unwrap(), &link).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_rename_failure_does_not_cleanup_the_new_target() {
        assert!(!cleanup_temporary(true, true));
        assert!(cleanup_temporary(true, false));
        assert!(!cleanup_temporary(false, false));
    }
}
