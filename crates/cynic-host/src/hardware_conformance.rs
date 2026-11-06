//! Re-exported from `cynic-protocol` (L0).
pub use cynic_protocol::host_substrate::conformance::*;


pub trait HostHardwareObservationPort {
    fn observe_hardware(&self) -> HardwareObservation;
}

pub fn observe_hardware_conformance(
    observer: &impl HostHardwareObservationPort,
) -> HardwareConformanceReport {
    assess_hardware(observer.observe_hardware())
}