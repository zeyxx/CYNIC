# Slice A — kernel/organ backend-health types → cynic-inference

But: résorber le doublon #1 (infra/CRATE_OWNERSHIP.md §doublon1).
Règle: foundation §481 one_boundary_one_reason. Cible existe déjà
(cynic-inference possède backend_identity/routing/model_registry).

PRÉCONDITIONS VÉRIFIÉES (observe):
- kernel Cargo.toml:13 → `cynic-inference = { path = "../cynic-inference" }`
  DÉJÀ une dep. Aucune nouvelle dep à ajouter.
- kernel `crate::organ::InferenceOrgan` (organ/mod.rs:142) = registry
  Dogs (backend health tracking). Doublon conceptuel avec inference.
- kernel `domain::organ::OrganPort` (introspection/senses) = COEUR
  kernel, NE PAS extraire. On touche SEULEMENT
  `crate::organ::{InferenceOrgan, registry, health}`.

PIÈCE DÉPLACÉE (types purs, pas de couplage main.rs):
1. crates/cynic-kernel/src/organ/registry.rs (58 LOC)
     → FUSIONNER dans crates/cynic-inference/src/domain/backend_identity.rs (33 LOC)
     Surface cible VÉRIFIÉE: BackendHealthSnapshot, BackendIdentity, BackendCapability, BackendStatus.
     Le kernel Backend/BackendHealth/BackendId doit soit fusionner dans backend_identity
     (BackendHealthSnapshot existe déjà et couvre le besoin), soit l'étendre.
     JAMAIS créer un module backend_health parallèle à backend_identity.
2. crates/cynic-kernel/src/organ/health.rs (227 LOC)
     → crates/cynic-inference/src/domain/backend_health.rs (append)
     type: ParseFailureGate (sliding-window).
     NOTE: health.rs ré-exporte `crate::domain::dog_health::{DogStats,
     ScoreFailureKind}`. À remapper: inference doit posséder ces types
     OU les prendre en paramètre générique. Voir DÉCISION ci-dessous.

DÉCISION REQUISE (couplage DogStats):
- `ParseFailureGate` dépend de `ScoreOutcome`/`DogStats` (dog_health).
  Option A: déplacer dog_health.rs aussi vers inference (il est
    "scoring ML" → askesis? ou inference?). Risqué (couplage judge).
  Option B (RECOMMANDÉE): ParseFailureGate devient générique sur le
    type d'outcome, OU on laisse health.rs dans kernel mais il
    CONSOMME le type Backend/BackendHealth déplacé depuis inference.
    → Split propre: registry.rs (types) → inference; health.rs (gate)
    reste kernel mais importe les types depuis inference.

PLAN FINAL (split minimal, risque min):
- ÉTAPE A1: créer cynic-inference/src/domain/backend_health.rs avec
    BackendId, DeclaredCapabilities, MeasuredCapabilities, BackendHealth,
    Backend (copie de registry.rs). Exposer dans inference/lib.rs:
    `pub use domain::backend_health::{Backend, BackendHealth, ...};`
- ÉTAPE A2: kernel/organ/registry.rs DEVIENT un re-export:
    `pub use cynic_inference::backend_health::*;` (compat ascendante,
    zéro changement pour les 30+ callers).
- ÉTAPE A3: kernel/organ/health.rs importe BackendHealth depuis
    cynic_inference au lieu de le redéfinir. ParseFailureGate reste
    kernel (couplé à dog_health) — c'est correct: le GATE est
    kernel-specific, les TYPES sont canoniques inference.
- ÉTAPE A4: tests organ/mod.rs:369 (organ_starts_empty, etc.) continuent
    de passer (types identiques, juste relocalisés).

POURQUOI CE SPLIT:
- Respecte §468 (protocol pur) — on ne touche pas protocol.
- Respecte §481 — types backend health ont UNE source (inference).
- RISQUE MIN: A2 est un re-export, donc aucun caller kernel ne change.
  A1/A3 sont des déplacements de types purs.

GARDE-FOU (read-back proof):
- `cynic dev crate-audit --json` doit rester vert (schema
  cynic.cli.crate_audit.v1). kernel perd ~58 LOC (registry.rs devient
  re-export), inference gagne ~58 LOC.
- tests: `cargo test -p cynic-kernel organ::` + `cargo test -p
  cynic-inference` doivent passer. ⇒ REQUIERT rustc non-ICE.
  ENV BLOCKANT: rustc 1.96 SIGSEGV sur libc/syn sur cette VM. À exécuter
  sur builder souverain (10.10.10.119) ou après déblocage.

CHECKLIST APPLICATION (à faire quand rustc OK):
  [ ] A1: écrire inference/src/domain/backend_health.rs
  [ ] A1: exposer dans inference/lib.rs
  [ ] A2: kernel/organ/registry.rs → re-export cynic_inference
  [ ] A3: kernel/organ/health.rs → importe BackendHealth depuis inference
  [ ] A4: cargo test -p cynic-kernel organ:: (vert)
  [ ] cargo test -p cynic-inference (vert)
  [ ] git: commit slice A (2 fichiers inference + 2 kernel modifiés)
  [ ] mettre à jour CRATE_OWNERSHIP.md doublon1 → "résolu slice A"
