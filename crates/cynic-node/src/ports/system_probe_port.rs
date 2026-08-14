use crate::domain::system_observation::{PlatformObservation, SystemMetricsSample};

pub trait SystemMetricsPort {
    fn sample_metrics(&mut self) -> SystemMetricsSample;
}

pub trait SystemProbePlatformPort {
    fn observe_platform(&self) -> PlatformObservation;
    fn restart_critical_vmid(&self, vmid: &str);
}
