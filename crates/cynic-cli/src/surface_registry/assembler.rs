mod io;

use io::read_json_file;
pub(crate) use io::{check, write};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::io::AsRawFd;
use std::path::{Component, Path, PathBuf};

const SCHEMA: &str = cynic_protocol::SURFACE_REGISTRY_SCHEMA;
const O_NOFOLLOW: i32 = 0o400000;
const O_NONBLOCK: i32 = 0o4000;
const O_DIRECTORY: i32 = 0o200000;
const O_CLOEXEC: i32 = 0o2000000;

pub(crate) fn assemble(manifest_path: &Path) -> Result<Value, String> {
    let registry_root = manifest_path
        .parent()
        .ok_or_else(|| "manifest has no parent directory".to_string())?;
    let root_file = open_verified_directory(registry_root, "repository root")?;
    let manifest_name = manifest_path
        .file_name()
        .ok_or_else(|| "manifest has no file name".to_string())?;
    let manifest_file = open_relative(&root_file, Path::new(manifest_name), "manifest")?;
    if !manifest_file
        .metadata()
        .map_err(|e| format!("manifest descriptor: {e}"))?
        .is_file()
    {
        return Err(format!(
            "manifest is not a regular file: {}",
            manifest_path.display()
        ));
    }
    let manifest_identity =
        fs::symlink_metadata(manifest_path).map_err(|e| format!("manifest identity: {e}"))?;
    if !same_identity(
        &manifest_identity,
        &manifest_file.metadata().map_err(|e| e.to_string())?,
    ) {
        return Err("manifest path/descriptor inode/device identity mismatch".into());
    }
    let manifest = read_json_file(&manifest_file, manifest_path)?;
    let manifest = manifest
        .as_object()
        .ok_or_else(|| format!("manifest must be an object: {}", manifest_path.display()))?;
    if manifest.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("manifest schema mismatch: expected {SCHEMA}"));
    }
    let entries = manifest
        .get("surfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| "manifest.surfaces must be an array".to_string())?;
    if entries.is_empty() {
        return Err("manifest.surfaces is empty".to_string());
    }
    let retired: &[Value] = manifest
        .get("retired")
        .map(|v| {
            v.as_array()
                .ok_or_else(|| "manifest.retired must be an array".to_string())
        })
        .transpose()?
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let fragments_root = registry_root.join("fragments");
    verify_directory(&fragments_root, "fragments directory")?;
    let fragments_file = open_relative(&root_file, Path::new("fragments"), "fragments directory")?;
    verify_identity(&fragments_file, &fragments_root, "fragments directory")?;

    assemble_with_descriptors(
        entries,
        retired,
        registry_root,
        &fragments_root,
        &root_file,
        &fragments_file,
    )
}

fn assemble_with_descriptors(
    entries: &[Value],
    retired: &[Value],
    registry_root: &Path,
    fragments_root: &Path,
    root_file: &File,
    fragments_file: &File,
) -> Result<Value, String> {
    verify_identity(root_file, registry_root, "repository root")?;
    verify_identity(fragments_file, fragments_root, "fragments directory")?;
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut surfaces = Vec::with_capacity(entries.len());

    for entry in entries {
        let object = entry
            .as_object()
            .ok_or_else(|| "manifest.surfaces entries must be objects".to_string())?;
        let id = required_string(object, "id", "manifest surface")?;
        let fragment = required_path(object, id, fragments_root, root_file)?;
        let path = fragment.relative.clone();
        if !ids.insert(id.to_string()) {
            return Err(format!("duplicate surface id in manifest: {id}"));
        }
        if !paths.insert(path.clone()) {
            return Err(format!(
                "duplicate fragment path in manifest: {}",
                path.display()
            ));
        }
        let fragment_value = read_fragment(fragment, id)?;
        if !fragment_value.is_object() {
            return Err(format!("fragment {path:?} must be an object"));
        }
        if fragment_value.get("id").and_then(Value::as_str) != Some(id) {
            return Err(format!("fragment id mismatch for {id}"));
        }
        surfaces.push(fragment_value);
    }

    for entry in retired {
        let object = entry
            .as_object()
            .ok_or_else(|| "manifest.retired entries must be objects".to_string())?;
        let id = required_string(object, "id", "retired entry")?;
        let reason = required_string(object, "reason", &format!("retired {id}"))?;
        if reason.is_empty() {
            return Err(format!("retired {id}: missing reason"));
        }
        let fragment = required_path(object, id, fragments_root, root_file)?;
        let path = fragment.relative.clone();
        if !ids.insert(id.to_string()) {
            if entries.iter().any(|entry| {
                entry
                    .as_object()
                    .and_then(|object| object.get("id"))
                    .and_then(Value::as_str)
                    == Some(id)
            }) {
                return Err(format!("retired surface is also active: {id}"));
            }
            return Err(format!("duplicate retired surface id: {id}"));
        }
        if !paths.insert(path.clone()) {
            return Err(format!(
                "duplicate fragment path in manifest: {}",
                path.display()
            ));
        }
        let fragment_value = read_fragment(fragment, id)?;
        if !fragment_value.is_object() {
            return Err(format!("retired fragment {path:?} must be an object"));
        }
        if fragment_value.get("id").and_then(Value::as_str) != Some(id) {
            return Err(format!("retired fragment id mismatch for {id}"));
        }
    }

    let mut discovered = BTreeSet::new();
    scan_fragments(fragments_file, Path::new("fragments"), &mut discovered)?;
    let orphans: Vec<_> = discovered.difference(&paths).collect();
    if !orphans.is_empty() {
        return Err(format!(
            "unclassified fragment(s): {}",
            orphans
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let mut document = Map::new();
    document.insert("schema".into(), Value::String(SCHEMA.into()));
    document.insert("surfaces".into(), Value::Array(surfaces));
    Ok(Value::Object(document))
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("{context}: missing {key}"))
}

fn required_path(
    object: &Map<String, Value>,
    id: &str,
    fragments_root: &Path,
    root_file: &File,
) -> Result<FragmentDescriptor, String> {
    let value = required_string(object, "path", &format!("surface {id}"))?;
    let relative = Path::new(value);
    if relative.is_absolute()
        || value.contains('\\')
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!(
            "surface {id}: path must be safe and relative: {value:?}"
        ));
    }
    let candidate = fragments_root.parent().unwrap().join(relative);
    let resolved_root = fs::canonicalize(fragments_root)
        .map_err(|e| format!("cannot resolve fragments directory: {e}"))?;
    let resolved =
        fs::canonicalize(&candidate).map_err(|e| format!("missing or unsafe path {value}: {e}"))?;
    if !resolved.starts_with(&resolved_root) || resolved == resolved_root {
        return Err(format!(
            "surface {id}: path must be beneath fragments directory"
        ));
    }
    reject_symlinks(&candidate, fragments_root)?;
    let path_identity =
        fs::symlink_metadata(&candidate).map_err(|e| format!("fragment {id} identity: {e}"))?;
    // Open and validate through the verified root descriptor. The returned
    // descriptor is the authority for reading; never reopen through candidate.
    let file = open_relative(root_file, relative, id)?;
    let identity = file
        .metadata()
        .map_err(|e| format!("fragment {id} descriptor: {e}"))?;
    if !identity.is_file() {
        return Err(format!("fragment {id} is not a regular file"));
    }
    if !same_identity(&path_identity, &identity) {
        return Err(format!(
            "fragment {id} path/descriptor inode/device identity mismatch"
        ));
    }
    Ok(FragmentDescriptor {
        relative: relative.to_path_buf(),
        file,
        identity: FileIdentity::from_metadata(&identity),
    })
}

fn reject_symlinks(path: &Path, root: &Path) -> Result<(), String> {
    let mut current = root.to_path_buf();
    for component in path.strip_prefix(root).unwrap().components() {
        if let Component::Normal(name) = component {
            current.push(name);
            if fs::symlink_metadata(&current)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err(format!(
                    "symlink components/files are not allowed: {}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

struct FragmentDescriptor {
    relative: PathBuf,
    file: File,
    identity: FileIdentity,
}

#[derive(Clone, Copy)]
struct FileIdentity {
    dev: u64,
    ino: u64,
    file_type: u32,
}

impl FileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            file_type: metadata.mode() & libc::S_IFMT,
        }
    }
}

fn read_fragment(fragment: FragmentDescriptor, id: &str) -> Result<Value, String> {
    // Check identity immediately before reading, then parse bytes from that
    // same stable descriptor rather than reopening the manifest path.
    let mut file = fragment.file;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    let observed = FileIdentity::from_metadata(&metadata);
    if observed.dev != fragment.identity.dev
        || observed.ino != fragment.identity.ino
        || observed.file_type != fragment.identity.file_type
    {
        return Err(format!(
            "fragment {id} descriptor inode/device identity mismatch"
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "fragment {} is not a regular file",
            fragment.relative.display()
        ));
    }
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|e| format!("cannot read fragment {}: {e}", fragment.relative.display()))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("invalid JSON in {}: {e}", fragment.relative.display()))
}

fn open_relative(root: &File, relative: &Path, id: &str) -> Result<File, String> {
    let mut current = PathBuf::from(format!("/proc/self/fd/{}", root.as_raw_fd()));
    let mut opened_dirs: Vec<File> = Vec::new();
    let components: Vec<_> = relative.components().collect();
    if components.is_empty() {
        return Err(format!("fragment {id}: empty path"));
    }
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(format!("fragment {id}: unsafe relative path"));
        };
        current.push(name);
        let final_component = index + 1 == components.len();
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC | if final_component { 0 } else { 0o200000 },
        );
        let next = options
            .open(&current)
            .map_err(|e| format!("cannot open fragment {id} ({}): {e}", relative.display()))?;
        if !final_component {
            // The O_DIRECTORY open above is the confinement check for this
            // component; retain only its proc-fd spelling for the next step.
            current = PathBuf::from(format!("/proc/self/fd/{}", next.as_raw_fd()));
            opened_dirs.push(next);
        } else {
            drop(opened_dirs);
            return Ok(next);
        }
    }
    unreachable!()
}

fn scan_fragments(
    dir: &File,
    relative: &Path,
    found: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let proc_path = PathBuf::from(format!("/proc/self/fd/{}", dir.as_raw_fd()));
    let mut entries: Vec<_> = fs::read_dir(&proc_path)
        .map_err(|e| {
            format!(
                "cannot scan fragments directory {}: {e}",
                relative.display()
            )
        })?
        .collect::<Result<_, _>>()
        .map_err(|e| {
            format!(
                "cannot scan fragments directory {}: {e}",
                relative.display()
            )
        })?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| format!("non-UTF-8 entry under fragments: {}", relative.display()))?;
        if name == "." || name == ".." || name.contains('/') {
            return Err(format!(
                "unsafe entry under fragments: {}/{}",
                relative.display(),
                name
            ));
        }
        let entry_relative = relative.join(name);
        let entry_path = proc_path.join(name);
        let mut directory_options = OpenOptions::new();
        directory_options
            .read(true)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC | 0o200000);
        match directory_options.open(&entry_path) {
            Ok(child) => scan_fragments(&child, &entry_relative, found)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotADirectory => {
                let mut file_options = OpenOptions::new();
                file_options
                    .read(true)
                    .custom_flags(O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC);
                let file = file_options.open(&entry_path).map_err(|e| {
                    if e.raw_os_error() == Some(40) {
                        format!(
                            "symlink components/files are not allowed under fragments: {}",
                            entry_relative.display()
                        )
                    } else {
                        format!(
                            "unsafe entry under fragments {}: {e}",
                            entry_relative.display()
                        )
                    }
                })?;
                let metadata = file.metadata().map_err(|e| e.to_string())?;
                if !metadata.is_file() {
                    return Err(format!(
                        "special entry under fragments: {}",
                        entry_relative.display()
                    ));
                }
                found.insert(entry_relative);
            }
            Err(error) => {
                if error.raw_os_error() == Some(40) {
                    return Err(format!(
                        "symlink components/files are not allowed under fragments: {}",
                        entry_relative.display()
                    ));
                }
                return Err(format!(
                    "unsafe entry under fragments {}: {error}",
                    entry_relative.display()
                ));
            }
        }
    }
    Ok(())
}

fn verify_directory(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{label} is missing: {e}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!("{label} is missing or unsafe"));
    }
    Ok(())
}

fn open_verified_directory(path: &Path, label: &str) -> Result<File, String> {
    let before = fs::symlink_metadata(path).map_err(|e| format!("{label}: {e}"))?;
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK | O_DIRECTORY | O_CLOEXEC);
    let file = options.open(path).map_err(|e| format!("{label}: {e}"))?;
    if !file
        .metadata()
        .map_err(|e| format!("{label}: {e}"))?
        .is_dir()
    {
        return Err(format!("{label} is not a directory"));
    }
    if !same_identity(&before, &file.metadata().map_err(|e| e.to_string())?) {
        return Err(format!("{label} descriptor inode/device identity mismatch"));
    }
    Ok(file)
}

fn verify_identity(file: &File, path: &Path, label: &str) -> Result<(), String> {
    let expected = fs::symlink_metadata(path).map_err(|e| format!("{label}: {e}"))?;
    if !same_identity(&expected, &file.metadata().map_err(|e| e.to_string())?) {
        return Err(format!("{label} descriptor inode/device identity mismatch"));
    }
    Ok(())
}

fn same_identity(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.dev() == b.dev()
        && a.ino() == b.ino()
        && (a.mode() & libc::S_IFMT) == (b.mode() & libc::S_IFMT)
}
#[cfg(test)]
mod tests;
