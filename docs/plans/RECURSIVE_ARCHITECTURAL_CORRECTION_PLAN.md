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
- [x] **Cycle 1.1 — Shims Ledgers & Pkg** (PR #703) : Supprimé `task_ledger.rs`, `idea_ledger.rs`, `coordination_ledger.rs`, `deps_surface_specs.rs`, `deps_surface_pins.rs`. Rebranché sur `cynic-agent`, `cynic-knowledge`, `cynic-pkg`.
- [x] **Cycle 1.2 — Découplage Cargo cynic-cli** (PR #711) : Supprimé `cynic-intel` et `cynic-traces`. Arêtes du runtime réduites de 163 à 161.

### Horizon 2 : Dégraissage Substrat & Sécurité (`cynic-host`)
**Objectif** : Sortir les domaines non-substrat hors de L1. Ramener `cynic-host` vers < 30k LOC.
- [x] **Cycle 2.1 — Secrets & Credentials** (PR #704) : Migré `credential.rs`, `credential_provisioning.rs`, `credential_hardening/`, `secret_material.rs` vers `cynic-secrets`.
- [ ] **Cycle 2.2 — Agent Access & Workloads** : Évaluation d'arbitrage : gestion des identités POSIX hôte maintenue dans `cynic-host` pour préserver le DAG strict.
- [x] **Cycle 2.3 — Purge des modules zombies** (PR #712) : Supprimé `fleet_discovery.rs`, `fleet_heartbeat.rs`, `tunnel_supervisor.rs` (-481 LOC).

### Horizon 3 : Partitionnement DDD & Canonicalisation (`cynic-protocol` & `cynic-types`)
**Objectif** : Structurer le sac plat de L0 et unifier les types de base.
- [x] **Cycle 3.1 — Unification atomique L0** (PR #710, #715) : Enrichi `cynic-types::AgentId` / `TaskId` et raccordé sur `cynic-agent::task_ledger` et `cynic-protocol::AgentIdentity`.
- [ ] **Cycle 3.2 — Partitionnement modulaire** : Grouper les 150 modules plats en sous-domaines (`protocol::agent::*`, `protocol::build::*`, `protocol::substrate::*`).
- [ ] **Cycle 3.3 — Alignement Glossaire** : Vérification systématique contre `contracts/glossary.json`.

### Horizon 4 : Autonomie Cluster & Découplage Démon (`cynic-node`)
**Objectif** : Isoler le daemon mesh pur de la facturation et du storage.
- [x] **Cycle 4.1 — Facturation** (PR #713) : Supprimé `finance_adapters.rs` et dépendance `cynic-finance` dans `cynic-node`. Arêtes réduites à 160.
- [ ] **Cycle 4.2 — Coordination Mesh pure** : Sanctuariser le rôle réseau p2p du daemon.

### Horizon 5 : Tranches Verticales Transversales & Invariants Substrat (NOUVEAU)
**Objectif** : Éradiquer les réinventions de patterns de bas niveau à travers toutes les crates.
- [x] **Cycle 5.1 — SSOT SSH Transport (`CYNIC-ARCH-003`)** (PR #708, #709) : Éradiqué tout `vec!["ssh", ...]` manuel dans `cynic-proxmox` et `cynic-host` au profit de `cynic_ssh::argv::ssh_argv`.
- [x] **Cycle 5.2 — Unification des Types Substrat (`CYNIC-ARCH-006`)** (PR #710, #715) : Raccordement systématique des ID d'agents et tâches sur `cynic-types`.
- [ ] **Cycle 5.3 — Pureté FCIS & Runner Unique (`CYNIC-ARCH-002`)** : Baigner tous les adaptateurs I/O dans `cynic-process::ArgvRunner` et bannir les invocations I/O orphelines.

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
| **2026-11-04 (PR #706)** | `cynic-protocol` & `cynic-audit-engine` | Les récurrences de dettes architecturales étaient détectées au cas par cas sans identifiants canoniques. | Création de `CynicErrorDomain::Architecture` et des codes `CYNIC-ARCH-001` à `CYNIC-ARCH-006` dans `error_catalog.rs`. Câblage dans `cynic-audit-engine::debt_pattern_rule`. |
| **2026-11-04 (PR #708, #709)** | `cynic-proxmox`, `cynic-host` | Invariance SSH SSOT contournée par constructions `vec!["ssh", ...]` manuelles. | Raccordement systématique sur `cynic_ssh::argv::ssh_argv` (`CYNIC-ARCH-003`). |
| **2026-11-04 (PR #710, #715)** | `cynic-types`, `cynic-agent`, `cynic-protocol` | Identifiants d'agents/tâches manipulés comme `String` nues (`CYNIC-ARCH-006`). | Enrichissement et raccordement de `cynic_types::AgentId` / `TaskId` dans `task_ledger` et `AgentIdentity`. |
| **2026-11-04 (PR #711, #713)** | `cynic-cli`, `cynic-node` | Dépendances inutilisées (`cynic-intel`, `cynic-traces`, `cynic-finance`). | Éradication des modules morts et dépendances dans Cargo.toml. Runtime edges abaissées à 160. |
| **2026-11-04 (PR #712)** | `cynic-host` | 3 modules zombies (`fleet_discovery`, `fleet_heartbeat`, `tunnel_supervisor`) sans consommateurs. | Purge physique complète (-481 LOC) de `cynic-host`. |
| **2026-11-04 (PR #714)** | `cynic-audit-engine` | Absence d'audit automatisé des budgets de taille et de la dérive de glossaire. | Implémentation des règles `CYNIC-ARCH-005` (budget god-crate) et `CYNIC-ARCH-006` (glossaire). |
