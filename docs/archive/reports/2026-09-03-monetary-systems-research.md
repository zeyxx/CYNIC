# Recherche des architectures monétaires pour CYNIC

Date : 2026-09-03
Statut : recherche comparative et audit de l’existant, non normatif

Ce document ne transforme aucune proposition en loi. Les documents sous `docs/` restent subordonnés aux contrats et au runtime observé.

## 1. Ce qui existe déjà dans CYNIC

Les artefacts existants décrivent plusieurs instruments :

- E-Score : identité/réputation comportementale, principalement wallet/token ;
- `$CYNIC` : protocol fuel transférable, stake et accès aux branches ;
- CYD : unité de compte proposée pour le compute, annoncée comme `1 CYD ≈ 1 Wh` ;
- SOL/USDC : actifs externes utilisés pour frais, financement et trésorerie ;
- E-Score/achievements/SBT : réputation ou preuves non transférables ;
- budgets/claims : droits de consommation internes, qui ne sont pas de la monnaie.

Sources :

- `docs/v2-architecture-canon/CYNIC_ESCORE_SPECIFICATION.md:156-226`
- `docs/v2-architecture-canon/CYNIC_ESCORE_INDUSTRIAL_ROADMAP.md:23-35,62-70,121-141`
- `docs/v2-architecture-canon/CYNIC_TOKENOMICS_ARCHITECTURE.md:1-68`
- `docs/v2-architecture-canon/CYNIC_DEFLATIONARY_MECHANICS.md:1-61`
- `infra/MARKET_ARCHITECTURE.md:495-584`
- `crates/cynic-protocol/src/energy_ledger.rs`
- `crates/cynic-protocol/src/agent_workload/`

## 2. Diagnostic de cohérence

CYNIC ne possède pas encore une monnaie unique formalisée. Il possède un ensemble de propositions :

- `$CYNIC` est appelé énergie liquide et protocol fuel ;
- CYD est appelé unité de compte indexée sur l’énergie ;
- E-Score est appelé carte de citoyenneté et pouvoir économique ;
- les budgets sont des limites d’exécution ;
- les stakes sont des garanties exposées au risque ;
- SOL/USDC servent de moyens de financement ou de règlement externes.

Ces objets doivent être séparés :

`UnitOfAccount ≠ Currency ≠ EnergyBudget ≠ Stake ≠ Reputation ≠ Authority`

La proposition `1 CYD ≈ 1 Wh` ne définit pas encore une valeur stable. Un Wh est une mesure physique ; sa valeur économique dépend du matériel, du marché, du type de calcul, de la disponibilité et de la qualité de la preuve. Il faut donc distinguer :

- énergie observée en joules/Wh ;
- coût de production ;
- valeur de preuve ;
- prix interne de règlement ;
- prix externe de marché.

## 3. Modèles observés à l’extérieur

### 3.1 Banque centrale et système bancaire à deux niveaux

Le modèle moderne sépare généralement :

- monnaie de banque centrale : actif de règlement ultime ;
- monnaie bancaire privée : créance convertible à par ;
- système de paiement et de règlement ;
- politique monétaire ;
- supervision et stabilité financière ;
- fourniture de liquidité en période de stress.

La banque centrale n’est pas seulement un émetteur. Elle fournit aussi une infrastructure de règlement, une politique de stabilité et une assurance de liquidité.

Références :

- BIS, lender of last resort and modern central banking : <https://www.bis.org/publ/bppdf/bispap79b_rh.pdf>
- ECB, central bank money settlement : <https://www.ecb.europa.eu/press/economic-bulletin/articles/2024/html/ecb.ebart202308_01~d9a13e1609.en.html>
- Federal Reserve, purposes and functions : <https://www.federalreserve.gov/monetarypolicy/pf.htm>
- Federal Reserve, money and payments/CBDC : <https://www.federalreserve.gov/publications/january-2022-cbdc.htm>

Leçon pour CYNIC : une monnaie exige au minimum une unité de compte, un émetteur, un registre de soldes, un mécanisme de règlement et un régime de crise. « Émettre des tokens pour récompenser le compute » ne suffit pas.

### 3.2 Erreurs des banques centrales et systèmes monétaires

Les risques récurrents sont :

- perte d’ancrage des anticipations et inflation persistante ;
- expansion de liquidité sans règle crédible ;
- confusion entre solvabilité comptable et capacité opérationnelle ;
- manque de transparence sur les réserves et les pertes ;
- absence de prêteur en dernier ressort ou intervention trop tardive ;
- conflit entre stabilité des prix, stabilité financière et soutien budgétaire ;
- décisions centralisées non contestables.

La BIS souligne que la stabilité dépend davantage de la confiance dans la valeur et dans le mandat que d’un simple bilan positif de la banque centrale.

Référence : <https://www.bis.org/publ/arpdf/ar2024e2.htm>

Leçon pour CYNIC : un ledger peut rester techniquement cohérent tout en perdant sa valeur si la règle d’émission, la convertibilité, les réserves ou la gouvernance ne sont pas crédibles.

### 3.3 Bitcoin

Bitcoin privilégie :

- émission prédéterminée ;
- plafond nominal ;
- règles vérifiables par les nœuds ;
- absence de prêteur central en dernier ressort ;
- ajustement par le prix plutôt que par une élasticité discrétionnaire de l’offre.

Leçon : crédibilité, simplicité et vérifiabilité, mais faible capacité d’adaptation à un gouvernement de ressources variables.

### 3.4 Ethereum

Ethereum combine :

- émission de récompenses de validation ;
- frais de base ajustés par congestion ;
- destruction du base fee ;
- priority fee vers le validateur ;
- offre totale dépendant de l’usage et de la sécurité du réseau.

Références :

- <https://ethereum.org/eth/supply/>
- <https://eips.ethereum.org/EIPS/eip-1559>

Leçon : séparer le prix de l’utilisation, la récompense de sécurité et le mécanisme de burn. Une politique d’offre dynamique doit avoir une fonction explicite, des bornes et une mesure de la demande.

### 3.5 Solana

Solana apporte plusieurs primitives utiles mais ne définit pas la monnaie CYNIC :

- SOL est l’actif natif pour frais et staking ;
- les tokens sont des comptes de mint avec supply, decimals, mint authority et freeze authority éventuelle ;
- les comptes d’associated token rendent la propriété et le stockage explicites ;
- Proof of History fournit une preuve d’ordre et de passage du temps, pas une preuve de valeur ;
- les frais locaux peuvent refléter la contention d’un état particulier ;
- le consensus et le règlement sont distincts de la politique monétaire du token applicatif.

Références officielles :

- historique et Proof of History : <https://solana.com/docs/intro/history>
- terminologie, comptes et autorités : <https://solana.com/docs/terminology>
- tokens, supply et mint authority : <https://solana.com/docs/tokens>
- programmes natifs et frais : <https://solana.com/docs/core/programs/builtin-programs>
- whitepaper : <https://solana.com/solana-whitepaper.pdf>

Leçon : réutiliser Solana pour le règlement, la preuve d’ordre et les comptes ne signifie pas lui déléguer la définition de la politique monétaire CYNIC. PoH n’est pas une mesure de compute économique et le staking n’est pas une réputation.

### 3.6 Stablecoins et runs

Les stablecoins montrent un risque particulier : une promesse de valeur stable peut devenir instable si les réserves sont insuffisantes, illiquides, opaques ou exposées à une vente coordonnée.

Références :

- IMF, Making Stablecoins Stable : <https://www.imf.org/-/media/files/publications/wp/2026/english/wpiea2026074-source-pdf.pdf>
- Federal Reserve, CBDC risks and payment design : <https://www.federalreserve.gov/publications/january-2022-cbdc.htm>

Leçon : toute promesse de convertibilité CYD/compute ou CYD/SOL exige une réserve, une politique de redemption, une gestion de run et une preuve indépendante des actifs sous-jacents.

## 4. Stages monétaires possibles pour CYNIC

### Stage 0 — Aucun argent

CYNIC ne fait que mesurer :

- budgets ;
- énergie ;
- temps ;
- preuves ;
- valeur d’outcome.

Aucun transfert ni émission. C’est le stage actuel du runtime observé.

### Stage 1 — Unité de compte interne et calibration pré-Genesis

CYD mesure une créance ou un coût interne, sans être encore transférable :

- compte analytique ;
- débit/crédit ;
- coût observé ;
- valeur de preuve ;
- dette ;
- escrow interne ;
- expiration et contestation.

Décision de conception : tous les CYD accumulés avant le token Genesis sont des crédits de calibration du système, pas une monnaie publique et pas une promesse de redemption. Ils servent à tester les règles d’attribution, de preuve, de budget, de contribution et de résistance Sybil.

Les phases publiques peuvent fonctionner comme des testnets/L2 de préparation : des participants observent, fournissent du compute, exécutent des workloads utiles et produisent des preuves. Le Genesis peut ensuite distribuer un airdrop dérivé d’un snapshot de contributions prouvées. Il ne doit pas être proportionnel au nombre de transactions, aux appels artificiels ou aux écrans sociaux.

La primitive à récompenser est le Proof-of-Useful-Work :

`mandat → workload → ressources observées → résultat utile → validation indépendante → crédit CYD`

Une preuve de compute seule ne suffit pas. Elle doit être liée à un travail déclaré, un résultat évalué, une fenêtre, une identité d’attribution, une détection de doublon et une limite anti-burst.

C’est le premier stage raisonnable pour CYNIC.

### Stage 2 — Crédit transférable sous mandat

Des agents ou offices peuvent transférer une allocation limitée entre comptes. Il faut alors :

- autorité d’émission ;
- autorité de transfert ;
- double entrée ;
- idempotence ;
- nonces ;
- limites ;
- révocation ;
- insolvabilité ;
- audit des soldes.

### Stage 3 — Monnaie publique de règlement

Un actif peut être publié ou réglé sur Solana. Il faut en plus :

- mint authority ;
- supply observable ;
- décimales ;
- comptes ;
- frais ;
- settlement finality ;
- gestion des clés ;
- politique de suspension/freeze ;
- arbitrage juridique et consentement de publication.

### Stage 4 — Politique monétaire adaptative

L’offre, les frais, les récompenses et les burns évoluent selon des métriques de demande, de capacité et de stabilité. Ce stage exige une simulation adversariale avant activation.

## 5. Architecture gouvernementale de la monnaie

Une future constitution monétaire doit séparer :

- `Treasury` : conserve et alloue les réserves ;
- `MonetaryAuthority` : propose ou applique la politique d’émission ;
- `SettlementAuthority` : confirme les transferts ;
- `Auditor` : observe les soldes et preuves ;
- `Judge` : traite les contestations ;
- `Office` : reçoit une allocation de travail ;
- `Agent` : dépense selon mandat ;
- `Market` : découvre un prix ou une demande, mais ne fabrique pas le fait comptable.

Le même agent ne doit pas :

- émettre la monnaie ;
- s’attribuer la récompense ;
- certifier seul son travail ;
- régler le transfert ;
- juger sa contestation.

## 6. Cartographie CYNIC à conserver

- E-Score : réputation et identité publique, non-transférable par défaut ;
- AgentReliability : fiabilité contextuelle d’une incarnation ou d’un métier ;
- EnergyBudget : droit de consommation, non transférable automatiquement ;
- CYD Stage 1 : unité interne de compte ;
- `$CYNIC` Stage 3 éventuel : actif de règlement public distinct ;
- SOL/USDC : actifs externes, réserves ou moyens de settlement ;
- Stake : garantie exposée au risque, pas mesure de compétence ;
- Proof : condition d’émission ou de règlement, jamais monnaie elle-même.

## 7. Décision actuelle et questions à résoudre

Décision actuelle : observer les phases de calibration avant de fixer la géométrie de distribution du Genesis. Le premier scénario de référence est celui d’un premier citoyen humain souverain possédant plusieurs machines, agents et ressources. Cela ne fusionne pas ces objets :

- le citoyen humain porte la souveraineté constitutionnelle ;
- les machines sont des nœuds ou fournisseurs de capacité observée ;
- les agents sont des travailleurs/office-holders recevant des mandats ;
- les workloads consomment des quotas bornés ;
- le trésor conserve les comptes, réserves et créances ;
- la gouvernance alloue une capacité, elle ne crée pas l’énergie physique.

Invariant monétaire : l’énergie physique ne peut pas être imprimée. Un CYD pré-Genesis peut être créé uniquement comme crédit comptable adossé à une contribution ou une allocation explicitement prouvée ; il ne constitue pas encore une monnaie publique redeemable. `EnergyBudget` est un droit de consommation, pas un solde CYD. Une autorisation de compute ne doit pas être confondue avec une émission monétaire.

Même dans une gouvernement à un seul citoyen, l’émission, l’exécution, la mesure et la certification doivent rester des fonctions séparables. Le même processus ne doit pas pouvoir émettre un crédit, certifier seul son travail et régler sa propre récompense.

1. CYD est-il une simple unité de compte ou une créance redeemable ?
2. La référence au Wh est-elle physique, comptable ou économique ?
3. Qui peut émettre, brûler, transférer et contester ?
4. La monnaie rémunère-t-elle un travail, une preuve, une capacité réservée ou une valeur publique ?
5. Les budgets agentiques peuvent-ils être convertis en CYD ?
6. Quel est le régime en cas de mauvaise mesure, fraude, fork ou perte de réserve ?
7. Quel rôle exact Solana joue-t-elle : settlement, réserve, émission, marché ou simple adapter ?
8. Quelle politique protège CYNIC contre l’inflation, la déflation excessive, le run et la concentration ?

## 8. Conclusion

CYNIC a déjà une imagination monétaire riche, mais pas encore une constitution monétaire cohérente. Le premier travail n’est pas de choisir une supply ou un token. C’est de séparer mesure, créance, monnaie, réputation, stake et autorité, puis de faire progresser le système par stages observables.

Le stage 1 — unité de compte interne non transférable, adossée à des preuves de travail et de ressources — est le point de départ le moins dangereux. Toute émission publique ou tout settlement Solana doit rester un stage ultérieur, après preuves de ledger, de mesure et de gouvernance.