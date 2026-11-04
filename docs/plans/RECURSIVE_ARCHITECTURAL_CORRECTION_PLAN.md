# PLAN VIVANT : Boucle Récursive d'Assainissement Architectural (Pattern Ralph / Goal Loop)

**Schema**: `cynic.plan.living_recursive_correction.v1`  
**Autorité**: `docs/specs/SPEC_STRUCTURAL_CORRECTION_COMPASS.md`, `contracts/glossary.json`, `AGENTS.md`  
**Principe directeur**: Plan vivant alimenté par la preuve matérielle (PoW), le feedback machine en boucle fermée, et l'intégration des émergences inconnues.

---

## 1. Philosophie & Dynamic Feedback Loop

Ce plan n'est pas une liste statique de tâches rigides. C'est un **état vivant du système** :
- **Entrée** : L'état mesuré du code à chaque itération (AST, graphe Cargo, LOC, audit unifié).
- **Transformation** : Exécution d'un cycle en 4 phases (Domain Pull, Canonicalisation, Éradication de Shims, Preuve distante).
- **Adaptation aux émergences** : Tout conflit imprévu, couplage caché, ou dépendance cyclique découverte pendant le vol est immédiatement journalisé dans la section *Journal des Émergences & Décisions* ci-dessous et ajuste l'ordre d'évaluation du cycle suivant.
- **Sortie / Critère d'arrêt** : Vérification stricte par les gates machines (0 dette, 0 violation, réduction mesurée du graphe).

---

## 2. État Vivant du Graphe & Cibles d'Assainissement

### Métriques initiales de la boucle (Commit `32c2ebc3d`)
- **Arêtes runtime Cargo totales** : 161 arêtes
- **Arêtes sortantes `cynic-cli`** : 29 dépendances directes
- **Poids `cynic-host`** : 72 195 LOC (107 modules)
- **Poids `cynic-node`** : 51 019 LOC
- **Poids `cynic-protocol`** : 43 847 LOC (>150 modules racine plats)
- **Audit unifié** : 100% maturité, 0 dette, 0 violation.

---

## 3. Les Horizons du Plan Vivant

### Horizon 1 : Éradication Transversale des Shims (`cynic-cli`)
**Objectif** : Couper les façades de ré-export et faire chuter les arêtes artificielles de `cynic-cli`.
- [ ] **Cycle 1.1 — Shims Ledgers** : Supprimer `task_ledger.rs`, `idea_ledger.rs`, `coordination_ledger.rs` de `cynic-cli/src/`. Rebrancher les surfaces directement sur `cynic-agent` et `cynic-knowledge`.
- [ ] **Cycle 1.2 — Shims Écosystème Pkg** : Supprimer `deps_surface_specs.rs`, `deps_surface_pins.rs`, `deps_surface_plan.rs` de `cynic-cli/src/`. Rebrancher sur `cynic-pkg`.
- [ ] **Cycle 1.3 — Découplage Cargo** : Retirer les dépendances superflues dans `crates/cynic-cli/Cargo.toml` et mesurer la baisse des arêtes.

### Horizon 2 : Dégraissage Substrat & Sécurité (`cynic-host`)
**Objectif** : Sortir les domaines non-substrat hors de L1. Ramener `cynic-host` vers < 30k LOC.
- [ ] **Cycle 2.1 — Secrets & Credentials** : Migrer `credential.rs`, `credential_provisioning.rs`, `credential_hardening/`, `secret_material.rs` vers `cynic-secrets`.
- [ ] **Cycle 2.2 — Agent Access & Workloads** : Déplacer `agent_access/` vers `cynic-agent`.
- [ ] **Cycle 2.3 — Purge des modules zombies** : Nettoyer les reliquats d'observateurs de flotte et de tunnels migrés.

### Horizon 3 : Partitionnement DDD & Canonicalisation (`cynic-protocol` & `cynic-types`)
**Objectif** : Structurer le sac plat de L0 et unifier les types de base.
- [ ] **Cycle 3.1 — Unification atomique L0** : Raccorder tous les structs de protocole sur les primitives de `cynic-types` (`AgentId`, `TaskId`, `Hash32`, `Scope`).
- [ ] **Cycle 3.2 — Partitionnement modulaire** : Grouper les 150 modules plats en sous-domaines (`protocol::agent::*`, `protocol::build::*`, `protocol::substrate::*`).
- [ ] **Cycle 3.3 — Alignement Glossaire** : Vérification systématique contre `contracts/glossary.json`.

### Horizon 4 : Autonomie Cluster & Découplage Démon (`cynic-node`)
**Objectif** : Isoler le daemon mesh pur de la facturation et du storage.
- [ ] **Cycle 4.1 — Facturation** : Extraire `finance_adapters/` vers `cynic-market`.
- [ ] **Cycle 4.2 — Coordination Mesh pure** : Sanctuariser le rôle réseau p2p du daemon.

---

## 4. Protocole de Gestion des Émergences Inconnues

Lorsqu'un agent ou sous-agent rencontre une anomalie non prévue :
1. **Règle Anti-Silencieuse** : Ne jamais forcer un hack ou un shim pour "faire passer le build".
2. **Journalisation Machine** : Inscrire l'inconnu découvert dans la section 5 ci-dessous avec :
   - Fichiers impliqués
   - Symptôme / Blocage
   - Décision d'arbitrage conforme au `SPEC_STRUCTURAL_CORRECTION_COMPASS.md`.
3. **Mise à jour dynamique de la cible** : Ajuster l'ordre des cycles du plan sans demander de permission si l'arbitrage respecte le DAG strict et FCIS.

---

## 5. Journal Vivant des Émergences & Arbitrages (PoW)

*(Ce journal est incrémenté en continu à chaque itération de la boucle)*

| Date / PR | Composant | Découverte / Émergence Inconnue | Arbitrage appliqué (SSOT) |
|---|---|---|---|
| **2026-11-04 (PR #700)** | `cynic-knowledge/src/lib.rs` | Conflit sur double import `Path` et module `service.rs` fantôme issu d'une branche avortée. | Purge immédiate de `service.rs`, normalisation des imports avec `WalkDir` restauré. |
| **2026-11-04 (PR #695)** | `cynic-host::toolchain` | Dépendance L1 $\to$ L2 cachée par des shims dans `cynic-host`. | Rapatriement pur et simple de tout le cycle de vie toolchain dans `cynic-build` (L2). |
| **2026-11-04 (PR #691)** | `infra/crate-topology.json` | Génération de topologie via `cargo metadata` prenait 15s par PR, causant des timeouts. | Création de `scripts/sync-topology.py` en Python pur (50ms). |
| **2026-11-04 (Cycle 2.1)** | `cynic-host` → `cynic-secrets` | Hoist de `credential.rs`, `credential_provisioning.rs`, `credential_hardening/`, `secret_material.rs`. Seul appelant externe était `cynic-cli/src/build_surface/services.rs`. Dépendances ajoutées à `cynic-secrets`: `cynic-process`, `rpassword`, `base64`. `cynic-host` perd 4 modules et le shim `secret_material`. | Migration réussie vers `cynic-secrets::credential::{types, provisioning, hardening}`. Rewiring de `cynic-cli` vers `cynic_secrets`. Suppression des fichiers source dans `cynic-host`. |
