use cynic_protocol::{
    ForgejoDesiredState, ForgejoObservedState, ForgejoPlan, NETWORK_PATH_EVIDENCE_SCHEMA,
    NetworkPathEvidence, SecurityEvidenceState as ForgejoEvidenceState, plan_forgejo_convergence,
};

use crate::repo_access::{
    RepoAccessStatusReport, current_project_root, current_repo_access_status,
    load_forgejo_desired_state,
};

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

#[derive(Debug)]
pub struct RuntimeForgejoObserver {
    repo_access: RepoAccessStatusReport,
}

impl RuntimeForgejoObserver {
    pub fn new(repo_access: RepoAccessStatusReport) -> Self {
        Self { repo_access }
    }
}

impl ForgejoObservationPort for RuntimeForgejoObserver {
    fn observe(&self, desired: &ForgejoDesiredState) -> ForgejoObservedState {
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
        let has_token = std::fs::read_to_string("/etc/cynic/secrets/forgejo-token")
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false)
            || std::env::var("FORGEJO_TOKEN")
                .map(|t| !t.trim().is_empty())
                .unwrap_or(false);

        let authenticated_state =
            if transport_reachability == ForgejoEvidenceState::Proven && has_token {
                evidence_refs.push("secret:forgejo-token".to_string());
                ForgejoEvidenceState::Proven
            } else {
                ForgejoEvidenceState::Unknown
            };
        let ssh_state = if transport_reachability == ForgejoEvidenceState::Proven {
            ForgejoEvidenceState::Proven
        } else {
            ForgejoEvidenceState::Unknown
        };

        ForgejoObservedState {
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
                confidentiality: authenticated_state,
                peer_identity: ssh_state,
                application_identity: authenticated_state,
                resolved_addresses: remote
                    .map(|remote| remote.api_resolved_addresses.clone())
                    .unwrap_or_default(),
                evidence_refs: evidence_refs.clone(),
            },
            owner_active: authenticated_state,
            repository_exists: authenticated_state,
            machine_principal_ready: authenticated_state,
            machine_key_enrolled: authenticated_state,
            repository_permission_granted: authenticated_state,
            ssh_host_identity: ssh_state,
            git_ssh_authenticated: authenticated_state,
            bootstrap_authority_revoked: authenticated_state,
            evidence_refs,
        }
    }
}

pub fn current_forgejo_plan() -> Result<ForgejoPlan, String> {
    let root = current_project_root().ok_or_else(|| "CYNIC project root not found".to_string())?;
    let desired = load_forgejo_desired_state(&root)?;
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
            machine_principal: "cynic-machine".into(),
            repository_permission: cynic_protocol::ForgejoRepositoryPermission::Write,
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
            machine_principal_ready: ForgejoEvidenceState::Proven,
            machine_key_enrolled: ForgejoEvidenceState::Proven,
            repository_permission_granted: ForgejoEvidenceState::Proven,
            ssh_host_identity: ForgejoEvidenceState::Proven,
            git_ssh_authenticated: ForgejoEvidenceState::Proven,
            bootstrap_authority_revoked: ForgejoEvidenceState::Proven,
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
            machine_principal: "cynic-machine".into(),
            repository_permission: cynic_protocol::ForgejoRepositoryPermission::Write,
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
}
