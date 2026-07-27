mod decision;
mod types;

pub use decision::reconcile_node_substrate;
pub use types::{
    NODE_SUBSTRATE_DECISION_SCHEMA, NODE_SUBSTRATE_OBSERVATION_SCHEMA, NodeNetworkAttachment,
    NodeSubstrateAction, NodeSubstrateDecision, NodeSubstrateDesiredState, NodeSubstrateExistence,
    NodeSubstrateObservation, NodeSubstratePower, PROVIDER_CONTROL_CAPABILITY_SCHEMA,
    PROVIDER_MACHINE_IDENTITY_SCHEMA, ProviderControlCapability, ProviderIdentityKind,
    ProviderMachineIdentity,
};

#[cfg(test)]
mod tests;
