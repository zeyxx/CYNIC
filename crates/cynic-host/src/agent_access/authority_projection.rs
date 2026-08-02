use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use cynic_protocol::{AgentSessionAuthority, agent_session_authority_digest};
use serde::Serialize;

use crate::ports::HostOperation;

pub const AGENT_AUTHORITY_PROJECTION_SCHEMA: &str = "cynic.host.agent_authority_projection.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAuthorityProjectionSpec {
    pub schema: String,
    pub source_path: String,
    pub target_path: String,
    pub owner: String,
    pub group: String,
    pub owner_id: Option<u32>,
    pub group_id: Option<u32>,
    pub mode: u32,
    pub authority: AgentSessionAuthority,
}

impl AgentAuthorityProjectionSpec {
    pub fn installed(source_path: impl Into<String>, authority: AgentSessionAuthority) -> Self {
        let runtime = crate::runtime_authority::default_runtime_authority_paths();
        let owner = "root".to_string();
        let group = runtime.principal.runtime_user;
        Self {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAuthorityProjectionObservation {
    pub schema: String,
    pub path: String,
    pub present: bool,
    pub mode: Option<u32>,
    pub owner_id: Option<u32>,
    pub group_id: Option<u32>,
    pub authority_revision: Option<u64>,
    pub authority_digest: Option<String>,
    pub content_valid: Option<bool>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAuthorityProjectionPlan {
    pub schema: String,
    pub status: String,
    pub desired_authority_digest: String,
    pub operations: Vec<HostOperation>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub expected_proofs: Vec<String>,
    pub rollback_intent: String,
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

pub fn plan_agent_authority_projection(
    desired: &AgentAuthorityProjectionSpec,
    source: &AgentAuthorityProjectionObservation,
    installed: &AgentAuthorityProjectionObservation,
) -> AgentAuthorityProjectionPlan {
    let mut blockers = Vec::new();
    let mut residual_unknowns = source
        .unknowns
        .iter()
        .chain(&installed.unknowns)
        .cloned()
        .collect::<Vec<_>>();
    let desired_digest = agent_session_authority_digest(&desired.authority).ok();
    if desired.schema != AGENT_AUTHORITY_PROJECTION_SCHEMA
        || desired.mode != 0o640
        || desired.owner != "root"
        || desired.owner_id.is_none()
        || desired.group_id.is_none()
        || desired.source_path.trim().is_empty()
        || desired.target_path.trim().is_empty()
        || desired_digest.as_deref() != Some(desired.authority.authority_digest.as_str())
    {
        blockers.push("agent_authority_projection_spec_invalid".to_string());
    }
    if !source.present
        || source.content_valid != Some(true)
        || source.authority_digest.as_deref() != desired_digest.as_deref()
    {
        blockers.push("agent_authority_projection_source_unproved".to_string());
    }
    let converged = installed.present
        && installed.content_valid == Some(true)
        && installed.authority_digest.as_deref() == desired_digest.as_deref()
        && installed.mode == Some(desired.mode)
        && installed.owner_id == desired.owner_id
        && installed.group_id == desired.group_id;
    let operations = if blockers.is_empty() && !converged {
        vec![HostOperation::InstallOwnedFile {
            source: desired.source_path.clone(),
            target: desired.target_path.clone(),
            user: desired.owner.clone(),
            group: desired.group.clone(),
            mode: desired.mode,
        }]
    } else {
        Vec::new()
    };
    if installed.present && installed.content_valid == Some(false) {
        residual_unknowns.push("installed_agent_authority_content_invalid".to_string());
    }
    blockers.sort();
    blockers.dedup();
    residual_unknowns.sort();
    residual_unknowns.dedup();
    AgentAuthorityProjectionPlan {
        schema: AGENT_AUTHORITY_PROJECTION_SCHEMA.to_string(),
        status: if !blockers.is_empty() {
            "blocked"
        } else if converged {
            "converged"
        } else {
            "planned"
        }
        .to_string(),
        desired_authority_digest: desired_digest.unwrap_or_default(),
        operations,
        blockers,
        residual_unknowns,
        expected_proofs: vec![
            "installed_authority_digest_matches".to_string(),
            "installed_authority_mode_0640".to_string(),
            "kernel_bootstrap_reads_installed_authority".to_string(),
        ],
        rollback_intent: "restore the previous content-addressed authority file".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use cynic_protocol::AgentCapabilityGrant;

    use super::*;
    use crate::agent_access::{AgentEnrollmentPolicy, compose_agent_session_authority};

    #[test]
    fn valid_source_and_absent_target_plan_one_bounded_install() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("authority.json");
        let target_path = directory.path().join("installed.json");
        let authority = compose_agent_session_authority(
            "node-a",
            1,
            2_000,
            &[AgentEnrollmentPolicy {
                observed_linux_uid: 1000,
                agent_types: vec!["codex".to_string()],
                max_ttl_ms: 900_000,
                capabilities: vec![AgentCapabilityGrant {
                    id: "coord.read".to_string(),
                    mutates: false,
                }],
            }],
        )
        .unwrap();
        std::fs::write(&source_path, serde_json::to_vec(&authority).unwrap()).unwrap();
        let mut spec =
            AgentAuthorityProjectionSpec::installed(source_path.display().to_string(), authority);
        spec.target_path = target_path.display().to_string();
        spec.owner_id = Some(0);
        spec.group_id = Some(0);

        let plan = plan_agent_authority_projection(
            &spec,
            &observe_agent_authority_projection(&source_path),
            &observe_agent_authority_projection(&target_path),
        );

        assert_eq!(plan.status, "planned");
        assert!(matches!(
            plan.operations.as_slice(),
            [HostOperation::InstallOwnedFile { source, target, mode, .. }]
                if source == &spec.source_path && target == &spec.target_path && *mode == 0o640
        ));
    }
}
