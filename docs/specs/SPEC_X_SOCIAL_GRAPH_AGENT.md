# SPEC — X Social Graph & Agent-Assisted Writing

## Vision

Un système qui capture passivement l'activité X de l'utilisateur, construit un graphe social multi-dimensionnel, et permet à un agent d'assister l'écriture (posts, replies, DMs) de manière contextualisée — sans jamais substituer l'utilisateur.

L'agent connaît le réseau social et les projets en cours. Il propose, l'utilisateur valide.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Chrome Headed (CDP)                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ User     │  │ Agent    │  │ mitmproxy        │  │
│  │ (manual) │  │ (MCP)    │  │ (capture)        │  │
│  └────┬─────┘  └────┬─────┘  └──────┬───────────┘  │
│       │              │               │              │
│       └──────┬───────┘               │              │
│              │                        │              │
│              ▼                        ▼              │
│     ┌────────────────┐    ┌────────────────────┐     │
│     │ Orchestrator   │    │ Traffic DB (SQLite)│     │
│     │ (simule clics) │    │ (endpoints, req,   │     │
│     │ + entropie     │    │  resp, timestamps) │     │
│     └───────┬────────┘    └────────┬───────────┘     │
│             │                      │                │
│             ▼                      ▼                │
│     ┌────────────────────────────────────────┐       │
│     │  Social Graph DB (SQLite)              │       │
│     │  Nodes: users, tweets, projects, DMs    │       │
│     │  Edges: follow, reply, like, RT, mention│       │
│     │  Layers: temporal, topical, project     │       │
│     └────────────────┬───────────────────────┘       │
│                      │                              │
│                      ▼                              │
│     ┌────────────────────────────────────────┐       │
│     │  Agent Context (Hermes session)        │       │
│     │  - Current project (GASdf, OSS, etc.)  │       │
│     │  - Social graph snapshot               │       │
│     │  - Proposed actions (drafts)            │       │
│     │  - User validation (go/stop)            │       │
│     └────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────┘
```

## Composants

### 1. Chrome Headed (CDP)
- Profil persistant (`chrome-profile/`)
- Cookies importés une fois
- Remote debugging port 9222
- L'utilisateur et l'agent partagent le même navigateur

### 2. mitmproxy (capture passive)
- Proxy port 8080
- Addon Python capture les requêtes XHR/GraphQL
- Stockage SQLite (`x-traffic.db`)
- Filtrage : endpoints pertinents uniquement (pas les assets)
- Acteur taggé (user/agent) pour distinguer les actions

### 3. Orchestrator (simulation)
- Simule les clics et l'écriture au clavier (Playwright CDP)
- Entropie humaine : délais aléatoires, frappe variable, mouvements souris
- Gestion du passcode 4012 (demande à l'utilisateur, saisie CDP)
- Rate limiting : 8-15s entre DMs, max 5-10 msg/min
- Strictly serial (pas de parallèle pour les DMs)
- Lock agent/humain (request-control.sh / release-control.sh)

### 4. Social Graph (SurrealDB)
- SurrealDB (cynic-surreal, L1 cluster) — graphe natif
- Nœuds : users, tweets, projects, dm_threads
- Arêtes : follow, reply, like, rt, mention, dm, collaboration
- Embeddings : cynic-knowledge/vector (RAG SIMD) pour recherche sémantique
- Ingestion : script parse x-traffic.db (SQLite staging) → SurrealDB
- L'agent interroge SurrealDB via CYNIC RAG → contexte pour les propositions

### 5. Agent Context (Hermes + CYNIC RAG)
- Projet en cours (GASdf, contribution OSS, etc.)
- Snapshot du graphe social (collaborateurs, topics actifs)
- Brouillons proposés (posts, replies, DMs)
- Validation utilisateur (go/stop)
- Interrogation SurrealDB via cynic-knowledge pour contextualiser

## Flux de données

1. **Capture** : mitmproxy intercepte le trafic X → SQLite
2. **Ingestion** : script parse les requêtes → extrait les tweets/profils/interactions → Social Graph DB
3. **Contexte** : l'agent charge le graphe + le projet en cours
4. **Proposition** : l'agent propose des actions (drafts)
5. **Validation** : l'utilisateur valide ou ajuste
6. **Exécution** : l'orchestrateur simule les clics/clavier
7. **Log** : l'action est enregistrée (acteur=agent, timestamp, résultat)

## Cas d'usage

### A. Assistance à l'écriture (posts/replies)
- L'agent observe le feed X
- Propose des replies pertinentes basées sur le contexte projet
- L'utilisateur valide → l'agent poste

### B. DMs contextualisés
- L'agent connaît les collaborateurs (@gcrtrd, etc.)
- Propose des DMs en fonction du projet en cours (GASdf, OSS)
- L'utilisateur valide → l'agent envoie (avec passcode 4012 si nécessaire)

### C. Graphe social
- Ingestion passive des tweets/profils lus
- Construction du graphe (qui interagit avec qui, sur quel topic)
- L'agent utilise le graphe pour contextualiser ses propositions

## Roadmap

| Phase | Livrable | Statut |
|-------|----------|--------|
| 1 | Chrome headed + CDP + cookies | Fait |
| 2 | mitmproxy + capture addon | Fait |
| 3 | Orchestrator (simulation + entropie) | À faire |
| 4 | Passcode 4012 handler | À faire |
| 5 | Social Graph DB (schema + ingestion) | À faire |
| 6 | Agent context (Hermes integration) | À faire |
| 7 | Assistance à l'écriture (drafts + validation) | À faire |
| 8 | Cache + single-flight | À faire |

## Principes

- **Assistance, pas substitut** : l'agent propose, l'utilisateur valide.
- **Pas de parallèle** pour les DMs (rate limits).
- **No retry** sur les échecs DM (risque de doublons).
- **Entropie humaine** : délais aléatoires, frappe variable.
- **Lock agent/humain** : l'agent ne touche au navigateur que s'il détient le lock.
- **Capture passive** : l'ingestion ne modifie pas le comportement de l'utilisateur.

## Références

- twitter-mcp (evangelinehelsinki) — Camoufox, cache, single-flight
- stealth-x — DOM automation + browser-internal fetch
- x-mcp-playwright (Glama) — 9 tools groupés, DMs, proxy
- browser-act x-dm-auto-chat — automation DMs complète, passcode 4012
- just-rich/X-DM-Followers — Selenium DM automation
EOF
echo "spec written"