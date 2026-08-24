//! Thin CLI glue for `cynic dev inference-gateway`.
//!
//! Spawns and supervises the always-on sovereign HTTP inference gateway.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use cynic_inference::{
    DomainTaskQueue, FsEmpiricalStore, HardwareBudget, InferenceGateway, InferenceGatewayConfig,
};

pub(super) fn run(
    listen: Option<String>,
    backend_port: Option<u16>,
    default_model: Option<String>,
    vault_dir: Option<PathBuf>,
    ledger: Option<PathBuf>,
) -> ExitCode {
    let listen_addr: SocketAddr = listen
        .unwrap_or_else(|| "127.0.0.1:8088".into())
        .parse()
        .unwrap_or_else(|_| "127.0.0.1:8088".parse().unwrap());

    let mut config = InferenceGatewayConfig::default();
    config.listen_addr = listen_addr;
    if let Some(bp) = backend_port {
        config.backend_port = bp;
    }
    if let Some(dm) = default_model {
        config.default_model = dm;
    }
    if let Some(vd) = vault_dir {
        config.vault_dir = vd;
    }

    let store = match ledger {
        Some(p) => FsEmpiricalStore::open(p),
        None => FsEmpiricalStore::open_default(),
    };

    let store = match store {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("inference-gateway: failed to open empirical ledger: {e}");
            return ExitCode::FAILURE;
        }
    };

    let task_queue = match DomainTaskQueue::open_default() {
        Ok(q) => Arc::new(q),
        Err(e) => {
            eprintln!("inference-gateway: failed to open domain task queue: {e}");
            return ExitCode::FAILURE;
        }
    };

    let hardware = probe_hardware_budget();
    let gateway = InferenceGateway::new(config, store, task_queue, hardware);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    eprintln!(
        "==> Starting CYNIC Sovereign Inference Gateway on http://{}",
        listen_addr
    );

    if let Err(e) = rt.block_on(async { gateway.serve().await }) {
        eprintln!("inference-gateway: server error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn probe_hardware_budget() -> HardwareBudget {
    let mut total_ram_bytes: u64 = 32 * 1024 * 1024 * 1024;
    let mut available_ram_bytes: u64 = 24 * 1024 * 1024 * 1024;

    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(val) = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse::<u64>().ok())
                {
                    total_ram_bytes = val * 1024;
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(val) = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse::<u64>().ok())
                {
                    available_ram_bytes = val * 1024;
                }
            }
        }
    }

    HardwareBudget {
        total_ram_bytes,
        available_ram_bytes,
        gpu_vram_bytes: 8 * 1024 * 1024 * 1024,
    }
}
