# CYNIC / ODYSSEUM — plan baseline multi-échelle

> Plan de préparation avant implémentation. Les termes et statuts sont
> provisoires jusqu'à observation, annotation humaine et preuve rejouable.

## 1. Cadre

Solana est ici une maison enflammée : un environnement de haute vitesse, de
forte créativité et d'externalités visibles. Les mathématiques ne sont pas une
religion ni une autorité morale ; elles sont un instrument de mesure, de
composition et de compréhension des géométries observées.

Le baseline doit donc répondre simultanément à trois questions :

```text
vertical : quelle capacité construisons-nous ?
horizontal : quelles contraintes traversent toutes les capacités ?
temporel/humain : qui observe, apprend, gouverne et corrige à quel moment ?
```

## 2. Couches verticales

### V0 — Autorité et inventaire

Objectif : savoir ce qui existe avant de le reconstruire.

À ingérer :

- `contracts/foundation.json` et `contracts/framework.json` ;
- `infra/runtime-authority.json`, `infra/registry.json` ;
- `cynic-observer/VISION.md` et `RESOURCE_MAP.md` ;
- documents identity et foundation externes ;
- surfaces actuelles et prototypes historiques d'ODYSSEUM/ASDF.

Sortie : fiches `resource_id`, propriétaire, statut, coût, dépendances,
preuves, inconnues et candidat de remplacement.

### V1 — Observation

Objectif : capter sans interpréter.

Premières sources :

- Solana RPC public ;
- Helius comme adapter enrichi ;
- warehouse Pump.fun ODYSSEUM ;
- métadonnées token ;
- capture X opt-in d'une page sélectionnée.

Sortie : événements bruts, timestamps, source, curseur et couverture.

### V2 — Preuve et provenance

Objectif : rendre chaque observation vérifiable et rejouable.

À réutiliser : `EvidenceEnvelope`, `EvidenceRef`, lineage et relations déjà
présents dans `cynic-protocol`.

Sortie : payload hashé, digest, provenance, dépendances, intégrité et unknowns.

### V3 — Normalisation et graphe

Objectif : relier mint, wallet, programme, pool, compte social, post, récit et
événement sans confondre observation et identité.

Sortie : `EntityRef`, événements normalisés, relations `observed`, `declared`,
`inferred`, `contested` ou `unknown`.

### V4 — Intelligence

Objectif : produire des interprétations contestables.

Premières analyses :

- OG selon fenêtre de couverture ;
- lineage et vamp/clone ;
- santé contextualisée ;
- anomalies ;
- désaccord entre sources et agents.

Sortie : facteurs, règles versionnées, preuves utilisées, hypothèses
concurrentes et abstention possible.

### V5 — Human-in-the-loop / Askesis

Objectif : faire de la curation humaine un composant du système, pas une
validation décorative.

Sortie : annotation, correction, contestation, décision de publication et
mesure du coût humain.

### V6 — Applications ODYSSEUM

Objectif : tester les primitives dans un contexte économique sans règlement
réel immédiat.

Premiers cas :

- profil de communauté ;
- trésorerie et événements ;
- proposal ;
- marché simulé ;
- domaine de risque isolé ;
- rapport de résolution.

### V7 — Règlement et fédération

Objectif : seulement après preuves suffisantes, publier des attestations
minimisées ou régler des états explicitement admis sur Solana.

La fédération arrive après le modèle mono-humain : calcul local, partage de
features/preuves et désaccords, jamais transfert obligatoire des vies numériques.

## 3. Plans horizontaux

Chaque couche verticale doit être évaluée à travers ces axes :

| Plan | Question baseline |
|---|---|
| Autorité | Qui a le droit de déclarer, corriger ou publier ? |
| Preuve | Qu'est-ce qui est observé, dérivé ou seulement raconté ? |
| Humanité | Quelle décision reste humaine, et comment est-elle contestée ? |
| Identité | Quel lien est prouvé entre compte, wallet, personne et rôle ? |
| Privacy | Quelle donnée reste locale, minimisée ou divulguée sélectivement ? |
| Sécurité | Quels secrets, permissions, attaques et surfaces de manipulation existent ? |
| Économie | Quel coût compute, RPC, stockage, attention et maintenance ? |
| Culture | Quel lore, symbole, langage ou conflit de valeurs est en jeu ? |
| Technique | Quelles dépendances, versions, formats, rate limits et fallbacks ? |
| Juridique | Quelle qualification ou obligation doit être vérifiée avant publication ? |
| Opérations | Quel health check, rollback, rétention et read-back prouvent la continuité ? |
| Contestation | Comment un nœud minoritaire ou un humain corrige le résultat ? |

### Ownership comme axe transversal obligatoire

L'ownership doit être observé comme une relation typée, pas comme un champ
unique `owner`. Une même ressource peut avoir un propriétaire légal, un
custodian technique, un décideur, un producteur de preuve et des bénéficiaires
différents.

| Dimension | Question | Exemple de sortie |
|---|---|---|
| Légal | Qui détient les droits ou obligations ? | entité, personne, contrat, unknown |
| Autorité | Qui peut décider ou modifier ? | humain, multisig, DAO, programme |
| Custody | Qui contrôle concrètement les clés, données ou serveurs ? | wallet, device, provider |
| Code | Qui possède, maintient et peut licencier le code ? | TALARIA, communauté, licence |
| Donnée | Qui a produit, collecté, conserve et peut supprimer la donnée ? | opérateur, source, nœud |
| Preuve | Qui atteste l'observation et qui peut la contester ? | observateur, quorum, humain |
| Modèle | Qui possède les poids, fixtures et règles d'inférence ? | CYNIC, communauté, provider |
| Publication | Qui autorise la projection publique ? | sujet, opérateur, gouvernance |
| Valeur | Qui reçoit les bénéfices et absorbe les coûts/risques ? | créateur, holders, treasury |
| Maintenance | Qui répond d'une panne, erreur ou dérive ? | maintainer, custodian, operator |

La fiche d'une ressource doit donc contenir :

```text
legal_owner
authority_holder
custodian
producer
maintainer
evidence_attestor
publication_authority
beneficiaries
liability_boundary
transfer_or_revocation_rule
```

Les confusions suivantes sont interdites :

```text
signature d'une transaction ≠ propriété économique du token
wallet financeur           ≠ identité certaine du créateur
compte GitHub              ≠ propriétaire légal du code
producteur d'une donnée    ≠ autorité pour la publier
créateur d'un modèle       ≠ propriétaire de la décision produite
custodian                  ≠ bénéficiaire
```

Le principe de souveraineté devient : aucune autorité ne doit être implicite.
Tout transfert d'ownership doit être explicite, borné, révocable lorsque
possible et lisible par les parties concernées.

## 4. Échelles temporelles

La même ressource doit être observée sur plusieurs horizons :

```text
T0 — instantané : état actuel et preuve immédiate
T1 — session : séquence d'actions et métadonnées locales
T2 — journée/semaine : rythme, anomalies et continuité
T3 — cycle : trajectoire d'un token, d'un récit ou d'une communauté
T4 — version : remplacement/amélioration d'un prototype
T5 — génération : effet de réseau, fédération et mutation culturelle
```

Une conclusion doit toujours indiquer son horizon. Un événement vrai à T0 peut
être faux comme description de T3. Une anomalie à T1 peut devenir un nouveau
motif à T3.

## 5. Échelles humaines

Le système ne doit pas traiter « l'humain » comme une variable unique :

```text
H0 — humain souverain : intention, consentement, gouvernance
H1 — opérateur-curateur : observation et annotation personnelle
H2 — équipe : désaccord, revue et distribution des responsabilités
H3 — communauté : règles, réputation, propositions et contestation
H4 — fédération : nœuds indépendants, attestations et conflits de sources
H5 — public : projection minimisée et lisible
```

Chaque passage d'une échelle à l'autre doit expliciter ce qui est perdu, ce qui
est agrégé et ce qui reste privé.

## 6. Baseline `node-0 / ASDFASDFA`

Le premier nœud doit produire, avant toute ambition fédérée :

1. inventaire des ressources économiques, humaines, techniques et culturelles ;
2. état runtime local et dépendances réellement disponibles ;
3. dossier Solana/Pump.fun du mint baseline ;
4. intégrité du warehouse et couverture temporelle ;
5. chronologie sociale opt-in minimale ;
6. graphe de relations avec statuts de preuve ;
7. premier dossier OG/vamp/health, non décisionnel ;
8. journal de curation et de désaccord humain ;
9. coût réel de production du dossier ;
10. liste des choses abandonnées, remplacées ou encore inconnues.

## 7. Gate avant code suivant

On ne passe à la couche suivante que si la précédente possède :

```text
owner
desired state
observer
port/adapter
proof schema
fixture ou cas réel
test de lecture/rejeu
unknowns
rollback ou retrait
```

Le code n'est donc pas le premier acte. Le premier acte est de rendre le champ
lisible : ressources, documents, autorités, humains, temps, coûts, preuves et
contradictions. Le code qui suit doit compresser cette compréhension, pas la
remplacer.
