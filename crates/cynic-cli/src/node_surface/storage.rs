//! Storage lifecycle commands — observe or reconcile node-local storage services.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

use clap::Subcommand;

pub(crate) async fn execute_storage_reconcile(
    state_dir: &str,
    warmup_seconds: u64,
    restart_window_seconds: u64,
    max_restarts_per_window: u32,
    apply: bool,
) -> ExitCode {
    cynic_node::usecases::storage_lifecycle_uc::execute_storage_reconcile(
        state_dir,
        warmup_seconds,
        restart_window_seconds,
        max_restarts_per_window,
        apply,
    ).await
}

/// Node storage arms: observe or reconcile a node-local storage service.
#[derive(Subcommand)]
pub enum NodeStorageCommand {
    /// Observe or reconcile one node-local storage service with a bounded restart budget.
    Reconcile {
        #[arg(long, default_value = "/var/lib/cynic/storage-lifecycle")]
        state_dir: String,
        #[arg(long, default_value_t = 30)]
        warmup_seconds: u64,
        #[arg(long, default_value_t = 300)]
        restart_window_seconds: u64,
        #[arg(long, default_value_t = 1)]
        max_restarts_per_window: u32,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) async fn execute_node_storage(command: NodeStorageCommand) -> ExitCode {
    match command {
        NodeStorageCommand::Reconcile {
            state_dir,
            warmup_seconds,
            restart_window_seconds,
            max_restarts_per_window,
            apply,
            json: _,
        } => {
            execute_storage_reconcile(
                &state_dir,
                warmup_seconds,
                restart_window_seconds,
                max_restarts_per_window,
                apply,
            )
            .await
        }
    }
}
