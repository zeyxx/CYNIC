//! Embedded SQLite state projection coordinator implementing `ProjectionCoordinatorPort`.
//! Manages single-writer transactions, projection watermarks, SMR event replay, and DLQ isolation.

use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::StorageError;
use crate::ports::{DomainProjector, ProjectionCoordinatorPort, StateTransaction, WalJournalPort};
use crate::types::{SequenceNumber, WalEntry};

/// Transaction implementation wrapping a mutable reference to a `rusqlite::Transaction`.
pub struct SqliteStateTransaction<'a, 'conn> {
    tx: &'a rusqlite::Transaction<'conn>,
}

impl<'a, 'conn> StateTransaction for SqliteStateTransaction<'a, 'conn> {
    fn execute_statement(&mut self, statement: &str) -> Result<(), StorageError> {
        self.tx
            .execute_batch(statement)
            .map_err(|e| StorageError::Io(format!("SQL batch failed: {e}")))
    }

    fn execute_sql(
        &mut self,
        sql: &str,
        params: &[&(dyn rusqlite::ToSql + Sync)],
    ) -> Result<usize, StorageError> {
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|&p| p as &dyn rusqlite::ToSql).collect();
        self.tx
            .execute(sql, rusqlite_params.as_slice())
            .map_err(|e| StorageError::Io(format!("SQL execute failed: {e}")))
    }
}

/// Internal coordinator state holding database connection and registered projectors.
struct SqliteProjectionState {
    conn: Connection,
    projectors: Vec<Box<dyn DomainProjector>>,
    last_applied_sequence: SequenceNumber,
}

/// Thread-safe SQLite projection coordinator adapter.
#[derive(Clone)]
pub struct SqliteProjectionAdapter {
    state: Arc<Mutex<SqliteProjectionState>>,
    db_path: PathBuf,
}

impl SqliteProjectionAdapter {
    /// Open or create an embedded SQLite projection database.
    /// Enforces WAL mode, foreign keys, and initializes watermark and DLQ tables.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let db_path = path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StorageError::Io(e.to_string()))?;
        }

        let mut conn = Connection::open(&db_path).map_err(|e| StorageError::Io(e.to_string()))?;

        // Mandatory performance & durability pragmas
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA busy_timeout = 5000;
            PRAGMA foreign_keys = ON;
            ",
        )
        .map_err(|e| StorageError::Io(format!("failed to set SQLite pragmas: {e}")))?;

        // Initialize SMR watermark table
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS _cynic_storage_watermark (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                last_applied_seq INTEGER NOT NULL,
                last_entry_hash TEXT NOT NULL,
                updated_at_unix_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS _cynic_dlq (
                dlq_id INTEGER PRIMARY KEY AUTOINCREMENT,
                sequence INTEGER NOT NULL,
                schema TEXT NOT NULL,
                entry_hash TEXT NOT NULL,
                error_reason TEXT NOT NULL,
                payload_bytes BLOB NOT NULL,
                quarantined_at_unix_ms INTEGER NOT NULL
            );
            ",
        )
        .map_err(|e| StorageError::Io(format!("failed to init watermark/dlq tables: {e}")))?;

        // Read existing watermark
        let last_seq: u64 = conn
            .query_row(
                "SELECT last_applied_seq FROM _cynic_storage_watermark WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let state = SqliteProjectionState {
            conn,
            projectors: Vec::new(),
            last_applied_sequence: SequenceNumber(last_seq),
        };

        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            db_path,
        })
    }

    /// Replay all unapplied events from a WAL journal through the registered projectors.
    pub fn replay_from_wal<W: WalJournalPort>(&self, wal: &W) -> Result<u64, StorageError> {
        let latest_wal_seq = wal.latest_sequence()?;
        let last_projected = self.last_projected_sequence()?;

        if latest_wal_seq <= last_projected {
            return Ok(0); // Up to date
        }

        let start_seq = last_projected.next();
        let stream = wal.scan_stream(start_seq, Some(latest_wal_seq))?;
        let mut count = 0u64;

        for entry_result in stream {
            let entry = entry_result?;
            self.project_entry(&entry)?;
            count += 1;
        }

        Ok(count)
    }

    /// Execute a read-only query and return the count of matching rows.
    pub fn query_count(&self, sql: &str) -> Result<u64, StorageError> {
        let state = self.state.lock();
        state
            .conn
            .query_row(sql, [], |row| row.get(0))
            .map_err(|e| StorageError::Io(format!("query_count failed: {e}")))
    }

    /// Path to the backing SQLite database file.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

impl ProjectionCoordinatorPort for SqliteProjectionAdapter {
    fn register_projector(
        &mut self,
        projector: Box<dyn DomainProjector>,
    ) -> Result<(), StorageError> {
        let mut state = self.state.lock();
        state.projectors.push(projector);
        Ok(())
    }

    fn project_entry(&self, entry: &WalEntry) -> Result<(), StorageError> {
        let mut state = self.state.lock();

        // 1. Find matching projector for this schema
        let matching_idx = state
            .projectors
            .iter()
            .position(|p| p.supported_schemas().contains(&entry.schema.as_str()));

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if let Some(idx) = matching_idx {
            let tx_result = state.conn.transaction();
            match tx_result {
                Ok(tx) => {
                    let mut state_tx = SqliteStateTransaction { tx: &tx };
                    let apply_result = state.projectors[idx].apply(entry, &mut state_tx);

                    match apply_result {
                        Ok(()) => {
                            // Update watermark atomically inside the transaction
                            let watermark_res = tx.execute(
                                "INSERT INTO _cynic_storage_watermark (id, last_applied_seq, last_entry_hash, updated_at_unix_ms)
                                 VALUES (1, ?1, ?2, ?3)
                                 ON CONFLICT(id) DO UPDATE SET
                                     last_applied_seq = ?1,
                                     last_entry_hash = ?2,
                                     updated_at_unix_ms = ?3;",
                                params![entry.sequence.0, entry.entry_hash.to_prefixed_hex(), now_ms],
                            );

                            if let Err(e) = watermark_res {
                                return Err(StorageError::Io(format!(
                                    "watermark update failed: {e}"
                                )));
                            }

                            tx.commit()
                                .map_err(|e| StorageError::Io(format!("tx commit failed: {e}")))?;
                            state.last_applied_sequence = entry.sequence;
                            Ok(())
                        }
                        Err(e) => {
                            // Projector failed: roll back and quarantine in DLQ
                            drop(tx);
                            tracing::error!(
                                "Projector for schema {} failed at seq {}: {e}. Quarantining to DLQ.",
                                entry.schema,
                                entry.sequence.0
                            );

                            let dlq_res = state.conn.execute(
                                "INSERT INTO _cynic_dlq (sequence, schema, entry_hash, error_reason, payload_bytes, quarantined_at_unix_ms)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                                params![
                                    entry.sequence.0,
                                    entry.schema,
                                    entry.entry_hash.to_prefixed_hex(),
                                    e.to_string(),
                                    entry.payload.as_ref(),
                                    now_ms,
                                ],
                            );

                            if let Err(dlq_err) = dlq_res {
                                tracing::error!("Failed to record in DLQ: {dlq_err}");
                            }

                            // Do NOT brick the node on projection error
                            Ok(())
                        }
                    }
                }
                Err(e) => Err(StorageError::Io(format!("failed to open transaction: {e}"))),
            }
        } else {
            // Unhandled schema: update watermark and skip cleanly
            let res = state.conn.execute(
                "INSERT INTO _cynic_storage_watermark (id, last_applied_seq, last_entry_hash, updated_at_unix_ms)
                 VALUES (1, ?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET
                     last_applied_seq = ?1,
                     last_entry_hash = ?2,
                     updated_at_unix_ms = ?3;",
                params![entry.sequence.0, entry.entry_hash.to_prefixed_hex(), now_ms],
            );

            if let Err(e) = res {
                return Err(StorageError::Io(format!("watermark update failed: {e}")));
            }

            state.last_applied_sequence = entry.sequence;
            Ok(())
        }
    }

    fn replay_all(&self) -> Result<u64, StorageError> {
        // Replay is orchestrated through `replay_from_wal`
        Ok(0)
    }

    fn last_projected_sequence(&self) -> Result<SequenceNumber, StorageError> {
        Ok(self.state.lock().last_applied_sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::WalJournalPort;
    use crate::types::{CausalWitness, Hash512, PoHTick, WalAppendInput};
    use crate::wal::WalFsAdapter;
    use bytes::Bytes;

    /// Mock projector for a test schema.
    struct TestAccountProjector;

    impl DomainProjector for TestAccountProjector {
        fn supported_schemas(&self) -> &[&str] {
            &["cynic.test.account.v1"]
        }

        fn apply(
            &self,
            entry: &WalEntry,
            tx: &mut dyn StateTransaction,
        ) -> Result<(), StorageError> {
            tx.execute_statement(
                "CREATE TABLE IF NOT EXISTS test_accounts (id TEXT PRIMARY KEY, balance INTEGER);",
            )?;

            let id = format!("acc-{}", entry.sequence.0);
            let balance = 1000i64;
            tx.execute_sql(
                "INSERT INTO test_accounts (id, balance) VALUES (?1, ?2);",
                &[&id, &balance],
            )?;

            Ok(())
        }
    }

    fn test_witness() -> CausalWitness {
        CausalWitness {
            principal_pubkey: [1u8; 32],
            signature: [2u8; 64],
            mandate_id: None,
            proof_anchor: None,
        }
    }

    #[test]
    fn sqlite_projection_lifecycle_and_watermark() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("state.db");
        let mut adapter = SqliteProjectionAdapter::open(&db_path).expect("open projection");

        assert_eq!(
            adapter.last_projected_sequence().unwrap(),
            SequenceNumber::GENESIS
        );

        adapter
            .register_projector(Box::new(TestAccountProjector))
            .unwrap();

        // Project an entry
        let entry = WalEntry {
            sequence: SequenceNumber(1),
            poh_tick: PoHTick(1),
            previous_hash: Hash512::GENESIS,
            commit_hash: Hash512::GENESIS,
            entry_hash: Hash512([9u8; 64]),
            schema: "cynic.test.account.v1".to_string(),
            payload: Bytes::from_static(b"payload"),
            witness: test_witness(),
        };

        adapter.project_entry(&entry).expect("project entry");
        assert_eq!(
            adapter.last_projected_sequence().unwrap(),
            SequenceNumber(1)
        );

        // Verify account row was written in SQLite
        let count = adapter
            .query_count("SELECT count(*) FROM test_accounts")
            .expect("count");
        assert_eq!(count, 1);

        // Reopen database and verify watermark persisted
        drop(adapter);
        let reopened = SqliteProjectionAdapter::open(&db_path).expect("reopen");
        assert_eq!(
            reopened.last_projected_sequence().unwrap(),
            SequenceNumber(1)
        );
    }

    #[test]
    fn sqlite_projection_replay_from_wal() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let wal_dir = temp_dir.path().join("wal");
        let db_path = temp_dir.path().join("state.db");

        let wal = WalFsAdapter::open(&wal_dir).expect("open WAL");
        let mut projection = SqliteProjectionAdapter::open(&db_path).expect("open projection");
        projection
            .register_projector(Box::new(TestAccountProjector))
            .unwrap();

        // Append 3 events to WAL
        for i in 1..=3 {
            let input = WalAppendInput {
                schema: "cynic.test.account.v1".to_string(),
                payload: Bytes::from(format!("entry {i}")),
                witness: test_witness(),
            };
            wal.append_batch(&[input]).expect("append");
        }

        assert_eq!(wal.latest_sequence().unwrap(), SequenceNumber(3));
        assert_eq!(
            projection.last_projected_sequence().unwrap(),
            SequenceNumber::GENESIS
        );

        // Replay WAL into projection
        let replayed = projection.replay_from_wal(&wal).expect("replay");
        assert_eq!(replayed, 3);
        assert_eq!(
            projection.last_projected_sequence().unwrap(),
            SequenceNumber(3)
        );

        let count = projection
            .query_count("SELECT count(*) FROM test_accounts")
            .unwrap();
        assert_eq!(count, 3);
    }
}
