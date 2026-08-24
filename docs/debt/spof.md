# SPOF Registry — CYNIC

**Criticité** : CRITIQUE = cluster total down | HIGH = fonction critique down | MEDIUM = dégradé

---

## CRITIQUE

### SPOF-001: Single Kernel Process
- **Fichier** : `crates/cynic-kernel/src/main.rs:33`
- **Composant** : `bootstrap::runtime_orchestrator::run_runtime()`
- **Impact** : Process crash = cluster total down (REST, WS, storage, coordination, inference)
- **Fréquence** : Toute panne kernel (OOM, panic, sigkill, bug)
- **Mitigation actuelle** : Aucune
- **Coût estimé** : 2-3 sem (leader election + hot standby + state sync)
- **Owner** : `cynic-kernel::bootstrap::runtime_orchestrator`
- **ROI** : 10/10 (évite downtime total)
- **Status** : `observed`

### SPOF-002: SurrealDB Single Instance
- **Fichier** : `crates/cynic-kernel/src/bootstrap/storage.rs:39-49`
- **Composant** : `SurrealHttpStorage::init()` — seule implémentation `StoragePort`/`CoordPort`
- **Impact** : SurrealDB down → `NullStorage` + `UnavailableCoord` = read-only dégradé, pas de claims/leases/heartbeat
- **Fréquence** : DB maintenance, network partition, OOM, disk full
- **Mitigation actuelle** : `ReconnectableStorage` (auto-reconnect) + `NullStorage` fallback
- **Coût estimé** : 1-2 sem (SurrealDB cluster / etcd / Consul / réplication)
- **Owner** : `cynic-kernel::storage::reconnectable` + `bootstrap::storage`
- **ROI** : 9/10 (stockage = vérité du runtime)
- **Status** : `observed`

### SPOF-003: Single REST Endpoint
- **Fichier** : `crates/cynic-kernel/src/bootstrap/runtime_orchestrator.rs:171-196`
- **Composant** : `run_rest_servers()` sur `CYNIC_REST_ADDR` unique
- **Impact** : API indisponible = pas de CLI, pas d'agents, pas de dashboard, pas de webhooks
- **Fréquence** : Port conflict, bind error, kernel crash, network issue
- **Mitigation actuelle** : Aucune (single listener)
- **Coût estimé** : 1 sem (LB + multiple replicas + shared storage)
- **Owner** : `cynic-kernel::bootstrap::server`
- **ROI** : 8/10 (API = interface unique)
- **Status** : `observed`

### SPOF-004: Coordination Single Writer
- **Fichier** : `crates/cynic-kernel/src/bootstrap/storage.rs:36-42` → `coord = storage_runtime.coord`
- **Composant** : `CoordPort` trait unique implémenté par SurrealDB
- **Impact** : Claims, leases, heartbeat, topology, agent sessions bloqués
- **Fréquence** : SurrealDB down, contention, lock contention
- **Mitigation actuelle** : Aucune (single writer)
- **Coût estimé** : 2-3 sem (CRDT pour claims/leases OU Raft-backed coordination)
- **Owner** : `cynic-kernel::domain::coord` + `storage::surreal`
- **ROI** : 9/10 (coordination = cœur distribué)
- **Status** : `observed`

---

## HIGH

### SPOF-005: Bootstrap Integrity Chain Single Path
- **Fichier** : `crates/cynic-kernel/src/bootstrap/runtime_orchestrator.rs:64`
- **Composant** : `infra::boot::seed_integrity_chain(&storage_port)`
- **Impact** : Chain broken = bootstrap non vérifié, trust anchor compromis
- **Fréquence** : Storage corruption, seed mismatch, first-boot failure
- **Mitigation actuelle** : Aucune (single path)
- **Coût estimé** : 1 sem (quorum validation + multi-seed)
- **Owner** : `cynic-kernel::infra::boot`
- **ROI** : 7/10 (trust anchor critique mais rare)
- **Status** : `observed`

### SPOF-006: Inference Router Single Port
- **Fichier** : `crates/cynic-kernel/src/bootstrap/runtime_orchestrator.rs:56-59`
- **Composant** : `build_initial_embedding_port()` → `AutoRecoveryEmbedding` unique
- **Impact** : Inference down = pas d'embeddings, pas de routing, pas de judge
- **Fréquence** : Model load failure, OOM, GPU reset, provider API down
- **Mitigation actuelle** : `AutoRecoveryEmbedding` (retry single port)
- **Coût estimé** : 1-2 sem (pool multi-provider + fallback local/remote)
- **Owner** : `cynic-inference::application::embedding_runtime`
- **ROI** : 6/10 (inference important mais dégradé acceptable)
- **Status** : `observed`

### SPOF-007: Organ Remediation Single Task
- **Fichier** : `crates/cynic-kernel/src/bootstrap/runtime_orchestrator.rs:88-94`
- **Composant** : `infra::tasks::spawn_organ_remediation()` — un seul task global
- **Impact** : Task crash = pas de restart d'organ failed, degradation silencieuse
- **Fréquence** : Task panic, cancellation, shutdown bug
- **Mitigation actuelle** : Aucune (single task)
- **Coût estimé** : 3-5 j (supervisor + watchdog + task health monitoring)
- **Owner** : `cynic-kernel::infra::tasks`
- **ROI** : 5/10 (remediation = safety net, pas core)
- **Status** : `observed`

---

## Mitigation Pattern Réutilisable

```rust
// Pattern actuel : ReconnectableStorage (OK pour reconnect, PAS pour HA)
pub struct ReconnectableStorage {
    inner: Arc<RwLock<Arc<dyn StoragePort>>>,
}

// Pattern HA requis : ReplicatedStorage
pub struct ReplicatedStorage {
    primary: Arc<dyn StoragePort>,
    replicas: Vec<Arc<dyn StoragePort>>,
    consensus: ConsensusProtocol,  // Raft / etcd / SurrealDB cluster
    failover: FailoverPolicy,
}
```

---

## Dépendances entre SPOF

```
SPOF-001 (process)
    └─→ SPOF-003 (REST) — même processus
    └─→ SPOF-007 (remediation) — même processus
    └─→ SPOF-006 (inference) — même processus

SPOF-002 (SurrealDB)
    └─→ SPOF-004 (coord) — même storage
    └─→ SPOF-005 (bootstrap chain) — même storage
```

**Ordre de résolution suggéré** : SPOF-002 → SPOF-004 → SPOF-001 → SPOF-003 → SPOF-005 → SPOF-006 → SPOF-007