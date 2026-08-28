# Cartographie de la verticale Git/worktree CYNIC

Date : 2026-08-28
Statut : audit read-only, aucune mutation autorisée

## Portée

Cette cartographie suit une ressource source/worktree depuis son identité et son
bootstrap jusqu’à sa rétention ou sa collecte. Elle distingue les faits
observés des décisions, des actions et des preuves.

## Chemin nominal observé

```text
SourceRepositoryBootstrapRequest
  → SourceRepositoryBootstrapPort::observe/apply
  → AgentWorkspaceLayoutPort::resolve
  → AgentWorkspaceProvisionPort::ensure
  → AgentWorkspaceObservationPort / TerminalObservationPort
  → AgentWorkspaceLifecycleStorePort
  → AgentWorkspaceClaimStorePort
  → reconcile_agent_workspace_lifecycle
  → AgentWorkspaceCollectionPort::collect
```

Ce chemin nominal ne couvre pas naturellement un worktree découvert sans
`workload_id`, sans record ou hors layout. Ce cas constitue une autre entrée de
la verticale et ne doit pas être injecté artificiellement dans le lifecycle
mutationnel.

## Matrice des frontières

| Étape | Surface actuelle | Owner observé | Faits | Décision/action | Preuve/read-back | Gap |
|---|---|---|---|---|---|---|
| Identité source | `cynic-protocol::source_repository_bootstrap` | protocol/node | repository, node, source, revision, principal | bootstrap required/sync/converged/blocked/unknown | observation source/revision | report sérialisé auto-déclarable ; convergence doit être recomputée par node |
| Bootstrap | `SourceRepositoryBootstrapPort` + adapter node | node/host | state root, Git repository, source, revision | observe ou apply | observation avant/après | report `Completed` possible sans preuve kernel durable |
| Layout | `RegistryAgentWorkspaceLayoutAdapter` | node/registry | roots relatives, repository/workspace store, principal, dedicated | résolution d’un layout | layout digest | `PathBuf` absolus internes ; preuve de containment à chaque action nécessaire |
| Provision | `LocalAgentWorkspaceProvisioner` | node | repository, parent, base commit | `git worktree add --detach` | succès commande, observation ultérieure | création et record lifecycle ne forment pas une transaction unique |
| Provision concurrent | `cynic-host::source_worktree` | host | source/target, lease, owner, FS | `git worktree add --detach` | rapport host | second provisioner et seconde policy d’autorité |
| Observation Git | `cynic-host::source_control` / future inventory | host | HEAD, status, branch, worktree metadata | observation/admission | snapshot/command proof | mélange avec mutation source dans `source_control` |
| Lifecycle admis | `AgentWorkspaceLifecycleObserver` | node | record workload, HEAD, clean, claims | keep/renew/recover/collect | lifecycle observation | indexé par `workload_id`, ne peut pas charger l’inconnu |
| Record | `AgentWorkspaceLifecycleStorePort` | node | completion, retention, protected roots | persist/load | CAS/read-back | aucun record générique pour anomalie worktree |
| Claim/fence | `AgentWorkspaceClaimStorePort` | kernel/node | lease, fencing, repository claims | reserve/renew/release/finalize | durable claim read-back | MCP absent ; fallback local non équivalent à un fence runtime |
| Rétention | lifecycle decision | node | terminal state, retention window, clean | Keep/Recover/Collect plan | plan | reachability detached non représentée comme précondition |
| Collection | `LocalAgentWorkspaceCollectionAdapter` | node | plan, guard, clean, disk | `git worktree remove --force` | path absent + capacity | force, race, reachability et recovery à renforcer |
| Cache GC | `cynic-host::space_gc`, build cache | host/build runner | bytes, files, cache paths | observation ou reclaim profile | path/read-back cache | ne doit pas absorber les worktrees |
| Projection | CLI/MCP | facade | rapports | aucune redécision | JSON/humain | MCP incompatible/absent |

## Problèmes de verticale

### 1. Deux propriétaires de `git worktree add`

`cynic-node::LocalAgentWorkspaceProvisioner` et
`cynic-host::source_worktree::materialize_source_worktree` peuvent tous deux
créer un worktree detached. Ils ne partagent pas une transaction lifecycle
unique ni nécessairement la même admission.

Décision requise : un seul owner de la mutation de provisionnement. L’autre
surface doit devenir une observation/adaptation, ou être retirée. Aucun nouveau
provisioner ne doit être ajouté.

### 2. Le lifecycle admis ne peut pas représenter l’inconnu

Les ports lifecycle demandent `workload_id`, record, claim et guard. C’est
correct pour un workspace admis, mais incorrect pour une entrée découverte sans
identité durable. La corrélation doit rester node-owned et ne doit jamais
synthétiser de workload.

### 3. L’authenticité du bootstrap est insuffisante

`SourceRepositoryBootstrapReport` est sérialisable et contient statut, décision,
lease, proof ref et authority. Ces champs ne sont pas une preuve par eux-mêmes.
Le node doit lire les observations autoritaires et recalculer la convergence.

### 4. `clean` est trop faible pour la collecte

L’observer lifecycle collecte `workspace_clean`, mais un detached HEAD clean peut
porter un commit non reachable depuis une ref durable. Il manque une observation
Git de reachability avant toute collecte.

### 5. Collection dangereuse

Le collecteur exécute actuellement `git worktree remove --force`. Une future
policy ne peut pas utiliser ce chemin sans :

- revalidation immédiate du target et du path canonique ;
- vérification du lock et des claims ;
- vérification de reachability/retention ;
- stop sur toute divergence ;
- read-back indépendant de l’absence et des refs ;
- stratégie de crash/reprise.

### 6. Les GC existants ne sont pas interchangeables

- `space_gc` mesure l’occupation ;
- build cache peut reclaim un profil explicitement reconstructible ;
- homeostasis détecte du waste hyperviseur/mesh ;
- kernel maintenance consolide ses données ;
- model retention produit un assessment sans autorisation ;
- workspace collection est le seul mutateur worktree.

Ils doivent rester fédérés par domaine, sans `GarbageCollector` transversal.

## Modèles concurrents pour la verticale

### Modèle A — Git-native

Git refs et reachability portent l’identité de conservation ; CYNIC garde
seulement le contexte opérateur.

- simple pour commits detached et récupération ;
- faible duplication ;
- ne suffit pas pour claims, budgets, leases et principal runtime ;
- exige une convention de refs durables.

### Modèle B — Lifecycle CYNIC-first

Le record node/kernel est l’owner du worktree ; Git est un substrate.

- bon contrôle des claims et lifecycle ;
- compatible avec les politiques runtime existantes ;
- ne représente pas naturellement les worktrees inconnus ;
- dépend d’une corrélation robuste et d’un store durable.

### Modèle C — Hybride par dimension (retenu pour étude)

- Git : contenu, refs, reachability, main/linked, locked/bare/prunable ;
- node : workspace_id, layout, lifecycle et corrélation ;
- kernel : claims, fencing, transitions durables ;
- host : observation substrate et adapters ;
- W3 : policy de collecte ;
- CLI/MCP : projections.

Ce modèle est retenu uniquement si une question n’a jamais deux décideurs. Il
ne crée pas deux authorities pour une même décision.

## Read-back des 20 worktrees

L’observation A0 montre :

- 20 entrées enregistrées ;
- 5 dirty ;
- 4 status inconnus ;
- 4 prunable ;
- 4 HEAD non reachables depuis les refs observées ;
- 0 locked ;
- 0 bare.

Les quatre entrées `/tmp` sont donc à la fois :

```text
fait Git prunable
+ path absent
+ status inconnu
+ HEAD non reachable depuis refs durables observées
```

Elles ne peuvent pas être envoyées au lifecycle collector actuel sans inventer
un owner ou un workload.

## Critères de fin d’audit

L’audit de la verticale sera considéré terminé lorsque :

1. chaque surface est associée à un seul owner d’action ;
2. chaque fait possède un observer identifié ;
3. l’inconnu est représentable sans faux record ;
4. Git et CYNIC ont des responsabilités complémentaires explicites ;
5. la reachability detached est un fait séparé de `clean` ;
6. les GC spécialisés ne se chevauchent pas ;
7. les scénarios crash/race ont un comportement connu ;
8. une seule tranche d’implémentation peut être choisie sans modifier plusieurs
   owners à la fois.

## Recommandation suivante

Ne pas reprendre le prototype W1-B actuel. Choisir une tranche de preuve unique :

```text
observer Git brut + anomalies par entrée + aucun classement/action
```

Puis faire une seule intégration host, avec le main/linked, locked/bare/prunable,
une limite d’entrée et une sortie complète. La reachability, la corrélation node
et la collecte restent des tranches séparées.
