# CYNIC Accountability — Audit complet

Statut : étude. Aucune construction, aucune modification de contrat.
Méthode : lecture du code vivant, pas des docs. Chaque verdict cite sa
preuve (fichier vivant) ou son absence constatée.

## 1. Chaîne modélisée

Constituant (humain) → identité (`CitizenAccount`, `NodeAccount`,
`AgentAccount`) → capacité et bornes (`OfficeAccount`, `MandateAccount`)
→ ressource observée (`ResourceAccount`, `TreasuryAccount`) → admission
(`AgentWorkloadEnvelope`) → action bornée (`AgentBoundedAction`) →
observation → preuve (`AgentCompletionEnvelope`, `RuntimeProof`) →
transition d'état → vue matérialisée.

Preuves types : `crates/cynic-protocol/src/peer/accounts.rs` (comptes 112-164,
`MandateAccount::is_valid_at`), `crates/cynic-protocol/src/agent_workload/types.rs`
(enveloppe 46, réclamation 92, enregistrement 104, décision 125),
`crates/cynic-protocol/src/agent_recursion.rs` (délégation 54, budget 23,
achèvement 83).

## 2. Ce qui vit

### 2.1 Vertical build — l'unique chaîne complète

`crates/cynic-node/src/usecases/build_scheduler/lifecycle.rs`,
`build_submission_orchestrator_uc.rs:127-141`,
`agent_workload_store_uc.rs:190-199` : réclamation, consommation d'action,
achèvement via les transitions pures du protocole, avec reçus
(`agent_workload_delivery_uc.rs:180-214`, réconciliation d'achèvement).

Magasin durable : `LocalAgentWorkloadStore`
(`crates/cynic-node/src/adapters/local_agent_workload_store.rs`) —
révisions `revision-NNN.json` sous racine d'état, derrière
`AgentWorkloadStorePort`. Port jetable, preuve durable.

### 2.2 Baux baseline — coordination coopérative réelle

`CoordinationLeaseStore`
(`crates/cynic-host/src/coordination_lease_store/mod.rs:37-248`) :
mémoire + JSON, claim, claim_batch, mutate, who. Exécuté, testé, utilisé.
Ce n'est pas une autorité d'exécution et ne prétend pas l'être.

### 2.3 Prédicats — purs, testés, branchés

Admission dispatch, budgets frères agrégés, fencing, générations
d'annulation, fenêtres max de bail : purs dans `cynic-protocol`, appelés
par les use-cases du vertical build. La garde anti-amplification
(`max_lease_window_ms`, `AgentWorkloadClaim:granted_at_ms`) est du code
qui tourne, pas de la prose.

## 3. Ce qui dort

### 3.1 Moteur d'état pair — décor

`apply_transaction` (`crates/cynic-protocol/src/peer/engine.rs:149`) :
zéro appelant vivant hors tests. Le nœud vérifie des chaînes WAL
(`adapters/peer_storage/wal_file.rs`), le kernel observe des pairs
(`api/rest/agent_session.rs:69`), personne ne transacte. La machine
constitutionnelle est décrite, branchée nulle part.

### 3.2 Scheduler d'agents général — absent

Aucun ordonnanceur node-owned n'applique durablement enveloppes de
délégation, budgets monotones, fencing, annulation, reprise et preuves
d'achèvement pour des charges générales. Les outils de coordination
déclarés (`cynic_coord_register/who/claim/release`, délégation,
achèvement) n'ont pas de contrepartie exécutante hors builds.

### 3.3 Binding mandat optionnel — écart loi/code

La loi exige fail-closed sans mandat admis. Le code porte
`mandate_binding: Option` avec commentaire legacy
(`agent_workload/types.rs:51-54`). L'écart est explicite et documenté
dans le type lui-même, mais non résorbé : qui migre les enveloppes
legacy, et quand l'option disparaît-elle ?

## 4. Inconnues et dettes nommées

- `cpu_ms: None` dans les budgets réservés : inconnu explicite par
  construction, à mesurer à l'achèvement. Correct, à conserver.
- `RecoveredUnattributed` dans les états terminaux : une reprise sans
  attribution est un aveu, pas une preuve. Quel chemin la produit, et
  qui la re-attribue ?
- `AgentWorkloadViolation.code` est une chaîne libre, pas un
  `CynicErrorCode` typé — contre la loi des codes d'erreur souverains.
- Les 9 outils MCP en arbre contournent toute cette chaîne (lecture
  seule, honnête aujourd'hui). Chaque futur outil mutant devra la
  traverser ; sans moteur, pas de mutation légitime.

## 5. Questions préalables à tout design

1. Le moteur général naît-il du vertical build (extension) ou à côté
   (neuf, avec preuve que le build ne l'absorbe pas) ?
2. Le binding mandat devient-il obligatoire, avec quel plan de migration
   des enveloppes legacy ?
3. Les transactions pair entrent-elles en service, ou sortent-elles du
   contrat (suppression plutôt que décor) ?
4. Les outils de coordination restent-ils déclarés sans exécuteur, ou
   l'exposition est-elle réduite à ce qui s'exécute ?

## 6. Suite proposée

Doc de décision accountability répondant aux quatre questions, puis
implémentation. À défaut : doctrine écrite « lecture seule sans mandat,
mutation interdite sans moteur », appliquée aux outils existants, avec
les fantômes coordination assumés comme tels dans l'exposition.
