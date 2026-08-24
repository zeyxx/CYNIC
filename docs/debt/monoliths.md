# Monoliths & SoC Violations Registry — CYNIC

**Critère** : Fichier > 400 lignes OU multiple concerns dans un fichier (types + I/O + policy + apply)
**Référence** : AGENTS.md §1 "No monolith surfaces" + §7 "Surface grammar"

---

## CRITIQUE (oversized + multi-concerns)

### MONO-001: cynic-host/src/registry.rs (1481 lignes)
- **Concerns mélangés** :
  - Types: `Registry`, `RegistryEntry`, `Role`, `Capabilities`
  - Scan: `scan_registry_files()`, `discover_registry_sources()`
  - Validation: `validate_registry()`, `check_role_consistency()`
  - Merge: `merge_registry()`, `resolve_conflicts()`
  - Persist: `write_registry()`, `write_registry_atomic()`
- **Split requis** :
  - `registry/types.rs` — types purs
  - `registry/scan.rs` — discovery + lecture
  - `registry/validate.rs` — règles pures
  - `registry/merge.rs` — résolution conflits
  - `registry/persist.rs` — écriture atomique
  - `registry/mod.rs` — façade thin
- **Owner** : `cynic-host::registry`
- **Effort** : 2-3 j
- **Status** : `done` (2026-08-24) — split atterri : `registry.rs` (1481 l.) supprimé, remplacé par `registry/{types,scan,validate,merge,persist}.rs` (2587 l.). Preuve : `cargo check -p cynic-host` vert (33.7s full, 0.15s incrémental post-commit), 41 warnings / 0 erreur. NB : contenu identique au lot préparé en session mais absorbé par le batch hétérogène `b0943423` de l'agent concurrent, pas committé isolément.

### MONO-002: cynic-host/src/federation_runtime.rs (565 lignes)
- **Concerns mélangés** :
  - `MeshObservation` — type de rapport
  - `MeshPeerObserver` — port observation (impl `ObservationPort`)
  - `LocalAutonomyResolver` — policy décision autonomie
  - `FederationRuntime` — orchestration runtime
- **Split requis** :
  - `federation/types.rs` — `MeshObservation`, `EmergenceState`
  - `federation/observe.rs` — `MeshPeerObserver` trait + impl
  - `federation/autonomy.rs` — `LocalAutonomyResolver` policy
  - `federation/runtime.rs` — `FederationRuntime` orchestration
- **Owner** : `cynic-host::federation_runtime`
- **Effort** : 1-2 j
- **Status** : `observed`

### MONO-003: cynic-host/src/peer_resolver_mesh.rs (565 lignes)
- **Concerns mélangés** :
  - `MeshPeerResolver` — résolution peers (logique pure)
  - `MeshPeerObserver` — observation mesh (port)
  - `OsHealthObserver` — probe OS health (importé mais couplé)
  - `resolve_peers_filtered` / `resolve_all_peers` / `fallback_peers` — fonctions
- **Note** : `fallback_peers()` déjà fix (return vec![]), `observed_at_unix_s` déjà fix (SystemTime)
- **Split requis** :
  - `peer_resolver/types.rs` — `PeerEndpoint`, `SignedPeerEndpoint`, `ResolvedPeers`
  - `peer_resolver/resolve.rs` — `MeshPeerResolver` pure logic
  - `peer_resolver/observe.rs` — `MeshPeerObserver` port + impl
  - `peer_resolver/health.rs` — OS health integration (déjà dans `os_health/`)
- **Owner** : `cynic-host::peer_resolver_mesh`
- **Effort** : 1-2 j
- **Status** : `observed`

### MONO-004: cynic-host/src/runtime_layout.rs (869 lignes)
- **Concerns mélangés** :
  - Types: `RuntimeLayout`, `Principal`, `Paths`, `BootstrapConfig`
  - Discovery: `discover_layout()`, `detect_principal()`, `probe_paths()`
  - Apply: `apply_layout()`, `ensure_directories()`, `write_manifest()`
  - Prove: `verify_layout()`, `read_back_proof()`
- **Split requis** :
  - `runtime_layout/types.rs`
  - `runtime_layout/discover.rs`
  - `runtime_layout/apply.rs`
  - `runtime_layout/prove.rs`
- **Owner** : `cynic-host::runtime_layout`
- **Effort** : 1-2 j
- **Status** : `observed`

### MONO-005: cynic-host/src/source_control.rs (622 lignes)
- **Concerns mélangés** :
  - Types: `SourceControl`, `GitRemote`, `ForgejoConfig`, `LeaseAdmission`
  - Git probe: `probe_git_repo()`, `get_remote_url()`, `get_branch()`
  - Forgejo adapter: `ForgejoClient`, `create_pr()`, `merge_pr()`
  - Lease admission: `admit_lease()`, `verify_lease()`
  - Sync: `sync_with_remote()`, `push_changes()`
- **Split requis** :
  - `source_control/types.rs`
  - `source_control/git_probe.rs`
  - `source_control/forgejo_adapter.rs`
  - `source_control/lease.rs`
  - `source_control/sync.rs`
- **Owner** : `cynic-host::source_control`
- **Effort** : 2-3 j
- **Status** : `observed`

---

## HIGH (oversized)

### MONO-006: cynic-host/src/framework_bootstrap/build_runner.rs (659 lignes)
- **Concerns** : Build orchestration + policy + execution + proof
- **Split** : `types` / `plan` / `execute` / `prove`

### MONO-007: cynic-host/src/framework_bootstrap/lifecycle/registry.rs (615 lignes)
- **Concerns** : Lifecycle registry + state machine + transitions + persistence

### MONO-008: cynic-host/src/domain/packages.rs (670 lignes)
- **Concerns** : Package types + resolution + validation + installation policy

### MONO-009: cynic-host/src/domain/services.rs (622 lignes)
- **Concerns** : Service types + registration + health + supervision policy

### MONO-010: cynic-host/src/domain/ecosystem_package/integration_apply.rs (493 lignes)
- **Concerns** : Apply logic + rollback + proof + error handling

### MONO-011: cynic-node/src/cli.rs (733 lignes)
- **Violation AGENTS.md §1** : CLI owns domain policy
- **Concerns** : Clap parse + use case dispatch + formatting + error handling
- **Fix** : Thin facade → delegate à `cynic-node::usecases::*`

### MONO-012: cynic-node/src/fleet_inventory.rs (1119 lignes)
- **Concerns** : Inventory + discovery + health + sync
- **Split** : `types` / `discover` / `health` / `sync`

### MONO-013: cynic-node/src/network_mesh_apply_plan.rs (730 lignes)
- **Concerns** : Plan + apply + rollback + evidence
- **Split** : `plan` / `apply` / `rollback` / `prove`

### MONO-014: cynic-cli/src/runtime_surface.rs (656 lignes)
- **Violation AGENTS.md §1** : 15 surfaces dans un fichier
- **Surfaces** : disk, network, fleet, inference, agents, hermes-run, cpu, memory, runtime, kernel, node, host, bootstrap, dev, chat
- **Split** : Un module par surface dans `cynic-cli/src/runtime_surface/`

### MONO-015: cynic-cli/src/chat_tui/state.rs (1459 lignes)
- **Concerns** : TUI state + chat history + runtime integration + key bindings

### MONO-016: cynic-protocol/src/health_signal.rs (1466 lignes)
- **Concerns mélangés** :
  - Types: `HealthSignal`, `SignalKind`, `Severity`, `Thresholds`
  - Classification: `classify_signal()`, `evaluate_thresholds()`
  - Aggregation: `aggregate_signals()`, `compute_health_score()`
  - Policy: `HealthPolicy`, `escalation_rules`
- **Split requis** :
  - `health_signal/types.rs`
  - `health_signal/classify.rs`
  - `health_signal/thresholds.rs`
  - `health_signal/aggregate.rs`
  - `health_signal/policy.rs`

### MONO-017: cynic-protocol/src/framework.rs (1016 lignes)
- **Concerns** : Framework spec géante — devrait être générée depuis contracts/
- **Note** : Contient `FrameworkSpec` + tous les sous-specs

### MONO-018: cynic-protocol/src/agent_coordination.rs (948 lignes)
- **Concerns** : Coordination + claims + leases + predicates
- **Split** : `types` / `claims` / `leases` / `predicates`

---

## Pattern de split standard (AGENTS.md §1)

```
module/
├── types.rs        # Pure types, serde, no I/O
├── spec.rs         # Policy tables, pure functions (optional)
├── probe.rs        # Read-only observation (port + local impl)
├── decide.rs       # Pure reconciliation from desired + observed
├── plan.rs         # Bounded action plan (argv, privilege, consent)
├── apply.rs        # Execution (injected argv runner)
├── prove.rs        # Read-back verification
└── mod.rs          # Thin facade re-exporting
```

---

## Fichiers déjà conformes (exemples)

| Fichier | Lignes | Pattern |
|---------|--------|---------|
| `cynic-host/src/os_health/probes.rs` | 2890 | `probe` only |
| `cynic-host/src/os_health/observe.rs` | 5369 | `observe` port + impl |
| `cynic-host/src/adapters/local_disk.rs` | 502 | `apply` adapter |
| `cynic-protocol/src/peer_resolver.rs` | 5886 | `types` + `resolve` (pure) |
| `cynic-protocol/src/build_node.rs` | 8127 | `types` + `spec` (policy tables) |

---

## Priorisation ROI

| Entrée | Criticité | Effet | ROI | Ordre |
|--------|-----------|-------|-----|-------|
| MONO-001 (registry.rs) | CRITIQUE | 2-3j | 10/10 | P1-1 |
| MONO-002 (federation_runtime.rs) | CRITIQUE | 1-2j | 9/10 | P1-2 |
| MONO-003 (peer_resolver_mesh.rs) | CRITIQUE | 1-2j | 8/10 | P1-3 |
| MONO-014 (runtime_surface.rs) | HIGH | 1-2j | 8/10 | P1-4 |
| MONO-016 (health_signal.rs) | HIGH | 1-2j | 7/10 | P2-1 |
| MONO-004 (runtime_layout.rs) | HIGH | 1-2j | 7/10 | P2-2 |
| MONO-005 (source_control.rs) | HIGH | 2-3j | 6/10 | P2-3 |
| MONO-011 (cli.rs node) | HIGH | 1-2j | 6/10 | P2-4 |
| MONO-012 (fleet_inventory.rs) | HIGH | 1-2j | 5/10 | P3-1 |
| Autres | MEDIUM | - | - | P3+ |