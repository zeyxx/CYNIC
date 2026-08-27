use super::assemble;
use serde_json::Value;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path};

const O_NOFOLLOW: i32 = 0o400000;
const O_DIRECTORY: i32 = 0o200000;
const O_CLOEXEC: i32 = 0o2000000;
const O_EXCL: i32 = 0o200;
const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
const S_IFMT: libc::mode_t = 0o170000;
const S_IFREG: libc::mode_t = 0o100000;
const S_IFLNK: libc::mode_t = 0o120000;

pub(crate) fn read_json(path: &Path) -> Result<Value, String> {
    let file = open_regular(path, "read")?;
    read_json_file(&file, path)
}

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

fn open_regular(path: &Path, operation: &str) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(O_NOFOLLOW | libc::O_NONBLOCK | O_CLOEXEC);
    let file = options
        .open(path)
        .map_err(|e| format!("cannot {operation} {}: {e}", path.display()))?;
    if !file
        .metadata()
        .map_err(|e| format!("cannot stat {}: {e}", path.display()))?
        .is_file()
    {
        return Err(format!("{} is not a regular file", path.display()));
    }
    Ok(file)
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
    let actual = read_json(output)?;
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
    let temporary = create_temporary_output(&parent, target_name, output)?;

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
        parent.sync_all().map_err(|e| {
            format!(
                "cannot fsync output directory {}: {e}",
                parent_path.display()
            )
        })
    })();

    if result.is_err() {
        let _ = unlink_at(&parent, &temporary.name);
    }
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TargetIdentity {
    dev: libc::dev_t,
    ino: libc::ino_t,
}

fn inspect_target(
    parent: &File,
    name: &std::ffi::OsStr,
    path: &Path,
) -> Result<Option<TargetIdentity>, String> {
    let name = c_name(name, path)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: name is NUL-free and stat points to writable storage.
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        return Err(format!("cannot inspect output {}: {error}", path.display()));
    }
    // SAFETY: fstatat initialized stat on success.
    let stat = unsafe { stat.assume_init() };
    if stat.st_mode & S_IFMT == S_IFLNK {
        return Err(format!("output target is a symlink: {}", path.display()));
    }
    if stat.st_mode & S_IFMT != S_IFREG {
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
    // SAFETY: child is NUL-free; every component is opened relative to the
    // previous descriptor with O_NOFOLLOW|O_DIRECTORY.
    let fd = unsafe {
        libc::openat(
            base.as_raw_fd(),
            c.as_ptr(),
            libc::O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(format!(
            "cannot open safe output parent {}: {}",
            full.display(),
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: fd is uniquely owned after successful open.
    Ok(unsafe { File::from_raw_fd(fd) })
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
        // SAFETY: candidate is NUL-free; parent is an anchored directory FD.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                candidate.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                0o600,
            )
        };
        if fd >= 0 {
            // SAFETY: fd is uniquely owned after successful openat.
            return Ok(TemporaryOutput {
                name: candidate,
                file: unsafe { File::from_raw_fd(fd) },
            });
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            continue;
        }
        return Err(format!(
            "cannot create temporary output beside {}: {error}",
            output.display()
        ));
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
    // SAFETY: both names are NUL-free and are resolved only in the anchored FD.
    let result = unsafe {
        libc::renameat(
            parent.as_raw_fd(),
            temporary.as_ptr(),
            parent.as_raw_fd(),
            target.as_ptr(),
        )
    };
    if result != 0 {
        return Err(format!(
            "cannot atomically replace {}: {}",
            output.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

fn unlink_at(parent: &File, name: &CString) -> Result<(), std::io::Error> {
    // SAFETY: name is NUL-free and resolved only in the anchored FD.
    let result = unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
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
}
