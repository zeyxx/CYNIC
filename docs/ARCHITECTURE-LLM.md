# CYNIC v2 - Architecture Axe LLM

## Pourquoi ces modules JS?

**Contexte:** CYNIC avait besoin d'un système LLM unifié et intelligent qui n'existait pas.

### Problèmes identifiés:
- Adapters isolés (Ollama, Claude Code, Anthropic)
- Pas de sélection intelligente
- Pas de retrieval reasoning-based
- Pas d'apprentissage continu

---

## Architecture Finale

```
packages/llm/src/
├── pricing/          → 💰 Coûts RÉELS
├── adapters/         → 🔌 Multi-LLM
├── orchestration/    → 🧠 Planification + Exécution
├── retrieval/       → 📚 PageIndex (RAG)
└── learning/       → 🧬 Apprentissage
```

---

## Flux de Données

```
Query
  │
  ▼
EnhancedPrometheus (analyse + plan)
  │    └── PageIndex (récupère contexte)
  │
  ▼
LearningSwitch (sélection adapter)
  │    └── PricingOracle (coûts réels)
  │    └── LearningEngine (stats)
  │
  ▼
Atlas (exécution)
  │
  ▼
LearningEngine (enregistre)
  │
  ▼
Persistence (PostgreSQL / VectorStore)
```

---

## Modules Créés

| Module | Fichier | Rôle |
|--------|----------|------|
| PricingOracle | `pricing/oracle.js` | Coûts réels (rien n'est gratuit) |
| IntelligentSwitch | `adapters/intelligent-switch.js` | Sélection LLM |
| LearningSwitch | `adapters/learning-switch.js` | + Apprentissage |
| PageIndex | `retrieval/page-index.js` | RAG reasoning (98.7%) |
| Prometheus | `orchestration/prometheus.js` | Planification |
| EnhancedPrometheus | `orchestration/enhanced-prometheus.js` | + Retrieval |
| Atlas | `orchestration/atlas.js` | Exécution |
| LearningEngine | `learning/index.js` | Thompson Sampling |
| LearningPersistence | `persistence-integration.js` | → PostgreSQL/Vector |

---

## Principes de Conception

### 1. RIEN N'EST Gratuit
- Claude Code: $20/mois
- Ollama: GPU electricity + amortissement
- API: prix réel par token

### 2. Apprentissage Continu
- Chaque requête = learning event
- Thompson Sampling pour exploration/exploitation
- Patterns indexés pour retrieval futur

### 3. Auto-Suffisance
- Hybrid local (Ollama/SQLite) + cloud (API)
- Fallback si un provider échoue

### 4. Privacy par Défaut
- SQLite local par défaut
- Sync opt-in seulement
- PoJ pour preuves

---

## Intégration avec l'Existant

Modules CYNIC existants:
- `packages/persistence` - PostgreSQL, Redis, SQLite, VectorStore, MerkleDAG, PoJChain
- `packages/core` - Logger, PHI constants
- `cynic-v1-python` - Python adapters (à connecter)

---

## Prochaines Étapes

1. Connecter PageIndex → VectorStore (HNSW)
2. Connecter Learning → PostgreSQL
3. Connecter Python adapters → JS modules
4. Tests d'intégration complète
