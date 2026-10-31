//! `cynic host` — local substrate operations absorbed from `cynic-host-exec`.
//!
//! WHY. The `cynic-host-exec` binary was a second entry point to the same
//! `cynic_host::host_exec_cli` dispatch. Every leaf below forwards to that
//! single parser (flat profile string + trailing args), so the CLI owns a
//! hyphen-free two-level grammar while the host crate owns the flags: one
//! parser, no drift, and zero new entries in the hyphen taxonomy guard.
//! Mutating leaves (`--apply`) run with the invoker's UID; root execution
//! travels exclusively through `cynic privilege exec <capability-id>` with
//! the generated sudoers membrane naming that exact argv.
//!
//! NEGATIVE BOUNDARY. This surface parses nothing itself beyond the two
//! levels: flag parsing lives in `cynic_host::host_exec_cli`. It never
//! spawns processes, never writes files, never decides policy.

use std::process::ExitCode;

use clap::Subcommand;

use crate::fail;

/// A host leaf: its flat host-exec profile plus passthrough args.
fn run(profile: &str, args: &[String]) -> ExitCode {
    let mut forwarded = Vec::with_capacity(args.len() + 1);
    forwarded.push(profile.to_string());
    forwarded.extend(args.iter().cloned());
    match cynic_host::host_exec_cli::execute(forwarded) {
        Ok(code) => code,
        Err(error) => fail(error),
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum BridgeCommand {
    /// Stage the privilege bridge bundle
    Bundle {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Plan the privilege bridge install
    Plan {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Install the privilege bridge (mutating: prefer the capability)
    Install {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Observe bridge status
    Status {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum MembraneCommand {
    /// Stage the PVE helper membrane bundle
    Bundle {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Install the PVE helper membrane (mutating: prefer the capability)
    Install {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum TransportCommand {
    /// Execute a declared transport capability
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Observe the transport matrix
    Matrix {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SshCommand {
    /// Inspect an SSH target's host key
    Inspect {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Admit an SSH host key (mutating: prefer the capability)
    Admit {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum BootstrapCommand {
    /// Bootstrap status
    Status {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Bootstrap plan
    Plan {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Bootstrap chain
    Chain {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Bootstrap evidence
    Evidence {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum RunnerCommand {
    /// Bootstrap the build runner
    Bootstrap {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum CapabilityCommand {
    /// Issue a signed bootstrap capability binding
    Issue {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PrincipalCommand {
    /// Prepare the runtime principal (mutating: prefer the capability)
    Prepare {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SeedCommand {
    /// Plan a runtime seed rotation
    Plan {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Reconcile a runtime seed rotation (mutating: prefer the capability)
    Reconcile {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum StorageCommand {
    /// Enroll the storage identity (mutating: prefer the capability)
    Enroll {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Migrate the storage schema (mutating: prefer the capability)
    Migrate {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum HardwareCommand {
    /// Enroll the hardware baseline (mutating: prefer the capability)
    Enroll {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ServiceCommand {
    /// Reconcile systemd services (mutating: prefer the capability)
    Reconcile {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum NetworkCommand {
    /// Observe the network baseline (add --apply to mutate via capability)
    Baseline {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Actuate a network profile (mutating: prefer the capability)
    Actuate {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum RequestCommand {
    /// Validate a host-exec request without executing
    Validate {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ToolingCommand {
    /// Current declared vs resolved dependency state.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// What an upgrade would change, without changing it.
    Plan {
        #[arg(long)]
        json: bool,
    },
    /// Advisory and drift audit over the dependency graph.
    Audit {
        #[arg(long)]
        json: bool,
    },
    /// Resolve and upgrade stale components.
    Upgrade {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        confirm: Option<String>,
        #[arg(long)]
        only: Vec<String>,
        /// Explicit source binary path for first-contact bootstrap.
        #[arg(long)]
        source: Option<String>,
    },
}

pub(crate) fn execute_tooling(command: ToolingCommand) -> ExitCode {
    match command {
        ToolingCommand::Status { json } => crate::deps_surface::execute_deps(json),
        ToolingCommand::Plan { json } => crate::deps_surface::execute_deps_plan(json),
        ToolingCommand::Audit { json } => crate::deps_surface::execute_deps_audit(json),
        ToolingCommand::Upgrade {
            json,
            yes,
            apply,
            confirm,
            only,
            source,
        } => crate::deps_surface::execute_deps_upgrade(
            json,
            apply,
            yes,
            confirm.as_deref(),
            &only,
            source.as_deref(),
        ),
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum HostCommand {
    /// Privilege bridge staging and install
    Bridge {
        #[command(subcommand)]
        command: BridgeCommand,
    },
    /// PVE sudoers membrane staging and install
    Membrane {
        #[command(subcommand)]
        command: MembraneCommand,
    },
    /// Declared transport capabilities
    Transport {
        #[command(subcommand)]
        command: TransportCommand,
    },
    /// SSH host-key inspection and trust admission
    Ssh {
        #[command(subcommand)]
        command: SshCommand,
    },
    /// Bootstrap observation chain
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommand,
    },
    /// Build runner staging
    Runner {
        #[command(subcommand)]
        command: RunnerCommand,
    },
    /// Bootstrap capability issuance
    Capability {
        #[command(subcommand)]
        command: CapabilityCommand,
    },
    /// Runtime principal preparation
    Principal {
        #[command(subcommand)]
        command: PrincipalCommand,
    },
    /// Runtime seed rotation ceremonies
    Seed {
        #[command(subcommand)]
        command: SeedCommand,
    },
    /// Storage identity and schema ceremonies
    Storage {
        #[command(subcommand)]
        command: StorageCommand,
    },
    /// Hardware baseline attestation
    Hardware {
        #[command(subcommand)]
        command: HardwareCommand,
    },
    /// Systemd service reconciliation
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    /// Network baseline and actuation
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },
    /// Host-exec request validation
    Request {
        #[command(subcommand)]
        command: RequestCommand,
    },
    /// Host tooling (dependency state, plan, audit, upgrade)
    Tooling {
        #[command(subcommand)]
        command: ToolingCommand,
    },
}

/// Resolve a two-level command to the flat host-exec profile it forwards to.
/// The profile strings are the load-bearing wire shared with the
/// `cynic-host-exec` binary argv and the capability templates: keep verbatim.
fn profile_of(command: &HostCommand) -> &'static str {
    match command {
        HostCommand::Bridge { command } => match command {
            BridgeCommand::Bundle { .. } => "bridge-bundle",
            BridgeCommand::Plan { .. } => "bridge-plan",
            BridgeCommand::Install { .. } => "bridge-install",
            BridgeCommand::Status { .. } => "bridge-status",
        },
        HostCommand::Membrane { command } => match command {
            MembraneCommand::Bundle { .. } => "pve-helper-bundle",
            MembraneCommand::Install { .. } => "pve-helper-install",
        },
        HostCommand::Transport { command } => match command {
            TransportCommand::Run { .. } => "transport-run",
            TransportCommand::Matrix { .. } => "transport-matrix",
        },
        HostCommand::Ssh { command } => match command {
            SshCommand::Inspect { .. } => "ssh-inspect",
            SshCommand::Admit { .. } => "ssh-trust-admit",
        },
        HostCommand::Bootstrap { command } => match command {
            BootstrapCommand::Status { .. } => "bootstrap-status",
            BootstrapCommand::Plan { .. } => "bootstrap-plan",
            BootstrapCommand::Chain { .. } => "bootstrap-chain",
            BootstrapCommand::Evidence { .. } => "bootstrap-evidence",
        },
        HostCommand::Runner { command } => match command {
            RunnerCommand::Bootstrap { .. } => "build-runner-bootstrap",
        },
        HostCommand::Capability { command } => match command {
            CapabilityCommand::Issue { .. } => "capability-issue",
        },
        HostCommand::Principal { command } => match command {
            PrincipalCommand::Prepare { .. } => "principal-prepare",
        },
        HostCommand::Seed { command } => match command {
            SeedCommand::Plan { .. } => "runtime-seed-plan",
            SeedCommand::Reconcile { .. } => "runtime-seed-reconcile",
        },
        HostCommand::Storage { command } => match command {
            StorageCommand::Enroll { .. } => "storage-identity-enroll",
            StorageCommand::Migrate { .. } => "storage-schema-migrate",
        },
        HostCommand::Hardware { command } => match command {
            HardwareCommand::Enroll { .. } => "hardware-baseline-enroll",
        },
        HostCommand::Service { command } => match command {
            ServiceCommand::Reconcile { .. } => "service-reconcile",
        },
        HostCommand::Network { command } => match command {
            NetworkCommand::Baseline { .. } => "network-baseline",
            NetworkCommand::Actuate { .. } => "network-actuate",
        },
        HostCommand::Request { command } => match command {
            RequestCommand::Validate { .. } => "host-exec-validate-request",
        },
        HostCommand::Tooling { .. } => "host-tooling",
    }
}

fn args_of(command: &HostCommand) -> &[String] {
    match command {
        HostCommand::Bridge { command } => match command {
            BridgeCommand::Bundle { args }
            | BridgeCommand::Plan { args }
            | BridgeCommand::Install { args }
            | BridgeCommand::Status { args } => args,
        },
        HostCommand::Membrane { command } => match command {
            MembraneCommand::Bundle { args } | MembraneCommand::Install { args } => args,
        },
        HostCommand::Transport { command } => match command {
            TransportCommand::Run { args } | TransportCommand::Matrix { args } => args,
        },
        HostCommand::Ssh { command } => match command {
            SshCommand::Inspect { args } | SshCommand::Admit { args } => args,
        },
        HostCommand::Bootstrap { command } => match command {
            BootstrapCommand::Status { args }
            | BootstrapCommand::Plan { args }
            | BootstrapCommand::Chain { args }
            | BootstrapCommand::Evidence { args } => args,
        },
        HostCommand::Runner { command } => match command {
            RunnerCommand::Bootstrap { args } => args,
        },
        HostCommand::Capability { command } => match command {
            CapabilityCommand::Issue { args } => args,
        },
        HostCommand::Principal { command } => match command {
            PrincipalCommand::Prepare { args } => args,
        },
        HostCommand::Seed { command } => match command {
            SeedCommand::Plan { args } | SeedCommand::Reconcile { args } => args,
        },
        HostCommand::Storage { command } => match command {
            StorageCommand::Enroll { args } | StorageCommand::Migrate { args } => args,
        },
        HostCommand::Hardware { command } => match command {
            HardwareCommand::Enroll { args } => args,
        },
        HostCommand::Service { command } => match command {
            ServiceCommand::Reconcile { args } => args,
        },
        HostCommand::Network { command } => match command {
            NetworkCommand::Baseline { args } | NetworkCommand::Actuate { args } => args,
        },
        HostCommand::Request { command } => match command {
            RequestCommand::Validate { args } => args,
        },
        HostCommand::Tooling { .. } => &[],
    }
}

pub(crate) fn execute_host(command: HostCommand) -> ExitCode {
    if let HostCommand::Tooling { command } = command {
        return execute_tooling(command);
    }
    run(profile_of(&command), args_of(&command))
}
