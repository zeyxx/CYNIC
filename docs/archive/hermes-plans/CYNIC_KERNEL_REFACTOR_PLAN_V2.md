# REFACTOR CYNIC-KERNEL — PLAN CORRIGÉ v2

> **8 prismes d'ingénierie** ont invalidé le plan initial (10 blockers, 15+ major).
> **Objectif** : Stabiliser le kernel, tuer la dette, frontières propres.
> **Stratégie** : Tuer > Réparer > Valider. Zéro déplacement de dette, zéro crate fantôme, zéro couplage prématuré.
> **Baseline réelle** : 230 fichiers, 43,434 LOC (pas 17k comme initialement annoncé).

---

## DIAGNOSTIC CORRIGÉ

| Métrique | Valeur réelle | Objectif | Delta |
|----------|---------------|----------|-------|
| Fichiers | 230 | ~180 | -50 |
| LOC | 43,434 | ~35,000 | -8,434 |
| Erreurs compilation | 28 | 0 | -28 |
| Dead code LOC | ~3,330 | 0 | -3,330 |
| Violations arch | 3 | 0 | -3 |
| Overlap bus | ~1,200 | ~1,200 (différé) | 0 |

---

## PHASE 0 — STABILISER (1 PR, ~2 heures)

**Objectif** : Kernel compile. Prérequis pour tout le reste.

### 0.1 — Fixer les imports cassés

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

**Vérification** : `crate::domain::organ::OrganPort` existe bien dans le kernel (77 LOC, trait pur).

### 0.2 — Déclarer les modules manquants

| Module manquant | Fichier | Action |
|-----------------|---------|--------|
| `crate::json` | `lib.rs` | Ajouter `pub mod json;` |
| `crate::infra::runtime_secret` | `infra/mod.rs` | Ajouter `pub mod runtime_secret;` |

### 0.3 — Supprimer les références à des modules morts

| Référence | Fichier | Action |
|-----------|---------|--------|
| `crate::infra::routing_calc` | `infra/health_loop.rs` | Supprimer l'import |
| `crate::infra::routing_calc` | `bootstrap/runtime_orchestrator.rs` | Supprimer l'import |
| `crate::infra::dog_performance` | `api/rest/types.rs` | Supprimer l'import |

### 0.4 — Vérification

```bash
cargo check -p cynic-kernel
# Résultat attendu : 0 errors
```

**Livrable** : Kernel compile. Preuve : `cargo check -p cynic-kernel` → 0 errors.

---

## PHASE 1 — TUER LE DEAD CODE (1 PR, ~3 heures)

**Objectif** : Supprimer tout module avec 0 référence externe. Pas de déplacement, suppression pure.

### 1.1 — Domain orphelin (723 LOC)

```bash
git rm crates/cynic-kernel/src/domain/chat.rs
git rm crates/cynic-kernel/src/domain/daily_budget.rs
git rm crates/cynic-kernel/src/domain/epoche.rs
git rm crates/cynic-kernel/src/domain/resource_guard.rs
```

Mettre à jour `domain/mod.rs`.

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

**⚠️ NE PAS SUPPRIMER** :
- `evidence_projection_store.rs` — utilisé par le pipeline PoH (Solana prism l'a identifié comme critique)
- `pattern_model_seal.rs` — utilisé pour le sealing PoH

Mettre à jour `infra/mod.rs`.

### 1.3 — Storage orphelin (350 LOC)

```bash
git rm crates/cynic-kernel/src/storage/memory.rs
rm -rf crates/cynic-kernel/src/storage/memory/
```

Mettre à jour `storage/mod.rs`.

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

## PHASE 2 — VALIDER (1 PR, ~1 heure)

**Objectif** : Vérifier que le kernel est stable et que les frontières sont propres.

### 2.1 — Checklist

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

# 5. Pas de références pendantes à organ/
grep -rn "crate::organ::InferenceOrgan" crates/cynic-kernel/
# Résultat attendu : 0 hits

# 6. Frontières propres (vérification partielle — Phase 1 a supprimé les modules cassés)
grep -rn "use crate::api::" crates/cynic-kernel/src/infra/
grep -rn "use crate::infra::crypto" crates/cynic-kernel/src/api/
# Résultat attendu : 0 hits (ou résidus à traiter dans une future session)
```

### 2.2 — Métriques

```bash
# LOC après refactor
find crates/cynic-kernel/src -name '*.rs' -exec wc -l {} + | tail -1

# Fichiers après refactor
find crates/cynic-kernel/src -name '*.rs' | wc -l
```

**Livrable** : Kernel stable, frontières propres, métriques documentées.

---

## CE QUI EST DÉFERRÉ (pas dans ce plan)

| Item | Raison |
|------|--------|
| **Phase 3 (bus merge)** | Bus immature, types incompatibles, couplage prématuré. Différer. |
| **Phase 4 (extract cynic-host)** | Dépendance circulaire kernel↔host, cynic-host déjà 90k LOC. Différer. |
| **`cynic-inference-runtime` crate** | Pas de frontière définie, crate fantôme. Différer. |
| **`SovereignRuntimePort` trait** | 0 implémentations, prématuré. Différer. |
| **Fine-tuning pipeline** | Hors scope, session dédiée. |
| **Split `api/` en crate séparée** | Trop complexe pour cette session. |
| **Split `infra/` en micro-crates** | Nécessite une session dédiée. |
| **Refonte complète du storage** | Dépend de la stabilisation post-refactor. |
| **Décomposition `AppState`** | God Object à traiter dans une session dédiée. |

---

## RÉSUMÉ DES CHANGEMENTS

| Phase | Action | LOC impact | Fichiers touchés | Risque |
|-------|--------|------------|------------------|--------|
| 0 | Fix imports | +5 imports | 5 fichiers | Near zero |
| 1 | Tuer dead code | -3,330 LOC | 16 fichiers supprimés | Near zero |
| 2 | Valider | 0 | 0 | Near zero |

**Total** : -3,330 LOC, 21 fichiers touchés, ~6 heures de travail.

---

## POST-REFACTOR (prochaines sessions)

Une fois le kernel stabilisé :
1. Décomposer `AppState` (God Object 40+ champs)
2. Fixer les violations architecture restantes (infra↔api)
3. Évaluer la fusion avec `cynic-agent-bus` (quand le bus sera stable)
4. Évaluer l'extraction vers `cynic-host` (quand les frontières seront claires)
5. Split `api/` en crate séparée
6. Split `infra/` en micro-crates

---

*Plan corrigé v2. Validation par 8 prismes d'ingénierie. Exécution phase par phase avec validation intermédiaire.*
