//! cynic-mcp: the dedicated in-tree MCP tool server.
//!
//! Every exposed tool is a real `#[tool]` implementation on [`server::CynicMcpServer`]
//! with a live executor — never a canned success. Tool authority stays in
//! `infra/agent-exposure.json`; this crate owns the implementations and the
//! stdio transport that serves them.

pub mod server;

use std::path::PathBuf;

/// Resolve the workspace root at runtime: explicit env override, else walk up
/// from the current directory. Never a compile-time baked path.
pub fn resolve_workspace_root() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CYNIC_SOURCE_ROOT") {
        let path = PathBuf::from(path);
        return is_workspace_root(&path).then_some(path).ok_or_else(|| {
            "CYNIC_SOURCE_ROOT does not contain Cargo.toml, crates/, and contracts/foundation.json"
                .to_string()
        });
    }
    let cwd = std::env::current_dir()
        .map_err(|error| format!("cannot observe current directory: {error}"))?;
    cwd.ancestors()
        .map(PathBuf::from)
        .find(|path| is_workspace_root(path))
        .ok_or_else(|| {
            "no workspace root above current directory (needs Cargo.toml, crates/, contracts/foundation.json)"
                .to_string()
        })
}

fn is_workspace_root(path: &std::path::Path) -> bool {
    path.join("Cargo.toml").is_file()
        && path.join("crates").is_dir()
        && path.join("contracts/foundation.json").is_file()
}

/// Observed workspace member inventory: every `crates/*` directory holding a
/// `Cargo.toml`. Protocol first, remainder sorted.
pub fn workspace_members() -> Vec<String> {
    let root = match resolve_workspace_root() {
        Ok(root) => root,
        Err(_) => return Vec::new(),
    };
    let mut names: Vec<String> = std::fs::read_dir(root.join("crates"))
        .into_iter()
        .flatten()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_members_start_with_protocol() {
        let names = workspace_members();
        assert!(!names.is_empty());
        assert_eq!(names[0], "cynic-protocol");
        assert!(names.contains(&"cynic-mcp".to_string()));
    }
}
