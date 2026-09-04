//! Orchestrator coordinating the WAL (SSOT) and SQLite (View Projection).

use std::path::{Path, PathBuf};

use cynic_protocol::peer::{
    GENESIS_PREV_HASH, LedgerTransaction, PeerState, TransactionReceipt, TransitionError, WalEntry,
    apply_transaction, replay_wal,
};

use super::projection::PeerSqliteProjection;
use super::wal_file::PeerWalFile;

#[derive(Debug)]
pub enum PeerStorageError {
    Wal(String),
    Projection(String),
    Transition(TransitionError),
}

impl std::fmt::Display for PeerStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wal(m) => write!(f, "wal error: {m}"),
            Self::Projection(m) => write!(f, "projection error: {m}"),
            Self::Transition(e) => write!(f, "transition error: {e}"),
        }
    }
}

impl std::error::Error for PeerStorageError {}

pub struct PeerStorageManager {
    wal_file: PeerWalFile,
    projection: PeerSqliteProjection,
    state: PeerState,
    wal_path: PathBuf,
    sqlite_path: PathBuf,
}

impl PeerStorageManager {
    /// Initialize storage from a directory containing `cynic-peer.wal` and `cynic-peer.db`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, PeerStorageError> {
        let dir = dir.as_ref();
        let wal_path = dir.join("cynic-peer.wal");
        let sqlite_path = dir.join("cynic-peer.db");

        let (wal_file, entries) = PeerWalFile::open(&wal_path).map_err(PeerStorageError::Wal)?;

        let (state, _) = replay_wal(&PeerState::new(), &entries, GENESIS_PREV_HASH)
            .map_err(|e| PeerStorageError::Wal(e.to_string()))?;

        let projection =
            PeerSqliteProjection::open(&sqlite_path).map_err(PeerStorageError::Projection)?;

        projection
            .project_state(&state)
            .map_err(PeerStorageError::Projection)?;

        Ok(Self {
            wal_file,
            projection,
            state,
            wal_path,
            sqlite_path,
        })
    }

    /// Commit a transaction: prospective transition -> WAL append with fsync -> memory update -> SQLite projection.
    pub fn commit(
        &mut self,
        tx: LedgerTransaction,
    ) -> Result<TransactionReceipt, PeerStorageError> {
        // 1. Calculate prospective transition to verify validity before writing WAL
        let (next_state, receipt) =
            apply_transaction(&self.state, &tx).map_err(PeerStorageError::Transition)?;

        // 2. Seal and append to WAL
        let entry = WalEntry::seal(self.wal_file.last_hash(), tx);
        self.wal_file
            .append(&entry)
            .map_err(PeerStorageError::Wal)?;

        // 3. Update memory state
        self.state = next_state;

        // 4. Project to SQLite
        self.projection
            .project_state(&self.state)
            .map_err(PeerStorageError::Projection)?;

        Ok(receipt)
    }

    /// Complete crash-recovery: deletes SQLite database and completely rebuilds it by replaying the WAL.
    pub fn recover_projection_from_wal(&mut self) -> Result<(), PeerStorageError> {
        if self.sqlite_path.exists() {
            std::fs::remove_file(&self.sqlite_path)
                .map_err(|e| PeerStorageError::Projection(e.to_string()))?;
        }

        let (wal_file, entries) =
            PeerWalFile::open(&self.wal_path).map_err(PeerStorageError::Wal)?;

        let (state, _) = replay_wal(&PeerState::new(), &entries, GENESIS_PREV_HASH)
            .map_err(|e| PeerStorageError::Wal(e.to_string()))?;

        let projection =
            PeerSqliteProjection::open(&self.sqlite_path).map_err(PeerStorageError::Projection)?;

        projection
            .project_state(&state)
            .map_err(PeerStorageError::Projection)?;

        self.wal_file = wal_file;
        self.projection = projection;
        self.state = state;
        Ok(())
    }

    pub fn state(&self) -> &PeerState {
        &self.state
    }

    pub fn projection(&self) -> &PeerSqliteProjection {
        &self.projection
    }
}
