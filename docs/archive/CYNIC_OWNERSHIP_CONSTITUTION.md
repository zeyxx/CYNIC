# CYNIC Ownership Constitution

Ce document formalise le principe d'ownership de CYNIC. Il complete l'architecture ecosysteme en separant clairement les proprietaires legaux, les operateurs, les wallets, les smart contracts, les flux protocolaires, les reserves fiscales et les preuves publiques.

La separation naming est definie dans `docs/foundation/TALARIA_CYNIC_NAMING.md` : TALARIA SAS est l'entite legale genesis, CYNIC est le framework/protocol.

La waterfall detaillee des revenus protocolaires est formalisee dans `docs/foundation/CYNIC_PROTOCOL_REVENUE_WATERFALL.md`.

La these centrale :

> La souverainete ne vient pas de l'absence de proprietaire. Elle vient de proprietaires identifies, responsables, mesures et limites par des regles publiques.

---

## 1. Lois Fondatrices

1.  **Every flow has an owner.** Aucun flux fiat, crypto, compute, wallet, smart contract, IP, taxe ou burn ne doit exister sans responsable identifie.
2.  **Every owner has duties.** Posseder une ressource CYNIC implique des devoirs : uptime, comptabilite, securite, reporting, fiscalite, maintenance ou gouvernance.
3.  **Every duty has receipts.** Les devoirs critiques doivent produire des preuves primaires : factures, hash de documents, transactions on-chain, rapports d'audit, logs de performance, registres et contrats. Les observations Kernel peuvent indexer et juger ces preuves, mais ne les remplacent pas.
4.  **Survival before burn.** Aucune branche ne doit bruler ou redistribuer ce qui est necessaire a ses couts directs, ses impots, sa securite, sa comptabilite et sa durabilite.
5.  **Token is not equity.** `$CYNIC` ne represente pas une action de TALARIA SAS, ne donne pas droit aux dividendes legaux, et ne constitue pas une creance sur les actifs de la societe.
6.  **Protocol rewards follow verified contribution.** Les revenus on-chain remunerent le travail, le risque, la liquidite, le compute, l'uptime ou la securite verifies ; ils ne remunerent pas la simple proximite sociale.

---

## 2. Couches d'Ownership

| Couche | Proprietaire | Responsabilite |
|--------|--------------|----------------|
| Legal | TALARIA SAS et ses associes | Statuts, cap table, IP, gouvernance, obligations fiscales |
| IP | TALARIA SAS | Marque, code, standards, licences, droits d'usage par les branches |
| Branches | Filiales ou entites controlees | Produits, revenus, contrats, operations commerciales |
| Infrastructure | Datacenter/InfraCo ou operateurs federes | GPU, serveurs, RPC, storage, hosting, uptime |
| Protocol | Smart contracts et treasuries identifies | Collecte de fees, routing, reserves, buyback, burn, rewards |
| Wallets | Entites ou operateurs nommes | Garde, comptabilite, off-ramp, signatures, separation des fonds |
| Merit | Sources primaires + Kernel | Preuves legales/protocolaires/operationnelles/marche, puis jugement Kernel et E-Score |

---

## 3. TALARIA SAS

TALARIA SAS est le tronc legal. Elle opere, protege et gouverne le framework/protocole CYNIC. Elle ne doit pas etre une coquille passive : elle protege l'IP, gouverne les standards, detient ou controle les branches, signe les contrats et porte la responsabilite institutionnelle.

Responsabilites :

- conserver la cap table legale ;
- detenir la marque, l'IP et les standards CYNIC ;
- signer les contrats B2B/B2G ;
- definir les regles d'admission des branches et operateurs ;
- maintenir les policies publiques : token/equity firewall, fair launch, operator charter, treasury policy ;
- tenir une comptabilite exploitable par l'administration francaise ;
- separer les wallets de la SAS, des filiales, des operateurs et des protocol treasuries.

Revenus possibles :

- licences et royalties IP ;
- management fees verses par les branches ;
- revenus B2B/B2G directs ;
- dividendes legaux de filiales ;
- protocol operator/governance fees explicitement routes vers des wallets d'entite.

Ces revenus sont imposables selon leur nature. Ils ne sont pas dus automatiquement aux holders de `$CYNIC`. Les revenus protocolaires on-chain peuvent remunerer une contribution verifiee, mais ne doivent pas etre presentes comme des dividendes legaux de TALARIA SAS.

---

## 4. InfraCo / Datacenter

Une branche infra ou datacenter peut posseder les machines et recevoir des revenus on-chain pour du travail verifie : compute, inference, RPC, storage, hosting, monitoring ou security.

Responsabilites :

- posseder ou louer les serveurs/GPU ;
- payer electricite, bande passante, locaux, maintenance, assurance et remplacement materiel ;
- prouver l'uptime et la capacite ;
- reserver les montants necessaires aux impots et couts directs ;
- fournir les donnees de performance au Kernel ;
- payer une licence, royalty ou management fee a TALARIA SAS si elle utilise l'IP ou les standards CYNIC.

Une InfraCo ne doit pas survivre en vendant du narratif token. Elle doit survivre en vendant du compute ou de l'infrastructure utile.

---

## 5. Wallet Ownership

Chaque wallet significatif doit avoir un owner, une fonction et une politique de controle.

| Wallet type | Owner attendu | Fonction |
|-------------|---------------|----------|
| SAS Treasury | TALARIA SAS | Revenus corporate, legal, compta, fiscalite |
| Branch Treasury | Filiale/branche | Revenus produit et couts directs |
| Protocol Treasury | Smart contract / multisig | Fees on-chain, reserves, rewards |
| Tax Reserve | Entite concernee | Stablecoin/fiat reserve pour impots |
| Burn Wallet | Smart contract | Buyback/burn transparent |
| Operator Wallet | Operateur | Paiement pour service mesure |
| LP/Vault Wallet | Smart contract / LP provider | Liquidite productive et risque associe |

Regles :

- aucun wallet d'entite ne doit etre confondu avec un wallet personnel ;
- les wallets critiques doivent etre multisig ou timelock quand c'est compatible avec l'operation ;
- les flows entre wallets doivent etre justifies par une categorie comptable ou protocolaire ;
- les snapshots de wallet ownership peuvent etre hashes et publies on-chain sans exposer les documents prives.

---

## 6. Protocol Revenue Waterfall

Chaque branche on-chain doit publier une waterfall avant de router ses flux.

La version complete est maintenue dans `docs/foundation/CYNIC_PROTOCOL_REVENUE_WATERFALL.md`.

Template :

```text
100% branch protocol revenue
  -> direct costs reserve
  -> tax/accounting reserve
  -> operator payments
  -> TALARIA SAS IP/governance fee
  -> security/audit reserve
  -> protocol surplus
       -> buyback
       -> burn
       -> grants
       -> E-Score rewards
       -> liquidity incentives
```

L'ordre est intentionnel. Le burn ne doit jamais passer avant les couts de survie.

---

## 7. GASdf Example

GASdf necessite de la liquidite operationnelle SOL/USDC pour fournir du gasless a grande echelle.

Waterfall indicative :

```text
GASdf fee revenue
  -> reimburse SOL fee payer
  -> RPC/Helius/Jupiter/indexer costs
  -> tax/accounting reserve
  -> fee payer / LP / operator margin
  -> TALARIA SAS IP/governance fee
  -> buyback/burn ASDF or CYNIC
  -> ecosystem rewards
```

Les fournisseurs de liquidite ne sont pas des insiders. Ils prennent un risque public : usage insuffisant, volatilite, smart contract risk, drain prevention, cooldown, slashing ou perte d'opportunite. Le rendement potentiel vient de l'utilisation reelle de GASdf, pas d'une promesse de TALARIA SAS.

---

## 8. Token / Equity Firewall

`$CYNIC` peut coordonner l'acces, le staking, le compute, la reputation, les rewards de contribution et la gouvernance protocolaire. Il ne doit pas representer la propriete legale de TALARIA SAS.

La policy autonome et publiable est maintenue dans `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md`.

Interdits :

- `1 token = 1 action` ;
- droit legal aux dividendes de TALARIA SAS ;
- droit automatique sur les profits des filiales ;
- allocation cachee pour insiders ;
- promesse de rendement ;
- buyback presente comme revenu garanti.

Autorises si correctement documentes :

- staking pour acceder au moteur ;
- paiement de compute ou inference ;
- staking operateur avec slashing ;
- rewards pour service mesure ;
- E-Score comme reputation et priorite ;
- buyback/burn discretionnaire ou contractuel, mais apres reserves de survie.

---

## 9. Kernel as Ownership Judge

Le Kernel est le cinquieme pole : il ne possede pas legalement la SAS, mais il juge les contributions qui pretendent meriter du poids dans le reseau.

Il mesure :

- uptime ;
- compute fourni ;
- requetes servies ;
- cout reel ;
- qualite des jugements ;
- incidents ;
- liquidite utile ;
- contributions techniques ;
- audits ;
- PoH et usage reel.

Le Kernel doit produire des observations exploitables par le E-Score et par les policies d'operateurs. Les regles de scoring doivent etre versionnees, auditables et contestables.

Le E-Score issu de ces observations est defini dans `docs/foundation/CYNIC_E_SCORE.md`. Il unifie reputation, citoyennete reseau, merite operateur et priorite protocolaire. Il mesure la responsabilite meritee dans le reseau, pas la valeur humaine ni les droits legaux dans TALARIA SAS.

---

## 10. Questions A Resoudre

1.  Quelle entite possede le datacenter : TALARIA SAS directement ou InfraCo filiale ?
2.  Quels wallets doivent exister au legal genesis du 23 juillet 2026 ?
3.  Quel pourcentage minimal chaque branche reserve pour taxes et couts directs ?
4.  Quelles branches ont le droit de faire buyback/burn des le lancement ?
5.  Quel est le format public du wallet ownership registry ?
6.  Quelle politique comptable s'applique aux tokens recus, swaps et burns ?
7.  Quelle procedure permet de modifier une waterfall ?
8.  Quels flows exigent multisig, timelock ou publication prealable ?
