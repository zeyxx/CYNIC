# CYNIC E-Score

Ce document definit le **E-Score** comme modele unique de reputation, citoyennete reseau, merite operateur, anti-Sybil, priorite protocolaire et contribution ecosysteme.

Il remplace toute doctrine separee de "citoyennete CYNIC". La citoyennete n'est pas un score distinct : c'est la lecture sociale et protocolaire du E-Score.

Documents relies :

- `docs/foundation/TALARIA_CYNIC_NAMING.md`
- `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md`
- `docs/CYNIC_ECOSYSTEM_ARCHITECTURE.md`
- `docs/foundation/CYNIC_OWNERSHIP_CONSTITUTION.md`
- `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md`
- `docs/foundation/CYNIC_LEGAL_GENESIS_PLAN.md`

---

## 1. Loi Centrale

```text
E-Score is earned by verified contribution.
It is not bought by token balance.
It is not granted by proximity.
It is not inherited from legal equity.
It is judged from primary evidence.
```

Le E-Score mesure la contribution d'un acteur au reseau CYNIC.

Il est calcule par le Kernel, mais il doit etre ancre dans les sources de verite primaires decrites dans `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md`.

Il ne mesure pas :

- la valeur humaine ;
- la moralite totale ;
- la citoyennete etatique ;
- les droits d'associe ;
- le droit aux dividendes ;
- la propriete de TALARIA SAS.

Il repond a une question precise :

```text
Cet acteur rend-il le reseau plus intelligent, plus utile, plus robuste et plus durable ?
```

---

## 2. Origine GASdf / HolDex

Le vieux modele GASdf/HolDex contenait deja l'intuition :

```text
La loyauté ecosysteme ne se mesure pas seulement par ce qu'un acteur hold.
Elle se mesure par ce qu'il apporte, utilise, construit, securise et maintient.
```

Dimensions historiques :

| Dimension | Lecture initiale |
|-----------|------------------|
| Hold | Stabilite et duree de detention |
| Burn | Contribution a la reduction de supply |
| Use | Usage reel des produits |
| Build | Contributions techniques ou integrations |
| Node | Ressources reseau, compute, RPC, hosting |
| Refer | Apport d'utilisateurs ou partenaires utiles |
| Duration | Anciennete active et non extractive |

CYNIC conserve ces dimensions, mais les etend avec le Kernel, le ROI reel, la qualite du travail et le `/judge`.

---

## 3. Dimensions Du E-Score

### 3.1 Hold

Mesure la stabilite d'exposition a l'ecosysteme.

Preuves possibles :

- anciennete wallet ;
- duree de detention ;
- absence de dump opportuniste ;
- participation pendant les phases difficiles ;
- lock volontaire ;
- coherence entre position et contribution.

Limite :

```text
Hold is signal, not citizenship by itself.
```

### 3.2 Burn

Mesure la contribution a la reduction de supply ou aux mecanismes de sacrifice productif.

Preuves possibles :

- burns on-chain ;
- burn-to-mint ;
- participation a des conversions publiques ;
- reduction d'extraction court-terme ;
- preuve que le burn respecte les regles publiees.

### 3.3 Use

Mesure l'utilisation reelle des branches produit.

Preuves possibles :

- appels API ;
- transactions ;
- sessions produit ;
- usage B&C / PoH ;
- usage GASdf ;
- usage CultScreener ;
- usage Talaria ;
- retention ;
- volume utile non wash.

### 3.4 Build

Mesure le travail livre.

Preuves possibles :

- commits ;
- audits ;
- designs ;
- integrations ;
- SDKs ;
- documentation exploitable ;
- incidents resolus ;
- tests ;
- prototypes transformes en production.

Regle :

```text
No primary receipt, no durable credit.
```

### 3.5 Node

Mesure la fourniture de ressources reseau.

Preuves possibles :

- compute ;
- GPU ;
- RPC ;
- hosting ;
- storage ;
- uptime ;
- latency ;
- inference servie ;
- monitoring ;
- securite operationnelle.

### 3.6 Refer

Mesure l'apport d'utilisateurs, builders, clients ou partenaires utiles.

Preuves possibles :

- utilisateurs actifs ;
- integrations signees ;
- clients B2B/B2G ;
- builders onboardes ;
- activite recurrente ;
- qualite des referrals ;
- faible taux de Sybil/farm.

### 3.7 Duration

Mesure la consistance temporelle.

Preuves possibles :

- regularite ;
- fiabilite ;
- maintenance apres lancement ;
- presence dans les periodes difficiles ;
- correction d'erreurs ;
- absence de comportement extractif ;
- coherence entre paroles, actions et resultats.

Regle :

```text
One proof opens the door.
Repeated proofs compound the E-Score.
```

### 3.8 Intelligence Utile

Mesure la qualite cognitive apportee au reseau.

Preuves possibles :

- decisions ameliorees ;
- risques identifies ;
- predictions correctes ;
- erreurs corrigees ;
- analyses clarifiees ;
- complexite reduite ;
- meilleurs prompts, heuristiques ou datasets ;
- contributions a de meilleurs jugements `/judge`.

Question Kernel :

```text
Cette contribution augmente-t-elle la capacite du reseau a juger correctement ?
```

### 3.9 ROI Reel

Mesure le retour reel pour le reseau, pas seulement l'effort.

Dimensions possibles :

- revenu genere ;
- cout reduit ;
- risque evite ;
- uptime augmente ;
- latence reduite ;
- qualite amelioree ;
- adoption creee ;
- capital economise ;
- erreur evitee ;
- temps gagne ;
- securite renforcee ;
- apprentissage cristallise.

Regle :

```text
Effort matters, but output compounds.
```

---

## 4. Role Du `/judge`

Le `/judge` de CYNIC evalue les contributions, signaux, claims, decisions et conflits qui alimentent le E-Score.

Il ne doit pas etre un oracle absolu. Il doit etre :

- multi-dimensionnel ;
- tracable ;
- contestable ;
- versionne ;
- auditable ;
- ameliore par retour d'experience ;
- limite par ses domaines de competence.

Regle :

```text
The Kernel judges claims, not souls.
```

Cycle minimal :

```text
claim
  -> evidence
  -> /judge
  -> observation
  -> E-Score update
  -> contest window
  -> crystallization
```

Exemple :

```text
Claim: "Operator X improved inference capacity."
Evidence: uptime logs, GPU metrics, invoices, request volume, latency delta.
/judge: quality and impact evaluation.
Observation: Kernel records result.
E-Score update: Node + ROI + Duration + Intelligence.
Contest: other operators can challenge data.
Crystallization: pattern influences future judgments.
```

---

## 5. Ce Que Le E-Score Peut Gouverner

Le E-Score peut influencer :

- priorite d'acces ;
- reputation publique ou pseudonyme ;
- admission comme operateur ;
- limites de rate, quotas ou credits ;
- eligibilite a certaines rewards de contribution ;
- priorite dans les grants ;
- confiance accordee aux signaux envoyes au Kernel ;
- delegation ou representation protocolaire ;
- friction anti-Sybil ;
- remises ;
- poids de certains votes protocolaires ;
- slashing thresholds ;
- operator tiers.

Il ne doit pas donner directement :

- action de TALARIA SAS ;
- dividende legal ;
- droit sur l'IP ;
- droit automatique sur la tresorerie ;
- immunite aux regles ;
- pouvoir legal sur les dirigeants ;
- droit de contourner KYC/AML quand la loi l'exige.

---

## 6. Citoyennete Reseau

La citoyennete CYNIC est la lecture humaine du E-Score.

Elle signifie :

```text
Cet acteur a prouve une responsabilite utile dans le reseau.
```

Elle ne signifie pas :

- identite civile ;
- KYC ;
- nationalite ;
- superiorite morale ;
- droit financier ;
- droit legal dans TALARIA SAS.

Levels indicatifs :

| Niveau | Description | Droits possibles |
|--------|-------------|------------------|
| Visitor | Utilisateur sans preuve durable | Acces public, usage limite |
| Participant | Usage ou contribution initiale | Historique, reputation faible |
| Contributor | Travail verifie | Rewards ponctuelles, priorite moderee |
| Operator | Ressource critique fournie | Paiements, staking, slashing, monitoring |
| Steward | Responsabilite durable sur une zone | Delegation protocolaire limitee |
| Guardian | Protection critique du reseau | Emergency actions encadrees |

Regle :

```text
More E-Score means more duties before more rights.
```

---

## 7. Privacy, Pseudonymat Et KYC

Le E-Score peut etre pseudonyme quand le risque le permet.

| Couche | Identite requise |
|--------|------------------|
| Usage public simple | Wallet / pseudo possible |
| Contribution technique | Pseudo possible si preuves suffisantes |
| Operator critique | Identite connue par l'entite ou multisig selon risque |
| Flux financier significatif | KYC/AML si requis |
| Relation legale avec SAS | Identite civile et contrat |
| Gouvernance legale | Identite legale selon droit applicable |

Objectif long terme :

```text
Reveal the minimum identity needed for the risk.
Prove the maximum contribution possible without exposing unnecessary personal data.
```

PoH et ZK peuvent reduire la divulgation, mais ne remplacent pas automatiquement les obligations legales existantes.

---

## 8. Anti-Capture

### 8.1 Capture Par Richesse

Posseder beaucoup de tokens ne doit pas suffire a obtenir un E-Score fort.

Le capital peut compter s'il produit :

- liquidite utile ;
- compute ;
- infrastructure ;
- distribution ;
- securite ;
- resilience.

Le capital passif ne doit pas dominer le travail verifie.

### 8.2 Capture Sociale

La proximite avec les fondateurs ne doit pas devenir une preuve.

```text
Proximity can invite trust.
Only receipts preserve it.
```

### 8.3 Capture Rhetorique

Parler bien ne suffit pas.

Une contribution doit etre reliee a :

- decision amelioree ;
- travail livre ;
- risque reduit ;
- utilisateur servi ;
- cout reduit ;
- connaissance cristallisee.

### 8.4 Capture Algorithmique

Le E-Score ne doit pas devenir un systeme opaque impossible a contester.

Obligations :

- expliquer les dimensions ;
- versionner les poids ;
- conserver les preuves ;
- permettre appel/contestation ;
- detecter biais et erreurs ;
- limiter les automatisations irreversibles.

---

## 9. Sanctions Et Decay

Le E-Score peut diminuer.

Causes :

- downtime non explique ;
- fraude ;
- faux claims ;
- extraction sans contribution ;
- conflits d'interets caches ;
- manipulation de score ;
- comportement Sybil ;
- negligence de securite ;
- refus de corriger une erreur critique ;
- abus de pouvoir protocolaire.

Mecanismes :

- decay naturel de l'influence inactive ;
- slashing operateur ;
- baisse de E-Score ;
- retrait de droits ;
- suspension ;
- publication d'un incident ;
- contestation publique ou semi-publique.

Le systeme doit separer erreur, incompetence, negligence et malveillance.

---

## 10. Relation Avec Le Token / Equity Firewall

Le E-Score peut interagir avec `$CYNIC`, mais ne doit pas transformer `$CYNIC` en action.

Autorise :

- priorite reseau ;
- reputation ;
- operator tiers ;
- rewards pour contribution mesuree ;
- staking operateur ;
- slashing ;
- access control ;
- grants ;
- remises ;
- anti-Sybil.

Interdit :

- dividende legal ;
- droit aux profits de TALARIA SAS ;
- droit aux actifs ;
- bridge automatique token -> equity ;
- remuneration de holder passif comme actionnaire ;
- promesse de rendement ;
- droit legal fonde sur E-Score.

---

## 11. Communication Policy

Phrases autorisees :

- The E-Score measures verified contribution to the CYNIC ecosystem.
- The network values useful intelligence, real work, real ROI and consistency.
- `/judge` evaluates claims and evidence, not human worth.
- Token balance alone does not define E-Score.
- More network rights imply more duties and more accountability.
- CYNIC citizenship is the social reading of E-Score, not a separate score.

Phrases interdites :

- CYNIC ranks human beings.
- Rich holders are better citizens.
- The Kernel decides who has legal rights.
- PoH replaces all KYC.
- E-Score guarantees income.
- E-Score gives equity.
- Citizenship is separate from E-Score.

---

## 12. Definition Of Done

Le E-Score est correctement formalise quand :

- il mesure des contributions verifiables ;
- il unifie reputation, citoyennete, merite operateur et priorite reseau ;
- il separe identite reseau et identite legale ;
- il ne depend pas uniquement du solde token ;
- il integre Hold, Burn, Use, Build, Node, Refer, Duration, Intelligence utile et ROI reel ;
- il est alimente par `/judge` sans devenir un oracle absolu ;
- il est contestable ;
- il respecte la privacy par defaut ;
- il augmente les devoirs en meme temps que les droits ;
- il ne devient pas un score social opaque ;
- il reste compatible avec le token/equity firewall.
