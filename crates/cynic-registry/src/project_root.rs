//! Project root discovery (host substrate, no VCS dependency).

use std::path::{Path, PathBuf};

pub fn current_project_root() -> Option<PathBuf> {
    current_project_root_from(std::env::current_dir().ok(), std::env::current_exe().ok())
}

fn current_project_root_from(current_dir: Option<PathBuf>, current_exe: Option<PathBuf>) -> Option<PathBuf> {
    find_project_root(current_dir.as_deref()).or_else(|| find_project_root(current_exe.as_deref()))
}

fn find_project_root(start: Option<&Path>) -> Option<PathBuf> {
    let start = start?;
    let initial = if start.is_dir() { start } else { start.parent()? };
    for candidate in initial.ancestors() {
        if candidate.join("Cargo.toml").is_file()
            && candidate.join("contracts/foundation.json").is_file()
            && candidate.join("infra").is_dir()
            && candidate.join("crates").is_dir()
        {
            return Some(candidate.to_path_buf());
        }
    }
    None
}
