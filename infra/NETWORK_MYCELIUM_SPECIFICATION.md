# CYNIC — Mycelium Network Architecture Specification (RFC-NET-001)

> **Status**: `ratified-rfc` (v1.1.0 — conformance-annotated)
> **Authority**: `contracts/foundation.json` (SoC §481, Zero-I/O Domain Core, Strict DAG, 4-Layer Host Observation)
> **Companions**: `infra/NETWORK_DOMAIN_TAXONOMY.md`, `infra/PLANE_TAXONOMY.md`, `crates/cynic-network/`
> **Target Scale**: 100+ heterogeneous nodes (Consumer, VPS, Baremetal Datacenter, Edge, Dedicated Storage/Compute)
> **Ratified**: 2026-10-05
> **Conformance audit**: 2026-10-05 (v1.1.0 — every claim annotated with observed implementation status)

---

## 0. HOW TO READ THIS DOCUMENT

This RFC states intent. It is not a description of the current tree, and it must never be
read as one. Every normative claim below carries a conformance marker:

| Marker | Meaning |
|---|---|
| `implemented` | Verified by grep against the tree at the audit SHA. Types, parsers or code exist and are reachable. |
| `partial` | The concept exists but the contract as written is not enforced end to end. Named gaps are stated inline. |
| `contract-only` | A specified boundary or invariant with no enforcing code. The authority is declared; nothing observes or proves it. |
| `absent` | Not implemented at all. Named here so it cannot be mistaken for existing. |

A claim marked `contract-only` or `absent` is a **declared intention, not a capability**. Any
plan, surface or report that treats it as shipped is reporting phantom authority
(see `cynic-phantom-authority-audit`). Conformance markers are re-verified by grep before each
network tranche, not trusted from memory.

---

## 1. PRINCIPLES & DESIGN INVARIANTS

### 1.1 First Principles
1. **Identity Precedes Routing**: A node is immutably identified by its cryptographic Ed25519 `NodeId` (`cynic-identity`). IP addresses, physical MACs, and WireGuard endpoints are ephemeral transport paths, never identity anchors or database primary keys.
2. **Zero-I/O Domain Core**: All network data models, parsing algorithms, route planning, access control rendering, and drift evaluations reside in `cynic-network` with zero side-effects. Network I/O occurs strictly behind explicit boundaries. `implemented`
3. **Data-Centric Evolution**: The network is not static infrastructure; it is an active producer of continuous telemetry. Routing paths, relay elections, and workload admissions are dynamically governed by empirical data, not hardcoded heuristics. `partial` — snapshot type exists (§4); the dynamic governance it feeds is not wired.
4. **Mycelium Mesh Topology**: Direct peer-to-peer connectivity via WireGuard when reachable; fallback to admitted relay nodes across symmetric NATs; zero central single point of failure. `partial` — mesh plan generation exists; relay election does not.
5. **Physical L2 Adjacency Optimization**: When nodes share a physical switch or hypervisor bridge, unencrypted line-rate transport is leveraged. `partial` — the underlay/L2 address concept exists in the vocabulary resolver (§5.1); the bypass decision rule does not.
6. **Safe Secret Injection**: All private keys and admission credentials conform to `cynic-secrets` and are injected via a secret provider port. Cleartext secrets are prohibited in code, git, logs, and proof bundles. `implemented`

### 1.2 Hybrid Architecture: Functional Core / Imperative Shell (FCIS) + Data-Oriented Design

CYNIC formally rejects dogmatic Enterprise Java-style Hexagonal Architecture:
- **Pitfall of Strict Hexagonal in Systems Rust**: Excessive `Arc<dyn Port>` indirections destroy CPU cache locality, inhibit compiler inlining, force heap allocations, and multiply boilerplate mock structures.
- **The FCIS + DOD Solution**:
  - **Functional Core (`cynic-network` core modules)**: 100% pure data structures and deterministic functions. Zero async runtimes, zero network sockets, zero mock objects. Testable in microseconds on any platform.
  - **Imperative Shell (`adapters/`)**: Direct, pragmatic I/O. Collects raw kernel data (`/proc`, `ip -j`, `wg show`), invokes the pure Functional Core, and executes the emitted Action Plan.
  - **Zero-Copy & Static Dispatch**: Parsers parse slices (`&str`, `&[u8]`) directly without allocating intermediate DTOs unless explicitly needed.

`implemented` and enforced by executable tests: `crates/cynic-network/tests/architecture_boundary.rs`
asserts that I/O primitives appear only under `src/adapters/`, that the pure core holds no
`std::process`/`reqwest`/`tokio::spawn`, and that the canonical plane vocabulary is not bypassed.

---

## 2. CANONICAL PLANE TAXONOMY

Three universal planes. This is the canonical set; `local` is deliberately **not** a plane.

| Plane | Scope | CIDR Allocation | Purpose & Access Rules | DNS Zone |
|---|---|---|---|---|
| **`mesh`** | `encrypted_transport_plane` | `100.64.0.0/10` (CGNAT) or `fd00::/8` (IPv6 ULA) | **Global Overlay**: full-mesh WireGuard interconnecting all nodes over WAN/Internet. Immunized against RFC 1918 LAN collisions. | `mesh.cynic` |
| **`service`** | `canonical_service_plane` | `10.10.10.0/24` (or local rack subnet) | **High-Speed LAN Optimization**: co-located baremetal nodes, Proxmox hypervisors, local container bridges. Unencrypted line-rate access. | `service.cynic` |
| **`admin`** | `administration_only` | `192.168.100.0/24` (Isolated OOB) | **Out-of-Band Management**: baremetal console, IPMI, Proxmox web GUI, root break-glass SSH. Never carries CYNIC workload traffic. | — |

**`local` is an attribute, not a plane.** It is the node's own underlay / L2-adjacency address
(§5.1). A fourth "local plane" would collide with the trio above and reintroduce exactly the
vocabulary divergence this taxonomy exists to remove.

**Legacy mapping.** `synapse → service`, `management → admin`, `overlay → mesh`. Legacy keys are
accepted **only** at the deserialization boundary. Business code reads canonical names.

| Concern | Status | Evidence |
|---|---|---|
| Canonical trio as the domain vocabulary | `implemented` | `crates/cynic-network/src/plane_vocabulary.rs` (+14 resolver tests) |
| Legacy aliases accepted at the boundary | `implemented` | serde aliases on registry DTOs; explicit compatibility test |
| Business code free of legacy literals | `implemented` for plane reads | nine readers migrated in T4.4; `mesh_plan` in T5. Remaining `synapse` literals are inline test fixtures, DNS zone FQDNs (`synapse.cynic`, 154 across 62 files), and two sites that already resolve `canonical_service_plane` first |
| Registry files canonically renamed | `implemented` | all four registries migrated in T4.3; zero legacy plane keys remain |
| Field names still carrying the legacy vocabulary | `partial` | `synapse_ip` / `management_ip` still propagate from `fleet_discovery` and `peer_resolver_mesh` into `cynic-protocol::federation_envelopes`. The read path is canonical; the struct field names are not. Renaming them is separate work |

---

## 3. PROTOCOL MATRIX BY CONCERN

Design intent for the protocol assignment per plane and concern.

| Plane / Concern | Protocol Stack | Security & Authentication | Operational Role | Status |
|---|---|---|---|---|
| **Data Plane (Heavy)** | WireGuard (UDP) / QUIC Streams | ChaCha20-Poly1305 / mTLS | High-throughput transfer (blobs, model weights, dataset partitions). | `partial` — mesh plan generation exists; transfer path not verified here |
| **Control Plane (Intra-Fleet)** | QUIC Streams / Persistent WebSockets | mTLS + Ed25519 Node Claims | Low-latency binary state sync, epoch consensus, task claiming, delta updates. | `partial` — REST/webhook surfaces exist; the intra-fleet stream is not the transport |
| **External & Operator Surface** | HTTP/1.1 & HTTP/2 REST / JSON | Bearer Tokens / Admission Grants | Ingestion gate: `POST /workloads`, `GET /health`, `GET /nodes`, webhooks. | `implemented` |
| **Telemetry & Metrics** | Compact UDP Datagrams | Ed25519 Signed Digest | Non-blocking health heartbeats and telemetry emission. | `partial` — snapshot type exists (§4); UDP emission and signing not implemented |
| **Agent Sandbox Egress** | SOCKS5 (`socks5h`) | Local loopback authentication | Userspace L5 egress containment for untrusted workloads. | `absent` — zero occurrences of `socks` in the tree |
| **Out-of-Band Recovery** | SSH (Dedicated port) | Ed25519 Host Keys | Break-glass access for substrate repair. Excluded from automated runtime loops. | `implemented` |

REST remains permitted at the cluster facade and operator-facing APIs. It is excluded from
intra-fleet mesh traffic, which carries the control plane.

---

## 4. CONTINUOUS DATA-CENTRIC METRIC SURVEILLANCE

CYNIC enforces the **4-Layer Host Observation Model** across all network interfaces. Probes parse
raw kernel state without forking subprocesses:

```text
[ Kernel Substrate: /proc/net/dev, /proc/net/tcp, ip -j link, wg show ]
                                  │
                                  ▼
[ Functional Core: cynic-network::probe ] (Zero-I/O Parsers)
                                  │
                                  ▼
[ NetworkTelemetrySnapshot: 4-Layer Observation ]
  ├── Layer 1: Physical (Link State, Duplex, Neg. Speed, Operating MTU)
  ├── Layer 2: Volume (rx/tx bytes, rx/tx packets, rx/tx dropped, CRC errors)
  ├── Layer 3: Usage (Open ports, active socket states, bandwidth derivatives)
  └── Layer 4: Pressure & Quality (Network PSI, peer RTT, jitter, loss %, WG handshake freshness)
                                  │
                                  ▼
[ Telemetry Consumers ]
  ├── cynic-storage (Append-only WAL for verifiable historical evidence)
  ├── cynic-intel (Dynamic relay selection, E-Score node reliability, workload routing)
  └── cynic-intel-soc (Automated alerts on rogue unlisted ports or packet loss spikes)
```

**Conformance.** The layers are unevenly realized, and the honest accounting is:

| Layer | Realized as | Status | Gap |
|---|---|---|---|
| L1 Physical | `parse_addr_from_json`, `parse_route_from_json` (address + route from `ip -j`) | `partial` | no link state, duplex, negotiated speed, or operating MTU field |
| L2 Volume | `InterfaceUsage` (rx/tx bytes, packets, dropped), `parse_wireguard_telemetry` | `partial` | no CRC error counters |
| L3 Usage | `InterfaceUsage`, `parse_listening_ports_from_proc_tcp` | `implemented` | — |
| L4 Pressure | `InterfacePressure`, `parse_pressure_from_dev` | `partial` | no peer RTT, jitter, loss %, or handshake freshness as a quality signal |

`NetworkTelemetrySnapshot` (`telemetry.rs`) currently carries listening ports, WireGuard peer
metrics, and dropped-packet count. The four layers are the target model; the snapshot type is
not yet their aggregate. Consumer wiring into `cynic-storage` / `cynic-intel` is `absent`.

---

## 5. PHYSICAL ADJACENCY & SUBNET ROUTING

### 5.1 L2 Direct Bypass `partial`

When nodes reside on the same physical Ethernet switch or hypervisor bridge:
- Direct RTT $< 0.5\,\text{ms}$ and matching subnets should trigger unencrypted direct routing.
- WireGuard encapsulation is bypassed for local traffic to maximize line-rate throughput and
  utilize jumbo frames (MTU 9000).

**Conformance.** The underlay/L2-adjacency concept exists and is canonically modelled: the plane
vocabulary resolver treats `local` as a node underlay address, distinct from the three planes,
and `RegistryNodeNetwork::underlay()` / `best_ip()` prefer it for direct contact. What does not
exist is the **decision rule** — no RTT threshold, no subnet-match test, no bypass emission in the
mesh plan. The address is modelled; the policy is `contract-only`.

### 5.2 The Subnet Router Pattern (Proxmox & Datacenter Gateways) `contract-only`

1. In clusters where containerized services reside on virtual bridges (e.g. Proxmox LXC containers
   on `10.10.10.0/24`):
   - Only the hypervisor/host runs the WireGuard mesh interface.
   - The hypervisor advertises the subnet prefix `10.10.10.0/24` to the mesh.
2. Remote nodes route packets destined for `10.10.10.0/24` through the hypervisor peer endpoint.
3. This eliminates the necessity of deploying WireGuard and managing keys inside every guest.

**Conformance.** No code advertises a subnet prefix or generates a route through a peer. The
`100.64.0.0/10` mesh CIDR appears only in `cynic-core::fleet_mesh` test data, not in a routing
plan. This section is design intent awaiting an executor.

---

## 6. MITIGATION OF OPERATIONAL BLIND SPOTS

Each mitigation names its risk, its contract, and whether anything enforces the contract.

### 6.1 MTU / Path MTU Discovery & TCP MSS Clamping `absent`
- **Risk**: WireGuard encapsulation drops standard MTU from 1500 to 1420 (or 1280 on PPPoE/IPv6). Path MTU discovery black holes silently stall large payload transfers.
- **Contract**: the declarative `nftables` ruleset generated by `cynic-network::firewall_policy` MUST mandate TCP MSS clamping:
  ```text
  tcp flags syn tcp option maxseg size set rt mtu
  ```
  Default WireGuard MTU pinned to 1420 with dynamic probe downscaling.
- **Conformance**: `firewall_policy.rs` generates no `maxseg` rule and pins no MTU. The generator exists; this mitigation does not.

### 6.2 CIDR Overlap Immunity `contract-only`
- **Risk**: Collisions across arbitrary RFC 1918 subnets in 100 disparate environments.
- **Contract**: the mesh overlay is strictly forbidden from allocating arbitrary RFC 1918 subnets. It allocates exclusively from the **CGNAT** block (`100.64.0.0/10`, RFC 6598) or **IPv6 ULA** (`fd00::/8`, RFC 4193).
- **Conformance**: the reservation is normative in this RFC and in the observation plan default, but no validator rejects an out-of-policy mesh CIDR. `cynic-network` contains no `100.64` literal.

### 6.3 WAN Partitions & Local Autonomy `partial`
- **Risk**: Network partitions isolate edge nodes, causing naive fail-closed architectures to terminate active workloads.
- **Contract**: upon peer heartbeat expiry ($>180\,\text{s}$), the node transitions to `LocalAutonomy`. It continues executing pre-admitted local tasks, appends proofs to its local WAL, and initiates CAS state reconciliation upon re-establishment.
- **Conformance**: `LocalAutonomy` exists in `cynic-host` (`local_autonomy_runtime.rs`, `emergence_runtime.rs`). The 180 s heartbeat threshold and the reconciliation trigger are not located in `cynic-network`.

### 6.4 Dual-Stack IPv4 / IPv6 Resilience `absent`
- **Risk**: Incompatibility between IPv6-only VPS instances and IPv4-only residential edge nodes.
- **Contract**: the network path models dual-stack endpoints. WireGuard tunnels cross-encapsulate transparently.
- **Conformance**: no `NetworkPath` type exists in `cynic-network::types`, and no `SocketAddrV6` appears in the crate. There is no dual-stack model to enforce.

### 6.5 Clock Drift & Monotonic Synchronization `contract-only`
- **Risk**: Drifting CMOS clocks on consumer hardware cause validation failures in admission tokens and TLS certificates.
- **Contract**: token validation permits a $\pm 60\,\text{s}$ skew grace window. Runtime ordering strictly depends on relative monotonic clocks and hash-linked epoch sequences, never host wall-clock time.
- **Conformance**: no skew constant or monotonic-ordering rule exists in `cynic-network`. The contract belongs to token validation in a higher layer, which is out of this crate's scope but is not verified by this RFC.

### 6.6 Zero-Downtime Key Rotation `contract-only`
- **Risk**: Key rotation causes immediate node disconnection before peers ingest the new public key.
- **Contract**: rotations enforce a dual-key grace period ($T_{\text{grace}} \approx 300\,\text{s}$). Peers accept traffic signed by either key until the new epoch attestation achieves quorum.
- **Conformance**: no grace period, dual-key acceptance, or quorum wait exists in `cynic-network`. Owned by the identity/secret layer.

### 6.7 Boot Ordering Deadlock Prevention `partial`
- **Risk**: System daemons crash looping at boot before physical interfaces acquire DHCP leases.
- **Contract**: systemd units declare `After=network-online.target` with `Wants=network-online.target`. Daemon initialization loops implement exponential backoff rather than immediate termination.
- **Conformance**: `network-online.target` appears in `cynic-node/src/dns_contract.rs` and its verification module — not in `cynic-network`. Unit manifests are not covered by this RFC's crate boundary.

### 6.8 Socket Descriptor Exhaustion `absent`
- **Risk**: Default OS limit of 1024 open file descriptors causes `EMFILE: Too many open files` under high peer counts.
- **Contract**: systemd unit manifests mandate `LimitNOFILE=65535`. Sockets are monitored in Layer 3 telemetry.
- **Conformance**: no `LimitNOFILE` anywhere in the tree. L3 telemetry counts listening ports but not descriptor pressure.

---

## 7. CRATE ARCHITECTURE & BOUNDARIES (FCIS APPLIED)

The layout below is the **target**. Each entry is annotated with its verified state at the audit
SHA; an unannotated path in the previous revision of this RFC was aspirational and is now marked.

### cynic-network (L1 Domain — Functional Core + Imperative Shell) — 4043 LOC

```text
cynic-network/src/
├── lib.rs                 — Public facade, re-exports                     [implemented, 52 LOC]
├── types.rs               — Pure data models (Plane, addresses, routes)   [implemented, 683 LOC]
├── probe.rs               — Pure parsers (/proc/net/dev, /proc/net/tcp, ip -j, wg show)
│                                                                        [implemented, 407 LOC]
├── discovery.rs           — Pure drift detection                          [implemented, 524 LOC]
├── firewall_policy.rs     — Pure nftables generator                       [implemented, 490 LOC]
├── mesh_plan.rs           — Pure WireGuard/Headscale plan generator       [implemented, 277 LOC]
├── dns.rs                 — Pure Corefile and hosts synthesizer           [implemented, 113 LOC]
├── telemetry.rs           — NetworkTelemetrySnapshot + WireGuardPeerMetrics
│                                                                        [implemented, 27 LOC]
├── plane_vocabulary.rs    — Central plane resolver; sole legacy translator [implemented, 100 LOC]
├── transport_readback.rs  — Transport read-back proofs                    [implemented, 345 LOC]
├── transport_security.rs  — Transport security assessment                [implemented, 173 LOC]
├── registry_maintenance.rs— Registry drift/maintenance helpers            [implemented, 108 LOC]
├── watchdog_alerter.rs    — Telemetry-driven alerting                     [implemented, 154 LOC]
├── port.rs                — NetworkObservationPort (4-layer report assembly)  [implemented, 17 LOC]
├── freebox.rs             — FreeboxProbePort trait only, no adapter            [implemented, 10 LOC — INCONSISTENT]
├── actuator.rs            — Actuation plan/report types (45 LOC)              [implemented]
└── adapters/              — Imperative Shell (the ONLY I/O in this crate)
    ├── host_probe.rs      — Reads /proc, ip -j, wg show                   [implemented, 249 LOC]
    ├── headscale.rs       — REST HTTP client for Headscale membership     [implemented, 206 LOC]
    └── finance.rs         — cynic-finance NetworkMeter over /proc/net/dev [implemented, 49 LOC — DEBT]
```

**Not present in this crate.** The previous revision listed these paths. None exist; they are
recorded here so they cannot be mistaken for existing capabilities:

- `socks5/` — pure SOCKS5 protocol engine. **absent** (§3).
- `ports/` as a directory of three trait modules. **absent**. One of the three traits does exist, in
  `port.rs`: `NetworkObservationPort` (4-layer read-only report assembly, implemented by the local
  and remote host probes). `NetworkDiscoveryProbePort`, `MeshTransportPort` and
  `MeshCoordinationPort` have no equivalent anywhere in the crate.
- `adapters/nftables.rs` — applies a generated ruleset to the kernel. **absent**.
- `NetworkPath` in `types.rs`. **absent** (§6.4).
- `types.rs` exposing a canonical `Plane`. **absent** — the plane type lives in
  `cynic-protocol::plane`, and `cynic-network` resolves it through `plane_vocabulary.rs`.

### Internal inconsistency

`src/freebox.rs` declares itself a Ports-and-Adapters / hexagonal domain model. §1.2 of this RFC
rejects that architecture in favour of FCIS. The file is 10 lines and harmless in practice, but it
documents the rejected pattern inside the crate this RFC governs, and its
`FreeboxProbePort::probe_freebox_lan(endpoint)` signature implies a remote HTTP call that no
adapter implements. Recorded for a dedicated tranche.

### Known debt inside this crate

- `adapters/finance.rs` duplicates `/proc/net/dev` reading already done by `adapters/host_probe.rs`,
  and gives the network crate a dependency on `cynic-finance`. It belongs to `cynic-finance`'s own
  adapter layer, or must consume the host probe. Recorded, not fixed: removing it changes a
  public re-export (`cynic_network::NetworkFinanceMeter`) and needs its own tranche.

### cynic-host (L1 Substrate)

No network business logic; delegates to `cynic-network` adapters. `partial` —
`adapters/local_network.rs` is now a re-export of `cynic_network::adapters::host_probe`, but
`src/peer_resolver_mesh.rs` still depends on upper layers (`cynic-build`, `cynic-node-protocol`),
which inverts the strict DAG.

### cynic-node (L3 Orchestration)

The previous revision specified `network/actuator_runner.rs`, `network/sync_loop.rs` and
`network/proxy_daemon.rs`. **No `network/` directory exists.** Actuation lives as flat modules at
the crate root instead: `network_actuator.rs`, `network_actuator_contract.rs`,
`network_actuator_status/`, `network_mesh_apply_plan/`, `network_mesh_plan/`,
`network_activation_chain.rs`, `network_root_policy/`, plus `actuator_main.rs`. `absent`:
the convergence sync loop and the SOCKS5 proxy daemon do not exist.

---

## 8. IMPLEMENTATION SEQUENCE

Derived from the conformance audit. Each tranche is bounded, committed and pushed separately.

| Tranche | Scope | State |
|---|---|---|
| T1 | Move I/O into `src/adapters/`; enforce FCIS with executable architecture tests | `done` |
| T2 | Extract `LocalNetworkAdapter` from `cynic-host` into `cynic-network/adapters/host_probe.rs` | `done` |
| T3 | Remove duplicated mesh registry models and the double JSON conversion | `done` |
| T4.1 | Canonical plane vocabulary in code; legacy aliases confined to the boundary | `done` |
| T4.2 | Plane accessors; no parsed-and-dropped fields; explicit service/reservation divergence error | `done` |
| T4.3 | Rewrite `infra/registry.json` and the other registries in canonical vocabulary | `done` |
| T4.4 | Migrate the nine registry readers onto the shared plane resolver | `done` |
| T5 | Reconcile `mesh_plan.rs`'s own registry model with the central resolver | `done` |
| T6 | Resolve the `peer_resolver_mesh.rs` DAG inversion | `open` |
| T7 | Pure actuation contracts, then a `cynic-network-actuator` facade | `open` |
| T8 | Decide the `finance.rs` adapter's owning crate | `open` |
| T8b | Reconcile `freebox.rs` with FCIS: implement its adapter or delete the orphan trait | `open` |
| T9 | SOCKS5h egress containment engine | `open` — design only (§3) |
| T10 | Subnet router advertisement + L2 bypass decision rule (§5) | `open` — design only |
| T11 | MSS clamping + MTU pinning in `firewall_policy` (§6.1) | `open` |
| T12 | Dual-stack path model (§6.4) | `open` |

Tranches T1–T4.2 are proven on the builder. The rest are ordered by dependency, not by
importance: T4.3 removes a live source of vocabulary divergence, T5 and T6 repair structural
defects, and T9–T12 are new capability with no existing executor.
