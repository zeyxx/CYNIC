# SPEC-CYNIC-STORAGE-V1: Hardened Sovereign Clean-Room Storage Architecture

- **Status**: Ratified, Hardened & Sealed (2026-09-11)
- **Author**: Titouan Corneloup (zeyxx) / Hermes Agent
- **Target Crate**: `crates/cynic-storage`
- **Related Requirements**: Foundation §11 (Digest Law: `sha3-512:`), §481 (`one_boundary_one_reason`), §489 (anti-centralization), Framework §462 (`substrate_behind_ports`), §476 (`proof_producers_are_low_layer`)
- **Supersedes**: `cynic-kernel/src/storage/` (SurrealDB network coupling, SPOF F2)

---

## 1. Executive Summary & Problem Statement

The CYNIC codebase historically dispersed storage across 83 files and 15,570 LOC:
1. **The Remote Daemon SPOF**: Coupling via external HTTP/WebSocket on port 8000 mixed mutable state with immutable evidence, requiring 1,800 LOC of reconnection boilerplate and violating single-binary node sovereignty.
2. **Inverted Dependency Direction**: The kernel directly implemented traits defined inside `cynic-node` (`AgentWorkspaceClaimStorePort`), violating hexagonal layering.
3. **Multi-Engine Bloat Hazard**: Attempting to embed SurrealKV, DuckDB, and SQLite simultaneously within a single binary introduces +100 MB of binary bloat, 18 GB compilation memory spikes, and severe CPU/allocator contention.

`cynic-storage` implements a hardened, sovereign, zero-external-daemon persistence layer based on **Event-Sourced State Machine Replication (SMR)**. It couples an immutable, cryptographically chained Binary WAL (single source of truth) with **Embedded SQLite** for fast O(1) state queries, and an **In-Memory Contiguous int8 SIMD Vector Buffer** for semantic memory crystals.

---

## 2. Hardened Architectural Laws & Invariants

1. **INV-01 (View Derives From Log, Never Inverse)**: The append-only, hash-chained Write-Ahead Log (WAL) is the sole source of runtime truth. Embedded database projections (SQLite) are disposable, reconstructible caches. Wiping SQLite must never cause data loss; replaying the WAL reconstructs the exact state.
2. **INV-02 (Hexagonal Purity & Zero Domain Bleed)**: Core storage domain contracts (`ports`) know nothing of underlying database drivers or network sockets. Storage ports do NOT declare hardcoded domain getters (e.g. `get_claim`, `get_account`); domain contexts own their schemas and project via the SMR reducer interface.
3. **INV-03 (Single-Binary Embedded Sovereignty)**: Storage operates strictly embedded within the CYNIC process space. No external database process (Postgres, remote SurrealDB, Redis, Qdrant) is required for steady-state sovereign operations.
4. **INV-04 (Cryptographic Causal Chain & 2-Stage Commit Binding)**: Every mutation record is cryptographically bound to its predecessor via length-prefixed, domain-separated SHA3-512 hashing. To prevent circular signature dependencies, hashing follows a strict 2-stage binding:
   - Stage 1: $\text{commit\_hash} = \text{SHA3-512}(\text{"CYNIC\_WAL\_COMMIT\_V1\textbackslash 0"} \parallel \text{seq} \parallel \text{prev\_hash} \parallel \text{poh\_tick} \parallel \text{schema} \parallel \text{payload\_hash} \parallel \text{mandate\_id} \parallel \text{proof\_anchor})$
   - Stage 2: Principal signs $\text{commit\_hash}$ with Ed25519.
   - Stage 3: $\text{entry\_hash} = \text{SHA3-512}(\text{"CYNIC\_WAL\_ENTRY\_V1\textbackslash 0"} \parallel \text{commit\_hash} \parallel \text{principal\_pubkey} \parallel \text{signature})$
   All digests must strictly carry the canonical `sha3-512:` prefix per Foundation §11.
5. **INV-05 (Handling Unformalized Matter)**: Entities not yet formalised into strict relational schemas are preserved as content-addressed, schema-tagged opaque envelopes (`cynic.opaque_envelope.v2`) carrying structured tags for indexed queries.
6. **INV-06 (Zero Secrets in the WAL)**: Raw credentials, private keys, API secrets, and unencrypted session tokens are strictly prohibited from being written to the WAL. Secrets must reside in principal-owned encrypted vaults; the WAL stores only cryptographic commitments (`sha3-512:secret_digest`) and URI handles (`vault://...`).
7. **INV-07 (Anti-Fabrication on Historical Matter)**: When historical records are projected into newly formalized schemas, missing attributes MUST be stored as `NULL` / `Option::None`. The projection engine is strictly prohibited from using SQL `DEFAULT` values or type-level defaults (`Default::default()`). Any unobserved field must propagate to the snapshot's `residual_unknowns` collection.
8. **INV-08 (Amnesia & Shattering Protection)**: A cognitive crystal may never be shattered based solely on telemetry variance. Shattering requires operator confirmation or multi-source consensus, preventing adversarial epistemic amnesia attacks. Certainty scores are clamped strictly to $[0.0, 1.0]$.
9. **INV-09 (Market Data Non-Authority)**: Market odds and futarchic prices stored in telemetry or metadata are descriptive projections of external sentiment only. They are strictly prohibited from driving SMR state transitions, mandate admissions, or crystal validity.
10. **MIG-01 (Genesis WAL Ingestion Law)**: Direct insertion of legacy data into SQLite without WAL event generation is an architectural violation of `INV-01`. All legacy state must enter CYNIC storage strictly as **Genesis WAL Events** (`WalEvent::LegacyImport`).

---

## 3. Physical Storage Topology (The Lean Core)

```text
                                 [ CYNIC RUNTIME PROCESS ]
                                             │
                           ┌─────────────────┴─────────────────┐
                           │           cynic-storage           │
                           └─────────────────┬─────────────────┘
                                             │
                 ┌───────────────────────────┼───────────────────────────┐
                 ▼                           ▼                           ▼
     [ ENGINE 1: BINARY WAL ]     [ ENGINE 2: SQLITE SMR ]    [ ENGINE 3: SIMD VECTORS ]
     • Native Rust binary log     • rusqlite in WAL mode      • Contiguous int8 buffer
     • sha3-512 + Ed25519 witness • Single micro-batched MPSC • AVX2/NEON direct dot prod
     • 64 MB chunked segments     • Read-only connection pool • <1.8 ms for <= 50k vectors
     • Source of Truth / SMR Log  • Disposable state views    • Zero index build overhead
```

---

## 4. Binary Frame Format & Frame Defense Bounds

### 4.1 Frame Size Limits (Anti-DOS Bounds)
To prevent OOM allocation attacks:
- `MAX_PAYLOAD_LEN`: 16 MiB (`16 * 1024 * 1024` bytes).
- `MAX_SCHEMA_LEN`: 256 bytes.
- `MAX_MANDATE_ID_LEN`: 128 bytes.
- `MIN_FRAME_LEN`: 224 bytes.

### 4.2 Binary Wire Layout

```text
+-------------------+-------------------+-------------------+-------------------+
| Magic (4 bytes)   | Version (2 bytes) | PayloadLen (4B BE)| HeaderCRC32C (4B) |
| 0x43 0x59 0x4E 0x57 | 0x00 0x01         | (u32 <= 16 MiB)   | Castagnoli CRC32C |
+-------------------+-------------------+-------------------+-------------------+
| Sequence (8B BE)  | PoHTick (8B BE)   | PrevHash (64B raw)| SchemaLen (2B BE) |
+-------------------+-------------------+-------------------+-------------------+
| Schema Bytes (UTF-8, SchemaLen bytes)                                         |
+-------------------------------------------------------------------------------+
| Payload Bytes (PayloadLen bytes)                                              |
+-------------------------------------------------------------------------------+
| Causal Witness:                                                               |
| PrincipalPubkey (32B) | MandateIdLen (2B) | MandateId (UTF-8)                 |
| ProofAnchorPresent (1B) | ProofAnchor (0 or 64B)                              |
| Signature (64B Ed25519 over commit_hash)                                     |
+-------------------------------------------------------------------------------+
| EntryHashDigest (64 bytes raw SHA3-512 over commit_hash + witness)            |
+-------------------------------------------------------------------------------+
```

### 4.3 Recovery State Machine (Torn Tail vs Poison Frame)
On initialization:
1. **Sequential Scan**: Validate `HeaderCRC32C`, `commit_hash`, `Signature`, and `EntryHashDigest`.
2. **Mid-Log Corruption (FATAL)**: If a checksum or cryptographic chain fails at byte offset $O$ where $O + \text{frame\_len} < \text{file\_len}$, the log has suffered tampering or hardware bit-rot. The engine **FAILS CLOSED** with `StorageError::CorruptedLedgerAtSequence(seq)`.
3. **Torn Tail at EOF (RECOVERABLE)**: If an incomplete frame occurs *strictly at the physical end of the file* (less than `MIN_FRAME_LEN` remaining or trailing nulls `0x00`):
   - Verify prior entries up to sequence $N$ passed cryptographic validation.
   - Truncate the file back to valid offset: `file.set_len(valid_tail_offset)`.
   - Issue explicit `fdatasync()`.

---

## 5. Ports & Zero-Allocation Interfaces

```rust
use bytes::Bytes;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct SequenceNumber(pub u64);

impl SequenceNumber {
    pub const GENESIS: Self = Self(0);
    pub fn next(&self) -> Self { Self(self.0 + 1) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PoHTick(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash512(pub [u8; 64]);

impl Hash512 {
    pub const GENESIS: Self = Self([0u8; 64]);
    pub fn to_prefixed_hex(&self) -> String { format!("sha3-512:{}", hex::encode(self.0)) }
    pub fn from_prefixed_hex(s: &str) -> Result<Self, String> {
        let hex_str = s.strip_prefix("sha3-512:").ok_or_else(|| format!("missing sha3-512: prefix in '{s}'"))?;
        let bytes = hex::decode(hex_str).map_err(|e| format!("invalid hex: {e}"))?;
        if bytes.len() != 64 { return Err(format!("expected 64 bytes, got {}", bytes.len())); }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CausalWitness {
    #[serde(with = "hex")]
    pub principal_pubkey: [u8; 32],
    #[serde(with = "hex")]
    pub signature: [u8; 64],
    pub mandate_id: Option<String>,
    pub proof_anchor: Option<Hash512>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WalEntry {
    pub sequence: SequenceNumber,
    pub poh_tick: PoHTick,
    pub previous_hash: Hash512,
    pub commit_hash: Hash512,
    pub entry_hash: Hash512,
    pub schema: String,
    pub payload: Bytes,
    pub witness: CausalWitness,
}

#[derive(Debug, Clone)]
pub struct WalAppendInput {
    pub schema: String,
    pub payload: Bytes,
    pub witness: CausalWitness,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WalReceipt {
    pub sequence: SequenceNumber,
    pub entry_hash: Hash512,
    pub poh_tick: PoHTick,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("WAL sequence violation: expected {expected:?}, got {actual:?}")]
    SequenceMismatch { expected: SequenceNumber, actual: SequenceNumber },
    #[error("Cryptographic hash chain broken at sequence {sequence:?}")]
    HashChainCorrupt { sequence: SequenceNumber, expected_prev: Hash512, actual_prev: Hash512 },
    #[error("Causal witness verification failed: {0}")]
    UnadmittedCausalWitness(String),
    #[error("Frame payload too large: {len} > {max}")]
    PayloadTooLarge { len: usize, max: usize },
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
    fn append_batch(&self, events: &[WalAppendInput]) -> Result<Vec<WalReceipt>, StorageError>;
    fn scan_stream<'a>(
        &'a self,
        from_seq: SequenceNumber,
        to_seq: Option<SequenceNumber>,
    ) -> Result<Box<dyn Iterator<Item = Result<WalEntry, StorageError>> + 'a>, StorageError>;
    fn latest_sequence(&self) -> Result<SequenceNumber, StorageError>;
    fn verify_integrity(&self) -> Result<crate::types::IntegrityReport, StorageError>;
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

/// Contiguous SIMD Vector Store Port
pub trait VectorStorePort: Send + Sync {
    fn insert_vector(&self, id: &str, vector: &[i8]) -> Result<(), StorageError>;
    fn search_nearest(&self, query: &[i8], k: usize, threshold: f32) -> Result<Vec<(String, f32)>, StorageError>;
    fn delete_vector(&self, id: &str) -> Result<(), StorageError>;
}

/// Master Unified Storage Port
pub trait StoragePort: Send + Sync {
    fn wal(&self) -> &dyn WalJournalPort;
    fn projections(&self) -> &dyn ProjectionCoordinatorPort;
    fn vectors(&self) -> &dyn VectorStorePort;
    fn checkpoint(&self) -> Result<crate::types::StorageCheckpoint, StorageError>;
}
```

---

## 6. Zero-Allocation Compact Metadata Tensor

To eliminate 21 heap allocations per event, metadata axes are interned as `u16` tokens with small-vector inlining:

```rust
use smallvec::SmallVec;

pub type AxisToken = u16;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompactMetadataTensor {
    pub schema_token: u16,
    pub causal_anchor: Hash512,

    // Inlined on the stack (zero heap allocations for standard telemetry)
    pub continuous_keys: SmallVec<[AxisToken; 8]>,
    pub continuous_vals: SmallVec<[f64; 8]>,

    pub discrete_keys: SmallVec<[AxisToken; 6]>,
    pub discrete_vals: SmallVec<[compact_str::CompactString; 6]>,

    pub residual_unknowns: SmallVec<[AxisToken; 4]>,
}
```

---

## 7. Systems Concurrency & Backpressure

1. **Dedicated Micro-Batched SQLite Writer**:
   The projection coordinator runs a dedicated thread that pulls batches up to 512 entries from an MPSC channel (`recv_many`) and applies them within a **single atomic transaction**, raising write capacity from 2,800 to >400,000 ops/sec.
2. **Dead-Letter Queue (DLQ) Quarantine**:
   If a projector encounters a malformed payload or unhandled schema error during SMR replay, the entry is quarantined to a `_cynic_dlq` table with full error context. The SMR loop logs an audited error and proceeds, preventing node bricking.
3. **Solana Settlement Separation**:
   WAL events are local process-velocity records. Solana settlement occurs **strictly out-of-band and batched**: an independent worker reads verified checkpoints (`StorageCheckpoint`), computes the Merkle root, and anchors the commitment to Solana according to operator publication policy.
4. **Deterministic Knowledge Distillation**:
   - Stochastic generator (LLM) proposes a candidate invariant ($< 200$ chars) and executable test harness.
   - Deterministic runtime executes the test harness: asserts failure before mutation, asserts exit code 0 after mutation.
   - Only upon deterministic read-back proof is the invariant converted to a memory crystal.
