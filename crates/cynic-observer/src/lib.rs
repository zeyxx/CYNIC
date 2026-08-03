pub mod consensus;
pub mod dashboard;
pub mod delegation;
pub mod federation;
pub mod parser;
pub mod signal_mapper;
pub mod solana_archive;
pub mod wal;
pub mod watcher;

use serde::{Deserialize, Serialize};

/// Structure d'un événement capté sur la blockchain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ObserverEvent {
    pub signature: String,
    pub slot: u64,
    pub program_id: String,
    pub payload: Vec<u8>,
}

/// Statut d'un événement soumis au consensus de la fédération.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventStatus {
    PendingDelegation,
    ProcessingByLeader,
    Validated,
    Rejected,
}
