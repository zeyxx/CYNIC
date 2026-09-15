use cynic_build::build_discovery::BuildDiscoveryReport;

pub trait BuildDiscoveryObservationPort {
    fn observe(&self) -> Result<BuildDiscoveryReport, String>;
}
