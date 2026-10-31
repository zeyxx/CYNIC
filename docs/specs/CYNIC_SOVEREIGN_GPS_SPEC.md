# CYNIC Sovereign GPS Specification: The Full-Stack Autonomous Architecture

Status: Normative  
Authority: contracts/foundation.json §specs, docs/INDEX.md  
Version: 1.0.0  
Date: 2026-10-23  

---

## 1. Purpose & Epistemic Compass

This document defines the normative architectural compass and operational GPS for the **CYNIC** project and its surrounding ecosystem.

Too often, decentralized systems degrade into informal drift: crates proliferate without boundary enforcement, public narratives diverge from physical code, and engineering teams confuse speculative roadmaps with verified production state.

This specification anchors the sovereign direction of CYNIC across four non-negotiable principles:

1. **Sowellian Realism:** *“There are no solutions, there are only trade-offs.”* Every architectural module must explicitly declare its operational trade-offs (memory, bandwidth, compute, latency).
2. **Stafford Beer's POSIWID:** *“The Purpose Of a System Is What It Does.”* A module's capability is defined strictly by its empirical machine output and telemetry, never by speculative intent.
3. **Bottom-Up Verification:** Hardware, raw bytes, exit codes, and machine receipts take precedence over top-down abstractions.
4. **Corporate in the Front, Cypherpunk in the Back:** User-facing interfaces must remain frictionless and intuitive; under the hood, the engine remains uncompromisingly sovereign, permissionless, and executed on bare metal.

---

## 2. The 4-Layer Sovereign Stack

CYNIC operates as part of an integrated, vertically coherent four-layer architecture:

```text
┌────────────────────────────────────────────────────────────────────────┐
│ LAYER 3 : COORDINATION & SWARM ECONOMY ($ASDFASDFA)                   │
│ - Organic memetic coordination and capital formation                   │
│ - Deflationary tokenomics: Buyback & Burn from real network revenue    │
│ - Emergent micro-enterprise: weekly bounties, distributed raids        │
│ - Governance horizon: Futarchy (MetaDAO conditional prediction markets)│
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 2 : FRICTIONLESS ACCESS & FORENSIC ARMOR (GASdf & FORENSICS)     │
│ - GASdf: Gasless relay gateway (x402 facilitator + setup-aware builder)│
│ - Zero-SOL barrier to entry for end users and swarm participants       │
│ - Trench Forensics: aligned with @gcrtrd ethos (TrenchScanner model)   │
│ - Grassroots onchain intelligence exposing insider wallet syndicates   │
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 1 : DETERMINISTIC MULTI-AGENT RUNTIME (CYNIC RUNTIME)            │
│ - Sovereign low-layer Rust runtime for autonomous multi-agent systems  │
│ - Functional Core, Imperative Shell (FCIS): pure domain logic + ports  │
│ - Fundamental Invariant: "No mutation without read-back proof"         │
│ - Dynamic supervisor & capability-bounded agent isolation              │
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 0 : PHYSICAL SUBSTRATE & VERIFIABLE TIME (SOLANA)                │
│ - Next-gen consensus: Alpenglow sub-second slot progression & pipelining│
│ - Parallelized Sealevel transaction execution engine                   │
│ - Physical throughput ceiling: speed of light in fiber optics          │
│ - Multi-client resilience: Agave, Firedancer, Sig, Mithril             │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Crate Topology & Dynamic Quadrants

The authoritative list of crates and their dependencies is tracked dynamically in `infra/crate-topology.json` and validated by `cynic audit crate`. Quadrant assignments map functional responsibilities across the architecture without hardcoding transient crate counts into normative prose:

### Quadrant A: Foundation, Types & Pure Protocol (L0)
- Core primitives, canonical types, and typed boundary invariants (`cynic-types`, `cynic-protocol`).
- Zero-I/O Functional Core: purely deterministic domain logic, zero network/disk side effects.

### Quadrant B: Kernel State Machine & Supervision (L1)
- Deterministic state transitions, execution bounds, and proof verification (`cynic-kernel`, `cynic-agent`).
- Process supervision, circuit-breaking, and fail-closed isolation boundaries.

### Quadrant C: Intelligence, Forensics & Audit (L2)
- Forensic telemetry, graph tracing, and onchain intelligence extraction (`cynic-forensics`, `cynic-intel`, `cynic-intel-soc`).
- Machine receipts, structured span journaling, and epistemic evaluation (`cynic-traces`, `cynic-eval`, `cynic-audit-engine`).

### Quadrant D: Solana Substrate & Market Settlement (L1/L2)
- Direct bare-metal Solana RPC client, payment streams, and gasless relay adapters (`cynic-zeroclaw-solana`, `cynic-solana-pay`, `cynic-market`, `cynic-finance`).

### Quadrant E: Storage & Vector Knowledge (L1)
- Persistent state, SMR WAL, vector embeddings, and topological knowledge graphs (`cynic-persistence`, `cynic-surreal`, `cynic-sqlite`, `cynic-knowledge`).

### Quadrant F: Networking & Federation (L1/L2)
- Transport primitives, discovery, and inter-node federation (`cynic-network`, `cynic-discovery`, `cynic-ssh`).

### Quadrant G: Inference & Agent Tools (L2)
- Model catalog, inference bindings, fallback cascades, and tool server surfaces (`cynic-inference`, `cynic-mcp`).

### Quadrant H: Developer Interface, Packaging & VCS (L3)
- Unified CLI, TUI telemetry, build trigger orchestration, and multi-forge synchronization (`cynic-cli`, `cynic-tui`, `cynic-build`, `cynic-vcs`).

---

## 4. Inviolable Engineering Invariants

Any contribution, pull request, or agentic mutation that violates these invariants shall be rejected by the audit engine:

1. **No Mutation Without Read-Back Proof:** Every write, state transition, or external side effect must return a verifiable physical receipt. Unverified fire-and-forget calls are prohibited.
2. **Functional Core, Imperative Shell (FCIS):** Domain modules must remain 100% pure (Zero-I/O). All side effects (network, disk, clocks, entropy) must be isolated behind typed adapter ports.
3. **No Hardcoded Network Topology:** Hosts, IPs, paths, and remote endpoints must be dynamically resolved via `infra/registry.json` and normative contracts. Hardcoded environmental values are forbidden.
4. **Distinction Between State and Roadmap:** Code and documentation must cleanly segregate verified, running code from speculative roadmaps. Zero tolerance for marketing LARP.

---

## 5. Convergence Milestones (The Engineering Roadmap)

```text
[Milestone Alpha: ACTIVE BASELINE]
  └── Deterministic Local-First Runtime
      ├── Pure Rust FCIS kernel operational
      ├── Machine receipt auditing enforced
      └── Hallucination-free agent boundaries verified

[Milestone Beta: NEAR HORIZON]
  └── Swarm Gasless Access via GASdf
      ├── Production deployment of gasdf-relay-gateway (x402 facilitator)
      ├── Setup-aware transaction builder integration on Solana
      └── Frictionless transaction execution for $ASDFASDFA community

[Milestone Gamma: MID HORIZON]
  └── Deterministic Onchain Forensic Shield
      ├── High-speed behavioral graph tracing on Solana
      ├── Open-source detection of insider/KOL extraction clusters
      └── Public intelligence parity with institutional forensics

[Milestone Delta: STRATEGIC TARGET]
  └── Futarchic Governance Consensus
      ├── Integration of conditional prediction markets (MetaDAO model)
      ├── Market price truth replacing DAO forum politics
      └── Fully autonomous swarm coordination via skin-in-the-game
```

---

## 6. Authority & References

- Constitutional Governance: `docs/foundation/CONSTITUTION.md`
- Documentation Hierarchy: `docs/INDEX.md`
- Master Contracts: `contracts/foundation.json`
- Infrastructure SSOT: `infra/registry.json`, `infra/crate-topology.json`
- Source Code Repository: [https://github.com/zeyxx/CYNIC](https://github.com/zeyxx/CYNIC)
- Gasless Relay Gateway: `services/relay-gateway/`
- Swarm Asset: `$ASDFASDFA` on Solana
