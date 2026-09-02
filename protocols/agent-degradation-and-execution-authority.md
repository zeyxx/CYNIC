# Protocole — Dégradation de coordination et autorité d’exécution agentique

- Statut : design validé par opérateur ; aucune autorité runtime n’est créée par ce document.
- Autorité : `contracts/foundation.json` → `infra/runtime-authority.json` → contrats Rust.
- Observation source : `cynic runtime mcp-observation --json`, 2026-09-01 : MCP `incompatible`, digest divergent, `cynic_runtime_surface` et `cynic_source_change_submit` absents.
- Contrats existants à étendre : `coordination_lease`, `agent_coordination_continuity`, `agent_workload`, `one_shot_capability`, `agent_isolation`.

## 1. But

Un agent reste utile sous perte de coordination sans jamais recevoir une autorité implicite. La dégradation de coordination et l’autorité d’exécution sont deux chantiers distincts :

1. le premier décide quelles capacités restent disponibles quand le scheduler/MCP est absent, incompatible ou instable ;
2. le second décide quelle autorité est requise par une action dans un runtime sain et fencé.

Ni un fallback humain, ni une proposition de capability, ni une exécution répétée ne deviennent une lease runtime ou une promotion automatique.

## 2. Chantier A — Dégradation de coordination

### 2.1 États et transition

```text
observe runtime/MCP
  stable       → RuntimeFenced
  incompatible → DegradedOperatorNotification
  unavailable  → DegradedOperatorNotification
  unstable     → DegradedOperatorNotification
  unknown      → BlockedUnknown

DegradedOperatorNotification
  → HumanScopedFallbackProposal (inert)
  → admission humaine bornée
  → LocalSourceSlice
  → post-slice receipt
  → expired | blocked | runtime re-observed stable
```

`ActiveStable` annule immédiatement tout fallback encore non consommé. `Unknown` ne peut pas ouvrir de fallback.

### 2.2 Capacités retenues

| État | Retenu | Suspendu |
|---|---|---|
| RuntimeFenced | ensemble autorisé par scheduler/lease | selon politique runtime |
| Incompatible / unavailable / unstable | observe, analyse, plan, lecture de preuves | toute mutation runtime et source, sauf fallback explicitement admis |
| Unknown | observation minimale | toute mutation |

Le fallback ne permet que `LocalSourceMutation` ou `LocalSourceValidation`; il interdit runtime, credential, network, fleet, release, commit, push et merge.

### 2.3 Transaction fallback

Une `HumanScopedFallbackProposal` doit lier :
- read-back de disponibilité (`incompatible`, `unavailable` ou `unstable`) ;
- cible source relative et liste des chemins admis ;
- `baseline_ref` et digest Git/worktree pré-slice ;
- budget d’actions et wall-time ;
- validation ciblée attendue ;
- profil d’isolation déclaré et zéro-egress ;
- révision observée du store de claims et absence de conflit ;
- rollback intent.

L’approbation humaine est distincte et expirante. Le `FallbackSliceReceipt` lie admission, Git pré/post, chemins réellement modifiés, validation, résultats, residual unknowns et fin de budget. Un chemin hors scope, une validation manquante, un conflit, l’expiration ou la réapparition d’un runtime stable donnent `blocked`, jamais une réussite partielle.

## 3. Chantier B — Autorité d’exécution

### 3.1 Classes

| Classe | Usage | Autorité |
|---|---|---|
| `ReadOnly` | observation sans mutation | scope, budget et profil d’isolation |
| `OneShot` | mutation rare/sensible | capability ed25519 unique, consommation atomique |
| `RoutineAttested` | mutation répétable stabilisée | capability signée, versionnée, révocable, TTL/max uses/budget et read-back par usage |

Une routine attestée est un contrat d’action paramétré, pas une ligne shell mémorisée.

### 3.2 Template fermé

Un template est un argv structuré : programme allowlisté, arguments fixes, slots de paramètres typés et grammaires fermées. Shell, pipelines, interpolation de chaînes, wildcards, secrets, chemins absolus machine-locaux et paramètres fournis comme règles libres sont refusés.

La capability lie au minimum : template digest, scope, identity, executor digest, isolation profile digest, budget, TTL, max uses, egress policy et obligations de read-back.

### 3.3 Proposition, jamais autopromotion

Le ledger redacted conserve : template/action digest, paramètres canonisés non-secrets, identity, scope, parent task/session/epoch, isolation/executor digests, pré/post proof, outcome, egress, coûts, rollback et residuals.

Un observateur local pur groupe des exécutions indépendantes et émet `CapabilityPromotionProposal`. Il ne signe pas et ne crée pas de capability. La proposition exige :
- plusieurs sessions et tâches parentes indépendantes ;
- zéro drift et zéro rollback dans la fenêtre ;
- executor/isolation identiques ;
- read-back cohérent ;
- scope minimal démontré ;
- preuve dans harness isolé.

Une admission humaine, ou un scheduler explicitement délégué et fencé, peut ensuite signer la capability. Échec, drift, changement de sandbox/executor, élargissement de paramètres, expiration ou incident entraînent suspension/révocation.

## 4. Frontière non négociable

Le fallback humain du chantier A ne crée pas une `RoutineAttested` du chantier B. Il peut uniquement réparer/valider localement le code qui rendra possible une coordination fencée. La promotion d’une routine ne peut avoir lieu que sous une coordination saine, ou rester une proposition inertielle.

## 5. Ordre bottom-up

1. `cynic-protocol` : compléter les types/predicates du fallback, puis types purs d’autorité d’exécution, ledger et proposition.
2. Tests protocolaires : transitions, expiration, scope, injection de paramètres, absence d’autopromotion, révocation.
3. `cynic-host` : ports de read-back, ledger et consommation/révocation atomique ; aucun shell.
4. `cynic-node` : observateur local de récurrence et projection de propositions inertielle.
5. `cynic-cli` : lecture de statut/propositions, admission humaine explicite et proof projection ; aucune politique locale dupliquée.
6. Conformance : perte/reprise MCP, fallback expiré, conflit, mutation hors scope, replay, sandbox modifiée, drift post-action et promotion non signée.
