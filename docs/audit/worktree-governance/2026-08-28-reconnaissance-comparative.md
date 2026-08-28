# Audit comparatif — gouvernance des worktrees CYNIC

Date : 2026-08-28
Statut : audit read-only, aucune décision d’implémentation

## Objectif

Établir ce qui existe réellement dans CYNIC, quels problèmes sont démontrés,
et quels modèles externes peuvent éclairer la gouvernance avant de choisir une
nouvelle frontière Rust.

## Baseline observée

Branche d’audit : `agent/hermes/worktree-inventory-w1`.

Ancrages committés :

- `1edd0423` — contrat protocole W1-A d’observation brute ;
- `d43b0da8` — design séparant observation et autorité ;
- `7ea7ede7` — contrat d’observation toolchain distinct.

La branche contient actuellement un prototype W1-B host non committé. Il n’est
pas une source de vérité et ne doit pas être promu sans nouvelle décision.

Observation Git courante : 20 worktrees enregistrés, dont quatre entrées `/tmp`
marquées `prunable` par Git. Les entrées `/tmp` sont des faits d’observation,
pas encore une autorisation de suppression ou de prune.

## Surfaces CYNIC existantes

### Contrat et preuve

- `crates/cynic-protocol/src/source_repository_bootstrap.rs` possède déjà
  l’identité repository/node, la source, la révision, le principal, le state
  root et les états `Converged`, `BootstrapRequired`, `SyncRequired`, `Blocked`,
  `Unknown`.
- `crates/cynic-protocol/src/evidence.rs` et `digest.rs` possèdent les primitives
  de provenance et la loi de digest canonique.
- `crates/cynic-protocol/src/agent_workload/workspace_types.rs` et
  `workspace_lifecycle.rs` décrivent un workspace déjà admis, avec workload,
  lease, claim, completion evidence et retention.
- Ces surfaces ne sont pas interchangeables : un worktree inconnu ne possède
  pas encore le workload/lease requis par `AgentWorkspace*`.

### Host

- `crates/cynic-host/src/source_control.rs` observe déjà un repository précis
  et son état Git, mais contient aussi des opérations mutationnelles de source.
- `crates/cynic-host/src/source_worktree.rs` matérialise directement un worktree
  et mélange observation, lease, argv et mutation. Il constitue un second chemin
  de provisionnement à comparer avec le provisioner node.
- `crates/cynic-host/src/update_lifecycle/port.rs` fournit un `ArgvRunner`
  injectable et adapté aux parsers read-only.

### Node et claims

- `crates/cynic-node/src/adapters/local_agent_workspace_provision.rs` et
  `local_agent_workspace_collection.rs` possèdent le lifecycle des workspaces
  admis.
- `crates/cynic-node/src/adapters/registry/agent_workspace_layout.rs` possède
  la résolution du layout runtime sous state root.
- `AgentWorkspaceLifecycleStorePort::load` est workload-keyed. Il ne permet pas
  de retrouver un record pour un candidat qui ne porte pas de `workload_id` sans
  inventer cette identité.
- Le store de claim durable est une frontière node/kernel distincte ; MCP ne
  doit pas être utilisé comme substitut.

### CLI/MCP

- Le CLI et le node exposent déjà des surfaces workspace/lifecycle.
- MCP est actuellement absent/incompatible selon `cynic runtime
  mcp-observation --json`. Il doit rester une projection future, non une
  dépendance de l’inventaire ou de la classification.

## Problèmes démontrés

| Problème | Preuve actuelle | Owner probable | État |
|---|---|---|---|
| Worktrees directs hors lifecycle durable | deux chemins `git worktree add` host/node | node lifecycle + host adapter | confirmé, non résolu |
| Entrées Git prunable récurrentes | `git worktree list --porcelain` / 4 `/tmp` | host observation puis policy W3 | observé, non actionnable |
| Worktree primaire et workspaces liés mélangés par Git | format officiel Git liste le main worktree en premier | host parser + layout node | confirmé |
| `--force` dans la collecte | `local_agent_workspace_collection.rs` | W3/collection owner | dangereux, non promu |
| Claims coordination non durables/unifiées | `reports/audit-coordination-claims.md` | kernel/node | audit existant, à revalider |
| MCP absent/incompatible | `cynic runtime mcp-observation --json` | MCP projection/release | bloqué, non bloquant pour W1 |
| Provenance déclarative confondue avec preuve | reports/bootstrap sérialisables et audits PEF | node/kernel proof | confirmé conceptuellement |

## Comparaison de modèles externes

### 1. Git natif : ressource principale + worktrees liés

La documentation officielle Git définit un repository avec un main worktree et
zéro ou plusieurs linked worktrees. `git worktree list --porcelain` expose des
attributs stables ; `locked`, `bare` et `prunable` sont des annotations distinctes.
`prunable` signifie que Git peut retirer une entrée administrative, pas que le
projet autorise une suppression métier.

Source : https://git-scm.com/docs/git-worktree

Ce modèle apporte :

- une identité Git observable ;
- la distinction main/linked ;
- des faits `locked`/`bare`/`prunable` ;
- une primitive native de réparation/prune.

Il ne fournit pas :

- ownership CYNIC ;
- principal runtime ;
- record lifecycle ;
- claim/fence métier ;
- preuve d’autorisation de suppression.

### 2. Controller/reconciler : desired, observed, status, ownership

Kubernetes sépare la ressource désirée, l’état observé et la réconciliation. Les
owner references structurent l’ownership ; les finalizers empêchent une
suppression tant qu’un contrôleur n’a pas terminé une étape de cleanup.

Source : https://kubernetes.io/docs/concepts/architecture/garbage-collection

Ce modèle apporte :

- ownership explicite ;
- contrôleur responsable de la convergence ;
- distinction observation/action ;
- cleanup bloqué par défaut tant que la preuve n’est pas acquise.

Risques de transposition :

- créer un faux contrôleur global dans CYNIC ;
- transformer un finalizer en autorité implicite ;
- oublier que CYNIC possède déjà node/kernel/host comme owners séparés.

### 3. Lifecycle de ressource CYNIC existant

Le lifecycle `AgentWorkspace*` est le modèle le plus proche pour un workspace
admis : record durable, retention, claim, fence, completion evidence et
read-back. Il est cependant workload-keyed et ne peut pas absorber un worktree
inconnu sans falsifier son admission.

## Synthèse des options

| Option | Avantage | Risque | Décision provisoire |
|---|---|---|---|
| Étendre `AgentWorkspace*` pour l’inventaire | réutilisation apparente | injecte un worktree inconnu dans un chemin mutationnel | rejetée |
| Faire classer le host | proximité des faits Git | host ne possède ni lifecycle ni authority | rejetée |
| Contrat protocol facts-only + host parser + node corrélation | owners séparés, unknown conservé | exige une nouvelle corrélation node | retenue pour étude |
| Rebuild MCP avant inventaire | coordination uniforme en façade | MCP absent, projection non autoritaire | rejetée comme prérequis |
| Nettoyage direct des `prunable` | rapide | transforme un fait Git en mutation non prouvée | rejetée |

## Décisions à prendre avant implémentation

1. Le champ path W1 est-il un path relatif à une zone layout déclarée, ou un
   identifiant opaque accompagné d’une preuve de path ? Il ne peut pas être
   implicitement relatif au repository root si les deux roots sont distincts.
2. Quels faits Git minimums sont conservés : `HEAD`, branch/detached, locked,
   bare, prunable, reason, et état d’existence ?
3. Quelle observation node lie un candidat à un record lifecycle sans inventer de
   `workload_id` ?
4. Où est vérifiée l’authenticité des read-backs : node, kernel, ou adapter
   signé ? Un champ sérialisé auto-déclaré ne suffit pas.
5. Quelle est la règle de conservation pour un worktree sans record ? La
   recommandation est `Unknown + retain`, jamais `Prunable`.
6. Le parser doit-il refuser toute extension Git inconnue ou la transporter sous
   forme de fait opaque borné ?

## Stop conditions

Aucune implémentation W1-B/W1-C ne doit démarrer tant que ces points ne sont pas
tranchés. En particulier :

- pas de nouvelle boucle de reviews sur un contrat non figé ;
- pas de commit du prototype host actuel ;
- pas de suppression physique des worktrees ;
- pas de reconstruction MCP comme contournement ;
- pas de scan externe/PEF dans W1.

## Prochaine tranche recommandée

Une décision d’architecture courte, suivie d’un audit adversarial ciblé sur les
surfaces existantes, puis une seule tranche host parser de faits Git. Le parser
ne classifie pas, ne prouve pas l’ownership et ne mutile pas. Le node sera traité
séparément après read-back des limites du parser.

## Références externes consultées

- Git `worktree` officiel : https://git-scm.com/docs/git-worktree
- Git documentation source : https://github.com/git/git/blob/master/Documentation/git-worktree.adoc
- Kubernetes garbage collection/finalizers : https://kubernetes.io/docs/concepts/architecture/garbage-collection

## Avis contradictoire OMP/Gemini

Un examen indépendant read-only par OMP/Gemini a confirmé la séparation des
owners, mais a fait émerger deux risques que l’audit initial ne couvrait pas
suffisamment :

1. **Un inventaire ne doit pas échouer globalement sur une anomalie locale.**
   Un flux peut contenir le worktree primaire, des worktrees runtime valides,
   des entrées `/tmp` prunables et des chemins hors layout. Le parser doit
   conserver l’ensemble des candidats avec un état local `outside`, `unknown`
   ou `malformed`; une seule entrée ne doit pas aveugler l’inventaire complet.
2. **`clean=true` ne prouve pas l’absence de travail utile.** Un commit local en
   `detached HEAD` peut être absent de `git status --porcelain` tout en n’étant
   rattaché à aucune ref durable. Toute future collecte doit donc vérifier la
   reachability du `HEAD` avant action, en plus de la propreté de l’arbre.

Autres points OMP à intégrer dans les tranches futures :

- `locked`, `bare` et les extensions Git doivent être conservés ou explicitement
  représentés comme inconnus, jamais ignorés silencieusement ;
- les verrous POSIX temporaires doivent avoir une stratégie de crash/reprise,
  sinon un arrêt brutal peut créer un blocage permanent ;
- la taxonomie PEF/external doit rester en quarantaine tant que le lifecycle
  local et la reachability Git ne sont pas prouvés ;
- un candidat sans record ne doit jamais recevoir un `workload_id` synthétique.

Cet avis est consultatif et n’est pas une autorité runtime. Il modifie toutefois
les critères d’audit : l’inventaire doit être résilient aux anomalies locales et
la collecte future doit être protégée par une preuve de reachability.
