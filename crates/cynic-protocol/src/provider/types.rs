use serde::{Deserialize, Serialize};

pub const PROVIDER_MACHINE_IDENTITY_SCHEMA: &str = "cynic.provider_machine_identity.v1";
pub const PROVIDER_CONTROL_CAPABILITY_SCHEMA: &str = "cynic.provider_control_capability.v1";
pub const NODE_SUBSTRATE_OBSERVATION_SCHEMA: &str = "cynic.node_substrate.observation.v1";
pub const NODE_SUBSTRATE_DECISION_SCHEMA: &str = "cynic.node_substrate.decision.v1";
pub const NODE_SUBSTRATE_PROVISIONING_SCHEMA: &str = "cynic.node_substrate.provisioning.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderIdentityKind {
    AwsWorkloadRole,
    KubernetesServiceAccount,
    ProxmoxApiToken,
    HostMachineIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderMachineIdentity {
    pub schema: String,
    pub provider: String,
    pub kind: ProviderIdentityKind,
    pub principal: String,
    pub credential_present: bool,
    pub secret_material_embedded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderControlCapability {
    pub schema: String,
    pub provider: String,
    pub resource_kind: String,
    pub operation: String,
    pub requires_explicit_apply: bool,
    pub produces_read_back_proof: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeSubstrateExistence {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeSubstratePower {
    Running,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeNetworkAttachment {
    Attached,
    Detached,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSubstrateDesiredState {
    pub node_id: String,
    pub provider: Option<String>,
    pub resource_id: Option<String>,
    pub desired_power: NodeSubstratePower,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provisioning: Option<NodeSubstrateProvisioningSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSubstrateProvisioningSpec {
    pub schema: String,
    pub image_ref: String,
    pub image_digest: String,
    pub runtime_seed_ref: String,
    pub runtime_seed_digest: String,
    pub network_profile_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSubstrateObservation {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub provider: Option<String>,
    pub resource_id: Option<String>,
    pub existence: NodeSubstrateExistence,
    pub power: NodeSubstratePower,
    pub network_attachment: NodeNetworkAttachment,
    pub observed_addresses: Vec<String>,
    pub management_reachable: Option<bool>,
    pub observation_authority: Option<String>,
    pub observed_at_unix_ms: Option<u64>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeSubstrateAction {
    ObserveProvider,
    Provision,
    RequestEnrollment,
    Start,
    Stop,
    AttachNetwork,
    ObserveManagement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSubstrateDecision {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub action: Option<NodeSubstrateAction>,
    pub mutates: bool,
    pub requires_explicit_apply: bool,
    pub blockers: Vec<String>,
    pub required_read_back: Vec<String>,
}
