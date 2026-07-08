# Organ-Wireguard (V3 Mesh Networking) — Legacy Specification

*Status: Legacy design record. This document describes the superseded V3
WireGuard-centric direction that is being absorbed into `cynic-node`,
`organ-fleet`, and the bounded `network-actuator` path. It is not the canonical
active authority for current network runtime ownership.*

---

## 1. Historical Objectives

- **Absolute Sovereignty**: Zero external telemetry or dependency on third-party SaaS (Tailscale) or dedicated orchestration daemons (Headscale).
- **Direct P2P Tunnels**: Dynamic mesh mapping with automated UDP hole punching for NAT traversal.
- **Dynamic Topology**: Automatic peer discovery and configuration reloading via the CYNIC kernel.
- **Local Private Keys**: Private keys are generated on-device and never leave the node. Only public keys and current endpoint addresses are stored in SurrealDB.

Current canonical split:
- `infra/registry.json:network_policy` owns desired network policy.
- `cynic-node` owns node-local runtime observation and bounded repair planning.
- `organ-fleet` owns the active mesh/DNS operator surfaces.
- `network-actuator` is the bounded application path for root-capable mutations.

---

## 2. Historical Architecture Overview

The model below describes the legacy design where `organ-wireguard` was meant
to run as the primary native mesh daemon on every node. That is no longer the
canonical runtime ownership model.

```
                  ┌──────────────────────┐
                  │ CYNIC Kernel Primary │
                  │     (SurrealDB)      │
                  └──────────┬───────────┘
                             │ (REST/gRPC)
                             ▼
                    ┌──────────────────┐
                    │  Organ-Wireguard │ (Runs on every node)
                    │      Daemon      │
                    └────────┬─────────┘
                             │ (Rewrites wg0.conf & reloads)
                             ▼
                    ┌──────────────────┐
                    │ Linux WireGuard  │
                    │    (Kernel)      │
                    └──────────────────┘
```

---

## 2.1 Registry-Governed IPAM

`infra/registry.json:network_policy` is the source of truth for Synapse IPAM.
The canonical service plane is `synapse`; agents and organs consume services
through `*.synapse.cynic` names, not LAN or Proxmox guest addresses.

Rules:
- Every routable node must have a registry Synapse reservation before it is
  provisioned.
- `organ-wireguard` must refuse duplicate Synapse addresses, addresses outside
  the declared CIDR, and node addresses that do not match their reservation.
- LAN, DHCP, and Proxmox guest addresses are provider evidence only. They may be
  used for bootstrap diagnostics, but they are not stable service contracts.
- Storage consumers must target `organ-surrealdb.synapse.cynic:8000` once the
  storage node has runtime proof. `192.168.*`, `10.10.*`, or host-local
  addresses are not valid consumer configuration.

This prevents agents from selecting plausible-looking addresses from observed
state when the network policy is incomplete.

---

## 2.2 Canonical MCP Operations

The portable agent interface for infrastructure is
`axioms/infrastructure-mcp-v1.json`. Today, `organ-wireguard` does not own the
canonical network/IPAM subset. The current split is:
- `cynic-node` for node-local network evidence and diagnostic truth
- `organ-fleet` for canonical mesh/DNS operator surfaces
- provider adapters such as Proxmox stay implementation details behind fleet operations

Canonical network operations:
- `network.ipam.check`
- `network.ipam.reserve`
- `network.ipam.release`
- `network.mesh.bootstrap`
- `network.mesh.status`
- `network.mesh.dns.render`
- `network.mesh.peer.rotate`

Canonical fleet/service operations that the legacy design depended on:
- `fleet.node.status`
- `fleet.node.provision`
- `fleet.node.exec`
- `service.deploy`
- `service.status`
- `service.health`

Current raw Proxmox MCP tools are not enough for this contract because they are
provider-specific and `cynic_proxmox_exec` does not cover LXC execution. Until
the canonical surface exists, raw provider tools may be used only as audited
bootstrap evidence or emergency actuation, never as the declared final
architecture.

---

## 3. Database Schema (SurrealDB)

The dynamic topology is coordinated using two tables in SurrealDB: `mesh_nodes` and `mesh_routes`.

### `mesh_nodes`
Stores the active identity, public key, and resolved endpoint of each node.
```surrealql
DEFINE TABLE mesh_nodes SCHEMAFULL;
DEFINE FIELD name ON TABLE mesh_nodes TYPE string;
DEFINE FIELD public_key ON TABLE mesh_nodes TYPE string;
DEFINE FIELD mesh_ip ON TABLE mesh_nodes TYPE string;         // e.g., "10.20.0.3/24"
DEFINE FIELD external_endpoint ON TABLE mesh_nodes TYPE string; // e.g., "198.51.100.4:51820"
DEFINE FIELD last_seen ON TABLE mesh_nodes TYPE datetime;
```

### `mesh_routes`
Specifies which nodes should peer with each other. By default, CYNIC generates a full mesh or star-mesh topology.
```surrealql
DEFINE TABLE mesh_routes SCHEMAFULL;
DEFINE FIELD source_node ON TABLE mesh_routes TYPE record<mesh_nodes>;
DEFINE FIELD target_node ON TABLE mesh_routes TYPE record<mesh_nodes>;
DEFINE FIELD allowed_ips ON TABLE mesh_routes TYPE array<string>;
```

---

## 4. Node Lifecycle & Key Rotation

```mermaid
sequenceDiagram
    participant Node as New Node (organ-wireguard)
    participant Kernel as CYNIC Kernel (SurrealDB)
    participant Peer as Active Peers

    Note over Node: 1. Generate wg key pair locally
    Node->>Node: wg genkey > private.key; wg pubkey < private.key > public.key
    
    Note over Node: 2. Query STUN for external port
    Node->>Node: Resolve external IP & NAT type
    
    Node->>Kernel: 3. Register public_key & external_endpoint
    Kernel-->>Node: Confirm IP assignment (e.g. 10.20.0.4)
    
    Note over Node: 4. Poll active peers from database
    Kernel->>Node: Send peer list
    
    Note over Node: 5. Generate /etc/wireguard/wg0.conf
    Node->>Node: Write peers config & wg-quick up wg0
    
    Note over Node, Peer: 6. UDP Hole Punching Handshake
```

---

## 5. NAT Traversal & Hole Punching

To traverse symmetric or restricted cone NATs without a centralized Headscale DERP server:
1. **STUN Discovery**: The `organ-wireguard` daemon periodically queries public or self-hosted STUN servers (configured in `registry.json`) to discover its external UDP address and port.
2. **Endpoint Advertising**: The discovered external endpoint is updated in the `mesh_nodes` table.
3. **UDP Keep-Alives**: WireGuard's `PersistentKeepalive` is set to `25` seconds on all peers to keep NAT translation tables open.
4. **Active Hole Punching**: If direct connection fails, `organ-wireguard` triggers a heartbeat loop by sending raw UDP packets to the target's advertised external port, forcing the local NAT router to open a return path.

---

## 6. Security & Sandboxing

- **Decoupled Key Management**: The private key is stored exclusively in `/etc/wireguard/private.key` with `0600` permissions.
- **Least Privilege execution**: `organ-wireguard` runs as a system service. It uses `setcap cap_net_admin+ep` on its helper binary or uses `sudo` with restricted rules in `/etc/sudoers.d/organ-wireguard` to allow *only* `/usr/bin/wg` and `/usr/bin/wg-quick` executions.
- **IPsec Fallback**: For environments where UDP/51820 is actively blocked by ISPs or firewalls, `organ-wireguard` will fallback to TCP encapsulation (e.g., using `wstunnel` or a WireGuard-over-TLS relay).

---

## 7. Ansible Layer 0 Bootstrap Boundary

As per the `CYNIC-INFRASTRUCTURE-ROADMAP.md` rules, Ansible's responsibility stops after the first boot:
1. Installs `wireguard` tools: `apt install -y wireguard wireguard-tools`
2. Generates initial `/etc/wireguard/wg0.conf` containing *only* the peer definition for the Kernel Primary (so it can join the initial network).
3. Installs and starts the `organ-wireguard` service.
4. From then on, any route change, peer addition, or key rotation is handled dynamically by `organ-wireguard` communicating with the SurrealDB cluster.
