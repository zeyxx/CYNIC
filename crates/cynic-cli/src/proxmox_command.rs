use clap::Subcommand;

#[derive(Subcommand)]
pub enum ProxmoxCommand {
    /// Observe all Proxmox cluster resources (VMs, LXC, nodes, storage)
    Observe {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },
    /// Show detailed info for a specific resource
    Info {
        /// Resource ID (lxc/100, qemu/105, node/pve)
        resource_id: String,
        /// Output JSON
        #[arg(long)]
        json: bool,
    },
    /// Destroy a stopped resource
    Destroy {
        /// Resource ID to destroy
        vmid: u32,
        /// Dry-run: verify resource exists and is stopped
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// List available VM/CT templates
    Templates,
}
