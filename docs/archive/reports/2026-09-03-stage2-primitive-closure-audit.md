# Audit de fermeture des verticales primitives vers le Stage 2

Date : 2026-09-03
Statut : audit read-only, non normatif
Révision observée : `0c4d2f8b`

## 1. Cible

CYNIC vise un gouvernement computationnel P2P capable d’accueillir davantage de citoyens humains, leurs agents et éventuellement des agents autonomes.

Le Stage 2 minimal est :

`observation → transaction → état local → réplication → consensus → finalité → preuve`

La montée en échelle doit rester bloquée tant que les primitives de citoyenneté, de mandat, d’exécution, de preuve, de ledger et de contestation ne sont pas fermées.

## 2. État réel observé

`infra/verticals/manifest.json` déclare 19 verticales. Leurs fichiers `intent.json` ont tous une forme structurelle `desired_state + observe + reconcile + bounded_action + read_back/proof` selon le détecteur lexical utilisé. Cette complétude structurelle ne prouve pas leur convergence runtime.

Les statuts de `contracts/framework.json` restent notamment :

- `sovereign_runtime` : `partial` ;
- `sovereign_inference` : `partial` ;
- `multi_source_truth` : `partial` ;
- `replayable_memory` : `partial` ;
- `talaria_code_evolution` : `blocked` ;
- `resource_metabolism` : `blocked` ;
- `node_recursion` : `blocked` ;
- `fleet_coordination` : `blocked` ;
- `runtime_federation` : `absent` ;
- `solana_public_settlement` : `absent` ;
- `one_person_futarchy` : `absent` ;
- `contestable_judgment` : `absent`.

La structure déclarée des verticales est donc en avance sur les preuves de runtime.

## 3. Primitives nécessaires à l’onboarding

### P0 — Citoyenneté et identité

État : partiellement présent.

Présent :

- `AgentScope`, `AgentMode`, `AgentBudgets` ;
- `AgentCitizenshipRecord` ;
- machine enrollment ;
- rôles déclaratifs dans `infra/agent-roles.json` ;
- preuves d’exécution dans `agent_execution_provenance.rs`.

Manquant :

- séparation exécutable Agent/Citizen/MandatedAgent ;
- liaison humaine → machines → agents → workloads ;
- identité multi-incarnation ;
- statut de citoyenneté révocable ;
- identité et mandat d’un agent autonome sans humain parent.

Gate d’échelle : aucun nouvel agent ne reçoit de droit par simple découverte, nom, modèle ou connexion réseau.

### P0 — Mandat et métier

État : rôles présents, office et mandat incomplets.

Présent : `build_runner`, `fleet_coordinator`, `scheduler_agent`, `runtime_publisher`, `human_admin` dans `infra/agent-roles.json`.

Manquant :

- office explicite ;
- mandat temporel avec objet, scope, budget et preuves ;
- conditions de suspension/retrait ;
- conflit d’intérêts ;
- voie d’appel ;
- séparation entre rôle, permission et performance.

### P0 — Admission et allocation des ressources

État : `resource_metabolism` et certaines formes d’admission sont bloqués.

Présent :

- `EnergyBudget` ;
- `TaskAdmissionPort` ;
- queue de tâches ;
- `EnergyLedger` déclaratif ;
- preuves de consommation partiellement modélisées.

Manquant ou non fermé :

- réservation atomique ;
- contrôle complet du wall-time ;
- clôture `Within | Exceeded | Unknown` ;
- cumul multidimensionnel ;
- allocation de capacité réellement observée ;
- politique d’allocation entre citoyen, nœud, agent et workload.

Règle : la gouvernance alloue un droit de consommation ; elle ne crée pas de CPU, GPU, RAM ou énergie.

### P0 — Ledger local et état constitutionnel

État : absent comme blockchain CYNIC.

Présent :

- journaux de fenêtres ;
- preuves d’exécution ;
- stockage de coordination ;
- quelques ancres PoH et événements.

Manquant :

- transactions formelles ;
- état canonique ;
- transition déterministe ;
- ledger local réconciliable ;
- replay ;
- détection de double dépense ;
- finalité ;
- fork choice ;
- snapshots d’état.

C’est la primitive racine du Stage 2.

### P0 — Consensus composable

État : mécanismes proposés, pas encore composés dans une chaîne CYNIC.

La famille doit rester séparée :

- PoH : ordre/temps ;
- PoW ou PoUW : travail utile ;
- PoS : garantie exposée ;
- PoJ : jugement ;
- PoSignal : observation externe ;
- PoCoverage : espace exploré ;
- PoOutcome : résultat réel.

Manquant :

- contrat commun Proof-of-X ;
- propriété prouvée par mécanisme ;
- adversaire et coût ;
- vérificateur ;
- règle de composition ;
- seuils et quorum ;
- comportement sous partition et conflit ;
- distinction consensus d’état / consensus épistémique / consensus de gouvernance / consensus de marché.

Un mécanisme ne doit pas être appelé consensus simplement parce qu’il produit un score.

### P1 — Fédération P2P de données

État : brique présente, pas blockchain.

`cynic-observer/src/federation.rs` possède libp2p, Gossipsub, Kademlia, mDNS, PeerId et diffusion d’événements/preuves.

Manquant :

- admission des pairs ;
- identité et rôle de chaque pair ;
- validation des messages ;
- anti-replay ;
- persistance ;
- ordre ;
- consensus ;
- finalité ;
- synchronisation de l’état.

Le composant est correctement classifié comme data federation plane.

### P1 — Miroir, jumeau et adversaire

État : principalement vision/propositions.

Manquant :

- miroir observé privé/public ;
- jumeau numérique dynamique ;
- jumeau adversarial ;
- consentement et minimisation ;
- modèle de données du sujet ;
- séparation entre simulation et fait ;
- règles de non-intervention sur l’original ;
- attribution des scénarios contrefactuels.

Cette verticale est requise avant une économie de réputation à grande échelle, mais ne doit pas bloquer le ledger minimal Stage 2 si elle reste hors des chemins d’autorité.

### P1 — Monnaie et trésor

État : doctrine riche, runtime absent.

Présent dans les documents : CYD, `$CYNIC`, PoU, PoB, PoL, treasury, E-Score et staking.

Manquant :

- comptes ;
- double entrée ;
- émission ;
- destruction ;
- réserves ;
- allocation ;
- récompense ;
- dette ;
- insolvabilité ;
- contestation ;
- relation exacte entre CYD-calibration et token Genesis.

Stage initial retenu : CYD est un crédit de calibration pré-Genesis sans redemption ni valeur publique.

### P1 — Settlement Solana

État : absent comme intégration CYNIC.

Solana doit fournir à terme :

- comptes publics ;
- mint/test token ;
- transferts ;
- burn ;
- signatures ;
- settlement ;
- ancrage de preuves.

Solana ne doit pas devenir la source de vérité des ressources physiques ni la constitution de CYNIC.

## 4. Monolithes et charge de fermeture

Les volumes observés confirment une dette de découpage :

- host : 88 424 LOC mesurées ;
- protocol : 52 245 ;
- node : 48 544 ;
- kernel : 47 356.

Le seuil LOC n’est qu’un signal. Les vrais problèmes sont les responsabilités multiples et les arêtes concrètes :

- kernel importe encore application/adapters inference ;
- node et host se recouvrent sur runtime, lifecycle et ressources ;
- protocol porte des familles nombreuses ;
- CLI projette plusieurs propriétaires depuis une surface très large.

Il ne faut pas lancer le Stage 2 en ajoutant un super-crate blockchain. Il faut fermer une primitive par owner existant et extraire uniquement quand une frontière prouvée l’exige.

## 5. Ordre de fermeture recommandé

### Vague A — Premier citoyen et premier peer

1. identité multi-entité : citoyen, node, ressource, agent, workload, preuve ;
2. mandat minimal ;
3. observation et allocation des ressources ;
4. ledger local CYD-calibration ;
5. proof-of-useful-work minimal ;
6. attribution et clôture.

### Vague B — Réseau P2P interne

1. pair admission ;
2. échange d’événements ;
3. réplication du ledger ;
4. transition d’état déterministe ;
5. consensus d’état minimal ;
6. finalité et recovery.

### Vague C — Adversarial calibration

1. double dépense ;
2. Sybil ;
3. collusion ;
4. spam/transaction farming ;
5. compute inutile ;
6. nœud menteur ;
7. partition ;
8. jumeau adversarial par citoyen.

### Vague D — Solana Devnet

1. token de test explicitement non canonique ;
2. mint/transfer/burn ;
3. settlement ;
4. preuve d’équivalence ledger ↔ chaîne ;
5. snapshot de calibration ;
6. simulation d’airdrop Genesis.

## 6. Critère d’onboarding public

CYNIC ne doit pas inviter de nouveaux humains ou agents autonomes avant de pouvoir prouver :

- qui est le sujet ;
- quel office il exerce ;
- quel mandat il possède ;
- quelles ressources il fournit ou consomme ;
- qui valide son travail ;
- comment ses crédits sont créés ;
- comment une fraude est contestée ;
- comment une panne ou partition est récupérée ;
- comment son miroir et son jumeau respectent consentement et confidentialité ;
- comment il quitte le réseau et récupère ou perd ses droits.

Un agent autonome peut rejoindre sans humain parent seulement lorsqu’un protocole d’admission définit explicitement sa citoyenneté, sa responsabilité, ses capacités, son trésor, son révocateur et ses preuves. La connectivité P2P ne vaut pas admission.

## 7. Conclusion

Le Stage 2 est bien la cible nécessaire, mais il commence par un peer mono-opérateur correctement instrumenté, pas par une décentralisation narrative.

Le premier citoyen humain peut contrôler plusieurs machines, agents et ressources. CYNIC doit néanmoins enregistrer ces fonctions comme entités séparées, afin que la calibration révèle les bonnes règles au lieu de masquer les transferts de valeur entre propriétaire, fournisseur, exécutant et vérificateur.

La prochaine verticale primitive est donc :

`citoyen → node → ressource → agent → workload → preuve → ledger de calibration`

C’est le premier segment qui relie gouvernement, économie, compute et future blockchain sans prétendre que le réseau est déjà décentralisé.