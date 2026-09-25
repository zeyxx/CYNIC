use cynic_vcs_forgejo::ForgejoTokenApiPort;
use cynic_vcs_forgejo::LocalForgejoTokenApiAdapter;

use crate::repo_access::{
    RepoAccessStatusReport, current_project_root, current_repo_access_status,
    load_forgejo_desired_state,
};

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

#[derive(Debug)]
pub struct RuntimeForgejoObserver {
    repo_access: RepoAccessStatusReport,
    token_port: Option<Box<dyn ForgejoTokenApiPort>>,
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

        cynic_vcs::ForgejoObservedState {
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
}
