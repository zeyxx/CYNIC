# CYNIC — Mycelium Network Architecture Specification (RFC-NET-001)

> **Status**: `draft` (Proposed Autonomous Network Specification)  
> **Authority**: `contracts/foundation.json` (SoC §481, Zero-I/O Domain Core, Strict DAG)  
> **Companions**: `infra/NETWORK_DOMAIN_TAXONOMY.md`, `infra/PLANE_TAXONOMY.md`, `crates/cynic-network/`  
> **Target Scale**: 100+ heterogeneous nodes (Consumer, VPS, Baremetal Datacenter, Edge, Dedicated Storage/Compute)  
> **Defined**: 2026-10-05  

---

## 1. PRINCIPLES & DESIGN INVARIANTS

### 1.1 First Principles
1. **Identity Precedes Routing**: A node is immutably identified by its cryptographic Ed25519 `NodeId` (`cynic-identity`). IP addresses and WireGuard endpoints are ephemeral transport paths (`NetworkPath`), never identity anchors or primary keys.
2. **Zero-I/O Domain Core**: All network data models, parsing logic, route planning, access control rendering, and drift evaluation reside in `cynic-network` with zero side-effects. Network I/O occurs solely behind explicit ports (`NetworkObservationPort`, `MeshTransportPort`, `MeshCoordinationPort`).
3. **Data-Centric Evolution**: Network state transitions, peer latencies, drop counts, and connection anomalies are captured as structured evidence payloads. Routing, relay election, and anomaly detection are driven by empirical data (`NetworkTelemetrySnapshot`), not static hardcoding.
4. **Mycelium Mesh Topology**: Direct peer-to-peer connectivity via WireGuard where possible; automatic fallback to admitted relay nodes (DERP/STUN) when crossing symmetric NATs; zero central points of failure (No single coordinator SPOF).
5. **Safe Secret Injection**: Secret keys (WireGuard private keys, coordination tokens) are managed under `cynic-secrets` (`RuntimeMachineIdentity`) and injected via `SecretProviderPort`. No cleartext secrets exist in data structures or git.

---

## 2. PROTOCOL MATRIX BY CONCERN

| Plane / Role | Transport Protocol | Security & Auth | Justification & Industrial Reference |
|---|---|---|---|
| **Data Plane** (Workloads, Blobs) | WireGuard (UDP) / QUIC Streams | ChaCha20-Poly1305 / mTLS | High throughput, zero head-of-line blocking, kernel-level encryption, seamless NAT traversal. |
| **Control Plane** (Tasks, Ledger, State) | QUIC Streams / gRPC / WebSockets | mTLS + Ed25519 Node Claims | Persistent bi-directional channels, binary framing, compact delta updates (Bitcoin Compact Blocks / Solana Turbine model). |
| **Telemetry & Discovery** | UDP Compact Datagrams | Ed25519 Signed Digest | Fire-and-forget metrics, non-blocking probe loops. Rate-limited and bounded. |
| **Agent Sandbox Egress** | SOCKS5 (`socks5h`) | Local loopback authentication | Userspace L5 containment for untrusted agent runtimes. Eliminates requirement for root nftables manipulation inside containers. |
| **Out-of-Band Recovery** | SSH (Dedicated port) | Ed25519 Host Keys | Dedicated break-glass path (`cynic-builder-ssh`), isolated from the software runtime overlay. |

---

## 3. IDENTIFIED BLIND SPOTS & MITIGATION CONTRACTS

### 3.1 Blind Spot 1: MTU / Path MTU Discovery & TCP MSS Clamping
* **Risk**: WireGuard encapsulates packets with a 60–80 byte overhead. Standard Ethernet MTU (1500) drops to 1420 (or 1280 on PPPoE/IPv6). Routers that drop ICMP "Fragmentation Needed" cause silent hangs on large TCP payloads (e.g. `git clone`, large JSON proofs, model downloads).
* **Mitigation**: Declarative nftables ruleset in `cynic-network::firewall_policy` MUST inject automated TCP MSS clamping:
  ```text
  tcp flags syn tcp option maxseg size set rt mtu
  ```
  All WireGuard interfaces standardize on MTU 1420 by default, with dynamic probe fallback.

### 3.2 Blind Spot 2: CIDR Overlaps in Heterogeneous Networks
* **Risk**: Across 100 nodes in disparate consumer homes and data centers, overlapping RFC 1918 subnets (`192.168.1.0/24`, `10.0.0.0/24`) are mathematically certain.
* **Mitigation**: The global overlay plane MUST NOT use arbitrary RFC 1918 subnets. It allocates exclusively from:
  1. **IPv6 Unique Local Addresses (ULA)**: `fd00::/8` with a cluster-unique 40-bit pseudo-random global ID (RFC 4193).
  2. **Carrier-Grade NAT (CGNAT) Space**: `100.64.0.0/10` (RFC 6598, ~4M addresses), completely segregated from local LAN ranges.

### 3.3 Blind Spot 3: Network Partition & Split-Brain (Local Autonomy)
* **Risk**: If a cluster node or edge cluster loses WAN connectivity to the mesh coordination plane, a naive "fail-closed" model terminates all workloads.
* **Mitigation**: Implementation of `cynic_node_protocol::local_autonomy`:
  - When peer heartbeat expires (>180s), node transitions to `LocalAutonomy`.
  - Node executes admitted local tasks, writing sealed append-only receipts to local disk WAL.
  - Upon reconnection, reconciliation occurs via CAS replay and cryptographic proof attestation.

### 3.4 Blind Spot 4: Ouroboros Bootstrap & DNS Leaks
* **Risk**: Resolving mesh relays via internal DNS fails if internal DNS depends on the overlay being already established. Additionally, sandbox processes can exfiltrate secrets via external DNS queries.
* **Mitigation**:
  - **Tier 0 Bootstrap**: Hardcoded/local fallback seed contains literal IP addresses, zero DNS dependency.
  - **DNS Leak Prevention**: Agent sandboxes execute with remote DNS resolution enforced (`socks5h`), routing all names through the internal proxy filter.

### 3.5 Blind Spot 5: Rate Limiting & DoS Defense (Noisy Neighbors)
* **Risk**: A runaway local agent or compromised peer flooding the WireGuard tunnel exhausts memory and CPU on consumer/edge peers.
* **Mitigation**: nftables-level and userspace rate-limiting (token bucket filter) per peer public key, reserving priority bandwidth for consensus and health heartbeats.

---

## 4. BOUNDED TOPOLOGY: SUBNET ROUTING & INGRESS/EGRESS

### 4.1 Dual-Plane Subnet Routing
Nodes within the same physical rack or Proxmox hypervisor utilize native unencrypted L2 bridges for internal communication (MTU 1500/9000, sub-millisecond RTT). Designated **Subnet Routers** announce local LAN prefixes to the overlay, eliminating redundant N×N WireGuard meshes within local clusters.

### 4.2 Reverse Proxy Ingress (L7)
Nodes possessing static public IPs (VPS, Baremetal) dynamically assume the `IngressProxy` capability:
- Terminate public TLS (Automated ACME / Cloudflare Origin).
- Authenticate requests via CYNIC admission tokens.
- Reverse-proxy internally across the mesh plane to target service endpoints.

### 4.3 SOCKS5 Userspace Sandbox Proxy
The `cynic-network` domain exports a pure SOCKS5 connection engine:
- Injected into agent sandboxes (`ALL_PROXY=socks5h://127.0.0.1:1080`).
- Enforces strict ACLs: blocks RFC 1918 management interfaces, logs outbound domain destinations into evidence journals, rejects unauthorized egress.

---

## 5. CRATE MAPPING & REFACTORING ROADMAP

```text
cynic-network (L1 Domain Core — Zero-I/O):
├── types.rs              — Plane, NodeAddress, NetworkPath, Telemetry
├── discovery.rs          — Pure drift evaluation, discovery reports
├── firewall_policy.rs    — nftables rendering with MSS clamping and rate limiting
├── mesh_plan.rs          — Pure WireGuard/Headscale topology plan generator
├── dns.rs                — Corefile and hosts generator
├── socks5/               — Userspace SOCKS5 protocol engine and policy filter
└── ports/
    ├── observation.rs    — NetworkObservationPort trait
    ├── discovery.rs      — NetworkDiscoveryProbePort trait
    └── mesh.rs           — MeshTransportPort, MeshCoordinationPort traits

cynic-host (L1 Substrate Adapters):
└── adapters/network/
    ├── local.rs          — /proc, ip, nftables host execution
    └── remote.rs         — ArgvRunner SSH executor

cynic-node (L3 Orchestration):
└── network/
    ├── actuator_runner.rs — Executes privileged cynic-network-actuator
    ├── sync_loop.rs       — Periodic mesh convergence and watchdog
    └── proxy_daemon.rs    — Hosts the local SOCKS5 listener for sandboxes
```
