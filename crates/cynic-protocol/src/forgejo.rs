use crate::{NetworkPathEvidence, SecurityEvidenceState as ForgejoEvidenceState};
use serde::{Deserialize, Serialize};

pub const FORGEJO_PLAN_SCHEMA: &str = "cynic.forgejo.plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoDesiredState {
    pub instance_id: String,
    pub owner: String,
    pub repository: String,
    pub api_url: String,
    pub git_ssh_url: String,
    pub machine_principal: String,
    pub repository_permission: ForgejoRepositoryPermission,
    pub machine_key_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgejoRepositoryPermission {
    Read,
    Write,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoObservedState {
    pub admin_path: NetworkPathEvidence,
    pub owner_active: ForgejoEvidenceState,
    pub repository_exists: ForgejoEvidenceState,
    pub machine_principal_ready: ForgejoEvidenceState,
    pub machine_key_enrolled: ForgejoEvidenceState,
    pub repository_permission_granted: ForgejoEvidenceState,
    pub ssh_host_identity: ForgejoEvidenceState,
    pub git_ssh_authenticated: ForgejoEvidenceState,
    pub bootstrap_authority_revoked: ForgejoEvidenceState,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgejoActionKind {
    ObserveApiIdentity,
    ActivateOwner,
    CreateRepository,
    EnsureMachinePrincipal,
    EnrollMachineKey,
    GrantRepositoryPermission,
    VerifySshHostIdentity,
    VerifyGitSshAuthentication,
    RevokeBootstrapAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoPlannedAction {
    pub kind: ForgejoActionKind,
    pub mutates: bool,
    pub requires_human_admin_secret: bool,
    pub proof: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoPlan {
    pub schema: String,
    pub status: String,
    pub desired: ForgejoDesiredState,
    pub actions: Vec<ForgejoPlannedAction>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn plan_forgejo_convergence(
    desired: ForgejoDesiredState,
    observed: &ForgejoObservedState,
) -> ForgejoPlan {
    let mut actions = Vec::new();
    let mut blockers = Vec::new();

    if !observed.admin_path.admits_authenticated_application() {
        blockers.push("forgejo_authenticated_admin_path_unproven".to_string());
        actions.push(observe(
            ForgejoActionKind::ObserveApiIdentity,
            "forgejo_admin_network_path",
        ));
    }

    match observed.owner_active {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Refuted => blockers.push("forgejo_owner_state_refuted".to_string()),
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::ActivateOwner,
            true,
            "forgejo_owner_active",
        )),
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_owner_state_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_owner_state_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => blockers.push("forgejo_owner_state_unknown".to_string()),
    }

    match observed.repository_exists {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_repository_state_refuted".to_string())
        }
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::CreateRepository,
            true,
            "forgejo_repository_identity",
        )),
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_repository_state_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_repository_state_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_repository_state_unknown".to_string())
        }
    }

    match observed.machine_principal_ready {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::EnsureMachinePrincipal,
            true,
            "forgejo_machine_principal_restricted_non_admin",
        )),
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_machine_principal_state_refuted".to_string())
        }
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_machine_principal_state_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_machine_principal_state_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_machine_principal_state_unknown".to_string())
        }
    }

    match observed.machine_key_enrolled {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_machine_key_state_refuted".to_string())
        }
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::EnrollMachineKey,
            true,
            "forgejo_machine_key_fingerprint",
        )),
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_machine_key_state_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_machine_key_state_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_machine_key_state_unknown".to_string())
        }
    }

    match observed.repository_permission_granted {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::GrantRepositoryPermission,
            true,
            "forgejo_repository_permission",
        )),
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_repository_permission_refuted".to_string())
        }
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_repository_permission_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_repository_permission_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_repository_permission_unknown".to_string())
        }
    }

    if observed.ssh_host_identity != ForgejoEvidenceState::Proven {
        actions.push(observe(
            ForgejoActionKind::VerifySshHostIdentity,
            "forgejo_ssh_host_identity",
        ));
    }
    if observed.git_ssh_authenticated != ForgejoEvidenceState::Proven {
        actions.push(observe(
            ForgejoActionKind::VerifyGitSshAuthentication,
            "forgejo_git_ssh_authentication",
        ));
    }
    match observed.bootstrap_authority_revoked {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::RevokeBootstrapAuthority,
            true,
            "forgejo_bootstrap_authority_revoked",
        )),
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_bootstrap_authority_still_active".to_string())
        }
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_bootstrap_revocation_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_bootstrap_revocation_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_bootstrap_revocation_unknown".to_string())
        }
    }

    ForgejoPlan {
        schema: FORGEJO_PLAN_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "ready"
        } else {
            "blocked"
        }
        .to_string(),
        desired,
        actions,
        blockers,
        evidence_refs: observed.evidence_refs.clone(),
    }
}

fn observe(kind: ForgejoActionKind, proof: &str) -> ForgejoPlannedAction {
    ForgejoPlannedAction {
        kind,
        mutates: false,
        requires_human_admin_secret: false,
        proof: proof.to_string(),
    }
}

fn mutation(
    kind: ForgejoActionKind,
    requires_human_admin_secret: bool,
    proof: &str,
) -> ForgejoPlannedAction {
    ForgejoPlannedAction {
        kind,
        mutates: true,
        requires_human_admin_secret,
        proof: proof.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desired() -> ForgejoDesiredState {
        ForgejoDesiredState {
            instance_id: "forgejo".into(),
            owner: "cynic".into(),
            repository: "CYNIC".into(),
            api_url: "http://forgejo:3000/api/v1".into(),
            git_ssh_url: "ssh://git@forgejo:22/cynic/CYNIC.git".into(),
            machine_principal: "cynic-cortex".into(),
            repository_permission: ForgejoRepositoryPermission::Write,
            machine_key_fingerprint: "SHA256:machine".into(),
        }
    }

    #[test]
    fn absent_resources_become_separate_explicit_mutations() {
        let observed = ForgejoObservedState {
            admin_path: NetworkPathEvidence {
                schema: crate::NETWORK_PATH_EVIDENCE_SCHEMA.to_string(),
                id: "forgejo-admin".into(),
                service_ref: "forgejo".into(),
                name_resolution: ForgejoEvidenceState::Proven,
                network_route: ForgejoEvidenceState::Proven,
                transport_reachability: ForgejoEvidenceState::Proven,
                confidentiality: ForgejoEvidenceState::Proven,
                peer_identity: ForgejoEvidenceState::Proven,
                application_identity: ForgejoEvidenceState::Proven,
                resolved_addresses: Vec::new(),
                evidence_refs: vec!["admin-path.json".into()],
            },
            owner_active: ForgejoEvidenceState::Proven,
            repository_exists: ForgejoEvidenceState::Absent,
            machine_principal_ready: ForgejoEvidenceState::Absent,
            machine_key_enrolled: ForgejoEvidenceState::Absent,
            repository_permission_granted: ForgejoEvidenceState::Absent,
            ssh_host_identity: ForgejoEvidenceState::Proven,
            git_ssh_authenticated: ForgejoEvidenceState::Absent,
            bootstrap_authority_revoked: ForgejoEvidenceState::Absent,
            evidence_refs: vec!["probe.json".into()],
        };

        let plan = plan_forgejo_convergence(desired(), &observed);
        assert_eq!(plan.status, "ready");
        assert_eq!(plan.actions.len(), 6);
        assert_eq!(plan.actions[0].kind, ForgejoActionKind::CreateRepository);
        assert_eq!(
            plan.actions[1].kind,
            ForgejoActionKind::EnsureMachinePrincipal
        );
        assert_eq!(
            plan.actions[4].kind,
            ForgejoActionKind::VerifyGitSshAuthentication
        );
        assert!(plan.actions[0].requires_human_admin_secret);
        assert!(!plan.actions[2].mutates);
    }

    #[test]
    fn unknown_resource_state_blocks_mutation_planning() {
        let observed = ForgejoObservedState {
            admin_path: NetworkPathEvidence {
                schema: crate::NETWORK_PATH_EVIDENCE_SCHEMA.to_string(),
                id: "forgejo-admin".into(),
                service_ref: "forgejo".into(),
                name_resolution: ForgejoEvidenceState::Proven,
                network_route: ForgejoEvidenceState::Proven,
                transport_reachability: ForgejoEvidenceState::Proven,
                confidentiality: ForgejoEvidenceState::Absent,
                peer_identity: ForgejoEvidenceState::Unknown,
                application_identity: ForgejoEvidenceState::Unknown,
                resolved_addresses: Vec::new(),
                evidence_refs: Vec::new(),
            },
            owner_active: ForgejoEvidenceState::Unknown,
            repository_exists: ForgejoEvidenceState::Unknown,
            machine_principal_ready: ForgejoEvidenceState::Unknown,
            machine_key_enrolled: ForgejoEvidenceState::Unknown,
            repository_permission_granted: ForgejoEvidenceState::Unknown,
            ssh_host_identity: ForgejoEvidenceState::Unknown,
            git_ssh_authenticated: ForgejoEvidenceState::Unknown,
            bootstrap_authority_revoked: ForgejoEvidenceState::Unknown,
            evidence_refs: Vec::new(),
        };

        let plan = plan_forgejo_convergence(desired(), &observed);
        assert_eq!(plan.status, "blocked");
        assert_eq!(plan.blockers.len(), 7);
        assert!(plan.actions.iter().all(|action| !action.mutates));
    }
}
