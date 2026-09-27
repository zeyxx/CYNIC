# CYNIC E-Score : Spécification v0.1

**Status :** Première itération — doctrine en construction.
**Dernière mise à jour :** 2026-07-15

Ce document formalise le moteur E-Score de l'écosystème CYNIC/Odysseum.
Il succède aux descriptions en prose de `ODYSSEUM_GAMIFICATION.md` et
`STOCKS_ON_SOLANA_GROUND_TRUTH.md`. Il ne remplace pas ces documents mais
en extrait les règles dans une forme défendable et évolutive.

---

## 1. Philosophie

Le E-Score n'est pas un score de richesse. C'est un CV comportemental
certifié cryptographiquement.

Son axiome fondateur est Cynique : **la vertu se prouve par l'acte,
pas par le statut ou la déclaration.**

Un bot peut acheter des tokens. Il ne peut pas simuler six mois de
drawdown assumé, une prédiction contra-majorité correcte, ou la décision
irrationnelle de donner du gas à un inconnu.

Le E-Score mesure l'irréductible humain dans l'on-chain.

---

## 2. Architecture en Couches

```
Layer 1 : Stats Primaires       (7 dimensions mesurables)
Layer 2 : Achievements          (jalons permanents, SBT)
Layer 3 : Facette / Classe      (archétype émergent des stats)
Layer 4 : Réputation de Faction (score local par communauté)
```

Les couches sont indépendantes mais s'influencent mutuellement.

---

## 3. Les 7 Stats Primaires : Ancrage à la Réalité 2026

Le E-Score n'est pas abstrait. Il est calibré pour capturer la réalité brutale des tranchées crypto (Crypto Twitter / Solana Memecoin Meta) en juillet 2026 : un environnement saturé de bots MEV, de wash trading, de cabales Pump.fun, de KOLs prédateurs, et de cycles d'attention où un token naît et meurt en 3 heures.

Chaque stat (0 à 1000) filtre ce bruit pour isoler le signal humain et l'engagement organique.

---

### 3.1 KARTERIA — Endurance

> *"Tu as tenu quand la cabale a dump."*

Mesure la résilience face à l'hyper-volatilité et à la manipulation de marché.

**Signaux (Ancrage 2026) :**
- **CTO Survival :** Ne pas vendre lors d'un "Dev Dump" ou "Rug Pull" brutal (drawdown >= 80%) et participer activement au CTO (Community Take Over).
- **Hold Duration :** Durée de conservation d'un token face à des chandelles rouges extrêmes, sans céder à la panique.
- Ancienneté du wallet (non-transférable).
- Engagement continu dans un vault Odysseum.

**Decay :** Très lent. Les faits historiques ne disparaissent pas.

---

### 3.2 PARRHESIA — Vérité Courageuse

> *"Tu as call le rug avant tout le monde."*

Mesure la capacité à identifier la vérité là où la masse (ou les KOLs) se trompe ou manipule.

**Signaux (Ancrage 2026) :**
- **Scam Detection :** Identifier publiquement un lancement "bundled" ou un scam KOL sur X/Twitter *avant* que le dump n'ait lieu (détecté via le quorum proxy).
- **Fade the Consensus :** Prendre une position (on-chain ou prédiction) strictement inverse à la méta shillée par les influenceurs.
- Précision des prédictions futarchiques contra-minoritaires (<=15% du capital).

**Decay :** Très lent. Une prédiction correcte est un fait historique.

---

### 3.3 ANAIDEIA — Audace Éhontée

> *"Tu étais là avant la bonding curve."*

Mesure la volonté de prendre le risque maximal au moment de l'incertitude absolue.

**Signaux (Ancrage 2026) :**
- **First Caller :** Être le premier à tweeter ou signaler un token sur Telegram avant qu'il ne dépasse 50% de sa bonding curve Pump.fun.
- **Graduation LP :** Fournir de la liquidité (LP) dans les toutes premières minutes d'une graduation Raydium/Meteora.
- Acheter et soutenir un CTO (Community Take Over) dans ses 48 premières heures critiques.

**Decay :** Modéré. L'audace passée compte, mais nécessite d'être rafraîchie.

---

### 3.4 PHILANTHROPIA — Don

> *"Tu as payé le DexScreener."*

Mesure le sacrifice financier direct pour la survie de la communauté, sans retour sur investissement garanti.

**Signaux (Ancrage 2026) :**
- **Curve Financer :** Financer les frais de graduation d'une bonding curve pour propulser le token communautaire sur Raydium.
- **DexScreener Update :** Payer (en SOL) depuis son wallet personnel pour les mises à jour sociales (DexScreener, BirdEye) du token CTO.
- Donations de gas directes à des inconnus de la communauté via GASdf / Kora Relayer.

**Decay :** Rapide. La philanthropie est une pratique continue, pas un fait isolé.

---

### 3.5 AUTARKEIA — Souveraineté

> *"Ton node fait tourner la communauté."*

Mesure la contribution à l'infrastructure décentralisée qui protège la communauté des censures et des pannes.

**Signaux (Ancrage 2026) :**
- **Device-Scale Observer :** Faire tourner le proxy `cynic-observer` localement pour alimenter la vue sociale fédérée de la communauté.
- Opérer un nœud RPC Solana privé partagé avec la communauté pour éviter les congestions réseau.
- Déployer des bots Telegram open-source ou des outils anti-MEV pour la communauté (prouvé via GitHub/On-chain).

**Decay :** Rapide. Un proxy offline ou un nœud RPC tombé fait chuter la stat instantanément.

---

### 3.6 POIESIS — Création

> *"Tu forges les mèmes qui nous font exister."*

Mesure la production de "Attention Capital" (la ressource la plus rare en 2026) via l'art, le code et les mèmes.

**Signaux (Ancrage 2026) :**
- **Meme Engineering :** Création de vidéos/médias originaux qui génèrent un engagement organique ultra-qualitatif sur X (non-boté, vérifié par les proxies communautaires).
- Déployer un smart contract analytique utile (ex: tracker de cabales de dev).
- NFT communautaire minté ayant généré un volume secondaire organique.

**Decay :** Modéré. La culture est vivante ou elle meurt.

---

### 3.7 KOINONIA — Communauté

> *"Tu as organisé le raid."*

Mesure la capacité à fédérer, organiser et protéger l'espace humain de la communauté.

**Signaux (Ancrage 2026) :**
- **Raid Leader :** Organiser et mener des raids organiques (X, Telegram) documentés et vérifiés par l'observateur local (sans utilisation de bots de raid).
- Agir comme modérateur bénévole actif (Telegram/Discord) pendant la période critique d'un CTO.
- Référrals traçables (nouveaux wallets onboardés avec survie et activité prouvées).

**Decay :** Rapide. L'absence d'un modérateur ou d'un leader se fait sentir immédiatement.

---

## 4. Modèle de Decay Bifurqué

Le E-Score distingue deux classes de contribution fondamentalement différentes.

### 4.1 Preuves Historiques — Permanent (SBT-ancré)

Certains faits comportementaux sont irréversibles. Ils ne peuvent pas être
effacés par l'inactivité. Ils alimentent le score de base permanent.

Exemples :
- Avoir survécu à un drawdown >= 80% pendant >= 6 mois → permanent
- Avoir eu une prédiction contra-minoritaire correcte → permanent
- Avoir fourni de la LP Day-1 → permanent
- Avoir déployé un outil utilisé par la communauté → permanent

Ces preuves sont ancrées dans un Achievement SBT (voir §6).

### 4.2 Présence Active — Décroissante

| Stat          | Decay Rate     | Raison                                         |
|---------------|----------------|------------------------------------------------|
| KARTERIA      | Très lent      | L'endurance passée reste une vérité            |
| PARRHESIA     | Lent           | La vérité historique ne s'efface pas           |
| ANAIDEIA      | Modéré         | L'audace sans expression récente s'érode       |
| PHILANTHROPIA | Modéré-rapide  | Le don doit être une pratique                  |
| AUTARKEIA     | Rapide         | L'infra offline = mort immédiate du signal     |
| POIESIS       | Modéré         | La culture stagnante perd sa force             |
| KOINONIA      | Modéré-rapide  | La communauté exige une présence réelle        |

**Formule générale :**
```
Stat_effective = Stat_base_permanente + Stat_presence × e^(-λ × jours_inactifs)
```
où λ est le taux de decay propre à chaque stat.

---

## 5. Le Score Combiné

### 5.1 E-Score Global (Solana-wide)

```
E-Score = Σ(wi × Stat_effective_i) × Quadratic_Modifier × Achievement_Bonus
```

**Pondérations indicatives (v0.1 — sujettes à calibration) :**

| Stat          | Poids (wi) |
|---------------|------------|
| KARTERIA      | 0.20       |
| PARRHESIA     | 0.18       |
| ANAIDEIA      | 0.15       |
| PHILANTHROPIA | 0.12       |
| AUTARKEIA     | 0.18       |
| POIESIS       | 0.10       |
| KOINONIA      | 0.07       |

**Quadratic Modifier (anti-cartel) :**

Au-delà d'un seuil de concentration, les rendements marginaux sont décroissants.
Inspiré de la logique Quadratic Voting :
```
QM = sqrt(raw_score / max_score_reseau)
```
L'ancienneté du wallet — non-transférable et non-achetable — agit comme
multiplicateur supplémentaire, inaccessible à un attaquant récent.

### 5.2 E-Score de Faction (communauté locale)

```
E-Score_faction = E-Score_global × (1 + Activite_locale_normalisee × 0.5)
```

Un wallet globalement fort mais inactif dans $BONK aura un E-Score $BONK
réduit. Un wallet plus faible globalement mais très engagé dans $BONK peut
surpasser un OG passif sur ce marché local.

Analogie MMORPG : ton niveau mondial vs. ta réputation de faction.
*Exalted worldwide, Neutral with Thunderbluff.*

---

## 6. Achievements — Les Jalons Permanents

### 6.1 Mécanique

Un Achievement est déclenché quand une combinaison de conditions on-chain
est satisfaite de façon irréversible. Il produit deux artefacts distincts :

1. Un **SBT "proof"** (non-transférable) → met à jour le profil de stat
   permanente et le Character Sheet du wallet. Ne peut pas être acheté.
2. Un **NFT visuel optionnel** (tradable) → le "cosmétique" du badge.
   Séparé de la preuve de mérite. N'augmente pas le E-Score.

Cette séparation garantit que l'identité ne peut pas être achetée —
seule l'esthétique peut l'être.

### 6.2 Achievements Baseline (universels sur Odysseum)

| Badge               | Stat          | Condition                                                         |
|---------------------|---------------|-------------------------------------------------------------------|
| Diamond Hands       | KARTERIA      | Drawdown >= 80%, hold >= 6 mois sans vente                        |
| Rekt Scholar        | KARTERIA      | Drawdown >= 80%, hold >= 6 mois, position finalement positive     |
| Futarchy Autist     | PARRHESIA     | Prédiction contra-minoritaire correcte (<=15% du capital total)   |
| Minority Report     | PARRHESIA     | 5 prédictions contra-minoritaires correctes                       |
| Liquidity Kamikaze  | ANAIDEIA      | LP fournie Day-1 (<=24h) sur un vault Odysseum                    |
| Gas Martyr          | PHILANTHROPIA | >=10 donations gas à wallets distincts via GASdf                  |
| Ignis Devotee       | PHILANTHROPIA | >= X tokens brûlés volontairement (seuil à calibrer)              |
| Node Guardian       | AUTARKEIA     | >= 30 jours uptime node RPC prouvé on-chain                       |
| CTO Veteran         | KOINONIA      | Participation active et documentée à >= 1 CTO on-chain            |
| Culture Maker       | POIESIS       | NFT communautaire avec volume secondaire prouvé post-mint         |
| Bridge Walker       | KOINONIA      | Actif dans >= 5 communautés Odysseum simultanément                |
| First Blood         | ANAIDEIA      | Premier wallet à staker sur un marché futarchique à sa création   |
| Franchise Pioneer   | KOINONIA      | Accord de franchise M&A approuvé + revenus routés on-chain prouvés|

### 6.3 Format d'un Achievement Custom (par communauté)

```json
{
  "id": "community_achievement_v1",
  "community_mint": "<token_mint_address>",
  "name": "...",
  "description": "...",
  "stat": "KARTERIA | PARRHESIA | ANAIDEIA | PHILANTHROPIA | AUTARKEIA | POIESIS | KOINONIA",
  "conditions": [
    { "type": "on_chain | attestation | api", "tier": "1 | 2 | 3", "params": {} }
  ],
  "evidence_tier": "1 | 2 | 3",
  "scope": "faction | global",
  "sbt": true,
  "nft_visual": "true | false",
  "approved_by": "<futarchy_proposal_id>"
}
```

---

## 7. Système de Facettes (Classes Émergentes)

La facette n'est pas choisie — elle émerge de la distribution des stats.
Elle est recalculée périodiquement (candidat : 14 jours).

Les seuils sont calculés en **percentile réseau**, pas en valeur absolue.
Cela garantit que les classes restent rares et méritées, même si l'ensemble
du réseau monte en niveau.

| Facette     | Condition de déclenchement                                       |
|-------------|------------------------------------------------------------------|
| Spartiate   | KARTERIA >= 70e pct + ANAIDEIA >= 70e pct                        |
| Oracle      | PARRHESIA >= 70e pct + AUTARKEIA >= 70e pct                      |
| Prométhée   | ANAIDEIA >= 70e pct + PHILANTHROPIA >= 70e pct                   |
| Hermès      | POIESIS >= 70e pct + KOINONIA >= 70e pct                         |
| Dédale      | AUTARKEIA >= 70e pct + POIESIS >= 70e pct                        |
| Socrate     | KOINONIA >= 70e pct + PARRHESIA >= 70e pct                       |
| Agathon     | PHILANTHROPIA >= 70e pct + KOINONIA >= 70e pct                   |
| Odysseus    | >= 5 stats au-dessus du 60e percentile réseau simultanément      |

Un wallet peut appartenir à plusieurs facettes si plusieurs seuils sont
atteints. La facette d'affichage principale est celle avec la combinaison
la plus forte (score de dominance).

---

## 8. Système de Registres (Skins Communautaires)

Le moteur E-Score est universel. La sémantique est skinnable.

Chaque communauté choisit son registre au moment de son setup Odysseum
via une proposition futarchique initiale. Les stats restent
mathématiquement comparables cross-communauté. L'identité culturelle est
intraduisible — un "Spartiate" CYNIC n'est pas un "Berserker" Nordic même
si leurs scores sont identiques.

### Registres disponibles (v0.1)

| Registre    | Vibe                            | Classe max       | Usage naturel                        |
|-------------|---------------------------------|------------------|--------------------------------------|
| CYNIC       | Philosophie souverainiste        | Odysseus         | CYNIC World Order, Odysseum natif    |
| CYPHERPUNK  | Hacker / Privacy / Résistance    | Sovereign        | Token politique, Chat Control, Aura  |
| NORDIC      | Viking / Guerrier                | Konungr          | Communautés maximalistes             |
| DEGEN       | Crypto natif / Accessible        | Sovereign Fren   | Pump.fun survivors, onboarding masse |
| BUSHIDO     | Japonais / Anime                 | Daimyo           | Asian Pacific market, anime tokens   |
| GNOSTIC     | Mission / Résistance spirituelle | Patriarch        | Communautés à cause (freedom, etc.)  |
| CUSTOM      | Défini par gouvernance           | Défini           | Méga-communautés identité unique     |

### Registre CYPHERPUNK (détail)

Particulièrement pertinent pour le token Chat Control / CYNIC World Order :

```
Stats    : Patience · Signal · Exploit · Relay · Daemon · Commit · Mesh
Classes  : Ghost · Zero-Day · Relayer · Sysadmin · Contributor · Sovereign
Vibe     : "Code is Speech." Le registre qui transforme le badge en statement politique.
```

### Registre DEGEN (détail)

Le plus accessible pour onboarder des communautés Pump.fun :

```
Stats    : Diamond · Alpha · Chad · Based · Node · Cook · Fren
Classes  : Diamond-Handed · Alpha Caller · Based Degen · Node Lord · Content Cook · Sovereign Fren
Vibe     : Parle la langue de la rue crypto. Zéro friction culturelle.
```

---

## 9. Modèle Oracle Hybride (Tiers d'Evidence)

| Tier | Type                      | Poids | Description                                                     |
|------|---------------------------|-------|-----------------------------------------------------------------|
| 1    | On-chain pur              | 100%  | Vérifiable directement sur Solana. Aucune confiance externe.   |
| 2    | Attestation communautaire | 60%   | >= 3 wallets qualifiés signent. Fenêtre de contestation 7j.    |
| 3    | Oracle API externe        | 30%   | Twitter, GitHub. Hash ancré on-chain. Decay accéléré.          |
| 4    | Attestation solo          | 10%   | Invalide seul. Uniquement en complément d'un signal T1 ou T2.  |

**Règle d'or :** Un Achievement ne peut jamais être déclenché par
des signaux Tier 3 ou 4 seuls. Tier 1 ou 2 obligatoire.

---

## 10. L'OIKONOMIA — Émergence, Pas Stat

La valeur économique créée pour une communauté n'est pas une stat
indépendante à farmer. Elle émerge comme bonus automatique quand la
combinaison d'autres comportements produit un résultat économique prouvé.

```
Exemple A — Futarchie productive :
  PARRHESIA (proposition passée en futarchie)
    × ANAIDEIA (capital engagé sur le marché)
    × résultat on-chain (vault +X% depuis la proposition)
  → OIKONOMIA Bonus automatique sur le E-Score de Faction

Exemple B — Franchise M&A :
  KOINONIA (accord de franchise soumis et accepté)
    × revenus on-chain routés vers le vault parent (prouvés)
  → OIKONOMIA Bonus + Achievement "Franchise Pioneer" SBT
```

L'OIKONOMIA reste invisible comme stat mais se manifeste comme :
- Multiplicateur du E-Score de Faction
- Condition de déblocage d'achievements rares
- Signal de réputation dans le Survivor Intake d'Odysseum

*Design principle : on ne récompense pas l'intention, on récompense le résultat.*

---

## 11. Tiers d'Accès (E-Score Gating sur Odysseum)

| Tier | Nom       | E-Score     | Accès                                                      |
|------|-----------|-------------|------------------------------------------------------------|
| 0    | Visiteur  | < 100       | Lecture seule, pas de vault                                |
| 1    | Citoyen   | 100 – 499   | Vaults standards, vote sur propositions existantes         |
| 2    | Steward   | 500 – 999   | Soumettre des propositions, vaults avancés                 |
| 3    | Archonte  | 1000 – 2499 | Vaults à haut rendement, dark pools                        |
| 4    | Souverain | >= 2500     | Accès complet, relayer premium, gouvernance méta-protocole |

Tier 0-1 : seuils en valeur absolue (accessibilité universelle).
Tier 3-4 : seuils en percentile réseau (rareté garantie à l'échelle).

---

## 12. Questions Ouvertes (v0.1)

1. **Calibration des poids (wi) :** nécessite des données réelles du réseau.
   Les pondérations actuelles sont des hypothèses de travail.

2. **Taux de decay (λ) par stat :** à affiner après les premiers mois mainnet.

3. **Seuil de qualification Tier 2 :** quel E-Score minimum pour qu'un
   wallet puisse attester le comportement d'un autre ?

4. **Privacy-E-Score compatibility :** comment préserver la confidentialité
   des wallets utilisant Aura Protocol tout en maintenant la vérifiabilité
   du E-Score ? Candidat : ZK-proof du score sans révéler le wallet.

5. **Portabilité cross-chain :** le E-Score reste-t-il Solana-only ou peut-il
   ingérer des signaux d'autres chaînes via un bridge d'attestation ?

6. **Migration de wallet :** comment le E-Score suit-il un wallet qui migre
   vers un nouveau programme ? Mécanisme de transfer d'identité prouvé ?

7. **Registres custom :** quel est le processus de validation qu'une
   communauté doit suivre pour créer et faire approuver un registre custom ?

---

## 13. Références

- [`ODYSSEUM_GAMIFICATION.md`](./ODYSSEUM_GAMIFICATION.md)
- [`CYNIC_TOKENOMICS_ARCHITECTURE.md`](./CYNIC_TOKENOMICS_ARCHITECTURE.md)
- [`CYNIC_MACRO_RED_TEAM.md`](./CYNIC_MACRO_RED_TEAM.md)
- [`CYNIC_COSMOLOGY_AND_IGNIS.md`](./CYNIC_COSMOLOGY_AND_IGNIS.md)
- [`STOCKS_ON_SOLANA_GROUND_TRUTH.md`](../../internal/STOCKS_ON_SOLANA_GROUND_TRUTH.md)
- [`ODYSSEUM_MANIFEST.md`](../ODYSSEUM_MANIFEST.md)
