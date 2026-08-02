use serde::{Deserialize, Serialize};

use crate::{AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope, SecurityEvidenceState};

pub const AGENT_ROOT_LAUNCH_REQUEST_SCHEMA: &str = "cynic.agent_root_launch_request.v1";
pub const AGENT_ROOT_LAUNCH_PLAN_SCHEMA: &str = "cynic.agent_root_launch_plan.v1";
pub const AGENT_SESSION_REQUEST_SCHEMA: &str = "cynic.agent_session_request.v1";
pub const AGENT_SESSION_AUTHORITY_SCHEMA: &str = "cynic.agent_session_authority.v1";
pub const AGENT_SESSION_PEER_OBSERVATION_SCHEMA: &str = "cynic.agent_session_peer_observation.v1";
pub const AGENT_SESSION_PLAN_SCHEMA: &str = "cynic.agent_session_plan.v1";
pub const AGENT_SESSION_RECORD_SCHEMA: &str = "cynic.agent_session_record.v1";
pub const AGENT_SESSION_RENEWAL_REQUEST_SCHEMA: &str = "cynic.agent_session_renewal_request.v1";
pub const AGENT_SESSION_RENEWAL_PLAN_SCHEMA: &str = "cynic.agent_session_renewal_plan.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionTransport {
    LocalUnixPeer,
    RemoteMutualIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionCredentialKind {
    LocalPeerBinding,
    OpaqueReference,
    SignedJwt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRootLaunchRequest {
    pub schema: String,
    pub request_id: String,
    pub agent_id: String,
    pub agent_type: String,
    pub node_id: String,
    pub objective: String,
    pub requested_at_unix_ms: u64,
    pub requested_ttl_ms: u64,
    pub transport: AgentSessionTransport,
    pub credential_kind: AgentSessionCredentialKind,
    #[serde(default)]
    pub capabilities: Vec<AgentCapabilityGrant>,
    pub budget: AgentBudget,
    #[serde(default)]
    pub expected_proofs: Vec<String>,
    pub confirmation_key_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRootLaunchPlan {
    pub schema: String,
    pub status: String,
    pub request_id: String,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
    pub session_request: Option<AgentSessionRequest>,
    pub delegation: Option<AgentDelegationEnvelope>,
}

impl AgentRootLaunchPlan {
    pub fn admitted(&self) -> bool {
        self.schema == AGENT_ROOT_LAUNCH_PLAN_SCHEMA
            && self.status == "planned"
            && self.session_request.is_some()
            && self.delegation.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionRequest {
    pub schema: String,
    pub request_id: String,
    pub agent_id: String,
    pub node_id: String,
    pub delegation_digest: String,
    pub requested_at_unix_ms: u64,
    pub requested_ttl_ms: u64,
    pub transport: AgentSessionTransport,
    pub credential_kind: AgentSessionCredentialKind,
    #[serde(default)]
    pub capabilities: Vec<AgentCapabilityGrant>,
    pub confirmation_key_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionEnrollment {
    pub principal: String,
    pub node_id: String,
    pub transport: AgentSessionTransport,
    pub credential_kind: AgentSessionCredentialKind,
    pub max_ttl_ms: u64,
    #[serde(default)]
    pub agent_types: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<AgentCapabilityGrant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionAuthority {
    pub schema: String,
    pub revision: u64,
    pub authority_digest: String,
    pub peer_observation_max_age_ms: u64,
    #[serde(default)]
    pub enrollments: Vec<AgentSessionEnrollment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionPeerObservation {
    pub schema: String,
    pub observed_at_unix_ms: u64,
    pub node_id: String,
    pub transport: AgentSessionTransport,
    pub principal: Option<String>,
    pub process_id: Option<u32>,
    pub user_id: Option<u32>,
    pub group_id: Option<u32>,
    pub transport_binding_sha256: Option<String>,
    pub peer_identity: SecurityEvidenceState,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionAction {
    Issue,
    ObservePeerAgain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionPlan {
    pub schema: String,
    pub status: String,
    pub request_id: String,
    pub agent_id: String,
    pub node_id: String,
    pub authority_revision: u64,
    pub credential_kind: AgentSessionCredentialKind,
    pub expires_at_unix_ms: Option<u64>,
    #[serde(default)]
    pub capabilities: Vec<AgentCapabilityGrant>,
    #[serde(default)]
    pub actions: Vec<AgentSessionAction>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl AgentSessionPlan {
    pub fn admitted(&self) -> bool {
        self.schema == AGENT_SESSION_PLAN_SCHEMA && self.status == "planned"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionState {
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionRecord {
    pub schema: String,
    pub session_id: String,
    pub request_id: String,
    pub agent_id: String,
    #[serde(default)]
    pub agent_type: Option<String>,
    pub node_id: String,
    pub delegation_digest: String,
    pub authority_revision: u64,
    pub credential_kind: AgentSessionCredentialKind,
    pub credential_digest: Option<String>,
    pub confirmation_key_sha256: Option<String>,
    pub transport_binding_sha256: Option<String>,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub generation: u64,
    pub state: AgentSessionState,
    pub revoked_at_unix_ms: Option<u64>,
    pub revocation_reason: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<AgentCapabilityGrant>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionRenewalRequest {
    pub schema: String,
    pub request_id: String,
    pub requested_at_unix_ms: u64,
    pub requested_ttl_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionRenewalPlan {
    pub schema: String,
    pub status: String,
    pub request_id: String,
    pub previous_session_id: String,
    pub agent_id: String,
    pub node_id: String,
    pub authority_revision: u64,
    pub replacement_generation: Option<u64>,
    pub expires_at_unix_ms: Option<u64>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl AgentSessionRenewalPlan {
    pub fn admitted(&self) -> bool {
        self.schema == AGENT_SESSION_RENEWAL_PLAN_SCHEMA && self.status == "planned"
    }
}

pub(super) fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
