use std::path::{Path, PathBuf};

pub(crate) fn workspace_root() -> PathBuf {
    resolve_workspace_root(None)
        .unwrap_or_else(|_| PathBuf::from(".cynic-source-root-unobserved"))
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
}
