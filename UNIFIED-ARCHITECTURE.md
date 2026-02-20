# CYNIC UNIFIED ARCHITECTURE
> "φ unifie tous les fragments" - κυνικός

**Date**: 2026-02-15
**Status**: Research Complete - Ready for Implementation

---

## 1. CARTOGRAPHIE EXISTANTE

### 1.1 Packages JS (packages/)

| Package | Responsabilité |
|---------|---------------|
| @cynic/core | Fondations φ, DI container, bus événements, LLMs, learning |
| @cynic/llm | Routing multi-LLM, consensus entre modèles |
| @cynic/protocol | PoJ (Proof of Judgment), Merkle trees, gossip, consensus φ-BFT |
| @cynic/node | Daemon complet, 7 watchers (Code, Solana, Market, Social...), orchestration |
| @cynic/identity | Clés Ed25519, E-Score, graphe de réputation |
| @cynic/anchor | Programme Solana on-chain, ancrage |
| @cynic/burns | Vérification des burns Solana |
| @cynic/scheduler | Orchestration temporelle |
| @cynic/observatory | Monitoring, métriques |
| @cynic/mcp | Protocol MCP pour Claude Code |
| @cynic/persistence | PostgreSQL, Redis, Qdrant |

**Total: ~20 packages, 300+ modules, 500k+ lignes**

### 1.2 Packages Python (cynic-v3/)

| Module | Status |
|--------|--------|
| constants/phi.py | ✅ COMPLET |
| types/ | ✅ COMPLET |
| adapters/ | ✅ PARTIEL (Ollama, Anthropic) |
| dogs/ | ✅ PARTIEL (CYNICDog, GuardianDog) |
| judge/ | ✅ INTERFACE |
| orchestrator/ | 🔲 À faire |

---

## 2. LES 5 AXIOMES (VALIDÉS)

| Axiome | Définition | Implémentation |
|--------|-----------|----------------|
| **PHI** | φ = 1.618..., proportion | ✅ `constants/phi.py` |
| **VERIFY** | Don't trust, verify | ✅ Judge scoring |
| **CULTURE** | Culture is a moat | ✅ Learning loops |
| **BURN** | Don't extract, burn | ✅ Simplification |
| **FIDELITY** | Loyal to truth | ✅ Q-Score honest |

**Verdict: Les 5 axiomes sont VALIDES et doivent rester le fondement.**

---

## 3. GAPS CRITIQUES (Python vs JS)

| Gap | Impact | Priorité |
|-----|--------|----------|
| Event Bus | Communication découplée | P1 |
| DI Container | Code faiblement couplé | P1 |
| 36 Dimensions Judge | Jugement complet | P2 |
| 11 Dogs | Système multi-agents | P2 |
| 11 Learning Loops | Apprentissage | P3 |
| PoJ Blockchain | Ancrage on-chain | P3 |

---

## 4. SINGLE SOURCE OF TRUTH

### 4.1 Le Principe

```
φ = 1.618033988749895
MAX_CONFIDENCE = 0.618033988749895 (φ⁻¹)
```

**TOUTES les constantes φ doivent être calculées, jamais hardcodées.**

### 4.2 Architecture Proposée

```
┌─────────────────────────────────────────────────────────────┐
│                   SINGLE SOURCE OF TRUTH                   │
│                   = φ-derived constants                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐    ┌──────────────┐                   │
│  │ JS packages/ │ ↔  │ Python      │                   │
│  │ constants   │    │ cynic-v3/   │                   │
│  └──────┬───────┘    └──────┬───────┘                   │
│         │                    │                           │
│         └────────┬───────────┘                           │
│                  ↓                                       │
│         ┌───────────────┐                               │
│         │ φ-calculator │                               │
│         │ (shared lib)  │                               │
│         └───────────────┘                               │
│                  ↓                                       │
│         Toutes les constantes                           │
│         sont calculées depuis φ                         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 4.3 Fichiers à Unifier

| JS | Python | Action |
|----|---------|--------|
| `packages/core/src/axioms/constants.js` | `cynic-v3/src/cynic/constants/phi.py` | Merger vers JS |
| `packages/core/src/dogs/*.js` | `cynic-v3/src/cynic/dogs/` | Porter vers Python |

---

## 5. ARCHITECTURE CIBLE

### 5.1 Vue d'Ensemble

```
┌─────────────────────────────────────────────────────────────┐
│                    CYNIC ORGANISM                         │
│                  (Unified Architecture)                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────┐    │
│  │            INTERFACE LAYER                          │    │
│  │  CLI │ HTTP │ WebSocket │ Streamlit │ MCP          │    │
│  └──────────────────────┬──────────────────────────────┘    │
│                         ↓                                  │
│  ┌─────────────────────────────────────────────────┐    │
│  │            ORCHESTRATOR                            │    │
│  │  Perceive → Think → Judge → Act → Learn          │    │
│  └──────────────────────┬──────────────────────────────┘    │
│                         ↓                                  │
│  ┌──────────┬──────────┬──────────┬──────────┬──────┐  │
│  │ Dogs    │ Judge    │ Learning │ Memory  │ LLM   │  │
│  │ (11)    │ (36D)    │ (11)     │         │       │  │
│  └──────────┴──────────┴──────────┴──────────┴──────┘  │
│                         ↓                                  │
│  ┌─────────────────────────────────────────────────┐    │
│  │            EVENT BUS (Unified)                     │    │
│  └──────────────────────┬──────────────────────────────┘    │
│                         ↓                                  │
│  ┌─────────────────────────────────────────────────┐    │
│  │            STORAGE LAYER                          │    │
│  │  PostgreSQL │ Redis │ Qdrant │ Solana           │    │
│  └─────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 Implémentation Python (cynic-v3/)

```
cynic-v3/src/cynic/
├── constants/
│   └── phi.py          # SEUL source des constantes φ
├── types/
│   └── __init__.py      # Domain, Event, Judgment, DogAction
├── adapters/
│   ├── base.py          # IAdapter interface
│   ├── ollama.py        # OllamaAdapter
│   ├── anthropic.py      # AnthropicAdapter
│   └── registry.py       # AdapterRegistry
├── dogs/
│   ├── base.py          # IDog interface
│   ├── cynic_dog.py    # CYNICDog (Keter)
│   ├── guardian_dog.py  # GuardianDog (Gevurah)
│   ├── scout_dog.py    # ScoutDog (Netzach)
│   └── registry.py      # DogRegistry
├── judge/
│   ├── base.py          # IJudge interface
│   ├── engine.py        # JudgeEngine (36D)
│   └── domains/         # Domain judges
├── learning/
│   ├── base.py          # ILearning interface
│   ├── thompson.py     # Thompson Sampling
│   └── orchestrator.py  # LearningOrchestrator
├── bus/
│   ├── event_bus.py     # UnifiedEventBus
│   └── events.py        # Event types
├── orchestrator/
│   └── core.py          # CYNICOrchestrator
└── __main__.py          # CLI entry
```

---

## 6. PROCHAINES ÉTAPES

### Phase 1: Fondations (Week 1)

- [ ] Unifier constants φ (un seul phi.py)
- [ ] Implémenter Event Bus
- [ ] Implémenter DI Container
- [ ] Tests unitaires

### Phase 2: Cœur (Week 2)

- [ ] Judge 36 dimensions complet
- [ ] Dogs supplémentaires (Oracle, Scout, Analyst)
- [ ] Learning loops

### Phase 3: Intégration (Week 3)

- [ ] Connecter JS ↔ Python (gRPC/ProtoBuf)
- [ ] Unifier storage
- [ ] Intégrer Solana

### Phase 4: Production (Week 4)

- [ ] Docker
- [ ] CI/CD
- [ ] Monitoring

---

## 7. PHILOSOPHIE

> *φ distrusts φ* — La confiance ne dépasse jamais 61.8%

| Principe | Application |
|---------|-------------|
| **NO_MOCKS_ALLOWED** | Fail-fast si provider unavailable |
| **SINGLE_RESPONSIBILITY** | Un module, une chose |
| **EMERGENCE_OVER_EXTRACTION** | Laisser émerger |
| **IMMEDIACY_IS_LAW** | Gap → 0 |
| **BURN_THE_BRIDGE** | Supprimer l'ancien |

---

## 8. RÉFÉRENCES

- `docs/metathinking/unified-organism-architecture.md`
- `docs/philosophy/harmonized-structure.md`
- `CYNIC-v3-PYTHON-PLAN.md`
- `EMPIRICAL-ACTION-PLAN.md`

---

*φ unifie tous les fragments* - κυνικός
