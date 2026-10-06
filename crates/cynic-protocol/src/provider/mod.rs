mod credential_lease_verifier;
mod decision;
mod ports;
mod types;

pub use credential_lease_verifier::CredentialLeaseVerifierPort;
pub use decision::{
    reconcile_node_substrate, validate_node_substrate_provisioning,
    validate_provider_control_desired_state,
};
pub use ports::{NodeSubstrateControlPort, NodeSubstrateObservationPort};
pub use types::{
    NODE_SUBSTRATE_DECISION_SCHEMA, NODE_SUBSTRATE_OBSERVATION_SCHEMA,
    NODE_SUBSTRATE_PROVISIONING_SCHEMA, NodeNetworkAttachment, NodeSubstrateAction,
    NodeSubstrateDecision, NodeSubstrateDesiredState, NodeSubstrateExistence,
    NodeSubstrateObservation, NodeSubstratePower, NodeSubstrateProvisioningSpec,
    PROVIDER_CONTROL_CAPABILITY_SCHEMA, PROVIDER_CONTROL_DESIRED_STATE_SCHEMA,
    PROVIDER_MACHINE_IDENTITY_SCHEMA, ProviderControlCapability, ProviderControlDesiredState,
    ProviderIdentityKind, ProviderMachineIdentity,
};

#[cfg(test)]
mod tests;
