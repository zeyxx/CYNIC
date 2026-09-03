# CYNIC comme gouvernement computationnel

Date : 2026-09-03
Statut : proposition normative en révision
Autorité : subordonnée à `contracts/foundation.json`, `contracts/framework.json` et `AGENTS.md`

Ce document formalise CYNIC comme un gouvernement computationnel souverain pour un opérateur humain. Il ne décrit pas un gouvernement des humains et ne donne aucune autorité au modèle, au marché, au score ou à l’agent au-delà des contrats qui les bornent.

Le document devient runtime law uniquement après transposition dans des types Rust, des contrats versionnés, des tests et des preuves de read-back.

## 1. Diagnostic de l’existant

CYNIC possède déjà des fragments de cette constitution :

- `protocols/agent-constitution.md` : citoyens, scopes, budgets, admission, harness, preuves et non-amplification ;
- `infra/agent-roles.json` : RBAC déclaratif et rôles `human_admin`, `build_runner`, `fleet_coordinator`, `scheduler_agent`, `runtime_publisher` ;
- `crates/cynic-protocol/src/agent_citizenship.rs` : identité, scope, mode, budget et admission pure ;
- `crates/cynic-protocol/src/agent_workload/` : enveloppes, livraison et complétion ;
- `crates/cynic-protocol/src/federation_membership.rs` : membership et budget délégué ;
- `crates/cynic-protocol/src/outcomes.rs` : outcomes, KPI et preuves ;
- `infra/surface-registry.json` : ownership et surfaces exposées.

Ce qui manque est la synthèse institutionnelle :

- les agents ont des métiers mais pas encore un modèle explicite de profession ;
- les rôles existent mais pas le cycle de vie d’un mandat ;
- l’autorité d’exécution existe séparément de la réputation, sans constitution générale qui les relie ;
- les fonctions d’observation, d’exécution, d’évaluation et d’appel ne sont pas modélisées comme pouvoirs distincts ;
- il n’existe pas de registre actif du vocabulaire gouvernemental CYNIC ;
- la preuve de performance agentique n’est pas encore un profil canonique distinct de l’E-Score.

## 2. Définition politique

CYNIC est une membrane de gouvernement computationnel à souveraineté humaine unique.

- Le territoire est l’ensemble des nœuds, workspaces, registres, processus, données et surfaces explicitement admis.
- Le constituant souverain est l’opérateur humain.
- Les citoyens computationnels sont les agents, workloads, nœuds et services autorisés à proposer ou exécuter une action bornée.
- La constitution est formée par les contrats et invariants exécutables.
- Le gouvernement est la boucle `desired state → observe → reconcile → bounded action → read-back proof`.
- Le droit de mutation vient d’un mandat vérifié, jamais d’une réputation ou d’une intention seule.

CYNIC ne cherche pas à maximiser l’autonomie d’un agent. Il cherche à rendre l’autonomie locale lisible, bornée, révocable et prouvable.

## 3. Séparation des pouvoirs

### 3.1 Pouvoir constituant — HumanAdmin

Le pouvoir constituant :

- définit et amende les contrats, politiques et états désirés ;
- approuve les changements de registre et les promotions de harness ;
- peut suspendre une zone ou tout le runtime ;
- ne doit pas exécuter directement les mutations runtime steady-state.

Source existante : rôle `human_admin` dans `infra/agent-roles.json`.

### 3.2 Pouvoir exécutif — agents et scheduler node-local

Le pouvoir exécutif :

- reçoit un mandat admis ;
- observe son environnement ;
- exécute uniquement les actions du mandat ;
- persiste consommation, résultat et preuve ;
- s’arrête à l’expiration, au dépassement ou à la révocation.

Sources existantes : `scheduler_agent`, `build_runner`, `fleet_coordinator`, `cynic.agent_workload_envelope.v1`.

### 3.3 Pouvoir de preuve — observers et attestateurs

Le pouvoir de preuve :

- observe le substrat ou un résultat ;
- ne peut pas transformer une déclaration en observation ;
- conserve provenance, fraîcheur, portée et inconnues ;
- ne s’auto-attribue pas une autorité supérieure parce qu’il produit une métrique.

Sources existantes : host observers, node observations, `RuntimeProof`, `RuntimeHarnessTargetAttestation`.

### 3.4 Pouvoir de jugement — validateurs et tribunaux de contestation

Le jugement :

- compare mandat, observation, outcome et preuve ;
- produit `Converged`, `Partial`, `Unknown`, `Blocked` ou `Rejected` ;
- ne réécrit pas le fait observé ;
- ne peut pas augmenter le budget ou le scope d’un agent ;
- doit permettre la contestation par une preuve indépendante ou une décision HumanAdmin.

Ce pouvoir n’est pas encore un bounded context complet dans le runtime.

### 3.5 Trésor — budgets et ressources

Le trésor :

- alloue tokens, CPU, RAM, wall-time, stockage et réseau ;
- mesure la consommation ;
- refuse les allocations non couvertes ;
- distingue `Within`, `Exceeded` et `Unknown` ;
- ne transforme jamais une inconnue en solde disponible.

Les types existent, mais l’audit mathématique a montré que certaines dimensions et réservations ne sont pas encore appliquées atomiquement.

### 3.6 Registre civil — identités, rôles et mandats

Le registre civil :

- lie une identité à un rôle, un nœud, un harness et une période ;
- conserve les transitions, révocations et preuves ;
- ne confond pas identité, rôle, réputation et mandat ;
- rend la chaîne de délégation lisible.

`infra/agent-roles.json` couvre une partie de cette fonction, mais reste déclaratif et ne constitue pas encore un cycle de vie complet des offices.

## 4. Métier, rôle, mandat et incarnation

Ces concepts doivent rester distincts.

### 4.1 Définition générale de l’agent CYNIC

> Un agent CYNIC est un sujet computationnel ou un système composé qui, dans un environnement déterminé, reçoit un objectif ou un mandat, transforme des observations en propositions ou actions, utilise des effecteurs bornés et produit un résultat ainsi que des preuves vérifiables.
>
> Un agent peut être supervisé par un humain, fonctionner selon des règles, utiliser un ou plusieurs modèles, déléguer à d’autres agents ou agir de manière autonome. La supervision humaine, le modèle utilisé et le degré d’autonomie ne définissent pas à eux seuls l’agent.
>
> Un agent CYNIC n’est jamais défini par son apparence conversationnelle, son fournisseur, son modèle, son processus ou son nom de rôle seul. Il est défini par la relation entre sujet, environnement, mandat, observations, capacités, actions, effets, limites et preuves.

Cette définition est volontairement stable et suffisamment large pour couvrir les futurs use cases. Elle ne crée aucun droit d’action : la capacité effective reste déterminée par le mandat, le scope, le budget, le harness, l’admission et les preuves.

### 4.2 Ontologie évolutive et axes indépendants

La constitution ne doit pas maintenir une liste plate et fermée de « types d’agents ». Elle maintient un noyau d’agent et des axes extensibles :

- `identity` : humain, machine, session, service, workload, système composé ;
- `incarnation` : processus, session Hermes, nœud, scheduler, incarnation distante ;
- `harness` : CLI, MCP, sandbox, builder, runtime CYNIC ;
- `model` : règle, modèle local, modèle distant, ensemble de modèles, humain ;
- `mode` : observation, recommandation, HITL, pré-approbation, autonomie supervisée, audit postérieur ;
- `profession` : observer, auditor, planner, builder, scheduler, executor, judge, treasurer, publisher, archivist, federation ambassador ;
- `scale` : action, run, session, workload, office, nœud, flotte, gouvernement ;
- `decision` : réflexe, état interne, objectif, utilité, apprentissage, planification ;
- `coordination` : solitaire, hiérarchique, multi-agent, fédérée ;
- `causal_capability` : observation, intervention, attribution, contrefactuel ;
- `authority` : proposition, lecture, exécution bornée, publication, révocation.

Ces axes sont orthogonaux par défaut. Un même agent peut être, par exemple, un système composé, incarné par une session Hermes, utilisant GPT-5.6-luna, en mode HITL, exerçant le métier d’auditeur sur un workload de niveau dépôt.

Une nouvelle valeur d’axe est additive et doit conserver les valeurs inconnues ou non observées. Elle ne devient un rôle institutionnel qu’après définition de son owner, de ses capacités, de ses interdictions, de son mandat, de ses preuves et de son test de non-amplification.

### 4.3 Agent, modèle, outil, harness et workload

- `Agent` : sujet computationnel porteur d’un objectif ou d’un mandat.
- `Model` : mécanisme de raisonnement ou de génération utilisé par l’agent ; il ne possède pas le mandat.
- `Tool` : effecteur ou source d’observation exposé à l’agent ; il ne devient pas agent par sa puissance.
- `Harness` : membrane d’exécution, de contexte et de supervision.
- `Session/Run` : incarnation temporelle d’un agent à un instant donné.
- `Workload` : unité de travail admise et budgétée.
- `Office/Profession` : fonction institutionnelle stable.
- `Mandate` : autorisation temporaire et bornée d’exercer une fonction.
- `Citizen` : agent ou sujet reconnu par le registre constitutionnel comme pouvant recevoir un mandat CYNIC.

### 4.4 HITL comme mode d’agence

`HITL` n’est ni un modèle, ni un métier, ni une citoyenneté. C’est un mode de contrôle qui qualifie la position de l’humain dans la boucle :

- le modèle peut proposer ;
- l’agent peut préparer et exécuter ce qui est déjà autorisé ;
- l’humain conserve les décisions réservées et les approbations ;
- l’approbation doit être liée à l’action exacte, au scope, au budget et au moment ;
- une approbation ne doit pas être transformée en mandat permanent.

La session actuelle est donc un `AgentSystem` en mode `HITL`, pas une instance de GPT autonome.

### 4.5 Agent causal

Un `Agent causal` est un agent dont l’action ou la proposition est reliée à une hypothèse d’intervention, un objet affecté, un résultat attendu, une observation postérieure et une attribution qualifiée.

Il ne suffit pas qu’un événement suive l’action. Une preuve causale doit distinguer :

- observation corrélationnelle ;
- intervention effectivement exécutée ;
- résultat observé ;
- facteurs concurrents ;
- comparaison contrefactuelle lorsque disponible ;
- degré d’attribution et inconnues.

`Agent causal` est donc une capacité/profil transversal, pas une classe qui remplace tous les autres agents.

### Incarnation

Une incarnation est l’instance concrète : agent Hermes, Codex, scheduler CYNIC, workload ou nœud.

### Métier

Le métier est la fonction stable exercée dans le gouvernement : observer, construire, planifier, auditer, publier, coordonner ou archiver.

### Rôle

Le rôle est l’ensemble des capacités admissibles pour une incarnation dans un contexte donné.

### Mandat

Le mandat est une autorisation temporaire et bornée :

- sujet ;
- métier/rôle ;
- objet ;
- contexte ;
- scope ;
- capacités ;
- budget ;
- durée ;
- preuves requises ;
- conditions de révocation ;
- voie de contestation.

Un agent peut changer d’incarnation sans changer de métier. Un métier peut être exercé par plusieurs incarnations. Un rôle ne vaut que dans le périmètre d’un mandat valide.

## 5. Métiers existants et métiers à formaliser

### Déjà déclarés

- `human_admin` : constitution et seeds bootstrap ;
- `build_runner` : validation/build borné ;
- `fleet_coordinator` : adoption et réconciliation fleet ;
- `scheduler_agent` : exécution des workloads, récupération et receipts ;
- `runtime_publisher` : publication de preuves read-back.

### Métiers implicites à nommer

- `observer` : produit des observations attribuées, aucune mutation ;
- `auditor` : compare faits, contrats et preuves ;
- `judge` : produit un verdict contestable ;
- `resource_treasurer` : alloue et clôt les budgets ;
- `archivist` : conserve preuves, décisions, révisions et rétentions ;
- `federation_ambassador` : transporte une preuve ou une demande entre nœuds sans devenir autorité globale.

Ces métiers ne doivent pas être ajoutés comme rôles runtime avant leur owner, leurs permissions, leurs preuves et leurs tests de non-amplification.

## 6. Réputation : trois registres séparés

### 6.1 Autorité

L’autorité est un droit contractuel d’agir. Elle dépend de l’identité, du mandat, du scope, du harness, de l’admission et des preuves. Elle n’est pas calculée par un score social.

### 6.2 Fiabilité agentique

La fiabilité mesure la qualité observée d’une incarnation ou d’un métier dans un contexte :

- validité des preuves ;
- respect du budget ;
- qualité de l’outcome ;
- régressions introduites ;
- fraîcheur ;
- calibration ;
- coût et latence ;
- performance par domaine.

Elle doit être contextuelle, versionnée, contestable et attachée à des preuves. Elle ne doit pas être automatiquement convertible en capacité de mutation.

### 6.3 E-Score

L’E-Score est la réputation économique et comportementale de wallets, tokens ou actions publiques. Il peut être un signal externe concernant une incarnation qui agit on-chain, mais il n’est pas :

- une autorité d’exécution ;
- un score de compétence de compilation ;
- un permis de mutation ;
- un substitut à un mandat ;
- une mesure universelle de valeur humaine.

Règle : `E-Score ≠ AgentReliability ≠ AgentAuthority`.

## 7. Cycle de vie d’un office

Un métier ou rôle suit le cycle :

`Proposed → Defined → Vetted → Mandated → Active → Suspended → Retired`

Chaque transition exige :

- un acteur autorisé ;
- un objet et un scope ;
- une raison ;
- une preuve ou un blocker ;
- une durée et une condition de sortie.

`Suspended` ne signifie pas `Rejected`. Une suspension conserve les preuves et permet une révision. `Retired` interdit les nouveaux mandats mais conserve l’historique.

## 8. Contrat minimal d’un mandat

Un mandat d’agent doit répondre à Where/What/How/Why/Proof :

- Where : nœud, principal, métier, rôle, harness et scope ;
- What : objectif, objet, état désiré et outcome ;
- How : capabilities, ports, actions autorisées, budgets et stop conditions ;
- Why : contrat défendu, valeur attendue, risque et non-goals ;
- Proof : observations indépendantes, critères de réussite, rétention et contestation.

Absence d’une réponse : `Unknown` ou `Blocked`, jamais un mandat implicitement complet.

## 9. Lois institutionnelles

1. Aucun agent ne juge seul sa propre promotion.
2. Aucun score ne crée de capacité.
3. Aucun mandat ne survit à son expiration sans renouvellement explicite.
4. Aucun enfant ne reçoit plus de scope, budget ou capacité que son parent.
5. Aucun observer ne possède la politique qu’il interprète.
6. Aucun publisher ne publie une preuve qu’il n’a pas reçue d’un producteur inférieur.
7. Aucun marché ne remplace une observation.
8. Aucun document ne remplace un contrat ou un read-back.
9. Toute séparation de pouvoir doit être vérifiable par les imports, ports, tests ou preuves.
10. Toute fonction qui n’a pas de métier, de propriétaire et de preuve est une fonction orpheline jusqu’à résolution.

## 10. Géométrie gouvernementale

Le graphe institutionnel n’est pas seulement un graphe Cargo.

- sommet : citoyen, agent, office, nœud, registre, trésor ou tribunal ;
- arête : délégation, observation, décision, exécution, publication, contestation ou révocation ;
- poids : budget, durée, portée ou niveau de preuve ;
- direction : intent vers le bas, preuve vers le haut ;
- frontière : port, contrat et owner ;
- coupure : suspension, expiration ou perte de preuve.

Un graphe Cargo acyclique peut donc contenir une géométrie politique incohérente : import concret d’un pouvoir exécutif dans un observer, juge qui augmente un budget, ou projection CLI qui devient un registre.

## 11. Architecture cible du Stage 2

CYNIC vise une blockchain P2P de gouvernement computationnel. Le Stage 2 n’est pas encore présent dans le runtime : il constitue la cible nécessaire de réplication et de convergence.

- `cynic-protocol` : constitution, contrats, règles économiques, preuves et règles de consensus pures ;
- `cynic-kernel` : machine d’état constitutionnelle, ledger local, validation des transitions, historique et participation au consensus ;
- `cynic-node` : peer runtime, exécution des workloads, scheduler, ressources, leases, watchdog et participation au réseau ;
- `cynic-observer` : fédération de données, capteurs, ingestion et diffusion des observations ;
- `cynic-host` : substrat physique et preuves de capacité ;
- `cynic-inference` : compute cognitif, modèles et mesures de performance ;
- `cynic-intel` : interprétation économique, épistémique et métriques de mérite ;
- `cynic-askesis` : raisonnement supérieur, forensique et synthèse ;
- Solana : settlement public ou ancrage externe, jamais la constitution monétaire de CYNIC.

Dans la cible, chaque peer porte une instance locale du kernel. Le kernel n’est pas un cerveau central. Les kernels convergent via des protocoles P2P et des règles de consensus composables. Le node héberge et exécute cette instance ; il ne devient pas l’autorité de toutes les autres.

### 11.1 Fédération de données et blockchain

`cynic-observer` possède déjà une fédération libp2p avec mDNS, Kademlia, Gossipsub, PeerId et diffusion d’événements/preuves (`crates/cynic-observer/src/federation.rs:1-19,21-35`). Cela constitue une brique de perception fédérée, pas encore un ledger blockchain.

Il manque pour appeler le système une blockchain CYNIC :

- transactions ou événements validables ;
- blocs/epochs ;
- machine d’état répliquée ;
- consensus et fork choice ;
- finalité ;
- persistance de chaîne ;
- règles de recovery, replay et partition.

La cible minimale est donc :

`observation → transaction → bloc/epoch → transition d’état → réplication → consensus → finalité`

### 11.2 Miroir et jumeau numérique

Chaque citoyen peut disposer de projections séparées :

- `Mirror` : projection observée, attribuée, fraîche et consentie ;
- `DigitalTwin` : modèle de simulation des états, ressources et trajectoires ;
- `AdversarialTwin` : modèle contradictoire qui cherche les attaques, collusions, Sybils et effets indésirables.

Le miroir ne fabrique pas une identité. Le jumeau ne remplace pas le citoyen. Le jumeau adversarial ne reçoit aucun droit sur l’original. Ces objets doivent rester séparés du ledger de faits et des décisions de gouvernance.

### 11.3 Famille des consensus

CYNIC compose plusieurs preuves spécialisées, sans appeler chacune « vérité » :

- PoH : ordre et temps ;
- PoW/PoUW : travail ou travail utile ;
- PoS : garantie économique et exposition au risque ;
- PoJ : jugement indépendant ;
- PoSignal : qualité d’une observation externe ;
- PoCoverage : exploration de zones peu couvertes ;
- PoOutcome : résultat réel après action.

Chaque mécanisme doit déclarer : propriété prouvée, entrée, adversaire, coût, vérification, sortie et propriété non prouvée. La composition ne doit pas réduire ces dimensions à un score universel.

La méritocratie CYNIC est donc une évaluation de contributions et performances vérifiées sous une règle déclarée, non une mesure universelle de la valeur d’un humain.

## 12. État initial → état final

### Initial

CYNIC possède citoyenneté, RBAC, workloads, scopes, budgets et preuves dans des documents et types séparés. Les métiers sont présents implicitement ou sous forme de rôles techniques. L’E-Score, la fiabilité d’agent et l’autorité ne sont pas encore unifiés conceptuellement.

### Transitoire

1. Valider ce document comme modèle, sans ajouter encore de score.
2. Construire une matrice `métier × rôle × mandat × pouvoir × preuve` depuis `infra/agent-roles.json` et `agent-constitution.md`.
3. Distinguer les rôles observés, déclarés, proposés et bloqués.
4. Définir un profil de fiabilité agentique séparé de l’E-Score.
5. Auditer les chemins où un score, une métrique ou un résultat peut augmenter une capacité.
6. Ajouter un registre actif du vocabulaire institutionnel.
7. Transposer un seul métier pilote en types et tests : `observer` ou `build_runner`.

### Final

Un agent reçoit un mandat lisible dans un sous-graphe institutionnel : métier, office, scope, budget, pouvoirs séparés, preuves et voie de contestation. L’agent sait où agir et pourquoi, mais ne peut pas convertir sa réputation ou son propre jugement en souveraineté.

## 13. Décisions encore ouvertes

- Le terme officiel est-il `gouvernement computationnel`, `cité computationnelle` ou `runtime constitutionnel` ?
- Le métier est-il un type stable de protocole ou une projection du registre ?
- Le jugement doit-il être produit par un module protocol pur ou par un bounded context `askesis/intel` ?
- Qui constitue le tribunal de contestation : HumanAdmin, peer jury, preuve cryptographique ou combinaison ?
- La fiabilité est-elle conservée par incarnation, par métier, par nœud ou par contexte ?
- Quelle part de la réputation peut être publiée sans transformer l’E-Score en autorité implicite ?

Aucune de ces questions ne doit être résolue par un score par défaut ou une constante esthétique.