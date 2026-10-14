//! Storage lifecycle commands — observe or reconcile node-local storage services.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

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
    )
    .await
}
