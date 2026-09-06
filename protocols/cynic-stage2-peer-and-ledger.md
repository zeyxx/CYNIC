# Stage 2 — CynicPeer et ledger constitutionnel local

Date : 2026-09-03
Statut : proposition de design en révision
Autorité : subordonnée à `contracts/foundation.json`, `contracts/framework.json` et aux preuves runtime

## 1. But

Le Stage 2 est la première forme réelle de CYNIC comme blockchain P2P de gouvernement computationnel.

La cible minimale n’est pas encore un token public. C’est un peer capable de :

- représenter un citoyen, ses nœuds, ses agents et ses ressources ;
- recevoir un mandat borné ;
- allouer une capacité observée ;
- exécuter un workload ;
- enregistrer un résultat et ses preuves ;
- répliquer un état déterministe ;
- converger avec d’autres peers ;
- produire des crédits CYD de calibration sans redemption publique.

Chaîne minimale :

`observation → transaction → transition d’état → journal local → réplication → consensus → finalité → preuve`

## 2. Définition du peer

Un `CynicPeer` est une unité P2P opérée par un nœud qui possède :

- une identité cryptographique de peer ;
- une instance locale du kernel constitutionnel ;
- un ledger local ;
- des capacités de substrat observées ;
- des agents et offices hébergés ;
- des liens vers des observers ;
- un état de membership ;
- une politique de consensus et de recovery ;
- un sink de preuves.

Un peer peut appartenir au premier citoyen humain, à une communauté ou à une organisation. La propriété du peer ne donne pas automatiquement le droit de falsifier son état ou les preuves produites par ses agents.

## 3. Entités du premier peer

Les entités sont séparées même lorsqu’elles appartiennent au même opérateur.

### CitizenAccount

Porte l’identité constitutionnelle et les décisions réservées à l’humain ou à une autorité reconnue.

Ne contient pas directement les soldes de ressources physiques.

### NodeAccount

Représente une machine ou un environnement de calcul :

- peer id ;
- identité machine ;
- capabilities observées ;
- capacité disponible ;
- état de membership ;
- preuves de liveness ;
- owner/custodian.

### AgentAccount

Représente une incarnation agentique :

- agent id ;
- parent éventuel ;
- profession/office ;
- harness ;
- modèle utilisé ;
- mode d’autonomie ;
- capacités reçues ;
- limites ;
- fiabilité observée ;
- preuves et inconnues.

Le modèle, par exemple GPT-5.6-luna, est une composante référencée de l’agent, pas son identité constitutionnelle.

### ResourceAccount

Représente une ressource observée ou une capacité allouable :

- CPU/GPU ;
- mémoire ;
- disque ;
- réseau ;
- énergie estimée ;
- fenêtre de validité ;
- source de mesure ;
- owner/custodian ;
- inconnues.

Une ressource ne peut être inscrite que par observation. Une déclaration seule crée un état `Declared`, pas `Available`.

### OfficeAccount

Décrit un métier institutionnel exercé dans un contexte : observer, builder, scheduler, auditor, judge, treasurer, publisher ou autre office émergent.

L’office porte les pouvoirs et interdictions, pas le budget individuel.

### MandateAccount

Autorise une action ou une série d’actions pendant une période bornée :

- sujet ;
- office ;
- objet ;
- scope ;
- capabilities ;
- budget ;
- fenêtre temporelle ;
- preuves requises ;
- stop conditions ;
- révocation ;
- contestation.

### WorkloadAccount

Décrit une unité de travail admise :

- workload id et idempotency key ;
- mandat ;
- node cible ;
- agent exécutant ;
- ressource réservée ;
- action bornée ;
- état ;
- consommation ;
- outcome ;
- preuves.

### EvidenceAccount

Référence une preuve content-adressée sans nécessairement contenir le payload privé :

- source ;
- producteur ;
- objet ;
- horodatage et fraîcheur ;
- digest ;
- statut ;
- relation causale éventuelle ;
- inconnues.

### TreasuryAccount

Représente les créances et allocations comptables du gouvernement CYNIC :

- réserve ;
- crédits CYD-calibration ;
- escrow ;
- dette ;
- récompenses en attente ;
- dépenses ;
- contestations.

Le trésor ne prétend pas posséder l’énergie physique. Il enregistre des allocations ou des créances liées à des preuves.

## 4. États et relations

Les états désirés, observés et comptables restent séparés :

`DesiredState ≠ ObservedState ≠ LedgerState ≠ PublicProjection`

Une relation entre entités doit être attribuable :

- citoyen possède ou administre un node ;
- node expose une ressource ;
- office autorise un mandat ;
- mandat admet un workload ;
- agent exécute le workload ;
- observer mesure la ressource ou le résultat ;
- auditor/judge évalue la preuve ;
- treasury enregistre une allocation ou un crédit.

Aucune relation ne doit être déduite seulement d’un nom, d’une adresse IP, d’un modèle, d’un wallet ou d’une proximité temporelle.

## 5. Transactions du ledger

Le ledger local n’accepte que des événements versionnés et idempotents. La première famille minimale est :

1. `CitizenRegistered`
2. `NodeBound`
3. `ResourceObserved`
4. `OfficeDefined`
5. `MandateGranted`
6. `WorkloadAdmitted`
7. `WorkloadStarted`
8. `ResourceReserved`
9. `ResourceConsumed`
10. `WorkloadCompleted`
11. `OutcomeObserved`
12. `EvidenceAttested`
13. `CalibrationCreditRecorded`
14. `DisputeOpened`
15. `CreditSuspended`
16. `MandateRevoked`
17. `PeerJoined`
18. `PeerCheckpointed`

Les événements sont append-only. Une correction produit un nouvel événement référencé ; elle ne réécrit pas silencieusement le passé.

Chaque transaction doit porter :

- transaction id ;
- issuer/subject ;
- parent ou predecessor ;
- nonce ou idempotency key ;
- payload typé ;
- input evidence refs ;
- signature ou autorité de soumission ;
- timestamp logique ;
- statut de validation.

## 6. Machine de transition

Une transition est valide seulement si :

- le schéma est connu ;
- le sujet possède l’office ou le mandat requis ;
- le mandat couvre l’objet et le scope ;
- le budget restant couvre la demande ;
- les preuves nécessaires sont fraîches ;
- l’idempotency key n’a pas déjà été consommée ;
- l’état précédent autorise la transition ;
- aucun conflit de fencing ou de nonce n’est détecté.

Une absence de preuve donne `Unknown` ou `Blocked`, jamais `Available`, `Complete` ou `Settled`.

La transition comptable ne mint pas d’énergie. Elle peut seulement :

- réserver une capacité observée ;
- enregistrer une consommation ;
- créer un crédit de calibration prévu par la politique ;
- déplacer une créance existante ;
- ouvrir une dette ou une contestation.

## 7. Allocation de ressources

La boucle du premier peer est :

`ResourceObserved → CapacityAvailable → GovernanceDecision → MandateGranted → ResourceReserved → WorkloadExecuted → ConsumptionReadBack`

La gouvernance peut allouer un quota à un agent ou à un citoyen. Elle ne peut pas déclarer disponible une capacité non observée.

Quand le propriétaire, le node, l’agent et le workload sont la même personne ou organisation, les quatre comptes restent distincts. Cette séparation est nécessaire pour mesurer les conflits d’intérêts et la contribution réelle.

## 8. CYD de calibration

Au Stage 2, CYD est une unité comptable pré-Genesis :

- non publique ;
- non redeemable ;
- non cotée ;
- non transférable par défaut ;
- liée à des preuves ;
- révisable ou contestable avant le snapshot Genesis.

Un crédit peut être produit par une politique `Proof-of-Useful-Work`, qui exige au minimum :

`mandat → workload → ressource observée → résultat utile → validation indépendante → attribution`

Le nombre de transactions, la durée de connexion, les appels artificiels et le volume de gossip ne constituent pas à eux seuls une contribution.

La politique de distribution Genesis est hors de ce Stage 2. Elle sera dérivée d’un snapshot de calibration et d’une règle publiée avant la clôture.

## 9. Consensus composable

Le premier consensus obligatoire est le consensus d’état : les peers doivent converger sur les transactions valides et l’état du ledger.

Les autres mécanismes sont des voies de preuve ou de sélection :

- PoH : ordre et temps ;
- PoW/PoUW : travail utile ;
- PoS : garantie engagée ;
- PoJ : évaluation indépendante ;
- PoSignal : qualité de l’observation ;
- PoCoverage : couverture de l’espace de travail ;
- PoOutcome : résultat post-action.

Chaque voie ne vote que sur sa propriété. Une preuve PoW ne peut pas seule établir une identité, une vérité, une compétence ou une autorité.

La composition finale doit définir :

- les entrées de chaque voie ;
- les poids ou seuils ;
- les conditions de quorum ;
- l’adversaire ;
- les coûts de manipulation ;
- la propriété garantie ;
- la propriété non garantie ;
- le comportement lorsque les voies sont en désaccord.

## 10. P2P et réplication

Le peer doit pouvoir fonctionner en mode :

1. `single_peer_lab` : un citoyen, un node, un ledger local ;
2. `multi_process_local` : plusieurs peers sur une machine ;
3. `private_multi_node` : plusieurs machines contrôlées ;
4. `public_devnet` : settlement expérimental Solana ;
5. `public_genesis` : token et gouvernance publics éventuels.

La fédération de données de `cynic-observer` alimente le peer, mais ne décide pas seule de la transition d’état. Les messages P2P doivent être validés, dédupliqués, ordonnés et liés à une identité de peer.

Le gossip ne vaut ni réception finale, ni transaction acceptée, ni preuve de règlement.

## 11. Miroir et jumeau

Le ledger ne doit pas être le miroir complet d’un humain.

- `PrivateMirror` : projection des faits autorisés pour le citoyen ;
- `PublicMirror` : projection minimisée et consentie ;
- `DigitalTwin` : modèle de simulation ;
- `AdversarialTwin` : modèle de stress et de contradiction.

Les twins consomment des preuves et peuvent produire des hypothèses. Ils ne modifient pas l’état canonique et ne reçoivent aucun pouvoir par leur capacité prédictive.

## 12. Critères de fermeture Stage 2

Le Stage 2 est fermé lorsque le premier peer peut prouver, sur un workload réel et reproductible :

- identité du citoyen, du node et de l’agent ;
- mandat et scope ;
- capacité observée avant allocation ;
- réservation avant exécution ;
- consommation après exécution ;
- résultat et validation indépendante ;
- crédit de calibration lié aux preuves ;
- replay déterministe ;
- idempotence ;
- reprise après arrêt ;
- rejet d’un double crédit ;
- distinction entre unknown, blocked, failed et completed ;
- absence d’augmentation d’autorité via un score.

Ensuite seulement : réplication multi-peer, consensus d’état, adversarial testing puis token Devnet.

## 13. Questions ouvertes

- Le ledger local est-il append-only par événement ou organisé immédiatement en blocs/epochs ?
- Le premier consensus d’état est-il leader-based, quorum-based ou dérivé d’une règle CYNIC nouvelle ?
- Le crédit CYD de calibration est-il uniquement un compteur ou une créance interne envers le trésor ?
- Une ressource doit-elle être mesurée par le node propriétaire, par un peer observateur indépendant, ou par les deux ?
- Quel mécanisme attribue équitablement la valeur entre citoyen, node, agent et preuve ?
- À quel moment une observation devient-elle une transaction admissible ?
- Quelle preuve minimale permet à un agent autonome sans citoyen humain parent de rejoindre le réseau ?

Aucune de ces questions ne doit être résolue par une constante tokenomics avant observation et simulation adversariale.