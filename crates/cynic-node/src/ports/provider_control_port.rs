use cynic_protocol::{ProviderControlCapability, ProviderMachineIdentity};

use crate::domain::fleet_reconcile::FleetReconcileAction;

pub trait ProviderControlPort {
    fn identity(&self) -> ProviderMachineIdentity;
    fn capabilities(&self) -> Vec<ProviderControlCapability>;
    fn apply_power(&self, action: &FleetReconcileAction) -> Result<(), String>;
}
