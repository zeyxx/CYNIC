# CYNIC Observer — vision architecturale

> Document de conception évolutif — proposition non canonique.
>
> Ce document décrit une direction. Il ne remplace ni `contracts/`, ni
> `infra/runtime-authority.json`, ni les preuves produites par le runtime.
> Toute décision durable doit être traduite en types, tests, rapports et
> read-back observables.

## 1. Finalité

`cynic-observer` fournit la couche d'observation et de preuve pour comprendre
des phénomènes économiques et culturels publics autour de Solana : tokens,
wallets, programmes, liquidité, récits, communautés et événements sociaux.

Le premier cas d'usage est une IA conversationnelle capable d'expliquer et de
tracer les memecoins Solana. Elle doit pouvoir comparer plusieurs tokens,
identifier l'OG selon une fenêtre d'observation explicite, repérer des clones ou
« vamps », et présenter le profil le moins risqué selon une règle de scoring
versionnée.

L'IA n'est pas la source de vérité. Elle projette des observations, des
interprétations calculées et des inconnues clairement séparées.

## 2. Relation avec CYNIC et ODYSSEUM

```text
sources publiques
      ↓
CYNIC Observer : observer, conserver, prouver
      ↓
CYNIC Intel    : comparer, interpréter, scorer
      ↓
CYNIC Askesis  : expliquer, contextualiser, déclarer l'incertitude
      ↓
ODYSSEUM       : application économique consommant les primitives
      ↓
programme Solana / gouvernance / règlement explicite
```

CYNIC reste la membrane privée, le système d'observation et le cadre de
preuve. ODYSSEUM est une application économique qui peut consommer ces
primitives pour qualifier des communautés, examiner des proposals, projeter
des E-Scores ou régler des conséquences autorisées.

ODYSSEUM ne devient pas l'autorité du runtime CYNIC. Un score CYNIC ne déclenche
pas seul un paiement, une sanction ou une publication : une politique explicite,
une autorité compétente et une preuve de read-back sont nécessaires.

Les intégrations Pump.fun déjà présentes dans ODYSSEUM (transactions brutes,
archives compactées, market ledger et revenue ledger) constituent des sources
de faits et des candidats à l'adaptation vers les ports d'observation communs.
Elles ne doivent pas créer un second modèle de preuve incompatible.

## 3. Principes de convergence

### 3.1 Observation avant interprétation

```text
desired query → observe → normalize → retain evidence → analyze → explain
```

Un tweet, un nom de token ou un résultat d'agrégateur n'est jamais injecté
directement dans une décision. Les données externes sont non fiables et peuvent
contenir des instructions adversariales ou des identités trompeuses.

### 3.2 Fait, inférence, récit, inconnu

Chaque réponse doit distinguer au minimum :

- `fact` : observation directement reliée à une source ;
- `inference` : conclusion produite par une règle versionnée ;
- `narrative` : interprétation culturelle ou historique ;
- `unknown` : couverture manquante, conflit ou preuve insuffisante.

« Ce compte est le deployer » exige une preuve on-chain. « Ce compte a été le
premier à employer ce narratif dans notre fenêtre X » est une observation
sociale. Ces deux propositions ne doivent pas être fusionnées.

### 3.3 La couverture fait partie du résultat

Toute conclusion doit publier sa fenêtre temporelle, son cluster, ses sources,
sa fraîcheur, ses limites d'indexation et les événements éventuellement
manquants. L'OG est toujours l'« earliest observed » selon une couverture
déclarée, jamais une affirmation absolue par défaut.

### 3.4 Jugement contestable

Les scores sont des instruments de comparaison, pas des mesures universelles de
la valeur humaine ou de la moralité d'une communauté. Ils doivent être :

- versionnés ;
- décomposables par facteur ;
- accompagnés des preuves utilisées ;
- recalculables ;
- contestables et corrigeables ;
- incapables d'amplifier silencieusement leur propre autorité.

### 3.5 Minimisation sociale

Les données Twitter/X, Telegram, Discord et autres plateformes peuvent exposer
des personnes, des pseudonymes ou des communautés. Le système collecte le
minimum nécessaire, sépare identité déclarée et identité prouvée, respecte les
conditions d'utilisation des fournisseurs et évite le doxxing, le harcèlement,
la surveillance privée et les listes noires automatiques.

## 4. Sources et rôle de chacune

### Preuve primaire

- Solana RPC et fournisseurs d'archive : signatures, slots, comptes,
  instructions, logs et balances ;
- Pump.fun / PumpSwap : création, bonding curve, migration, pools et frais ;
- Raydium, Meteora, Orca, Jupiter et autres programmes observés directement ;
- Metaplex, IPFS et Arweave : métadonnées et lineage des assets.

### Contexte social et culturel

- X/Twitter : chronologie du narratif, annonces, reprises, citations et
  relations entre comptes ;
- Telegram et Discord : communautés, propositions, coordination publique et
  activité déclarée ;
- GitHub, sites, domaines, DNS et archives web : infrastructure, code,
  antériorité et cohérence des déclarations.

### Signaux secondaires

- DexScreener, Birdeye et autres indexeurs de marché ;
- RugCheck, Bubblemaps ou services similaires ;
- médias, Reddit, YouTube et autres surfaces de propagation.

Les signaux secondaires enrichissent ou contredisent le dossier ; ils ne
remplacent pas une preuve primaire.

## 5. Responsabilités des composants

### `cynic-observer`

Responsable de l'observation read-only, de l'acquisition, de la normalisation,
de la déduplication, de la conservation des payloads et de la déclaration de
couverture.

Structure cible :

```text
src/
  ports/          # interfaces d'observation et de stockage
  domain/         # événements, preuves, curseurs, couverture
  adapters/       # Solana, Helius, Pump, X, Telegram, Discord, web
  normalization/  # événements source → événements communs
  evidence/       # hash, enveloppe, index, rétention
  federation/     # transport optionnel, jamais autorité implicite
```

Les adaptateurs sont remplaçables. Les ports, les schémas et les preuves sont
durables.

### `cynic-intel`

Responsable des règles pures d'interprétation : provenance, graphe de wallets,
similarité de tokens, détection de coordination, risques, OG score, vamp score
et health score.

Il ne doit pas appeler directement X, Solana ou un shell. Il reçoit des vues
normalisées et retourne des décisions accompagnées de preuves et d'inconnues.

### `cynic-askesis`

Responsable de la projection conversationnelle et de la synthèse RAG. Il ne
possède ni les données brutes ni les règles de scoring. Chaque réponse doit
conserver les références d'évidence et pouvoir être reformulée sans les perdre.

### `cynic-protocol`

Responsable des contrats partagés : `ObservationEnvelope`, `EvidenceRef`,
`EntityRef`, `NormalizedEvent`, `Coverage`, `Unknown`, versions de règles et
liaison entre claim et preuve.

### `cynic-kernel`, CLI et MCP

Responsables de l'orchestration et de la projection. Ils ne recalculent pas les
scores et ne deviennent pas une seconde autorité de données.

### ODYSSEUM

Consomme des rapports ou attestations explicitement admis. Il peut utiliser les
preuves pour ses proposals, ses communautés, ses marchés ou ses programmes,
mais doit conserver la provenance et la version du jugement reçu.

## 6. Modèle minimal de preuve

Chaque observation doit pouvoir être réduite à une enveloppe comparable :

```json
{
  "schema": "cynic.observation_envelope.v1",
  "source": "solana|twitter|telegram|pump_fun|metadata",
  "entity_refs": ["mint:...", "wallet:..."],
  "observed_at": "...",
  "published_at": "...",
  "source_id": "signature-or-post-id",
  "raw_payload_hash": "blake3:...",
  "normalizer": "adapter-name@version",
  "coverage": {
    "from": "...",
    "to": "...",
    "complete": false
  },
  "evidence_tier": "T1|T2|T3|T4",
  "unknowns": []
}
```

Les données brutes, les événements normalisés et les interprétations sont trois
objets différents. Une interprétation ne doit pas écraser la preuve originale.

## 7. Les trois premières analyses

### OG

Comparer : première apparition du token, première liquidité, première
apparition du narratif, continuité, originalité des métadonnées et éventuels
liens entre présence sociale et activité on-chain.

Sortie : score d'antériorité, facteurs, fenêtre d'observation et inconnues.

### Vamp / clone

Comparer : texte, image, ticker, timing, wallets de financement, deployer,
programmes utilisés, métadonnées et trajectoire de liquidité.

Sortie : similarité et éléments communs observés ; jamais accusation automatique
de fraude ou attribution certaine d'une identité.

### Health

Évaluer séparément : autorités, distribution, liquidité, comportements de
trading, provenance, transparence et qualité de couverture.

Sortie : profil de risque contextualisé. « Le moins risqué dans cet échantillon »
est acceptable ; « safe » ou « sain au sens absolu » ne l'est pas.

## 8. Double boucle d'observation et d'apprentissage

Le système doit compenser un biais naturel de conception. Une approche
bottom-up stabilise les primitives à partir d'observations choisies et
révisables. Une approche top-down cherche des structures émergentes dans un
espace plus large. Elles sont complémentaires, mais leurs données et leurs
niveaux de confiance doivent rester distingués.

### 8.1 Boucle bottom-up — précision et curation

```text
utilisateur choisit → capture opt-in → preuve locale → curation humaine
       → primitive stable → fixture → règle → modèle évalué
```

Cette boucle commence par un périmètre réduit : un mint, quelques wallets,
quelques comptes publics et une chronologie connue. Elle permet de :

- définir ce qu'est un événement utile ;
- distinguer une preuve d'une interprétation ;
- corriger les erreurs d'identité ou de déduplication ;
- construire des fixtures historiques annotées ;
- entraîner et évaluer les modèles sur des cas compréhensibles ;
- conserver l'opérateur comme arbitre humain pendant la phase d'apprentissage.

La capture personnelle reste opt-in, locale par défaut et réversible. Une
annotation humaine ne devient pas automatiquement une vérité : elle est une
attestation ou une hypothèse identifiée comme telle.

### 8.2 Boucle top-down — découverte et émergence

```text
requête / univers borné → collecte large → signaux faibles → clusters
       → hypothèse → retour vers curation ciblée
```

Cette boucle peut explorer un univers de tokens, de posts ou de wallets pour
faire apparaître des familles, des reprises de narratif, des comportements
coordonnés ou des anomalies. Elle doit rester bornée par :

- une fenêtre temporelle ;
- un univers de sources déclaré ;
- un budget de collecte ;
- une politique de rétention ;
- un niveau de précision attendu ;
- une séparation stricte entre signal de découverte et preuve publiée.

L'ingestion large ne signifie donc pas « tout conserver et tout savoir ». Elle
produit des candidats, que la boucle bottom-up revient examiner avec des
preuves plus précises et une validation humaine.

### 8.3 Boucle de compensation

```text
bottom-up : les primitives deviennent exactes
top-down   : les structures inattendues deviennent visibles
human      : les hypothèses sont sélectionnées et contestées
ML         : les tâches répétitives sont accélérées après validation
```

Le ML ne doit pas définir seul les catégories d'OG, de vamp ou de santé. Il
peut proposer des clusters, extraire des entités, retrouver des analogies et
prioriser les dossiers. La règle, la preuve et la possibilité d'abstention
restent inspectables.

## 9. Thèse de contexte : deux économies crypto

La distinction entre `efficient finance` et `alternate finance` peut servir de
cadre de recherche, pas de prédiction garantie de marché.

`efficient finance` regroupe les améliorations de systèmes existants :
tokenisation, stablecoins, accès au marché, néobanques et infrastructure
financière.

`alternate finance` regroupe les formes nouvelles de coordination et de
formation de valeur : monnaies internet, collectibles, perps, capital
formation native internet, futarchy, DAOs, DePIN et memecoins.

CYNIC Observer doit pouvoir observer les deux sans privilégier par avance une
thèse de marché. ODYSSEUM peut ensuite sélectionner une application ou une
communauté, mais les données doivent permettre de falsifier le récit initial.

L'hypothèse opérationnelle est que les cycles de créativité crypto peuvent
réarranger des primitives existantes, surtout lorsque le coût d'expérimentation
diminue avec l'IA. Cela justifie un moteur capable de comparer les primitives,
les communautés et leurs trajectoires ; cela ne constitue pas une garantie de
retour du marché ni une recommandation d'allocation.

### 9.1 Thèse directrice

La thèse de travail est que le prochain cycle crypto sera davantage
`futarchique`, `privacy by default` et orienté vers l'« one-person enterprise at
planetary scale ».

Cette phrase décrit une direction de conception, pas un fait déjà démontré :

```text
une personne souveraine
  + agents et modèles portables
  + distribution mondiale
  + capital programmable
  + règlement Solana
  + privacy locale et divulgation sélective
  + marchés qui estiment les conséquences
  = entreprise individuelle à échelle planétaire
```

Dans cette lecture, les memecoins ont constitué un test à haute vitesse des
primitives suivantes :

- lancement sans permission ;
- formation de capital native internet ;
- coordination communautaire ;
- distribution par le récit ;
- liquidité programmable ;
- réputation et attribution dans un environnement adversarial ;
- nécessité de distinguer signal, preuve et manipulation.

Ils ne sont pas nécessairement le produit final. Ils ont servi de laboratoire
où les limites de la provenance, de la distribution, de la gouvernance et de la
protection des opérateurs sont devenues visibles.

### 9.2 Conséquences d'architecture

Cette thèse impose quatre exigences :

1. **Futarchie explicable** : un marché peut estimer une conséquence, mais ne
   fabrique ni le fait observé ni l'autorité de résolution. Les questions,
   règles, participants, preuves et résolutions doivent rester inspectables.
2. **Privacy par défaut** : l'observation est locale et minimisée ; la
   publication passe par consentement, réduction de données, attestations et
   divulgation sélective. Le stockage brut ne devient pas automatiquement un
   produit public.
3. **Échelle sans dépossession** : les agents, modèles et protocoles amplifient
   l'opérateur mais ne deviennent pas le propriétaire silencieux de son
   identité, de sa mémoire, de son capital ou de ses décisions.
4. **Solana comme règlement, non comme omniscience** : Solana peut engager et
   régler explicitement une décision ; CYNIC doit encore observer les faits
   sociaux, économiques et humains qui fondent cette décision.

Le rôle de CYNIC Observer est donc de rendre cette économie émergente lisible
et contestable. Le rôle d'ODYSSEUM est de transformer certaines primitives
validées en applications économiques. Le rôle de l'humain reste de choisir les
objectifs, les limites, les destinataires et les conséquences acceptables.

La privacy doit être traitée comme une hypothèse de conception importante :
surveillance institutionnelle, collecte sociale et modèles opaques augmentent
la nécessité d'une architecture locale, minimisée et contrôlable. Elle ne doit
pas devenir un prétexte pour collecter davantage de données privées.

## 10. Feuille de route de convergence

1. Stabiliser les contrats d'observation et de preuve dans `cynic-protocol`.
2. Adapter le warehouse Pump.fun d'ODYSSEUM à ces contrats, en lecture seule.
3. Produire un rapport Solana mono-mint rejouable.
4. Ajouter métadonnées et première chronologie X avec couverture explicite.
5. Implémenter OG, Vamp et Health sur fixtures synthétiques et cas historiques.
6. Ajouter Telegram/Discord comme contexte social séparé de la preuve on-chain.
7. Construire le RAG Askesis avec citations obligatoires et abstention.
8. Définir le contrat de consommation ODYSSEUM : rapport, proposal,
   contestation et read-back, avant toute écriture on-chain.

## 11. Questions à résoudre avant activation publique

- Quelle couverture X/Telegram est légalement et techniquement durable ?
- Quelle politique de rétention s'applique aux contenus sociaux bruts ?
- Qui peut contester ou corriger un jugement ?
- Quelle preuve cryptographique ODYSSEUM accepte-t-il ?
- Quelle latence est acceptable entre observation, score et publication ?
- Comment détecter un conflit entre Helius, RPC public et archive locale ?
- Comment empêcher qu'un score de santé devienne une recommandation financière
  déguisée ?
- Quels signaux peuvent être publiés sans exposer des personnes ou des petits
  groupes ?

La convergence est atteinte lorsque la même chaîne de provenance peut être
rejouée depuis une source, inspectée par un humain, expliquée par Askesis,
contestée par un opérateur et consommée par ODYSSEUM sans que le récit ou le
score ne remplace la preuve inférieure.

## 12. Lignée documentaire et autorités

Cette vision doit être reliée aux documents fondateurs, mais elle ne doit pas
dupliquer leurs décisions ni transformer une projection narrative en loi
runtime.

### Autorité d'exécution

Pour une question de comportement réel, la hiérarchie reste :

```text
observation et read-back
  → contracts/foundation.json
  → infra/runtime-authority.json
  → infra/registry.json
  → Cargo.toml
  → crates Rust canoniques
```

Les schémas d'observation, de preuve, de consentement, d'attestation et de
publication doivent être promus dans `cynic-protocol` lorsqu'ils deviennent
réels et testés. Tant qu'ils restent dans cette vision, ils sont des propositions.

### Documents de conception CYNIC

La présente vision doit se relier à :

- `AGENTS.md` pour les invariants de développement et de preuve ;
- `contracts/foundation.json` et `contracts/framework.json` pour les capacités
  et leurs propriétaires ;
- `WHITEPAPER.md` et `MANIFESTO.md` pour les projections publiques, sans les
  utiliser comme preuve runtime ;
- `crates/cynic-intel` pour les règles d'interprétation et de score ;
- `crates/cynic-askesis` pour la projection humaine et conversationnelle.

Le lien correct est une référence de provenance : chaque claim important doit
indiquer le contrat, la règle ou l'observation qui le fonde. Il ne faut pas
maintenir plusieurs versions éditoriales d'une même capacité.

### Documents ODYSSEUM et TALARIA

Les documents d'ODYSSEUM décrivent l'application économique, ses communautés,
ses mécanismes et ses programmes Solana. Les documents de TALARIA décrivent la
genèse légale, l'édition du logiciel et les responsabilités institutionnelles.
Ils doivent être reliés par des références croisées et des statuts explicites :

```text
proven        = démontré par contrat, code, test ou read-back
proposed      = direction de conception
projected     = récit ou interface publique
blocked       = idée nécessitant encore une autorité ou une preuve
```

Un document ODYSSEUM ne peut donc pas déclarer qu'une capacité CYNIC existe si
le runtime CYNIC ne peut pas en produire la preuve. Réciproquement, CYNIC ne
doit pas imposer à ODYSSEUM une mécanique économique qui n'est pas admise par
les contrats et la gouvernance de l'application.

### Humanité, jeu et ZK

La curation humaine reste l'autorité ultime de la boucle : l'humain choisit
l'objectif, accepte ou rejette une annotation, conteste un jugement et décide
de la publication ou du règlement.

Le Proof-of-Humanity par le jeu peut fournir un signal de participation,
d'attention, de continuité ou de non-automatisation. Tant qu'il n'est pas
formalisé et testé, il ne doit pas être appelé preuve cryptographique d'humanité
ou d'unicité.

Le ZK peut ensuite permettre de prouver sélectivement un prédicat sans révéler
la donnée brute : âge d'une attestation, appartenance à un ensemble, respect
d'une règle, possession d'un droit ou unicité selon un protocole donné. Le ZK
ne prouve pas à lui seul que le sujet est humain : il faut encore définir la
source d'identité, l'unicité, le consentement, la révocation et la résistance
aux fermes de comptes.

Ces deux axes doivent donc apparaître dans les documents fondateurs comme des
directions à prouver, puis être reliés à des contrats et des fixtures avant de
servir de garde-fou dans ODYSSEUM.

## 13. Références fondatrices à relier

Les deux corpus locaux fournis par l'opérateur sont les sources de conception
les plus pertinentes pour cette vision. Ils doivent être reliés par identifiants
et références croisées, sans être importés automatiquement comme autorité
runtime.

### Corpus architecture

```text
foundation/v2-architecture-canon/
  CYNIC_PROOF_OF_HUMANITY_FILTER.md
  CYNIC_ESCORE_SPECIFICATION.md
  CYNIC_ESCORE_INDUSTRIAL_ROADMAP.md
  ODYSSEUM_ARCHITECTURE.md
  ODYSSEUM_PRIMITIVES_AND_CULTURE.md
  CYNIC_TALARIA_CORPORATE_DOCTRINE.md
  CYNIC_LEGAL_AND_FUTARCHIC_LAUNCH.md
  escore-registry.schema.json
```

Ces documents alimentent respectivement :

- les primitives d'identité, de contribution et de jugement ;
- les mécanismes de preuve et d'E-Score ;
- la relation Observer → Intel → ODYSSEUM ;
- les registres culturels et leurs règles de gouvernance ;
- les frontières produit, légales et institutionnelles.

### Corpus identité

```text
identity/
  CYNIC-CONSTITUTION-FULL.md
  CYNIC-IDENTITY-GRAPH.md
  CYNIC-SOVEREIGNTY-VISION.md
  CYNIC-PRIMARY-SOURCES.md
  CYNIC-PHI-CONVERGENCE.md
  AXIOM-VIII-FRACTAL.md
  KYON-HUMAN-MODEL-PERENNIAL-MAP.md
```

Ce corpus fournit le sens humain et philosophique : l'humain comme souverain,
le kernel comme persistance, les agents comme raisonnement borné, la
fractalisation des capteurs et la limite de toute prétention à la certitude.

### Corrections à appliquer avant promotion

La vision actuelle corrige déjà plusieurs formulations trop fortes du corpus
fondateur. Avant de les relier à un contrat ou à une publication, les points
suivants doivent être reformulés :

- un jeu produit un signal comportemental probabiliste, pas une preuve
  inforgeable d'humanité ;
- une preuve ZK prouve uniquement les propriétés inscrites dans son circuit,
  pas l'humanité ou l'unicité par magie ;
- une capture locale opt-in remplace toute vision de type interception globale,
  MITM ou collecte invisible ;
- l'observation de groupes privés nécessite consentement, périmètre, rétention
  et révocation explicites ;
- un score comportemental ne doit pas prétendre lire « l'âme numérique » ni
  déduire seul une identité réelle ;
- les formulations de séparation légale ou d'immunité institutionnelle sont
  des hypothèses soumises à conseil juridique, jamais des garanties techniques ;
- les preuves formelles d'un programme financier ne prouvent pas la validité
  des données sociales, des hypothèses de marché ou des décisions humaines.

Le travail de convergence consiste donc à produire une table machine-readable
de claims fondateurs avec `claim_id`, `source_ref`, `status`, `owner`,
`required_proof` et `contradictions`. Tant qu'une telle table n'existe pas, les
documents restent une constellation de doctrine et de recherche, pas une
seconde hiérarchie d'autorité.

## 14. Principe : rendre le mensonge visible et coûteux

La finalité de l'observation multi-source n'est pas de prétendre détecter
parfaitement le mensonge. Elle est de rendre la fabrication et le maintien d'un
faux récit progressivement plus visibles et plus coûteux.

```text
claim
  → traces attendues
  → observations indépendantes
  → cohérences et contradictions
  → coût de maintien du récit
  → jugement contestable
```

Un acteur peut simuler une part d'aléa, une partie de poker ou un comportement
social. Il devient plus difficile de simuler durablement, sur plusieurs
contextes, une trajectoire cohérente d'apprentissage, d'incertitude, d'erreurs,
de décisions et de relations on-chain.

Le système doit donc mesurer notamment :

- la stabilité ou l'évolution temporelle d'une identité ;
- les révisions après erreur et les changements de stratégie ;
- la cohérence entre activité sociale, activité on-chain et contributions ;
- les coûts de coordination entre comptes ou wallets ;
- les contradictions entre déclarations, métadonnées et règlements ;
- la quantité d'effort nécessaire pour maintenir une identité artificielle.

Ce principe ne justifie ni la surveillance totale, ni la punition automatique,
ni le diagnostic psychologique. Une contradiction est un signal à examiner ;
elle devient une décision seulement après preuve suffisante, règle explicite,
possibilité de réponse humaine et conservation de la contestation.

## 15. Le désaccord comme signal

Le désaccord entre nœuds, agents ou observateurs — humain, LLM, programme,
source sociale ou source on-chain — n'est pas nécessairement un défaut à
résoudre. C'est souvent le signal le plus utile : il révèle une limite de
couverture, une ambiguïté de modèle, une divergence de valeurs ou une tentative
de manipulation.

```text
observations concurrentes
        ↓
classification du désaccord
        ↓
preuves et hypothèses séparées
        ↓
arbitrage humain ou règle explicite
        ↓
jugement + minorité conservée
```

Les désaccords doivent être typés :

- `data_conflict` : deux sources décrivent des états différents ;
- `coverage_conflict` : les sources n'observent pas la même période ou le même
  univers ;
- `interpretation_conflict` : les mêmes faits produisent des inférences
  différentes ;
- `value_conflict` : le résultat dépend d'une définition de « sain », « OG » ou
  « contribution » ;
- `authority_conflict` : deux règles ou propriétaires prétendent décider ;
- `adversarial_signal` : une source semble chercher à tromper, polluer ou
  manipuler l'observation.

Un consensus numérique ne doit pas effacer une minorité sans conserver sa
position, ses preuves, son niveau de confiance et la règle qui l'a écartée.
L'humain garde l'autorité ultime de gouvernance, mais son désaccord avec le
système doit lui aussi rester visible et révisable.

Pour les modèles, le meilleur comportement n'est pas toujours de choisir une
réponse : lorsque la divergence dépasse un seuil, Askesis doit présenter les
hypothèses concurrentes, demander une observation supplémentaire ou s'abstenir.
La diversité des nœuds augmente la qualité seulement si leurs sources,
modèles, incitations et angles morts restent inspectables.

## 16. Émergence, conscience et anomalies

L'observation multi-échelle peut faire apparaître des propriétés qui ne sont
présentes dans aucune trace isolée : continuité d'un sujet, adaptation à la
nouveauté, auto-correction, transfert d'apprentissage, arbitrage entre buts,
réflexion sur ses propres décisions et production d'un style singulier.

Ces propriétés sont des signaux compatibles avec une agence ou une forme de
métacognition. Elles ne constituent pas, à elles seules, une preuve de
conscience. Un agent artificiel peut les simuler ; un humain peut les manifester
de manière irrégulière ou ne pas disposer des mêmes capacités selon le contexte.

```text
trace locale
  → motif temporel
  → cohérence cross-contextes
  → propriété émergente candidate
  → interprétation prudente
  → validation humaine / contradiction
```

Le système doit distinguer les anomalies par rapport à plusieurs références :

- `statistical_anomaly` : fréquence, durée ou distribution inhabituelle ;
- `temporal_anomaly` : rupture de rythme ou changement de régime ;
- `behavioral_anomaly` : décision incompatible avec la trajectoire précédente ;
- `semantic_anomaly` : changement de récit, de vocabulaire ou d'intention
  déclarée ;
- `relational_anomaly` : nouveau lien entre comptes, wallets ou communautés ;
- `provenance_anomaly` : contradiction entre source, auteur déclaré et preuve ;
- `coverage_anomaly` : absence ou apparition d'une donnée explicable par la
  collecte plutôt que par le phénomène observé.

Une anomalie n'est pas une fraude, une maladie, un mensonge ou une preuve de
conscience. C'est un écart qui demande une observation supplémentaire. Les
baselines doivent être personnelles et contextuelles lorsque le consentement
le permet ; une moyenne globale ne doit pas devenir la définition implicite du
comportement normal.

La couche `cynic-intel` peut classer et prioriser ces écarts. Askesis doit les
présenter comme des hypothèses concurrentes, sans profiler psychologiquement un
individu ni publier une inférence sensible sans consentement explicite.

## 17. Géométrie des modes de réflexion

Les modes de réflexion ne forment pas une checklist linéaire ni une taxonomie
fermée. Ils sont des primitives composables. Une même question peut traverser
plusieurs modes dans un ordre particulier, avec des poids et des tensions
différents.

```text
question
  → observation
  → causalité + graphe
  → Bayes
  → contrefactuel
  → adversarial
  → métacognition
```

Une autre géométrie pourrait être :

```text
narratif
  → phénoménologie
  → herméneutique
  → valeurs
  → décision
  → read-back
```

Ces compositions peuvent produire des raisonnements qualitativement différents
à partir des mêmes observations. Il faudra pouvoir observer :

- la séquence des modes ;
- leurs poids relatifs ;
- les transitions et boucles ;
- les désaccords entre géométries ;
- les propriétés émergentes de la composition ;
- la qualité du raisonnement après read-back.

Le système ne doit pas imposer trop tôt une « géométrie correcte » de la pensée.
Il peut enregistrer des profils versionnés, comparer leurs résultats et laisser
la curation humaine décider quelles combinaisons deviennent des primitives,
des protocoles d'analyse ou des modèles ML.

## 18. Économie : tendre vers un coût marginal proche de zéro

L'objectif économique n'est pas de supprimer magiquement tous les coûts. Il est
de réduire le coût marginal de l'observation et de l'analyse après une première
phase de calibration.

```text
coûts fixes
  infrastructure minimale + développement + sécurité + curation initiale

coût marginal cible
  capture locale + données publiques + calcul réutilisable
  + preuves compactes + modèles déjà calibrés
```

Les « hacks » doivent être compris comme des leviers légitimes : réutiliser des
données déjà observées avec consentement, choisir des sources publiques et
interopérables, exécuter le calcul localement, compacter les preuves, partager
des primitives plutôt que des payloads bruts et laisser les fournisseurs rester
interchangeables. Aucune optimisation ne doit contourner une autorisation,
violer les conditions d'une plateforme, exposer des secrets ou externaliser un
risque juridique vers les utilisateurs.

### Phase 1 — un humain, une boucle complète

Le premier nœud est l'opérateur lui-même. Il sert à calibrer :

- les primitives d'observation ;
- les limites de collecte ;
- les annotations ;
- les profils de raisonnement ;
- les erreurs d'identité et de provenance ;
- le rapport entre coût de calcul, qualité et utilité.

Cette phase doit produire un modèle fonctionnel à petite échelle, pas une
illusion de couverture planétaire. Les données personnelles restent locales,
les exports sont choisis, et l'opérateur peut supprimer ou corriger ses traces.

### Phase 2 — fédération

Une fois le modèle compris à l'échelle d'un humain, la fédération peut ajouter
des nœuds sans centraliser toute leur vie numérique :

```text
nœud local
  → observation et extraction locales
  → features / preuves minimisées
  → agrégation fédérée
  → modèle ou registre partagé
  → vérification et contestation
```

La fédération peut partager :

- des événements publics normalisés ;
- des hashes et attestations ;
- des statistiques agrégées ;
- des mises à jour de modèles ;
- des exemples anonymisés ou consentis ;
- des désaccords et anomalies.

Elle ne doit pas exiger par défaut le partage des contenus bruts, des messages
privés, des identifiants réels ou des journaux complets d'appareil.

Le gain recherché est un effet de réseau : chaque nœud paie une partie de la
collecte qui lui est utile, tandis que les primitives, détecteurs et preuves
réutilisables augmentent la valeur de tous les nœuds. La fédération ne devient
une autorité que pour les claims qu'elle peut attribuer, vérifier et contester.

## 19. Convergence produit : creator capital markets sur Solana

La direction produit finale est de transformer des memecoins et communautés
éphémères en domaines économiques structurés : entreprises de créateurs,
communautés capitalisées et marchés de conséquences réglés sur Solana.

```text
ASDFASDFA → cas baseline et laboratoire culturel
CYNIC     → observation, provenance, identité, risque, jugement contestable
ODYSSEUM  → creator capital markets, gouvernance et marchés
TALARIA   → entité mère, développement et droits/licences à documenter
Solana    → engagement, règlement et read-back on-chain
```

### 19.1 Du memecoin au domaine économique

```text
créateur / communauté
  → profil et provenance observables
  → trésorerie et règles publiques
  → domaine économique isolé
  → marchés de participation ou de conséquences
  → règlement et comptabilité vérifiables
```

Les termes « stocks on Solana » et « options on Solana » décrivent ici des
fonctions de marché souhaitées. Ils ne préjugent pas de la qualification
juridique des instruments, des restrictions géographiques ou des obligations
de protection des investisseurs. Toute activation publique exige une revue
juridique compétente.

### 19.2 Structurer l'anarchie

La proposition ne supprime pas les primitives libérales ; elle cherche à les
rendre composables et vérifiables :

```text
liberté de créer
  + association volontaire
  + propriété et custody explicites
  + sortie et contestation
  + règles publiques
  + risque isolé
  + règlement vérifiable
```

« Structurer l'anarchie » signifie remplacer le chaos implicite par des bornes
explicites, sans remplacer l'initiative par une autorité centrale opaque. La
« fusion du communisme et du capitalisme » peut rester une métaphore culturelle ;
techniquement, il s'agit d'un assemblage entre commons partagés, coordination
de marché et souveraineté individuelle.

### 19.3 Trust minimisé, pas confiance supprimée

Les mathématiques peuvent réduire la confiance nécessaire dans les calculs de
règlement, les règles de marge, les signatures, les transitions d'état et la
reproductibilité des rapports. Elles ne suppriment pas la confiance liée aux
oracles, développeurs, interfaces, fournisseurs de données, gouvernance,
qualifications juridiques et décisions humaines.

Le produit doit donc publier une matrice de confiance résiduelle, plutôt que
promettre un système entièrement trustless.

### 19.4 Séquençage

1. prouver ASDFASDFA comme dossier forensique et baseline culturelle ;
2. produire un profil de communauté et de trésorerie avec provenance ;
3. tester des mécanismes de gouvernance et de marché simulés ;
4. isoler mathématiquement les domaines de risque ;
5. définir oracles, résolutions, liquidations, sorties et contestations ;
6. faire valider la structure juridique et les obligations applicables ;
7. seulement ensuite envisager des instruments publics réels.

ODYSSEUM ne devient pas une bourse par simple extension de prototypes. CYNIC
rend d'abord les faits, règles et risques lisibles ; TALARIA maintient la
séparation entre développement logiciel et activité économique effectivement
opérée ; Solana ne règle que les états explicitement admis.
