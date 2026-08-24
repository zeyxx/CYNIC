# Ownership Debt Registry — CYNIC

**Critère** : Module/crate sans owner clair, responsibility floue, ou boundary violée
**Référence** : AGENTS.md §6 "One canonical decision source per concern" + §2 "No hardcoding when dynamic contract exists"

---

## CRITIQUE (missing owner / boundary violation)

### OWN-001: Registry Writer — Single Writer sans Owner
- **Fichier** : `cynic-host/src/registry.rs` (fonctions `write_registry*`)
- **Problème** : Qui a le droit d'écrire le registry ? Pas de principal, pas de lease, pas d'admission
- **Actuel** : N'importe quel code appelant `write_registry()` — pas de guard
- **Requis** : `RegistryWriter` capability + lease admission + audit log
- **Owner manquant** : `cynic-host::registry::persist` (après split)
- **Status** : `observed`

### OWN-002: Kernel Process Lifecycle — Pas d'Owner Explicite
- **Fichier** : `cynic-kernel/src/main.rs` + `bootstrap/runtime_orchestrator.rs`
- **Problème** : Process démarre tout, mais qui le supervise ? systemd ? process supervisor ? pas de contrat
- **Actuel** : Implicite — l'opérateur lance le binaire
- **Requis** : `KernelProcessSupervisor` port + adapters (systemd, process, k8s)
- **Owner manquant** : `cynic-kernel::bootstrap::supervisor`
- **Status** : `observed`

### OWN-003: SurrealDB Connection — Owner Implicite
- **Fichier** : `cynic-kernel/src/bootstrap/storage.rs`
- **Problème** : Connection string hardcodée dans `StorageConfig`, pas de principal owner
- **Actuel** : `CYNIC_SURREAL_URL` env var — pas de contrat d'install/update
- **Requis** : `DatabaseProvisioner` port + install provenance tracking
- **Owner manquant** : `cynic-kernel::storage::provision`
- **Status** : `observed`

### OWN-004: REST API Keys — Rotation sans Owner
- **Fichier** : `cynic-kernel/src/bootstrap/runtime_orchestrator.rs:100-103`
- **Problème** : `load_rest_api_key()` + `RoleKeyMap::from_environment()` — pas de rotation policy
- **Actuel** : Env var statique, pas d'expiry, pas de rotation
- **Requis** : `ApiKeyManager` port + rotation policy + audit
- **Owner manquant** : `cynic-kernel::api::auth::keys`
- **Status** : `observed`

### OWN-005: Agent Session Runtime — Owner Flou
- **Fichier** : `cynic-kernel/src/bootstrap/agent_session.rs` + `cynic-protocol/src/agent_workload/`
- **Problème** : Sessions agents gérées par kernel MAIS workload delivery par node — double ownership
- **Actuel** : Kernel = session store, Node = delivery/execution — pas de boundary claire
- **Requis** : `AgentSessionAuthority` trait — un seul owner par session depth
- **Owner manquant** : `cynic-kernel::bootstrap::agent_session` OU `cynic-node::usecases::agent_workload`
- **Status** : `observed`

---

## HIGH (boundary violations)

### OWN-006: CLI Owns Domain Policy (AGENTS.md §1 violation)
- **Fichiers** :
  - `cynic-cli/src/agent_exposure_audit/checks.rs` (453 lignes)
  - `cynic-cli/src/ownership_audit.rs` (459 lignes)
  - `cynic-cli/src/kernel_boundary_audit.rs` (611 lignes)
  - `cynic-cli/src/dev_surface/check.rs` (564 lignes)
- **Problème** : CLI (surface) own audit logic, classification rules, policy tables
- **Requis** : Move vers `cynic-host::audit` ou `cynic-protocol::audit`
- **Owner correct** : `cynic-host::domain::audit` (policy) + `cynic-protocol::audit` (types)
- **Status** : `observed`

### OWN-007: Network Actuation — Double Ownership
- **Fichiers** :
  - `cynic-host/src/network_actuation.rs` (928 lignes) — host level
  - `cynic-node/src/network_mesh_apply_plan.rs` (730 lignes) — node level
  - `cynic-node/src/network_repair.rs` (425 lignes) — node level
- **Problème** : Host actuate + Node actuate + Node repair = 3 layers, pas de delegation claire
- **Requis** : `NetworkActuationPort` (host) → `NetworkApplyPort` (node) — delegation explicite
- **Owner correct** : Host = plan/decide, Node = apply/prove
- **Status** : `observed`

### OWN-008: Disk Observation — Triple Ownership
- **Fichiers** :
  - `cynic-host/src/domain/disk/probe.rs` — probe types + logic
  - `cynic-host/src/fleet_disk_observer.rs` — fleet rollup
  - `cynic-host/src/adapters/local_disk.rs` — adapter local
  - `cynic-host/src/adapters/remote_disk.rs` — adapter remote
- **Problème** : Probe + Observer + Adapters mélangés, pas de port clair
- **Requis** : `DiskObservationPort` (trait) → `LocalDiskAdapter` / `RemoteDiskAdapter` → `FleetDiskObserver` (rollup)
- **Owner correct** : `cynic-host::domain::disk` (port + types) + adapters séparés
- **Status** : `observed`

---

## MEDIUM (implicit ownership)

### OWN-009: Bootstrap Seed — Qui Génère / Qui Vérifie ?
- **Fichiers** : `cynic-node/src/usecases/bootstrap_seed_uc.rs` + `cynic-kernel/src/infra/boot/`
- **Problème** : Node génère, Kernel vérifie — pas de contrat de passage de témoin
- **Requis** : `BootstrapSeedContract` — generator + verifier + evidence

### OWN-010: Inference Models — Cache vs Runtime
- **Fichiers** : `cynic-inference/` + `cynic-kernel/src/api/rest/inference_router.rs`
- **Problème** : Kernel route l'inférence MAIS inference crate gère models/cache — boundary floue
- **Requis** : `InferenceRuntimePort` (kernel) ← `InferenceEngineAdapter` (inference crate)

### OWN-011: Fleet Heartbeat — Protocol vs Host
- **Fichiers** :
  - `cynic-protocol/src/fleet_heartbeat.rs` (19310 lignes) — schema + types
  - `cynic-host/src/fleet_heartbeat.rs` (2888 lignes) — port + store
  - `cynic-host/src/fleet_health_observer.rs` (5086 lignes) — observer
- **Problème** : Protocol owns schema MAIS host owns port/store/observer — split artificiel
- **Requis** : `FleetHeartbeatPort` dans protocol, impl dans host

### OWN-012: Node Enrollment — Kernel vs Node
- **Fichiers** :
  - `cynic-kernel/src/api/rest/node_enrollment.rs` — REST handlers
  - `cynic-kernel/src/bootstrap/node_enrollment.rs` — runtime loading
  - `cynic-node/src/adapters/registry/mod.rs` — node-side registry
- **Problème** : Kernel = enrollment authority MAIS node = registry participant — circular

---

## Pattern de résolution (AGENTS.md §4, §6)

```rust
// 1. Déclarer le PORT (trait) — owner: protocol crate
pub trait RegistryWriterPort {
    async fn write(&self, registry: &Registry, lease: LeaseGuard) -> Result<Proof, Error>;
}

// 2. Implémenter l'ADAPTER — owner: host crate
pub struct LocalRegistryWriter {
    storage: Arc<dyn StoragePort>,
    lease_authority: Arc<dyn LeaseAuthority>,
}

// 3. Injecter via runtime layout — owner: node/bootstrap
fn build_runtime(layout: &RuntimeLayout) -> RegistryWriterPort {
    LocalRegistryWriter::new(layout.storage(), layout.lease_authority())
}

// 4. Surface CLI thin — owner: cli crate
fn cmd_registry_apply(args: Args, writer: Arc<dyn RegistryWriterPort>) -> Result<()> { ... }
```

---

## Mapping Owner Correct par Domaine

| Domaine | Owner Canonique (Protocol) | Adapter (Host) | Orchestration (Node) | Surface (CLI) |
|---------|---------------------------|----------------|---------------------|---------------|
| Registry | `cynic-protocol::registry::types` | `cynic-host::registry::persist` | `cynic-node::usecases::registry_uc` | `cynic-cli::registry_surface` |
| Network | `cynic-protocol::network::types` | `cynic-host::network::actuate` | `cynic-node::usecases::network_uc` | `cynic-cli::network_surface` |
| Disk | `cynic-protocol::disk::types` | `cynic-host::domain::disk::probe` | `cynic-host::fleet_disk_observer` | `cynic-cli::disk_surface` |
| Coordination | `cynic-protocol::agent_coordination` | `cynic-host::coord::store` | `cynic-node::usecases::coord_uc` | `cynic-cli::coord_surface` |
| Inference | `cynic-protocol::inference::types` | `cynic-inference::adapters::*` | `cynic-node::usecases::inference_uc` | `cynic-cli::inference_surface` |
| Agent Runtime | `cynic-protocol::agent_workload` | `cynic-kernel::bootstrap::agent_session` | `cynic-node::usecases::agent_workload_uc` | `cynic-cli::agent_surface` |

---

## Priorisation

| Entrée | Criticité | Effort | ROI | Ordre |
|--------|-----------|--------|-----|-------|
| OWN-001 (Registry Writer) | CRITIQUE | 3-5j | 10/10 | P1-1 |
| OWN-002 (Kernel Supervisor) | CRITIQUE | 2-3sem | 9/10 | P0-SPOF |
| OWN-003 (DB Provisioning) | CRITIQUE | 1-2sem | 9/10 | P0-SPOF |
| OWN-006 (CLI Policy) | HIGH | 2-3j | 8/10 | P1-2 |
| OWN-007 (Network) | HIGH | 3-5j | 7/10 | P2-1 |
| OWN-008 (Disk) | HIGH | 2-3j | 7/10 | P2-2 |
| OWN-005 (Agent Session) | HIGH | 1-2sem | 6/10 | P2-3 |
| OWN-004 (API Keys) | HIGH | 1-2sem | 6/10 | P2-4 |
| OWN-009 à OWN-012 | MEDIUM | - | - | P3+ |