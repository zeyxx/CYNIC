# CYNIC — Scientific Operating System for Agents

**Created**: 2026-10-08
**Status**: VISION
**Branch**: `agent/hermes/convergence-triage`

---

## The Vision

Transform CYNIC into a scientific operating system for agents, where:
- **Hermes** pilots the interaction
- **CYNIC** provides physical rigor and unfalsifiable memory
- **Daily work data** trains sovereign models in a closed loop

---

## The 12 Required Capabilities

| # | Capability | System | Status | Surface |
|---|---|---|---|---|
| 1 | **Sandboxing** | `cynic-sandbox` | ✅ Exists | `cynic-sandbox` crate (bwrap) |
| 2 | **Restauration** | Git worktree + commit-tree | ✅ Exists | `git worktree add --detach` + `GIT_INDEX_FILE` |
| 3 | **Convergence** | `cynic-convergence` | ✅ Exists | `cynic source worktree --apply` |
| 4 | **Hypothèses** | `cynic-judge` | ✅ Exists | `cynic judge patch/runtime` |
| 5 | **Observations** | Read-back proof | ✅ Exists | `cynic runtime proof persist` |
| 6 | **Preuve** | `cynic-runtime-proof` | ✅ Exists | `cynic runtime proof persist` |
| 7 | **Coordination** | `cynic-agent` | ✅ Exists | `cynic agent claim/release/status` |
| 8 | **Distillation** | `cynic-contradiction-distiller` | ✅ Created | Skill (this session) |
| 9 | **Mémoire** | `cynic-memory` | 🔲 Planned | SurrealKV + SQLite FTS5 |
| 10 | **Falsification** | `cynic-epistemic` | 🔲 Planned | Popperian H₀ engine |
| 11 | **Rollback** | Git + bwrap snapshot | ✅ Exists | `git worktree` + `cynic-sandbox` |
| 12 | **Isolation** | Worktree + sandbox | ✅ Exists | `cynic source worktree --allocate` |

---

## The 5 Target Systems

### 1. cynic-harness — Cybernetic Harness & Epistemic Engine

**Goal**: Sandbox + Popperian falsification + LLM tribunal

| Component | Capability | Status |
|---|---|---|
| Sandbox with safety net | 1, 2, 11 | ✅ Ready |
| Popperian falsification engine | 10 | 🔲 Planned |
| LLM tribunal (semantic arbitration) | 4 | ✅ Ready (`cynic judge`) |

**Bootstrap**: Use existing `cynic-sandbox` + `cynic judge` + `cynic runtime proof`

### 2. cynic-memory — Living Memory & Fact Registry

**Goal**: Immutable fact registry + zero amnesia + hybrid search

| Component | Capability | Status |
|---|---|---|
| Immutable fact registry | 6 | ✅ Ready (`cynic runtime proof persist`) |
| Zero amnesia (FTS + vectors) | 9 | 🔲 Planned |
| Hybrid search (FTS + graph) | 9 | 🔲 Planned |

**Bootstrap**: Use `cynic runtime proof persist` + `cynic data` (SQLite)

### 3. cynic-mcp — Sovereign MCP Gateway for Hermes

**Goal**: 4 MCP tools + 3 Hermes skills

| Component | Capability | Status |
|---|---|---|
| `cynic_epistemic_probe` | 10 | 🔲 Planned |
| `cynic_memory_query` | 9 | 🔲 Planned |
| `cynic_sandbox_run` | 1 | ✅ Ready |
| `cynic_data_observe` | 5 | ✅ Ready |
| `cynic-epistemic-protocol` skill | 10 | 🔲 Planned |
| `cynic-memory-recall` skill | 9 | 🔲 Planned |
| `cynic-subagent-coordination` skill | 7 | ✅ Ready (`cynic-agent-coordination`) |

**Bootstrap**: Wire existing surfaces to MCP, create 2 new skills

### 4. cynic-data — Data Center & SFT/DPO

**Goal**: Golden slice extraction + SFT/DPO + RLVR gyms

| Component | Capability | Status |
|---|---|---|
| Golden slice extraction | 5 | ✅ Ready (`cynic data`) |
| SFT trajectories | 9 | 🔲 Planned |
| DPO contrastive pairs | 8 | 🔲 Planned |
| RLVR gyms | 10 | 🔲 Planned |
| Arrow/Parquet export | 5 | 🔲 Planned |

**Bootstrap**: Use `infra/data_system/` + `cynic data` surfaces

### 5. cynic-token-ledger — Token Accounting & Compute Ecology

**Goal**: Strict ledger + L0→L3 cascade

| Component | Capability | Status |
|---|---|---|
| Token ledger | 6 | ✅ Ready (`cynic build invoice`) |
| L0→L3 cascade | 4 | 🔲 Planned |
| GPU-second accounting | 6 | 🔲 Planned |

**Bootstrap**: Use `cynic build invoice` + `cynic runtime metrics`

---

## The Bootstrap Strategy

**Use CYNIC to develop CYNIC.** Every session generates training data.

```
Hermes uses CYNIC → produces traces → trains CYNIC → CYNIC improves → ...
```

### Phase 1 — Dogfooding (NOW)

Use existing surfaces to develop the 5 systems:
- `cynic-sandbox` for safe execution
- `cynic agent claim/release` for coordination
- `cynic build proof` for validation
- `cynic runtime proof persist` for memory
- `cynic judge patch` for evaluation
- `cynic source worktree --allocate` for isolation

### Phase 2 — Data Collection (DURING)

Every operation produces structured data:
- Merge conflicts → SFT training data
- Contradictions → DPO contrastive pairs
- Rebase failures → RLVR gym scenarios
- Domain classifications → SFT training data

### Phase 3 — Model Training (AFTER)

Train sovereign models on collected data:
- SFT on successful trajectories
- DPO on contradiction resolutions
- RLVR on merge/rebase scenarios

---

## The Bootstrap Experiment Skill

The convergence current is the first bootstrap experiment. It exercises:
- Sandboxing (worktree isolation)
- Restauration (merge/rebase with rollback)
- Convergence (domain classification + cherry-pick)
- Hypothèses (predicting conflicts)
- Observations (read-back proof)
- Distillation (contradiction resolution)

See `cynic-bootstrap-experiment` skill for the formal protocol.

---

## Execution Log

| Date | Milestone |
|---|---|
| 2026-10-08 | Vision documented |
| 2026-10-08 | `cynic-contradiction-distiller` created |
| 2026-10-08 | Capability map completed (12 capabilities, 5 systems) |
| 2026-10-08 | Bootstrap strategy defined |
