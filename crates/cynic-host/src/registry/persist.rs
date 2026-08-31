//! Registry persistence — I/O operations with atomic write and read-back proof

use std::fs;
use std::path::PathBuf;

use crate::registry::scan::{ScanError, load_registry, resolve_registry_path};
use crate::registry::types::RegistryDocument;
use crate::registry::validate::{ValidationError, validate_registry_or_error};

/// Persistence error
#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error("registry path resolution failed: {0}")]
    PathResolve(String),
    #[error("failed to read registry: {0}")]
    Read(#[from] ScanError),
    #[error("registry validation failed: {0:?}")]
    Validation(Vec<ValidationError>),
    #[error("failed to write registry: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to create parent directories: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("atomic write failed: {source}")]
    AtomicWrite {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("read-back verification failed: expected {expected:?}, got {actual:?}")]
    ReadBackMismatch {
        expected: RegistryDocument,
        actual: RegistryDocument,
    },
}

/// Write registry atomically with read-back proof
pub fn write_registry_atomic(
    doc: &RegistryDocument,
    explicit_path: Option<&str>,
) -> Result<WriteProof, PersistError> {
    let path = resolve_registry_path(explicit_path)?;

    // Validate before write
    validate_registry_or_error(doc).map_err(PersistError::Validation)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| PersistError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    // Serialize
    let json = serde_json::to_vec_pretty(doc).map_err(|e| PersistError::Write {
        path: path.clone(),
        source: std::io::Error::other(e.to_string()),
    })?;

    // Atomic write: write to temp file, then rename
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, &json).map_err(|source| PersistError::AtomicWrite {
        path: temp_path.clone(),
        source,
    })?;

    // Rename is atomic on POSIX
    fs::rename(&temp_path, &path).map_err(|source| PersistError::AtomicWrite {
        path: path.clone(),
        source,
    })?;

    // Read-back proof: load and compare
    let actual = load_registry(&path)?;
    if !registry_documents_equal(doc, &actual) {
        return Err(PersistError::ReadBackMismatch {
            expected: doc.clone(),
            actual,
        });
    }

    Ok(WriteProof {
        path: path.display().to_string(),
        bytes_written: json.len(),
        verified: true,
    })
}

/// Write registry without atomic guarantee (for tests/non-production)
pub fn write_registry(
    doc: &RegistryDocument,
    explicit_path: Option<&str>,
) -> Result<WriteProof, PersistError> {
    let path = resolve_registry_path(explicit_path)?;

    validate_registry_or_error(doc).map_err(PersistError::Validation)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| PersistError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let json = serde_json::to_vec_pretty(doc).map_err(|e| PersistError::Write {
        path: path.clone(),
        source: std::io::Error::other(e.to_string()),
    })?;

    fs::write(&path, &json).map_err(|source| PersistError::Write {
        path: path.clone(),
        source,
    })?;

    // Read-back proof
    let actual = load_registry(&path)?;
    if !registry_documents_equal(doc, &actual) {
        return Err(PersistError::ReadBackMismatch {
            expected: doc.clone(),
            actual,
        });
    }

    Ok(WriteProof {
        path: path.display().to_string(),
        bytes_written: json.len(),
        verified: true,
    })
}

/// Write proof — machine-readable evidence of successful write
#[derive(Debug, Clone, serde::Serialize)]
pub struct WriteProof {
    pub path: String,
    pub bytes_written: usize,
    pub verified: bool,
}

/// Compare two registry documents for equality (ignoring field order in maps)
fn registry_documents_equal(a: &RegistryDocument, b: &RegistryDocument) -> bool {
    // Compare nodes
    if a.nodes.len() != b.nodes.len() {
        return false;
    }
    for (node_a, node_b) in a.nodes.iter().zip(b.nodes.iter()) {
        if !registry_nodes_equal(node_a, node_b) {
            return false;
        }
    }

    // Compare fleet_desired
    match (&a.fleet_desired, &b.fleet_desired) {
        (Some(a_fleet), Some(b_fleet)) => {
            if a_fleet.targets.len() != b_fleet.targets.len() {
                return false;
            }
            for (ta, tb) in a_fleet.targets.iter().zip(b_fleet.targets.iter()) {
                if ta.id != tb.id
                    || ta.provider != tb.provider
                    || ta.resource_id != tb.resource_id
                    || ta.role != tb.role
                    || ta.desired_power != tb.desired_power
                {
                    return false;
                }
            }
        }
        (None, None) => {}
        _ => return false,
    }

    // Compare network_policy (simplified - compare planes keys and cidrs)
    match (&a.network_policy, &b.network_policy) {
        (Some(a_pol), Some(b_pol)) => {
            if a_pol.planes.len() != b_pol.planes.len() {
                return false;
            }
            for (key, plane_a) in &a_pol.planes {
                if let Some(plane_b) = b_pol.planes.get(key) {
                    if plane_a.cidr != plane_b.cidr {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        (None, None) => {}
        _ => return false,
    }

    // Compare lifecycle
    match (&a.lifecycle, &b.lifecycle) {
        (Some(a_life), Some(b_life)) => match (&a_life.kernel_release, &b_life.kernel_release) {
            (Some(a_rel), Some(b_rel)) => {
                if a_rel.build_runner_role != b_rel.build_runner_role {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        },
        (None, None) => {}
        _ => return false,
    }

    true
}

fn registry_nodes_equal(
    a: &crate::registry::types::RegistryNode,
    b: &crate::registry::types::RegistryNode,
) -> bool {
    a.id == b.id
        && a.name == b.name
        && a.aliases == b.aliases
        && a.role == b.role
        && a.canonical_role == b.canonical_role
        && a.provider_controls == b.provider_controls
        && a.host_family == b.host_family
        && a.substrate.as_ref().map(|s| &s.family) == b.substrate.as_ref().map(|s| &s.family)
        && registry_services_equal(&a.services, &b.services)
        && registry_network_links_equal(&a.network_links, &b.network_links)
}

fn registry_services_equal(
    a: &[crate::registry::types::RegistryNodeService],
    b: &[crate::registry::types::RegistryNodeService],
) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (sa, sb) in a.iter().zip(b.iter()) {
        if sa.name != sb.name
            || sa.artifact != sb.artifact
            || sa.port != sb.port
            || sa.unit != sb.unit
            || sa.supervisor.as_ref().map(|s| &s.provider)
                != sb.supervisor.as_ref().map(|s| &s.provider)
            || sa.health.as_ref().map(|h| (&h.kind, h.port, &h.process))
                != sb.health.as_ref().map(|h| (&h.kind, h.port, &h.process))
        {
            return false;
        }
    }
    true
}

fn registry_network_links_equal(
    a: &[crate::registry::types::RegistryNetworkLink],
    b: &[crate::registry::types::RegistryNetworkLink],
) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (la, lb) in a.iter().zip(b.iter()) {
        if la.plane != lb.plane
            || la.provider != lb.provider
            || la.interface != lb.interface
            || la.connection != lb.connection
            || la.address != lb.address
            || la.gateway != lb.gateway
            || la.routes != lb.routes
            || la.default_route != lb.default_route
        {
            return false;
        }
    }
    true
}

/// Load registry with validation
pub fn load_and_validate_registry(
    explicit_path: Option<&str>,
) -> Result<RegistryDocument, PersistError> {
    let path = resolve_registry_path(explicit_path)?;
    let doc = load_registry(&path)?;
    validate_registry_or_error(&doc).map_err(PersistError::Validation)?;
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::types::{RegistryDocument, RegistryNode};
    use tempfile::NamedTempFile;

    fn minimal_doc() -> RegistryDocument {
        RegistryDocument {
            network_policy: None,
            lifecycle: None,
            nodes: vec![RegistryNode {
                name: Some("test-node".to_string()),
                id: Some("test-node".to_string()),
                aliases: vec![],
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
        }
    }

    #[test]
    fn write_and_read_back() {
        let doc = minimal_doc();
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();

        let proof = write_registry(&doc, Some(path)).unwrap();
        assert!(proof.verified);
        assert_eq!(proof.path, path);

        // Load and verify
        let loaded = load_and_validate_registry(Some(path)).unwrap();
        assert_eq!(loaded.nodes.len(), 1);
        assert_eq!(loaded.nodes[0].name.as_deref(), Some("test-node"));
    }

    #[test]
    fn atomic_write_and_read_back() {
        let doc = minimal_doc();
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();

        let proof = write_registry_atomic(&doc, Some(path)).unwrap();
        assert!(proof.verified);

        let loaded = load_and_validate_registry(Some(path)).unwrap();
        assert_eq!(loaded.nodes.len(), 1);
    }

    #[test]
    fn rejects_invalid_registry() {
        let mut doc = minimal_doc();
        // Add node with duplicate ID
        doc.nodes.push(RegistryNode {
            name: Some("other".to_string()),
            id: Some("test-node".to_string()), // duplicate
            aliases: vec![],
            role: Some("kernel_primary".to_string()),
            canonical_role: None,
            provider_controls: vec![],
            transports: None,
            host_family: None,
            substrate: None,
            services: vec![],
            network_links: vec![],
        });

        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();

        let err = write_registry(&doc, Some(path)).unwrap_err();
        assert!(matches!(err, PersistError::Validation(_)));
    }

    #[test]
    fn write_proof_serializable() {
        let proof = WriteProof {
            path: "/test/registry.json".to_string(),
            bytes_written: 123,
            verified: true,
        };
        let json = serde_json::to_string(&proof).unwrap();
        assert!(json.contains("verified"));
        assert!(json.contains("123"));
    }
}
