//! Sovereign daemon execution surface — kernel & node daemons in the single binary.

use clap::Subcommand;
use std::process::ExitCode;
use tokio_util::sync::CancellationToken;

#[derive(Subcommand, Debug, Clone)]
pub enum DaemonCommand {
    /// Run the CYNIC Sovereign Kernel daemon (long-running).
    Kernel {
        #[arg(long)]
        reset: bool,
    },
    /// Run the CYNIC Node Control daemon (long-running).
    Node {
        #[arg(long, default_value = "/etc/cynic/node-control.toml")]
        config: String,
        #[arg(long)]
        watchdog: bool,
    },
}

pub async fn dispatch(cmd: DaemonCommand) -> ExitCode {
    match cmd {
        DaemonCommand::Kernel { reset } => {
            let mut entry_flags =
                cynic_kernel::bootstrap::entry::KernelEntryFlags::from_env_args();
            if reset {
                entry_flags.force_reprobe = true;
            }
            cynic_kernel::bootstrap::entry::load_env_file();
            cynic_kernel::bootstrap::entry::configure_stdio_mode(entry_flags);
            cynic_kernel::bootstrap::entry::init_tracing(entry_flags);

            if entry_flags.force_reprobe {
                if let Err(e) = cynic_kernel::bootstrap::entry::main_verify().await {
                    eprintln!("kernel bootstrap verify failed: {e}");
                    return ExitCode::FAILURE;
                }
            }

            match cynic_kernel::bootstrap::runtime_orchestrator::run_runtime(entry_flags).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("kernel runtime exited with error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        DaemonCommand::Node { config, watchdog } => {
            if watchdog {
                return cynic_node::watchdog::execute(&config).await;
            }

            let cfg = match cynic_node::config::load(&config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("failed to load node config from {config}: {e}");
                    return ExitCode::FAILURE;
                }
            };

            let cancel_token = CancellationToken::new();
            cynic_node::shutdown::install_signal_handler(cancel_token.clone());

            match cynic_node::node_control_runtime::run(cfg, cancel_token).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("node control runtime exited with error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
