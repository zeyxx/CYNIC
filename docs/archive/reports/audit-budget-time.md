# Audit E — Budget & Time

**Date :** 2026-08-20  
**Scope :** Budgets, TTL, time injection  
**Authority :** Read-only audit, no code modified  
**Constitution ref :** §4 Budget Monotonicity, §5 Admission Gate

---

## Files audited

| File | Lines | Role |
|---|---|---|
| `crates/cynic-protocol/src/agent_citizensehip.rs` | 355 | AgentBudgets, budgets_within |
| `crates/cynic-protocol/src/agent_workload/lifecycle.rs` | 326 | claim_agent_workload, renew_agent_workload_claim |
| `crates/cynic-host/src/runtime_harness.rs` | 433 | validate_runtime_harness_target_attestation_at(now) |
| `crates/cynic-node/src/usecases/build_scheduler/lifecycle.rs` | 243 | claim_if_available, monitor_claim |
| `crates/cynic-protocol/src/breakglass.rs` | ~150 | BreakglassAdmission TTL |
| `crates/cynic-protocol/src/node_enrollment.rs` | 240 | MachineEnrollment TTL |
| `crates/cynic-protocol/src/node_control.rs` | ~150 | NodeControl TTL |
| `crates/cynic-protocol/src/node_identity.rs` | ~300 | NodeIdentity TTL |
| `crates/cynic-protocol/src/federation_envelopes.rs` | ~150 | Lease TTL |
| `crates/cynic-mcp/src/kernel_session.rs` | ~300 | Session expiry |
| `crates/cynic-host/src/kernel_tunnel.rs` | ~300 | TunnelProof, max_failures |

---

## Questions & Answers

### 1. Is there any code path where a budget can be amplified without parent approval?

**Réponse : Oui.**

**Chemin identifié :**

`cynic-protocol/src/agent_workload/lifecycle.rs:89-151` — `renew_agent_workload_claim()` :
- Permet de renouveler un claim avec un **nouveau** `expires_at_ms` (ligne 106)
- Vérifie `require_live_claim` (claim existant non expiré)
- **Aucune vérification** que le nouvel `expires_at_ms` ne dépasse pas la fenêtre maximale autorisée par le parent
- **Aucune référence** au budget parent ou à la fenêtre de délégation initiale

**Scénario d'amplification :**
1. Agent A reçoit claim avec `expires_at_ms = now + 60s`
2. Agent A appelle `renew_agent_workload_claim` avec `expires_at_ms = now + 3600s`
3. Si le scheduler ne vérifie pas la borne maximale → amplification

**Atténuation actuelle :** Le `claim_ttl_secs` dans `BuildSchedulerConfig` limite la fenêtre de renouvellement, c'est une limite de configuration, pas une limite structurelle. Pas de champ `max_lease_window_ms` dans `AgentWorkloadClaim`.

---

### 2. How is wall-clock injected — chrono::Utc::now() directly, or injectable?

**Réponse : Mixte.**

**Directement (non-injectable) :**
- `runtime_harness.rs:149` — `chrono::Utc::now().timestamp_millis()` dans `validate_runtime_harness_target_attestation()`
- `update_lifecycle/time.rs` — `now_ms()` appelle `chrono::Utc::now()`
- `kernel_tunnel.rs` — appelle `SystemTime::now()` pour health checks
- `node_enrollment_lifecycle_uc.rs:28` — `SystemTime::now().duration_since(UNIX_EPOCH)`

**Injectable (pur avec paramètre) :**
- `runtime_harness.rs:153-243` — `validate_runtime_harness_target_attestation_at(context, attestation, now_unix_ms)` — version pure
- `coordination_scaling_harness.rs` — les scenarios sont des données, l'évaluation est pure
- `build_scheduler/lifecycle.rs:23` — `current_unix_ms()` est une helper locale, mais non-injectable par l'appelant

**Gap :** `validate_runtime_harness_target_attestation()` (sans `_at`) est celle qui est appelée en production. Elle appelle `chrono::Utc::now()` directement. La version `_at` n'est utilisée que dans les tests.

---

### 3. What happens when a lease expires mid-execution — hard stop or graceful completion?

**Réponse : Cancellation token → arrêt propre.**

**Mécanisme :**
- `build_scheduler/lifecycle.rs:64-131` — `monitor_claim()` tourne dans une boucle tokio
- Quand le claim expire : `durable.record.claim.as_ref().map(|claim| &claim.guard) != Some(&guard)` (ligne 89) → `cancellation.cancel()`
- `CancellationToken` propagé à l'executor → arrêt propre de la tâche

**Ce qui n'est PAS géré :**
- Pas de "grace period" après expiration — la cancellation est immédiate
- Pas de persistance de l'état partiel (la tâche est annulée, pas mise en pause)
- Pas de notification au parent "claim expiré pendant l'exécution"

---

### 4. Are budget consumptions atomic with the mutation, or can a mutation succeed without decrementing?

**Réponse : Non-atomique.**

**Dans `build_scheduler/mod.rs` :**
1. Le scheduler claim un workload (CAS sur store) → `AgentWorkloadState::Claimed`
2. Exécution du build (process spawn)
3. En cas de succès : `complete_agent_workload()` → `consumed_actions += 1` + `state = Done`
4. En cas d'échec : `finish_blocked()` → `state = Blocked` (pas de décrément)

**Scénario de désynchronisation :**
- Étape 1 réussit (claim persisté)
- Étape 2 échoue silencieusement (process spawn retourne exit code 0 mais ne compile rien)
- Étape 3 incrémente `consumed_actions` malgré l'échec fonctionnel

**Pas de transaction atomique** entre "exécution réussie" et "budget décrémenté". Le budget est décrémenté sur la base du succès technique (exit code), pas du succès fonctionnel (build réel).

---

## Summary of gaps

| # | Gap | Severity | Constitution ref |
|---|---|---|---|
| 1 | `renew_agent_workload_claim` sans borne maximale parentale | High | §4 Non-Amplification |
| 2 | `validate_runtime_harness_target_attestation()` appelle `chrono::Utc::now()` directement | Medium | §7 Time Injection |
| 3 | Budget décrémenté sur exit code, pas sur succès fonctionnel | Medium | §4 Atomicity |
| 4 | Pas de persistance de l'état partiel à l'expiration du claim | Low | §5 Bounded Action |

---

## Recommendations for U5 (Time Injection)

1. Remplacer tous les appels à `chrono::Utc::now()` et `SystemTime::now()` par une horloge injectable (`fn now_unix_ms() -> u64`)
2. Ajouter un paramètre `now_unix_ms: u64` à toutes les fonctions de validation temporelle en production
3. Exiger une `Clock` trait dans les modules de domaine
4. Les tests injectent une clock fixe ; la production injecte `SystemClock`
