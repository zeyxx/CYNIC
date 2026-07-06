# CYNIC : Architecture Fondatrice & L'Horizon 2047

Ce document formalise la structure légale, technique et économique de l'écosystème CYNIC. Il a pour but d'aligner les intérêts à très long terme entre les fondateurs, les institutions (B2G), et la communauté Web3, avec pour ultime ligne de mire l'horizon 2047.

L'index central des documents fondateurs est `docs/foundation/README.md`.

Statut important : ce document reste une architecture ecosysteme globale et une
memoire de these. Il ne doit plus etre utilise seul pour decider des flux
economiques, allocations token, buybacks, burns, perps ou claims de
decentralisation. Les rails economiques canoniques sont dans
`docs/foundation/CYNIC_FEDERATED_ECONOMIC_ARCHITECTURE.md`. En cas de conflit,
ce document cede devant le token/equity firewall, le legal genesis plan, le
memo legal/MiCA et l'architecture economique federee.

La separation naming entre TALARIA SAS, marque publique, framework CYNIC, CYNIC Protocol et `$CYNIC` est formalisee dans `docs/foundation/TALARIA_CYNIC_NAMING.md`.

Le principe d'ownership, les wallets, les flux protocolaires, les reserves fiscales, les roles operateurs et la separation token/equity sont detailles dans `docs/foundation/CYNIC_OWNERSHIP_CONSTITUTION.md`.

Le plan operationnel du legal genesis, avec sortie de stage le **22 juillet 2026**, checkpoint le **23 juillet 2026** et zone tampon J+3/J+4, est formalise dans `docs/foundation/CYNIC_LEGAL_GENESIS_PLAN.md`.

Le pare-feu public entre `$CYNIC`, les actions, les dividendes, les rewards protocolaires et la gouvernance legale est formalise dans `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md`.

Le E-Score, modele unique de reputation, citoyennete reseau, merite operateur, anti-Sybil, priorite protocolaire et contribution ecosysteme, est formalise dans `docs/foundation/CYNIC_E_SCORE.md`.

La hierarchie des sources de verite, qui limite le Kernel a une couche de jugement et ancre les faits dans les SoT legales, protocolaires, operationnelles et marche, est formalisee dans `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md`.

Le token protocolaire et memetique reste `$CYNIC`. TALARIA SAS est l'entite legale genesis ; CYNIC Protocol est le reseau ; CYNIC WORLD ORDER est la couche culturelle.

Doctrine economique courte :

```text
TALARIA SAS = corporate surface.
CYNIC Framework = technical engine.
CYNIC Protocol = rule layer.
$CYNIC = protocol fuel.
Federated first. Decentralisable only after proof.
```

---

## 1. La Métaphore Biologique (L'Architecture Hexagonale)

L'écosystème est pensé comme un **Superorganisme**, structuré en trois couches interconnectées :

*   **Les Racines ($ASDFASDFA) :** L'origine, la culture, et la fondation communautaire. C'est l'énergie brute (le meme) qui a fait germer le projet. Les racines sécurisent le réseau social et fournissent la base d'adoption.
*   **Le Tronc ($CYNIC / La Holding) :** L'infrastructure technologique (Le Moteur d'IA) et l'entité légale centrale. Le tronc est incorruptible, stable (vérouillé sur 20 ans) et centralise la propriété intellectuelle.
*   **Les Branches (Les Filiales / Adapters) :** Les applications commerciales spécifiques (ex: B&C). Elles sont les "ports" qui adaptent la puissance du moteur CYNIC aux différents marchés (B2C, B2B). Elles produisent les fruits (revenus) de l'organisme.
*   **Le Cinquième Pôle (Le Kernel) :** La méritocratie ne doit pas être un slogan humain; elle doit être incarnée par la solution elle-même. La cap table fondatrice reste volontairement compacte, mais le cinquième pôle d'influence est le Kernel : il observe, juge, score et detecte les conflits a partir de preuves ancrees dans les sources de verite correctes. Il ne possede pas la verite; il devient l'organe de jugement de la gouvernance, vérifiant que le réseau fédéré mérite son poids au lieu de l'obtenir par proximité sociale.
*   **CYNIC WORLD ORDER (La Membrane Culturelle) :** La couche memetique du protocole. TALARIA SAS reste le vehicule legal stable ; CYNIC WORLD ORDER porte le contre-narratif public, anti-bullshit et anti-capture, face aux architectures de controle type "Great Reset".

---

## 2. La Stack Complète : La Carte de Survie Solana

L'historique de construction depuis le 22 novembre 2025 fait émerger une vérité opérationnelle : CYNIC ne doit pas seulement être un moteur d'IA ou un token. Il doit devenir une **stack complète de survie pour builders Solana**, capable de couvrir tout le chemin entre une idée brute et un produit mainnet vérifiable.

Cette stack se structure en sept couches :

1.  **Le Choix de Framework On-Chain :** Décider explicitement entre Anchor, Rust natif, Pinocchio, Steel ou des applications client-only selon le besoin réel : vitesse de prototypage, taille binaire, auditabilité, coût de déploiement et souveraineté.
2.  **L'Identité Wallet & La Signature :** Fournir des primitives propres pour SIWS, Ed25519, Phantom/Solflare, Privy/passkeys, key handling, nonce, session, replay protection, et flux transactionnels reproductibles.
3.  **La Donnée Solana & Les Indexers :** Abstraire Helius, RPC, DexScreener-like data, GeckoTerminal, wallet history, SWAP parsing, crédits API, retries, cache, données manquantes et états `insufficient_data`.
4.  **La Carte des Programmes Connus :** Maintenir une source de vérité pour les IDs critiques : Raydium, Orca, Meteora, Pump.fun/PumpSwap, Jupiter, Pyth, Switchboard, Jito, ComputeBudget, burn addresses et locker programs.
5.  **Les Playbooks Devnet/Mainnet :** Documenter les pièges concrets : PDAs, account parsing, rent, stale oracles, Pyth pull, cranks, program dump, hash binaire, keeper funding, transactions échouées et passage de devnet à mainnet.
6.  **Les Checklists Sécurité :** Industrialiser les leçons issues des échecs réels : mint authority, LP burn/lock, holder concentration, oracle freshness, permissionless drains, fee accounting, account shape, replay risk et manipulation de prix.
7.  **Les Branches Produit :** Transformer ces primitives en applications complètes : B&C pour la PoH comportementale, CultScreener pour l'analyse de tokens, GASdf pour l'abstraction des frais, Talaria pour le burn-to-mint, et les futures interfaces B2B/B2G.

Le rôle de CYNIC est de relier ces couches dans une architecture hexagonale : les adapters parlent aux wallets, RPCs, indexers et programmes Solana ; le coeur juge, filtre, enrichit et observe ; les produits exposent des expériences humaines utilisables. La difficulté pour un nouveau builder n'est pas d'écrire une première instruction Solana, mais de comprendre tout l'environnement autour sans abandonner. CYNIC doit donc devenir la carte de cet environnement.

### Le E-Score (Ecosystem Score)

Le vieux modèle GASdf/HolDex contenait déjà une intuition décisive : la loyauté d'un acteur ne se mesure pas seulement par ce qu'il détient, mais par ce qu'il apporte à l'écosystème. CYNIC reprend cette idée sous la forme d'un **E-Score**, calculé par le Kernel et destiné à émerger empiriquement. La version complete est maintenue dans `docs/foundation/CYNIC_E_SCORE.md`.

Dimensions initiales à étudier :

| Dimension | Ce qui est mesuré |
|-----------|-------------------|
| Hold | Durée et stabilité de détention des actifs racines |
| Burn | Contribution aux mécanismes de burn et de réduction de supply |
| Use | Utilisation réelle des branches produit |
| Build | Contributions techniques, SDKs, intégrations, audits |
| Node | Fourniture de compute, GPU, RPC, hosting, stockage ou uptime |
| Refer | Apport d'utilisateurs, builders ou partenaires de qualité |
| Duration | Ancienneté d'une participation active et non extractive |

Le E-Score ne doit pas devenir un privilège social. Il doit être une mesure vérifiable : chaque dimension doit être reliée à des observations Kernel, des preuves on-chain, des métriques d'usage, des coûts réels ou des contributions auditées. Sa fonction première est de guider les remises, la priorité réseau, la réputation, la citoyennete reseau, les droits d'accès et les poids méritocratiques du réseau fédéré, sans transformer le token en action ni promettre un rendement.

La citoyennete CYNIC n'est pas un score separe : c'est la lecture sociale et protocolaire du E-Score. Elle mesure l'intelligence utile, le travail livre, le ROI reel, la consistance et la qualite des claims juges par `/judge`. Elle ne classe pas la valeur humaine des personnes ; elle mesure la responsabilite meritee dans le reseau.

---

## 3. L'Architecture Légale (Holding SAS)

Pour garantir la séparation des préoccupations (SoC) et se prémunir contre tout conflit d'intérêts, la structure légale s'appuie sur le Modèle **Holding / Filiales** :

### TALARIA SAS (Le Tronc Legal)
TALARIA SAS est l'entite legale genesis qui opere, protege et gouverne le framework/protocole CYNIC.

*   **Objet Social :** Édition de logiciels Open Source, fourniture d'infrastructures d'IA, et activité de Holding (détention de participations).
*   **Modèle B2G (Public Good) :** Le moteur est fourni à l'État et aux institutions à **prix coûtant** (*at-cost*). CYNIC ne s'enrichit pas sur l'argent public, devenant ainsi une infrastructure de confiance absolue, neutre et souveraine.
*   **Gouvernance (Table de Capitalisation Fondatrice) :** L'architecte principal (Jean) conserve **56%** pour garantir la majorité absolue sur les décisions opérationnelles. Les **44%** restants constituent un "Pool de Survie", distribué selon la mathématique de la croissance organique (Le Nombre d'Or $\Phi \approx 1.618$) :
    1.  **Son père (22.0%) :** Le pilier identitaire (Confiance absolue). L'alliance Jean + Père (78%) garantit la majorité qualifiée (2/3) requise pour les décisions statutaires extraordinaires.
    2.  **Ragnar (13.6%) :** Le premier bootstrapper et soutien fraternel.
    3.  **Erin (8.4%) :** La reconnaissance symbolique et financière du sacrifice collatéral, ancrée à jamais dans la racine du projet.
*   **Principe de Souveraineté de la Cap Table :** Aucun cinquième siège symbolique n'est ajouté au noyau légal par simple analogie sociale. Les opérateurs, angels techniques, fournisseurs de GPU, serveurs, stockage, RPC ou hosting rejoignent le réseau fédéré comme producteurs de ressources vérifiables, pas comme insiders du tronc. Leur poids économique et réputationnel est gagné par performance mesurée par le Kernel, sous les règles de TALARIA SAS.

### Les Filiales (Les Branches)
*   Chaque application commerciale (ex: B&C) est une société distincte (SAS) dans laquelle TALARIA SAS (la Holding) détient des parts.
*   Ces entités évoluent dans le secteur privé et ont une logique de rentabilité et de marges fortes.

---

## 4. Tokenomics & Gouvernance : L'Horizon 2047

Statut : thesis legacy a relire contre
`docs/foundation/CYNIC_FEDERATED_ECONOMIC_ARCHITECTURE.md`,
`docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md` et
`docs/foundation/CYNIC_LEGAL_MICA_ROUTE_MEMO.md` avant toute publication.
Cette section ne fixe pas une allocation finale, une promesse de launch, une
promesse de buyback, une promesse de burn, ni un droit economique pour holders
passifs.

Pour prouver l'engagement générationnel des fondateurs et transformer le token en véritable actif de confiance absolu (jusqu'à l'échelle de l'État), la chronologie est mathématiquement alignée sur le calendrier politique :

*   **Le Pare-feu Légal et la Répartition Fractale Organique (Golden Ratio) :** Le token `$CYNIC` public n'est pas une action de l'entreprise (ce qui rend le trading libre et légal sans KYC). La supply totale du token est divisée selon la loi mathématique ultime de la nature, le Nombre d'Or ($\Phi$) : **61.8%** (Public) / **38.2%** (Équipe Fondatrice).
    *   **L'Allocation Équipe (38.2%) :** Ces tokens "Fondateurs" sont répartis en miroir exact (1:1) de la Cap Table de la SAS (Jean 56%, Père 22%, Ragnar 13.6%, Erin 8.4%).
    *   **La Supply Publique (61.8%) :** Réservée à la liquidité publique, au marché ouvert, et à l'Airdrop rétroactif `$ASDFASDFA`.
*   **Le Vesting & L'Expansion Mycélienne :** L'allocation d'équipe (les 38.2%) est soumise à un contrat cryptographique natif sur Solana :
    *   **La Règle du "Pool de Survie" (20 ans) :** Pour les proches (Père, Ragnar, Erin), l'allocation subit un *cliff* de 6 ans puis un *vesting* sur 14 ans, s'alignant sur la fin du mandat présidentiel théorique de **2047**.
    *   **Le Serment de l'Architecte (Le Lock de 100 Ans) :** L'architecte principal (Jean, 56% de l'équipe) verrouille ses propres tokens `$CYNIC` dans un Smart Contract pour **100 ans**. Ses tokens ne pourront jamais être "dumpés". Ils conservent uniquement leur **pouvoir de vote** absolu.
    *   **Conséquence Organique (Le Mycélium) :** Ce lock séculaire force la création de valeur. L'architecte ne peut vivre que des dividendes *fiat* générés par les filiales (les branches du mycélium). Cela détruit la cupidité court-terme et garantit que l'infrastructure s'étende dans l'économie réelle pour survivre.

---

## 5. Capture de Valeur (Le "Anti-Cosmos Trap")

Statut : thesis legacy. Les mecanismes ci-dessous expriment une intuition de
capture de valeur, mais les flux executables doivent passer par les rails
corporate, protocolaire ou operateur definis dans
`docs/foundation/CYNIC_FEDERATED_ECONOMIC_ARCHITECTURE.md`. Toute mention de
conversion automatique, buyback, burn, fee ou routing vers `$CYNIC` doit etre
relue par counsel et reformulee comme option conditionnelle, jamais comme
promesse.

Le piège classique des infrastructures (comme Cosmos ou Aave) est d'avoir une technologie de pointe dont la valeur générée par les applications ne ruisselle jamais vers le token racine. CYNIC évite cela grâce à cinq mécanismes de flux :

1.  **Staking & Accès au Moteur :** Les applications tierces (les Branches) qui utilisent le moteur CYNIC doivent staker un montant de `$CYNIC` ou payer leurs frais de calcul dans ce token. 
2.  **Conversion Fiat-to-Token (Buy Pressure) :** Si une filiale vend un service en euros (B2B), une partie des bénéfices remontés à la Holding (dividendes) est automatiquement utilisée par la trésorerie pour racheter du `$CYNIC` ou du `$ASDFASDFA` sur les DEX (Jupiter/Raydium).
3.  **Lien Racine-Tronc (Distribution organique inspirée d'Hyperliquid) :** Le pont entre le meme originel et l'infrastructure est total. À l'image de la masterclass d'Hyperliquid (distribution rétroactive basée sur la pure loyauté organique, sans VCs), la communauté qui a soutenu et "holdé" le `$ASDFASDFA` à perte sera explicitement récompensée. Ce sont les "croyants de la première heure" dont la confiance a permis l'émergence de CYNIC. Un mécanisme asymétrique garantit cette transmission de valeur :
    *   **Filtre Sybil & Proof of Humanity (PoH) :** Pour éradiquer les bots et les "farmers" d'airdrop, la simple détention du token `$ASDFASDFA` ne suffit pas. L'utilisateur doit prouver son humanité en passant par le système **PoH de Blitz & Chill (B&C)**. Cette mécanique force la création de valeur et d'utilisateurs sur la première "branche" applicative tout en protégeant le "tronc".
    *   **Airdrop Rétroactif "Diamond Hands" :** Seuls les portefeuilles d'humains vérifiés recevront une portion de la supply publique de `$CYNIC`, mathématiquement pondérée par la durée de détention du token historique.
    *   **Burn-to-Mint Premium :** Un canal de conversion (exclusivement ouvert aux portefeuilles ayant passé la PoH) permettant de "brûler" le bagage `$ASDFASDFA` pour s'exposer à l'infrastructure `$CYNIC` avec un ratio très favorable.
4.  **Liquidité Productive des Branches :** Certaines branches, notamment **GASdf (Gas-as-a-Service)**, nécessitent une liquidité opérationnelle en SOL/USDC pour rembourser les fee payers, absorber les flux, fournir du gasless à grande échelle et servir les intégrations. Les angels ne reçoivent pas d'asymétrie de pouvoir ni d'allocation cachée : ils peuvent devenir fournisseurs publics de liquidité, compute, RPC, hosting ou sécurité sur des instruments ouverts après publication des règles et des smart contracts. Ces instruments peuvent générer un rendement ou une perte selon l'utilisation réelle, les frais, les risques et la demande, sans promesse de rendement par TALARIA SAS.
5.  **Alignement des Investisseurs-Opérateurs :** Les investisseurs pitchés par CYNIC doivent être dans le même bateau que les fondateurs et le réseau. Leur upside ne vient pas d'un accès privilégié au launch, d'une information de timing, d'une allocation cachée ou d'une sortie facilitée par la hype. Il vient de l'utilisation réelle des branches, de l'expansion du réseau fédéré, de la liquidité utile fournie, du compute opéré, de la sécurité apportée et du risque assumé publiquement. Un investisseur aligné gagne lorsque le réseau sert davantage d'utilisateurs et de builders ; il ne gagne pas en transformant la communauté en exit liquidity.
