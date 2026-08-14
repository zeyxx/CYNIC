use cynic_kernel::bootstrap::entry::{
    KernelEntryFlags, configure_stdio_mode, init_tracing, load_env_file,
    print_version_and_exit_if_requested, run_mcp_proxy_mode_if_requested,
};
use cynic_kernel::{bootstrap, klog};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_env_file();
    let entry_flags = KernelEntryFlags::from_env_args();

    if print_version_and_exit_if_requested(entry_flags)? {
        return Ok(());
    }

    configure_stdio_mode(entry_flags);
    init_tracing(entry_flags);

    if run_mcp_proxy_mode_if_requested(entry_flags).await? {
        return Ok(());
    }

    klog!("CYNIC V2 — SOVEREIGN BOOT");
    bootstrap::runtime_orchestrator::run_runtime(entry_flags).await
}
