# CYNIC Protocol Revenue Waterfall

Ce document definit comment les revenus protocolaires CYNIC sont reserves, routes, redistribues et multiplies par le E-Score.

Il depend de `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md` : la waterfall ne cree pas la verite. Elle execute des regles publiees a partir de faits prouves dans les sources de verite correctes.

Documents relies :

- `docs/foundation/TALARIA_CYNIC_NAMING.md`
- `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md`
- `docs/foundation/CYNIC_E_SCORE.md`
- `docs/foundation/CYNIC_OWNERSHIP_CONSTITUTION.md`
- `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md`
- `docs/foundation/CYNIC_LEGAL_GENESIS_PLAN.md`

---

## 1. Lois Fondatrices

```text
Ownership follows causal contribution.
Survival before burn.
Tax before rewards.
Costs before surplus.
Contribution before multiplier.
No passive revenue by default.
No legal dividend through protocol rewards.
```

Le rendement appartient d'abord a celui qui prend le risque. Chaque couche qui rend ce rendement possible peut recevoir une fraction mesurable du flux qu'elle a cause : infrastructure, liquidite, signal, execution, securite, IP, gouvernance, compute ou jugement.

Le protocole peut redistribuer des revenus pour des contributions mesurees. Il ne doit pas creer un dividende masque pour holders passifs.

---

## 2. Boundary Source Of Truth

Une waterfall correcte separe quatre choses :

```text
Primary SoTs prove facts.
Kernel computes allocation signals.
Smart contracts execute published rules.
TALARIA SAS owns legal/IP/governance rights.
```

Le Kernel peut influencer :

- E-Score multiplier ;
- reward eligibility ;
- slashing recommendation ;
- operator tier ;
- governance signal ;
- anomaly warning.

Le Kernel ne doit pas seul :

- transferer des fonds corporate ;
- creer des droits legaux ;
- override des smart contracts ;
- saisir le PnL utilisateur ;
- inventer des fees ;
- reassigner l'IP ;
- modifier la cap table ;
- bypasser les reserves tax/accounting.

---

## 3. Waterfall Generale

Chaque branche doit publier sa waterfall avant de router des flux significatifs.

Template :

```text
100% branch protocol revenue
  -> direct costs reserve
  -> tax/accounting reserve
  -> risk and security reserve
  -> operator/service payments
  -> measured contribution rewards
       -> E-Score multiplier
  -> TALARIA SAS IP/governance fee if applicable
  -> protocol surplus
       -> buyback
       -> burn
       -> grants
       -> liquidity incentives
       -> ecosystem reserves
```

L'ordre est intentionnel. Une branche qui brule avant de survivre ment a son propre marche.

---

## 4. Categories De Flux

### 4.1 Direct Costs Reserve

Reserve pour les couts qui rendent le service possible :

- RPC ;
- indexers ;
- compute ;
- GPU ;
- bandwidth ;
- storage ;
- hosting ;
- monitoring ;
- gas/fees ;
- off-ramp ;
- support ;
- maintenance.

```text
No direct cost coverage, no sustainable revenue.
```

### 4.2 Tax / Accounting Reserve

Reserve pour :

- impots ;
- TVA si applicable ;
- comptabilite ;
- audit ;
- reporting ;
- conversion crypto/fiat ;
- pertes de change ;
- obligations locales.

```text
No burn before tax reserve.
No reward before accounting clarity.
```

### 4.3 Risk And Security Reserve

Reserve pour :

- audits ;
- bug bounties ;
- incidents ;
- exploit response ;
- slashing buffer ;
- smart contract risk ;
- emergency migrations.

### 4.4 Operator / Service Payments

Paiements dus aux acteurs qui rendent un service mesurable :

- compute provider ;
- GPU provider ;
- RPC provider ;
- hosting provider ;
- LP provider ;
- fee payer ;
- auditor ;
- keeper ;
- maintainer ;
- data provider ;
- security responder.

Ces paiements remunerent un service, pas une proximite sociale.

### 4.5 Measured Contribution Rewards

Rewards pour contribution verifiee :

- travail livre ;
- integration ;
- signal utile ;
- strategie validee ;
- utilisateur apporte ;
- risque pris ;
- liquidite productive ;
- uptime ;
- incident resolu ;
- decision amelioree.

La reward de base doit etre calculee avant multiplicateur E-Score.

```text
base_reward = measured_contribution_value
final_reward = base_reward * f(E-Score)
```

### 4.6 TALARIA SAS IP / Governance Fee

TALARIA SAS peut recevoir une fee si la branche utilise :

- marque CYNIC ;
- IP ;
- standards ;
- kernel governance ;
- policies ;
- software ;
- coordination legale ;
- distribution institutionnelle.

Cette fee est un revenu corporate taxable de TALARIA SAS. Elle n'est pas due aux holders de `$CYNIC`.

### 4.7 Protocol Surplus

Surplus restant apres couts, taxes, reserves, operators, contributors et fees applicables.

Usages possibles :

- buyback ;
- burn ;
- grants ;
- liquidity incentives ;
- E-Score rewards ;
- security reserve ;
- ecosystem reserves.

Le surplus ne doit pas etre presente comme un rendement garanti.

---

## 5. E-Score Comme Multiplicateur

Le E-Score peut modifier :

- poids de vote protocolaire ;
- priorite reseau ;
- montant de rewards sur contribution mesuree ;
- remises ;
- operator tiers ;
- slashing thresholds ;
- grants priority.

Il ne doit pas multiplier :

- revenus passifs ;
- droits legaux ;
- dividendes ;
- profits de TALARIA SAS ;
- simple detention de token.

Regle :

```text
The E-Score can multiply measured contribution.
It cannot create passive income.
```

Forme indicative :

```text
multiplier(E) = 1 + cap * (1 - exp(-kE))
```

Proprietes requises :

- plafonnee ;
- sous-lineaire ;
- decroissante si inactivite ;
- contestable ;
- versionnee ;
- robuste aux Sybils ;
- separee de la cap table ;
- compatible avec le token/equity firewall.

---

## 6. Gouvernance Protocolaire

Le E-Score peut servir de poids de vote dans la gouvernance protocolaire.

Forme indicative :

```text
vote_weight = token_weight * f(E-Score)
```

`f(E-Score)` doit etre plafonnee. Le but est d'eviter deux captures :

- pure plutocratie par token balance ;
- aristocratie opaque par score non contestable.

Gouvernable par E-Score :

- parametres de fees ;
- grants ;
- operator registry ;
- slashing ;
- E-Score formulas ;
- liquidity incentives ;
- branch-level policies ;
- protocol treasury routing.

Non gouvernable directement par E-Score :

- statuts TALARIA SAS ;
- nomination du president ;
- cession d'actions ;
- dividendes legaux ;
- transfert d'IP ;
- controle legal des filiales.

---

## 7. Performance Fee Sur ROI Utilisateur

Si un utilisateur realise un ROI exceptionnel avec son propre capital et son propre risque, le protocole ne doit pas confisquer son upside.

Forme propre :

```text
user realized profit
  -> user keeps majority of upside
  -> protocol charges public performance fee if applicable
  -> fee is routed by waterfall
```

Fourchette indicative a etudier :

```text
0.5% - 5% of realized profit
```

Plus haut seulement si CYNIC fournit directement :

- strategie ;
- signal `/judge` ;
- execution ;
- capital ;
- liquidite ;
- risk buffer ;
- compute ;
- automation ;
- protection anti-rug ;
- routing.

Regle :

```text
The more causal work the network provides, the larger the justified protocol fee.
The more independent the user's risk and decision, the smaller the justified protocol fee.
```

---

## 8. Exemple +10000% ROI

Hypothese :

```text
User capital: 1,000
Exit value: 101,000
Realized profit: 100,000
Protocol performance fee: 2%
Protocol fee: 2,000
User gross profit after protocol fee: 98,000
```

Exemple de routing des `2,000` :

```text
2,000 protocol fee
  -> 25% direct infra / execution / RPC / compute
  -> 20% signal, judge, data, strategy contributors
  -> 15% liquidity providers / risk buffers
  -> 10% TALARIA SAS IP/governance fee
  -> 10% security/audit reserve
  -> 10% buyback/burn
  -> 10% grants / E-Score rewards / ecosystem reserve
```

Ce template n'est pas une promesse. Chaque branche doit publier ses propres parametres avant usage significatif.

---

## 9. GASdf Waterfall Indicative

```text
GASdf fee revenue
  -> reimburse SOL fee payer
  -> RPC/indexer/Jupiter/Helius costs
  -> tax/accounting reserve
  -> LP / fee payer / operator margin
  -> risk and security reserve
  -> measured contribution rewards
       -> E-Score multiplier
  -> TALARIA SAS IP/governance fee
  -> protocol surplus
       -> buyback/burn
       -> liquidity incentives
       -> grants
```

GASdf ne doit pas vendre du rendement. GASdf peut payer les acteurs qui rendent le gasless possible.

---

## 10. Fractal Ownership

L'ownership est fractal parce que chaque flux possede plusieurs causes.

```text
User owns:
  -> capital
  -> timing
  -> risk
  -> final decision

Protocol owns:
  -> routing rules
  -> smart contract execution
  -> treasury logic
  -> published fee logic

Operators own:
  -> compute
  -> uptime
  -> RPC
  -> liquidity
  -> security

Primary SoTs own:
  -> legal facts
  -> protocol facts
  -> operational facts
  -> market facts

Kernel judges:
  -> evidence
  -> conflicts
  -> E-Score
  -> Proof of History

TALARIA SAS owns:
  -> IP
  -> standards
  -> legal governance
  -> brand
```

La redistribution doit suivre cette causalite, pas un pourcentage arbitraire.

---

## 11. Red Flags

Bloquer et relire si une waterfall cree :

- redistribution a holders passifs ;
- promesse de rendement ;
- buyback garanti ;
- taxes non reservees ;
- couts directs ignores ;
- IP fee non documentee ;
- rewards sans contribution mesuree ;
- E-Score non contestable ;
- performance fee cachee ;
- fee variable non publiee ;
- route vers wallet personnel ;
- confusion entre protocol reward et dividende ;
- Kernel traite comme source primaire.

---

## 12. Definition Of Done

Une waterfall est prete quand :

- chaque flux a un owner ;
- chaque fee a une justification ;
- couts directs et taxes passent avant burn ;
- TALARIA SAS fees sont separees des protocol rewards ;
- E-Score multiplie uniquement des contributions mesurees ;
- vote_weight et reward_multiplier sont plafonnes ;
- la performance fee est publique avant usage ;
- les wallets sont identifies par fonction ;
- le protocole ne promet pas de rendement ;
- les faits sont ancres dans les SoT primaires ;
- le modele respecte le token/equity firewall.
