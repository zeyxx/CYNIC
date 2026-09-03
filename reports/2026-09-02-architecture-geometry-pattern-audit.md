# Géométrie architecturale et patterns agentiques de CYNIC

Date : 2026-09-02
Statut : audit read-only et modèle de discussion, non normatif
Révision observée : `6431121234638b2d6c4b34bbb78bb97bed0e1faf`

Ce rapport part du pattern communiqué par l’opérateur et attribué à Ahmad Osman : modularité, Domain-Driven Design, spécifications douloureusement explicites et documentation abondante. La citation fournie est une source de discussion, pas une autorité runtime.

Ce document vit dans `reports/` : il ne modifie ni les contrats, ni l’ownership, ni la topologie du runtime.

## 1. Thèse

La discipline structurelle réduit davantage les erreurs des agents que l’augmentation brute de leur capacité de génération.

Pour CYNIC, la recette devient :

1. modularité ;
2. Domain-Driven Design ;
3. spécifications explicitant toutes les frontières ;
4. documentation exhaustive aux frontières, minimale dans les implémentations ;
5. preuve architecturale exécutable.

La quantité documentaire seule ne suffit pas. Une bonne carte doit être subordonnée au terrain observé et ne jamais devenir une seconde autorité.

## 2. Définition de la géométrie architecturale

La géométrie de CYNIC se décrit par cinq objets.

### 2.1 Régions

Les régions sont les bounded contexts dans lesquels un langage et un modèle restent cohérents. Une crate peut matérialiser une région, mais crate et bounded context ne sont pas synonymes : une crate technique peut contenir plusieurs domaines, et un domaine peut traverser plusieurs unités de compilation.

### 2.2 Frontières

Une frontière nomme un propriétaire, une responsabilité primaire et ce qui lui est interdit. La forme CYNIC existe déjà dans `infra/CRATE_MODELS.md` : PRIMAIRE / POSSÉDÉ / DÉLÉGUÉ / INTERDIT.

### 2.3 Arêtes

Une arête représente une traversée de frontière. Elle doit porter :

- sa source et sa cible ;
- sa direction ;
- le type de relation : contrat partagé, port, adaptateur, projection, événement, appel concret ou compatibilité ;
- l’objet transporté ;
- le propriétaire de la décision ;
- la preuve d’existence et de conformité.

Une dépendance Cargo ne suffit pas à qualifier l’arête. Elle prouve une possibilité de compilation, pas la nature sémantique du couplage.

### 2.4 Flux orientés

Les flux cohérents ont une orientation :

- intent et capability descendent ;
- observation, consommation et preuve remontent ;
- une projection expose sans recalculer ;
- une mutation traverse une admission explicite.

### 2.5 Cartes

Les documents, diagrammes et registres sont des cartes de cette géométrie. Ils ne sont valides que s’ils indiquent leur source, leur statut et leur fraîcheur. Le code exécuté et l’état observé restent le terrain.

## 3. Les cinq coordonnées d’une frontière

Toute responsabilité doit répondre à :

### Where

- bounded context et propriétaire ;
- crate/module ;
- nœud, principal et harness ;
- registre ou store autoritaire ;
- isolation boundary.

### What

- objet exact ;
- état désiré ;
- état observé ;
- invariants et unités ;
- inconnues résiduelles.

### How

- port et adaptateur ;
- règle pure de réconciliation ;
- action bornée ;
- consentement, budget, stop condition et rollback ;
- read-back indépendant.

### Why

- contrat source ;
- risque évité ;
- propriété protégée ;
- raison pour laquelle une frontière existante ne peut pas absorber autrement la responsabilité ;
- observation qui réfuterait la décision.

### Proof

- producteur inférieur de la preuve ;
- consommateur ;
- schéma et fraîcheur ;
- résultat en cas d’absence : `Unknown` ou `Blocked`, jamais succès implicite.

`infra/SOVEREIGN_SYSTEM_DESIGN.md:18-27`, `AGENTS.md:206-215` et `crates/cynic-protocol/src/constitutional_law.rs:61-159` couvrent déjà une grande partie de ce modèle. Le trou n’est pas la formulation ; c’est son application uniforme et son read-back.

## 4. Mesure de la topologie actuelle

Mesure effectuée depuis les manifests et les sources Rust sous `crates/*/src`, sur le SHA indiqué. LOC = lignes non vides et non commentaires ; cette métrique mesure un volume, pas la complexité cognitive.

| Crate | Fichiers Rust | LOC mesurées | Traits publics |
|---|---:|---:|---:|
| cynic-host | 532 | 88 424 | 94 |
| cynic-protocol | 220 | 52 245 | 15 |
| cynic-node | 328 | 48 544 | 88 |
| cynic-kernel | 284 | 47 356 | 28 |
| cynic-cli | 195 | 29 534 | 0 |
| cynic-inference | 138 | 20 884 | 24 |
| cynic-observer | 42 | 7 554 | 9 |
| cynic-askesis | 55 | 5 602 | 12 |
| cynic-intel | 36 | 3 814 | 12 |
| cynic-market | 6 | 861 | 0 |
| cynic-zeroclaw-solana | 4 | 295 | 0 |

Total : 11 crates, 1 840 fichiers Rust, 305 113 LOC mesurées, 282 traits publics.

Quatre crates dépassent le seuil indicatif de 40 kLOC : host, protocol, node et kernel. Ce n’est pas une preuve de mauvaise architecture, mais un signal nécessitant une analyse de responsabilités et de charge cognitive.

## 5. Graphe déclaré des crates

Le graphe Cargo observé contient 26 arêtes dirigées et aucun cycle déclaré.

Principaux hubs :

- `cynic-protocol` reçoit les dépendances de neuf autres crates : il est le shared kernel structurel ;
- `cynic-cli` dépend de six crates : c’est acceptable seulement s’il reste composition/projection ;
- `cynic-kernel` dépend de cinq crates : il reste un agrégateur à forte gravité ;
- `cynic-host` et `cynic-node` ont simultanément beaucoup d’arêtes et beaucoup de responsabilités.

Le DAG Cargo est sain au sens acyclique. Il ne prouve toutefois pas :

- que les imports cross-domain passent par des ports ;
- que le propriétaire du type est le propriétaire de la décision ;
- que les dépendances sont nécessaires ;
- que les projections restent minces ;
- que le flux d’autorité suit la bonne direction.

Le grep initial a produit un faux cycle protocol → host parce que `constitutional_law.rs` contient des chemins `cynic_host::...` dans des chaînes de test. Cela démontre qu’un simple grep n’est pas une preuve suffisante : la future mesure doit utiliser le graphe Cargo et une analyse syntaxique des imports, pas compter les mots.

## 6. État des disciplines du pattern Ahmad

### 6.1 Modularité — loi forte, réalisation incomplète

Déjà couvert :

- `one_boundary_one_reason` ;
- `protocol_is_pure` ;
- `cli_is_adapter` ;
- `proof_producers_are_low_layer` ;
- ports et adaptateurs largement présents ;
- tests d’architecture dans six crates.

Écarts :

- quatre crates au-dessus du seuil indicatif de 40 kLOC ;
- kernel encore dépendant de surfaces concrètes host/inference/node ;
- host agrège substrat, lifecycle, fleet, build et Forgejo ;
- protocol risque de devenir le réceptacle de tout type partagé ;
- cinq crates n’ont pas de `tests/architecture_boundary.rs` : inference, intel, observer, market et zeroclaw-solana.

### 6.2 DDD — domaines nombreux, bounded contexts non stabilisés

CYNIC possède beaucoup de modules nommés par domaine, mais ses crates principales restent souvent nommées par couche technique : host, node, kernel, protocol, CLI. Ces noms n’imposent pas un langage métier borné et peuvent devenir des attracteurs de responsabilités.

Signaux :

- `cynic-intel` est défini comme propriétaire de tout raisonnement/ML/algorithme, définition trop large pour constituer un bounded context stable ;
- `host` possède plusieurs sous-domaines opérationnels indépendants ;
- `kernel` conserve des résidus de domaines produit ;
- plusieurs termes ont des sens concurrents : epoch, slot, graph, topology, vector, proof, observation ;
- le seul `DOMAIN_GLOSSARY.md` trouvé est archivé sous `docs/archive/` ; aucun ubiquitous language actif n’est présent.

### 6.3 Spécifications explicites — couverture riche, autorité fragmentée

Déjà couvert :

- design gate typé dans `constitutional_law.rs` ;
- grilles OWNS/DELEGATES/FORBIDDEN ;
- contrats JSON et schémas Rust ;
- audits crate/ownership/kernel ;
- plans explicitant souvent desired/observe/reconcile/action/proof.

Écarts :

- `CRATE_MODELS.md`, `CRATE_OWNERSHIP.md` et `SOVEREIGN_SYSTEM_DESIGN.md` utilisent tous un vocabulaire d’autorité ou de canonicité ;
- `CRATE_OWNERSHIP.md` est simultanément présenté comme draft et comme autorité ;
- des descriptions architecturales peuvent annoncer des capacités plus fortes que leur statut runtime ;
- aucune relation machine-readable ne relie systématiquement une règle documentaire à son contrat source et à son audit de read-back.

### 6.4 Documentation — abondante, mais à normaliser

Le dépôt contient 204 fichiers Markdown suivis par Git. Soixante-dix-sept emploient au moins un terme comme canonical, canonique, source de vérité ou SSOT. Ce nombre ne signifie pas que 77 documents revendiquent tous l’autorité ; il indique néanmoins une forte densité de vocabulaire canonique qui augmente le coût de discrimination pour un agent.

Répartition principale : 79 sous `docs/`, 38 sous `.cortex/`, 28 sous `infra/`, 26 sous `crates/`, 11 sous `protocols/`.

La correction n’est pas de supprimer toute documentation. Elle est de distinguer :

- loi : contrats et types ;
- spécification : protocole subordonné à une loi nommée ;
- décision : ADR daté avec alternatives et conséquences ;
- carte observée : générée depuis le code/runtime avec SHA et fraîcheur ;
- plan : intention non admise ;
- rapport : preuve de lecture, non autorité ;
- archive : historique explicitement non actif.

## 7. Read-back des audits existants

Le binaire local observé rapporte :

- `cynic dev crate-audit --json` : `blocked`, quatre crates bloquées ;
- `cynic dev ownership-audit --json` : `ok`, avec deux migrations encore déclarées ;
- `cynic dev kernel-audit --json` : `blocked`, 284 fichiers, 151 couverts, 133 non couverts et deux hotspots.

Les quatre blocages crate-audit concernent CLI, node, protocol et un ancien `cynic-mcp` attendu par l’audit alors que cette crate n’est plus membre du workspace observé. Ce dernier point signale une possible projection compilée ou règle d’audit périmée ; il doit rester `Unknown` jusqu’au rebuild/read-back du CLI.

Le statut `ownership-audit=ok` malgré deux migrations n’est pas nécessairement incorrect, mais son vocabulaire peut être mal interprété par un agent : « schéma cohérent » et « architecture convergée » doivent être deux statuts distincts.

## 8. Comparaison de patterns

### DDD Context Map

À absorber : bounded contexts, ubiquitous language, relations upstream/downstream, shared kernel, anti-corruption layer.

Usage CYNIC : qualifier les arêtes entre domaines. `cynic-protocol` ne doit être un shared kernel que pour les contrats réellement partagés, pas pour toute logique réutilisable.

### C4

À absorber : niveaux de zoom Context / Container / Component / Code et vues de déploiement séparées.

Usage CYNIC : ne pas confondre crate, processus, nœud et bounded context. Une crate est une unité de compilation ; un container C4 est une unité runtime ; un bounded context est une frontière sémantique.

### Hexagonal / Ports and Adapters

Déjà largement couvert. Le manque est une mesure des arêtes concrètes qui traversent une frontière malgré l’existence d’un port.

### arc42

À absorber : contraintes, vues building-block/runtime/deployment, décisions, risques et glossaire dans une structure finie. Intérêt principal : réduire la dispersion documentaire sans perdre le Why.

### Architecture tests

Déjà présents dans six crates. À étendre à toutes les frontières actives et à alimenter depuis une politique machine-readable unique plutôt que par des assertions locales divergentes.

### Team Topologies

À absorber comme critère de split : charge cognitive et flux de valeur, pas uniquement LOC. Pour un opérateur humain assisté d’agents, le « team API » devient l’interface contextuelle donnée aux agents. Une frontière est bonne si un agent peut agir sans charger tout le dépôt.

## 9. Invariants géométriques proposés

Ces invariants sont des conclusions d’audit, pas encore une loi admise.

1. Une capacité possède exactement un propriétaire de décision.
2. Une responsabilité primaire appartient à exactement un bounded context.
3. Toute arête cross-context est nommée et qualifiée.
4. Toute traversée concrète non autorisée est bloquée par un test d’architecture.
5. Le graphe d’autorité est acyclique, même si le graphe d’appels runtime peut comporter des boucles.
6. Intent et capability descendent ; observation et preuve remontent.
7. CLI, MCP et rapports sont des feuilles de projection, jamais des centres de politique.
8. Une inconnue ne devient ni arête valide, ni absence d’arête, ni convergence.
9. Une carte désirée et une carte observée sont deux objets séparés.
10. Toute documentation dite canonique nomme sa source supérieure et son test de dérive.
11. Les termes polysémiques sont qualifiés par contexte dans un glossaire actif.
12. Un split est justifié par responsabilité et charge cognitive ; le LOC n’est qu’un signal.

## 10. Forme minimale d’un atlas machine-readable

Un futur atlas ne doit pas devenir une nouvelle autorité. Il doit projeter les contrats existants et les observations de code.

### Sommet

- `context_id` ;
- `owner` ;
- `primary_responsibility` ;
- `ubiquitous_language_ref` ;
- `authority_source_ref` ;
- `runtime_units` ;
- `source_units` ;
- `status: observed | declared | proposed | blocked | unknown`.

### Arête

- `from_context` ;
- `to_context` ;
- `relationship: port | contract | event | adapter | projection | concrete_import | compatibility` ;
- `direction` ;
- `transported_type` ;
- `decision_owner` ;
- `allowed` ;
- `source_evidence` ;
- `residual_unknowns`.

### Read-back

- graphe Cargo ;
- imports syntaxiques ;
- registre des surfaces ;
- consommateurs des ports ;
- tests d’architecture ;
- statut des capacités dans `FrameworkSpec` ;
- runtime/deployment observé lorsque la relation concerne un processus ou un nœud.

## 11. État initial, transitoire, final

### État initial

CYNIC possède les bonnes lois, de nombreux ports et plusieurs cartes, mais les bounded contexts et la sémantique des arêtes ne sont pas représentés par une source unique. Les agents doivent reconstruire cette géométrie en lisant plusieurs documents et peuvent choisir la mauvaise carte.

### État transitoire

1. Réactiver un glossaire borné par contexte, sans restaurer l’ancien glossaire archivé comme autorité.
2. Inventorier les bounded contexts réels indépendamment des crates.
3. Projeter le DAG Cargo et les imports cross-crate avec classification port/concret.
4. Relier chaque context à PRIMAIRE/OWNS/DELEGATES/FORBIDDEN.
5. Donner aux documents un type et un statut explicites.
6. Étendre les architecture-boundary tests aux cinq crates non couvertes.
7. Distinguer dans les audits `schema_coherent`, `migration_pending` et `runtime_converged`.

### État final

Un agent reçoit un sous-graphe borné contenant :

- son contexte et son langage ;
- les frontières autorisées ;
- les ports disponibles ;
- les interdictions ;
- le desired/observed delta ;
- les preuves exigées ;
- les inconnues.

Il n’a pas besoin de charger 204 documents pour savoir où agir. La documentation devient une projection adaptée au contexte, générée depuis une géométrie vérifiable.

## 12. Plus petite prochaine tranche

Avant toute correction Rust : produire la table des bounded contexts actifs et qualifier les 26 arêtes Cargo selon le Context Map DDD : shared contract, customer/supplier via port, projection, adapter ou couplage concret à éliminer.

Cette table doit réutiliser `infra/CRATE_MODELS.md`, `infra/surface-registry.json` et les audits existants. Elle ne doit pas introduire une nouvelle taxonomie concurrente. Le premier résultat attendu est une carte observée et contestable, pas une extraction de crate.