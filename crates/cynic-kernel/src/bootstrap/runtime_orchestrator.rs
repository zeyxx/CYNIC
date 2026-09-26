//! Minimal runtime orchestrator — waits for shutdown.
//! Full orchestration (storage, API, agents) will be re-enabled as separate crates.

use crate::bootstrap::entry::KernelEntryFlags;

pub async fn run_runtime(_flags: KernelEntryFlags) -> Result<(), Box<dyn std::error::Error>> {
    klog!("[Kernel] Runtime supervisor starting (minimal mode)");

    // TODO: Wire storage (cynic-storage), API (cynic-api), agents (cynic-agent-bus)
    klog!("[Kernel] Waiting for runtime components...");

    tokio::signal::ctrl_c().await?;
    klog!("[Kernel] Shutdown signal received");
    Ok(())
}
