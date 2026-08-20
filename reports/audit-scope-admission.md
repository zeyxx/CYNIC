# Audit D — Scope & Admission Gate

**Date :** 2026-08-20  
**Scope :** Utilisation d'`admit_mutation` dans les surfaces de mutation  
**Authority :** Read-only audit, no code modified  
**Constitution ref :** §4 Budget Monotonicity, §5 Admission Gate, §6 Scope Hierarchy

---

## Files audited

| File | Lines | Role |
|---|---|---|
| `crates/cynic-protocol/src/agent_citizenship.rs` | 355 | AgentScope, AgentMode, AgentBudgets, admit_mutation, budgets_within |
| `crates/cynic-node/src/usecases/builder_http.rs` | 283 | Réception webhook forgejo → création workload |
| `crates/cynic-node/src/usecases/build_workload_submission_uc.rs` | 180 | Soumission build workload |
| `crates/cynic-node/src/usecases/build_scheduler/mod.rs` | ~350 | Scheduler loop claim/execute/complete |
| `crates/cynic-node/src/usecases/node_enrollment_lifecycle_uc.rs` | 53 | Renouvellement enrollment |
| `crates/cynic-node/src/usecases/release_webhook.rs` | ~150 | Webhook release source sync |
| `crates/cynic-host/src/update_lifecycle/apply.rs` | 135 | Apply phase mutations |
| `crates/cynic-host/src/update_lifecycle/repair.rs` | ~200 | Repair mutations |
| `crates/cynic-host/src/update_lifecycle/repair_loop.rs` | ~100 | Boucle de repair |
| `crates/cynic-host/src/update_lifecycle/gate.rs` | ~100 | Gate vérifications |
| `crates/cynic-host/src/update_lifecycle/policy.rs` | ~200 | Politique de mutations autorisées par phase |
| `crates/cynic-host/src/update_lifecycle/port.rs` | ~50 | ArgvRunner port |

---

## Questions & Answers

### 1. How many mutation surfaces exist?

**Réponse : 12 surfaces de mutation identifiées.**

**Inventaire par type de mutation :**

| # | Surface | Fichier | Type | Cible |
|---|---|---|---|---|
| 1 | `builder_http.rs` — POST /webhook/forgejo | `cynic-node/src/usecases/builder_http.rs:61-216` | Création workload | Store local |
| 2 | `agent_workload_delivery_uc.rs` | `cynic-node/src/usecases/agent_workload_delivery_uc.rs` | Création delivery record | Store local |
| 3 | `build_scheduler/mod.rs` — claim | `cynic-node/src/usecases/build_scheduler/mod.rs` | Claim workload | Store local |
| 4 | `build_scheduler/mod.rs` — execute | `cynic-node/src/usecases/build_scheduler/mod.rs` | Exécution build | Process spawn |
| 5 | `build_scheduler/release_publication.rs` | `cynic-node/src/usecases/build_scheduler/release_publication.rs` | Publication release | Filesystem/HTTP |
| 6 | `node_enrollment_lifecycle_uc.rs` — renew | `cynic-node/src/usecases/node_enrollment_lifecycle_uc.rs:37-49` | Renouvellement enrollment | HTTP kernel |
| 7 | `release_webhook.rs` | `cynic-node/src/usecases/release_webhook.rs` | Source sync trigger | Filesystem/process |
| 8 | `update_lifecycle/apply.rs` — apply_phase | `cynic-host/src/update_lifecycle/apply.rs:17-135` | Application phase | Process spawn |
| 9 | `update_lifecycle/repair.rs` — repair_apply | `cynic-host/src/update_lifecycle/repair.rs:95-192` | Repair | Process spawn |
| 10 | `update_lifecycle/gate.rs` — run_gates (execute=true) | `cynic-host/src/update_lifecycle/gate.rs` | Gate execution | Process spawn |
| 11 | `update_lifecycle/promote.rs` — promote | `cynic-host/src/update_lifecycle/promote.rs` | Promotion phase | State file |
| 12 | `kernel_tunnel.rs` — reconcile | `cynic-host/src/kernel_tunnel.rs` | SSH tunnel spawn | Process spawn |

---

### 2. Which ones call `admit_mutation` (or equivalent) and which bypass it?

**Réponse : Aucune n'appelle `admit_mutation`.**

**Recherche exhaustive :**
```
grep -r "admit_mutation" crates/ --include="*.rs"
```
Résultat : Seulement dans `agent_citizenship.rs` (définition + tests).

**Ce qui existe à la place :**

| Surface | Gate alternatif | Vérifie scope ? | Vérifie budget ? |
|---|---|---|---|
| `builder_http.rs` | HMAC + commit SHA validation | Non | Non |
| `build_workload_submission_uc.rs` | `admit_build_agent_request` (local) | Non (pas de scope dans BuildAgentRequest) | Non (pas de budget dans admission) |
| `build_scheduler` | `claim_agent_workload` (fence + expiry) | Non | Oui (via `admit_workload_action`) |
| `node_enrollment` | `SecurityEvidenceState::Proven` check | Non | Non |
| `update_lifecycle/apply.rs` | `--yes` flag + phase policy | Non | Non |
| `update_lifecycle/repair.rs` | `--yes` flag + phase policy | Non | Non |
| `kernel_tunnel.rs` | Aucun gate (auto-reconcile) | Non | Non |

**Gap critique :** `admit_mutation` est la gate canonique définie dans `agent_citizensehip.rs` mais n'est appelée par aucune surface de mutation. C'est une fonction pure jamais câblée.

---

### 3. Is scope containment (Crate contains File) checked at every mutation surface, or only in tests?

**Réponse : Seulement dans les tests.**

**Recherche :**
```
grep -r "\.contains\(" crates/ --include="*.rs"
```
Résultat : Seulement dans `agent_citizenship.rs:319-323` (tests de `AgentScope::contains`).

**Aucune surface de mutation n'appelle `AgentScope::contains`.**

**Conséquence :** Un agent avec une claim `File` peut muter une `Crate` entière. Un agent avec une claim `Module` peut muter le `Node`. Aucun scope check n'est effectué dans le runtime.

---

### 4. Is budget monotonicity enforced, or only documented?

**Réponse : Enforcée dans 1 surface sur 12, partielle ailleurs.**

**Surface avec enforcement :**
- `build_scheduler` via `admit_workload_action` (`lifecycle.rs:71-87`) : vérifie `record.consumed_actions >= record.envelope.delegation.budget.actions` → violation `action_budget_exhausted`

**Surfaces sans enforcement :**
- `builder_http.rs` — pas de vérification de budget
- `build_workload_submission_uc.rs` — pas de vérification de budget dans l'admission
- `update_lifecycle/apply.rs` — pas de limite sur le nombre d'actions
- `update_lifecycle/repair.rs` — pas de limite
- `kernel_tunnel.rs` — pas de budget du tout

**Budget field exists but unused :**
- `AgentBudgets` dans `agent_citizensehip.rs:78-81` — défini mais jamais lu par les surfaces de mutation
- `AgentBudgets` dans `mask_surface.rs:34-37` — hardcodé à 1000/3600, jamais décrémenté

**Gap :** La monotonicité budgétaire est une règle documentée (constitution §4) mais non câblée dans 11/12 surfaces.

---

### 5. Which surfaces mutate without an explicit `--apply` or equivalent?

**Réponse : 7 surfaces sur 12.**

**Surfaces AVEC gate explicite (--yes / --apply) :**
- `update_lifecycle/apply.rs:57` — `--yes` requis pour appliquer
- `update_lifecycle/repair.rs:95` — `--yes` requis pour repair_apply
- `update_lifecycle/promote.rs:52` — `--yes` requis pour promouvoir
- `update_lifecycle/gate.rs` — `--execute` requis pour exécuter les gates
- `release_webhook.rs` — gate implicite via enrollment check

**Surfaces SANS gate explicite (auto-mutation) :**
- `builder_http.rs` — crée un workload dans le store dès réception webhook (ligne 212)
- `agent_workload_delivery_uc.rs` — crée un delivery record dès réception (ligne 263)
- `build_scheduler` — claim + exécution automatiques dans la boucle (lignes 111-150)
- `node_enrollment_lifecycle_uc.rs` — renouvellement automatique quand expiration proche (ligne 37)
- `kernel_tunnel.rs` — auto-reconcile du tunnel SSH (ligne 122)

**Justification design :** Le scheduler et le delivery sont conçus comme "souverains" — ils s'exécutent sans intervention humaine. C'est un choix de design, mais cela signifie que ces surfaces ne respectent pas le pattern "bounded action with explicit --apply".

**Gap :** La constitution §5 exige "bounded explicit apply". Les surfaces "souveraines" (scheduler, tunnel, enrollment renewal) sont des exceptions non-documentées.

---

## Summary of gaps

| # | Gap | Severity | Constitution ref |
|---|---|---|---|
| 1 | `admit_mutation` jamais appelée par aucune surface de mutation | Critical | §5 Admission Gate |
| 2 | Scope containment (`AgentScope::contains`) jamais vérifié en runtime | High | §6 Scope Hierarchy |
| 3 | Budget monotonicity non-câblée dans 11/12 surfaces | High | §4 Budget Monotonicity |
| 4 | 7/12 surfaces mutent sans gate `--apply` explicite | Medium | §5 Bounded Action |
| 5 | `admit_build_agent_request` n'utilise pas `admit_mutation` — gate local redondant | Medium | §6 One Canonical Decision |
| 6 | Gate d'admission redondant entre workload submission et scheduler | Medium | §6 One Canonical Decision |

---

## Recommendations for U3 (Admission Gate Integration)

1. **Câbler `admit_mutation`** dans `builder_http.rs`, `build_workload_submission_uc.rs`, `build_scheduler/mod.rs` avant toute mutation
2. **Remplacer `admit_build_agent_request`** par un appel à `admit_mutation` (ou étendre `admit_mutation` pour inclure les checks actuels de `BuildAgentRequest`)
3. **Ajouter `AgentScope::contains`** dans le gate d'admission de chaque workload/delivery
4. **Ajouter `budgets_within`** dans le gate d'admission de chaque surface
5. **Documenter les exceptions** — les surfaces "souveraines" (scheduler, tunnel, enrollment) doivent déclarer explicitement pourquoi elles bypassent le gate `--apply`
6. **Test de régression** — retirer le gate d'une surface et vérifier que la mutation est refusée
