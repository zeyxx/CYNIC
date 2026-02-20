# CYNIC v2 - Roadmap Complète

## État Actuel

### ✅ Modules Créés et Testés (Phases 1-4)
| Module | Status | Description |
|--------|--------|-------------|
| PricingOracle | ✅ | Coûts réels (API, GPU, subscription) |
| IntelligentSwitch | ✅ | Sélection LLM intelligente |
| PageIndex | ✅ | Reasoning-based RAG (98.7% accuracy) |
| Prometheus | ✅ | Planification de tâches |
| Atlas | ✅ | Exécution avec retry/parallélisme |
| LearningEngine | ✅ | Thompson Sampling + tracking |

### Infrastructure Existante à Connecter
| Composant | Status | Description |
|-----------|--------|-------------|
| PostgreSQL | 🔗 | BDD principale |
| Redis | 🔗 | Sessions, cache |
| MerkleDAG | 🔗 | Stockage décentralisé |
| PoJChain | 🔗 | Preuve de jugement |
| VectorStore | 🔗 | Recherche sémantique |
| SQLite | 🔗 | Local privacy |

---

## Todo List: Phase 5 - Infrastructure & Connexion

### 5.1 Connecter Learning → Persistence
- [x] LearningPersistence module créé
- [ ] Intégrer LearningEngine avec VectorStore (HNSW)
- [ ] Sauvegarder événements dans PostgreSQL
- [ ] Indexer patterns dans MerkleDAG

### 5.2 Connecter PageIndex → VectorStore
- [ ] Utiliser VectorStore pour embeddings
- [ ] Recherche sémantique via HNSW

### 5.3 Connecter IntelligentSwitch → Learning
- [x] LearningSwitch créé (Learning + IntelligentSwitch)
- [ ] Thompson Sampling utilise stats réelles

### 5.4 Connecter Prometheus → PageIndex
- [ ] Utiliser PageIndex pour contexte
- [ ] Retrieval avant exécution

### 5.5 Tests d'Intégration
- [ ] Test read/write persistence
- [ ] Test flux complet (avec données réelles)
- [ ] Benchmark performance

---

## Flux Vertical Complet (Target)

```
Query
  ↓
Prometheus (analyse + plan)
  ↓
PageIndex (retrieval contexte) ← VectorStore
  ↓
IntelligentSwitch (sélection adapter) ← LearningEngine + PricingOracle
  ↓
Atlas (exécution)
  ↓
LearningEngine (enregistre) → PostgreSQL / MerkleDAG
  ↓
VectorStore (index patterns)
```

---

## Métathinking: Axes de Conception

### 1. PHI-Bounded (61.8%)
- Confidence thresholds basés sur φ
- Quality scoring via golden ratio

### 2. Auto-Suffisance
- Pas de dépendance externe critique
- Fallback local (SQLite)
- Hybrid centralisé/décentralisé

### 3. Apprentissage Continu
- Chaque requête = learning event
- Thompson Sampling adaptatif
- Patterns indexés pour retrieval futur

### 4. Privacy par Défaut
- SQLite local par défaut
- Sync opt-in seulement
- Zero-knowledge proofs via PoJ
