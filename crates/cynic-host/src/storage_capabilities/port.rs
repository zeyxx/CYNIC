use super::types::ProbeObservation;

pub trait StorageCapabilityObservationPort {
    fn observe_cli_version(&self) -> ProbeObservation;
    fn observe_server_version(&self, endpoint: &str) -> ProbeObservation;
    fn observe_server_health(&self, endpoint: &str) -> ProbeObservation;
    fn observe_engine(&self) -> ProbeObservation;
}
