# SPEC-CYNIC-STORAGE-V1: Sovereign Clean-Room Storage Architecture

- **Status**: Ratified & Sealed (2026-09-11)
- **Author**: Titouan Corneloup (zeyxx) / Hermes Agent
- **Target Crate**: `crates/cynic-storage`
- **Related Requirements**: Foundation §11 (Digest Law: `sha3-512:`), §481 (`one_boundary_one_reason`), §489 (anti-centralization), Framework §462 (`substrate_behind_ports`), §476 (`proof_producers_are_low_layer`)
- **Supersedes**: `cynic-kernel/src/storage/` (SurrealDB monolithic network coupling, SPOF F2)

---

## 1. Executive Summary & Problem Statement

The CYNIC codebase historically dispersed storage across 83 files and 15,570 LOC:
1. **The SurrealDB Network SPOF (`cynic-kernel`)**: Coupling via external HTTP/WebSocket on port 8000 mixed mutable runtime state with immutable evidence, requiring 1,800 LOC of reconnection churn and violating single-binary node sovereignty.
2. **Inverted Dependency Direction**: `cynic-kernel` directly implemented traits defined inside `cynic-node` (`AgentWorkspaceClaimStorePort`), violating hexagonal layering.
3. **Storage Fragmentation**: Multiple storage paradigms operated in silos with no unified interface: remote SurrealDB, embedded SQLite, Filesystem CAS, and ad-hoc JSON files.

`cynic-storage` provides a unified, sovereign, zero-external-daemon persistence layer based on **Event-Sourced State Machine Replication (SMR)**. It couples an immutable, cryptographically chained Binary WAL (the single source of truth) with **Embedded In-Process SurrealDB (SurrealKV)** for rich graph, vector, and multi-model query capabilities, and a lightweight embedded SQLite adapter for constrained nodes.

---

## 2. Core Architectural Laws & Invariants

1. **INV-01 (View Derives From Log, Never Inverse)**: The append-only, hash-chained Write-Ahead Log (WAL) is the sole source of runtime truth. Embedded database projections (SurrealKV / SQLite) are disposable, reconstructible caches. Wiping database files must never cause data loss; replaying the WAL reconstructs the exact state.
2. **INV-02 (Hexagonal Purity & Zero Domain Bleed)**: Core storage domain contracts (`ports`) know nothing of underlying database drivers or network sockets. Storage ports do NOT declare hardcoded domain getters (e.g. `get_claim`, `get_account`); domain contexts own their schemas and project via the SMR reducer interface.
3. **INV-03 (Zero External Daemon SPOF)**: Storage operates strictly embedded within the CYNIC process space. No external database process (Postgres, remote SurrealDB, Redis, Qdrant) is required for steady-state sovereign operations. SurrealDB runs in-process via `Surreal::new::<SurrealKv>`.
4. **INV-04 (Cryptographic Causal Chain & Proof-of-History)**: Every mutation record is cryptographically bound to its predecessor via length-prefixed, domain-separated SHA3-512 hashing:
   $$\text{entry\_hash} = \text{SHA3-512}(\text{"CYNIC\_WAL\_ENTRY\_V1\textbackslash 0"} \parallel \text{seq} \parallel \text{prev\_hash} \parallel \text{poh\_tick} \parallel \text{schema\_len} \parallel \text{schema} \parallel \text{payload\_len} \parallel \text{payload\_hash} \parallel \text{witness})$$
   All digests must strictly carry the canonical `sha3-512:` prefix per Foundation §11. Wall-clock time is an observed payload attribute, never the cryptographic backbone of causal time.
5. **INV-05 (Handling Unformalized Matter)**: Entities not yet formalised into strict relational/graph schemas are preserved as content-addressed, schema-tagged opaque envelopes (`cynic.opaque_envelope.v2`) carrying structured tags for indexed queries.
6. **INV-06 (Zero Secrets in the WAL)**: Raw credentials, private keys, API secrets, and unencrypted session tokens are strictly prohibited from being written to the WAL. Secrets must reside in principal-owned encrypted vaults; the WAL stores only cryptographic commitments (`sha3-512:secret_digest`) and URI handles (`vault://...`).
7. **INV-07 (Anti-Fabrication on Historical Matter)**: When historical records are projected into newly formalized schemas, missing attributes MUST be stored as `NULL` / `Option::None`. The projection engine is strictly prohibited from using SQL/schema `DEFAULT` values or type-level defaults (`Default::default()`). Any unobserved field must propagate to the snapshot's `residual_unknowns` collection.
8. **MIG-01 (Genesis WAL Ingestion Law)**: Direct insertion of legacy data into storage engines without WAL event generation is an architectural violation of `INV-01`. All legacy state from legacy SurrealDB and SQLite databases must enter CYNIC storage strictly as **Genesis WAL Events** (`WalEvent::LegacyImport`).

---

## 3. The Dual-Engine Physical Topology

Storage separates durable sequential append from rich multi-dimensional queries:

```text
                                 [ CYNIC RUNTIME PROCESS ]
                                             │
                           ┌─────────────────┴─────────────────┐
                           │           cynic-storage           │
                           └─────────────────┬─────────────────┘
                                             │
                 ┌───────────────────────────┴───────────────────────────┐
                 ▼                                                       ▼
     [ ENGINE 1: BINARY WAL ]                              [ ENGINE 2: EMBEDDED SURREALKV ]
     • Native Rust binary log                              • surrealdb = { version = "2", features = ["kv-surrealkv"] }
     • sha3-512 + Ed25519 + fdatasync                      • In-process, zero daemon, zero network port
     • 64 MB chunked segments                              • Native Graph (->), HNSW Dynamic Vector Index (D-dim)
     • Source of Truth / SMR Log                           • Materialized View / GraphRAG Intelligence
```

---

## 4. The 5 Operational Data Planes

Storage is organized into five functional data planes serving the cybernetic and economic organism:

### Plane 1: Cognitive Crystals (Wisdom 4D & GraphRAG)
- **Role**: Unit of distilled operational knowledge and empirical learning.
- **Attributes**: Content string, dynamic $D$-dimensional embedding vector (supporting 384, 768, 1024, 1536+ and Matryoshka sub-vector truncation), Welford M2 certainty score, causal edge relationships (`->supports->`, `->invalidates->`).
- **Operation**:
  - Semantic retrieval via native in-process HNSW: `SELECT * FROM crystal WHERE embedding <|40, COSINE|> $query_vec AND certainty > 0.8`.
  - Shattering mechanism: When read-back observation proves a crystal invalid, its certainty decays; sub-threshold crystals are shattered (quarantined from active memory).

### Plane 2: Causal Governance Graph (Mandates, Claims & Emergence)
- **Role**: Strict enforcement of downward causal authority and upward proof verification.
- **Graph Topology**:
  `Constituent (HumanAdmin)` $\to$ `MandateAccount` $\to$ `ClaimScope` $\to$ `AgentAccount` $\to$ `RuntimeProof`.
- **Operation**: Enables instant graph traversal queries: verifying whether an agent holds an active, unrevoked mandate before mutation, and tracing any historical code change back to constituent intent.

### Plane 3: Epistemic Verdicts & Phi Consensus
- **Role**: Epistemic scoring of truth claims, agent outputs, and PR submissions.
- **Attributes**: Multi-axial evaluation vectors (`fidelity`, `phi`, `verify`, `culture`, `burn`, `sovereignty`), anomaly indicators, voter counts, and cryptographic integrity hashes.
- **Settlement Export**: Verdicts achieving high epistemic consensus are batched for public commitment to Solana via Helius.

### Plane 4: Metabolic Ledger & Escrow
- **Role**: Accounting for compute energy, tokens, wall-time, and workflow friction.
- **Contracts**: `MetabolicInvoice`, `ResourceAccount`, `EscrowReservation`.
- **Operation**:
  - Escrow on Result: Agents reserve metabolic quota; quota is refunded and reputation (E-Score) is rewarded upon valid `RuntimeProof` read-back.
  - Failure Burn: Failed tasks or uncontained loops burn energy quota as system friction.
  - Workflow Tax: Inter-agent delegation and chatter carry incremental metabolic fees, discouraging bureaucratic sprawl.

### Plane 5: Opaque Envelopes (The Sandbox for Unformalized Matter)
- **Role**: Lossless ingestion of nascent concepts, experimental schemas, or evolving agent communications.
- **Structure**:
  ```rust
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
- **Operation**: Retained losslessly in WAL; indexed in SurrealKV/SQLite by tags (`key=value`), enabling queries over unformalized matter prior to schema promotion.

---

## 5. Binary Frame Format & Segmented Log Architecture

### 5.1 Binary Wire Format (Per Frame)

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

### 5.2 Deterministic Crash Recovery Protocol (Torn Tail Truncation)

On initialization (`open`):
1. **Sequential Scan**: Validate `HeaderCRC32C` and `EntryHashDigest` frame by frame.
2. **Mid-Log Corruption (FATAL)**: If a checksum or cryptographic chain fails at byte offset $O$ where $O + \text{frame\_len} < \text{file\_len}$, the log has suffered tampering or hardware bit-rot. The engine **FAILS CLOSED** with `StorageError::CorruptedLedgerAtSequence(seq)`. No auto-truncation is allowed.
3. **Torn Tail at EOF (RECOVERABLE)**: If an incomplete frame or CRC mismatch occurs *strictly at the physical end of the file*:
   - Verify that all prior entries up to sequence $N$ passed cryptographic validation.
   - Truncate the file back to the end of sequence $N$ (`file.set_len(valid_tail_offset)`).
   - Issue an explicit `fdatasync()` on the file and parent directory.
   - Emit an audited event `cynic.storage.torn_write_recovered`.

---

## 6. Ports & Interfaces (Pure Rust Contracts)

```rust
use std::fmt::Debug;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalWitness {
    pub principal_pubkey: [u8; 32],
    pub signature: [u8; 64],
    pub mandate_id: Option<String>,
    pub proof_anchor: Option<Hash512>,
}

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
    fn append_batch(&self, events: &[WalAppendInput]) -> Result<Vec<WalReceipt>, StorageError>;
    fn scan_stream(
        &self,
        from_seq: SequenceNumber,
        to_seq: Option<SequenceNumber>,
    ) -> Result<Box<dyn Iterator<Item = Result<WalEntry, StorageError>> + '_>, StorageError>;
    fn latest_sequence(&self) -> Result<SequenceNumber, StorageError>;
    fn verify_integrity(&self) -> Result<IntegrityReport, StorageError>;
}

/// SMR Transaction boundary for projecting views
pub trait StateTransaction {
    fn execute_statement(&mut self, statement: &str) -> Result<(), StorageError>;
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

/// Master Unified Storage Port
pub trait StoragePort: Send + Sync {
    fn wal(&self) -> &dyn WalJournalPort;
    fn projections(&self) -> &dyn ProjectionCoordinatorPort;
    fn checkpoint(&self) -> Result<StorageCheckpoint, StorageError>;
}

#[derive(Debug, Clone)]
pub struct StorageCheckpoint {
    pub sequence: SequenceNumber,
    pub root_hash: Hash512,
    pub created_at_unix_ms: u64,
}
```

---

## 7. Concurrency, I/O & Systems Invariants

1. **Filesystem Hardening**:
   - Process umask `0077` on directory/file creation.
   - Storage directory permissions: strictly `0700` (`rwx------`).
   - Database and WAL file permissions: strictly `0600` (`rw-------`).
   - Non-root execution enforcement: Fail closed if running as UID 0 unless explicitly overridden.
   - Advisory kernel lock: `flock(LOCK_EX | LOCK_NB)` on `$CYNIC_DATA_DIR/storage/wal.lock` preventing multi-process split-brain.
2. **Durability & Flush Discipline**:
   - `fdatasync()` used instead of `fsync()` to eliminate ext4/XFS journal metadata overhead.
   - Group commit actor: incoming appends register via oneshot channels; actor flushes up to $N$ entries or every 5 ms.

---

## 8. Legacy Migration Pipeline (SurrealDB $\to$ SMR)

Per **MIG-01**, all historical state must be ingested through the WAL:
1. **Extraction**: `cynic-storage migrate` reads legacy SurrealDB export and SQLite dumps.
2. **Genesis Event Envelope**: Each table row is transformed into a `LegacyImportPayload`.
3. **Sequential Ingestion**: Appended to WAL sequence `0..N` with valid genesis signatures and `fdatasync`.
4. **SMR Replay**: Projection coordinator populates embedded SurrealKV / SQLite tables.
5. **Dual-Read Verification Gate**: Assert row counts and checksums match identically before legacy instances are decommissioned:
   $$\sum \mathcal{H}(\text{payload}_{\text{legacy}}) == \sum \mathcal{H}(\text{payload}_{\text{reconstructed}})$$

---

## 9. Formal Admission Proofs (Verification Suite)

Prior to merging `crates/cynic-storage` into main, the implementation must provide deterministic proof passing on the remote builder (`10.10.10.119`):

1. **`test_wal_deterministic_reconstruction`**: Append 10,000 heterogeneous events; delete projection databases; replay from WAL; assert state snapshot hash matches pre-deletion state.
2. **`test_wal_torn_tail_recovery`**: Append 100 events; simulate crash by writing 17 partial bytes at EOF; run recovery; assert 100 events intact and file cleanly truncated.
3. **`test_wal_mid_log_corruption_fails_closed`**: Corrupt byte at sequence 42; assert initialization fails with `HashChainCorrupt` and refuses reads/writes.
4. **`test_wal_multi_process_flock_rejection`**: Spawn two processes targeting the same storage dir; assert second process immediately exits with `LockContention`.
5. **`test_anti_fabrication_unknowns_preserved`**: Ingest an unformalized envelope with missing keys; verify projected view returns `None` and records field in `residual_unknowns` with zero defaulting.

---

## 10. Analytical Data Warehouse Architecture (Parquet + Embedded DuckDB)

The operational OLTP node runtime (WAL + SurrealKV) is reserved for real-time state machine transitions and low-latency agent coordination. Analytical workloads (cross-node forensics, retroactive telemetry audits, multi-month metabolic trends) are offloaded to an **embedded columnar OLAP engine**:

1. **Cold Segment Columnarization**:
   - Sealed WAL segments older than the active retention window (e.g. 7 days) and verified by checkpoint are transformed into Apache Parquet columnar files:
     `$CYNIC_DATA_DIR/storage/warehouse/year=2026/month=09/events_seg_000042.parquet`
   - Parquet encoding leverages Dictionary Encoding, Run-Length Encoding (RLE), Bit-Packing, and Zstandard (zstd) compression, achieving 15–30x lossless compaction over raw JSON.
2. **In-Process DuckDB Query Plane**:
   - Zero external analytical cluster (no ClickHouse daemon, no Snowflake, zero remote dependency).
   - Embedded `duckdb` vectorizes SQL scans directly over local/remote Parquet files via SIMD execution kernels:
     ```sql
     SELECT
         discrete_axes['spatial.crate'] AS crate_name,
         AVG(continuous_axes['metabolic.joules']) AS avg_joules,
         SUM(continuous_axes['metabolic.tokens']) AS total_tokens
     FROM read_parquet('/var/lib/cynic/storage/warehouse/*/*.parquet')
     WHERE continuous_axes['epistemic.certainty'] > 0.8
     GROUP BY crate_name;
     ```

---

## 11. The 6 Degrees of Information Density & Mathematical Compression Limits

Storage in CYNIC is governed by **Rate-Distortion Theory** and statistical physics (coarse-graining / renormalization group). It rejects arbitrary classification in favor of six rigorous degrees of density:

```text
DEGREE                  NATURE & ENUM             SUBSTRATE & ENGINE              DISTORTION TOLERATED
──────────────────────────────────────────────────────────────────────────────────────────────────────
Degree 0: Raw Signal    Continuous Flux           RAM Ring-Buffers                High (Volatile,
                        (stdout, tokens, pipes)   (Ephemera, zero-I/O)            transient dissipation)
                                │
Degree 1: Causal Atom   Discrete Event Link       Append-Only Binary WAL          D = 0 (Strict Shannon
                        (WalEntry, Witness, PoH)  (sha3-512, Ed25519, fdatasync)  lossless conservation)
                                │
Degree 2: State View    O(1) Operational State    SurrealKV / SQLite              D = 0 (Deterministic
                        (Active claims, balances) (Disposable SMR caches)         materialized projection)
                                │
Degree 3: Valence Graph Relational Entanglement   SurrealKV Graph Engine          D = 0 (Structural
                        (Mandate->Claim->Proof)   (Edges ->supports->, ->refutes) topological links)
                                │
Degree 4: Wisdom Crystal Semantic Distillation    In-Memory HNSW / SurrealKV      D > 0 (Controlled
                        (State vector |ψ⟩, M2)    (int8 / Matryoshka quantized)   semantic lossy reduction)
                                │
Degree 5: Condensate    Historical Forensics      Columnar Parquet + DuckDB       D = 0 on dimensions,
                        (Fleet ledger warehouse)  (Zstd dictionary, bit-packed)   lossless analytics
```

### 11.1 Mathematical Principles of Transition
1. **Shannon Lossless Bound ($D = 0$)**: Raw execution logs contain $H(X) \approx 1.2\text{--}2.5 \text{ bits/byte}$. Lossless compression alone achieves at most $3\times\text{--}6\times$ compaction. Therefore, lossless compression is strictly reserved for Degree 1 (WAL) and Degree 5 (Parquet).
2. **Johnson-Lindenstrauss & Vector Quantization (Degree 4)**: For $N$ semantic lessons, projecting into a $D$-dimensional Hilbert space preserves relative Euclidean distances within $(1 \pm \epsilon)$. Quantizing float32 embeddings to `int8` yields a **$4\times$ memory reduction with $<1.5\%$ top-k recall loss**. Further 1-bit Matryoshka binary quantization yields a **$32\times$ reduction (128 bytes per vector)** for ultra-high-speed candidate pre-filtering.

---

## 12. The Metadata Tensor & Unbounded Fiber Bundle Model

No dimension count is fixed. Metadata is modeled mathematically as a **Fiber Bundle** $E \xrightarrow{\pi} B$:
- **The Base $B$**: Minimal invariant consensus manifold (SequenceNumber, ContentDigest, CausalSignature).
- **The Fiber $F_x$**: Extensible, dynamic $K$-dimensional metadata tensor attached to event $x$.

```rust
use std::collections::BTreeMap;

/// Extensible, unbounded metadata tensor attached to any storage envelope.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MetadataTensor {
    /// Invariant base manifold
    pub schema_uri: String,
    pub causal_anchor: [u8; 64],

    /// Continuous scalar degrees of freedom (energy, probability, duration, market score)
    /// e.g. {"metabolic.joules": 14.2, "epistemic.certainty": 0.94, "market.price": 0.38}
    pub continuous_axes: BTreeMap<String, f64>,

    /// Discrete structural degrees of freedom (spatial scope, agent phase, tenant, role)
    /// e.g. {"spatial.crate": "cynic-storage", "agent.phase": "crystallization"}
    pub discrete_axes: BTreeMap<String, String>,

    /// Explicit residual unknowns for unobserved dimensions
    pub residual_unknowns: Vec<String>,
}
```

### Invariant: Zero Schema Migration Penalty
1. **In WAL (L0)**: Serialized as a zero-copy binary envelope without loss.
2. **In SurrealKV (L1)**: Ingested dynamically via schema-flexible document/graph structures without `ALTER TABLE`.
3. **In Parquet (L2)**: Stored as typed `MAP<VARCHAR, DOUBLE>` and `MAP<VARCHAR, VARCHAR>` columns, allowing new dimensions to be introduced across the fleet with zero backfill cost.

---

## 13. The 4-Step Distillation Protocol (Degree 1 $\to$ Degree 4)

A Crystal of Wisdom is never generated by unconstrained LLM narration. It is forged through a deterministic thermodynamic filter:

```text
[Raw Observation Stream (Degree 1)]
               │
               ▼  Step 1: Shannon Information Filter (Surprise / Non-Triviality)
[Emergence Candidate]
               │
               ▼  Step 2: Welford M2 Statistical Variance & Certainty Convergence
[Hypothetical Invariant]
               │
               ▼  Step 3: Falsification by Inverse Read-Back Proof
[Empirical Law]
               │
               ▼  Step 4: int8 Vector Quantization & Graph Valence Binding
[Sealed Crystal (Degree 4)]
```

1. **Step 1: Shannon Surprise Filter**:
   $$I(E) = -\log_2 P(E)$$
   Events with $P(E) \approx 1$ (routine zero-exit builds, expected test passes) convey $I \to 0$ and are bypassed. Events with $P(E) \ll 1$ (rare toolchain crashes, unexpected kernel faults) trigger candidate emergence.
2. **Step 2: Welford M2 Convergence**:
   Computes rolling mean and sample variance $s_n^2 = \frac{M_{2, n}}{n}$ online:
   $$M_{2, n} = M_{2, n-1} + (x_n - \bar{x}_{n-1})(x_n - \bar{x}_n)$$
   Certainty $C = 1 - \frac{s_n}{\bar{x}_n}$. A candidate crystallizes only when sample count $n \ge 3$ and $C \ge 0.80$.
3. **Step 3: Inverse Read-Back Falsification**:
   The agent must prove causal validity:
   - Initial state: Command $A$ fails with error $X$.
   - Mutation applied: Candidate rule $B$ enacted.
   - Read-back proof: Identical command $A$ yields exit code 0.
   Failure to demonstrate causal inversion aborts crystallization.
4. **Step 4: int8 Vector Quantization & Valence Bonding**:
   The distilled invariant ($< 200$ chars) is vectorized into a dynamic $D$-dimensional embedding, quantized to `int8`, and bound to domain nodes via valence edges (`->supports->`, `->refutes->`).
5. **Landauer Shattering Protocol**:
   When future read-back observation refutes an existing crystal ($C \to 0$), Landauer dissipation is recorded: the crystal is shattered (quarantined from active graph traversals) with its refutation proof permanently anchored in the WAL.
