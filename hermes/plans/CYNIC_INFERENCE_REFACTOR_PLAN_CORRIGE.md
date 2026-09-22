# REFACTOR CYNIC-INFERENCE — PLAN CORRIGÉ

> **Contexte** : 8 prismes d'ingénierie (446+ tool calls) ont identifié 10 blockers et 15+ problèmes majeurs dans le plan initial.
> **Objectif** : Refactorer `cynic-inference` sans le casser, sans créer de cycles DAG, sans déplacer la dette.
> **Stratégie** : Refondre les briques défaillantes, pas les déplacer. Stabiliser avant d'extraire.

---

## PRINCIPES DIRECTEURS

1. **Pas de déplacement de dette** — on ne `git mv` pas un god object, on le tue ou on le recompose
2. **Pas de crate fantôme** — pas de `cynic-inference-runtime` tant qu'il n'a pas une frontière nette
3. **Pas de trait prématuré** — pas de `SovereignRuntimePort` tant qu'on n'a pas 2+ impls
4. **DAG strict** — zéro cycle, zéro dépendance circulaire
5. **Kernel d'abord** — le kernel ne compile plus, on le répare avant tout

---

## DIAGNOSTIC DES 8 PRISMES (résumé)

| Problème | Cause racine | Solution |
|----------|-------------|----------|
| `organ/` god object | 564 LOC, couplage fort, 0 consommateur externe | **Tuer** — supprimer, pas déplacer |
| `cynic-inference-runtime` vide | Plan = prose, pas de `src/` | **Ne pas créer** — pas de frontière définie |
| `SovereignRuntimePort` mauvais crate | Amplifie le bus, 0 impl | **Ne pas ajouter** — prématuré |
| Cycle kernel ↔ runtime | `OrganPort` dans kernel, `organ/` dans runtime | **Ne pas créer de runtime** — pas de cycle possible |
| "54 fichiers kernel" faux | Seulement 2 fichiers production référencent `organ/` | Blast radius = 2 fichiers, pas 54 |
| `CoordinatorReasoner` dupliqué | Existe déjà dans `ports.rs` | **Ne pas dupliquer** |
| `cynic-agent-bus-server` concret | Dépend de `InMemoryLedger` | **Injecter le trait `Ledger`** |
| `cynic-agent-bus` ré-exporte concrets | `pub use ledger::*` expose `InMemoryLedger` | **Déplacer les impls dans `adapters/`** |
| Pas de fine-tuning | QLoRA mentionné, 0 code | **Déférer** — hors scope |
| Tests boundary faibles | Pas de vérification DAG | **Renforcer** — ajouter checks cross-module |

---

## PLAN EN 7 PHASES

### Phase 0 — Réparer le kernel (prérequis)

**Problème** : `cynic-kernel` ne compile plus — 2 fichiers référencent `crate::organ::InferenceOrgan` qui n'existe pas.

**Fichiers concernés** :
- `crates/cynic-kernel/src/infra/tasks/runtime_loops/probes.rs:5`
- `crates/cynic-kernel/src/infra/tasks/lifecycle.rs:89`

**Action** :
1. Remplacer `crate::organ::InferenceOrgan` par un import depuis `cynic_inference::organ::InferenceOrgan` (chemin direct, pas de module intermédiaire)
2. Ajouter `cynic-inference` aux dépendances de `cynic-kernel/Cargo.toml` (si pas déjà fait)
3. Vérifier que `cynic_inference::organ` est accessible (re-export depuis `lib.rs`)

```rust
// probes.rs — AVANT
use crate::organ::InferenceOrgan;

// probes.rs — APRÈS
use cynic_inference::organ::InferenceOrgan;
```

```bash
# Vérification
cargo check -p cynic-kernel  # doit passer
```

**Livrable** : Kernel compile. Preuve : `cargo check -p cynic-kernel` → 0 errors.

---

### Phase 1 — Tuer `organ/` (pas déplacer)

**Problème** : `organ/` est un god object de 564 LOC avec couplage fort. 0 consommateur externe réel (le kernel a ses propres types `OrganPort`).

**Fichiers à supprimer** :
- `crates/cynic-inference/src/organ/inference_organ.rs` (198 LOC)
- `crates/cynic-inference/src/organ/backend_handle.rs` (139 LOC)
- `crates/cynic-inference/src/organ/health.rs` (227 LOC)
- `crates/cynic-inference/src/organ/registry.rs` (35 LOC)
- `crates/cynic-inference/src/organ/tests.rs` (673 LOC — tests orphelins)

**Vérification préalable** :
```bash
# Confirmer 0 consommateur externe
grep -rn "cynic_inference::organ" crates/ --include='*.rs' | grep -v "cynic-inference/src/"
# Résultat attendu : 0 hits
```

**Action** :
1. Supprimer le répertoire `crates/cynic-inference/src/organ/`
2. Mettre à jour `crates/cynic-inference/src/lib.rs` — retirer `pub mod organ;`
3. Mettre à jour tout fichier qui importe depuis `organ/` (vérifier avec `grep`)

```bash
git rm -r crates/cynic-inference/src/organ/
# Mettre à jour lib.rs
# Mettre à jour les imports
```

**Livrable** : `organ/` supprimé. 564 LOC de dette en moins. Preuve : `cargo check -p cynic-inference` → 0 errors.

---

### Phase 2 — Nettoyer `root/` (cosmétique, mécanique)

**Problème** : 7 fichiers root/ sont des concerns infrastructure/domain dumpés au mauvais endroit.

**Fichiers à déplacer** :

| Fichier | Destination | Raison |
|---------|-------------|--------|
| `config_loader.rs` | `infra/` | I/O (lit fichiers config) |
| `routing_calc.rs` | `infra/` | Logique de routing (pas domaine pur) |
| `progress.rs` | `infra/` | Progression I/O |
| `ods_discovery.rs` | `infra/` | Découverte I/O |
| `vector_memory.rs` | `infra/` | Stockage vectoriel |
| `verdict_cache.rs` | `infra/` | Cache I/O |
| `embedding.rs` | `infra/` | Wrapper embedding |
| `inference.rs` | `infra/` | Wrapper inference |

**Action** :
```bash
# Déplacer les fichiers
git mv crates/cynic-inference/src/config_loader.rs crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/routing_calc.rs crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/progress.rs crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/ods_discovery.rs crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/vector_memory.rs crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/verdict_cache.rs crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/embedding.rs crates/cynic-inference/src/infra/
git mv crates/cynic-inference/src/inference.rs crates/cynic-inference/src/infra/

# Mettre à jour infra/mod.rs
# Mettre à jour lib.rs (retirer les pub use root)
```

**Livrable** : `src/` ne contient plus que `domain/`, `infra/`, `adapters/`, `ports/`, `math/`, `application/`, `lib.rs`. Preuve : `cargo check -p cynic-inference` → 0 errors.

---

### Phase 3 — Purger `application/` (dégraissage)

**Problème** : `application/` est un fourre-tail de 32 fichiers. Certains chevauchent le bus, d'autres sont des use cases légitimes.

**Stratégie** : Ne PAS créer de nouveau crate. Dégraisser et réorganiser.

**Fichiers à supprimer** (chevauchement bus ou dette) :

| Fichier | Raison |
|---------|--------|
| `fleet_routing.rs` | Chevauche `cynic-agent-bus-coordinator` — le routing appartient au coordinator |
| `multi_agent_service.rs` | Chevauche le bus — l'orchestration agentique appartient au bus |
| `proxy_routing.rs` | Politique opérationnelle durcie — à externaliser en config |
| `node_runtime_ops.rs` | Spécifique au node — appartient à `cynic-node` |

**Fichiers à garder dans `application/`** (use cases légitimes) :

| Fichier | Raison |
|---------|--------|
| `chat_service.rs` | Use case chat |
| `embedding_runtime.rs` | Use case embedding |
| `summarization_service.rs` | Use case summarization |
| `inference_service.rs` | Use case inference |
| `inference_gateway.rs` | Gateway HTTP |
| `model_catalog.rs` | Catalogue de modèles |
| `model_process_pool.rs` | Pool de processus modèles |
| `model_store_service.rs` | Service de stockage modèles |
| `model_sync_pipeline.rs` | Pipeline de sync modèles |
| `peer_attach.rs` | Attachement de pairs |
| `profile_bootstrap.rs` | Bootstrap de profil |
| `engine_selector.rs` | Sélection d'engine |
| `model_selection.rs` | Sélection de modèle |
| `backend_verification.rs` | Vérification de backend |
| `failure_gallery.rs` | Galerie d'échecs |
| `hf_discovery.rs` | Découverte HuggingFace |
| `hermes_runtime_service.rs` | Service runtime Hermes |
| `local_runtime_service.rs` | Service runtime local |
| `runtime_planner.rs` | Planificateur de runtime |
| `routing_policy.rs` | Politique de routing |
| `vault_consolidate*.rs` | Consolidation de vault |
| `agent_harness/` | Harness de benchmark |
| `bench_basket_launcher.rs` | Lanceur de benchmarks |
| `matrix_sweep_executor.rs` | Exécuteur de sweeps |
| `matrix_live_runner.rs` | Runner live |

**Action** :
```bash
# Supprimer les fichiers en chevauchement
git rm crates/cynic-inference/src/application/fleet_routing.rs
git rm crates/cynic-inference/src/application/multi_agent_service.rs
git rm crates/cynic-inference/src/application/proxy_routing.rs
git rm crates/cynic-inference/src/application/node_runtime_ops.rs

# Mettre à jour application/mod.rs
```

**Livrable** : `application/` passe de 32 à 28 fichiers. Zéro chevauchement avec le bus. Preuve : `cargo check -p cynic-inference` → 0 errors.

---

### Phase 4 — Renforcer `cynic-agent-bus` (hexagonal strict)

**Problème** : Le bus ré-exporte des types concrets (`InMemoryLedger`) et le serveur dépend de ces concrets.

**Action 4.1** — Déplacer les implémentations concrètes dans `adapters/` :

```
cynic-agent-bus/
├── src/
│   ├── lib.rs
│   ├── types.rs
│   ├── identity.rs
│   ├── proof.rs
│   ├── claim.rs
│   ├── ledger.rs          ← trait uniquement
│   ├── ports.rs           ← traits uniquement
│   └── adapters/          ← NOUVEAU
│       ├── mod.rs
│       ├── in_memory_ledger.rs
│       └── in_memory_claim_store.rs
```

```bash
# Créer le répertoire adapters/
mkdir -p crates/cynic-agent-bus/src/adapters

# Déplacer les implémentations concrètes
git mv crates/cynic-agent-bus/src/ledger.rs crates/cynic-agent-bus/src/adapters/in_memory_ledger.rs
git mv crates/cynic-agent-bus/src/claim.rs crates/cynic-agent-bus/src/adapters/in_memory_claim_store.rs

# Créer un nouveau ledger.rs avec juste le trait
# Créer un nouveau claim.rs avec juste le trait
```

**Action 4.2** — Mettre à jour `lib.rs` :

```rust
// AVANT
pub mod claim;
pub mod ledger;
pub use claim::*;
pub use ledger::*;

// APRÈS
pub mod adapters;
pub mod identity;
pub mod proof;
pub mod types;
pub mod ports;

// Ré-exports explicites (pas de glob)
pub use identity::{generate_keypair, agent_id_from_pubkey};
pub use proof::{sha256_bytes, sha256_hex, Proof};
pub use types::*;

// Traits
pub use ports::{
    AgentExecutor, CoordinatorReasoner, IdentityProvider,
    AgentError, IdentityError, ReasoningError,
    SessionId, AgentKind, Evaluation, SuggestedAction,
};
```

**Action 4.3** — Corriger `cynic-agent-bus-server` :

```rust
// AVANT (handlers.rs)
use cynic_agent_bus::*;
// ...
State(ledger): State<Arc<InMemoryLedger>>,

// APRÈS
use cynic_agent_bus::{Ledger, InMemoryLedger};
// ...
State(ledger): State<Arc<dyn Ledger>>,
```

**Livrable** : Bus = traits + types purs. Serveur = dépend du trait. Preuve : `cargo check -p cynic-agent-bus -p cynic-agent-bus-server` → 0 errors.

---

### Phase 5 — Renforcer les tests boundary

**Problème** : Les tests `architecture_boundary.rs` ne vérifient pas le DAG, les imports cross-module, ou les dépendances concrètes.

**Action** — Ajouter des tests dans `cynic-agent-bus/tests/architecture_boundary.rs` :

```rust
#[test]
fn bus_lib_does_not_re_export_concrete_types() {
    let lib = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"));
    assert!(
        !lib.contains("pub use ledger::*"),
        "lib.rs must not glob-re-export ledger (concrete types leak)"
    );
    assert!(
        !lib.contains("pub use claim::*"),
        "lib.rs must not glob-re-export claim (concrete types leak)"
    );
}

#[test]
fn concrete_impls_live_in_adapters() {
    let ledger = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/adapters/in_memory_ledger.rs"));
    assert!(
        ledger.contains("impl Ledger for InMemoryLedger"),
        "InMemoryLedger must impl Ledger trait in adapters/"
    );
}

#[test]
fn server_depends_on_trait_not_concrete() {
    // Vérifier que le serveur utilise Arc<dyn Ledger> et non Arc<InMemoryLedger>
    // (test indirect via cargo check — si ça compile, c'est bon)
}
```

**Livrable** : Tests boundary renforcés. Preuve : `cargo test -p cynic-agent-bus --test architecture_boundary` → 0 failures.

---

### Phase 6 — Corriger le coordinator (DAG strict)

**Problème** : Le coordinator utilise des types concrets (`InMemoryLedger`) et ne wire pas le trait `CoordinatorReasoner`.

**Action 6.1** — Injecter les traits dans le coordinator :

```rust
// coordinator.rs — AVANT
pub struct Coordinator {
    ledger: Arc<InMemoryLedger>,
    claim_store: Arc<InMemoryClaimStore>,
    ...
}

// coordinator.rs — APRÈS
pub struct Coordinator {
    ledger: Arc<dyn Ledger>,
    claim_store: Arc<dyn ClaimStore>,
    ...
}
```

**Action 6.2** — Ne PAS ajouter `SovereignRuntimePort` (prématuré). Le coordinator utilise `CoordinatorReasoner` (déjà dans `ports.rs`).

**Action 6.3** — Mettre à jour `Cargo.toml` du coordinator :

```toml
# AVANT
[dependencies]
cynic-agent-bus = { path = "../cynic-agent-bus" }

# APRÈS
[dependencies]
cynic-agent-bus = { path = "../cynic-agent-bus" }
# PAS de dépendance cynic-inference — le coordinator ne connaît pas le runtime
```

**Livrable** : Coordinator = orchestration pure, dépend des traits du bus. Preuve : `cargo check -p cynic-agent-bus-coordinator` → 0 errors.

---

### Phase 7 — Validation finale

**Checklist** :

```bash
# 1. Tous les crates compilent
cargo check -p cynic-inference
cargo check -p cynic-agent-bus
cargo check -p cynic-agent-bus-server
cargo check -p cynic-agent-bus-coordinator
cargo check -p cynic-kernel

# 2. Tests boundary passent
cargo test -p cynic-agent-bus --test architecture_boundary
cargo test -p cynic-agent-bus-coordinator

# 3. Pas de cycle DAG
# (vérification manuelle : aucun crate ne dépend d'un crate qui dépend de lui)

# 4. Zéro dette déplacée
# (organ/ supprimé, pas déplacé)
# (application/ dégraissé, pas extrait dans un crate vide)

# 5. Kernel compile
cargo check -p cynic-kernel
```

**Livrable** : Build complet vert. Preuve : `cargo check` (workspace) → 0 errors.

---

## CE QUI EST DÉFERRÉ (pas dans ce plan)

| Item | Raison |
|------|--------|
| `cynic-inference-runtime` crate | Pas de frontière nouveau crate défini. Trop prématuré. |
| `SovereignRuntimePort` trait | 0 implémentations. Prématuré. |
| Fine-tuning pipeline | Hors scope. Nécessite dataset mining + training infrastructure. |
| `cynic-fleet` crate (split organ/) | `organ/` est tué, pas splité. |
| A/B testing, canary, rollback | Features futures, pas architecture. |
| Memory estimator fix (20B+) | Nécessite un vrai registry d'architectures. |

---

## RÉSUMÉ DES CHANGEMENTS

| Phase | Action | LOC impact | Fichiers touchés |
|-------|--------|------------|------------------|
| 0 | Réparer kernel | +2 imports | 2 fichiers kernel |
| 1 | Tuer `organ/` | -564 LOC | 5 fichiers supprimés |
| 2 | Nettoyer `root/` | 0 LOC (déplacement) | 8 fichiers déplacés |
| 3 | Purger `application/` | -4 fichiers | 4 fichiers supprimés |
| 4 | Renforcer bus | +50 LOC (adapters/) | 5 fichiers bus + serveur |
| 5 | Renforcer tests | +30 LOC | 1 fichier test |
| 6 | Corriger coordinator | +20 LOC | 1 fichier coordinator |
| 7 | Validation | 0 | 0 |

**Total** : ~564 LOC supprimés, ~100 LOC ajoutés, 25+ fichiers modifiés/supprimés.

---

## POST-REFACTOR (prochaine session)

Une fois le build stabilisé :
1. Définir la frontière `cynic-inference-runtime` (si toujours nécessaire)
2. Ajouter `SovereignRuntimePort` (quand 2+ impls existent)
3. Builder le fine-tuning pipeline
4. Corriger le memory estimator (20B+)
5. Ajouter A/B testing / canary

---

*Plan corrigé. Exécution phase par phase avec validation intermédiaire.*
