use crate::forgejo_token::{
    ForgejoTokenApiPort, ForgejoTokenAuthState, ForgejoTokenObservation, ForgejoTokenSpec,
    LocalForgejoTokenApiAdapter,
};
use cynic_vcs::{
    FORGEJO_TOKEN_SPEC_SCHEMA, ForgejoDesiredState, ForgejoObservedState, ForgejoPlan,
    plan_forgejo_convergence,
};
use cynic_network::transport_security::{
    NETWORK_PATH_EVIDENCE_SCHEMA, NetworkPathEvidence,
};
use cynic_protocol::SecurityEvidenceState as ForgejoEvidenceState;

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

/// Build a ForgejoTokenSpec from the desired state + a canonical token path.
///
/// The token path follows the Secret Integrity Gate convention:
/// `/etc/cynic/secrets/forgejo-token` owned by `root:cynic` mode 0640.
pub fn forgejo_token_spec(
    desired: &ForgejoDesiredState,
    token_path: Option<&str>,
) -> ForgejoTokenSpec {
    ForgejoTokenSpec {
        schema: FORGEJO_TOKEN_SPEC_SCHEMA.to_string(),
        principal: desired.machine_principal.clone(),
        api_url: desired
            .api_url
            .strip_suffix("/api/v1")
            .unwrap_or(&desired.api_url)
            .trim_end_matches('/')
            .to_string(),
        target_path: token_path
            .map(|p| p.to_string())
            .unwrap_or_else(|| "/etc/cynic/secrets/forgejo-token".to_string()),
        mode: 0o640,
        scopes: vec![
            "write:repository".to_string(),
            "write:organization".to_string(),
            "write:admin".to_string(),
        ],
        expiry_seconds: Some(7776000),
    }
}

#[derive(Debug)]
pub struct RuntimeForgejoObserver {
    repo_access: RepoAccessStatusReport,
    /// Injectible token probe. When `None`, the observer falls back to a
    /// file-presence check only (legacy behaviour) — it cannot prove API
    /// validity, so the plan stays blocked on `forgejo_token_api_validation`.
    token_port: Option<Box<dyn ForgejoTokenApiPort>>,
    /// The spec the injected port is bound to (parallels `repo_access`).
    token_spec: Option<ForgejoTokenSpec>,
}

impl RuntimeForgejoObserver {
    pub fn new(repo_access: RepoAccessStatusReport) -> Self {
        Self {
            repo_access,
            token_port: None,
            token_spec: None,
        }
    }

    /// Variant that observes the Forgejo API token through the injected port.
    ///
    /// With this variant the observer does not conflate "token file present"
    /// with "token API valid". A present-but-expired/rejected token surfaces
    /// as a distinct `Refuted`/probe-blocked signal instead of a false `Proven`.
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

    /// Read-back proof of the token observation (file metadata + API validity).
    /// Returns `None` when no token port was bound.
    pub fn token_observation(&self) -> Option<ForgejoTokenObservation> {
        let port = self.token_port.as_ref()?;
        let spec = self.token_spec.as_ref()?;
        Some(port.observe(spec))
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
    let token_spec = forgejo_token_spec(&desired, None);
    let observer = RuntimeForgejoObserver::with_token_port(
        current_repo_access_status(),
        Box::new(LocalForgejoTokenApiAdapter),
        token_spec,
    );
    Ok(build_forgejo_plan(desired, &observer))
}

/// Read-back proof of the token lifecycle (file metadata + API validity + plan).
///
/// Composable and observable as its own surface. Used by `current_forgejo_plan()`
/// and projected directly by `cynic forgejo token-status`.
pub fn current_forgejo_token_status()
-> Result<cynic_vcs::forgejo_token::ForgejoTokenStatusReport, String> {
    let root = current_project_root().ok_or_else(|| "CYNIC project root not found".to_string())?;
    let desired = load_forgejo_desired_state(&root)?;
    let spec = forgejo_token_spec(&desired, None);
    let port: Box<dyn ForgejoTokenApiPort> = Box::new(LocalForgejoTokenApiAdapter);
    let observation = port.observe(&spec);
    let plan =
        cynic_vcs::forgejo_token::plan_forgejo_token_convergence(spec.clone(), &observation);
    let token_action_needed = matches!(
        observation.api_authenticated,
        ForgejoTokenAuthState::Absent | ForgejoTokenAuthState::Refuted
    );
    Ok(cynic_vcs::forgejo_token::ForgejoTokenStatusReport {
        schema: cynic_vcs::forgejo_token::FORGEJO_TOKEN_STATUS_SCHEMA.to_string(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_vcs::FORGEJO_TOKEN_OBSERVATION_SCHEMA;
    use cynic_protocol::SecurityEvidenceState as ForgejoEvidenceState;

    #[derive(Debug)]
    struct FixedTokenPort(ForgejoTokenObservation);

    impl ForgejoTokenApiPort for FixedTokenPort {
        fn observe(&self, _spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
            self.0.clone()
        }

        fn create(
            &self,
            _spec: &ForgejoTokenSpec,
        ) -> Result<cynic_vcs::ForgejoTokenMetadata, String> {
            Err("not used in observation test".to_string())
        }

        fn remove(&self, _spec: &ForgejoTokenSpec, _token_sha1: &str) -> Result<(), String> {
            Err("not used in observation test".to_string())
        }
    }

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
            repository_permission: cynic_vcs::ForgejoRepositoryPermission::Write,
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
    fn runtime_observer_uses_injected_token_observation() {
        let token = ForgejoTokenObservation {
            schema: FORGEJO_TOKEN_OBSERVATION_SCHEMA.to_string(),
            principal: "cynic-machine".to_string(),
            path: "/redacted".to_string(),
            secret_present: true,
            owner_root: Some(true),
            mode_o640: Some(true),
            api_authenticated: ForgejoTokenAuthState::Refuted,
            unknowns: Vec::new(),
            evidence_refs: vec!["token-api-check".to_string()],
        };
        let observer = RuntimeForgejoObserver::with_token_port(
            RepoAccessStatusReport {
                schema: "test",
                mutates: false,
                profile_path: "profile.json".into(),
                profile_exists: true,
                canonical_project_root: None,
                known_repos: Vec::new(),
                sovereign_remotes: Vec::new(),
                entries: Vec::new(),
            },
            Box::new(FixedTokenPort(token)),
            ForgejoTokenSpec::default(),
        );
        assert_eq!(
            observer.token_observation().unwrap().api_authenticated,
            ForgejoTokenAuthState::Refuted
        );
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
            repository_permission: cynic_vcs::ForgejoRepositoryPermission::Write,
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
