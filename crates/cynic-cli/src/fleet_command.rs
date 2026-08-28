use clap::Subcommand;

#[derive(Subcommand)]
pub enum FleetCommand {
    /// Observe the full Proxmox fleet (VMs, LXC, storage, SDN)
    Observe {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },
    /// Show status of all fleet resources
    Status,
    /// Show detailed info for a specific resource
    Info {
        /// Resource ID (lxc/100, qemu/105, node/pve)
        resource_id: String,
    },
    /// Destroy a stopped resource with proof
    Destroy {
        /// Resource ID to destroy
        vmid: u32,
        /// Dry-run: verify resource exists and is stopped
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation (requires --yes)
        #[arg(long)]
        yes: bool,
    },
    /// List available templates (VM/CT templates)
    Templates,
}
