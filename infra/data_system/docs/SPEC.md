# CYNIC Data System — Technical Specifications & Requirements (Cahier des Charges)

**Document Status**: Specification & Requirements Baseline  
**Scope**: `infra/data_system`  

---

## 1. Architectural Architecture Layers

```
Layer 0: CAPTURE
  └── Input: Active producer state stores (e.g., Hermes SQLite state.db)
  └── Output: Frozen, content-addressed captures (/var/lib/cynic/vault/corpus/captures/*.sqlite)
  └── SLA: Read-only access (`mode=ro`, `immutable=1`), SHA-256 fingerprinting.

Layer 1: CANONICAL
  └── Input: Frozen capture SQLite databases
  └── Output: Normalized sessions, turns, tools, human signals, native metadata
  └── Storage: Canonical SQLite / Rust memory representations
  └── SLA: 100% column conservation, lossless roundtrip, determinism across instances.

Layer 2: MODEL (Causal Hypergraph)
  └── Input: Normalized canonical sessions
  └── Output: Directed Acyclic Hypergraph (DAG), Lamport poset orderings, subagent trees
  └── Topology: Multi-session causal links, N-ary compactions, tool concurrency ($A \parallel B$)

Layer 3: PROJECTION
  └── Input: Causal hypergraphs and state snapshots
  └── Output: Parquet datasets for SFT (ChatML/loss-masked), DPO pairs, Eval rubrics
  └── SLA: Zero unscrubbed secrets, schema-validated Parquet, full lineage back to raw rows.
```

---

## 2. Register of Invariants ($\alpha$)

### Stabilized & Automated Invariants (Layers 0 & 1)
* **$\alpha_1$ (Content Immutability)**: Captures are frozen and verified by SHA-256 hash.
* **$\alpha_2$ (DDL Closure)**: Every table in the producer schema is either covered by column fate contracts or formally declared in `out_of_scope`.
* **$\alpha_3$ (Strict Conservation)**: `NULL != ""` across all turns and tool raw outputs.
* **$\alpha_4$ (Determinism)**: $f(\text{capture}) \rightarrow \text{session}$ is a pure function. Repeated calls yield byte-identical sessions.
* **$\alpha_5$ (Storage Idempotence)**: Writing a session twice produces identical state via transactional upsert.
* **$\alpha_6$ (Fail-Closed Drift Guard)**: Missing in-scope tables raise `SourceSchemaDrift`.
* **$\alpha_7$ (Lossless Round-Trip)**: `read(write(normalize(x))) == normalize(x)` without information drift.
* **$\alpha_8$ (Cross-Oracle Telemetry)**: Token and cost telemetry verified against independent model usage tables.
* **$\alpha_9$ (Source Isolation)**: Pipeline tests execute exclusively against frozen captures, never live stores.

### Stabilized Topological Invariants (Layer 2)
* **$\alpha_{10}$ (Causal DAG Acyclicity)**: All inter-session delegation relationships form a strictly acyclic directed graph (tested on 603 child sessions).
* **$\alpha_{11}$ (Compaction Conservation)**: $N$-ary absorptions map distinct message UIDs to their summary node without loss across multi-pass compactions.
* **$\alpha_{12}$ (Subagent Resolution)**: 100% of declared parent sessions resolve to existing capture sessions (0 orphaned children).
* **$\alpha_{13}$ (Tool Concurrency Poset)**: $K > 1$ parallel tool calls within a single assistant turn form concurrent, incomparable elements ($A \parallel B$) in Lamport's partial order.
* **$\alpha_{14}$ (CoT Phenotype Conservation)**: Thinking fields (`reasoning_content`, `codex_reasoning_items`, `reasoning_details`) remain distinctly preserved.

---

## 3. Performance & Resource SLAs

1. **Throughput SLA**:
   Normalization of single large sessions ($>18,000$ messages) must complete in $< 5.0$ seconds in Python, and $< 0.1$ seconds in native Rust.
2. **Memory Safety**:
   Cursor streaming must be strictly enforced on SQLite connections. Loading unbounded record collections into contiguous memory arrays is prohibited to prevent host segmentation faults.
3. **Auditability**:
   Every normalization and projection run must emit structured, machine-parseable JSON events conforming to `cynic.canonical.event.v1`.
