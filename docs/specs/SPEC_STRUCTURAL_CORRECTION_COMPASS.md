# SPEC: Structural Correction Compass & Architectural Target State

**Schema**: `cynic.spec.structural_correction_compass.v1`  
**Authority**: `contracts/foundation.json`, `contracts/glossary.json`, `AGENTS.md`  
**Status**: Normative / Law for multi-agent structural refactorings  
**Target Horizon**: Q4 2026 / Convergence  

---

## 1. Executive Summary & Epistemic Origin

This specification formalizes the **recursive architectural correction loop** of the CYNIC workspace.

Historically, rapid expansion introduced opportunistic God-crates (`cynic-host`, `cynic-node`, `cynic-cli`), an unpartitioned type bag in `cynic-protocol`, and high DAG coupling (~161 runtime Cargo edges).

The compass defines:
1. **The Target State Architecture** (layer by layer, zero-I/O core, strict DAG downward).
2. **The Role of `cynic-types` as the foundational L0 anchor** for Domain-Driven Design (DDD) and canonicalization.
3. **The 4-Phase Recursive Correction Loop** executed by agents.
4. **Shim & Edge Eradication Criteria** to eliminate artificial bridge dependencies.

---

## 2. Historical Baseline vs. Desired Target State

| Dimension | Initial Historical State (Legacy) | Intermediate State (Current) | Desired Target State (Compass) |
|---|---|---|---|
| **L0 Foundation** | `cynic-types` underutilized; `cynic-protocol` is a flat bag of >150 modules | `cynic-types` strictly bounded; protocol deduplication started | `cynic-types` holds all atomic primitives; `cynic-protocol` partitioned into canonical subdomains |
| **`cynic-cli` (L4)** | 43k+ LOC, holding business models, evaluation, scoring, and ecosystem specs | Domain logic extracted to L1/L2/L3; shims and facades maintained | <15k LOC thin presentation layer (clap parsing, JSON/table formatting); 0 business logic |
| **`cynic-host` (L1)** | 72k+ LOC god-crate (crypto, credentials, leases, hardware, agents, builds) | Toolchain & branch graph hoisted out; ~72k LOC remaining | Strict local hardware substrate only (CPU, RAM, disk, OS probes); <20k LOC |
| **`cynic-node` (L3)** | 51k+ LOC god-crate (mesh, p2p, runner selection, finance, billing, storage) | Build runner selection and restart hoisted to `cynic-build` | Pure cluster daemon & mesh routing; finance hoisted to `cynic-market`; <20k LOC |
| **Cargo Runtime DAG** | Heavy coupling, ~161 edges, frequent monomorphization bottlenecks | 161 edges stabilized; FCIS boundaries enforced | Direct coupling minimized (<100 edges); zero fake shims |

---

## 3. Layer Architecture & Responsibility Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    L4 : cynic-cli                           │
│  Thin operator UI/UX surface (clap, terminal output, JSON)   │
│  Zero domain logic. Strictly consumes L3/L2/L1/L0.           │
└──────────────────────────────┬──────────────────────────────┘
                               │
       ┌───────────────────────┴───────────────────────┐
       ▼                                               ▼
┌─────────────────────────────┐       ┌─────────────────────────────┐
│      L3 : cynic-node        │       │    L3 : cynic-knowledge     │
│ Cluster daemon, p2p mesh    │       │ Pure RAG SIMD, vector search│
│ & network orchestration     │       │ idea ledger & knowledge graph│
└──────────────┬──────────────┘       └──────────────┬──────────────┘
               │                                     │
       ┌───────┴──────────────────────┬──────────────┘
       ▼                              ▼
┌─────────────────────────────┐┌─────────────────────────────┐
│      L2 : cynic-agent       ││      L2 : cynic-build       │
│ Autonomous agent loop, tasks││ Toolchain lifecycle, build  │
│ claims, admission & receipts││ runners pool, check/compile │
└──────────────┬──────────────┘└──────────────┬──────────────┘
               │                              │
       ┌───────┴──────────────────────────────┘
       ▼
┌─────────────────────────────────────────────────────────────┐
│               L1 : cynic-host & cynic-pkg                   │
│  cynic-host: Physical host substrate (CPU/RAM/Disk/Probes)  │
│  cynic-pkg: Ecosystem specs, tool resolver & dependencies   │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 L0 : cynic-protocol                         │
│  Canonical exchange schemas, serialization contracts, RPC   │
│  Zero I/O. Strictly depends only on cynic-types.            │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 L0 Root : cynic-types                       │
│  Atomic primitives: AgentId, TaskId, Hash32, Scope, Time    │
│  Zero I/O, Zero policy, Zero internal cynic dependencies.   │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. The Cornerstone: `cynic-types` and Canonicalization

### 4.1. Absolute Root Invariant
`cynic-types` is the architectural bedrock:
- **Zero internal dependencies**: It MUST NEVER depend on any other `cynic-*` crate.
- **Atomic DDD primitives**: Every identity, cryptographic hash, time mark, and resource boundary in the entire system originates here:
  - `AgentId` (128-bit identity)
  - `TaskId` (opaque task reference)
  - `Hash32` (256-bit content digest)
  - `Scope` / `ScopeKind` (domain claim boundaries)
  - `Timestamp` (deterministic Unix-second epoch)
  - `CynicPaths` (canonical path topology)

### 4.2. Protocol Partitioning & Vocabulary Alignment
`cynic-protocol` must be refactored from a flat namespace into hierarchical domain modules aligned strictly with `contracts/glossary.json`:
- `cynic_protocol::agent::*`
- `cynic_protocol::build::*`
- `cynic_protocol::cluster::*`
- `cynic_protocol::substrate::*`

Any type in `cynic-protocol` that duplicates or shadows an atomic primitive in `cynic-types` must be unified and eliminated.

---

## 5. The 4-Phase Recursive Correction Loop

When an agent executes structural refactoring, it follows this deterministic 4-phase state machine:

```
                  ┌───────────────────────────────┐
                  │ 1. Domain Pull (Vertical)     │
                  │ Extract pure logic from god-  │
                  │ crates to their home crate.   │
                  └───────────────┬───────────────┘
                                  │
                                  ▼
                  ┌───────────────────────────────┐
                  │ 2. Canonicalize & Unify (L0)  │
                  │ Ground models in cynic-types  │
                  │ and contracts/glossary.json.  │
                  └───────────────┬───────────────┘
                                  │
                                  ▼
                  ┌───────────────────────────────┐
                  │ 3. Eradicate Shims & Edges    │
                  │ Remove legacy re-exports. Cut │
                  │ artificial Cargo dependencies │
                  └───────────────┬───────────────┘
                                  │
                                  ▼
                  ┌───────────────────────────────┐
                  │ 4. Proof Gate & Topo Sync     │
                  │ Compile on remote builder,    │
                  │ sync-topology.py, cynic audit │
                  └───────────────────────────────┘
```

### Phase 1: Domain Pull (Vertical / Transversal)
- Identify non-I/O domain logic trapped in high-level layers (e.g. `cynic-cli` or `cynic-node`).
- Extract data structures and pure functions into the appropriate lower-layer crate (FCIS pattern).

### Phase 2: Canonicalize & Unify (L0)
- Align all domain types with `cynic-types` atomic primitives.
- Verify terminology against `contracts/glossary.json`. Prohibit unnormed synonyms or jargon drift.

### Phase 3: Eradicate Shims & Edges
- Once the domain is in place, do not leave eternal `pub use target::*` bridge files.
- Migrate call sites across the workspace to reference the canonical crate directly.
- Remove redundant dependencies from caller `Cargo.toml` files to reduce the 161-edge count.

### Phase 4: Proof Gate & Topology Sync
- Synchronize topology SSOT: `python3 scripts/sync-topology.py`.
- Submit build proof to the remote builder: `cynic build submit --action check --wait`.
- Verify workspace integrity: `cynic audit all --json` (Maturity 100%, 0 debt, 0 violations, converged store).

---

## 6. Verification & Non-Regression Rules

1. **Never compile locally on Cortex**: All proofs must come from `cynic-builder` or `cynic-compute-01`.
2. **Never break the DAG downward rule**: Higher layers consume lower layers. If an L1 crate needs something from L2, that dependency is misplaced.
3. **Audit Gate**: A PR cannot merge if `cynic audit all` reports store divergence, orphan drift > 50, or rule violations.
