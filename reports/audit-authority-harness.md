# Audit B — Authority & Harness

**Date :** 2026-08-20  
**Scope :** Déclaration, validation et promotion des harness kinds et autorités  
**Authority :** Read-only audit, no code modified  
**Constitution ref :** §5 Admission Gate, §7 Harness Binding, AGENTS.md Runtime Harness

---

## Files audited

| File | Lines | Role |
|---|---|---|
| `crates/cynic-protocol/src/harness.rs` | 717 | Build harness scenarios + HarnessAuthority enum (4 variants) |
| `crates/cynic-host/src/runtime_harness.rs` | 433 | Runtime harness contexts + HarnessAuthority enum (5 variants) |
| `crates/cynic-host/src/harness_progression.rs` | 332 | Progression decision pure + required proofs per kind |
| `crates/cynic-host/src/update_lifecycle/promote.rs` | 92 | Phase promotion with --yes gate |
| `crates/cynic-cli/src/runtime_surface.rs` | ~600 | CLI wiring of progression |
| `crates/cynic-cli/src/runtime_surface/host_checks.rs` | 92 | CLI: cynic runtime harness --json, conformance, metrics |
| `crates/cynic-cli/src/mask_surface.rs` | 140 | CLI: cynic mask → AgentCitizenship (scope, budgets, no harness) |
| `crates/cynic-cli/src/lifecycle_surface_render.rs` | 234 | Human-readable render of lifecycle status + progression |
| `crates/cynic-node/src/network_mesh_apply_plan.rs` | ~265 | Uses RuntimeHarnessKind::ClosedLocalLab |
| `crates/cynic-cli/src/network_surface/mesh.rs` | ~91 | Exposes harness_kind in CLI |

---

## Questions & Answers

### 1. Is `HarnessAuthority` used consistently across `harness.rs` (build) and `runtime_harness.rs` (runtime), or are they two separate enums?

**Réponse : Deux enums séparés, incompatibles.**

**Build harness (`harness.rs:42-47`) :**
```rust
pub enum HarnessAuthority {
    ConformanceGate,
    StagingObservation,
    ExplicitCanary,
    ReadOnlyProjection,
}
```

**Runtime harness (`runtime_harness.rs:67-73`) :**
```rust
pub enum HarnessAuthority {
    TestProjection,
    ConformanceGate,
    ReleaseCandidate,
    FleetCanary,
    RuntimeAuthority,
}
```

**Différences :**
- Build utilise `StagingObservation` / `ExplicitCanary` / `ReadOnlyProjection`
- Runtime utilise `TestProjection` / `ReleaseCandidate` / `FleetCananry` / `RuntimeAuthority`
- Le seul variant commun est `ConformanceGate`
- Build n'a pas `TestProjection` ou `ReleaseCandidate`
- Runtime n'a pas `ReadOnlyProjection` ou `ExplicitCanary`

**Conséquence :** Il n'y a pas de type unifié d'autorité. Une fonction qui accepte `harness::HarnessAuthority` ne peut pas recevoir `runtime_harness::HarnessAuthority` et vice versa. La comparaison d'autorité entre build et runtime est impossible sans mapping ad-hoc.

**Gap majeur :** Le meta-harness (constitution §7) ne peut pas agréger les autorités de build et runtime car les types sont disjoints.

---

### 2. Does the CLI ever surface harness kind to the operator, or is it internal only?

**Réponse : Oui, partiellement.**

**Surface live :**
- `cynic-cli/src/runtime_surface/host_checks.rs:9-17` — `execute_harness()` appelle `runtime_harness_catalog(kind)` et émet le JSON du catalogue
- `cynic-cli/src/runtime_surface/host_checks.rs:19-31` — `execute_conformance()` montre le rapport de conformance
- `cynic-cli/src/mask_surface.rs:98-107` — expose `scope: "node"` et budgets dans l'AgentCitizenship JSON
- `cynic-cli/src/lifecycle_surface_render.rs:6-51` — affiche `report.policy.harness.authority` et la progression
- `cynic-cli/src/network_surface/mesh.rs:43,52,81,91` — expose `harness_kind` dans les commandes mesh

**Ce qui est visible :**
- `cynic runtime harness --json [--kind <kind>]` → liste les contextes disponibles
- `cynic runtime conformance --json` → montre le statut de qualification
- `cynic lifecycle status --json` → montre l'autorité actuelle et la progression
- `cynic mask forge/start` → montre scope + budgets (mais pas le harness kind)

**Ce qui n'est PAS visible :**
- Le `harness_kind` actuel du nœud (on sait qu'il est dans le registry.json, mais pas exposé directement)
- L'autorité effective calculée (elle est dans le registry mais pas dans un rapport read-only)
- Les gaps de preuve entre le kind actuel et le kind suivant

---

### 3. Is the promotion flow (lab → production) implemented anywhere, or only documented?

**Réponse : Oui, implémenté partiellement.**

**Decision pure (Live) :**  
`cynic-host/src/harness_progression.rs:44-160` :
- `decide_harness_progression()` — fonction pure qui décide si on peut progresser
- `required_proof_categories(kind)` — quelles preuves requises par niveau (lignes 92-129)
- `authority_for_kind(kind)` — associe chaque kind à son autorité (lignes 132-140)
- `is_single_step()` — un seul échelle à la fois (lignes 143-152)
- `HarnessProgressionVerdict::Allowed/Blocked` avec raisons

**Required proofs par niveau :**
| Kind | Required proofs |
|---|---|
| ClosedLocalLab | (none) |
| Conformance | Bootstrap, Hardware |
| Staging | + HostConformance, BuildReproduction, Network |
| Canary | + TargetAttestation |
| Production | + RollbackIntent |

**State transition (Live) :**  
`cynic-host/src/update_lifecycle/promote.rs:9-92` :
- `promote(state_path, to, yes)` — transition de phase
- Requiert `target.index() == from.index() + 1` (lignes 23-38)
- Requiert `last_gate_ok == Some(true)` (lignes 40-50)
- Requiert `--yes` explicite (lignes 52-66)
- Met à jour `state.current_phase` et persiste

**CLI wiring (Live) :**  
`cynic-cli/src/runtime_surface.rs:569-582` :
- `RuntimeHarnessKindArg` (CLI arg) → `RuntimeHarnessKind` (protocol)
- Construit `HarnessProgressionEvidence` + appelle `decide_harness_progression()`

**Gap :** La progression n'est pas liée à l'admission gate. `decide_harness_progression()` est appelée par le CLI pour affichage, mais aucune mutation surface ne vérifie "le kind actuel autorise cette mutation" avant d'agir. Un agent en ClosedLocalLab peut théoriquement appeler une API qui muterait la production si le network le permet.

---

### 4. Can a citizen operate without declaring a harness?

**Réponse : Oui, via le mask surface.**

**Mask surface (`mask_surface.rs:98-125`) :**
- `AgentCitizenship` (struct local au CLI) a `schema`, `agent_id`, `scope`, `budgets`, `residual_risk`
- **Pas de champ `harness_kind`** — le citoyen est créé sans harness explicite
- Le scope est hardcodé à `"node"` (ligne 101, 120)
- Les budgets sont hardcodés à `actions: 1000, wall_seconds: 3600` (lignes 102, 121)

**AgentCitizenshipRecord (protocol) :**
- `agent_citizenship.rs:117-127` — a `schema`, `agent_id`, `scope`, `budgets`, `residual_risk`, `evidence`
- **Pas de champ `harness_kind`** non plus

**Harness binding implicite :**
- `harness_progression.rs:8` — `pub use crate::runtime_harness::{HarnessAuthority, RuntimeHarnessKind}` — le harness kind est accessible mais n'est pas dans le citoyen
- Le registry.json définit le kind du nœud, mais un citoyen local n'a pas à le déclarer

**Gap constitutionnel :** La constitution §7 exige que "toute mutation soit bound à un harness déclaré". Actuellement, un citoyen peut être créé et opérer sans que son harness soit vérifié. Le `admit_mutation()` gate ne vérifie pas le harness kind du citoyen cible.

---

### 5. What surfaces mutate real state without `explicit_apply_required = true`?

**Réponse : Plusieurs.**

**Mutation surfaces auditées :**

| Surface | `--apply` / `--yes` | explicit_apply_required | Mutation |
|---|---|---|---|
| `update_lifecycle/promote.rs` | `--yes` requis (ligne 52) | N/A | Oui (state persist) |
| `mask_surface.rs:start` | `--yes` requis (ligne 113) | N/A | Oui (inference start) |
| `mask_surface.rs:forge` | `--yes` pour hand-off (ligne 88) | N/A | Oui (forge + start) |
| `build_scheduler/lifecycle.rs:claim_if_available` | Aucun gate d'opérateur | N/A | Oui (CAS sur store) |
| `agent_workload/lifecycle.rs:claim_agent_workload` | Aucun gate d'opérateur | N/A | Oui (CAS sur store) |
| `runtime_surface/host_checks.rs:execute_harness` | `--json` only | N/A | Non (read-only) |
| `runtime_surface/host_checks.rs:execute_conformance` | `--json` only | N/A | Non (read-only) |

**Gap critique :**
- Le build scheduler (`claim_if_available`) et le workload lifecycle (`claim_agent_workload`) font des mutations durables sur le store **sans gate d'opérateur explicite**. Ils sont appelés automatiquement par la boucle du scheduler.
- Ces mutations sont "automatiques" par design (le scheduler est souverain), mais elles ne passent pas par un gate `--apply`/`--yes` explicite.
- `validate_harness_scenario` (harness.rs:311-351) définit `explicit_apply_required` sur les scénarios build, mais ce champ n'est pas câblé dans le scheduler runtime.

---

## Summary of gaps

| # | Gap | Severity | Constitution ref |
|---|---|---|---|
| 1 | Deux `HarnessAuthority` enums séparés (build vs runtime), incompatibles | Critical | §7 Harness Binding |
| 2 | `harness_kind` absent de `AgentCitizenshipRecord` et `AgentCitizenship` CLI | High | §7 Harness Binding |
| 3 | Progression flow est une décision pure non câblée dans l'admission gate | High | §7 Harness Binding |
| 4 | Scheduler claim workload sans gate d'opérateur explicite | Medium | §5 Bounded Action |
| 5 | `RuntimeHarnessKind` exposé dans CLI mais pas l'autorité effective calculée | Low | §7 Harness Binding |
| 6 | `required_proof_categories` n'est pas utilisé par les mutation surfaces pour vérifier l'admission | High | §7 Harness Binding |

---

## Recommendations for U2 (Authority Enum Unification)

1. Créer un seul `HarnessAuthority` enum dans `cynic-protocol` avec les 5 variants unifiés : `TestProjection`, `ConformanceGate`, `ReleaseCandidate`, `FleetCanary`, `RuntimeAuthority`
2. Déplacer `RuntimeHarnessKind` dans `cynic-protocol` (ou re-export depuis un module partagé)
3. Ajouter `harness_kind: RuntimeHarnessKind` dans `AgentCitizenshipRecord` (et son équivalent CLI)
4. Câbler `required_proof_categories()` dans l'admission gate — un citoyen ne peut pas muter si les preuves requises pour son harness kind ne sont pas satisfaites
5. Ajouter un champ `explicit_apply_required: bool` à `AgentCitizenshipRecord` et le vérifier dans `admit_mutation()`
