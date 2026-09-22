# CYNIC-KERNEL — REFACTOR RADICAL v3

> **Vision** : Le kernel est un superviseur de runtime. Rien de plus.
> **Stratégie** : Définir → Tuer → Reconstruire. Zéro réparation de code mort.
> **Résultat** : 230 fichiers → ~80 fichiers, 43k LOC → ~12k LOC.

---

## DÉFINITION CONSTITUTIONNELLE DU KERNEL

Le kernel porte **exactement 5 responsabilités** :

| # | Responsabilité | Module | LOC estimé |
|---|----------------|--------|------------|
| 1 | **Runtime supervision** | `bootstrap/` | 2,000 |
| 2 | **Stockage** | `storage/` (SurrealDB adapter) | 4,000 |
| 3 | **Surface API** | `api/rest/` (endpoints minimaux) | 3,000 |
| 4 | **Identité** | `infra/node_identity.rs` | 140 |
| 5 | **Introspection** | `introspection/` | 1,000 |

**Total cible** : ~10,140 LOC, ~80 fichiers.

### Le kernel NE DOIT PAS porter :
- ❌ Inference routing (`RoutingCalculator` → cynic-inference)
- ❌ Dog performance monitoring (→ cynic-inference)
- ❌ Inference process lifecycle (`organ/` → cynic-inference)
- ❌ Agent coordination (→ cynic-agent-bus)
- ❌ Fleet management (→ cynic-host)
- ❌ Remediation (→ cynic-host)
- ❌ Slack alerts (→ cynic-host)
- ❌ WebSocket (→ cynic-host)
- ❌ Admin/monitoring REST (→ cynic-host)
- ❌ Business logic (escore, futarchy, Solana → crates dédiées)

---

## TAXONOMIE RADICALE

### KURE (生きる — Vivre) — Ce qui reste dans le kernel

```
bootstrap/          → Composition root, startup, orchestration
  entry.rs          → main()
  runtime_state.rs  → Startup state (minimal)
  services.rs       → Service wiring (minimal)
  server.rs         → HTTP server bootstrap

domain/             → Types purs uniquement
  events.rs         → KernelEvent
  health_gate.rs    → HealthGate trait
  metrics.rs        → Metrics types
  probe.rs          → Probe trait
  sanitize.rs       → Input sanitization
  storage/          → StoragePort trait + sub-traits
  verdict_queue.rs  → VerdictQueue
  zones.rs          → ZoneConfig

storage/            → SurrealDB adapter
  surreal/          → SurrealHttpStorage impl
  client.rs         → DB client
  auth.rs           → Auth
  schema.rs         → Schema bootstrap

api/rest/           → Endpoints minimaux uniquement
  health.rs         → /health, /ready, /live
  capabilities.rs   → /capabilities
  discovery.rs      → /discovery
  types.rs          → AppState (minimal)

infra/              → Services essentiels uniquement
  config/           → Config loading
  crypto.rs         → Crypto utilities
  node_identity.rs  → ed25519 node ID
  task_health.rs    → Task health monitoring

introspection/      → Forensics engine
  engine.rs         → Crash detection
  soc.rs            → SOC thresholds

probe/              → Hardware probes
senses/             → Location
```

### SHINE (死ね — Meurs) — Ce qui est supprimé immédiatement

| Module | LOC | Raison |
|--------|-----|--------|
| `domain/organ.rs` | 77 | Trait inference → cynic-inference |
| `domain/chat.rs` | 4 | Re-export cynic-inference |
| `domain/daily_budget.rs` | 149 | Orphelin |
| `domain/epoche.rs` | 229 | Orphelin |
| `domain/resource_guard.rs` | 341 | Orphelin |
| `domain/coord.rs` | 658 | Agent coordination → bus |
| `domain/lease_proof.rs` | 200 | Agent coordination → bus |
| `domain/poih.rs` | 150 | Agent coordination → bus |
| `domain/contract.rs` | 100 | Agent coordination → bus |
| `domain/enrichment.rs` | 200 | Business logic → cynic-inference |
| `domain/ccm/` | 1,500 | Business logic → cynic-intel |
| `domain/state_log.rs` | 300 | Agent coordination → bus |
| `domain/verdict_queue.rs` | 200 | Business logic → cynic-intel |
| `domain/storage/null.rs` | 576 | Test mock → tests/ |
| `domain/storage/memory.rs` | 350 | Test mock → tests/ |
| `infra/routing_calc.rs` | 0 (manquant) | Inference routing → cynic-inference |
| `infra/dog_performance.rs` | 0 (manquant) | Inference monitoring → cynic-inference |
| `infra/domain_router.rs` | 120 | Inference routing → cynic-inference |
| `infra/health_loop.rs` | 334 | Orphelin |
| `infra/node_enrollment_sqlite.rs` | 288 | SQLite mort |
| `infra/node_control_sqlite.rs` | 171 | SQLite mort |
| `infra/evidence_projection_store.rs` | 125 | PoH → bus |
| `infra/coordination_journal.rs` | 232 | Agent coordination → bus |
| `infra/coordination_authority.rs` | 104 | Agent coordination → bus |
| `infra/error_classifier.rs` | 145 | Orphelin |
| `infra/pattern_model_seal.rs` | 279 | PoH → bus |
| `infra/remediation.rs` | 342 | Host concern → cynic-host |
| `infra/runtime_takeover.rs` | 150 | Host concern → cynic-host |
| `infra/alerts/` | 200 | Host concern → cynic-host |
| `infra/probes/fleet/` | 200 | Host concern → cynic-host |
| `infra/tasks/` | 2,000 | Agent coordination → bus |
| `storage/memory.rs` | 350 | Test mock → tests/ |
| `storage/surreal/activity.rs` | 835 | Business logic → cynic-intel |
| `storage/surreal/coord.rs` | 658 | Agent coordination → bus |
| `storage/surreal/workspace_claims.rs` | 397 | Agent coordination → bus |
| `api/rest/data/` | 1,200 | Admin → cynic-host |
| `api/rest/inference_router.rs` | 500 | Inference → cynic-inference |
| `api/rest/inference_proxy.rs` | 400 | Inference → cynic-inference |
| `api/websocket/` | 519 | Host concern → cynic-host |
| `build_scheduler.rs` | 333 | Orphelin |
| `capability_issuer.rs` | 89 | Orphelin |
| `bin/memory_benchmark.rs` | 176 | Orphelin |

**Total à tuer** : ~15,000 LOC, ~150 fichiers.

---

## EXÉCUTION — 3 PHASES RADICALES

### Phase A — TUER TOUT LE DEAD CODE (1 PR)

```bash
# Domain orphelin + business logic
git rm -r crates/cynic-kernel/src/domain/organ.rs
git rm -r crates/cynic-kernel/src/domain/chat.rs
git rm -r crates/cynic-kernel/src/domain/daily_budget.rs
git rm -r crates/cynic-kernel/src/domain/epoche.rs
git rm -r crates/cynic-kernel/src/domain/resource_guard.rs
git rm -r crates/cynic-kernel/src/domain/coord.rs
git rm -r crates/cynic-kernel/src/domain/lease_proof.rs
git rm -r crates/cynic-kernel/src/domain/poih.rs
git rm -r crates/cynic-kernel/src/domain/contract.rs
git rm -r crates/cynic-kernel/src/domain/enrichment.rs
git rm -r crates/cynic-kernel/src/domain/ccm/
git rm -r crates/cynic-kernel/src/domain/state_log.rs
git rm -r crates/cynic-kernel/src/domain/verdict_queue.rs
git rm -r crates/cynic-kernel/src/domain/storage/null.rs
git rm -r crates/cynic-kernel/src/domain/storage/memory.rs

# Infra orphelin + inference + host
git rm -r crates/cynic-kernel/src/infra/health_loop.rs
git rm -r crates/cynic-kernel/src/infra/node_enrollment_sqlite.rs
git rm -r crates/cynic-kernel/src/infra/node_control_sqlite.rs
git rm -r crates/cynic-kernel/src/infra/evidence_projection_store.rs
git rm -r crates/cynic-kernel/src/infra/coordination_journal.rs
git rm -r crates/cynic-kernel/src/infra/coordination_authority.rs
git rm -r crates/cynic-kernel/src/infra/error_classifier.rs
git rm -r crates/cynic-kernel/src/infra/pattern_model_seal.rs
git rm -r crates/cynic-kernel/src/infra/remediation.rs
git rm -r crates/cynic-kernel/src/infra/runtime_takeover.rs
git rm -r crates/cynic-kernel/src/infra/alerts/
git rm -r crates/cynic-kernel/src/infra/probes/fleet/
git rm -r crates/cynic-kernel/src/infra/tasks/
git rm -r crates/cynic-kernel/src/infra/domain_router.rs

# Storage business logic + agent coordination
git rm -r crates/cynic-kernel/src/storage/memory.rs
git rm -r crates/cynic-kernel/src/storage/surreal/activity.rs
git rm -r crates/cynic-kernel/src/storage/surreal/coord.rs
git rm -r crates/cynic-kernel/src/storage/surreal/workspace_claims.rs

# API inference + admin + websocket
git rm -r crates/cynic-kernel/src/api/rest/data/
git rm -r crates/cynic-kernel/src/api/rest/inference_router.rs
git rm -r crates/cynic-kernel/src/api/rest/inference_proxy.rs
git rm -r crates/cynic-kernel/src/api/websocket/

# Top-level orphelin
git rm -r crates/cynic-kernel/src/build_scheduler.rs
git rm -r crates/cynic-kernel/src/capability_issuer.rs
git rm -r crates/cynic-kernel/src/bin/memory_benchmark.rs
```

**Mettre à jour** : `lib.rs`, `domain/mod.rs`, `infra/mod.rs`, `storage/mod.rs`, `api/rest/mod.rs`.

### Phase B — RÉPARER LES IMPORTS CASSÉS (1 PR)

Après suppression, les fichiers restants ne doivent plus référencer :
- `crate::organ::InferenceOrgan` → supprimé avec `domain/organ.rs`
- `crate::infra::routing_calc` → supprimé
- `crate::infra::dog_performance` → supprimé
- `crate::domain::ccm` → supprimé
- `crate::domain::coord` → supprimé

**Fichiers à patcher** :
- `bootstrap/runtime_orchestrator.rs` → supprimer routing_calc usage
- `bootstrap/services.rs` → supprimer dog_perf_collector
- `api/rest/types.rs` → supprimer routing_calc + dog_perf_collector fields
- `bootstrap/rest_state.rs` → supprimer routing_calc field

### Phase C — VALIDER (1 PR)

```bash
cargo check -p cynic-kernel
cargo check --workspace
cargo test -p cynic-kernel
```

**Métriques finales** :
- Fichiers : 230 → ~80
- LOC : 43,434 → ~10,000
- Erreurs : 28 → 0
- Dead code : ~15,000 LOC → 0

---

## RÉSUMÉ

| Phase | Action | LOC impact | Fichiers |
|-------|--------|------------|----------|
| A | Tuer dead code + business logic | -15,000 | -150 |
| B | Réparer imports restants | 0 | ~5 |
| C | Valider | 0 | 0 |

**Total** : -15,000 LOC, -150 fichiers, kernel = runtime supervisor pur.

---

*Plan v3 radical. Exécution immédiate.*
