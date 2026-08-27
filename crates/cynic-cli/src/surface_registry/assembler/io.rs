use super::assemble;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

const O_NOFOLLOW: i32 = 0o400000;
const O_NONBLOCK: i32 = 0o4000;
const O_CLOEXEC: i32 = 0o2000000;

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
        .custom_flags(O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC);
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
    let parent = output
        .parent()
        .ok_or_else(|| format!("output has no parent: {}", output.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;

    // Safe replacement policy: an existing target may only be replaced if it
    // is a regular file. Symlinks and every other file type are rejected.
    reject_unsafe_existing_target(output)?;
    let temporary = create_temporary_output(parent, output)?;
    let result = (|| {
        temporary
            .file
            .write_all(payload.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", temporary.path.display()))?;
        temporary
            .file
            .sync_all()
            .map_err(|e| format!("cannot fsync {}: {e}", temporary.path.display()))?;
        reject_unsafe_existing_target(output)?;
        fs::rename(&temporary.path, output)
            .map_err(|e| format!("cannot atomically replace {}: {e}", output.display()))?;
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| format!("cannot fsync output directory {}: {e}", parent.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary.path);
    }
    result
}

fn reject_unsafe_existing_target(output: &Path) -> Result<(), String> {
    match fs::symlink_metadata(output) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(format!("output target is a symlink: {}", output.display()))
        }
        Ok(metadata) if !metadata.is_file() => Err(format!(
            "output target is not a regular file: {}",
            output.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "cannot inspect output {}: {error}",
            output.display()
        )),
    }
}

struct TemporaryOutput {
    path: PathBuf,
    file: File,
}

fn create_temporary_output(parent: &Path, output: &Path) -> Result<TemporaryOutput, String> {
    let name = output
        .file_name()
        .ok_or_else(|| format!("output has no file name: {}", output.display()))?
        .to_string_lossy();
    let pid = std::process::id();
    for attempt in 0..100u32 {
        let path = parent.join(format!(".{name}.cynic-tmp-{pid}-{attempt}"));
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .custom_flags(O_NOFOLLOW | O_CLOEXEC);
        match options.open(&path) {
            Ok(file) => return Ok(TemporaryOutput { path, file }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "cannot create temporary output {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Err(format!(
        "cannot allocate a unique temporary output beside {}",
        output.display()
    ))
}

use std::io::Write;
