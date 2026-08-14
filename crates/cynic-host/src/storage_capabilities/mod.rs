mod local;
mod policy;
mod port;
mod types;

pub use port::StorageCapabilityObservationPort;
pub use types::{
    CapabilityEvidence, ProbeObservation, ProbeStatus, StorageCapabilitiesReport,
    StorageCapability, StorageCapabilityObservations, StorageRisk,
};

use local::LocalStorageCapabilityObserver;

pub fn observe_storage_capabilities(endpoint: &str) -> StorageCapabilitiesReport {
    observe_storage_capabilities_with(endpoint, &LocalStorageCapabilityObserver)
}

pub fn observe_storage_capabilities_with(
    endpoint: &str,
    observer: &impl StorageCapabilityObservationPort,
) -> StorageCapabilitiesReport {
    policy::build_report(
        endpoint,
        observer.observe_cli_version(),
        observer.observe_server_version(endpoint),
        observer.observe_server_health(endpoint),
        observer.observe_engine(),
    )
}
