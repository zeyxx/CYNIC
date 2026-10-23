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
│ LAYER 1 : DETERMINISTIC MULTI-AGENT RUNTIME (CYNIC KERNEL)             │
│ - Low-layer sovereign Rust runtime for autonomous agent systems        │
│ - Zero-I/O Functional Core, Imperative Shell (FCIS) architecture       │
│ - Fundamental Invariant: "No mutation without read-back proof"         │
│ - Deterministic elimination of agentic drift and LLM hallucinations   │
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 0 : PHYSICAL SUBSTRATE & VERIFIABLE TIME (SOLANA)                │
│ - Sub-second finality (400ms slots) via Proof of History (PoH)         │
│ - Parallelized Sealevel transaction execution engine                   │
│ - Physical throughput ceiling: speed of light in fiber optics          │
│ - Multi-client resilience: Agave, Firedancer, Sig, Mithril             │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Crate Topology Allocation (The 47 Workspace Crates)

Every crate within the CYNIC Cargo workspace is formally mapped to an operational quadrant. No crate may exist without an explicit functional assignment:

### Quadrant A: Kernel, Types & Foundational Discipline
- `crates/cynic-types`: Primitive types, invariants, and canonical identities.
- `crates/cynic-protocol`: Wire serialization, message framing, and typed schemas.
- `crates/cynic-kernel`: Pure logic execution core (Zero-I/O).
- `crates/cynic-askesis`: Rigorous self-discipline, boundary checks, and behavioral constraints.

### Quadrant B: Forensic Intelligence, Auditing & Evaluation
- `crates/cynic-audit-engine`: Deterministic gatekeeper validating rule conformance.
- `crates/cynic-forensics`: Graph analysis, transaction tracing, and anomaly extraction.
- `crates/cynic-intel`: Structured threat intelligence gathering and pattern scoring.
- `crates/cynic-intel-soc`: Security operations center telemetry and incident response.
- `crates/cynic-traces`: Trace emission, span journaling, and machine receipts.
- `crates/cynic-eval`: Epistemic validation and regression testing against ground truth.

### Quadrant C: Solana Settlement, Gas & Market Operations
- `crates/cynic-zeroclaw-solana`: Bare-metal Solana client, RPC multiplexing, and signature verification.
- `crates/cynic-solana-pay`: Native payment stream parsing and verification.
- `crates/cynic-market`: Order book reading, liquidity evaluation, and market mechanics.
- `crates/cynic-finance`: Financial accounting, burn tracking, and balance reconciliation.

### Quadrant D: Node Lifecycle, Fleet Supervision & Infrastructure
- `crates/cynic-node`: Autonomous node agent daemon and runtime hosting.
- `crates/cynic-node-protocol`: Inter-node consensus, heartbeat, and synchronization.
- `crates/cynic-host`: Bare-metal host interface and hardware telemetry.
- `crates/cynic-supervisor`: Process supervision, auto-recovery, and fail-closed circuits.
- `crates/cynic-fleet`: Multi-agent fleet orchestration and work allocation.
- `crates/cynic-lifecycle`: Startup, healthcheck, migration, and shutdown state machines.
- `crates/cynic-proxmox`: Virtualization and hypervisor orchestrator integration.
- `crates/cynic-lxc`: Hardened OS-level container isolation for agent execution.
- `crates/cynic-sandbox`: Restricted evaluation boundaries for untrusted code execution.

### Quadrant E: Storage, Knowledge & Evidence Journaling
- `crates/cynic-storage`: Tiered evidence storage (K1-K8) and immutable disk journals.
- `crates/cynic-knowledge`: Vector storage, RAG retrieval, and factual knowledge graphs.
- `crates/cynic-pool`: Resource pooling, connection caching, and memory reuse.
- `crates/cynic-secrets`: Secure key management, vault integration, and zero-leak memory.

### Quadrant F: Network, Channel & P2P Discovery
- `crates/cynic-network`: Transport abstractions (QUIC, TCP, WebSocket) with mutual TLS.
- `crates/cynic-channel`: Bounded async message channels with backpressure control.
- `crates/cynic-discovery`: Local and remote peer discovery mechanisms.
- `crates/cynic-discovery-host`: Host-level network discovery agent.
- `crates/cynic-consensus`: Local consensus arbitration and voting logic.
- `crates/cynic-workload`: Task queue dispatch and priority scheduling.

### Quadrant G: Inference, Model Metrology & AI Tools
- `crates/cynic-inference`: Local and remote LLM provider bindings with fallback cascades.
- `crates/cynic-huggingface`: Tokenizer loading, weight management, and local inference execution.
- `crates/cynic-mcp`: Model Context Protocol server exposing verified tools to agents.

### Quadrant H: Developer Interface, Packaging & Version Control
- `crates/cynic-cli`: Unified command-line interface with typed argument validation.
- `crates/cynic-tui`: Terminal user interface for live observability and operator telemetry.
- `crates/cynic-build`: Hermetic build orchestration, sccache tracking, and compiler safety.
- `crates/cynic-pkg`: Packaging and dependency bundle verification.
- `crates/cynic-release`: Release gating, changelog verification, and cryptographic signing.
- `crates/cynic-vcs`: Version control primitives and branch policy enforcement.
- `crates/cynic-vcs-providers`: Forgejo, GitHub, and remote git provider integrations.
- `crates/cynic-domain-ops`: Domain-level operational automation.

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
