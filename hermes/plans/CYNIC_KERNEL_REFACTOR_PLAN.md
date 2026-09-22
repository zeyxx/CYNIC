# REFACTOR CYNIC-KERNEL — PLAN DÉTAILLÉ

> **Contexte** : Audit kernel (220 fichiers, 17,103 LOC) a révélé 28+ erreurs compilation, 3,300 LOC dead code, violations architecture, overlap avec cynic-agent-bus.
> **Objectif** : Stabiliser, tuer la dette, frontières propres. Pas de monolith移位.
> **Stratégie** : Tuer > Réparer > Extraire. Ordre de dépendance respecté.

---

## DIAGNOSTIC RÉSUMÉ

| Catégorie | Métrique |
|-----------|----------|
| Fichiers | 220 |
| LOC total | 17,103 |
| Erreurs compilation | 28 |
| Dead code LOC | 3,330 (19%) |
| Violations architecture | 3 critiques |
| Overlap bus | ~1,200 LOC |
| Dette extractible | ~4,800 LOC (28%) |

---

## PHASE 0 — RÉPARER LES IMPORTS CASSÉS (préalable)

**Objectif** : Kernel compile. Prérequis pour tout le reste.

### 0.1 — `crate::organ::InferenceOrgan` → chemin correct

**Fichiers** :
- `crates/cynic-kernel/src/infra/tasks/runtime_loops/probes.rs:5`
- `crates/cynic-kernel/src/infra/tasks/lifecycle.rs:89`

**Changement** :
```rust
// AVANT
use crate::organ::InferenceOrgan;

// APRÈS — utiliser le kernel's own trait
use crate::domain::organ::OrganPort;
```

### 0.2 — Modules non déclarés dans `lib.rs` et `mod.rs`

| Module manquant | Fichier | Action |
|-----------------|---------|--------|
| `crate::json` | `lib.rs` | Ajouter `pub mod json;` ou inline le type |
| `crate::infra::runtime_secret` | `infra/mod.rs` | Ajouter `pub mod runtime_secret;` |
| `crate::infra::routing_calc` | `infra/mod.rs` | **Supprimer les références** (module mort) |
| `crate::infra::dog_performance` | `infra/mod.rs` | **Supprimer les références** (module mort) |

**Fichiers à mettre à jour** :
- `crates/cynic-kernel/src/lib.rs` — ajouter `pub mod json;`
- `crates/cynic-kernel/src/infra/mod.rs` — ajouter `pub mod runtime_secret;`
- `crates/cynic-kernel/src/infra/health_loop.rs` — supprimer `routing_calc`
- `crates/cynic-kernel/src/bootstrap/runtime_orchestrator.rs` — supprimer `routing_calc`
- `crates/cynic-kernel/src/api/rest/types.rs` — supprimer `dog_performance`

### 0.3 — Vérification

```bash
cargo check -p cynic-kernel
# Résultat attendu : 0 errors
```

**Livrable** : Kernel compile. Preuve : `cargo check -p cynic-kernel` → 0 errors.

---

## PHASE 1 — TUER LE DEAD CODE (3,330 LOC)

**Objectif** : Supprimer tout module avec 0 référence externe. Pas de déplacement, suppression pure.

### 1.1 — Domain orphelin (723 LOC)

```bash
git rm crates/cynic-kernel/src/domain/chat.rs
git rm crates/cynic-kernel/src/domain/daily_budget.rs
git rm crates/cynic-kernel/src/domain/epoche.rs
git rm crates/cynic-kernel/src/domain/resource_guard.rs
```

Mettre à jour `domain/mod.rs` :
```rust
// Retirer les lignes :
// pub mod chat;
// pub mod daily_budget;
// pub mod epoche;
// pub mod resource_guard;
```

### 1.2 — Infra orphelin (1,678 LOC)

```bash
git rm crates/cynic-kernel/src/infra/node_enrollment_sqlite.rs
git rm crates/cynic-kernel/src/infra/node_control_sqlite.rs
git rm crates/cynic-kernel/src/infra/health_loop.rs
git rm crates/cynic-kernel/src/infra/evidence_projection_store.rs
git rm crates/cynic-kernel/src/infra/coordination_journal.rs
git rm crates/cynic-kernel/src/infra/coordination_authority.rs
git rm crates/cynic-kernel/src/infra/error_classifier.rs
git rm crates/cynic-kernel/src/infra/pattern_model_seal.rs
```

Mettre à jour `infra/mod.rs`.

### 1.3 — Storage orphelin (350 LOC)

```bash
git rm crates/cynic-kernel/src/storage/memory.rs
# Mettre à jour storage/mod.rs
# memory/ sous-dossier :
rm -rf crates/cynic-kernel/src/storage/memory/
```

### 1.4 — Top-level orphelin (598 LOC)

```bash
git rm crates/cynic-kernel/src/build_scheduler.rs
git rm crates/cynic-kernel/src/capability_issuer.rs
git rm crates/cynic-kernel/src/bin/memory_benchmark.rs
```

Mettre à jour `lib.rs`.

### 1.5 — Vérification

```bash
cargo check -p cynic-kernel
# Résultat attendu : 0 errors
```

**Livrable** : -3,330 LOC. Kernel compile. Preuve : `git show --stat HEAD` montre suppressions.

---

## PHASE 2 — FIXER LES VIOLATIONS ARCHITECTURE

**Objectif** : Infra ne dépend plus d'API. API ne dépend plus d'Infra.

### 2.1 — `infra` → `api` (CRITIQUE)

**Problème** : `runtime_loops.rs` et `state_log.rs` importent `AppState` depuis `api/rest/types.rs`.

**Fichiers** :
- `crates/cynic-kernel/src/infra/tasks/runtime_loops.rs`
- `crates/cynic-kernel/src/infra/tasks/state_log.rs`

**Solution** : Déplacer `AppState` dans `domain/` ou créer un `RuntimeState` dans `infra/`.

```rust
// NOUVEAU : crates/cynic-kernel/src/infra/runtime_state.rs
pub struct RuntimeState {
    pub storage: Arc<dyn StoragePort>,
    pub claim_store: Arc<dyn ClaimStore>,
    // ...
}

// api/rest/types.rs — déplacer AppState vers infra/runtime_state.rs
// runtime_loops.rs — utiliser infra::runtime_state::RuntimeState
```

### 2.2 — `api` → `infra` (CRITIQUE)

**Problème** : `auth.rs` importe `infra/crypto.rs`. `types.rs` importe `infra/task_health.rs`.

**Solution** : API dépend uniquement de `domain/`. Les implémentations sont injectées par le bootstrap.

```rust
// auth.rs — AVANT
use crate::infra::crypto;

// auth.rs — APRÈS
use crate::domain::organ::OrganPort; // ou un AuthPort trait
```

```rust
// types.rs — AVANT
use crate::infra::task_health::TaskHealth;

// types.rs — APRÈS
use crate::domain::HealthStatus; // type domaine pur
```

### 2.3 — `storage` → `infra` (MODÉRÉ)

**Problème** : `storage/auth.rs` importe `infra/runtime_secret.rs`. `storage/client.rs` importe `infra/config.rs`.

**Solution** : Injection par constructeur dans bootstrap.

```rust
// storage/auth.rs — AVANT
use crate::infra::runtime_secret::SecretMaterial;

// storage/auth.rs — APRÈS
pub struct AuthConfig {
    pub secret_material: SecretMaterial,
}
// Config passée par le bootstrap, pas importée directement
```

### 2.4 — Vérification

```bash
cargo check -p cynic-kernel
# Vérifier : plus d'imports inter-couches illégitimes
grep -rn "use crate::api::" crates/cynic-kernel/src/infra/  # doit être vide
grep -rn "use crate::infra::crypto" crates/cynic-kernel/src/api/  # doit être vide
```

**Livrable** : Frontières propres. Preuve : grep ci-dessus → 0 hits.

---

## PHASE 3 — RÉCUPER AVEC CYNIC-AGENT-BUS

**Objectif** : Éviter la duplication. Le kernel utilise le bus.

### 3.1 — Overlap identifié

| Kernel type | Bus type | Action |
|-------------|----------|--------|
| `domain/coord.rs::ClaimEntry` | `cynic_agent_bus::Claim` | Supprimer kernel, utiliser bus |
| `domain/storage/types.rs::ProofOfHistory` | `cynic_agent_bus::Hash32` | Supprimer kernel, utiliser bus |
| `storage/schema_bootstrap.sql::work_claim` | `cynic_agent_bus::Claim` | Supprimer table, utiliser bus |
| `storage/surreal/workspace_claims.rs` | `cynic_agent_bus::ClaimStore` | Supprimer, utiliser bus |

### 3.2 — Actions

```bash
# 1. Ajouter cynic-agent-bus aux dépendances kernel
# crates/cynic-kernel/Cargo.toml :
# [dependencies]
# cynic-agent-bus = { path = "../cynic-agent-bus" }

# 2. Remplacer les types
# domain/coord.rs — supprimer ClaimEntry, utiliser cynic_agent_bus::Claim
# domain/storage/types.rs — supprimer ProofOfHistory, utiliser cynic_agent_bus::Hash32

# 3. Supprimer les tables dupliquées
# storage/schema_bootstrap.sql — supprimer work_claim, agent_workspace_claim

# 4. Supprimer workspace_claims.rs
git rm crates/cynic-kernel/src/storage/surreal/workspace_claims.rs
git rm crates/cynic-kernel/src/storage/surreal/workspace_claims_tests.rs
```

### 3.3 — Vérification

```bash
cargo check -p cynic-kernel
# Vérifier : plus de duplication
grep -rn "ProofOfHistory\|ClaimEntry" crates/cynic-kernel/src/  # doit être vide
```

**Livrable** : Kernel utilise le bus. Preuve : grep ci-dessus → 0 hits.

---

## PHASE 4 — EXTRAIRE LES BRIQUES HOST

**Objectif** : Le kernel ne contient que le runtime core. Les concerns host vont dans `cynic-host`.

### 4.1 — Modules à extraire

| Module | LOC | Cible |
|--------|-----|-------|
| `infra/alerts/slack.rs` | ~100 | `cynic-host` |
| `infra/probes/fleet/` | ~200 | `cynic-host` |
| `infra/remediation.rs` | 342 | `cynic-host` |
| `infra/runtime_takeover.rs` | ~150 | `cynic-host` |
| `infra/domain_router.rs` | ~120 | `cynic-protocol` |
| `api/rest/data/` | ~1,200 | `cynic-host` |
| `api/websocket/mod.rs` | 519 | `cynic-host` |

### 4.2 — Actions

```bash
# Déplacer vers cynic-host
mkdir -p crates/cynic-host/src/{alerts,probes/fleet,remediation,takeover,websocket}

git mv crates/cynic-kernel/src/infra/alerts/slack.rs crates/cynic-host/src/alerts/
git mv crates/cynic-kernel/src/infra/probes/fleet/ crates/cynic-host/src/probes/fleet/
git mv crates/cynic-kernel/src/infra/remediation.rs crates/cynic-host/src/remediation.rs
git mv crates/cynic-kernel/src/infra/runtime_takeover.rs crates/cynic-host/src/takeover.rs
git mv crates/cynic-kernel/src/api/websocket/ crates/cynic-host/src/websocket/
git mv crates/cynic-kernel/src/api/rest/data/ crates/cynic-host/src/rest/data/

# Mettre à jour les mod.rs
```

### 4.3 — Vérification

```bash
cargo check -p cynic-kernel
cargo check -p cynic-host
```

**Livrable** = -2,500 LOC du kernel. Preuve : `git show --stat HEAD`.

---

## PHASE 5 — VALIDATION FINALE

### 5.1 — Checklist

```bash
# 1. Kernel compile
cargo check -p cynic-kernel

# 2. Tous les crates dépendants compilent
cargo check -p cynic-agent-bus
cargo check -p cynic-agent-bus-server
cargo check -p cynic-agent-bus-coordinator
cargo check -p cynic-host

# 3. Tests boundary passent
cargo test -p cynic-agent-bus --test architecture_boundary

# 4. Pas de dead code restant
grep -rn "daily_budget\|epoche\|resource_guard\|health_loop\|memory_benchmark" crates/cynic-kernel/
# Résultat attendu : 0 hits

# 5. Pas de duplication bus
grep -rn "ProofOfHistory\|ClaimEntry\|work_claim" crates/cynic-kernel/
# Résultat attendu : 0 hits

# 6. Frontières propres
grep -rn "use crate::api::" crates/cynic-kernel/src/infra/
grep -rn "use crate::infra::crypto" crates/cynic-kernel/src/api/
# Résultat attendu : 0 hits
```

### 5.2 — Métriques cibles

| Métrique | Avant | Après | Delta |
|----------|-------|-------|-------|
| Fichiers kernel | 220 | ~180 | -40 |
| LOC kernel | 17,103 | ~12,000 | -5,100 |
| Erreurs compilation | 28 | 0 | -28 |
| Dead code LOC | 3,330 | 0 | -3,330 |
| Violations arch | 3 | 0 | -3 |
| Overlap bus | ~1,200 | 0 | -1,200 |

---

## RÉSUMÉ DES CHANGEMENTS

| Phase | Action | LOC impact | Fichiers touchés |
|-------|--------|------------|------------------|
| 0 | Fix imports | +5 imports | 5 fichiers |
| 1 | Tuer dead code | -3,330 LOC | 16 fichiers supprimés |
| 2 | Fix violations | +50 LOC | 6 fichiers modifiés |
| 3 | Fusionner bus | -1,200 LOC | 4 fichiers modifiés |
| 4 | Extraire host | -2,500 LOC | 7 fichiers déplacés |
| 5 | Validation | 0 | 0 |

**Total** : ~7,000 LOC supprimés, 33 fichiers touchés.

---

## CE QUI EST DÉFERRÉ

| Item | Raison |
|------|--------|
| Split `api/` en crate séparée | Trop complexe pour cette session |
| Split `infra/` en micro-crates | Nécessite une session dédiée |
| Refonte complète du storage | Dépend de la stabilisation post-refactor |
| Finetuning pipeline | Hors scope — session dédiée |

---

*Plan détaillé. Exécution phase par phase avec validation intermédiaire.*
