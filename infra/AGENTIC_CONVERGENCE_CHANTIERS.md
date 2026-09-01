# Trois chantiers de convergence agentique

- Statut : plan de travail validé par l’opérateur ; non autorité runtime.
- Portée : les trois chantiers sont distincts. Aucun ne peut se substituer aux preuves des deux autres.
- Boucle commune : desired state → observation → reconciliation pure → action explicitement admise → read-back proof.

## Vue d’ensemble

```text
A. coordination dégradée ──────┐
                                ├──► B. autorité d’exécution
C. build fédéré / déploiement ─┘          │
                                           └──► aucune mutation sans isolation + preuve
```

- A traite la perte de l’autorité de coordination : MCP/scheduler incompatible, absent ou instable.
- B traite la nature de l’autorité d’une action : lecture, one-shot, routine attestée.
- C traite l’autorité de compilation, livraison et déploiement : Forgejo → runner isolé → preuve durable.

La dégradation A ne peut jamais créer une capability B. Une capability B ne peut jamais remplacer un runner C. Le runner C ne peut jamais accepter un commit qui n’est pas lu depuis Forgejo et lié à une admission valide.

## A — Dégradation de coordination MCP / scheduler

### État observé

- Le MCP actuel est incompatible : son artefact ne porte plus le crate/runtime attendu.
- Le contrat de fallback existe déjà dans `cynic_protocol::coordination_lease`.
- Le premier slice ajoute une proposition inertielle et un receipt de slice locale ; il reste non commité au moment de cette note.

### Desired state

Quand la coordination est `unavailable`, `incompatible` ou `unstable`, l’opérateur peut admettre une unique slice source locale, bornée et temporaire. Elle reste `best_effort_unfenced`, jamais une lease.

### Prochaines preuves

1. Compléter types/predicates de `proposal → admission → receipt → expiry` dans `cynic-protocol`.
2. Ajouter le port host qui observe Git pré/post, paths effectifs et validation ciblée.
3. Projeter en CLI read-only ; aucune auto-admission.
4. Au retour d’une coordination stable, invalider immédiatement le fallback.

### Interdictions

Aucun fallback humain ne peut muter runtime, réseau, credentials, fleet, release, commit, push ou merge ; ni promouvoir une action vers B.

## B — Autorité d’exécution et promotion de capability

### Desired state

Toute exécution est classée sans ambiguïté :

| Classe | Autorité | Exemple de preuve |
|---|---|---|
| `ReadOnly` | scope + budget + isolation | rapport d’observation |
| `OneShot` | capability signée, consommée atomiquement | binding + receipt post-action |
| `RoutineAttested` | template signé/versionné, usages bornés, révocable | ledger + read-back par exécution |

Une routine ne provient jamais d’une commande shell mémorisée. Elle décrit un template argv fermé, des paramètres typés, le digest d’exécuteur, le profil d’isolation, budget/TTL/max-uses et le read-back obligatoire.

### Prochaines preuves

1. Créer les types purs protocolaires : execution authority, argv template fermé, parameter grammar, routine capability, execution ledger entry, promotion proposal et revocation decision.
2. Tester les prédicats : injection de paramètres, replay, dépassement de budget, sandbox/exécuteur modifié, drift post-action, révocation et partition.
3. Créer les ports host de ledger et consommation atomique.
4. Créer l’observateur local de récurrence ; il ne produit que des propositions.
5. Ajouter l’admission runtime seulement après signature humaine ou scheduler réellement fencé.

### Invariants

- proposition ≠ capability ;
- aucune auto-promotion ;
- pas de shell, d’`eval`, de `sh -c`, de chemins absolus ou de secrets dans le template/ledger ;
- un drift suspend la routine vers one-shot ou blocked.

## C — Build fédéré et déploiement automatique admis

### Précondition C.0 : l'isolation précède Git

Le premier `git` d'un agent — clone, fetch, `worktree add`, création de
branche ou checkout — doit s'exécuter **dans un périmètre déjà isolé**. Un
agent ne reçoit donc jamais le dépôt Cortex partagé, même pour matérialiser son
propre worktree : `git worktree add` modifie l'administration Git du dépôt
source et reste une mutation partagée.

```text
agent admis + profil d'isolation
  → sandbox créée et attestée (aucun dépôt partagé monté RW)
  → mirror/clone Git privé dans le stockage de la sandbox
  → fetch Forgejo read-only de la base SHA déclarée
  → worktree/branche dédiés, dans ce mirror privé seulement
  → processus agent avec ce worktree comme unique cwd RW
```

Le checkout humain Cortex est lecture/intégration seulement. Le dépôt Builder
par défaut et tout worktree d'un autre agent sont absents de la sandbox. Un
conteneur qui monte le checkout partagé reste `blocked_shared_git_admin`, pas
isolé.

La preuve minimale lie `agent_id`, `workload_id`, profil sandbox, PID/cgroup ou
conteneur, git-dir privé, branche, base SHA Forgejo, worktree/cwd effectif,
montages RW limités au workspace, et profil réseau observé. L'absence d'un de
ces liens bloque le lancement avant toute commande Git.

### État observé

| Surface | Observation | Verdict |
|---|---|---|
| CLI Cortex installée | `/home/user/.local/bin/cynic`, 5 août, sans `build` | obsolète |
| Source CLI | `crates/cynic-cli/src/build_surface.rs` expose run/observe-workload/list-workloads/webhook | contrat partiel présent |
| Forgejo | branche locale V5 et branche distante `v1-debt-purge` divergentes | input non apparié |
| Builder runtime | `/var/lib/cynic/runtime/releases/release-h6/cynic`, 26 août, expose `build` | surface présente |
| Builder runner | aucune unité `cynic*` active, aucun bootstrap/pipeline trouvé sur PATH, aucune state workload | unavailable/unobserved |
| Checkout builder | aucun checkout propre pour notre SHA ; checkout queue concurrent dirty | pas utilisable |

Le binaire Cortex ne doit plus être employé comme autorité de compilation. Il peut au mieux rester une projection jusqu’à mise à jour explicitement admise et prouvée.

### Dette de contrat déjà visible dans le code source

- `cynic-cli/src/build_surface.rs` lit une observation JSON locale par défaut et exige un endpoint kernel ; il ne découvre pas/observe pas le runner fédéré lui-même.
- `cynic-host/src/build_workflow.rs` fabrique actuellement un `LeaseGuard` local (`agent_id=local`, token `1`) : il ne lit pas une lease fencée durable.
- le webhook est une surface d’écoute, pas encore une chaîne Forgejo signée → admission → queue durable → runner → proof → déploiement.

### Desired state

```text
Forgejo event signé / PR admise
  → source SHA lu et apparié au SSOT
  → BuildWorkload durable, idempotency key et budget
  → scheduler node-owned + lease fencing + isolation profile
  → checkout éphémère à SHA exact sur runner sélectionné
  → compilation/test par argv typé
  → BuildProofBundle signé + artifact digest + SBOM/provenance
  → gate de déploiement humain/policy
  → canary/apply explicite
  → observation post-déploiement + rollback intent
```

Le mot « automatique » signifie déclenchement et progression sous admission ; jamais push, release ou activation implicite.

### Ordre de convergence C

1. **C.0 sandbox-first** : provisionner/attester le périmètre isolé, puis y cloner ou initialiser un mirror Git privé ; aucune commande Git agent n'est autorisée avant cette preuve.
2. **Observe** : ajouter une observation versionnée du runner réel : identité, artifact hash/version, toolchain, queue/workload state, supervisor, isolation, checkout state, capacité et residual unknowns.
3. **Reconcile** : décision pure `ready | blocked | degraded`, y compris `shared_git_admin` et `workspace_outside_sandbox`, sans fallback shell.
4. **Repair/provision** : plan séparé pour installer/activer runner et artefacts, avec OneShot/bootstrap binding, sans toucher au checkout source par défaut.
5. **Dispatch** : BuildRunnerPort produit un workload durable avec commit Forgejo exact, admission, fencing, budget et isolation digest.
6. **Webhook** : vérification Forgejo signée, idempotence par delivery ID/commit/phase ; webhook ne compile ni ne déploie directement.
7. **Proof** : runner écrit un bundle durable ; CLI/MCP ne font que le lire.
8. **Deploy** : artefact attesté → plan canary → apply explicite → read-back/rollback. Aucun `build` réussi ne vaut déploiement.

### Gates de sécurité

- SHA local = SHA Forgejo = SHA checkout builder avant toute validation ;
- checkout runner propre et éphémère par SHA, jamais le dépôt source builder détaché/dégradé ;
- branche d’un autre agent jamais touchée ;
- un runner absent/inactif produit `blocked`, jamais un cargo local de substitution ;
- les credentials de webhook et Forgejo restent hors Git et hors preuves ;
- aucune activation de service, déploiement ou push sans admission opérateur explicite.

## Dépendances et ordre opérateur

1. Faire C.0 : aucun nouvel agent ne reçoit le Git partagé ; sandbox, mirror et branche privée sont attestés avant toute opération Git.
2. Héberger C.0 sur une VM Proxmox dédiée, conformance d'abord : Cortex devient console/inférence éteignable ; LXC/Bwrap restent des étagements de workload prouvés.
3. Faire C/observe et C/reconcile : le build fédéré devient visible, et l’alignement Forgejo → sandbox mirror → runner devient maintenable.
4. Faire C/provision + dispatch + proof : la validation de code quitte Cortex et produit une preuve durable sans dépendre de son compilateur.
5. Terminer A jusqu’au receipt et son read-back : le fallback MCP reste sûr pendant la réparation du runtime de coordination.
6. Faire B/types purs : les actions récurrentes restent non promotables tant que C n’est pas fencé.
7. Faire B/admission de routine, avec le runner C comme premier consommateur.
8. Faire C/webhook + canary deploy, après preuve durable du build et seulement avec une admission de déploiement distincte.

## Preuve de progression

Chaque chantier produit son rapport JSON versionné et une commande de lecture seule. Les trois rapports doivent déclarer leurs inconnus ; aucune couleur verte globale n’est admise tant qu’un runner, une lease, une signature, un checkout SHA ou un post-deployment read-back reste inconnu.
