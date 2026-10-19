//! Peer command types — domain types for the CynicPeer sovereign state engine.
//!
//! These describe the commands accepted by the peer SMR ingress. They are
//! domain types, not CLI types: they live in cynic-node because the peer
//! engine is a node capability, not a CLI surface.

use clap::Subcommand;
use cynic_types::paths::CynicPaths;

#[derive(Debug, Clone, Subcommand)]
pub enum PeerCommands {
    /// Show peer SMR status, accounts summary and state root digest.
    Status {
        /// Storage directory containing cynic-peer.wal and cynic-peer.db.
        #[arg(long, default_value_t = CynicPaths::state_dir().join("peer").display().to_string())]
        dir: String,
        /// Optional registry path used for auto-genesis bootstrap on fresh storage.
        /// Omit to use the resolved canonical registry.
        #[arg(long)]
        registry: Option<String>,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Inspect an account (citizen, node, or mandate) from local SQLite projection.
    Inspect {
        /// Storage directory containing cynic-peer.wal and cynic-peer.db.
        #[arg(long, default_value_t = CynicPaths::state_dir().join("peer").display().to_string())]
        dir: String,
        /// Account kind: citizen, node, or mandate.
        #[arg(long)]
        kind: String,
        /// Unique account identifier.
        #[arg(long)]
        id: String,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Submit work to the peer. The peer is the SMR;  is the verb.
    Submit {
        #[command(subcommand)]
        command: PeerSubmitCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PeerSubmitCommand {
    /// Submit a signed ledger transaction JSON file to the peer SMR.
    Tx {
        /// Storage directory containing cynic-peer.wal and cynic-peer.db.
        #[arg(long, default_value_t = CynicPaths::state_dir().join("peer").display().to_string())]
        dir: String,
        /// Path to JSON file containing a LedgerTransaction.
        #[arg(long)]
        file: String,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
}
