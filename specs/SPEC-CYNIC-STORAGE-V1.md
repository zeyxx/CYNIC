# SPEC-CYNIC-STORAGE-V1: Sovereign Clean-Room Storage Architecture

- **Status**: Ratified by Engineering Tribunal (2026-09-11)
- **Author**: Titouan Corneloup (zeyxx) / Hermes Agent
- **Target Crate**: `crates/cynic-storage`
- **Related Requirements**: Foundation §11 (Digest Law: `sha3-512:`), §481 (`one_boundary_one_reason`), §489 (anti-centralization), Framework §462 (`substrate_behind_ports`), §476 (`proof_producers_are_low_layer`)
- **Supersedes**: `cynic-kernel/src/storage/` (SurrealDB monolithic coupling, SPOF F2)

---

## 1. Executive Summary & Problem Statement

The CYNIC codebase currently disperses storage across 83 files and 15,570 LOC:
1. **The SurrealDB SPOF (`cynic-kernel`)**: A remote HTTP/WebSocket daemon dependency (13 tables) mixing mutable state with immutable evidence, requiring 1,800 LOC of reconnection boilerplate.
2. **Inverted Dependency Direction**: `cynic-kernel` directly implements traits defined inside `cynic-node` (`AgentWorkspaceClaimStorePort`), violating hexagonal layering.
3. **Storage Fragmentation**: Four separate storage paradigms operate simultaneously with no unified interface: SurrealDB, embedded SQLite, Filesystem Content-Addressed Stores (CAS), and ad-hoc JSON files.

`cynic-storage` is a standalone, clean-room crate providing a unified, sovereign, zero-external-daemon persistence layer based on **Event-Sourced State Machine Replication (SMR)**. It decouples the append-only ledger of truth from disposable, read-optimized relational and vector projections.

---

## 2. Core Architectural Laws & Invariants

1. **INV-01 (View Derives From Log, Never Inverse)**: The append-only, hash-chained Write-Ahead Log (WAL) is the sole source of runtime truth. The embedded SQLite database is a disposable, reconstructible cache. Deleting SQLite must never cause data loss; replaying the WAL reconstructs the exact state.
2. **INV-02 (Hexagonal Purity & Zero Domain Bleed)**: Core storage domain contracts (`ports`) know nothing of `rusqlite`, filesystems, or networks. Storage ports do NOT declare hardcoded domain getters (e.g. `get_claim`, `get_account`); domain contexts own their schemas and project via the SMR reducer interface.
3. **INV-03 (Zero External Daemon SPOF)**: Storage operates strictly embedded within the CYNIC process space. No external database process (Postgres, SurrealDB, Redis, Qdrant) is required for steady-state sovereign operations.
4. **INV-04 (Cryptographic Causal Chain & Proof-of-History)**: Every mutation record is cryptographically bound to its predecessor via length-prefixed, domain-separated SHA3-512 hashing:
   $$\text{entry\_hash} = \text{SHA3-512}(\text{"CYNIC\_WAL\_ENTRY\_V1\textbackslash 0"} \parallel \text{seq} \parallel \text{prev\_hash} \parallel \text{poh\_tick} \parallel \text{schema\_len} \parallel \text{schema} \parallel \text{payload\_len} \parallel \text{payload\_hash} \parallel \text{witness})$$
   All digests must strictly carry the canonical `sha3-512:` prefix per Foundation §11. Wall-clock time is an observed payload attribute, never the cryptographic backbone of causal time.
5. **INV-05 (Handling Unformalized Matter)**: Entities not yet formalised into strict relational schemas are preserved as content-addressed, schema-tagged opaque envelopes (`cynic.opaque_envelope.v2`) carrying structured tags for indexed queries.
6. **INV-06 (Zero Secrets in the WAL)**: Raw credentials, private keys, API secrets, and unencrypted session tokens are strictly prohibited from being written to the WAL. Secrets must reside in principal-owned encrypted vaults; the WAL stores only cryptographic commitments (`sha3-512:secret_digest`) and URI handles (`vault://...`).
7. **INV-07 (Anti-Fabrication on Historical Matter)**: When historical records are projected into newly formalized relational schemas, missing attributes MUST be stored as `NULL` / `Option::None`. The projection engine is strictly prohibited from using SQL `DEFAULT` values or type-level defaults (`Default::default()`). Any unobserved field must propagate to the snapshot's `residual_unknowns` collection.
8. **MIG-01 (Genesis WAL Ingestion Law)**: Direct insertion of legacy data into SQLite without WAL event generation is an architectural violation of `INV-01`. All legacy state from SurrealDB and existing SQLite databases must enter CYNIC storage strictly as **Genesis WAL Events** (`WalEvent::LegacyImport`).

---

## 3. Dependency Graph & Hexagonal Topology

```text
                     cynic-protocol (Pure contracts, schemas, domain traits)
                            ▲                   ▲
                            │                   │
                     cynic-storage (WAL + SMR + SQLite Projections)
                            ▲                   ▲
             ┌──────────────┴──────────┐        │
             │                         │        │
        cynic-kernel              cynic-node ───┘
```

- `cynic-storage` depends **only** on `cynic-protocol` and low-layer utility primitives (`sha3`, `rusqlite`, `parking_lot`, `bytes`).
- `cynic-storage` NEVER imports `cynic-kernel`, `cynic-host`, or `cynic-node`.
- Consumers interact with storage through abstract traits; vendor drivers are internal adapters.

---

## 4. Binary Frame Format & Segmented Log Architecture

### 4.1 Binary Wire Format (Per Frame)

To prevent boundary collisions, partial-write deadlocks, and torn pages, all WAL records follow a strict binary framing protocol:

```text
+-------------------+-------------------+-------------------+-------------------+
| Magic (4 bytes)   | Version (2 bytes) | PayloadLen (4B BE)| HeaderCRC32C (4B) |
| 0x43 0x59 0x4E 0x57 | 0x00 0x01         | (u32)             | Castagnoli CRC32C |
+-------------------+-------------------+-------------------+-------------------+
| Sequence (8B BE)  | PoHTick (8B BE)   | PrevHash (64B raw)| SchemaLen (2B BE) |
+-------------------+-------------------+-------------------+-------------------+
| Schema Bytes (UTF-8, SchemaLen bytes)                                         |
+-------------------------------------------------------------------------------+
| Payload Bytes (PayloadLen bytes)                                              |
+-------------------------------------------------------------------------------+
| Causal Witness: PrincipalPubkey (32B) | Signature (64B Ed25519)               |
+-------------------------------------------------------------------------------+
| EntryHashDigest (64 bytes raw SHA3-512)                                       |
+-------------------------------------------------------------------------------+
```

### 4.2 Segment Rotation & Directory Layout

The filesystem adapter (`WalFsAdapter`) segments the log into fixed-size chunks:

```text
$CYNIC_DATA_DIR/storage/
├── wal.lock                     # Advisory kernel flock(2)
├── wal/
│   ├── 00000000000000000001.seg # 64 MB immutable segments
│   ├── 00000000000000000002.seg
│   └── active.seg               # Current append target
├── state.db                     # Disposable SQLite projection (WAL mode)
└── state.db-wal
```

### 4.3 Deterministic Crash Recovery Protocol (Torn Tail Truncation)

On initialization (`open`):
1. **Sequential Scan**: Validate `HeaderCRC32C` and `EntryHashDigest` frame by frame.
2. **Mid-Log Corruption (FATAL)**: If a checksum or cryptographic chain fails at byte offset $O$ where $O + \text{frame\_len} < \text{file\_len}$, the log has suffered tampering or hardware bit-rot. The engine **FAILS CLOSED** with `StorageError::CorruptedLedgerAtSequence(seq)`. No auto-truncation is allowed.
3. **Torn Tail at EOF (RECOVERABLE)**: If an incomplete frame or CRC mismatch occurs *strictly at the physical end of the file*:
   - Verify that all prior entries up to sequence $N$ passed cryptographic validation.
   - Truncate the file back to the end of sequence $N$ (`file.set_len(valid_tail_offset)`).
   - Issue an explicit `fdatasync()` on the file and parent directory.
   - Emit an audited event `cynic.storage.torn_write_recovered`.

---

## 5. Ports & Interfaces (Pure Rust Contracts)

```rust
use std::fmt::Debug;

/// Strong newtypes preventing primitive obsession
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SequenceNumber(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoHTick(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hash512(pub [u8; 64]);

impl Hash512 {
    pub fn to_prefixed_hex(&self) -> String {
        format!("sha3-512:{}", hex::encode(self.0))
    }
}

/// Cryptographic witness asserting execution mandate and proof anchor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalWitness {
    pub principal_pubkey: [u8; 32],
    pub signature: [u8; 64],
    pub mandate_id: Option<String>,
    pub proof_anchor: Option<Hash512>,
}

/// Raw entry read from the WAL
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalEntry {
    pub sequence: SequenceNumber,
    pub poh_tick: PoHTick,
    pub previous_hash: Hash512,
    pub entry_hash: Hash512,
    pub schema: String,
    pub payload: bytes::Bytes,
    pub witness: CausalWitness,
}

#[derive(Debug, Clone)]
pub struct WalAppendInput {
    pub schema: String,
    pub payload: bytes::Bytes,
    pub witness: CausalWitness,
}

#[derive(Debug, Clone)]
pub struct WalReceipt {
    pub sequence: SequenceNumber,
    pub entry_hash: Hash512,
    pub poh_tick: PoHTick,
}

#[derive(Debug, Clone)]
pub struct IntegrityReport {
    pub verified_up_to_seq: SequenceNumber,
    pub root_hash: Hash512,
    pub total_entries: u64,
    pub is_valid: bool,
}

/// Zero-leak domain storage error enum
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("WAL sequence violation: expected {expected:?}, got {actual:?}")]
    SequenceMismatch { expected: SequenceNumber, actual: SequenceNumber },
    
    #[error("Cryptographic hash chain broken at sequence {sequence:?}")]
    HashChainCorrupt { sequence: SequenceNumber, expected_prev: Hash512, actual_prev: Hash512 },
    
    #[error("Causal witness verification failed: {0}")]
    UnadmittedCausalWitness(String),
    
    #[error("Storage I/O failure: {0}")]
    Io(String),
    
    #[error("Serialization / Codec error: {0}")]
    Codec(String),
    
    #[error("Entity not found: {schema} id={id}")]
    NotFound { schema: String, id: String },
    
    #[error("Projection failure at sequence {sequence:?} for schema {schema}: {cause}")]
    ProjectionFailed { sequence: SequenceNumber, schema: String, cause: String },
    
    #[error("Storage lock contention: {0}")]
    LockContention(String),
    
    #[error("Substrate disk pressure limit reached (>90% full)")]
    SubstrateDiskPressure,
}

/// Append-only immutable log port
pub trait WalJournalPort: Send + Sync {
    /// Atomic batch append with single group-commit durability boundary
    fn append_batch(&self, events: &[WalAppendInput]) -> Result<Vec<WalReceipt>, StorageError>;
    
    /// Streaming iterator yielding entries without unbounded heap allocation
    fn scan_stream(
        &self,
        from_seq: SequenceNumber,
        to_seq: Option<SequenceNumber>,
    ) -> Result<Box<dyn Iterator<Item = Result<WalEntry, StorageError>> + '_>, StorageError>;

    fn latest_sequence(&self) -> Result<SequenceNumber, StorageError>;
    fn verify_integrity(&self) -> Result<IntegrityReport, StorageError>;
}

/// SMR Transaction boundary for projecting SQLite views
pub trait StateTransaction {
    fn execute_sql(&mut self, sql: &str, params: &[&(dyn rusqlite::ToSql + Sync)]) -> Result<usize, StorageError>;
}

/// Bounded-context projection reducer
pub trait DomainProjector: Send + Sync {
    fn supported_schemas(&self) -> &[&str];
    fn apply(&self, entry: &WalEntry, tx: &mut dyn StateTransaction) -> Result<(), StorageError>;
}

/// SMR Replay & Projection Coordinator
pub trait ProjectionCoordinatorPort: Send + Sync {
    fn register_projector(&mut self, projector: Box<dyn DomainProjector>) -> Result<(), StorageError>;
    fn project_entry(&self, entry: &WalEntry) -> Result<(), StorageError>;
    fn replay_all(&self) -> Result<u64, StorageError>;
    fn last_projected_sequence(&self) -> Result<SequenceNumber, StorageError>;
}

/// Embedded Vector Storage Port for Cognitive Crystals & Semantic Search
pub trait VectorStorePort: Send + Sync {
    fn insert_vector(&self, id: &str, vector: &[f32]) -> Result<(), StorageError>;
    fn search_nearest(&self, query: &[f32], k: usize, threshold: f32) -> Result<Vec<(String, f32)>, StorageError>;
    fn delete_vector(&self, id: &str) -> Result<(), StorageError>;
}

/// Generic Read View Port (Zero domain coupling)
pub trait ReadViewPort: Send + Sync {
    fn query_raw(&self, sql: &str, params: &[String]) -> Result<Vec<Vec<u8>>, StorageError>;
    fn query_json(&self, sql: &str, params: &[String]) -> Result<Vec<String>, StorageError>;
}

/// Master Unified Storage Port
pub trait StoragePort: Send + Sync {
    fn wal(&self) -> &dyn WalJournalPort;
    fn projections(&self) -> &dyn ProjectionCoordinatorPort;
    fn vectors(&self) -> &dyn VectorStorePort;
    fn read_view(&self) -> &dyn ReadViewPort;
    fn checkpoint(&self) -> Result<StorageCheckpoint, StorageError>;
}
```

---

## 6. Extensibility for Unformalized Matter & Vector Embeddings

### 6.1 Enriched Opaque Envelope Contract (`cynic.opaque_envelope.v2`)

Unformalized matter is stored with structured tags to allow indexing without full schema formalization:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpaqueEnvelope {
    pub envelope_id: String,
    pub schema: String,
    pub source_principal: String,
    pub recorded_at_unix_ms: u64,
    pub indexed_tags: std::collections::BTreeMap<String, String>,
    pub parent_envelope_id: Option<String>,
    pub payload_json: String,
    pub payload_hash: String,
    pub residual_unknowns: Vec<String>,
}
```

**Materialized Projection Schema**:
```sql
CREATE TABLE IF NOT EXISTS opaque_envelopes (
    envelope_id TEXT PRIMARY KEY,
    schema TEXT NOT NULL,
    source_principal TEXT NOT NULL,
    recorded_at_unix_ms INTEGER NOT NULL,
    parent_envelope_id TEXT,
    payload_json TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    residual_unknowns_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_opaque_lookup 
ON opaque_envelopes(schema, source_principal, recorded_at_unix_ms);

CREATE TABLE IF NOT EXISTS opaque_envelope_tags (
    envelope_id TEXT NOT NULL,
    tag_key TEXT NOT NULL,
    tag_value TEXT NOT NULL,
    PRIMARY KEY (envelope_id, tag_key),
    FOREIGN KEY (envelope_id) REFERENCES opaque_envelopes(envelope_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_opaque_tag_search 
ON opaque_envelope_tags(tag_key, tag_value);
```

### 6.2 Dual-Plane Embedded Vector Architecture (Crystals)

Without external vector daemons, embeddings (1024-dim cosine) operate via:
1. **SQLite BLOB Storage**: Binary vector representation (`1024 * 4 = 4096 bytes` IEEE-754 float array) stored in the `crystals` table.
2. **In-Memory SIMD Cosine Scanner**: AVX2/NEON scanner capable of scanning 10,000 vectors in < 2 ms.
3. **Reconstructibility**: The vector index is entirely derived. If wiped or corrupted, it is rebuilt from SQLite or the WAL on boot, satisfying **INV-01**.

---

## 7. Concurrency, I/O & Systems Invariants

1. **Filesystem Hardening**:
   - Explicit umask `0077` on creation.
   - Storage directory permissions: strictly `0700` (`rwx------`).
   - Database and WAL file permissions: strictly `0600` (`rw-------`).
   - Root execution refusal: Fail closed if running as UID 0 unless explicitly overridden.
   - Advisory kernel lock: `flock(LOCK_EX | LOCK_NB)` on `$CYNIC_DATA_DIR/storage/wal.lock` to prevent multi-process split-brain.
2. **Durability & Flush Discipline**:
   - `fdatasync()` used instead of `fsync()` to eliminate ext4/XFS journal metadata overhead.
   - Group commit actor: appends register via oneshot channels; actor flushes up to $N$ entries or every 5 ms.
3. **SQLite Concurrency & Tuning**:
   - Dedicated single-writer thread on an MPSC channel for all state projections.
   - Multi-reader pool strictly opened with `SQLITE_OPEN_READ_ONLY`.
   - Mandatory Pragmas:
     ```sql
     PRAGMA journal_mode = WAL;
     PRAGMA synchronous = NORMAL;
     PRAGMA busy_timeout = 5000;
     PRAGMA foreign_keys = ON;
     PRAGMA cache_size = -64000;
     PRAGMA mmap_size = 268435456;
     PRAGMA temp_store = MEMORY;
     ```
   - Projection Watermark Table:
     ```sql
     CREATE TABLE IF NOT EXISTS _cynic_storage_watermark (
         id INTEGER PRIMARY KEY CHECK (id = 1),
         last_applied_seq INTEGER NOT NULL,
         last_entry_hash TEXT NOT NULL,
         updated_at_unix_ms INTEGER NOT NULL
     );
     ```

---

## 8. Legacy Migration Pipeline (SurrealDB $\to$ SMR)

Per **MIG-01**, all historical state must be ingested through the WAL:

1. **Extraction**: `cynic-storage migrate` reads legacy SurrealDB and SQLite dumps.
2. **Genesis Event Envelope**:
   ```rust
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct LegacyImportPayload {
       pub legacy_source: String, // "surrealdb" | "sqlite_agent_sessions"
       pub legacy_table: String,
       pub legacy_id: String,
       pub legacy_created_at_unix_ms: u64,
       pub canonical_schema: String,
       pub record_json: String,
   }
   ```
3. **Sequential Ingestion**: Appended to WAL sequence `0..N` with valid genesis signatures and `fdatasync`.
4. **SMR Replay**: Projection coordinator populates SQLite tables.
5. **Dual-Read Verification Gate**: Assert row counts and checksums match identically before SurrealDB is decommissioned:
   $$\sum \mathcal{H}(\text{payload}_{\text{legacy}}) == \sum \mathcal{H}(\text{payload}_{\text{reconstructed}})$$

---

## 9. Formal Admission Proofs (Verification Suite)

Prior to merging `crates/cynic-storage` into main, the implementation must provide deterministic proof passing on the remote builder (`10.10.10.119`):

1. **`test_wal_deterministic_reconstruction`**: Append 10,000 heterogeneous events; delete `state.db`; replay from WAL; assert state snapshot hash matches pre-deletion state.
2. **`test_wal_torn_tail_recovery`**: Append 100 events; simulate crash by writing 17 partial bytes at EOF; run recovery; assert 100 events intact and file cleanly truncated.
3. **`test_wal_mid_log_corruption_fails_closed`**: Corrupt byte at sequence 42; assert initialization fails with `HashChainCorrupt` and refuses reads/writes.
4. **`test_wal_multi_process_flock_rejection`**: Spawn two processes targeting the same storage dir; assert second process immediately exits with `LockContention`.
5. **`test_anti_fabrication_unknowns_preserved`**: Ingest an unformalized envelope with missing keys; verify projected view returns `None` and records field in `residual_unknowns` with zero defaulting.
