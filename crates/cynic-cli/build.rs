use std::{
    collections::BTreeSet,
    env,
    ffi::{CString, OsStr},
    fs::{self, File},
    io::{Read, Write},
    os::{fd::AsFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
    process::exit,
};

use rustix::fs::{self as rfs, AtFlags, FileType, Mode, OFlags};
use serde_json::Value;

const SURFACE_REGISTRY_RELATIVE_MANIFEST: &str = "../../infra/surface-registry/manifest.json";
const GENERATED_FILE: &str = "surface_registry_fragment_index.rs";
const CYNIC_SURFACE_REGISTRY_SCHEMA: &str = "cynic.surface_registry.v1";

fn fail(message: &str) -> ! {
    eprintln!("surface registry fragment index generation failed: {message}");
    exit(1);
}

fn c_name(name: &OsStr, path: &Path) -> CString {
    CString::new(name.as_bytes())
        .unwrap_or_else(|_| fail(&format!("unsafe path: {}", path.display())))
}

fn open_directory(path: &Path) -> File {
    let base = File::open(if path.is_absolute() { "/" } else { "." })
        .unwrap_or_else(|e| fail(&format!("cannot anchor directory {}: {e}", path.display())));
    let mut current = base;
    for component in path.components() {
        let std::path::Component::Normal(name) = component else {
            if matches!(component, std::path::Component::ParentDir) {
                fail(&format!(
                    "parent component is not allowed: {}",
                    path.display()
                ));
            }
            continue;
        };
        let name = c_name(name, path);
        // Name is NUL-free; resolution is relative to an FD and refuses symlinks.
        let owned = rfs::openat(
            current.as_fd(),
            &name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .unwrap_or_else(|error| {
            fail(&format!(
                "cannot open directory {}: {error}",
                path.display()
            ))
        });
        current = File::from(owned);
    }
    let metadata = current
        .metadata()
        .unwrap_or_else(|e| fail(&format!("cannot stat directory {}: {e}", path.display())));
    if !metadata.is_dir() {
        fail(&format!("not a directory: {}", path.display()));
    }
    current
}

fn open_regular_at(parent: &File, name: &OsStr, path: &Path) -> File {
    let name = c_name(name, path);
    // Name is NUL-free and parent is an anchored directory descriptor.
    let owned = rfs::openat(
        parent.as_fd(),
        &name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .unwrap_or_else(|error| fail(&format!("cannot read {}: {error}", path.display())));
    let file = File::from(owned);
    let metadata = file
        .metadata()
        .unwrap_or_else(|e| fail(&format!("cannot stat {}: {e}", path.display())));
    if !metadata.is_file() {
        fail(&format!("{} is not a regular file", path.display()));
    }
    file
}

fn same_identity(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino() && a.file_type() == b.file_type()
}

fn read_regular(path: &Path) -> String {
    let parent_path = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = open_directory(parent_path);
    let name = path
        .file_name()
        .unwrap_or_else(|| fail(&format!("path has no file name: {}", path.display())));
    let path_identity = fs::symlink_metadata(path)
        .unwrap_or_else(|e| fail(&format!("cannot inspect {}: {e}", path.display())));
    if path_identity.file_type().is_symlink() || !path_identity.is_file() {
        fail(&format!(
            "{} is not a regular non-symlink file",
            path.display()
        ));
    }
    let file = open_regular_at(&parent, name, path);
    let file_identity = file
        .metadata()
        .unwrap_or_else(|e| fail(&format!("cannot stat {}: {e}", path.display())));
    if !same_identity(&path_identity, &file_identity) {
        fail(&format!("{} changed while opening", path.display()));
    }
    let mut text = String::new();
    let mut reader = file;
    reader
        .read_to_string(&mut text)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));
    let after = reader
        .metadata()
        .unwrap_or_else(|e| fail(&format!("cannot stat {}: {e}", path.display())));
    if !same_identity(&file_identity, &after) {
        fail(&format!("{} changed while reading", path.display()));
    }
    text
}

fn manifest_path(cargo_manifest_dir: &Path) -> PathBuf {
    let mut registry_root = cargo_manifest_dir.to_path_buf();
    registry_root.pop();
    registry_root.pop();
    registry_root.push("infra");
    registry_root.push("surface-registry");
    let path = registry_root.join("manifest.json");
    let metadata = fs::symlink_metadata(&path)
        .unwrap_or_else(|e| fail(&format!("manifest not found at {}: {e}", path.display())));
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        fail(&format!(
            "manifest is not a regular file: {}",
            path.display()
        ));
    }
    path
}

fn parse_manifest(path: &Path) -> Value {
    let payload = read_regular(path);
    serde_json::from_str(&payload).unwrap_or_else(|error| {
        fail(&format!(
            "invalid JSON manifest {}: {error}",
            path.display()
        ))
    })
}

fn normalize_path(path: &str) -> &str {
    if path.ends_with(".json")
        && !path.contains("../")
        && !path.starts_with('/')
        && !path.contains('\\')
    {
        path
    } else {
        fail(&format!("invalid fragment path: {path}"));
    }
}

fn write_index(manifest: &Value) {
    let schema = manifest
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if schema != CYNIC_SURFACE_REGISTRY_SCHEMA {
        fail(&format!(
            "manifest schema mismatch: expected {CYNIC_SURFACE_REGISTRY_SCHEMA}, got {schema}"
        ));
    }
    let surfaces = manifest
        .get("surfaces")
        .and_then(Value::as_array)
        .unwrap_or_else(|| fail("manifest.surfaces must be a JSON array"));
    if surfaces.is_empty() {
        fail("manifest.surfaces must not be empty");
    }
    let mut fragments_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fragments_root.pop();
    fragments_root.pop();
    fragments_root.push("infra");
    fragments_root.push("surface-registry");
    let fragments_parent = open_directory(&fragments_root);
    let mut seen_ids = BTreeSet::new();
    let mut out = format!(
        "pub(crate) const SURFACE_REGISTRY_FRAGMENT_INDEX: [(&str, &str); {}] = [\n",
        surfaces.len()
    );
    for surface in surfaces {
        let id = surface
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| fail("manifest surface is missing id"));
        if !seen_ids.insert(id.to_string()) {
            fail(&format!("duplicate surface id in manifest: {id}"));
        }
        let path = normalize_path(
            surface
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_else(|| fail(&format!("manifest surface {id} is missing path"))),
        );
        let fragment_path = fragments_root.join(path);
        println!("cargo:rerun-if-changed=../../infra/surface-registry/{path}");
        let fragment_payload =
            read_regular_relative(&fragments_parent, Path::new(path), &fragment_path);
        let fragment: Value = serde_json::from_str(&fragment_payload)
            .unwrap_or_else(|e| fail(&format!("invalid JSON in fragment {path}: {e}")));
        let fragment_id = fragment
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| fail(&format!("fragment {path} is missing an id")));
        if fragment_id != id {
            fail(&format!(
                "fragment {path} id mismatch: manifest expects {id}, fragment has {fragment_id}"
            ));
        }
        out.push_str(&format!("    ({id:?}, {fragment_payload:?}),\n\n"));
    }
    out.push_str("];\n");
    let out_path =
        PathBuf::from(env::var("OUT_DIR").unwrap_or_else(|_| fail("OUT_DIR is not set")))
            .join(GENERATED_FILE);
    atomic_write(&out_path, out.as_bytes());
}

fn atomic_write(path: &Path, payload: &[u8]) {
    let parent_path = path
        .parent()
        .unwrap_or_else(|| fail(&format!("output has no parent: {}", path.display())));
    let parent = open_directory(parent_path);
    let target = path
        .file_name()
        .unwrap_or_else(|| fail(&format!("output has no file name: {}", path.display())));
    let target_c = c_name(target, path);
    let mut temp_name = None;
    let mut temp = None;
    for _attempt in 0..100u32 {
        let candidate = CString::new(format!(
            ".{}.cynic-tmp-{}-{_attempt}",
            target.to_string_lossy(),
            std::process::id()
        ))
        .unwrap();
        // Candidate is NUL-free and parent is anchored; O_EXCL prevents races.
        match rfs::openat(
            parent.as_fd(),
            &candidate,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_bits_truncate(0o600),
        ) {
            Ok(owned) => {
                temp_name = Some(candidate);
                temp = Some(File::from(owned));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                fail(&format!(
                    "cannot create generated output beside {}: {error}",
                    path.display()
                ));
            }
        }
    }
    let name = temp_name.unwrap_or_else(|| fail("cannot allocate temporary generated output"));
    let mut file = temp.unwrap();
    let mut renamed = false;
    let result = (|| {
        file.write_all(payload).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        // Refuse replacing symlinks, directories, devices, or other special files.
        // statat with SYMLINK_NOFOLLOW observes the directory entry itself.
        match rfs::statat(parent.as_fd(), &target_c, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) => {
                if !matches!(FileType::from_raw_mode(stat.st_mode), FileType::RegularFile) {
                    return Err("generated output target is not a regular file".to_string());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        // Both names are NUL-free and anchored to parent.
        rfs::renameat(parent.as_fd(), &name, parent.as_fd(), &target_c)
            .map_err(|error| error.to_string())?;
        renamed = true;
        parent.sync_all().map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        if cleanup_temporary(true, renamed) {
            let _ = rfs::unlinkat(parent.as_fd(), &name, AtFlags::empty());
        }
        fail(&format!("unable to write {}: {error}", path.display()));
    }
}

fn cleanup_temporary(operation_failed: bool, renamed: bool) -> bool {
    operation_failed && !renamed
}

fn read_regular_relative(parent: &File, relative: &Path, path: &Path) -> String {
    let components: Vec<_> = relative.components().collect();
    let mut current = parent
        .try_clone()
        .unwrap_or_else(|e| fail(&format!("cannot clone input anchor: {e}")));
    for (index, component) in components.iter().enumerate() {
        let std::path::Component::Normal(name) = component else {
            fail(&format!("invalid relative input path: {}", path.display()));
        };
        if index + 1 == components.len() {
            return read_regular_at(&current, name, path);
        }
        let name = c_name(name, path);
        // Directory component is resolved relative to an anchored FD.
        let owned = rfs::openat(
            current.as_fd(),
            &name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .unwrap_or_else(|error| {
            fail(&format!(
                "cannot open input directory {}: {error}",
                path.display()
            ))
        });
        current = File::from(owned);
    }
    fail(&format!("input path has no file name: {}", path.display()))
}

fn read_regular_at(parent: &File, name: &OsStr, path: &Path) -> String {
    let file = open_regular_at(parent, name, path);
    let identity = file
        .metadata()
        .unwrap_or_else(|e| fail(&format!("cannot stat {}: {e}", path.display())));
    let mut reader = file;
    let mut text = String::new();
    reader
        .read_to_string(&mut text)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));
    let after = reader
        .metadata()
        .unwrap_or_else(|e| fail(&format!("cannot stat {}: {e}", path.display())));
    if !same_identity(&identity, &after) {
        fail(&format!("{} changed while reading", path.display()));
    }
    text
}

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty() && s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()));
    // fail-closed: a binary with no verifiable source identity must not claim
    // a proven build. The proof surface (operator, deploy, admission) reads
    // CYNIC_GIT_SHA and rejects "unknown".
    let git_sha = match git_sha {
        Some(sha) => sha,
        None => {
            let is_release = env::var("PROFILE").as_deref() == Ok("release");
            if is_release {
                eprintln!(
                    "surface registry version surface: release binary built outside a \
                     verifiable git work tree — refusing to bake 'unknown' identity \
                     (CYNIC_GIT_SHA cannot resolve to a real commit). Build from a \
                     checked-out repository with .git present."
                );
                exit(1);
            }
            "unknown".to_string()
        }
    };
    let profile = env::var("PROFILE").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=CYNIC_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=CYNIC_BUILD_PROFILE={profile}");
    let build_proof = env::var("CYNIC_BUILD_PROOF_SHA").unwrap_or_default();
    println!("cargo:rustc-env=CYNIC_BUILD_PROOF_SHA={build_proof}");

    let cargo_manifest_dir =
        env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| fail("CARGO_MANIFEST_DIR is not set"));
    let manifest_path = manifest_path(Path::new(&cargo_manifest_dir));
    println!("cargo:rerun-if-changed={SURFACE_REGISTRY_RELATIVE_MANIFEST}");
    write_index(&parse_manifest(&manifest_path));
}

#[cfg(test)]
mod tests {
    use super::{cleanup_temporary, open_directory};
    use std::fs;

    #[test]
    fn absolute_directory_paths_are_opened_component_by_component() {
        let root = std::env::temp_dir().join(format!(
            "cynic-build-path-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nested = root.join("one/two");
        fs::create_dir_all(&nested).unwrap();
        assert!(open_directory(&nested).metadata().unwrap().is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_rename_failure_keeps_ownership_with_the_renamed_target() {
        assert!(!cleanup_temporary(true, true));
        assert!(cleanup_temporary(true, false));
    }
}
