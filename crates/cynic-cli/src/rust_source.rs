use std::path::{Path, PathBuf};

pub(crate) fn workspace_root() -> PathBuf {
    resolve_workspace_root(None).unwrap_or_else(|_| PathBuf::from(".cynic-source-root-unobserved"))
}

pub(crate) fn resolve_workspace_root(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let (candidate, exact) = if let Some(path) = explicit {
        (path.to_path_buf(), true)
    } else if let Some(path) = std::env::var_os("CYNIC_SOURCE_ROOT") {
        (PathBuf::from(path), true)
    } else {
        (
            std::env::current_dir()
                .map_err(|error| format!("cannot observe current source directory: {error}"))?,
            false,
        )
    };

    let root = if exact {
        is_workspace_root(&candidate).then_some(candidate.clone())
    } else {
        candidate
            .ancestors()
            .find(|path| is_workspace_root(path))
            .map(Path::to_path_buf)
    }
    .ok_or_else(|| {
        format!(
            "source root {} does not contain Cargo.toml, crates/, contracts/foundation.json, and infra/registry.json",
            candidate.display()
        )
    })?;

    root.canonicalize().map_err(|error| {
        format!(
            "cannot canonicalize source root {}: {error}",
            root.display()
        )
    })
}

fn is_workspace_root(path: &Path) -> bool {
    path.join("Cargo.toml").is_file()
        && path.join("crates").is_dir()
        && path.join("contracts/foundation.json").is_file()
        && path.join("infra/registry.json").is_file()
}

/// Canonical workspace crate inventory, OBSERVED from `crates/*/Cargo.toml` —
/// never a hardcoded list. Adding or removing a crate directory updates every
/// audit scan automatically. `cynic-protocol` first (protocol_is_pure law:
/// wire contracts home), remainder sorted. Entries without a `Cargo.toml`
/// are not workspace members and are excluded.
pub(crate) fn workspace_crate_names() -> Vec<String> {
    let crates_root = workspace_root().join("crates");
    let Ok(entries) = std::fs::read_dir(&crates_root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            entry.path().join("Cargo.toml").is_file().then_some(name)
        })
        .collect();
    names.sort();
    if let Some(position) = names.iter().position(|name| name == "cynic-protocol") {
        let protocol = names.remove(position);
        names.insert(0, protocol);
    }
    names
}
pub(crate) fn relative_to_workspace(path: &Path) -> String {
    relative_path(&workspace_root(), path)
}

pub(crate) fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(crate) fn rust_files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

pub(crate) fn line_count(path: &Path) -> Option<usize> {
    std::fs::read_to_string(path)
        .ok()
        .map(|contents| contents.lines().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_explicit_non_workspace_root() {
        let path = std::env::temp_dir();
        let result = resolve_workspace_root(Some(&path));
        assert!(result.is_err());
    }

    #[test]
    fn resolves_the_current_workspace_without_compile_time_paths() {
        let root = resolve_workspace_root(None).expect("workspace root");
        assert!(root.join("contracts/foundation.json").is_file());
        assert!(root.join("infra/registry.json").is_file());
    }

    #[test]
    fn crate_inventory_is_observed_not_hardcoded() {
        let names = workspace_crate_names();
        assert!(names.contains(&"cynic-protocol".to_string()));
        assert!(names.contains(&"cynic-cli".to_string()));
        assert!(names.contains(&"cynic-mcp".to_string()));
        assert_eq!(names[0], "cynic-protocol");
        // Every entry is backed by a real crate directory — no phantoms.
        let root = resolve_workspace_root(None).expect("workspace root");
        for name in &names {
            assert!(
                root.join("crates").join(name).join("Cargo.toml").is_file(),
                "{name} has no crate directory"
            );
        }
        let mut sorted = names.clone();
        sorted.sort();
        if let Some(position) = sorted.iter().position(|name| name == "cynic-protocol") {
            let protocol = sorted.remove(position);
            sorted.insert(0, protocol);
        }
        assert_eq!(names, sorted);
    }
}
