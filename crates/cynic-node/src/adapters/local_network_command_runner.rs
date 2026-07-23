use crate::domain::network_execution::NetworkExecutionBoundary;
use crate::network_actuator::CommandExecution;
use crate::network_actuator::execution::run_command;
use crate::network_command_surface::NetworkCommandSurface;
use crate::ports::network_command_runner_port::NetworkCommandRunnerPort;

pub(crate) struct LocalNetworkCommandRunner;

impl NetworkCommandRunnerPort for LocalNetworkCommandRunner {
    fn execution_boundary(&self) -> NetworkExecutionBoundary {
        NetworkExecutionBoundary::UnverifiedLocalProcess
    }

    fn run_command(&self, command: &NetworkCommandSurface) -> CommandExecution {
        run_command(command)
    }
}
