use crate::domain::network_execution::NetworkExecutionBoundary;
use crate::network_actuator::CommandExecution;
use crate::network_command_surface::NetworkCommandSurface;
use cynic_host::runtime_harness::RuntimeHarnessTargetAttestation;

pub(crate) trait NetworkCommandRunnerPort {
    fn execution_boundary(&self) -> NetworkExecutionBoundary;

    fn target_attestation(&self) -> Option<&RuntimeHarnessTargetAttestation> {
        None
    }

    fn run_command(&self, command: &NetworkCommandSurface) -> CommandExecution;
}
