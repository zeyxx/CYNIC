//! Côté hôte de la projection d'autorité : I/O + construction registry.
//! Le domaine pur (spec/observation/plan, `plan_agent_authority_projection`)
//! vit en L0 (`cynic_protocol::agent_access`).

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use cynic_protocol::agent_access::{
    AGENT_AUTHORITY_PROJECTION_SCHEMA, AgentAuthorityProjectionObservation,
    AgentAuthorityProjectionPlan, AgentAuthorityProjectionSpec,
};
use cynic_protocol::{AgentSessionAuthority, agent_session_authority_digest};

/// Construit le spec « installé » : registry + résolution d'identités
/// `/etc/passwd` + `/etc/group` (host-only, invariants 0o640/root).
pub fn installed_agent_authority_spec(
    source_path: impl Into<String>,
    authority: AgentSessionAuthority,
) -> AgentAuthorityProjectionSpec {
    let runtime = cynic_registry::runtime_authority::default_runtime_authority_paths();
    let owner = "root".to_string();
    let group = runtime.principal.runtime_user;
    AgentAuthorityProjectionSpec {
        schema: AGENT_AUTHORITY_PROJECTION_SCHEMA.to_string(),
        source_path: source_path.into(),
        target_path: runtime
            .config_root
            .join("agent-session-authority.json")
            .display()
            .to_string(),
        owner_id: crate::identity_database::resolve_identity_id("/etc/passwd", &owner),
        group_id: crate::identity_database::resolve_identity_id("/etc/group", &group),
        owner,
        group,
        mode: 0o640,
        authority,
    }
}

pub fn observe_agent_authority_projection(
    path: impl AsRef<Path>,
) -> AgentAuthorityProjectionObservation {
    let path = path.as_ref();
    let mut unknowns = Vec::new();
    let metadata = std::fs::symlink_metadata(path).ok();
    let authority = match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<AgentSessionAuthority>(&bytes) {
            Ok(authority) => Some(authority),
            Err(error) => {
                unknowns.push(format!("agent_authority_json_invalid:{error}"));
                None
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            unknowns.push(format!("agent_authority_unreadable:{error}"));
            None
        }
    };
    let computed_digest = authority
        .as_ref()
        .and_then(|authority| agent_session_authority_digest(authority).ok());
    let content_valid = authority
        .as_ref()
        .map(|authority| computed_digest.as_deref() == Some(authority.authority_digest.as_str()));
    AgentAuthorityProjectionObservation {
        schema: AGENT_AUTHORITY_PROJECTION_SCHEMA.to_string(),
        path: path.display().to_string(),
        present: metadata.is_some(),
        mode: metadata
            .as_ref()
            .map(|metadata| metadata.permissions().mode() & 0o777),
        owner_id: metadata.as_ref().map(MetadataExt::uid),
        group_id: metadata.as_ref().map(MetadataExt::gid),
        authority_revision: authority.as_ref().map(|authority| authority.revision),
        authority_digest: authority.map(|authority| authority.authority_digest),
        content_valid,
        evidence_refs: vec![format!("agent_authority_file:{}", path.display())],
        unknowns,
    }
}
