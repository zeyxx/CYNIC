use cynic_host::source_control::{
    GitSourceControlAdapter, SourceControlPort, SourcePushAuthorization,
    observe_source_push_target, observe_source_worktree,
};
use cynic_host::source_lease::SourceLeaseVerifier;
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::{LeaseGuard, SecurityEvidenceState, SourceChangeProposal, SourceMatterClass};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "op")]
pub enum SourceControlOp {
    #[serde(rename = "observe")]
    Observe {
        #[serde(default)]
        expected_owner: Option<String>,
    },
    #[serde(rename = "propose")]
    Propose { proposal: SourceChangeProposalInput },
    #[serde(rename = "commit")]
    Commit { proposal: SourceChangeProposalInput },
    #[serde(rename = "push")]
    Push {
        proposal: SourceChangeProposalInput,
        remote: String,
        branch: String,
    },
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SourceChangeProposalInput {
    pub schema: String,
    pub proposal_id: String,
    pub repository_id: String,
    pub worktree_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub base_commit: String,
    pub actor: String,
    pub node_id: String,
    pub intent: String,
    pub matter: String,
    pub paths: Vec<String>,
    pub validation: String,
    pub rollback: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl SourceChangeProposalInput {
    fn to_protocol(&self) -> Result<SourceChangeProposal, String> {
        let matter = serde_json::from_value::<SourceMatterClass>(serde_json::Value::String(
            self.matter.clone(),
        ))
        .map_err(|error| format!("invalid matter class: {error}"))?;
        let validation = serde_json::from_value::<SecurityEvidenceState>(
            serde_json::Value::String(self.validation.clone()),
        )
        .map_err(|error| format!("invalid validation state: {error}"))?;
        Ok(SourceChangeProposal {
            schema: self.schema.clone(),
            proposal_id: self.proposal_id.clone(),
            repository_id: self.repository_id.clone(),
            worktree_id: self.worktree_id.clone(),
            lease_id: self.lease_id.clone(),
            fencing_token: self.fencing_token,
            base_commit: self.base_commit.clone(),
            actor: self.actor.clone(),
            node_id: self.node_id.clone(),
            intent: self.intent.clone(),
            matter,
            paths: self.paths.clone(),
            validation,
            rollback: self.rollback.clone(),
            evidence_refs: self.evidence_refs.clone(),
        })
    }
}

pub fn source_control_lease_guard(op: &SourceControlOp) -> Result<Option<LeaseGuard>, String> {
    match op {
        SourceControlOp::Commit { proposal } | SourceControlOp::Push { proposal, .. } => proposal
            .to_protocol()
            .map(|proposal| Some(proposal.lease_guard())),
        SourceControlOp::Observe { .. } | SourceControlOp::Propose { .. } => Ok(None),
    }
}

pub fn run_source_control(
    project_root: &str,
    op: &SourceControlOp,
    lease_verifier: &dyn SourceLeaseVerifier,
) -> serde_json::Value {
    let runner = LocalArgvRunner;
    let adapter = GitSourceControlAdapter::new(project_root, &runner, lease_verifier);
    match op {
        SourceControlOp::Observe { expected_owner } => serde_json::json!({
            "schema": "cynic.mcp.source_control_observation.v1",
            "snapshot": adapter.observe(),
            "worktree": observe_source_worktree(std::path::Path::new(project_root), expected_owner.as_deref()),
        }),
        SourceControlOp::Propose { proposal } => {
            source_control_with_input(proposal, |proposal| adapter.admit(&proposal))
        }
        SourceControlOp::Commit { proposal } => {
            source_control_with_input(proposal, |proposal| adapter.commit(&proposal))
        }
        SourceControlOp::Push {
            proposal,
            remote,
            branch,
        } => source_control_with_input(proposal, |proposal| {
            adapter.push(
                &proposal,
                remote,
                branch,
                SourcePushAuthorization {
                    remote_target: observe_source_push_target(&proposal.repository_id, remote),
                    transport_identity: SecurityEvidenceState::Unknown,
                    authentication: SecurityEvidenceState::Unknown,
                    remote_commit_matches: SecurityEvidenceState::Unknown,
                },
            )
        }),
    }
}

fn source_control_with_input<T: serde::Serialize>(
    input: &SourceChangeProposalInput,
    operation: impl FnOnce(SourceChangeProposal) -> T,
) -> serde_json::Value {
    match input.to_protocol() {
        Ok(proposal) => serde_json::to_value(operation(proposal))
            .unwrap_or_else(|_| serde_json::json!({"status":"error"})),
        Err(error) => {
            serde_json::json!({"schema":"cynic.mcp.source_control.v1","status":"blocked","blockers":[error]})
        }
    }
}
