# Fleet Network — Canonical Architecture (first principles, data-centric)

Status: `proposed` — awaiting operator validation before any code moves.
Schema: `cynic.fleet_network_architecture.v1`
Scope: mesh overlay, fleet membership, network observation, network actuation,
network policy. Supersedes the scattered mesh types listed in §2.

---

## 1. First principles — what the fleet network must do

Before naming any crate or technology, state the irreducible job. A fleet
network has exactly six obligations. Everything else is an implementation
choice.

| # | Obligation | Question it answers | Failure mode if absent |
|---|---|---|---|
| 1 | **Identity** | Who is this node, cryptographically? | IP becomes identity; DHCP/renumber breaks trust |
| 2 | **Reachability** | Can node A reach node B, encrypted? | Manual routes, plaintext on the wire |
| 3 | **Membership** | Who is in the fleet *now*? | Stale peer lists, ghost nodes |
| 4 | **Policy** | Who may talk to whom, on what port? | Flat trust, lateral movement |
| 5 | **Configuration** | What is the desired state? | Hand-edited configs, drift |
| 6 | **Observation** | What is the actual state? | Silent divergence, no read-back |

**Data-centric law.** Obligations 5 and 6 are *data*, not code. The desired
state is a document; the observed state is a document; reconciliation is a
pure function `reconcile(desired, observed) -> plan`. No node computes its
own truth — truth is captured, canonicalized, then compared. This is the same
Capture → Canonical → Model → Projection law already ratified in
`docs/design/data-centric-framework-architecture.md`, applied to the network.

**Corollary — no SPOF.** No single node may be the only holder of any of the
six obligations. The Headscale control plane is *replaceable*: if it dies, the
WireGuard overlay keeps forwarding (obligation 2 survives), membership is
served from the last signed snapshot (obligation 3 degrades to read-only), and
the kernel reconciles on restart. A control plane that is a SPOF is not a
control plane, it is a hostage.

---

## 2. Cartography — the current state (evidence)

### 2.1 Type fragmentation

**158 types** matching `fleet|mesh|wireguard|network` across **10 crates**:

| Crate | Layer | Count | What it actually holds today |
|---|---|---|---|
| `cynic-protocol` | L0 | 26 | `FleetHeartbeat`, `FleetRole`, `FleetNodeStatus`, `FleetMemoryNode`, `FleetDiskNode`, `FleetCpuNode`, `NetworkModel`, `FederatedMeshNodeStatus` |
| `cynic-core` | L0 domain | 10 | `MeshNode`, `MeshRoute`, `NetworkMeshPlan`, `NetworkMeshPeer`, `WireguardMeshAssessment` |
| `cynic-network` | L1 | 29 | `MeshRole`, `MeshSovereignty`, `NetworkReport`, `PhysicalInterface`, `InterfacePressure`, `WireguardPeerMetrics`, `FleetNetworkNode` |
| `cynic-host` | L1 | 48 | `NetworkActuation*` (10), `MeshPeerResolver`, `FleetNetworkObserver`, `FleetWatchdogObserver`, `WgReadbackObserver`, `DesiredWgPeer`, `RegistryNetwork*` (3), `MeshObservation`, `MeshPeerObserver` |
| `cynic-node` | L3 | 35 | `MeshTopology`, `PeerIdentity`, `DesiredMeshPeer`, `MeshCoordinationReport`, `MeshMember`, `MeshJoinGrant`, `NetworkSyncPlan`, `WireguardPeerSpec`, `WireguardHandshakeObservation`, `CanonicalMeshStatus`, `MeshRuntimeObservation` |
| `cynic-kernel` | L3 | 6 | `NetworkDetails`, `FleetDetails`, `FleetNodeObservation`, `FleetInventory*` |
| `cynic-discovery` | L2 | 1 | `FleetCapabilities` |
| `cynic-consensus` | L2 | 2 | `FleetConsensusStatus`, `FleetEpochConsensusState` |
| `cynic-observer` | L2 | 1 | `FleetMeshSource` |
| `cynic-federation` | L2 | 0 | created, empty |

### 2.2 Defects found (falsifiable)

**D1 — DAG inversion: `cynic-core` (L0) → `cynic-pkg` (L1).**
`cynic-core/src/domain/shared.rs:12` re-exports `SurfaceStatus`, `BoundaryKind`,
`LifecycleStatus`, `LifecycleDisposition`, `LifecycleProjection` from
`cynic_pkg::integration::lifecycle`. A pure domain leaf depending on a host
substrate crate. *Falsifier:* `cargo tree -p cynic-core` shows `cynic-pkg`.

**D2 — Vocabulary conflict in `infra/registry.json`, intersection ∅.**

| Source | Plane names |
|---|---|
| `network_policy.planes` | `service` (10.10.10.0/24, canonical), `admin` (192.168.100.0/24), `mesh` (10.20.0.0/24) |
| `nodes[].network` | `management` (192.168.100.x), `synapse` (10.10.10.x) |
| `nodes[].network_links[].plane` | `management` |

`synapse` and `service` are the same plane under two names; `management` and
`admin` are the same plane under two names. Any code that joins on plane name
silently matches nothing. *Falsifier:* grep for `"synapse"` and `"service"` in
the same expression — there is none, because the join is done by humans.

**D3 — Three mesh type families for one concept.**
`MeshNode` (core) / `NetworkMeshPeer` (core) / `MeshMember` (node) /
`FleetNetworkNode` (network) / `DesiredWgPeer` (host) all describe "a peer in
the overlay". Five names, one concept.

**D4 — WireGuard's declared desired state exists, but no reader points at it.**
`infra/verticals/network/intent.json` declares the WireGuard overlay properly:
`desired_state.wireguard.nodes[]` with `node_id`, `overlay_ip`, `endpoint_plane`,
`ssh_access`, `watchdog`, plus `overlay_cidr`, `listen_port`, `topology`,
`keepalive_s`, `key_storage`. This is a real desired-state document.

But **four production readers look for `/network_policy/wireguard/nodes`**:

| Reader | File:line | Behaviour on the real registry |
|---|---|---|
| `parse_registry_network_specs` | `cynic-node/src/usecases/network_sync_uc.rs:56` | `Err("missing_network_policy_wireguard_nodes")` |
| `resolve_overlay_peer_ips` | `cynic-node/src/watchdog.rs:186` | silent `Vec::new()` |
| discovery wg merge | `cynic-network/src/discovery.rs:262` | silent skip |
| firewall wg port binding | `cynic-network/src/firewall_policy.rs:211` | silent skip (port 44196 never opened) |

`infra/registry.json` has `network_policy` = `{canonical_service_plane, planes}`
only — **no `wireguard` key**. So in production every WireGuard sync fails or
silently no-ops, and the firewall rule for the overlay port is never emitted.

**Why the tests did not catch it:** the unit tests build inline JSON fixtures
(`network_sync_uc.rs:221`) that mirror `/network_policy/wireguard/nodes` — a
schema that was designed and never landed in the registry. The test asserts the
parser works on the fixture, which is true. Nothing asserts the fixture matches
the real document. *Falsifier:* `python3 -c "import json; print('wireguard' in
json.load(open('infra/registry.json'))['network_policy'])"` → `False`.

**D5 — Two crates were declared in a session that never landed.**
`cynic-federation` and `cynic-agent-execution` were created as empty crates in
an earlier session, then lost when the filesystem was corrupted. Neither exists
in the current workspace (`find` returns nothing; the 49 `Cargo.toml` members
contain neither). This is not an open defect — it is a warning against
declaring a crate before it has a responsibility. Per `pruning-policy.json`
`suspect_over_splitting`, an empty crate is a directory with a `Cargo.toml`.
Create them only when §3.3 shows a real owner is missing.

**D7 — Three documents claim network desired state, with no precedence rule.**

| Document | Plane names | WireGuard nodes | Declared authority |
|---|---|---|---|
| `infra/registry.json` `network_policy` | `service`, `admin`, `mesh` | **absent** | "runtime placement registry" |
| `infra/verticals/network/intent.json` `desired_state` | `synapse`, `management`, `overlay` | **present** (5 nodes) | `owner_crate: cynic-node` |
| `infra/network/topology.json` `planes` | `service`, … | — | `proposed_ssot_pending_operator_acceptance` |

Three names for the same plane (`service`/`synapse`; `admin`/`management`;
`mesh`/`overlay`), two WireGuard documents (one absent, one real), and the
document that admits it is only *proposed* is the one whose plane names the
readers use. Nothing in the repo states which wins. This is the root cause of
D2 and D4 together: the vocabulary drifted because the authority was never
declared, and the readers were written against a fourth, imagined schema.

**Resolution rule (proposed, §4.1):** `registry.json` is the runtime placement
SSOT (it is what nodes are shipped and what the bootstrap reads);
`intent.json` is the per-vertical *intent* document that the registry is
reconciled against. Plane vocabulary is owned by exactly one of them — the
operator picks which in §7.

---

## 3. Target architecture — bounded contexts

Apply the ratified law: *one responsibility, one owning crate*; *domain must not
depend on adapters*; *name states responsibility*.

### 3.1 Five bounded contexts (SoC)

| Context | Responsibility | Owns | Does NOT own |
|---|---|---|---|
| **Fleet identity & membership** | Who is in the fleet, their role, their key | Node identity, role, membership snapshot, heartbeats | IPs, routes, interfaces |
| **Mesh overlay** | Encrypted peer-to-peer reachability | Peer list, handshake state, route advertisement | Host firewall, DNS |
| **Network observation** | Read the actual state, I/O-free signatures | `NetworkReport`, interface/route/handshake snapshots | Any mutation |
| **Network actuation** | Apply the desired state, privileged | Plan → apply → read-back, firewall rules | Deciding *what* the state is |
| **Network policy** | Declare the desired state | Planes, CIDRs, ACLs, enrollment grants | Executing anything |

### 3.2 Hexagon per context

```
<context>/
├── domain/      pure: entities, value objects, invariants   (Zero-I/O)
├── ports/       traits the domain needs (no I/O imports)
├── adapters/    concrete: HTTP / argv / registry reader     (I/O lives here)
└── usecases/    orchestration (domain + ports, never adapters)
```

### 3.3 Crate placement (DAG-checked)

```
L0  cynic-protocol        constitutional laws
    ├── FleetRole, FleetHeartbeat, FleetNodeStatus
    ├── NetworkPlane, PlaneCidr, MeshProviderKind     (NEW: canonical plane enum)
    └── (NO mesh adapter types, NO observation types)

L0  cynic-core            domain: entities + ports
    ├── MeshNode, MeshRoute, MeshError                 (already there)
    ├── FleetMeshPort                                  (already there)
    ├── MeshTopology, PeerIdentity                     (migrated from cynic-node)
    └── FleetMembershipPort, NetworkObservationPort    (NEW ports)

L1  cynic-network         adapters + observation types
    ├── HeadscaleClient (FleetMeshPort impl)           (already there)
    ├── LocalNetworkAdapter, NetworkReport             (already there)
    ├── WireguardOverlayAdapter                        (NEW: wg config render + apply plan)
    └── (re-exports core types; defines NO mesh entity)

L1  cynic-host-probes     host probe parsers (pure)
    └── wg show / ip route / ss parsers                (migrated from cynic-host)

L2  cynic-discovery       fleet membership discovery
    └── FleetCapabilities, DiscoveredNode              (already there)

L2  cynic-consensus       epoch consensus over membership
    └── FleetEpochConsensusState                       (already there)

L3  cynic-node            orchestration: use cases only
    ├── mesh_sync_uc, network_sync_uc                  (consume ports, no types)
    └── (MeshTopology, DesiredMeshPeer migrate OUT)

L3  cynic-kernel          fleet rollup projection
    └── FleetInventory* (consume cynic-network reports)
```

**Removals:**
- `cynic-core` loses its `cynic-pkg` dependency (D1). `SurfaceStatus` and
  friends move to `cynic-protocol` (they are constitutional lifecycle states,
  not package concerns).
- `cynic-host` loses all 48 mesh/network types to `cynic-network` +
  `cynic-host-probes`.
- `cynic-node` loses its mesh *types* (keeps its use cases).

---

## 4. Canonical network policy (technologies and systems)

### 4.1 Planes — one name each, declared once

Resolve D2 by making `network_policy.planes` the **single naming authority**.
Node-level `network_links[].plane` must reference these names, never synonyms.

| Plane | CIDR | Canonical | Purpose | Technology |
|---|---|---|---|---|
| `service` | 10.10.10.0/24 | **yes** | CYNIC service plane (kernel, forge, storage, builder) | routed L2/L3, static |
| `admin` | 192.168.100.0/24 | no | bare-metal management, hypervisor control | physical LAN |
| `mesh` | 10.20.0.0/24 | no | encrypted overlay, node-to-node | WireGuard (Noise_IK) |
| `fleet` | 100.64.0.0/10 | no | Headscale-assigned node addresses | WireGuard + Headscale |

Renames required in `registry.json`: `synapse` → `service`, `management` →
`admin`. A migration shim is forbidden — rename the data, fix the readers.

### 4.2 Technology decisions

| Layer | Choice | Rejected alternative | Why |
|---|---|---|---|
| Overlay transport | **WireGuard** | IPsec, OpenVPN | Kernel-space, ~4k LOC, auditable, no userspace daemon SPOF |
| Control plane | **Headscale** (sovereign) | Tailscale SaaS | SaaS sends telemetry to a third party; violates sovereignty |
| Node discovery | **Signed registry snapshot + mDNS on the LAN** | DHT, central directory | Registry is already the declared SSOT; mDNS for local liveness |
| Policy language | **JSON ACL (Headscale-compatible) derived from `registry.json`** | Hand-written ACL file | Hand-written drifts; derived cannot |
| DNS | **MagicDNS base domain `cynic.ts`** | External resolver | Resolver must not be a SPOF; `1.1.1.1` is fallback only |
| Key storage | **`cynic-secrets` (`SecretClass::HeadscaleApiKey`)** | env var only, plaintext file | Capability-based; value never reaches an agent |
| Config application | **Declared desired state → pure render → privileged apply → read-back** | Imperative `wg set` scripts | No read-back = no proof = no config |

### 4.3 Data-centric reconciliation loop

```
  registry.json (desired)                 probes (observed)
        │                                        │
        ▼                                        ▼
  ┌─────────────┐                         ┌──────────────┐
  │ FleetPolicy │                         │ NetworkReport│
  │  (L0 types) │                         │  (L1 types)  │
  └──────┬──────┘                         └──────┬───────┘
         │                                       │
         └──────────► reconcile() ◄──────────────┘
                    (pure function)
                          │
                          ▼
                   NetworkApplyPlan
                          │
                          ▼
              ┌───────────────────────┐
              │ privileged actuation  │  (explicit --apply)
              └───────────┬───────────┘
                          ▼
                   read-back proof
                          │
                          ▼
                compare(desired, observed)
                          │
              ┌───────────┴───────────┐
              ▼                       ▼
          converged                 drift → plan
```

`reconcile` is pure: same inputs, same plan, no I/O. That makes it testable
without a fleet, and makes drift a *data* comparison rather than a heuristic.

---

## 5. No-SPOF analysis

| Component | SPOF today? | Mitigation |
|---|---|---|
| Headscale (container 100) | **yes** — single control plane | WireGuard keeps forwarding if it dies; membership served from last signed snapshot; documented restore path |
| `cynic-kernel` (0.0.0.0:3030) | **yes** — single kernel | Fleet discovery tolerates absence (`isolated`, explicit unknown); no hard dependency |
| Forgejo (10.10.10.101) | **yes** — single forge | GitHub `origin` is the declared relay |
| SurrealDB (10.10.10.103) | **yes** — single store | Declared `storage_primary`; replica is future work, reported as `unknown` |
| Registry file | no — declarative, versioned in git | Any node can read it |
| DNS (`1.1.1.1`) | no — fallback | MagicDNS primary |

**Honest statement:** Headscale, kernel, forge and storage are each a SPOF
today. The architecture makes their *absence an explicit unknown* rather than a
silent failure — that is the achievable guarantee now. Removing the SPOF is a
separate, larger effort and is **not** claimed here.

---

## 6. Unification plan (phased, one proof per phase)

| Phase | Action | Proof |
|---|---|---|
| **F0** | **Stop the silent failure.** All four readers point at `/network_policy/wireguard/nodes`; make each return an explicit typed error (or a named `unknown`) instead of `Vec::new()` when the path is absent. No behaviour change yet — only the absence becomes visible. | a run on the real registry emits 4 explicit `unknown`, not 4 silent empties |
| **F1** | Resolve D7: declare the single plane-vocabulary owner; rename the losing document's names to match; fix readers | grep finds zero mixed-vocabulary plane joins |
| **F2** | Resolve D4: point the four readers at the real desired-state document (whichever §7 selects), or move `wireguard.nodes` into the registry | `cynic network sync` on a real node resolves 5 peers, not 0 |
| **F3** | Resolve D1: move `SurfaceStatus` family from `cynic-pkg` to `cynic-protocol`; drop `cynic-pkg` from `cynic-core` | `cargo tree -p cynic-core` has no `cynic-pkg` |
| **F4** | Resolve D3: migrate `MeshTopology`, `PeerIdentity`, `DesiredMeshPeer` from `cynic-node` to `cynic-core`; delete duplicates | `cynic-node` declares zero mesh entity |
| **F5** | Move the 48 `cynic-host` network types to `cynic-network` + `cynic-host-probes` | `cynic-host` mesh type count = 0 |
| **F6** | Add `FleetMembershipPort` + `NetworkObservationPort` to `cynic-core`; `WireguardOverlayAdapter` to `cynic-network` | ports compile, adapters impl them, tests pass |
| **F7** | Derive ACLs from the chosen authority instead of a hand-written `acls.json` | regenerating ACLs is idempotent; diff is empty |
| **F8** | Wire `reconcile()` (pure) + `cynic network apply --apply` (privileged) + read-back | drift detected and repaired on a real node |

**F0 before everything.** Every later phase touches the readers; doing F0 first
means each later step is observable rather than silently no-op.

---

## 7. Open decisions for the operator

1. **Plane rename** — `synapse`→`service`, `management`→`admin` (align on
   `network_policy`), or the reverse (align on node-level names)?
2. **`cynic-federation` / `cynic-agent-execution`** — fuse into an existing
   crate, or give them a real responsibility now?
3. **Headscale SPOF** — accept the documented restore path, or invest in a
   second control plane?
4. **ACL source** — derive from `registry.json` (single authority), or keep a
   separate `acls.json` that references it?
