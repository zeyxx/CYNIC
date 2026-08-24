//! Registry scanning — discover and load registry sources, no validation

use std::path::{Path, PathBuf};

use crate::registry::types::{RegistryDocument, RegistryNode};
use crate::runtime_registry::resolve_runtime_registry_path;

/// Registry loading error
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("failed to resolve registry path: {0}")]
    PathResolve(String),
    #[error("failed to read registry file {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("failed to parse registry {path}: {source}")]
    Parse { path: PathBuf, source: serde_json::Error },
}

/// Resolve registry path with fallback logic
pub fn resolve_registry_path(explicit_path: Option<&str>) -> Result<PathBuf, ScanError> {
    resolve_runtime_registry_path(explicit_path.map(Path::new))
        .map_err(|e| ScanError::PathResolve(e))
}

/// Load registry document from path
pub fn load_registry(path: &Path) -> Result<RegistryDocument, ScanError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|source| ScanError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::from_str(&raw)
        .map_err(|source| ScanError::Parse {
            path: path.to_path_buf(),
            source,
        })
}

/// Find node by name/alias in registry
pub fn find_node<'a>(
    registry: &'a RegistryDocument,
    name: &str,
) -> Option<&'a RegistryNode> {
    registry.nodes.iter().find(|node| node_matches(node, name))
}

/// Check if node matches requested name (name, id, or alias)
fn node_matches(node: &RegistryNode, requested: &str) -> bool {
    registry_node_name(node).as_deref() == Some(requested)
        || node.aliases.iter().any(|alias| alias == requested)
}

/// Get display name for node (name or id)
fn registry_node_name(node: &RegistryNode) -> Option<String> {
    node.name
        .clone()
        .or_else(|| node.id.clone())
        .filter(|value| !value.is_empty())
}

/// Scan for all registry files in standard locations
pub fn discover_registry_sources() -> Vec<PathBuf> {
    let mut sources = Vec::new();

    // Default registry path
    if let Ok(default) = crate::runtime_registry::resolve_runtime_registry_path(None) {
        if default.exists() {
            sources.push(default);
        }
    }

    // Check common locations
    let candidates = [
        "/etc/cynic/registry.json",
        "/var/lib/cynic/registry.json",
        "./registry.json",
        "./infra/registry.json",
    ];

    for candidate in candidates {
        let path = Path::new(candidate);
        if path.exists() && !sources.iter().any(|p| p == path) {
            sources.push(path.to_path_buf());
        }
    }

    sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn loads_valid_registry() {
        let content = r#"{"nodes":[{"name":"test-node","role":"kernel_primary"}]}"#;
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), content).unwrap();

        let doc = load_registry(file.path()).unwrap();
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.nodes[0].name.as_deref(), Some("test-node"));
    }

    #[test]
    fn returns_error_for_missing_file() {
        let path = Path::new("/nonexistent/registry.json");
        let err = load_registry(path).unwrap_err();
        assert!(matches!(err, ScanError::Read { .. }));
    }

    #[test]
    fn returns_error_for_invalid_json() {
        let content = r#"{invalid json}"#;
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), content).unwrap();

        let err = load_registry(file.path()).unwrap_err();
        assert!(matches!(err, ScanError::Parse { .. }));
    }

    #[test]
    fn finds_node_by_name() {
        let doc = RegistryDocument {
            network_policy: None,
            lifecycle: None,
            nodes: vec![RegistryNode {
                name: Some("test-node".to_string()),
                id: None,
                aliases: vec!["alias1".to_string()],
                role: Some("kernel_primary".to_string()),
                canonical_role: None,
                provider_controls: vec![],
                transports: None,
                host_family: None,
                substrate: None,
                services: vec![],
                network_links: vec![],
            }],
            fleet_desired: None,
        };

        assert!(find_node(&doc, "test-node").is_some());
        assert!(find_node(&doc, "alias1").is_some());
        assert!(find_node(&doc, "nonexistent").is_none());
    }
}