<<<<<<< HEAD
use cynic_vcs_forgejo::ForgejoTokenApiPort;
use cynic_vcs_forgejo::LocalForgejoTokenApiAdapter;
=======
use cynic_protocol::{
    ForgejoDesiredState, ForgejoObservedState, ForgejoPlan, NETWORK_PATH_EVIDENCE_SCHEMA,
    NetworkPathEvidence, SecurityEvidenceState as ForgejoEvidenceState, plan_forgejo_convergence,
};
>>>>>>> github/main

use crate::repo_access::{
    RepoAccessStatusReport, current_project_root, current_repo_access_status,
    load_forgejo_desired_state,
};

<<<<<<< HEAD
use cynic_vcs::forgejo_token::{
    ForgejoTokenAuthState, ForgejoTokenObservation, ForgejoTokenSpec, ForgejoTokenStatusReport,
    FORGEJO_TOKEN_STATUS_SCHEMA, plan_forgejo_token_convergence,
};
use cynic_vcs::{
    ForgejoDesiredState, ForgejoPlan, build_forgejo_plan, forgejo_token_spec,
};

use cynic_network::transport_security::{
    NETWORK_PATH_EVIDENCE_SCHEMA, NetworkPathEvidence,
};

use cynic_protocol::SecurityEvidenceState as ForgejoEvidenceState;
=======
pub trait ForgejoObservationPort: Send + Sync {
    fn observe(&self, desired: &ForgejoDesiredState) -> ForgejoObservedState;
}

pub fn build_forgejo_plan(
    desired: ForgejoDesiredState,
    observer: &dyn ForgejoObservationPort,
) -> ForgejoPlan {
    let observed = observer.observe(&desired);
    plan_forgejo_convergence(desired, &observed)
}
>>>>>>> github/main

#[derive(Debug)]
pub struct RuntimeForgejoObserver {
    repo_access: RepoAccessStatusReport,
<<<<<<< HEAD
    token_port: Option<Box<dyn ForgejoTokenApiPort>>,
    token_spec: Option<ForgejoTokenSpec>,
=======
>>>>>>> github/main
}

impl RuntimeForgejoObserver {
    pub fn new(repo_access: RepoAccessStatusReport) -> Self {
<<<<<<< HEAD
        Self {
            repo_access,
            token_port: None,
            token_spec: None,
        }
    }

    pub fn with_token_port(
        repo_access: RepoAccessStatusReport,
        token_port: Box<dyn ForgejoTokenApiPort>,
        token_spec: ForgejoTokenSpec,
    ) -> Self {
        Self {
            repo_access,
            token_port: Some(token_port),
            token_spec: Some(token_spec),
        }
    }

    pub fn token_observation(&self) -> Option<ForgejoTokenObservation> {
        let port = self.token_port.as_ref()?;
        let spec = self.token_spec.as_ref()?;
        Some(port.observe(spec))
    }
}

impl cynic_vcs::ForgejoObservationPort for RuntimeForgejoObserver {
    fn observe(&self, desired: &ForgejoDesiredState) -> cynic_vcs::ForgejoObservedState {
=======
        Self { repo_access }
    }
}

impl ForgejoObservationPort for RuntimeForgejoObserver {
    fn observe(&self, desired: &ForgejoDesiredState) -> ForgejoObservedState {
>>>>>>> github/main
        let remote = self
            .repo_access
            .sovereign_remotes
            .iter()
            .find(|remote| remote.id == desired.instance_id);
        let name_resolution = match remote.map(|remote| remote.api_resolution_status.as_str()) {
            Some("resolved") => ForgejoEvidenceState::Proven,
            Some("unresolved") => ForgejoEvidenceState::Absent,
            _ => ForgejoEvidenceState::Unknown,
        };
        let transport_reachability =
            match remote.map(|remote| remote.api_tcp_reachability_status.as_str()) {
                Some("open") => ForgejoEvidenceState::Proven,
                Some("closed_or_filtered" | "unresolved") => ForgejoEvidenceState::Absent,
                _ => ForgejoEvidenceState::Unknown,
            };
        let mut evidence_refs = Vec::new();
        if let Some(remote) = remote {
            evidence_refs.push(format!(
                "repo-access:{}:{}",
                remote.id, remote.api_tcp_reachability_status
            ));
        }
<<<<<<< HEAD
        let authenticated_state = match self.token_observation() {
            Some(observation) => {
                if observation.api_authenticated == ForgejoTokenAuthState::Proven {
                    evidence_refs.extend(observation.evidence_refs);
                }
                match observation.api_authenticated {
                    ForgejoTokenAuthState::Proven => ForgejoEvidenceState::Proven,
                    ForgejoTokenAuthState::Refuted => ForgejoEvidenceState::Refuted,
                    ForgejoTokenAuthState::Absent => ForgejoEvidenceState::Absent,
                    ForgejoTokenAuthState::Unknown => ForgejoEvidenceState::Unknown,
                }
            }
            None => ForgejoEvidenceState::Unknown,
        };
        let ssh_state = if transport_reachability == ForgejoEvidenceState::Proven {
            ForgejoEvidenceState::Proven
        } else {
            ForgejoEvidenceState::Unknown
        };

        cynic_vcs::ForgejoObservedState {
=======
        ForgejoObservedState {
>>>>>>> github/main
            admin_path: NetworkPathEvidence {
                schema: NETWORK_PATH_EVIDENCE_SCHEMA.to_string(),
                id: format!("{}-admin", desired.instance_id),
                service_ref: desired.instance_id.clone(),
                name_resolution,
                network_route: if transport_reachability == ForgejoEvidenceState::Proven {
                    ForgejoEvidenceState::Proven
                } else {
                    ForgejoEvidenceState::Unknown
                },
                transport_reachability,
<<<<<<< HEAD
                confidentiality: authenticated_state,
                peer_identity: ssh_state,
                application_identity: authenticated_state,
=======
                confidentiality: ForgejoEvidenceState::Unknown,
                peer_identity: ForgejoEvidenceState::Unknown,
                application_identity: ForgejoEvidenceState::Unknown,
>>>>>>> github/main
                resolved_addresses: remote
                    .map(|remote| remote.api_resolved_addresses.clone())
                    .unwrap_or_default(),
                evidence_refs: evidence_refs.clone(),
            },
<<<<<<< HEAD
            owner_active: authenticated_state,
            repository_exists: authenticated_state,
            machine_principal_ready: authenticated_state,
            machine_key_enrolled: authenticated_state,
            repository_permission_granted: authenticated_state,
            ssh_host_identity: ssh_state,
            git_ssh_authenticated: authenticated_state,
            bootstrap_authority_revoked: authenticated_state,
=======
            owner_active: ForgejoEvidenceState::Unknown,
            repository_exists: ForgejoEvidenceState::Unknown,
            machine_key_enrolled: ForgejoEvidenceState::Unknown,
            ssh_host_identity: ForgejoEvidenceState::Unknown,
            git_ssh_authenticated: ForgejoEvidenceState::Unknown,
>>>>>>> github/main
            evidence_refs,
        }
    }
}

pub fn current_forgejo_plan() -> Result<ForgejoPlan, String> {
    let root = current_project_root().ok_or_else(|| "CYNIC project root not found".to_string())?;
    let desired = load_forgejo_desired_state(&root)?;
<<<<<<< HEAD
    let token_spec = forgejo_token_spec(&desired, None);
    let observer = RuntimeForgejoObserver::with_token_port(
        current_repo_access_status(),
        Box::new(LocalForgejoTokenApiAdapter),
        token_spec,
    );
    Ok(build_forgejo_plan(desired, &observer))
}

pub fn current_forgejo_token_status() -> Result<ForgejoTokenStatusReport, String> {
    let root = current_project_root().ok_or_else(|| "CYNIC project root not found".to_string())?;
    let desired = load_forgejo_desired_state(&root)?;
    let spec = forgejo_token_spec(&desired, None);
    let port: Box<dyn ForgejoTokenApiPort> = Box::new(LocalForgejoTokenApiAdapter);
    let observation = port.observe(&spec);
    let plan = plan_forgejo_token_convergence(spec.clone(), &observation);
    let token_action_needed = matches!(
        observation.api_authenticated,
        ForgejoTokenAuthState::Absent | ForgejoTokenAuthState::Refuted
    );
    Ok(ForgejoTokenStatusReport {
        schema: FORGEJO_TOKEN_STATUS_SCHEMA.to_string(),
        principal: spec.principal.clone(),
        token: observation,
        forgejo_plan_status: plan.status.clone(),
        token_action_needed,
        requires_human_admin_secret: false,
        plan_action: if plan.actions.is_empty() {
            "observe".to_string()
        } else {
            format!("{:?}", plan.actions[0].kind)
        },
        evidence_refs: plan
            .evidence_refs
            .iter()
            .filter(|r| r.starts_with("secret:"))
            .cloned()
            .collect(),
    })
=======
    let observer = RuntimeForgejoObserver::new(current_repo_access_status());
    Ok(build_forgejo_plan(desired, &observer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::SecurityEvidenceState as ForgejoEvidenceState;

    struct FixedObserver(ForgejoObservedState);

    impl ForgejoObservationPort for FixedObserver {
        fn observe(&self, _desired: &ForgejoDesiredState) -> ForgejoObservedState {
            self.0.clone()
        }
    }

    #[test]
    fn planning_depends_on_observation_port_not_local_processes() {
        let desired = ForgejoDesiredState {
            instance_id: "forgejo".into(),
            owner: "owner".into(),
            repository: "repo".into(),
            api_url: "http://forgejo:3000/api/v1".into(),
            git_ssh_url: "ssh://git@forgejo:22/owner/repo.git".into(),
            machine_key_fingerprint: "SHA256:key".into(),
        };
        let observer = FixedObserver(ForgejoObservedState {
            admin_path: NetworkPathEvidence {
                schema: NETWORK_PATH_EVIDENCE_SCHEMA.to_string(),
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
            repository_exists: ForgejoEvidenceState::Proven,
            machine_key_enrolled: ForgejoEvidenceState::Proven,
            ssh_host_identity: ForgejoEvidenceState::Proven,
            git_ssh_authenticated: ForgejoEvidenceState::Proven,
            evidence_refs: vec!["proof.json".into()],
        });

        let plan = build_forgejo_plan(desired, &observer);
        assert_eq!(plan.status, "ready");
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn runtime_observer_does_not_invent_authenticated_state_from_tcp() {
        let observer = RuntimeForgejoObserver::new(RepoAccessStatusReport {
            schema: "test",
            mutates: false,
            profile_path: "profile.json".into(),
            profile_exists: true,
            canonical_project_root: None,
            known_repos: Vec::new(),
            sovereign_remotes: Vec::new(),
            entries: Vec::new(),
        });
        let observed = observer.observe(&ForgejoDesiredState {
            instance_id: "forgejo".into(),
            owner: "owner".into(),
            repository: "repo".into(),
            api_url: "http://forgejo:3000/api/v1".into(),
            git_ssh_url: "ssh://git@forgejo:22/owner/repo.git".into(),
            machine_key_fingerprint: "SHA256:key".into(),
        });
        assert_eq!(
            observed.admin_path.transport_reachability,
            ForgejoEvidenceState::Unknown
        );
        assert_eq!(observed.repository_exists, ForgejoEvidenceState::Unknown);
        assert_eq!(
            observed.git_ssh_authenticated,
            ForgejoEvidenceState::Unknown
        );
    }
>>>>>>> github/main
}
