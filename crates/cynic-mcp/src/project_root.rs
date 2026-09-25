use std::env;
use std::path::{Path, PathBuf};

const MCP_PROJECT_ROOT_ENV: &str = "CYNIC_MCP_PROJECT_ROOT";
const ALLOW_LOCAL_DEV_BUILD_ENV: &str = "CYNIC_ALLOW_LOCAL_DEV_BUILD";

pub(crate) fn resolve_mcp_project_root() -> PathBuf {
    if let Some(explicit) = project_root_from_env() {
        return explicit;
    }

    let fallback = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if allow_local_dev_build() {
        if let Some(found) = discover_project_root(&fallback) {
            return found;
        }

        if let Some(found) = env::current_exe().ok().and_then(|exe| {
            exe.parent()
                .map(Path::to_path_buf)
                .and_then(|start| discover_project_root(&start))
        }) {
            return found;
        }
    }

    fallback
}

fn project_root_from_env() -> Option<PathBuf> {
    let value = env::var_os(MCP_PROJECT_ROOT_ENV)?;
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() {
        return None;
    }

    Some(path)
}

fn allow_local_dev_build() -> bool {
    env::var(ALLOW_LOCAL_DEV_BUILD_ENV)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

fn discover_project_root(start: &Path) -> Option<PathBuf> {
    let mut cursor = Some(start);
    while let Some(candidate) = cursor {
        if candidate.join("Cargo.toml").is_file() && candidate.join("crates").is_dir() {
            return Some(candidate.to_path_buf());
        }
        cursor = candidate.parent();
    }

    None
}
