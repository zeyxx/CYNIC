use serde::{Deserialize, Serialize};

pub const PROVIDER_MACHINE_IDENTITY_SCHEMA: &str = "cynic.provider_machine_identity.v1";
pub const PROVIDER_CONTROL_CAPABILITY_SCHEMA: &str = "cynic.provider_control_capability.v1";

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
