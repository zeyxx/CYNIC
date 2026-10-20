//! Host hardware and capacity observation probes.

pub mod capacity;
pub mod cpu;
pub mod disk;
pub mod memory;

use clap::Subcommand;
use std::process::ExitCode;

pub(crate) use capacity::execute_capacity;
pub(crate) use cpu::execute_cpu;
pub(crate) use disk::{RuntimeDiskCommand, execute_disk_command};
pub(crate) use memory::execute_memory;

#[derive(Debug, Subcommand)]
pub enum NodeHostCommand {
    /// Probe host CPU model, topology, jiffies, and pressure stalls.
    Cpu {
        #[arg(long)]
        json: bool,
    },
    /// Probe host memory capacity, swap, hugepages, and pressure stalls.
    Memory {
        #[arg(long)]
        json: bool,
    },
    /// Probe host storage capacity, filesystems, caches and large files.
    Disk {
        #[command(subcommand)]
        command: RuntimeDiskCommand,
    },
    /// Assess local capacity: load tier + RAM/CPU headroom (read-only).
    Capacity {
        #[arg(long)]
        json: bool,
    },
    /// Audit host commits, divergence, and node health state.
    Audit {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 80)]
        max_commits: usize,
    },
}

pub fn execute_node_host(command: NodeHostCommand) -> ExitCode {
    match command {
        NodeHostCommand::Cpu { json } => execute_cpu(json),
        NodeHostCommand::Memory { json } => execute_memory(json),
        NodeHostCommand::Disk { command } => execute_disk_command(command),
        NodeHostCommand::Capacity { json } => execute_capacity(json),
        NodeHostCommand::Audit { json, max_commits } => {
            crate::node_surface::activation::execute_host_audit(json, max_commits)
        }
    }
}
