<<<<<<< HEAD
use cynic_protocol::{
    NodeSubstrateAction, NodeSubstrateDesiredState, ProviderControlCapability,
    ProviderMachineIdentity,
};
=======
use cynic_protocol::{ProviderControlCapability, ProviderMachineIdentity};

use crate::domain::fleet_reconcile::FleetReconcileAction;
>>>>>>> github/main

pub trait ProviderControlPort {
    fn identity(&self) -> ProviderMachineIdentity;
    fn capabilities(&self) -> Vec<ProviderControlCapability>;
<<<<<<< HEAD
    fn apply_power(
        &self,
        desired: &NodeSubstrateDesiredState,
        action: &NodeSubstrateAction,
    ) -> Result<(), String>;
=======
    fn apply_power(&self, action: &FleetReconcileAction) -> Result<(), String>;
>>>>>>> github/main
}
