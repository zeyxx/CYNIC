# CYNIC — Mycelium Network Architecture Specification (RFC-NET-001)

> **Status**: `ratified-rfc` (v1.0.0)  
> **Authority**: `contracts/foundation.json` (SoC §481, Zero-I/O Domain Core, Strict DAG, 4-Layer Host Observation)  
> **Companions**: `infra/NETWORK_DOMAIN_TAXONOMY.md`, `infra/PLANE_TAXONOMY.md`, `crates/cynic-network/`  
> **Target Scale**: 100+ heterogeneous nodes (Consumer, VPS, Baremetal Datacenter, Edge, Dedicated Storage/Compute)  
> **Ratified**: 2026-10-05  

---

## 1. PRINCIPLES & DESIGN INVARIANTS

### 1.1 First Principles
1. **Identity Precedes Routing**: A node is immutably identified by its cryptographic Ed25519 `NodeId` (`cynic-identity`). IP addresses, physical MACs, and WireGuard endpoints are ephemeral transport paths (`NetworkPath`), never identity anchors or database primary keys.
2. **Zero-I/O Domain Core**: All network data models, parsing algorithms, route planning, access control rendering, and drift evaluations reside in `cynic-network` with zero side-effects. Network I/O occurs strictly behind explicit ports (`NetworkObservationPort`, `MeshTransportPort`, `MeshCoordinationPort`).
3. **Data-Centric Evolution**: The network is not static infrastructure; it is an active producer of continuous telemetry. Routing paths, relay elections, and workload admissions are dynamically governed by empirical data (`NetworkTelemetrySnapshot`), not hardcoded heuristics.
4. **Mycelium Mesh Topology**: Direct peer-to-peer connectivity via WireGuard when reachable; automatic fallback to admitted relay nodes (DERP/STUN) across symmetric NATs; zero central single point of failure (No single coordinator SPOF).
5. **Physical L2 Adjacency Optimization**: When nodes share a physical switch or hypervisor bridge, unencrypted line-rate transport is leveraged, avoiding unnecessary encapsulation overhead.
6. **Safe Secret Injection**: All private keys and admission credentials conform to `cynic-secrets` (`RuntimeMachineIdentity`) and are injected via `SecretProviderPort`. Cleartext secrets are prohibited in code, git, logs, and proof bundles.

---

## 2. CANONICAL PLANE TAXONOMY

| Plane | Scope | CIDR Allocation | Purpose & Access Rules | DNS Zone |
|---|---|---|---|---|
| **`mesh`** | `encrypted_transport_plane` | `100.64.0.0/10` (CGNAT) or `fd00::/8` (IPv6 ULA) | **Global Overlay**: Full-mesh WireGuard interconnecting all 100+ heterogeneous nodes over WAN/Internet. Immunized against RFC 1918 LAN collisions. | `mesh.cynic` |
| **`service`** | `canonical_service_plane` | `10.10.10.0/24` (or local rack subnet) | **High-Speed LAN Optimization**: Co-located baremetal nodes, Proxmox hypervisors, and local container bridges (`vmbr0`). Unencrypted line-rate access. | `service.cynic` |
| **`admin`** | `administration_only` | `192.168.100.0/24` (Isolated OOB) | **Out-of-Band Management**: Baremetal console, IPMI, Proxmox web GUI, root break-glass SSH. Never carries CYNIC workload traffic. | — |

---

## 3. PROTOCOL MATRIX BY CONCERN

| Plane / Concern | Protocol Stack | Security & Authentication | Operational Role |
|---|---|---|---|
| **Data Plane (Heavy)** | WireGuard (UDP) / QUIC Streams | ChaCha20-Poly1305 / mTLS | High-throughput transfer (blobs, model weights, dataset partitions). Zero head-of-line blocking. |
| **Control Plane (Intra-Fleet)** | QUIC Streams / Persistent WebSockets | mTLS + Ed25519 Node Claims | Low-latency binary state sync, epoch consensus, task claiming, delta updates (Compact Blocks model). |
| **External & Operator Surface** | HTTP/1.1 & HTTP/2 REST / JSON | Bearer Tokens / Admission Grants | Standard ingestion gate: `POST /workloads`, `GET /health`, `GET /nodes`, webhooks (Forgejo/GitHub). |
| **Telemetry & Metrics** | Compact UDP Datagrams | Ed25519 Signed Digest | High-frequency non-blocking health heartbeats and telemetry emission. Rate-limited and bounded. |
| **Agent Sandbox Egress** | SOCKS5 (`socks5h`) | Local loopback authentication | Userspace L5 egress containment for untrusted workloads. Prevents root container privilege escalation. |
| **Out-of-Band Recovery** | SSH (Dedicated port) | Ed25519 Host Keys | Break-glass access for substrate repair (`cynic-builder-ssh`). Excluded from automated runtime loops. |

---

## 4. CONTINUOUS DATA-CENTRIC METRIC SURVEILLANCE

CYNIC enforces the **4-Layer Host Observation Model** across all network interfaces. Probes parse raw kernel state without forking subprocesses:

```text
[ Kernel Substrate: /proc/net, /sys/class/net, /proc/pressure/net, wg show ]
                                  │
                                  ▼
[ Pure Parsers: cynic-network::probe ] (Zero-I/O)
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

---

## 5. PHYSICAL ADJACENCY & SUBNET ROUTING

### 5.1 L2 Direct Bypass
When nodes reside on the same physical Ethernet switch or hypervisor bridge:
- Direct RTT $< 0.5\,\text{ms}$ and matching subnets trigger unencrypted direct routing.
- WireGuard encapsulation is bypassed for local traffic to maximize line-rate throughput (1G/10G/40G) and utilize jumbo frames (MTU 9000).

### 5.2 The Subnet Router Pattern (Proxmox & Datacenter Gateways)
1. In clusters where containerized services reside on virtual bridges (e.g. Proxmox LXC containers on `10.10.10.0/24`):
   - Only the hypervisor/host runs the WireGuard mesh interface.
   - The hypervisor advertises the subnet prefix `10.10.10.0/24` to the mesh.
2. Remote nodes route packets destined for `10.10.10.0/24` through the hypervisor peer endpoint.
3. This eliminates the necessity of deploying WireGuard and managing keys inside every individual guest container.

---

## 6. MITIGATION OF OPERATIONAL BLIND SPOTS

### 6.1 MTU / Path MTU Discovery & TCP MSS Clamping
- **Risk**: WireGuard encapsulation drops standard MTU from 1500 to 1420 (or 1280 on PPPoE/IPv6). Path MTU discovery black holes silently stall large payload transfers.
- **Contract**: Declarative `nftables` rulesets generated by `cynic-network::firewall_policy` MUST mandate TCP MSS clamping:
  ```text
  tcp flags syn tcp option maxseg size set rt mtu
  ```
  Default WireGuard MTU is pinned to 1420 with dynamic probe downscaling.

### 6.2 CIDR Overlap Immunity
- **Risk**: Collisions across arbitrary RFC 1918 subnets (`192.168.1.0/24`, `10.0.0.0/24`) in 100 disparate environments.
- **Contract**: The mesh overlay is strictly forbidden from allocating arbitrary RFC 1918 subnets. It allocates exclusively from the **CGNAT** block (`100.64.0.0/10`, RFC 6598) or **IPv6 ULA** (`fd00::/8`, RFC 4193).

### 6.3 WAN Partitions & Local Autonomy
- **Risk**: Network partitions isolate edge nodes, causing naive fail-closed architectures to terminate active workloads.
- **Contract**: Upon peer heartbeat expiry ($>180\,\text{s}$), the node transitions to `LocalAutonomy` (`cynic-node-protocol`). The node continues executing pre-admitted local tasks, appends proofs to its local WAL, and initiates CAS state reconciliation upon network re-establishment.

### 6.4 Dual-Stack IPv4 / IPv6 Resilience
- **Risk**: Incompatibility between IPv6-only VPS instances and IPv4-only residential edge nodes.
- **Contract**: `NetworkPath` models dual-stack endpoints (`Option<SocketAddrV4>`, `Option<SocketAddrV6>`). WireGuard tunnels cross-encapsulate transparently (IPv4-over-IPv6 and IPv6-over-IPv4).

### 6.5 Clock Drift & Monotonic Synchronization
- **Risk**: Drifting CMOS clocks on consumer hardware cause validation failures in admission tokens and TLS certificates.
- **Contract**: Token validation permits a $\pm 60\,\text{s}$ skew grace window. Runtime ordering strictly depends on relative monotonic clocks (`Instant::now()`) and hash-linked epoch sequences (`cynic-consensus`), never host wall-clock time.

### 6.6 Zero-Downtime Key Rotation
- **Risk**: Key rotation causes immediate node disconnection before peers ingest the new public key.
- **Contract**: Rotations enforce a dual-key grace period ($T_{\text{grace}} \approx 300\,\text{s}$). Peers accept traffic signed by either key until the new epoch attestation achieves quorum.

### 6.7 Boot Ordering Deadlock Prevention
- **Risk**: System daemons crash looping at system boot before physical network interfaces acquire DHCP leases.
- **Contract**: Systemd units declare `After=network-online.target` with `Wants=network-online.target`. Daemon initialization loops implement exponential backoff rather than immediate termination.

### 6.8 Socket Descriptor Exhaustion
- **Risk**: Default OS limit of 1024 open file descriptors causes `EMFILE: Too many open files` under high peer counts.
- **Contract**: Systemd unit manifests mandate `LimitNOFILE=65535`. Sockets are actively monitored in Layer 3 telemetry.

---

## 7. CRATE ARCHITECTURE & BOUNDARIES

```text
cynic-network (L1 Domain Core — Zero-I/O):
├── types.rs              — Plane, NodeAddress, NetworkPath, TelemetrySnapshot
├── discovery.rs          — Pure drift detection, topology evaluation
├── firewall_policy.rs    — Declarative nftables generator with MSS clamping and rate limits
├── mesh_plan.rs          — Pure WireGuard/Headscale routing plan generator
├── dns.rs                — Pure Corefile and hosts synthesizer
├── socks5/               — Pure userspace SOCKS5 protocol engine and policy filter
└── ports/
    ├── observation.rs    — NetworkObservationPort trait (4 layers)
    ├── discovery.rs      — NetworkDiscoveryProbePort trait
    └── mesh.rs           — MeshTransportPort, MeshCoordinationPort traits

cynic-host (L1 Substrate Adapters):
└── adapters/network/
    ├── local.rs          — /proc, /sys, ip, nftables concrete host executor
    └── remote.rs         — ArgvRunner SSH executor for remote probes

cynic-node (L3 Orchestration):
└── network/
    ├── actuator_runner.rs — Executes root-owned cynic-network-actuator
    ├── sync_loop.rs       — Continuous mesh reconciliation and watchdog
    └── proxy_daemon.rs    — Hosts the local SOCKS5 listener for agent sandboxes
```
