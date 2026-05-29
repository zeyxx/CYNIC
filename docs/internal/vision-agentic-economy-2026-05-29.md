<!-- RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate. -->

# Vision : CYNIC dans l'Économie Agentique

**Date :** 2026-05-29
**Auteurs :** T. + Claude

---

## Thèse

L'économie agentique (agents IA autonomes opérant dans DeFi, trading, gouvernance) crée un besoin fondamental : **une infrastructure de jugement indépendante et souveraine**.

Les agents IA peuvent exécuter, mais ils ne peuvent pas juger. Ils ont besoin d'un oracle de confiance qui :
1. Évalue indépendamment (pas de single point of failure)
2. Plafonne sa confiance (pas de faux positifs absolus)
3. Suspend son jugement quand nécessaire (EPOCHÉ)
4. Apprend de ses erreurs (cristaux, mémoire)

**CYNIC est cette infrastructure.**

---

## Le Paysage Agentique (2026)

### Agents existants

- **Trading bots** : Exécution automatique sur DEX (Jupiter, Raydium)
- **DeFi agents** : Yield farming, liquidation, arbitrage
- **Social agents** : Création de contenu, community management
- **MCP tools** : Extensions Claude/Cursor pour interagir avec des services

### Le problème de confiance

Chaque agent a besoin de répondre à :
- "Ce token est-il sûr ?" → CYNIC /judge
- "Cette opportunité est-elle réelle ?" → CYNIC /observe + crystal memory
- "Dois-je agir ou attendre ?" → HOWL/BARK/EPOCHÉ

Sans jugement, les agents sont aveugles. Ils exécutent sans filtrer.

---

## Positionnement CYNIC

### Layer de jugement (infrastructure)

```
Agent (ex: trading bot)
    │
    ├── Data Layer (KAIROS, Hyperliquid, Binance)
    │
    ├── Judgment Layer (CYNIC)  ← ICI
    │       ├── /judge  (verdict sur un stimulus)
    │       ├── /observe  (injection de signal)
    │       └── /crystals  (mémoire)
    │
    └── Execution Layer (Jupiter, DEX, wallet)
```

CYNIC ne remplace pas les agents. Il les **arme** avec du jugement.

### MCP Server

CYNIC expose déjà un serveur MCP (Model Context Protocol) :
- Compatible Claude Desktop, Cursor, etc.
- Les agents MCP peuvent appeler `/judge` nativement
- Intégration zero-config

### Verdict-as-a-Service pour agents

API REST standard. N'importe quel agent peut :
```bash
# Avant d'exécuter un trade
verdict=$(curl -s cynic:3030/judge -d '{"content":"MINT","domain":"token-analysis"}')
if echo "$verdict" | jq -r '.verdict' | grep -q "BARK"; then
    echo "ABORT — token flagged"
    exit 1
fi
```

---

## Marché adressable

### TAM (Total Addressable Market)

- Agents crypto autonomes : estimation $50B+ d'ici 2027
- Chaque agent = un client potentiel pour CYNIC
- Infrastructure layer → revenus récurrents (API calls)

### SAM (Serviceable)

- Agents Solana (écosystème principal de CYNIC)
- Trading bots DeFi
- Wallets avec filtrage intégré

### SOM (Obtainable — 12 mois)

- 50-100 agents/bots intégrant l'API
- 10-20 clients Pro (99€/mois)
- 2-3 clients Enterprise

---

## Concurrence

Aucun concurrent direct dans "jugement souverain multi-modèle pour agents IA" :

- **GoPlus** : règles statiques, pas de jugement IA, centralisé
- **RugCheck** : heuristiques, pas d'agents, pas de mémoire
- **Chainanalysis** : compliance, pas de jugement temps réel
- **Nansen** : analytics, pas de verdict

---

## Roadmap Agentic

1. **Fait** : API REST, MCP server, 20K verdicts
2. **Q3 2026** : SDK Python/TypeScript, documentation agents
3. **Q4 2026** : Programme Solana on-chain (Pinocchio)
4. **2027** : Réseau de Dogs décentralisé, token de gouvernance

---

*Document de vision — support pour le pitch Futardio.*
