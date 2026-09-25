# CYNIC Source Of Truth Hierarchy

Ce document definit la hierarchie des sources de verite CYNIC.

Son objectif est d'empecher une confusion critique : le Kernel observe, juge, score et detecte les conflits, mais il ne devient jamais la source primaire de verite legale, protocolaire, operationnelle ou economique.

Documents relies :

- `docs/foundation/TALARIA_CYNIC_NAMING.md`
- `docs/CYNIC_ECOSYSTEM_ARCHITECTURE.md`
- `docs/foundation/CYNIC_OWNERSHIP_CONSTITUTION.md`
- `docs/foundation/CYNIC_E_SCORE.md`
- `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md`
- `docs/foundation/CYNIC_LEGAL_GENESIS_PLAN.md`

---

## 1. Loi Centrale

```text
The Kernel does not own the truth.
The Kernel judges evidence anchored in the correct source of truth.
```

Le Kernel ne doit pas devenir un faux registre legal, un faux smart contract, une fausse comptabilite, un faux oracle de marche ou un proprietaire economique.

Il sert a observer, verifier, juger, scorer, detecter les conflits, produire le E-Score et signaler les incoherences.

Il ne sert pas a creer des droits legaux, remplacer les statuts, remplacer la comptabilite, remplacer les smart contracts, inventer un ownership non prouve ou rendre une preuve vraie par simple jugement.

---

## 2. Hierarchie Des Sources

### 2.1 Legal Source Of Truth

Source primaire pour les droits legaux :

- statuts de TALARIA SAS ;
- pacte d'associes ;
- registre des mouvements de titres ;
- contrats ;
- factures ;
- comptabilite ;
- decisions sociales ;
- registres obligatoires ;
- documents fiscaux ;
- avis juridiques ;
- contrats de licence IP.

Elle decide qui possede les actions, qui dirige la SAS, qui detient ou exploite l'IP, quels contrats lient les parties, quels revenus sont corporate et quels droits sont opposables legalement.

Le Kernel peut lire, hasher, observer ou juger des preuves issues de cette couche. Il ne peut pas modifier seul les droits legaux.

### 2.2 Protocol Source Of Truth

Source primaire pour les droits et flux on-chain :

- smart contracts ;
- transactions Solana ;
- wallet registry ;
- treasury registry ;
- branch waterfall publiee ;
- multisig transactions ;
- timelocks ;
- token accounts ;
- staking contracts ;
- slashing records ;
- burn transactions.

Elle decide quels flux on-chain ont eu lieu, quels wallets ont recu ou envoye, quels droits protocolaires existent, quels burns sont reels, quels rewards ont ete executes et quels votes protocolaires sont valides.

Le Kernel peut interpreter ces donnees, mais la transaction on-chain reste la preuve primaire.

### 2.3 Operational Source Of Truth

Source primaire pour le travail reel :

- logs d'uptime ;
- metrics GPU ;
- metrics RPC ;
- latence ;
- request volume ;
- incident reports ;
- signed operator reports ;
- invoices ;
- monitoring ;
- deployment logs ;
- audit reports ;
- support tickets ;
- commits ;
- test reports.

Elle prouve qui a opere quoi, combien de compute a ete fourni, quelle qualite de service a ete livree, quels incidents ont eu lieu, quels couts ont ete supportes et quels services justifient paiement.

Le Kernel peut juger la qualite et l'impact de ces preuves. Il ne doit pas les remplacer.

### 2.4 Market Source Of Truth

Source primaire pour le ROI reel et l'effet de marche :

- prix ;
- liquidite ;
- volume ;
- realized PnL ;
- slippage ;
- fees ;
- usage ;
- demande ;
- retention ;
- conversion ;
- churn ;
- market depth ;
- oracle data ;
- DEX/CEX data si applicable.

Elle prouve quel ROI a ete realise, quelle liquidite etait disponible, quels effets de reseau sont observables, quelle demande existe et quelle valeur economique a ete creee ou detruite.

Le Kernel peut analyser et scorer. Il ne doit pas declarer un ROI sans ancrage marche.

### 2.5 Kernel Judgment Layer

Le Kernel est une couche de jugement, pas une source primaire.

Il produit :

- observations ;
- verdicts `/judge` ;
- E-Score ;
- conflict signals ;
- anomaly detection ;
- confidence bounds ;
- decay ;
- slashing recommendations ;
- governance signals.

Ces sorties sont secondaires :

```text
Kernel output = interpretation of primary evidence.
```

Elles doivent rester contestables contre les sources primaires.

---

## 3. Regle De Contestation

Tout jugement Kernel doit pouvoir etre conteste par preuve primaire.

```text
Kernel says: operator uptime was poor.
Operator contests with signed logs + monitoring + third-party probes.
Result: Kernel must revise, explain, or mark uncertainty.
```

```text
Kernel says: ROI was high.
Market SoT shows no realized exit, only unrealized mark-to-market.
Result: reward cannot be based on realized profit.
```

```text
Kernel says: actor deserves reward.
Protocol SoT shows no eligible smart contract flow.
Result: no automatic payout until the waterfall permits it.
```

---

## 4. E-Score Anchoring

Le E-Score est calcule par le Kernel, mais ancre dans les bonnes sources de verite.

```text
E-Score input
  -> legal proofs when legal rights are involved
  -> on-chain proofs when protocol rights are involved
  -> operational proofs when work is involved
  -> market proofs when ROI is involved

Kernel
  -> verifies
  -> judges
  -> scores
  -> emits E-Score

E-Score output
  -> protocol weight
  -> reputation
  -> operator tier
  -> reward multiplier on measured contribution
```

Regle :

```text
No primary proof, no durable E-Score.
```

---

## 5. Revenue Waterfall Boundary

Une waterfall ne doit pas dire :

```text
Kernel owns judgment, therefore Kernel owns allocation.
```

Elle doit dire :

```text
Primary SoTs prove facts.
Kernel computes allocation signals.
Smart contracts execute published rules.
TALARIA SAS owns legal/IP/governance rights.
Operators own their measured service.
Users own their capital, risk and realized PnL.
```

Le Kernel peut influencer E-Score multiplier, reward eligibility, slashing recommendation, operator tier, governance signal et anomaly warning.

Le Kernel ne doit pas seul transferer des fonds corporate, creer des droits legaux, override des smart contracts, saisir le PnL utilisateur, inventer des fees, reassigner l'IP, modifier la cap table ou bypasser les reserves tax/accounting.

---

## 6. Proof Of History

La Proof of History CYNIC est l'historique verifiable d'un acteur dans les sources de verite correctes.

Elle inclut :

- ce qui a ete fait ;
- quand cela a ete fait ;
- combien de temps cela a tenu ;
- quel cout a ete supporte ;
- quel ROI reel a ete produit ;
- quelles preuves primaires existent ;
- comment le Kernel a juge ces preuves ;
- quelles contestations ont eu lieu ;
- quelles corrections ont ete appliquees.

Formule :

```text
E-Score = protocol weight of an actor's verified Proof of History.
```

Mais :

```text
Proof of History is anchored in primary SoTs.
Kernel judgment is the interpretation layer.
```

---

## 7. Red Flags

Bloquer et relire si un design implique :

- le Kernel comme registre legal ;
- le Kernel comme comptabilite ;
- le Kernel comme proprietaire de wallets ;
- le Kernel comme source unique du ROI ;
- le Kernel comme createur de droits financiers ;
- le Kernel comme remplacement des smart contracts ;
- le Kernel comme arbitre non contestable ;
- une reward sans preuve primaire ;
- un E-Score impossible a challenger ;
- une waterfall executee sans regle publiee.

---

## 8. Definition Of Done

La hierarchie SoT est respectee quand :

- chaque droit legal pointe vers une preuve legale ;
- chaque flux on-chain pointe vers une transaction ou un smart contract ;
- chaque travail remunere pointe vers une preuve operationnelle ;
- chaque ROI pointe vers une preuve marche ;
- chaque E-Score pointe vers des preuves primaires ;
- chaque jugement Kernel est contestable ;
- chaque waterfall separe preuve, jugement et execution ;
- le Kernel reste juge, pas proprietaire de la verite.
