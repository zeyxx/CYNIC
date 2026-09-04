//! Peer ingress services exposing status, account inspection, and transaction submission.
use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use std::path::Path;

use cynic_protocol::peer::{LedgerTransaction, TransactionReceipt};

use super::genesis::bootstrap_genesis_from_registry;
use super::manager::PeerStorageManager;
use super::projection::PeerSqliteProjection;

pub const PEER_STATUS_SCHEMA: &str = "cynic.peer.status.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerStatusReport {
    pub schema: &'static str,
    pub epoch: u64,
    pub last_sequence: u64,
    pub last_tx_hash: String,
    pub state_root_digest: String,
    pub citizens_count: usize,
    pub nodes_count: usize,
    pub agents_count: usize,
    pub offices_count: usize,
    pub mandates_count: usize,
    pub resources_count: usize,
    pub workloads_count: usize,
    pub genesis_bootstrapped_now: bool,
}

/// Query peer status, auto-bootstrapping from registry if storage is fresh.
pub fn peer_status(
    storage_dir: impl AsRef<Path>,
    registry_path: Option<impl AsRef<Path>>,
) -> Result<PeerStatusReport, String> {
    let storage_dir = storage_dir.as_ref();
    let mut manager = PeerStorageManager::open(storage_dir)
        .map_err(|e| format!("failed to open peer storage: {e}"))?;

    let mut bootstrapped = false;
    if manager.state().last_sequence == 0 {
        if let Some(reg_path) = registry_path {
            let txs = bootstrap_genesis_from_registry(&mut manager, reg_path)
                .map_err(|e| format!("genesis bootstrap failed: {e}"))?;
            bootstrapped = txs > 0;
        }
    }

    let state = manager.state();
    Ok(PeerStatusReport {
        schema: PEER_STATUS_SCHEMA,
        epoch: state.epoch,
        last_sequence: state.last_sequence,
        last_tx_hash: state.last_tx_hash.clone(),
        state_root_digest: state.state_root_digest(),
        citizens_count: state.citizens.len(),
        nodes_count: state.nodes.len(),
        agents_count: state.agents.len(),
        offices_count: state.offices.len(),
        mandates_count: state.mandates.len(),
        resources_count: state.resources.len(),
        workloads_count: state.workloads.len(),
        genesis_bootstrapped_now: bootstrapped,
    })
}

/// Fast indexed query of an account by kind ("citizen", "node", "mandate") from SQLite.
pub fn peer_inspect_account(
    storage_dir: impl AsRef<Path>,
    kind: &str,
    id: &str,
) -> Result<Option<serde_json::Value>, String> {
    let sqlite_path = storage_dir.as_ref().join("cynic-peer.db");
    let projection = PeerSqliteProjection::open(&sqlite_path)?;

    match kind.to_lowercase().as_str() {
        "citizen" | "citizens" => {
            let acct = projection.get_citizen(id)?;
            Ok(acct.map(|a| serde_json::to_value(a).unwrap_or_default()))
        }
        "node" | "nodes" => {
            let acct = projection.get_node(id)?;
            Ok(acct.map(|a| serde_json::to_value(a).unwrap_or_default()))
        }
        "mandate" | "mandates" => {
            let acct = projection.get_mandate(id)?;
            Ok(acct.map(|a| serde_json::to_value(a).unwrap_or_default()))
        }
        other => Err(format!(
            "unsupported account kind '{other}', expected citizen, node, or mandate"
        )),
    }
}

/// Submit a signed transaction to the local peer storage.
pub fn peer_submit_tx(
    storage_dir: impl AsRef<Path>,
    tx: LedgerTransaction,
) -> Result<TransactionReceipt, String> {
    let mut manager = PeerStorageManager::open(storage_dir.as_ref())
        .map_err(|e| format!("failed to open peer storage: {e}"))?;
    manager
        .commit(tx)
        .map_err(|e| format!("transaction commit failed: {e}"))
}

/// Execute a PeerCommands variant and emit the JSON result.
pub fn execute_peer_command(command: &crate::cli::PeerCommands) -> ExitCode {
    match command {
        crate::cli::PeerCommands::Status {
            dir,
            registry,
            json: _,
        } => match peer_status(dir, registry.as_deref()) {
            Ok(report) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_default()
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("peer status failed: {e}");
                ExitCode::FAILURE
            }
        },
        crate::cli::PeerCommands::Inspect {
            dir,
            kind,
            id,
            json: _,
        } => match peer_inspect_account(dir, kind, id) {
            Ok(Some(account)) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&account).unwrap_or_default()
                );
                ExitCode::SUCCESS
            }
            Ok(None) => {
                eprintln!("{kind} account '{id}' not found");
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("peer inspect failed: {e}");
                ExitCode::FAILURE
            }
        },
        crate::cli::PeerCommands::SubmitTx { dir, file, json: _ } => {
            let raw = match std::fs::read_to_string(file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("failed to read transaction file: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let tx: LedgerTransaction = match serde_json::from_str(&raw) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("failed to parse transaction JSON: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match peer_submit_tx(dir, tx) {
                Ok(receipt) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&receipt).unwrap_or_default()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("transaction submission failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
