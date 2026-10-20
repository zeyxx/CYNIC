# CYNIC — NETWORK DOMAIN TAXONOMY v1

> **Status**: `normative`
> **Authority**: `contracts/foundation.json` (hard boundary `strict_dag_and_single_responsibility`)
> **Companion**: `infra/CANONICAL_CRATE_MAP.md` v3, `infra/ARCHITECTURE_CONSTELLATIONS.md`
> **Measured**: 2026-09-28 from `cargo metadata` (47 workspace crates) and `src/` LOC census

---

## 1. DOMAIN DEFINITION

The **network domain** encompasses all crates that model, observe, secure, or coordinate
communication between CYNIC nodes, agents, and external services. It spans four concerns:

| Concern | Description | Example crates |
|---------|-------------|----------------|
| **Transport** | WireGuard mesh, TCP/HTTP readback, DNS | `cynic-network` |
| **Coordination** | Agent claims, task ledger, fleet orchestration | `cynic-agent-bus`, `cic-agent-bus-coordinator` |
| **Economy** | Bilateral channels, HTLC, contribution pools | `cynic-channel`, `cynic-pool` |
| **Discovery** | Capability declaration, node observation | `cynic-discovery`, `cynic-discovery-host` |
| **Consensus** | Fleet epoch sequencing, WAL, quorum | `cynic-consensus` |
| **Protocol** | Node identity, enrollment, activation | `cynic-node-protocol` |

---

## 2. CANONICAL CRATE TAXONOMY

### 2.1 Transport Layer

```
cynic-network (L1, 3,943 LOC)
├── types.rs              — MeshRole, MeshSovereignty, NetworkAlertLevel, FleetNetworkReport
├── discovery.rs          — NodeReachability, DiscoveredNode, NetworkDiscoveryProbePort
├── firewall_policy.rs    — DynamicFirewallPolicy, NetworkPlaneScope, nftables rendering
├── probe.rs              — Pure parsers (dev, proc/tcp, wg show, ip -j)
├── transport_readback.rs — TransportKind, TransportState, WgDumpPeer
├── mesh_plan.rs          — MeshRegistry, NetworkMeshPlan generation
├── transport_security.rs — TransportScope, SecurityEvidenceState, NetworkPathEvidence
├── watchdog_alerter.rs   — reconcile_network_watchdog_outbox
├── registry_maintenance.rs — reconcile_registry_observations
├── dns.rs                — DnsHostRecord, Corefile/systemd rendering
├── actuator.rs           — NetworkActuatorBundleManifest, ActivationPlan
├── telemetry.rs          — WireguardPeerMetrics, NetworkTelemetrySnapshot
├── port.rs               — NetworkObservationPort trait
├── freebox.rs            — FreeboxProbePort trait
└── lib.rs                — Facade re-exports
```

**Verdict**: Cross-cutting singleton. No split planned. God-object ceiling not breached.

### 2.2 Coordination Layer

```
cynic-agent-bus (L2, 908 LOC)
├── types.rs    — Task, Claim, TaskStatus, TaskKind, ClaimStatus, ClaimError, Proof
├── identity.rs — generate_keypair, agent_id (ed25519-dalek)
├── claim.rs    — ClaimStore trait, InMemoryClaimStore, ClaimManager
├── ledger.rs   — Ledger trait, InMemoryLedger, TaskEvent
├── ports.rs    — AgentExecutor, IdentityProvider, CoordinatorReasoner traits
└── proof.rs    — sha256_bytes, sha256_hex, diff_hash, verify_proof, handoff_hash

cynic-agent-bus-server (L2, 113 LOC)
├── lib.rs      — create_app, run_server (Axum)
└── handlers.rs — health, create_task, get_task

cynic-agent-bus-coordinator (L2, 479 LOC)
├── coordinator.rs — Coordinator, FleetTopology, Lead, Domain, decompose_plan
└── taxonomy.rs    — TaskCategory, Tier, ModelTier
```

**Verdict**: Constellation `agent-bus` complete. Server is REST projection. Coordinator is fleet orchestrator.

### 2.3 Economy Layer

```
cynic-channel (L2, 323 LOC)
├── types.rs    — ChannelPhase, ChannelSettlementProof
├── channel.rs  — AgentChannel (open, lock_task_htlc, settle_task_htlc, generate_settlement_proof)
├── htlc.rs     — TaskHtlc (new, fulfill, is_fulfilled)
└── tests.rs    — In-crate tests

cynic-pool (L2, 790 LOC)
├── types.rs     — Worker, Contribution, Share, Block, Epoch, Contributor
├── protocol.rs  — PoolMessage, Task, TaskInputs, TaskResult
├── difficulty.rs — Difficulty, PPLNSCalculator, HalvingSchedule
├── merkle.rs    — hash_leaf, hash_node, pool_merkle_root, pool_merkle_proof
├── reward.rs    — RewardState, ContributorBalance
└── error.rs     — PoolError
```

**Verdict**: `cynic-channel` is the cleanest crate (100% pure, zero I/O). `cynic-pool` has broken `[[bin]]` target and unused deps.

### 2.4 Discovery Layer

```
cynic-discovery (L2, 376 LOC)
├── types.rs — NodeRole, Capability, NodeCapabilities, FleetCapabilities
├── node.rs  — NodeCapabilities, FleetCapabilities
├── event.rs — DiscoveryEvent
└── port.rs  — DiscoveryObservationPort, NodeDiscoveryObservationPort

cynic-discovery-host (L1, 196 LOC)
└── lib.rs   — Re-exports + build_report_to_capabilities, capability_delta_event
```

**Verdict**: Clean separation. Discovery-host is adapter bridge.

### 2.5 Consensus Layer

```
cynic-consensus (L2, 1,539 LOC)
├── types.rs           — NodeEpochAttestation, FleetConsensusStatus, FleetEpochConsensusState
├── evaluator.rs       — compute_epoch_fingerprint, evaluate_fleet_consensus
└── peer/
    ├── mod.rs         — Re-exports
    ├── accounts.rs    — CitizenAccount, NodeAccount, AgentAccount, OfficeAccount, MandateAccount
    ├── engine.rs      — TransitionError, TransitionOutcome, apply_transaction
    ├── genesis.rs     — PeerGenesisDesiredState, derive_genesis_transactions
    ├── state.rs       — PeerState, AdmittedWorkload, WorkloadExecutionPhase
    ├── transactions.rs — LedgerTransaction, LedgerTransactionPayload
    └── wal.rs         — WalEntry, verify_wal_chain, replay_wal
```

**Verdict**: Fully self-contained (zero cynic-internal deps). WAL with hash-chain verification.

### 2.6 Protocol Layer

```
cynic-node-protocol (L2, 5,748 LOC)
├── node_activation.rs           — NodeActivation, activation state machine
├── node_enrollment.rs           — NodeEnrollment, enrollment chain
├── node_control.rs              — NodeControl, control plane
├── pre_os.rs                    — PreOS lifecycle
├── bootstrap_federation.rs      — Bootstrap federation
├── federation.rs                — Federation membership
├── node_identity.rs             — NodeIdentity
├── machine_enrollment.rs        — MachineEnrollment
├── deployment_epoch.rs          — DeploymentEpoch
├── local_autonomy.rs            — LocalAutonomy
├── emergence.rs                 — Emergence
├── runtime_takeover_activation.rs — RuntimeTakeoverActivation
├── peer_resolver.rs             — PeerResolver
└── node_execution_mode.rs       — NodeExecutionMode
```

**Verdict**: Created 2026-09-26 from cynic-protocol extraction. Does not exist on `main` branch.

---

## 3. DEPENDENCY GRAPH (network domain only)

```
Layer 0 (Pure Leaves):
  cynic-types (no cynic deps)
  cynic-protocol (no cynic deps)

Layer 1 (Host Substrate):
  cynic-network → cynic-protocol
  cynic-host-probes → (none)
  cynic-host → cynic-protocol, cynic-types, cynic-node-protocol, cynic-workload,
                 cynic-inference, cynic-vcs, cynic-eval, cynic-vcs-forgejo,
                 cynic-storage, cynic-build, cynic-sandbox, cynic-lifecycle,
                 cynic-network, cynic-forensics, cynic-release, cynic-lxc,
                 cynic-pkg, cynic-intel
  cynic-discovery-host → cynic-discovery, cynic-host, cynic-build

Layer 2 (Autonomous Domains):
  cynic-node-protocol → cynic-protocol
  cynic-agent-bus → cynic-protocol, cynic-types
  cynic-agent-bus-server → cynic-agent-bus
  cynic-agent-bus-coordinator → cynic-agent-bus
  cynic-channel → cynic-protocol
  cynic-pool → (none)
  cynic-market → cynic-protocol, cynic-intel [INTEL UNUSED — dead dependency]
  cynic-discovery → cynic-protocol
  cynic-consensus → (none)

Layer 3 (Orchestration):
  cynic-kernel → cynic-host, cynic-inference, cynic-protocol, cynic-secrets,
                 cynic-workload, cynic-build, cynic-intel, cynic-intel-soc,
                 cynic-askesis, cynic-node [INVERSION], cynic-storage,
                 cynic-forensics, cynic-eval, cynic-node-protocol
  cynic-node → cynic-protocol, cynic-node-protocol, cynic-secrets,
                cynic-workload, cynic-vcs, cynic-storage, cynic-build,
                cynic-host, cynic-lifecycle, cynic-network, cynic-consensus,
                cynic-intel-soc, cynic-inference, cynic-release, cynic-lxc,
                cynic-sandbox
```

---

## 4. GOVERNANCE RULES

### R1. Naming Convention
All crates MUST follow `cynic-<family>-<role>`. No exceptions.

### R2. Strict DAG
Dependencies flow downward only: L4 → L3 → L2 → L1 → L0.
**Violation**: `cynic-kernel` (L3) depends on `cynic-node` (L3) — same-layer inversion.

### R3. God-Object Ceiling
Any crate exceeding 25,000 LOC is a constitutional violation.
**Current breaches**: `cynic-host` (90K), `cynic-node` (58K), `cynic-kernel` (40K), `cynic-protocol` (35K).

### R4. Zero-I/O Domain Core
Domain modules (`entry.rs`, `parse.rs`, state machines) MUST be 100% pure.
Side-effects MUST stay behind explicit ports in `runner.rs` or `adapters/`.

### R5. No Unused Dependencies
Every declared dependency MUST be used in source code.
**Current violations**: `cynic-network` (thiserror, chrono, tempfile), `cynic-pool` (chrono, tracing, tokio), `cynic-consensus` (thiserror, chrono, ed25519-dalek), `cynic-market` (cynic-intel).

### R6. No Production Panics
Production code MUST NOT contain `unwrap()` or `expect()` that can panic.
**Current violations**: `cynic-agent-bus-coordinator` (coordinator.rs:126), `cynic-pool` (reward.rs:51).

### R7. HTTP Servers Must Be Secured
Any HTTP projection crate MUST implement:
- Authentication (token or mTLS)
- Authorization (scope-based)
- Rate limiting
- TLS (for non-loopback deployments)
**Current violation**: `cynic-agent-bus-server` has none of these.

### R8. No Hardcoded Runtime Values
IPs, ports, paths, and interface names MUST come from registry or configuration.
**Current violations**: `cynic-network` (wg0, 10.10.10.0/24, 192.168.100.0/24, 44196, 51820, 22, infra/registry.json).

### R9. Architecture Boundary Tests
Every crate MUST have `tests/architecture_boundary.rs` enforcing:
- No forbidden dependencies
- No I/O in domain modules
- No `std::process::Command` in pure modules

### R10. Orphan Crate Policy
Any crate without a workspace consumer is an orphan.
**Current orphans**: `cynic-market` (merge into `cynic-intel-futarchy`), `cynic-pool` (rename to `cynic-agent-pool`).

---

## 5. EXTRACTION SESSIONS

### Session N1: Fix cynic-network
- Remove unused deps: `thiserror`, `chrono`, `tempfile`
- Fix `extract_allowed_ports_from_registry` filter bypass bug
- Externalize hardcoded values to registry-driven config
- Expand `tests/architecture_boundary.rs` coverage

### Session N2: Fix cynic-pool
- Remove broken `[[bin]]` target (or create `src/main.rs`)
- Remove unused deps: `chrono`, `tracing`, `tokio` (move to dev-dep)
- Remove dead code: `RewardSummary`, `Contributor`, `ShareProof`, `Block`, `BlockHeader`, `MerkleProof`
- Replace `unwrap()` in `reward.rs:51` with safe pattern

### Session N3: Fix cynic-consensus
- Remove unused deps: `thiserror`, `chrono`, `ed25519-dalek`

### Session N4: Fix cynic-market
- Remove unused dep: `cynic-intel`
- Implement `CollusionDetector::detect()` or remove stub
- Remove dead code variants

### Session N5: Fix cynic-agent-bus-coordinator
- Replace `unwrap()` at line 126 with `match` or `if let`
- Fix synthetic `AgentId` collision risk

### Session N6: Fix cynic-agent-bus-server
- Add authentication middleware
- Add rate limiting
- Add TLS support
- Restrict bind address (not 0.0.0.0 by default)

### Session N7: Fix cynic-kernel inversion
- Remove `cynic-node` dependency from `cynic-kernel`
- Replace with `HostPort` traits in `cynic-protocol` residue

### Session N8: Restore cynic-node-protocol
- Create or restore the crate from `main` branch
- Ensure all consumers can resolve it

---

## 6. NETWORK PLANES

| Plane | CIDR | Purpose | Registry key |
|-------|------|---------|--------------|
| service | 10.10.10.0/24 | Service plane, DNS zone `service.cynic` | `network_policy.planes.service` |
| management | 192.168.100.0/24 | Direct bare-metal management | `network_policy.planes.management` |
| overlay | 10.20.0.0/24 | WireGuard encrypted full-mesh | `network_policy.planes.overlay` |

---

## 7. REGISTERED HOSTS

| Node ID | Role | Service IP | Services |
|---------|------|------------|----------|
| cynic-host-proxmox | hypervisor | 10.10.10.1 | — |
| local-kernel | kernel_primary | 10.10.10.100 | cynic-kernel (HTTP :3030) |
| cynic-lxc-surrealdb | storage_primary | 10.10.10.103 | surrealdb (:8000) |
| cynic-builder | build_runner | 10.10.10.119 | cynic-node (HTTP :9090) |
| forgejo | forge_primary | 10.10.10.101 | SSH (:22), forgejo (:3000) |
| cynic-lxc-headscale | mesh_coordinator | 10.10.10.40 | headscale (:8080) |

---

*v1 — 2026-09-28. Normative taxonomy for the CYNIC network domain.*
*Referenced from `contracts/foundation.json`, `AGENTS.md`, and `infra/CANONICAL_CRATE_MAP.md`.*
