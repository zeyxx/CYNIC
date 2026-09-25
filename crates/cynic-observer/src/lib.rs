<<<<<<< HEAD
#![allow(unused_imports)]
pub mod buffer;
pub mod consensus;
pub mod dashboard;
pub mod delegation;
pub mod domain;
pub mod federation;
pub mod metabolism;
pub mod parser;
pub mod runner;
pub mod signal_mapper;
pub mod solana_archive;
pub mod source;
pub mod wal;
pub mod watcher;

pub use buffer::*;
pub use domain::*;
use serde::{Deserialize, Serialize};
pub use solana_archive::{FanoutPlanRequest, HeliusPlan, plan_fanout_request};
pub use source::*;

/// Structure d'un événement capté sur la blockchain (compatibilité Solana).
=======
pub mod consensus;
pub mod dashboard;
pub mod delegation;
pub mod federation;
pub mod parser;
pub mod signal_mapper;
pub mod wal;
pub mod watcher;

use serde::{Deserialize, Serialize};

/// Structure d'un événement capté sur la blockchain.
>>>>>>> github/main
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
