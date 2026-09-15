//! Sovereign Ledger Books — zero-SQL typed aggregate books for CYNIC.
//! Implements double-entry bookkeeping (*partita doppia*) across 6 canonical books:
//! 1. Citizen & Node identities (`citizens`, `nodes`)
//! 2. Mandates & Offices (`mandates`)
//! 3. Coordination & Claims (`claims`)
//! 4. Metabolism & Invoices (`invoices`)
//! 5. Evidence & Runtime Proofs (`proofs`)
//! 6. Unformalized Matter (`envelopes`)

use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

use crate::error::StorageError;
use crate::ports::{DomainProjector, ProjectionCoordinatorPort, StateTransaction, WalJournalPort};
use crate::projection::SqliteProjectionAdapter;
use crate::types::{CausalWitness, WalAppendInput, WalEntry, WalReceipt};

/// Trait implemented by any domain entity that can be posted to a LedgerBook.
pub trait BookEntry: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Canonical schema URI identifying this entry type in the WAL journal and SMR projections.
    fn schema_uri() -> &'static str;

    /// Unique identifier for this book entry.
    fn entry_id(&self) -> &str;
}

/// Abstract contract for a Ledger Book storing typed entries.
pub trait LedgerBook<T: BookEntry>: Send + Sync {
    /// Post a new or updated entry to the book, backed by an immutable WAL receipt.
    fn post(&self, entry: &T, witness: CausalWitness) -> Result<WalReceipt, StorageError>;

    /// Read an entry from the book by ID in O(1).
    fn read(&self, id: &str) -> Result<Option<T>, StorageError>;

    /// Scan all entries currently inscribed in the book.
    fn scan_all(&self) -> Result<Vec<T>, StorageError>;
}

/// Generic SMR projector that posts journal events into a dedicated SQLite book table.
pub struct GenericBookProjector<T: BookEntry> {
    table_name: &'static str,
    schema_uri: &'static str,
    _marker: std::marker::PhantomData<T>,
}

impl<T: BookEntry> GenericBookProjector<T> {
    pub fn new(table_name: &'static str) -> Self {
        Self {
            table_name,
            schema_uri: T::schema_uri(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: BookEntry> DomainProjector for GenericBookProjector<T> {
    fn supported_schemas(&self) -> &[&str] {
        std::slice::from_ref(&self.schema_uri)
    }

    fn apply(&self, entry: &WalEntry, tx: &mut dyn StateTransaction) -> Result<(), StorageError> {
        let create_table_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                schema TEXT NOT NULL,
                updated_seq INTEGER NOT NULL,
                payload_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_{}_schema ON {}(schema);",
            self.table_name, self.table_name, self.table_name
        );
        tx.execute_statement(&create_table_sql)?;

        let payload_str = std::str::from_utf8(&entry.payload)
            .map_err(|e| StorageError::Codec(format!("invalid UTF-8 in payload: {e}")))?;

        let book_entry: T = serde_json::from_str(payload_str)
            .map_err(|e| StorageError::Codec(format!("failed to deserialize book entry: {e}")))?;

        let insert_sql = format!(
            "INSERT INTO {} (id, schema, updated_seq, payload_json)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                 schema = ?2,
                 updated_seq = ?3,
                 payload_json = ?4;",
            self.table_name
        );

        let id = book_entry.entry_id().to_string();
        let schema = self.schema_uri.to_string();
        let seq = entry.sequence.0 as i64;
        let json_payload = payload_str.to_string();

        tx.execute_sql(&insert_sql, &[&id, &schema, &seq, &json_payload])?;

        Ok(())
    }
}

/// Concrete Zero-SQL typed book backed by an append-only WAL and an SMR projection view.
pub struct TypedBook<T: BookEntry, W: WalJournalPort> {
    table_name: &'static str,
    wal: Arc<W>,
    projection: SqliteProjectionAdapter,
    _marker: std::marker::PhantomData<T>,
}

impl<T: BookEntry, W: WalJournalPort> TypedBook<T, W> {
    pub fn new(
        table_name: &'static str,
        wal: Arc<W>,
        mut projection: SqliteProjectionAdapter,
    ) -> Result<Self, StorageError> {
        let projector = GenericBookProjector::<T>::new(table_name);
        projection.register_projector(Box::new(projector))?;

        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                schema TEXT NOT NULL,
                updated_seq INTEGER NOT NULL,
                payload_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_{}_schema ON {}(schema);",
            table_name, table_name, table_name
        );
        let count_check = format!("SELECT count(*) FROM {}", table_name);
        if projection.query_count(&count_check).is_err() {
            let conn = rusqlite::Connection::open(projection.db_path())
                .map_err(|e| StorageError::Io(e.to_string()))?;
            conn.execute_batch(&create_sql)
                .map_err(|e| StorageError::Io(format!("failed to create book table: {e}")))?;
        }

        Ok(Self {
            table_name,
            wal,
            projection,
            _marker: std::marker::PhantomData,
        })
    }
}

impl<T: BookEntry, W: WalJournalPort> LedgerBook<T> for TypedBook<T, W> {
    fn post(&self, entry: &T, witness: CausalWitness) -> Result<WalReceipt, StorageError> {
        let payload_json = serde_json::to_string(entry)
            .map_err(|e| StorageError::Codec(format!("serialization failed: {e}")))?;

        let input = WalAppendInput {
            schema: T::schema_uri().to_string(),
            payload: Bytes::from(payload_json),
            witness,
        };

        let receipts = self.wal.append_batch(&[input])?;
        let receipt = receipts
            .into_iter()
            .next()
            .ok_or_else(|| StorageError::Io("empty receipt from WAL append".to_string()))?;

        // Read back entry from WAL and project into SQLite book
        let mut stream = self
            .wal
            .scan_stream(receipt.sequence, Some(receipt.sequence))?;
        if let Some(entry_res) = stream.next() {
            let wal_entry = entry_res?;
            self.projection.project_entry(&wal_entry)?;
        }

        Ok(receipt)
    }

    fn read(&self, id: &str) -> Result<Option<T>, StorageError> {
        let sql = format!(
            "SELECT payload_json FROM {} WHERE id = ?1 LIMIT 1;",
            self.table_name
        );

        let conn = rusqlite::Connection::open(self.projection.db_path())
            .map_err(|e| StorageError::Io(e.to_string()))?;

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Io(format!("prepare failed: {e}")))?;

        let mut rows = stmt
            .query(rusqlite::params![id])
            .map_err(|e| StorageError::Io(format!("query failed: {e}")))?;

        if let Some(row) = rows.next().map_err(|e| StorageError::Io(e.to_string()))? {
            let json_str: String = row.get(0).map_err(|e| StorageError::Io(e.to_string()))?;
            let item: T = serde_json::from_str(&json_str)
                .map_err(|e| StorageError::Codec(format!("deserialize failed: {e}")))?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    fn scan_all(&self) -> Result<Vec<T>, StorageError> {
        let sql = format!(
            "SELECT payload_json FROM {} ORDER BY updated_seq ASC;",
            self.table_name
        );

        let conn = rusqlite::Connection::open(self.projection.db_path())
            .map_err(|e| StorageError::Io(e.to_string()))?;

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Io(format!("prepare failed: {e}")))?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| StorageError::Io(format!("query_map failed: {e}")))?;

        let mut items = Vec::new();
        for row_res in rows {
            let json_str = row_res.map_err(|e| StorageError::Io(e.to_string()))?;
            let item: T = serde_json::from_str(&json_str)
                .map_err(|e| StorageError::Codec(format!("deserialize failed: {e}")))?;
            items.push(item);
        }

        Ok(items)
    }
}

// ── Domain BookEntry Implementations ─────────────────────────────────────────

// 1. Identity & Node Accounts
impl BookEntry for cynic_protocol::peer::CitizenAccount {
    fn schema_uri() -> &'static str {
        cynic_protocol::peer::CITIZEN_ACCOUNT_SCHEMA
    }
    fn entry_id(&self) -> &str {
        &self.citizen_id
    }
}

impl BookEntry for cynic_protocol::peer::NodeAccount {
    fn schema_uri() -> &'static str {
        cynic_protocol::peer::NODE_ACCOUNT_SCHEMA
    }
    fn entry_id(&self) -> &str {
        &self.node_id
    }
}

// 2. Governance & Mandate Accounts
impl BookEntry for cynic_protocol::peer::MandateAccount {
    fn schema_uri() -> &'static str {
        cynic_protocol::peer::MANDATE_ACCOUNT_SCHEMA
    }
    fn entry_id(&self) -> &str {
        &self.mandate_id
    }
}

// 3. Coordination & Claims
impl BookEntry for cynic_protocol::agent_coordination::Claim {
    fn schema_uri() -> &'static str {
        cynic_protocol::agent_coordination::CLAIM_SCHEMA
    }
    fn entry_id(&self) -> &str {
        &self.claim_id
    }
}

// 4. Metabolism & Invoices
impl BookEntry for cynic_protocol::metabolic_invoice::MetabolicInvoice {
    fn schema_uri() -> &'static str {
        cynic_protocol::metabolic_invoice::METABOLIC_INVOICE_SCHEMA
    }
    fn entry_id(&self) -> &str {
        &self.source_commit
    }
}

// 5. Evidence & Proofs
impl BookEntry for cynic_protocol::runtime::RuntimeProof {
    fn schema_uri() -> &'static str {
        cynic_protocol::runtime::RUNTIME_PROOF_SCHEMA
    }
    fn entry_id(&self) -> &str {
        &self.gate
    }
}

// 6. Unformalized Matter / Opaque Envelope
impl BookEntry for crate::types::OpaqueEnvelope {
    fn schema_uri() -> &'static str {
        "cynic.opaque_envelope.v2"
    }
    fn entry_id(&self) -> &str {
        &self.envelope_id
    }
}

/// Sovereign master ledger holding the 6 specialized books of CYNIC.
pub struct CynicLedger<W: WalJournalPort> {
    pub citizens: TypedBook<cynic_protocol::peer::CitizenAccount, W>,
    pub nodes: TypedBook<cynic_protocol::peer::NodeAccount, W>,
    pub mandates: TypedBook<cynic_protocol::peer::MandateAccount, W>,
    pub claims: TypedBook<cynic_protocol::agent_coordination::Claim, W>,
    pub invoices: TypedBook<cynic_protocol::metabolic_invoice::MetabolicInvoice, W>,
    pub proofs: TypedBook<cynic_protocol::runtime::RuntimeProof, W>,
    pub envelopes: TypedBook<crate::types::OpaqueEnvelope, W>,
}

impl<W: WalJournalPort + 'static> CynicLedger<W> {
    pub fn open(wal: Arc<W>, projection: SqliteProjectionAdapter) -> Result<Self, StorageError> {
        Ok(Self {
            citizens: TypedBook::new("citizens", wal.clone(), projection.clone())?,
            nodes: TypedBook::new("nodes", wal.clone(), projection.clone())?,
            mandates: TypedBook::new("mandates", wal.clone(), projection.clone())?,
            claims: TypedBook::new("claims", wal.clone(), projection.clone())?,
            invoices: TypedBook::new("invoices", wal.clone(), projection.clone())?,
            proofs: TypedBook::new("proofs", wal.clone(), projection.clone())?,
            envelopes: TypedBook::new("envelopes", wal, projection)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CausalWitness, Hash512, PoHTick, SequenceNumber};
    use crate::wal::WalFsAdapter;
    use cynic_protocol::agent_citizenship::AgentBudgets;
    use cynic_protocol::agent_coordination::{
        AgentPhase, CLAIM_SCHEMA, Claim, ClaimRequest, ClaimScope, ClaimStatus, ClaimType,
    };

    fn test_witness() -> CausalWitness {
        CausalWitness {
            principal_pubkey: [11u8; 32],
            signature: [22u8; 64],
            mandate_id: Some("mandate-root".to_string()),
            proof_anchor: None,
        }
    }

    #[test]
    fn ledger_book_zero_sql_post_read_scan() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let wal_dir = temp_dir.path().join("wal");
        let db_path = temp_dir.path().join("state.db");

        let wal = Arc::new(WalFsAdapter::open(&wal_dir).expect("open wal"));
        let projection = SqliteProjectionAdapter::open(&db_path).expect("open projection");

        let book: TypedBook<Claim, WalFsAdapter> =
            TypedBook::new("claims", wal, projection).expect("book new");

        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_test_123".to_string(),
            request: ClaimRequest {
                schema: "cynic.claim_request.v1".to_string(),
                agent_id: "agent_alpha".to_string(),
                session_id: "session_001".to_string(),
                scope: ClaimScope::Crate {
                    crate_name: "cynic-storage".to_string(),
                },
                claim_type: ClaimType::Exclusive,
                budget: AgentBudgets {
                    actions: 10,
                    wall_seconds: 600,
                },
                ttl_seconds: 1800,
                max_lease_window_ms: 3600000,
                parent_claim_id: None,
            },
            lease_id: "lease_999".to_string(),
            fencing_token: 42,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 3600000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Crystallization,
            consumed: AgentBudgets::default(),
        };

        // 1. Inscribe in Book (post)
        let receipt = book.post(&claim, test_witness()).expect("post claim");
        assert_eq!(receipt.sequence, SequenceNumber(1));

        // 2. Read from Book (read)
        let retrieved = book
            .read("claim_test_123")
            .expect("read claim")
            .expect("must exist");
        assert_eq!(retrieved.claim_id, "claim_test_123");
        assert_eq!(retrieved.request.agent_id, "agent_alpha");
        assert_eq!(retrieved.phase, AgentPhase::Crystallization);

        // 3. Scan all entries in Book (scan_all)
        let all = book.scan_all().expect("scan all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].claim_id, "claim_test_123");
    }
}
