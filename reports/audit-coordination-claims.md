# Audit A — Coordination & Claims

**Date :** 2026-08-20  
**Scope :** Création, détention, expiration, et libération des claims dans CYNIC  
**Authority :** Read-only audit, no code modified  
**Constitution ref :** §4 Budget Monotonicity, §5 Admission Gate, §6 Scope Hierarchy, §11 Non-Amplification

---

## Files audited

| File | Lines | Role |
|---|---|---|
| `crates/cynic-protocol/src/coordination_lease.rs` | 119 | Types purs : LeaseGrant, LeaseGuard, LeaseAction, TargetClaimOutcome |
| `crates/cynic-protocol/src/coordination_runtime.rs` | 77 | Observation de task (heartbeat, lease_expires_at) |
| `crates/cynic-protocol/src/coord.rs` | 70 | Chemins /coord/*, LeaseVerification |
| `crates/cynic-protocol/src/agent_citizenship.rs` | 355 | AgentScope, AgentMode, AgentBudgets, admit_mutation |
| `crates/cynic-protocol/src/agent_workload/lifecycle.rs` | 326 | claim_agent_workload, renew_agent_workload_claim, admit_workload_action |
| `crates/cynic-protocol/src/agent_workload/types.rs` | — | AgentWorkloadClaim, AgentWorkloadState, AgentWorkloadAdmission |
| `crates/cynic-node/src/usecases/build_scheduler/lifecycle.rs` | 243 | claim_if_available, monitor_claim, finish |
| `crates/cynic-node/src/adapters/local_agent_workload_store.rs` | 365 | Stockage local durable (JSON files + symlinks + locks) |
| `crates/cynic-node/src/adapters/local_agent_workload_delivery_lease.rs` | 79 | Livraison de leases aux agents |
| `crates/cynic-host/src/local_autonomy_runtime.rs` | 144 | tick OS + kernel reachability |
| `crates/cynic-protocol/src/health_signal.rs` | ~1320 | HealthSignal pour AgentClaimExpired, RenewAgentClaim |

---

## Questions & Answers

### 1. Is there any durable claim store, or are claims in-memory only?

**Réponse partielle.** Il existe deux couches :

**Couche A — Durable par workload (Live)**  
`LocalAgentWorkloadStore` (`cynic-node/src/adapters/local_agent_workload_store.rs`) :
- Persiste les `DurableAgentWorkloadRecord` sur disque (fichiers JSON dans `root/workload_id/revision_N.json`)
- Utilise `compare_and_swap` avec révision atomique (lignes 101-195)
- Verrouillage fichier exclusif via `open_lock` + `lock_exclusive`
- Read-back post-persist (lignes 163-167)
- Preuve persistée via `agent_execution_runtime_observer::persist_capabilities` (lignes 182-192)

**Couche B — Observation task (Partiel)**  
`CoordinationTaskObservation` (`coordination_runtime.rs`) :
- Possède `lease_expires_at` et `heartbeat_at` (lignes 28-29)
- Mais le magasin est en mémoire — pas de persistance des claims de coordination
- Les claims de coordination (`LeaseGuard`, `coord_claim_path`) ne sont **pas persistées** entre redémarrages

**Gap majeur :** Le `coordination_lease` (claims agent-agent) est un type pur sans store. Le `LocalAgentWorkloadStore` ne persiste que les workloads, pas les claims de coordination inter-nœuds. Si le nœud redémarre, toutes les claims de coordination sont perdues.

---

### 2. How is TTL enforced — heartbeat, wall-clock check, or lease fencing token ?

**Trois mécanismes coexistants :**

**Mécanisme 1 — TTL sur claim de workload (Live)**  
`claim_agent_workload` (`lifecycle.rs:14-69`) :
- `expires_at_ms` requis, doit être > `now_ms` (lignes 47-51)
- `guard.fencing_token` doit être > `record.last_fencing_token` (lignes 53-57)
- Renouvellement via `renew_agent_workload_claim` (lignes 89-151) avec vérification `require_live_claim` + fenêtre de renouvellement anticipée

**Mécanisme 2 — Monitor claim dans build_scheduler (Live)**  
`monitor_claim` (`build_scheduler/lifecycle.rs:64-131`) :
- Boucle tokio avec `tokio::time::sleep(1s)`
- Renouvellement déclenché quand `expires_at_ms <= now_ms + ttl_ms/2` (lignes 96-100)
- Utilise `renew_agent_workload_claim` avec CAS atomique sur le store
- Cancellation token pour arrêt propre

**Mécanisme 3 — Heartbeat sur CoordinationTask (Partiel)**  
`CoordinationTaskObservation` (`coordination_runtime.rs:29`) :
- Champ `heartbeat_at` existe mais est `Option<String>` — pas de validation de fraîcheur
- Aucun code ne vérifie "heartbeat_at trop vieux → claim expirée"
- Aucun code ne fait de renouvellement automatique du heartbeat

**Gap :** Le mécanisme 3 est décorrélé du mécanisme 1. Un agent pourrait avoir un `heartbeat_at` frais dans `CoordinationTaskObservation` mais un claim expiré dans `AgentWorkloadRecord`. Pas de source unique de vérité.

---

### 3. What happens when the store is unreachable — fail-closed or fail-open ?

**Réponse : Fail-closed au niveau workload, fail-open au niveau coordination.**

**Workload store (Fail-closed)**  
`LocalAgentWorkloadStore::compare_and_swap` (`local_agent_workload_store.rs:101-195`) :
- `fs::create_dir_all` échoue → `error("workload_store_root_create_failed")` → mutation refusée
- `open_lock` échoue → `error("workload_store_lock_failed")` → mutation refusée
- `read_back` mismatch → `error("workload_store_read_back_mismatch")` → mutation refusée
- Aucun chemin ne permet de muter sans store joignable

**Coordination store (Fail-open / inexistant)**  
`admit_mutation` (`agent_citizenship.rs:139-173`) :
- `store_available: false` → `AgentCoordinationStoreUnavailable` ajouté aux codes
- Mais la fonction retourne `Refused`, pas de fallback
- **Problème :** Si le store de coordination est en mémoire et que le nœud redémarre, les claims sont perdues → pas de persistence → pas de détection de "store unreachable"

**Gap :** Pas de distinction entre "store injoignable" et "store perdu après redémarrage". Le code actuel suppose que le store est toujours disponible en mémoire locale, ce qui n'est pas vrai en production.

---

### 4. Does Cynic-preempts-Baseline work at runtime, or only in tests?

**Réponse : Type-level only, pas de runtime enforcement.**

**Type definition (Live)**  
`AgentMode::preempts` (`agent_citizenship.rs:65-74`) :
```rust
pub fn preempts(self, other: Self) -> bool {
    matches!((self, other), (Self::Cynic, Self::Baseline))
}
```

**Runtime usage :** Aucune occurrence dans le code production. `AgentMode` n'est ni sérialisé dans les workloads persistés, ni vérifié dans `admit_mutation`. Le paramètre `mode` n'est même pas dans `AgentCitizenshipRecord` — il est seulement dans la doc et le type enum.

**Tests (Live)**  
`agent_citizenship.rs:291-298` teste `preempts` en isolation.

**Gap :** Cynic-preempts-Baseline est une règle documentée mais non câblée. Aucun magasin de claims n'utilise ce champ pour résoudre les conflits. Deux claims en conflit en mode Baseline se résolvent par "first-wins" au niveau du store (via CAS revision), mais pas par préemption explicite.

---

### 5. Can a claim be amplified without parent delegation?

**Réponse : Oui, partiellement — par renouvellement de claim.**

**Chemin d'amplification potentiel :**
1. Un agent détient un claim avec `expires_at_ms` initial
2. `renew_agent_workload_claim` (`lifecycle.rs:89-151`) permet de renouveler avec un **nouveau** `expires_at_ms` (ligne 106)
3. Il n'y a **pas de vérification** que le nouvel `expires_at_ms` ne dépasse pas la fenêtre autorisée par le parent
4. Un agent pourrait théoriellement renouveler indéfiniment en passant `now_ms + chaque_fois_plus_long`

**Vérification manquante dans `renew_agent_workload_claim` :**
- `require_live_claim` vérifie que le claim existe et est non expiré
- Aucune vérification de `expires_at_ms` maximal autorisé
- Aucune référence au budget parent ou à la fenêtre de délégation initiale

**Atténuation actuelle :** Le `claim_ttl_secs` dans `BuildSchedulerConfig` limite la fenêtre de renouvellement, c'est une limite de configuration, pas une limite structurelle.

**Gap :** Pas de type `MaxLeaseWindow` ou `ParentDelegationExpiry` qui bornerait la durée maximale d'un claim indépendamment des renouvellements.

---

## Summary of gaps

| # | Gap | Severity | Constitution ref |
|---|---|---|---|
| 1 | Claims de coordination non persistus entre redémarrages | Critical | §5 Admission Gate |
| 2 | Pas de source unique de vérité pour heartbeat vs claim TTL | High | §4 Budget Monotonicity |
| 3 | Cynic-preempts-Baseline non câblé en runtime | Medium | §6 Mode Precedence |
| 4 | Renouvellement de claim sans borne maximale parentale | High | §4 Non-Amplification |
| 5 | Pas de fail-closed explicite sur store de coordination perdu | Medium | §5 Admission Gate |
| 6 | Champ `mode` absent de `AgentCitizenshipRecord` | Medium | §6 Mode Precedence |
| 7 | Pas de distinction "store injoignable" vs "store perdu après restart" | Low | §5 Admission Gate |

---

## Recommendations for U1 (Canonical Coordination Store)

1. Persister les claims de coordination (LeaseGuard, AgentMode, AgentScope, TTL) dans le même store durable que les workloads
2. Ajouter un `max_lease_window_ms` dans `AgentWorkloadClaim` pour limiter les renouvellements
3. Câbler `AgentMode` dans `admit_mutation` avec vérification de préemption
4. Ajouter une source unique de vérité `ClaimHeartbeat` qui agrège workload heartbeat + coordination heartbeat
5. Distinguer `StoreUnreachable` (réessayer) de `StoreLostAfterRestart` (refuser avec `AgentCoordinationStoreUnavailable`)
