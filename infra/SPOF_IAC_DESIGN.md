# SPOF Industrialisation — IaaS/IaC Design (V4)

Status: design doc avant code — audit existant + principes CYNIC — figé pour câblage StorageTopologyPort + preuve

## 1. Audit existant (live 2026-08-31, preuves file:line)

| Service | SPOF actuel | Preuve | Mode | Risque | Atténuation existante |
|---------|-------------|--------|------|--------|------------------------|
| Storage SurrealDB | `url: String` single endpoint | `infra/config_storage.rs:9`, `bootstrap/storage.rs:39` `SurrealHttpStorage::init(storage_config)` | 1 url, pas de `urls: Vec` | CRITIQUE: toutes mutations `store_verdict/claim/finalize/PoH/leases` échouent jusqu'au retour même endpoint | `ReconnectableStorage` RwLock + `NullStorage` fail-closed (writes Err, audit continue) + reconnector background 30s — mais repointe même url |
| Coord lease | `LeaseGuard` mémoire + `.agent-claims.json` rev file | `coordination_lease.rs:11` types purs sans store durable, audit-coordination-claims gap Critical | mémoire | CRITIQUE: restart → perte claims | `LocalAgentWorkloadStore` persiste workloads mais pas `coord_claim_path` |
| Fleet discovery | `infra/registry.json` single file + `/etc/cynic/registry.json` | `runtime_registry.rs:DEFAULT_REGISTRY_PATH` | 1 file | MAJEUR: 1 file corrompu → fleet aveugle | 4 modes (Explicit/Env/Installed/Dev) via `CYNIC_REGISTRY` env per-env |
| Inference queue | 1 slot per Dog, `BinaryHeap` | `inference_queue.rs:81` `capacity: u32` | 1 slot | MAJEUR: D19 déjà clos (dropped compté, purge dead, orphan reclaim) | bounded + priority + dropped AtomicU32 |
| Mesh | 1 provider synapse | `network.rs:129` `registry_path: String` | 1 provider | MAJEUR | non industrialisé |

Tous: 1 process, 1 DB, 1 endpoint — dette-taxonomie Infra1/Coord1/GC1

## 2. Principes CYNIC (AGENTS.md) — filtres avant code

1. **Desired→Observe→Reconcile→Bounded Action→Read-back Proof** (coherence pattern) — chaque terme a owner concret; sans proof pas de convergence
2. **Truth flows upward; intent flows downward** — registry dirige, health remonte par proof, jamais l'inverse (Truth1)
3. **Policy portable; substrate injecté** — budget/policy typés, adapters (systemd/process, Surreal/Memory, local/SSH) injectés, pas hardcodés
4. **Declarative desired + observed proof, not blind IaC** — un `.json` désiré sans observation+proof n'est pas IaC
5. **One boundary one reason (§481)** — storage, coord, fleet, inference chacun son crate/port, pas de monolithe
6. **Adapters disposable; ports/proofs durables** — remplacer Surreal par Memory ne doit pas recréer modèle
7. **Unknown is data** — single endpoint non répliqué = residual `single_endpoint_no_replication` explicit, pas 0
8. **Host 4 layers** (Physical/Volume/Usage/Pressure) — chaque famille a owner par couche, lower manquante = explicit unknown
9. **Organic always wins** — le compass propose, n'impose pas
10. **Design gate 6Q** avant code — desired, observe port, reconcile pur, bounded argv, proof, owner

## 3. Desired state déclaratif (IaaS) — DÉCOUPLÉ client vs cible (3.7-flash)

Fichiers SSOT scellés dans `contracts/foundation.json` (comme `cynic.dev.debt_audit.v1`):

`infra/storage_topology.json` (cible IaaS, `cynic.storage_topology.v1`):
```json
{
  "schema": "cynic.storage_topology.v1",
  "engine": { "backend": "standalone_process", "cluster_mode": "none" },
  "target_topology": { "replicas": 1, "residual_risk": "single_endpoint_no_replication" },
  "budget": { "wall_time_ms": 5000, "cpu_ms": 5000 }
}
```
`infra/storage_client_policy.json` (politique client, `cynic.storage_client_policy.v1`, consommée par `cynic-kernel`):
```json
{
  "schema": "cynic.storage_client_policy.v1",
  "client_policy": {
    "endpoints": ["http://127.0.0.1:8000"],
    "failover_mode": "ordered_fallback",
    "write_quorum": 1,
    "timeout_ms": 5000
  }
}
```
Cible IaaS (`replicas`, `engine.cluster_mode`) ≠ politique client (`endpoints` ordonnés + `failover_mode` + `write_quorum`). `cynic-kernel` reste consommateur passif d'une liste ordonnée avec fallback, pas propriétaire de topologie. Valeurs injectées via `CYNIC_STORAGE_TOPOLOGY_PATH` / `CYNIC_STORAGE_CLIENT_POLICY_PATH` (comme `CYNIC_REGISTRY`), pas hardcodées. Pour `raft` multi-nœuds, `engine.backend` doit être `tikv` (Surreal fichier/mémoire ne réplique pas par `surreal sql` seul) — évite AddReplica irréalisable.

Même découplage pour `coord_topology.json` / `coord_client_policy.json`, `fleet_topology.json` — un désiré par service, pas N configs divergentes.

## 4. Observe port (pur, injecté)

Types dans `cynic_protocol::storage_topology::{StorageTopologyDesired, StorageTopologyObserved, StorageTopologyReport}` et `cynic_protocol::storage_client_policy::{StorageClientPolicy}` (jamais dans CLI). `cynic_protocol` scinde `StorageTopologyDesired` (cible IaaS) et `StorageClientPolicy` (liste ordonnée endpoints + failover) — `cynic-kernel` ne dépend que de `StorageClientPolicy`, pas de topologie IaaS.

Parsers purs dans `cynic_host::domain::storage::probe` (pas d'I/O). Ports:

```rust
pub trait StorageTopologyObservationPort { fn observe_topology(&self) -> StorageTopologyObserved; }
pub struct LocalStorageTopologyAdapter { /* reads /proc, /sys, argv */ }
pub struct RemoteStorageTopologyAdapter<R: ArgvRunner> { /* SSH, injectable */ }
pub struct FleetStorageTopologyObserver<R> { /* rollup → FleetStorageTopologyReport to cynic.fleet_storage.v1 */ }
```

Chaque famille reuse 4 layers: Physical (NIC/disk), Volume (mount/route), Usage (bytes), Pressure (PSI/drops).

## 5. Reconcile pur

`reconcile_storage_topology(desired: &StorageTopologyDesired, observed: &StorageTopologyObserved) -> StorageTopologyPlan`

Règles: si `observed.endpoints.len() < desired.replicas` → `plan.actions.push(AddReplica)`, si `leader == None` → `ElectLeader`, si `poll_cost > budget` → `Backpressure` (Layer4). Jamais de mutation silencieuse.

## 6. Bounded action

Plan → `argv` explicite (`surreal sql --endpoint ...`), `privilege: bool`, `mutates: true`, `--yes/--apply`, `rollback_intent: "restore single endpoint"`, `stop_condition: proof bundle blocked`. Via `ArgvRunner` injecté, pas `Command::new` hardcodé.

## 7. Proof bundle (read-back)

Après action, `StorageTopologyReport { schema, desired_digest, observed_digest, plan_digest, actions_applied, residual_unknowns: ["single_endpoint_no_replication" si replicas<desired, "storage_degraded" si NullStorage, "poll_cost_exceeded_budget" si >budget], proof_ref }` versionné `cynic.storage_topology_report.v1`, persisté, stale check `observed_at` vs `now`.

## 8. Plan d'industrialisation (vagues V4)

V4a: `cynic_protocol::storage_topology` types + `cynic_host::domain::storage::probe` parsers (pre-inventaire surfaces existantes, extend existing, pas dupliqué)
V4b: `StorageTopologyObservationPort` + Local/Remote adapters + `FleetStorageTopologyObserver`
V4c: `reconcile_storage_topology` pur + `StorageTopologyPlan` + CLI `cynic storage topology --json` thin projection
V4d: bounded apply + proof bundle + rollback + `cynic dev storage-audit --json` (même pattern debt-audit: table-driven, pure probes, HashSet covered, blast 1/N/∞)

Même IaaS transposé à coord (`coord_topology.json` + CoordPort) et fleet — un pattern, N adapters.

Preuve: `cynic dev debt-audit --json` Infra1 passe de `avéré` → `blocked` (single endpoint residual) → `ok` quand replicas==desired et proof frais. `make check` per-crate, `RUST_MIN_STACK` Build1.

Fichiers: ce doc + `.hermes/plans/spof-iaas-v4.md` + futurs `crates/cynic-protocol/src/storage_topology.rs`, `crates/cynic-host/src/domain/storage/probe.rs`.
