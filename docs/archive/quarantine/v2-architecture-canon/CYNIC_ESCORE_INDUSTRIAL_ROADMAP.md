# ODYSSEUM E-SCORE : Roadmap Industrielle v1.0

**Statut :** Plan d'exécution validé
**Date :** 2026-07-15
**Horizon :** 12 mois vers production décentralisée (M0 à M12)

> Ce document est une roadmap d'ingénierie stricte, en adéquation avec `AGENTS.md`.
> Chaque jalon (milestone) produit des artefacts exécutables et vérifiables.
> Aucune prose ne remplace un test qui passe ou une preuve cryptographique.

---

## Principes d'exécution (Industrial Bar)

1. **Bottom-up strict.** Rien ne repose sur quelque chose d'instable. On valide le noyau Rust avant d'écrire un endpoint HTTP.
2. **Proof first.** Chaque couche produit une preuve *machine-readable* (`metrics --json`) avant d'être utilisée comme fondation.
3. **Séparation des préoccupations.** On-chain = source de vérité et ancrage. Off-chain = indexation et observation. Jamais l'inverse.
4. **Décentralisation programmée.** Talaria SAS démarre en position d'oracle *bootstrap* pour lancer la machine. Cette autorité expire de manière déterministe (hardcodée).
5. **Nœuds économiquement contraints.** Un nœud non-staké ne peut pas soumettre de signal de Tier 2 (T2). La confiance est remplacée par la contrainte économique (slashing).

---

## Vue d'ensemble des 7 couches (L0 à L6)

```text
L6  DISTRIBUTION          Docker · Nix · Harness: lab → conformance → staging → prod
L5  INTERFACE PUBLIQUE    API REST · WebSocket · MCP Server · SDK TS (@solana/kit)
L4  SCORE ENGINE          Agrégation · Decay · Facettes · Registry rendering
L3  ORACLE SOCIAL         Telegram · Twitter Pool · GitHub — réseau fédéré
L2  INDEXING PIPELINE     Helius DAS · Pump/PumpSwap classifier · Price oracle
L1  ON-CHAIN PROGRAMS     Anchor 1.1.2 : escore-state · oracle-registry · achievement-sbt
L0  SUBSTRATE KERNEL      cynic-kernel : types · schémas · tests — 0 dépendance ext.
```

**Règle d'or :** Une couche ne peut entrer en *staging* qu'après que la couche inférieure a passé son *harness* de *conformance*.

---

## L0 — Substrate Kernel

**Crate :** `crates/cynic-kernel`
**Statut cible :** `make check` vert, zéro dépendances externes, coverage 100% des types et règles de calcul.

### Livrables

1. **Types Core :** `EScoreSignal`, `EScoreComputed`, `EvidenceTier`, `Stat`, `Facette`.
2. **Registry Validator :** Charge et valide les JSON de registres (ex: `escore-registry.cynic.json`) contre le JSON Schema draft-07.
3. **Moteur de Decay & Quadratique :** Fonctions pures pour le decay exponentiel (basé sur le temps) et le modificateur quadratique anti-concentration.
4. **Merkle Trees :** Fonctions pour construire un arbre de Merkle à partir d'un set de signaux, générer et vérifier des preuves (préparation pour les ZK-proofs via Aura).

**Tests requis (Gate) :** Round-trip de sérialisation, decay strictement monotone, validateur JSON robuste contre les erreurs de schéma, vérification des preuves Merkle.
**Proof Output :** `cynic runtime metrics --json` → `kernel_coverage: 1.0`

---

## L1 — On-Chain Programs

**Stack :** Anchor 1.1.2 + Solana CLI 3.1.10
**Tests :** LiteSVM (unit) + Surfpool (intégration avec *mainnet fork* et cheatcodes)
**Client :** Codama → `@solana/kit v7` typed clients

### Les 4 Programs Anchor

1. **`escore-state` :** Stocke le *snapshot* du score par wallet (PDA: `["escore", wallet_pubkey]`). Contient le `score_root` (Merkle root) et le timestamp. N'est mis à jour que périodiquement par un crank autorisé.
2. **`oracle-registry` :** Enregistre les nœuds autorisés à soumettre des signaux T2. Gère les stakes (`node_pubkey`, `stake_amount`, `tier`, `reputation`). Talaria est `NodeTier::Bootstrap` jusqu'à M6.
3. **`signal-relay` :** Reçoit les observations signées des nœuds. Vérifie le stake du nœud, valide la signature ed25519, vérifie l'unicité (`evidence_hash`), et émet un event Anchor loggé. **Il n'agrège pas, il émet.**
4. **`achievement-sbt` :** Frappe des SBT non-transférables. Vérifie l'éligibilité via Merkle proof (`claim_achievement`). Peut être brûlé uniquement par le détenteur.

**Tests requis (Gate) :** Couverture LiteSVM complète, intégration Surfpool avec simulation de temps et balances.
**Proof Output :** `cynic runtime conformance --json --kind on-chain`

---

## L2 — Indexing Pipeline

**Crate :** `crates/cynic-node/indexer`
**Runtime :** Tokio async, Axum (webhooks), SQLx + PostgreSQL (stockage *append-only*)

### Architecture d'Ingestion

*   **Ingress :** Webhooks Helius (validation HMAC) ou Geyser gRPC.
*   **Router :** Dispatche vers les classifiers spécifiques.
*   **Classifiers :**
    *   `PumpFunClassifier` : Détecte les créations (ANAIDEIA), les trades (KARTERIA via hold duration), la graduation (PHILANTHROPIA). Filtre anti-wash-trading (24h bi-directionnel).
    *   `OdysseumClassifier` : Parse les events de `escore-state`, vaults, et proposals (Codama IDL).
    *   `KoraClassifier` : Track les donations de gas (PHILANTHROPIA).
*   **Enrichissement :** Jupiter Price API (cache en mémoire 60s) pour calculer les drawdowns.
*   **Stockage :** PostgreSQL. Table `signals` est strictement *append-only*.

**Proof Output :** `metrics --json` → latence p99 < 500ms, taux de filtrage wash-trading.

---

## L3 — Oracle Social Fédéré

**Modèle :** Réseau de nœuds décentralisés (opérés par la communauté). Pour soumettre un signal valide (T2), un nœud doit avoir staké des tokens CYNIC. Fraude prouvée = Slash.

### Le paradigme "Device-Scale Observer" (L'œil absolu local)

Au lieu de scraper via des API hostiles, CYNIC se déploie comme un nœud personnel (Personal Node) agissant comme un **Man-in-the-Middle systémique** et de gestion d'identité sur les appareils des utilisateurs stakés :

1. **Twitter / X (Capture Passive Proxy) :**
    *   Le nœud CYNIC (via `mitmproxy`) capture passivement le GraphQL pendant que l'humain navigue. Aucune requête API supplémentaire. Zéro coût, indétectable.
    *   Signaux : PARRHESIA (opinions contra-majoritaires), ANAIDEIA (premiers appels sur tokens).
    *   *Quorum requis :* N≥3 nœuds doivent observer le même tweet pour valider un signal T2 (anti-sybil).
2. **GitHub (CLI Session Identity) :**
    *   Cynic utilise le keychain local (`gh auth token` ou agent SSH) pour observer la session active de l'humain.
    *   Signaux : POIESIS (commits/PRs), AUTARKEIA (outillage).
3. **Telegram (Protocol-level Sync) :**
    *   Approche "personal cli login" (Telethon/MTProto). Cynic se connecte comme un client local de l'utilisateur, voyant les mêmes groupes/canaux privés.
    *   Signaux : KOINONIA (entraide), POIESIS.
4. **Mobile Vision (Filouteries Périphériques) :**
    *   **VPN/WireGuard local :** Le profil VPN du téléphone route le trafic X/Social vers le nœud CYNIC local (mitmproxy).
    *   **Notification Mirroring :** Interception des push notifications via l'API d'accessibilité Android ou Pushbullet/KDE Connect pour capter les signaux d'alertes en temps réel sans ouvrir l'app.
    *   **PWA / Extension Mises :** Extension injectée dans des navigateurs crypto-friendly (Kiwi, Mises) pour capter l'état du DOM sur mobile.

**Lien d'identité :** Attestation T2 (signature on-chain d'un message liant le wallet au handle social).

---

## L4 — Score Engine

**Crate :** `crates/cynic-kernel/escore/engine`
**Design :** Lazy, stateless, purement déterministe.

1.  **Récupération :** Fetch tous les signaux PostgreSQL d'un wallet.
2.  **Pondération & Decay :** Applique le poids du Tier de preuve (T1=1.0, T4=0.15), le multiplicateur de Quorum social, et le decay exponentiel (sauf pour les preuves permanentes).
3.  **Anti-baleine :** Modificateur quadratique global basé sur le max du réseau.
4.  **Application du Registre :** Rend le score selon le registre demandé (CYNIC, DEGEN, BUSHIDO, etc.) et applique les modificateurs de faction.
5.  **Classification :** Détermine le Tier d'accès (0-4) et la Facette émergente (ex: Spartiate, Oracle).
6.  **Preuve :** Génère la racine de Merkle des signaux utilisés pour ancrage optionnel.

---

## L5 — Interface Publique

**Stack :** Axum (API), TypeScript (SDK)

*   **API REST & WS :** `/v1/score/:wallet`, `/v1/score/:wallet/proof`, `/v1/achievements/:wallet`, WebSocket streams.
*   **MCP Server (`cynic-escore`) :** Expose les outils aux agents IA (`get_wallet_score`, `verify_score_proof`). Fondamental pour l'intégration avec les DAO gérées par IA.
*   **SDK TypeScript (`@cynic/escore-sdk`) :** Intégration frontend (React) basée sur `@solana/kit`. Support de la vérification de preuves côté client.

---

## L6 — Distribution & Packaging

*   **Builds Reproductibles :** Nix Flakes pour tous les binaires (Rust, Python, Node). `nix build .#cynic-indexer`.
*   **Conteneurisation :** Docker Compose pour staging et déploiement des nœuds communautaires.
*   **Harness de Conformance :** Exécution automatisée des suites de validation (Lab → Conformance → Staging → Production).

---

## Calendrier de Décentralisation (M0 - M12)

### M0 - M2 : Fondation (Talaria SAS en contrôle)
*   **M0 :** Kernel L0 complet, tests 100%. Validation du schéma JSON des registres.
*   **M1 :** Programs L1 sur Devnet. Intégration Surfpool validée.
*   **M2 :** Pipeline L2 actif avec Classifier Pump.fun. Ingestion des premiers signaux réels.
*   **Odysseum Hub :** Initialisation des packages UI (`odysseum-ui`) et câblage du SDK Solana (`@solana/kit`).
*   *Gouvernance : Talaria a 100% de l'autorité (Bootstrap).*

### M3 - M5 : Activation du Signal Social & Alpha
*   **M3 :** Déploiement des oracles Telegram (L3) en local.
*   **M4 :** Déploiement de l'observateur GitHub. Premières attestations `wallet <-> handle` on-chain.
*   **M5 :** Lancement du Device-Scale Observer (Proxy local Twitter) avec un quorum de 3 nœuds.
*   **Odysseum Hub (Alpha) :** Première interface "Character Sheet". Les utilisateurs lient leurs identités sociales et voient leurs premières stats.
*   *Gouvernance : Ouverture du `node-staking`. La communauté commence à opérer des oracles personnels.*

### M6 - M8 : MVP Public & Révocation d'Autorité
*   **M6 :** **Expiration de l'autorité Bootstrap de Talaria (hardcodée).** Passage au vote futarchique. Déploiement du Score Engine L4.
*   **M7 :** Lancement de l'API L5, du SDK, et du serveur MCP. Intégration de la Mobile Vision (Notification mirroring).
*   **M8 :** Ancrage quotidien des Merkle roots on-chain (`escore-state`). Preuves vérifiables sans API.
*   **Odysseum Hub (Beta) :** Ouverture des Factions (Communautés) et des premiers SBT Achievements (Quêtes).

### M9 - M12 : Réseau Autonome & Metaverse
*   **M9 :** Premier vote futarchique pour ajouter un Registre CUSTOM.
*   **M10 :** 10+ nœuds communautaires indépendants stakés dans `oracle-registry` capturant le trafic via proxy/mobile.
*   **M11 :** Le pool d'observation fonctionne exclusivement sur des sessions communautaires (0 session Talaria).
*   **M12 :** **Production Déclarée.** Passage du `conformance harness --kind production`. Le système est industriel, résilient et totalement décentralisé.
*   **Odysseum Hub (V1) :** Hub Gamifié complet (RPG-lite). Les E-Scores influencent la puissance de vote, l'accès aux vaults et le statut dans la MetaDAO.
