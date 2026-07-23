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
                confidentiality: ForgejoEvidenceState::Unknown,
                peer_identity: ForgejoEvidenceState::Unknown,
                application_identity: ForgejoEvidenceState::Unknown,
                resolved_addresses: remote
                    .map(|remote| remote.api_resolved_addresses.clone())
                    .unwrap_or_default(),
                evidence_refs: evidence_refs.clone(),
            },
            owner_active: ForgejoEvidenceState::Unknown,
            repository_exists: ForgejoEvidenceState::Unknown,
            machine_key_enrolled: ForgejoEvidenceState::Unknown,
            ssh_host_identity: ForgejoEvidenceState::Unknown,
            git_ssh_authenticated: ForgejoEvidenceState::Unknown,
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
}
