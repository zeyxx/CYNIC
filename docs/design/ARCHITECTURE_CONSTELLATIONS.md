# CYNIC — Architecture Constellations & Crate Taxonomy

> **Status**: Normative Architectural SSOT (Single Source of Truth)  
> **Authority**: Subordinate to observed runtime state and `contracts/foundation.json` (§strict_dag_and_single_responsibility, §one_boundary_one_reason, §rust_runtime_authority, §low_layer_first).  
> **Supersedes and Consolidates**: `CRATE_ATLAS.md`, `CRATE_MAP.md`, `CRATE_MODELS.md`, `CRATE_OWNERSHIP.md`, `CRATE_TOPOLOGY.md`, `CRATE_VOIE.md`, `CRATE_EMERGENTES.md`.

---

## 1. Executive Summary & Core Law

CYNIC is a low-layer sovereign Rust runtime organism. Its architecture balances strict domain isolation, zero-cost abstraction, and high-throughput vertical scaling.

To prevent architectural drift and eliminate "God-object" monoliths, the workspace is governed by a constitutional boundary:

> **Constitutional Law (`contracts/foundation.json` §strict_dag_and_single_responsibility)**:  
> *Every crate follows the canonical `cynic-<family>-<role>` taxonomy. Architecture is a strict downward DAG (Layer 4 Projections -> Layer 3 Orchestrators -> Layer 2 Domains -> Layer 1 Host Substrate -> Layer 0 Pure Protocol). Cross-family concrete coupling, upward dependencies, God-object orchestrators exceeding 25k LOC, and domain I/O outside injected ports are constitutional violations.*

---

## 2. Decoupling the Two Orthogonal Axes

Architectural discussions in CYNIC distinguish two separate, orthogonal axes: the distributed infrastructure stack (RFC-002) and the Rust code dependency graph (Hexagonal DAG).

```
AXIS 1: DISTRIBUTED INFRASTRUCTURE STACK (RFC-002)
  L0 Physical Hardware & Mesh -> L1 Settlement & SSOT -> L2 P2P State Channels -> L3 Compute Routing -> L4 Intent/Energy Market

AXIS 2: RUST WORKSPACE SOFTWARE CONSTELLATION (HEXAGONAL DAG)
  L4 Projections (CLI/MCP) -> L3 Orchestration (Kernel/Node) -> L2 Autonomous Domains -> L1 Host Substrate -> L0 Pure Protocol
```

### Axis 1: Distributed Infrastructure Stack (RFC-002)

Defines how physical and virtual nodes interact across the network without a centralized coordinator:

- **Level 0 — Physical Hardware & Raw Network**: WireGuard mesh, nftables firewall, sealed Ed25519 node identities, monotone hardware clocks.
- **Level 1 — Settlement & SSOT**: Git (Forgejo sovereign remote), BFT Quorum 2/3 (`cynic-consensus`), immutable content-addressed storage (CAS), SurrealDB coordination store.
- **Level 2 — P2P State Channels (`cynic-channel`)**: Sub-millisecond in-memory peer exchanges (micro-task leases, HTLC computation preimages).
- **Level 3 — Multi-hop Onion Routing**: Sphinx-style encrypted packet delegation to remote compute/GPU nodes.
- **Level 4 — Intent & Energy Market**: Compute, VRAM, and energy scarcity arbitration via bidding.

#### Pragmatic Baseline & Risk/Reward (ROI/RR) Decision
- **Operational Reality**: The current active cluster consists of 3 dedicated physical machines (Cortex, Proxmox, Builder) connected via high-bandwidth low-latency (0.2 ms) links over WireGuard.
- **Registry Truth (2026-09-28)**: `infra/registry.json` declares **8 node targets** — 1 observed (hypervisor), 6 candidate (cortex_host, kernel_primary, storage_primary, build_runner, forge_primary, mesh_coordinator), 1 stub (no role/status). One fleet target is an empty object `{}` — recorded drift, not a working node. Registry nodes stay targets until local proof promotes them to observed.
- **Strategic Freeze**: Developing complex cryptographic state channels (HTLCs) and multi-hop onion routing for a 3-node LAN introduces high debugging overhead with low near-term ROI.
- **Active Production SSOT**: **Level 0 (Hardware/WireGuard) and Level 1 (Git/Consensus/SurrealDB) constitute the active production baseline**. Inter-node task dispatch runs via direct Unix domain sockets, authenticated local HTTP/2, and WireGuard transport. Levels 2, 3, and 4 remain dormant future protocol specifications until wide-area multi-party federation demands them.

---

### Axis 2: Rust Software Workspace Constellation (Hexagonal DAG)

Defines the compilation order, boundary enforcement, and code topology across workspace crates:

```
[Layer 4: Projections & Surfaces]       cynic-cli, cynic-mcp
             │
             ▼
[Layer 3: Orchestration & Bus]           cynic-kernel, cynic-node
             │
             ▼
[Layer 2: Autonomous Domains]           cynic-storage, cynic-inference, cynic-build,
                                         cynic-vcs, cynic-intel, cynic-askesis,
                                         cynic-eval (created), cynic-trace (planned)
             │
             ▼
[Layer 1: Host Substrate]               cynic-host, cynic-network, cynic-lxc,
                                        cynic-sandbox, cynic-pkg, cynic-forensics
             │
             ▼
[Layer 0: Pure Protocol & Types]        cynic-protocol, cynic-types, cynic-identity
```

- **Layer 0 — Pure Protocol & Foundation Types**:
  - `cynic-protocol`, `cynic-types`, `cynic-identity`.
  - Zero-I/O, zero system calls, pure serde schemas, state machine definitions, and error catalogs.
- **Layer 1 — Host Substrate & Machine Anchoring**:
  - `cynic-host`, `cynic-host-probes`, `cynic-network`, `cynic-lxc`, `cynic-sandbox`, `cynic-forensics`, `cynic-pkg`.
  - Concrete operating system interfaces (Linux systemd, Proxmox VE, SSH, nftables, sockets) exposed exclusively through portable ports (`ObservationPort`, `ProcessPort`, `NetworkPort`).
- **Layer 2 — Autonomous Business Domains**:
  - Independent hexagonal modules handling specific functional capabilities behind typed traits.
   - Core: `cynic-storage`, `cynic-vcs`, `cynic-build`, `cynic-release`, `cynic-askesis`, `cynic-domain-ops`, `cynic-intel`.
   - Families: `cynic-trace` (planned, one consolidated crate), `cynic-eval` (created 2026-09-26, consolidated per R7), `cynic-inference` (extracting) (see §3).
- **Layer 3 — Orchestration & Supervision**:
  - `cynic-kernel`, `cynic-node`.
  - In-memory event routing, lease coordination, supervision trees, and scheduler loops.
  - Consumes Layer 2 and Layer 1 capabilities **exclusively through injected traits**. Concrete imports from higher or peer domains are forbidden.
- **Layer 4 — Projections & Surfaces**:
  - `cynic-cli`, `cynic-mcp`.
  - Thin adapters responsible for argument parsing, dependency injection, and JSON/human rendering. No business logic or state retention.

---

## 3. Cohesive Domain Architecture & The Solana/Agave Precedent

CYNIC rejects the dogmatic anti-pattern of artificial "micro-crating" (creating 5 tiny crates for one cohesive feature).
Following the architectural precedent of **Solana / Agave** (`solana-runtime`, `solana-ledger`), domain features are consolidated into **cohesive bounded-context crates** using internal modules and Cargo feature flags, rather than fragmented across dozens of micro-crates.

### The 4 Pragmatic Extraction Criteria (No Dogma)

Before extracting or creating a new crate, answer these four systems engineering criteria:

1. **Dependency Firewall**: Does this code introduce heavy or toxic dependencies (e.g. RocksDB, SQLite, LibGit2, heavy HTTP stacks) that must not pollute the rest of the workspace?
2. **Autonomous Deployment Scope**: Does this component need to be embedded in a standalone daemon/tool (e.g. `cynic-pkg` installer, `cynic-node` supervisor) without pulling the full kernel?
3. **High Cohesion vs Coupling**: If two components change together and communicate thousands of times per second in-process, they MUST remain in the same crate partitioned via internal submodules (`mod types;`, `mod parse;`).
4. **Sandboxing & Blast Radius**: Does this code execute untrusted guest workloads requiring strict capability and OS isolation (e.g. `cynic-sandbox`)?

### Cohesive Domain Families (Layer 2)

- **`cynic-trace`** (planned): Consolidated multi-agent trace harvesting, zero-I/O parsing (SQLite, JSONL), and deterministic token scrubbing in internal modules (`mod types; mod parse; mod scrub;`), NOT fragmented into separate crates (R7).
- **`cynic-eval`** (created 2026-09-26, wave B): Consolidated epistemic evaluation engine, axiom verification, and DPO pair generation in internal modules. No `cynic-eval-*` split — operator vetoes 2026-09-25, confirmed 2026-09-28.
- **`cynic-inference`** (extracting): Engine management and slot scheduling managed within a single cohesive crate using feature flags (`features = ["local-llama", "remote-api"]`). 25.3K LOC — under the reduction ratchet (§5 rule 5).

---

## 4. Vertical Scaling & Hardware Optimization (Ryzen 5700G, RAM, NVMe)

CYNIC is engineered for high local throughput on bare metal (AMD Ryzen 7 5700G with Radeon Vega Graphics, 8 cores / 16 threads, NVMe PCIe storage, system RAM):

1. **Elimination of Indirection Overhead**:
   - Micro-state channels and onion routing across local nodes add serializations, heap allocations, and context switching.
   - For vertical scaling, data flows through zero-copy in-memory buffers (`bytes::Bytes`), contiguous slices, and memory-mapped files (`mmap`) directly from NVMe to inference memory.
2. **Compiler Throughput & Monolith Deconstruction**:
   - Oversized crates (measured 2026-09-28: `cynic-host` 93K LOC, `cynic-node` 58K LOC, `cynic-kernel` 40K LOC, `cynic-protocol` 35K LOC, `cynic-cli` 35K LOC, `cynic-inference` 25.3K LOC) degrade `rustc` compilation performance and trigger LLVM/libc internal compiler errors (ICE) on complex macro expansions.
   - Clean-room extraction into focused domain crates allows Cargo to leverage all 16 Ryzen threads concurrently during `cargo check` and `cargo build`, achieving incremental sub-second build times.
3. **Decoupled Local Transport**:
   - Communication between `cynic-kernel`, `cynic-node`, and local inference servers uses direct Unix domain sockets (`/run/cynic/*.sock`) or HTTP/2 over WireGuard, preserving sub-millisecond latencies.

---

## 5. Architectural Invariants & Boundary Rules

### Rule 1: Canonical Naming Taxonomy
Every crate MUST adhere to the pattern:
```
cynic-<family>             → Legitimate cross-cutting singleton
cynic-<family>-<role>      → Member of a domain constellation
```
- `<role>` describes WHAT the crate does, never a technical layer name (forbidden: `-domain`, `-ports`, `-adapter`, `-application`).
- Permitted role conventions: `-types` (pure schemas), `-parser` (zero-I/O parsers), `-engine` (runtime coordination), `-runtime` (execution substrate).

### Rule 2: The 4-Quadrant Crate Model
Every crate specification is documented as:
- **PRIMARY**: The single, non-overlapping responsibility of the crate.
- **OWNS**: Concrete modules, data structures, and internal logic.
- **DELEGATES**: Interfaces consumed from other crates via injected ports.
- **FORBIDDEN**: Responsibilities reserved for other boundaries (e.g., domain I/O, upward coupling).

### Rule 3: Strict Downward DAG
- Dependencies point exclusively downward (Layer $N \to \text{Layer } M$ where $N > M$).
- Pure domain crates (`cynic-protocol`, `cynic-storage`, `cynic-trace-*`) must NEVER import orchestrator or host crates (`cynic-kernel`, `cynic-host`, `cynic-node`).

### Rule 4: Zero-I/O Domain Core
- Domain logic (`parse.rs`, state machines, validation rules) must be 100% pure: no `std::process`, no `std::net`, no `tokio::spawn`, no filesystem mutations.
- Side effects are confined to adapters implementing explicit ports in `runner.rs` or `adapters/`.

### Rule 5: Monolith Ratchet (< 25,000 LOC Ceiling)
- Crates exceeding 25,000 LOC are under an active reduction ratchet. Measured 2026-09-28, six crates breach: `cynic-host` (93K), `cynic-node` (58K), `cynic-kernel` (40K — grew from 6.3K via organ/senses/probe modules), `cynic-protocol` (35K), `cynic-cli` (35K), `cynic-inference` (25.3K).
- Net line-of-code growth in monolith crates is blocked. New features must be placed in extracted domain crates.

### Rule 6: Executable Boundary Verification
- Every crate boundary MUST be guarded by executable integration tests in `tests/architecture_boundary.rs`.
- Audited continuously via `cynic dev crate-audit`, `cynic dev kernel-audit`, and `cynic dev ownership-audit`.

---

## 6. Concrete Coupling Audit & Decoupling Strategy

Historical analysis identified a central structural defect: `cynic-kernel` was functioning as a concrete aggregator rather than an event-driven bus.

### Historical Defect Topology
```
protocol (pure) ◄──────┐
   ▲                   │
   │                   │
inference ──ports──► host  (host consumes inference via traits ✓)
   ▲                   │
   └─────concrete─── kernel (kernel imported concrete logic ✗)
                       │
                node (imported concretely by kernel ✗)
```

1. **Kernel → Host Violation**: `cynic-kernel` historically imported concrete modules from `cynic-host` (`runtime_registry`, `lifecycle`, `authority`). **Resolution**: `cynic-host` exposes `LifecyclePort`, `RuntimeRegistryPort`, and `RuntimeAuthorityPort`; `cynic-kernel` consumes only these traits.
2. **Kernel → Inference Violation**: `cynic-kernel` imported concrete routing and proxy types (`fleet_routing`, `model_catalog`, `SlotTracker`). **Resolution**: Logic relocated to `cynic-inference`; kernel communicates via `InferenceRoutingPort`.
3. **Kernel → Node Violation**: `cynic-kernel` imported `fleet_observability` and `mesh_registry` directly. **Resolution**: `cynic-node` exposes `FleetObservabilityPort` and `MeshRegistryPort`.
4. **Phantom Dependencies**: Historical Cargo manifests declared unused `host <- kernel` and `node <- kernel` edges. **Resolution**: Cleaned up to ensure acyclic DAG compilation.

---

## 7. Crate Taxonomy & Responsibility Grid

| Crate | Layer | Primary Responsibility | Owns | Delegates | Forbidden |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `cynic-protocol` | L0 | Pure schemas, contracts, errors | Serde types, state machines, constitutional laws | Host I/O → L1; Coordination → L3 | File I/O, network calls, runtime mutations |
| `cynic-types` | L0 | Shared scalar identifiers & hashes | Content-addressed hashes, timestamps | Protocol schemas → `cynic-protocol` | System calls, I/O |
| `cynic-identity` | L0 | Pure cryptographic identities | Ed25519 keys, signature verification | Key storage → `cynic-host` | Private key generation without hardware entropy |
| `cynic-host` | L1 | Host substrate & OS abstraction | Systemd, PVE, SSH adapters, probe telemetry | Domain logic → L2; Coordination → L3 | Memory bus coordination, business rules |
| `cynic-network` | L1 | Network substrate & firewall | WireGuard mesh actuation, nftables rules | Host probes → `cynic-host` | In-memory message routing |
| `cynic-lxc` | L1 | Container virtualization substrate | LXC execution bridge, unprivileged wrappers | Host orchestration → `cynic-host` | Global scheduling |
| `cynic-storage` | L2 | Coordination data & SSOT storage | SurrealDB drivers, table schemas, PoH records | Host disk I/O → `cynic-host` | Direct UI rendering |
| `cynic-inference` | L2 | Local & remote LLM execution | Llama-server adapter, slot management | GPU/process probes → `cynic-host` | Memory coordination |
| `cynic-build` | L2 | Sovereign build pipeline & receipts | Build admission, reproducible runner logic | Subprocess execution → `cynic-host` | Direct uncontained shell commands |
| `cynic-release` | L2 | Release eligibility & relay gating | Cryptographic release gating, checksums | Build runner → `cynic-build` | Direct git pushes without proof |
| `cynic-vcs` | L2 | Source control management & locks | Worktree gating, commit proof creation | Git CLI / SSH → `cynic-host` | Direct dirty working tree mutation |
| `cynic-vcs-forgejo` | L1 | Forgejo VCS transport & webhook adapter | API tokens, webhooks, forgejo notifications | Types & ports → `cynic-vcs` | Autonomous policy decisions |
| `cynic-askesis` | L2 | Epistemic reflection & audit log | Append-only proof bundles, history queries | Storage persistence → `cynic-storage` | In-flight coordination |
| `cynic-intel` | L2 | Epistemic scoring & reasoning | E-Score calculation, futarchy algorithms | Trace parsing → `cynic-trace` (planned) | Raw host access |
| `cynic-kernel` | L3 | Coordination bus & memory authority | Leases, fencing tokens, federation membrane | Host observation → `cynic-host` (ports) | Concrete domain imports, CLI rendering |
| `cynic-node` | L3 | Node-local supervision & watchdog | Node agent tree, local build submission | Coordination → `cynic-kernel` (ports) | Global cluster state ownership |
| `cynic-cli` | L4 | Thin operator CLI facade | CLI parsing, port injection, output formats | All domain logic → L2/L3 | Business logic, state persistence |
| `cynic-mcp` | L4 | Thin MCP protocol projection | JSON-RPC tool endpoints, schema translation | Coordination → `cynic-kernel` (ports) | Bootstrap authority, scheduling |

> **Coverage note (2026-09-28)**: this grid lists the architecturally load-bearing crates only. The exhaustive 47-crate inventory, LOC census, and per-family verdicts live in `infra/CANONICAL_CRATE_MAP.md` v3, which this document references as the taxonomy SSOT.

---

## 8. Phased Extraction & Migration Slices

To prevent regression while deconstructing monoliths, extractions follow a zero-downtime slice sequence:

```
[Slice 0: Manifest Cleanup] ──► [Slice 1: Inference Extraction] ──► [Slice 2: Host Lifecycle Ports]
                                                                               │
[Slice 5: Trace & Eval Crates] ◄── [Slice 4: Node Ports] ◄── [Slice 3: Host Authority Ports]
```

1. **Slice 0 (Manifest Hygiene)**: Purge phantom dependencies in `Cargo.toml` across all crates.
2. **Slice 1 (Inference Extraction)**: Complete migration of kernel inference routing types to `cynic-inference`.
3. **Slice 2 (Host Lifecycle Inversion)**: Replace `cynic-kernel` re-exports of `cynic-host::lifecycle` with `LifecyclePort`.
4. **Slice 3 (Host Authority Inversion)**: Expose `RuntimeRegistryPort` and `RuntimeAuthorityPort` from `cynic-host` to eliminate kernel concrete imports.
5. **Slice 4 (Node Observability Inversion)**: Expose `FleetObservabilityPort` and `MeshRegistryPort` from `cynic-node`.
6. **Slice 5 (Materialize Trace & Eval Families)**:
   - Create `cynic-trace` as ONE consolidated crate: agent log ingestion, zero-I/O parsing (SQLite, JSONL), and deterministic token scrubbing in internal modules (`mod types; mod parse; mod scrub;`).
   - Create `cynic-eval` as ONE consolidated crate: validation loops, axiom verification, and DPO pair generation in internal modules (DONE 2026-09-26, wave B).
   - No micro-crate splits (`cynic-trace-types`, `cynic-trace-parser`, `cynic-trace-scrubber`, `cynic-eval-axioms`, `cynic-eval-dataset`) — R7 consolidation per `CANONICAL_CRATE_MAP.md` v3, operator vetoes 2026-09-25, confirmed 2026-09-28. The 4 extraction criteria of §3 (dependency firewall, autonomous deployment, cohesion, sandboxing) remain the admission test for any future split.
7. **Slice 6 (Ratchet Enforcement)**: Lock crate boundaries with `tests/architecture_boundary.rs` and verify via `cynic dev check --json`.
