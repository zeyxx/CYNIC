//! Read-only filesystem discovery and capacity observations for vault import.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct DestinationCapacity {
    pub(crate) same_filesystem: Option<bool>,
    pub(crate) required_destination_bytes: Option<u64>,
    pub(crate) destination_available_bytes: Option<u64>,
    pub(crate) status: &'static str,
}

/// User download directories are observed dynamically, never encoded as an
/// operator-specific absolute path. An explicit root wins when a desktop or
/// download client uses a non-standard location.
pub(crate) fn operator_download_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for key in ["CYNIC_MODEL_DOWNLOADS_DIR", "XDG_DOWNLOAD_DIR"] {
        if let Some(path) = std::env::var_os(key).filter(|value| !value.is_empty()) {
            roots.push(PathBuf::from(path));
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        if let Ok(raw) = fs::read_to_string(config.join("user-dirs.dirs")) {
            if let Some(value) = raw.lines().find_map(|line| {
                let value = line.trim().strip_prefix("XDG_DOWNLOAD_DIR=")?.trim();
                let value = value.trim_matches('"');
                value
                    .strip_prefix("$HOME/")
                    .map(|relative| home.join(relative))
                    .or_else(|| (!value.is_empty()).then(|| PathBuf::from(value)))
            }) {
                roots.push(value);
            }
        }
        roots.push(home.join("Downloads"));
    }
    roots
}

pub(crate) fn destination_probe_path(to: &Path) -> PathBuf {
    nearest_existing_parent(to)
}

pub(crate) fn destination_capacity(
    from: &Path,
    to: &Path,
    destination_available_bytes: Option<u64>,
) -> DestinationCapacity {
    let source_size = fs::metadata(from).ok().map(|metadata| metadata.len());
    let destination_root = destination_probe_path(to);
    let same_filesystem = match (fs::metadata(from), fs::metadata(&destination_root)) {
        #[cfg(unix)]
        (Ok(source), Ok(destination)) => {
            use std::os::unix::fs::MetadataExt;
            Some(source.dev() == destination.dev())
        }
        _ => None,
    };
    let required_destination_bytes = match (same_filesystem, source_size) {
        (Some(true), _) => Some(0),
        (Some(false), size) => size,
        (None, _) => None,
    };
    let status = match (required_destination_bytes, destination_available_bytes) {
        (Some(required), Some(available)) if available >= required => "ok",
        (Some(_), Some(_)) => "insufficient",
        _ => "unknown",
    };
    DestinationCapacity {
        same_filesystem,
        required_destination_bytes,
        destination_available_bytes,
        status,
    }
}

fn nearest_existing_parent(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    while !current.exists() {
        if !current.pop() {
            return PathBuf::from(".");
        }
    }
    if current.is_file() {
        current.pop();
    }
    current
}
