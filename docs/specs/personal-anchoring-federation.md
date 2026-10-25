# Personal Anchoring Federation — "Entonnoir" Model

**Status:** Draft v1
**Date:** 2026-10-25
**Author:** hermes (agent)
**Scope:** Design a personal health/habit/time-anchoring system on CYNIC infra.

---

## 1. Problem

The operator has lost time-consciousness and has marked amnesias.
He needs a system that:
- Anchors every action to a timestamp.
- Measures domains: sport, meditation, nutrition, sobriety (alcohol, tobacco, cannabis, hard drugs with 1/3/6 month milestones).
- Tracks projects and actions that materialize (market-visible), not just code.
- Shows a "general return at instant T" based on actions.
- Works across devices (Pixel 7a, PC, compute-01).
- Does not require 50+ hours of dev before a judgeable result.

---

## 2. Model: "Entonnoir" (Funnel)

### 2.1 Principles

- **Local-first**: each node stores data locally first. No network dependency for writes.
- **Append-only ledger**: data propagates between nodes like a blockchain. Each entry is a block.
- **Content-addressed**: each block is addressed by its hash (blake3 or sha256).
- **Roles**: nodes access and propagate data according to their role.
- **No mutation without read-back proof**: each write is a block, each propagation is a verification.

### 2.2 Roles

| Node | Role | What it does |
|---|---|---|
| Pixel 7a (Kyon mobile) | **sensing** | Captures daily entries (sport, meditation, nutrition, sobriety, project actions). Local Room DB. |
| PC (webapp) | **viewing** | Displays dashboard, general return at instant T. Reads from local or federated data. |
| compute-01 | **anchoring** | Always-on node. Stores the canonical ledger. Receives blocks from mobile/PC. |
| builder | **build** | Build runner. Not involved in anchoring data. |
| cortex | **inference** | Embedding/reranking. Not involved in anchoring data. |

### 2.3 Block format

Each entry is a block in an append-only ledger.

```json
{
  "schema": "cynic.anchor_block.v1",
  "block_id": "blk_<uuid>",
  "sequence": 1,
  "timestamp_unix_s": 1729876543,
  "node_id": "pixel-7a",
  "domain": "sport | meditation | nutrition | sobriety | project",
  "metric": "duration_minutes | count | boolean | milestone",
  "value": 30,
  "unit": "minutes | count | bool",
  "proof": "optional hash or reference",
  "prev_hash": "sha256 of previous block",
  "hash": "sha256 of this block"
}
```

### 2.4 Propagation

1. Mobile writes block to local Room DB.
2. Mobile syncs block to compute-01 via cynic-node HTTP (POST /anchor).
3. compute-01 validates block (sequence monotonic, hash chain intact, timestamp sane).
4. compute-01 appends block to canonical ledger.
5. compute-01 propagates block to other nodes via mesh (cynic-network).
6. PC reads blocks from compute-01 (GET /anchor/summary) or local cache.

### 2.5 Consensus

Reuses cynic-consensus:
- Each node maintains a PeerState.
- Blocks are LedgerTransactions with a new payload variant: `AnchorEntry`.
- Sequence monotonicity enforced by apply_transaction.
- Checkpointing via PeerCheckpointed.

---

## 3. Schema: Domains and Metrics

### 3.1 Domains

| Domain | Metrics | Unit |
|---|---|---|
| sport | duration, type (run, gym, walk, etc.) | minutes |
| meditation | duration | minutes |
| nutrition | quality_score (1-5), meals_logged | score / count |
| sobriety | alcohol (yes/no), tobacco (yes/no), cannabis (days_since), hard_drugs (days_since) | bool / days |
| project | actions_materialized (count), hours_on_cynic, hours_on_public_project | count / hours |

### 3.2 Milestones (sobriety)

| Substance | Milestones |
|---|---|
| alcohol | 1 week, 1 month, 3 months, 6 months |
| tobacco | 1 week, 1 month, 3 months, 6 months |
| cannabis | 1 month, 3 months, 6 months |
| hard_drugs | 1 month, 3 months, 6 months |

### 3.3 General return at instant T

A single number computed from recent blocks:

```
return = (sport_hours * 1.0) + (meditation_hours * 1.5) + (nutrition_score * 0.5) + (sobriety_days * 2.0) + (actions_materialized * 3.0) - (cynic_hours_excess * 0.5)
```

Weights are adjustable. The formula is a heuristic, not a medical truth.
It is displayed on the PC dashboard and on the mobile app.

---

## 4. Reusable Bricks (Canonical)

| Brick | Crate/Path | What it provides | Reuse |
|---|---|---|---|
| Storage (WAL + projections) | crates/cynic-storage/ | WAL append-only SHA-256, SQLite/SurrealKV projections, ports (StoragePort, WalJournalPort, ProjectionCoordinatorPort, VectorStorePort), types (Hash256, SequenceNumber, WalEntry, WalReceipt) | Block storage, content-addressed entries, read-back proof |
| Traceability | contracts/burn-receipt.v1.json | Claim → receipt → raw evidence → method → anchor | Block verification pattern |
| Local-first mobile | apps/kyon-mobile/ | Room DB, sync opt-in, sensing | Mobile app base (structure, not screens) |
| HTTP server | crates/cynic-node/ (port 9090) | /health, /status, /capacity, /admin/* | Add /anchor routes here |

### Excluded (not canonical for this facet)

- **cynic-consensus**: ledger de coordination de nœuds (citizens, nodes, resources, mandates, workloads). Pas pour données personnelles. Domaine différent.
- **cynic-network**: mesh de nœuds CYNIC. Fédération d'ancrage pourrait être plus simple (sync HTTP). Pas besoin de mesh complet.
- **cynic-finance**: domaine finance, pas santé.
- **cynic-askesis**: métaphore. À revoir.

### Pattern CYNIC (générique)

Ce qu'on construit ici est une brique de la facette "OS cognitif souverain" de CYNIC.
Certaines briques pourront être utilisées pour d'autres facettes.
Donc tout doit utiliser le pattern CYNIC et être générique :

- **FCIS** : pure logic in domain modules, side effects behind ports.
- **Dynamic registries** : pas de hardcode. Domaines et métriques déclarés dans un registre.
- **Typed argv** : pas de strings brutes. Types forts pour les domaines et métriques.
- **No mutation without read-back proof** : chaque écriture est vérifiée.
- **Append-only WAL** : cynic-storage comme source de vérité.
- **Projections dérivées** : SQLite/SurrealKV comme cache, jamais source de vérité.

---

## 5. What Does NOT Exist Yet

- Federation blockchain between nodes (no consensus, no block propagation).
- Sync of anchoring data between devices.
- HTTP routes for anchoring in cynic-node.
- Mobile app for anchoring (Kyon mobile exists but is for sensing, not anchoring).

---

## 6. Implementation Plan (Organic, Bottom-Up)

### Phase 1: Schema + Ledger (1-2 days)
1. Define `cynic.anchor_block.v1` JSON schema.
2. Add `AnchorEntry` variant to `LedgerTransactionPayload` in cynic-consensus.
3. Add `apply_transaction` handler for `AnchorEntry`.
4. Unit tests for block validation.

### Phase 2: HTTP Routes in cynic-node (1-2 days)
1. Add `POST /anchor` (receive block, validate, append to ledger).
2. Add `GET /anchor/{id}` (retrieve block).
3. Add `GET /anchor/summary` (aggregated view for dashboard).
4. Add SQLite storage adapter in cynic-node (local-first, no SurrealDB dependency).
5. Integration tests.

### Phase 3: Mobile App (2-3 days)
1. Reuse Kyon mobile structure (Kotlin, Compose, Room DB).
2. Add anchoring screens: daily entry, domain tabs, milestone view.
3. Add sync to compute-01 via cynic-node HTTP.
4. Add general return display.

### Phase 4: Webapp (1-2 days)
1. Simple HTML/JS or Rust/WASM dashboard.
2. Reads from compute-01 (GET /anchor/summary).
3. Displays general return at instant T.
4. Displays milestone progress.

### Phase 5: Federation (1-2 days)
1. Propagate blocks between nodes via cynic-network mesh.
2. Checkpoint via cynic-consensus.
3. Verify propagation with read-back proof.

**Total estimated: 6-10 days.** Each phase produces a judgeable result.

---

## 7. Pitfalls

- Do not create a new crate. Reuse cynic-node's HTTP server.
- Do not store personal health data in cloud. Local-first only.
- Do not gamify. No streaks, no badges, no dopamine loops.
- Do not spend 50+ hours without a judgeable result. Each phase must produce something testable.
- The kernel MANIFEST claims it's a REST/MCP server. It is not. Use cynic-node instead.

---

## 8. Open Questions

1. Which node is the anchoring primary? (Recommendation: compute-01, always-on.)
2. Block format: JSON or CBOR? (Recommendation: JSON for v1, CBOR later if needed.)
3. General return formula: weights TBD by operator.
4. Milestone notifications: push or pull? (Recommendation: pull, no push.)