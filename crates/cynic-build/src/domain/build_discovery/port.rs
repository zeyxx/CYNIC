use cynic_protocol::build_discovery::BuildDiscoveryReport;

pub trait BuildDiscoveryObservationPort {
    fn observe(&self) -> Result<BuildDiscoveryReport, String>;
}
