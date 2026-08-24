# Code Duplication Registry — CYNIC

**Critère** : Même logique implémentée en 2+ endroits, ou code copié-collé
**Référence** : AGENTS.md §6 "One canonical decision source per concern"

---

## ACTIF (duplication confirmée)

### DUP-001: OS Health Probe — Double Implémentation
- **Localisations** :
  1. `cynic-host/src/os_health/probes.rs` — `read_available_mem_mib()`, `read_cpu_jiffies()`, `read_disk_usage()`, `read_network_bytes()`
  2. `cynic-host/src/peer_resolver_mesh.rs:15` — `use crate::os_health::probes;` (import OK mais logique dupliquée ?)
- **Analyse** : `peer_resolver_mesh.rs` IMPORTE `probes.rs` — pas de duplication réelle, MAIS `MeshPeerObserver` réimplémente l'agrégation health au lieu d'utiliser un port
- **Résolution** : Déjà résolu par import — vérifier qu'aucune logique n'est copiée
- **Status** : `verified_resolved` (import used, pas de copy-paste)

### DUP-002: Network Preflight — Host vs Node
- **Localisations** :
  1. `cynic-host/src/network_actuation.rs` (928 lignes) — `preflight_network()`, `apply_network_plan()`
  2. `cynic-node/src/network_repair.rs` (425 lignes) — `repair_network()`, `validate_network_state()`
  3. `cynic-node/src/network_mesh_apply_plan.rs` (730 lignes) — `apply_mesh_plan()`
- **Duplication** : Validation CIDR, gateway reachability, interface state — logique similaire 3x
- **Canonique** : `cynic-host::network::preflight` (host = decide)
- **Action** : Node appelle host port, ne réimplémente pas
- **Status** : `observed`

### DUP-003: Disk Observation — Probe + Observer + Adapter
- **Localisations** :
  1. `cynic-host/src/domain/disk/probe.rs` — `DiskProbe`, `probe_disk()`
  2. `cynic-host/src/fleet_disk_observer.rs` — `FleetDiskObserver`, `observe_fleet_disk()`
  3. `cynic-host/src/adapters/local_disk.rs` — `LocalDiskAdapter`, `observe_disk()`
  4. `cynic-host/src/adapters/remote_disk.rs` — `RemoteDiskAdapter`, `observe_disk()`
- **Duplication** : `probe_disk()` logic copiée dans adapters au lieu d'être partagée
- **Canonique** : `domain/disk/probe.rs` = pure probe logic, adapters = I/O only
- **Action** : Adapters délèguent à probe.rs pure functions
- **Status** : `observed`

### DUP-004: Agent Session / Workload — Kernel vs Node vs Protocol
- **Localisations** :
  1. `cynic-kernel/src/bootstrap/agent_session.rs` — session lifecycle
  2. `cynic-protocol/src/agent_workload/delivery.rs` — delivery types
  3. `cynic-protocol/src/agent_workload/lifecycle.rs` — workload lifecycle
  4. `cynic-node/src/usecases/agent_workload_uc.rs` (si existe) — node use case
  5. `cynic-node/src/local_agent_session.rs` (547 lignes) — node local session
- **Duplication** : Session state machine, lease logic, heartbeat — 3+ implémentations
- **Canonique** : `cynic-protocol::agent_workload` = types + pure state machine
- **Action** : Kernel et Node utilisent protocol types, pas de reimplémentation
- **Status** : `observed`

### DUP-005: Fleet Heartbeat — Protocol vs Host
- **Localisations** :
  1. `cynic-protocol/src/fleet_heartbeat.rs` (19310 lignes) — schema + types + store traits
  2. `cynic-host/src/fleet_heartbeat.rs` (2888 lignes) — port + store impl
  3. `cynic-host/src/fleet_health_observer.rs` (5086 lignes) — observer
- **Duplication** : Types définis dans protocol MAIS store traits redéfinis dans host
- **Canonique** : Protocol = types + port traits, Host = impl
- **Action** : Déplacer `FleetHeartbeatStorePort` vers protocol, host impl seulement
- **Status** : `observed`

### DUP-006: Peer Resolution — Protocol vs Host vs Node
- **Localisations** :
  1. `cynic-protocol/src/peer_resolver.rs` (5886 lignes) — `PeerEndpoint`, `SignedPeerEndpoint`, `resolve_peers()` pure
  2. `cynic-host/src/peer_resolver_mesh.rs` (565 lignes) — `MeshPeerResolver` + `MeshPeerObserver`
  3. `cynic-node/src/usecases/federated_mesh_resolver.rs` — node use case
- **Duplication** : `PeerEndpoint` construction, signature verification — logique partagée
- **Canonique** : Protocol = types + pure resolve, Host = observer port, Node = use case
- **Note** : `cynic-protocol/src/peer_resolver.rs` a tests avec `available_mem_mib: 0` — déjà aligné
- **Status** : `observed` (architecture OK, vérifier impl)

---

## RÉSOLU (depuis audit précédent)

| ID | Duplication | Résolution | Preuve |
|----|-------------|------------|--------|
| DUP-OLD-001 | `resolve_peers` / `resolve_all_peers` 95% identiques | Déléguent à `resolve_peers_filtered` | `peer_resolver_mesh.rs:96-116` |
| DUP-OLD-002 | `fallback_peers()` retournait unsigned | Retourne `vec![]` fail-closed | `peer_resolver_mesh.rs:111` |
| DUP-OLD-003 | `observed_at_unix_s: 0` hardcodé | `SystemTime::now()` | `peer_resolver_mesh.rs:100-103` |
| DUP-OLD-004 | `emergence_state: None // TODO` | Implémenté via `MeshPeerResolver` | `federation_runtime.rs:226` |
| DUP-OLD-005 | `fleet_heartbeat.rs` orphelin | Utilisé par `fleet_health_observer.rs` | `fleet_health_observer.rs` imports |

---

## Pattern Anti-Duplication (AGENTS.md §6)

```rust
// 1. Canonical decision source = pure module dans protocol
// cynic-protocol/src/network/preflight.rs
pub fn validate_cidr(cidr: &str) -> Result<Cidr, Error> { ... }
pub fn check_gateway_reachable(iface: &str, gw: IpAddr) -> Result<bool, Error> { ... }

// 2. Host adapters utilisent les fonctions pures
// cynic-host/src/network_actuation.rs
use cynic_protocol::network::preflight::{validate_cidr, check_gateway_reachable};

// 3. Node use cases délèguent au host port
// cynic-node/src/usecases/network_uc.rs
use cynic_host::network::NetworkActuationPort;
```

---

## Priorisation

| Entrée | Criticité | Effort | ROI | Ordre |
|--------|-----------|--------|-----|-------|
| DUP-002 (Network) | HIGH | 2-3j | 8/10 | P1-1 |
| DUP-003 (Disk) | HIGH | 2-3j | 7/10 | P1-2 |
| DUP-004 (Agent Session) | HIGH | 1-2sem | 7/10 | P2-1 |
| DUP-005 (Fleet Heartbeat) | MEDIUM | 1-2j | 6/10 | P2-2 |
| DUP-006 (Peer Resolution) | MEDIUM | 1-2j | 5/10 | P3-1 |