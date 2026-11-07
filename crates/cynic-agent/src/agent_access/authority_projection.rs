//! Partie pure de la projection d'autorité (L2).
//!
//! Déplacée depuis `cynic-host`. Seuls le spec/observation/plan et
//! `plan_agent_authority_projection` vivent ici. `installed()` (registry +
//! résolution `/etc/passwd`) et `observe_agent_authority_projection` (I/O)
//! restent côté hôte.

use cynic_protocol::agent_access_port::AgentAccessOperation;
use cynic_protocol::{agent_session_authority_digest, AgentSessionAuthority};
use serde::Serialize;

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
    pub operations: Vec<AgentAccessOperation>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub expected_proofs: Vec<String>,
    pub rollback_intent: String,
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
        vec![AgentAccessOperation::InstallOwnedFile {
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
    use cynic_protocol::agent_session::{compose_agent_session_authority, AgentEnrollmentPolicy};

    fn observation(
        path: &str,
        present: bool,
        digest: Option<String>,
    ) -> AgentAuthorityProjectionObservation {
        AgentAuthorityProjectionObservation {
            schema: AGENT_AUTHORITY_PROJECTION_SCHEMA.to_string(),
            path: path.to_string(),
            present,
            mode: Some(0o640),
            owner_id: Some(0),
            group_id: Some(0),
            authority_revision: Some(1),
            authority_digest: digest,
            content_valid: Some(true),
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn valid_source_and_absent_target_plan_one_bounded_install() {
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
        let digest = agent_session_authority_digest(&authority).unwrap();
        let spec = AgentAuthorityProjectionSpec {
            schema: AGENT_AUTHORITY_PROJECTION_SCHEMA.to_string(),
            source_path: "/src/authority.json".to_string(),
            target_path: "/etc/cynic/agent-session-authority.json".to_string(),
            owner: "root".to_string(),
            group: "cynic".to_string(),
            owner_id: Some(0),
            group_id: Some(0),
            mode: 0o640,
            authority,
        };
        let mut absent = observation("/etc/cynic/agent-session-authority.json", false, None);
        absent.content_valid = None;

        let plan = plan_agent_authority_projection(
            &spec,
            &observation("/src/authority.json", true, Some(digest)),
            &absent,
        );

        assert_eq!(plan.status, "planned");
        assert!(matches!(
            plan.operations.as_slice(),
            [AgentAccessOperation::InstallOwnedFile { source, target, mode, .. }]
                if source == &spec.source_path && target == &spec.target_path && *mode == 0o640
        ));
    }
}
