//! Host hardware and capacity observation probes.

pub mod capacity;
pub mod cpu;
pub mod disk;
pub mod memory;

use clap::Subcommand;
use std::process::ExitCode;

pub use capacity::execute_capacity;
pub use cpu::execute_cpu;
pub use disk::{RuntimeDiskCommand, execute_disk, execute_disk_command};
pub use memory::execute_memory;

#[derive(Subcommand)]
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
}

pub fn execute_node_host(command: NodeHostCommand) -> ExitCode {
    match command {
        NodeHostCommand::Cpu { json } => execute_cpu(json),
        NodeHostCommand::Memory { json } => execute_memory(json),
        NodeHostCommand::Disk { command } => execute_disk_command(command),
        NodeHostCommand::Capacity { json } => execute_capacity(json),
    }
}
