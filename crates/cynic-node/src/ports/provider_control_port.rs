use cynic_protocol::{
    NodeSubstrateAction, NodeSubstrateDesiredState, ProviderControlCapability,
    ProviderMachineIdentity,
};

pub trait ProviderControlPort {
    fn identity(&self) -> ProviderMachineIdentity;
    fn capabilities(&self) -> Vec<ProviderControlCapability>;
    fn apply_power(
        &self,
        desired: &NodeSubstrateDesiredState,
        action: &NodeSubstrateAction,
    ) -> Result<(), String>;
}
