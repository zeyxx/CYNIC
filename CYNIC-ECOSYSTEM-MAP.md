# CYNIC ECOSYSTEM MAP - Vue Complète Interconnectée

> Document de synthèse : TOUS les modules, patterns et lois identifiés
> Résulte de l'analyse approfondie depackages/node/src/ et sous-modules

---

## 1. ARCHITECTURE GLOBALE (7×7 Fractal Matrix)

```
                    Keter (Crown)
         ┌─────────────────────────────────────┐
         │           CYNIC (Orchestrateur)      │
         │      Synthesis / Consensus           │
         └─────────────────────────────────────┘
                         │
    ┌──────────┬─────────┼─────────┬──────────┐
    │ C1       │ C2      │ C3      │ C4      │
    │ CODE     │ ORGANISM│ MARKET  │ SOCIAL  │
    │ Analyst  │ Guardian│ Oracle  │ Scout   │
    │ Deployer │ Janitor │ Sage    │ Scholar │
    │ Architect│         │         │         │
    └──────────┴─────────┴─────────┴──────────┘
```

**7 Colonnes (Cognitive Functions):**
- **C1 CODE** - Software engineering (Analyst, Deployer, Architect, Janitor)
- **C2 ORGANISM** - Self-management (Guardian, Janitor, CYNIC)
- **C3 MARKET** - Market data/trading (Oracle, Sage)
- **C4 SOCIAL** - Community/interactions (Scout, Scholar)
- **C5 BLOCKCHAIN** - Solana integration
- **C6 IDENTITY** - Reputation/E-Score
- **C7 LEARNING** - Continuous improvement

**7 Lignes (Lifecycle Stages):**
- **PERCEIVE → JUDGE → DECIDE → ACT → ACCOUNT → LEARN → EMERGE**

---

## 2. LES 11 DOGS (Sefirot Agents)

| Dog | Sefira | L1 | Role | Input | Output |
|-----|--------|-----|------|-------|--------|
| **CYNIC** | Keter | ❌ | Synthesis, final decisions | Events, judgments | Consensus, final output |
| **Sage** | Chochmah | ❌ | Strategic insight | Patterns, history | Recommendations, plans |
| **Oracle** | Tiferet | ❌ | Balance, harmony | Multi-source data | Mediated decisions |
| **Guardian** | Gevurah | ✅ | Security | Threats, risks | Block/allow decisions |
| **Architect** | Chesed | ✅ | Design, structure | Requirements | Architectures, patterns |
| **Analyst** | Binah | ✅ | Metrics, analysis | Data | Insights, metrics |
| **Scholar** | Daat | ✅ | Research, verification | Claims | Verified facts |
| **Deployer** | Hod | ❌ | Deployment, ops | Artifacts | Deployed systems |
| **Janitor** | Yesod | ✅ | Cleanup, refactoring | Code, data | Cleaned output |
| **Scout** | Netzach | ✅ | Exploration | Unknown territory | Discoveries |
| **MCP** | Malkhut | ❌ | Execution | Commands | Results |

---

## 3. LE SYSTÈME DE JUGEMENT (36 Dimensions)

### Structure: 5 Axioms × 7 Dimensions + META

| Axiom | Dimensions | Theme |
|-------|------------|-------|
| **PHI** | COHERENCE, ELEGANCE, STRUCTURE, HARMONY, PRECISION, COMPLETENESS, PROPORTION | Structure/Beauty |
| **VERIFY** | ACCURACY, PROVENANCE, INTEGRITY, VERIFIABILITY, TRANSPARENCY, REPRODUCIBILITY, CONSENSUS | Verification |
| **CULTURE** | AUTHENTICITY, RESONANCE, NOVELTY, ALIGNMENT, RELEVANCE, IMPACT, LINEAGE | Memetics |
| **BURN** | UTILITY, SUSTAINABILITY, EFFICIENCY, VALUE_CREATION, SACRIFICE, CONTRIBUTION, IRREVERSIBILITY | Utility |
| **FIDELITY** | COMMITMENT, ATTUNEMENT, CANDOR, CONGRUENCE, ACCOUNTABILITY, VIGILANCE, KENOSIS | Truth |
| **THE_UNNAMEABLE** | Explained variance (35 → 1) | META |

### Verdict System:
- **HOWL**: Exceptional (Q ≥ 80)
- **WAG**: Passes (Q ≥ 50)
- **GROWL**: Needs work (Q ≥ 38.2)
- **BARK**: Critical (Q < 38.2)

### φ-Bounded Confidence:
- Max confidence: **61.8%** (φ⁻¹)
- Min doubt: **38.2%** (φ⁻²)
- "φ distrusts φ" - max confidence never exceeds 61.8%

---

## 4. MODULES PRINCIPAUX ET INTERCONNEXIONS

### Orchestration Layer (`packages/node/src/orchestration/`)
```
unified-orchestrator.js
    ├── kabbalistic-router.js     → Dogs routing (Sefirot)
    ├── q-learning-router.js       → Learned dog selection
    ├── planning-gate.js          → "Think before acting"
    ├── brain.js                  → Consciousness layer
    ├── budget-monitor.js         → Cost control
    └── decision-event.js         → Unified event model
```

### Learning Layer (`packages/node/src/learning/`)
```
learning/
    ├── thompson-sampler.js       → Multi-armed bandit (Bayesian RL)
    ├── model-intelligence.js      → Model-task affinities
    ├── sona.js                   → Pattern-dimension correlations
    ├── reasoning-bank.js         → Successful reasoning trajectories
    └── behavior-modifier.js      → Confidence calibration
```

### Memory Layer (`packages/node/src/memory/`)
```
memory/
    ├── tiered-memory.js          → 4-tier: Vector/Episodic/Semantic/Working
    ├── shared-memory.js          → Collective intelligence
    ├── hilbert.js                → Hilbert curve spatial indexing
    ├── fourier.js                → Pattern frequency analysis
    └── user-lab.js               → Per-user isolated context

persistence/
    └── state-persister.js        → PostgreSQL crash recovery
```

### LLM Layer (`packages/llm/src/`)
```
llm/
    ├── router.js                 → Complexity classification
    ├── adapters/
    │   ├── intelligent-switch.js  → Cost/Speed/Quality/Privacy scoring
    │   └── learning-switch.js    → Thompson Sampling for models
    ├── pricing/
    │   └── oracle.js              → Real-time pricing for all LLMs
    └── retrieval/
        └── page-index.js         → Hybrid RAG (tree + vector, 98.7%)
```

### Perception Layer (`packages/node/src/perception/`)
```
perception/
    ├── index.js                  → Unified perception orchestrator
    ├── solana-watcher.js         → Blockchain events
    ├── market-watcher.js         → Jupiter, DexScreener, Birdeye
    ├── social-watcher.js         → Social sentiment
    ├── filesystem-watcher.js     → File changes
    └── machine-health-watcher.js → System health
```

### Network Layer (`packages/node/src/network/`)
```
network/
    ├── network-node.js            → P2P orchestration
    ├── validator-manager.js      → Validator set management
    ├── solana-anchoring.js       → On-chain truth anchoring
    ├── state-sync-manager.js     → State synchronization
    ├── fork-detector.js          → Chain fork resolution
    └── escore-provider.js         → E-Score calculation
```

### Protocol Layer (`packages/protocol/src/`)
```
protocol/
    ├── consensus/                 → φ-BFT consensus
    │   ├── engine.js             → Slot-based production
    │   ├── voting.js             → Weighted voting
    │   ├── lockout.js            → Exponential lockout
    │   └── finality.js           → 32 confirmations
    ├── gossip/                   → Gossip protocol
    ├── crypto/                   → Hash, signatures
    └── poj/                      → Proof of Judgment (Solana)
```

---

## 5. PATTERNS & LOIS IDENTIFIÉS

### 🔷 LOI 1: φ-Aligned Confidence
```
MAX_CONFIDENCE = φ⁻¹ = 0.618 (61.8%)
MIN_DOUBT = φ⁻² = 0.382 (38.2%)
```
- Utilisé dans: Jugements, consensus, pricing, polling intervals

### 🔷 LOI 2: Fractal Matrix (7×7)
- 7 cognitive functions × 7 lifecycle stages
- Chaque cellule peut contenir un mini-orchestrateur

### 🔷 LOI 3: Thompson Sampling (Exploration/Exploitation)
```
α = successes + 1
β = failures + 1
sample = Beta(α, β)
```
- Utilisé dans: Sélection de modèles, suggestions, routing

### 🔷 LOI 4: Exponential Lockout
```
lockout_slots = φ^n (where n = rounds since vote)
```
- Previent le flip-flopping dans le consensus

### 🔷 LOI 5: Hilbert Curve Spatial Indexing
- Préserve la localité dans l'espace N→1D
- Optimise la recherche de vecteurs similaires

### 🔷 LOI 6: Factory + Config Pattern
```
createActor(domainConfig) → Class
createDecider(domainConfig) → Class
createJudge(domainConfig) → Class
```
- 65% shared logic / 35% domain-specific config

### 🔷 LOI 7: Tiered Memory
```
Vector (embeddings) → Episodic (events) → Semantic (knowledge) → Working (context)
```

### 🔷 LOI 8: E-Score Reputation
```
E-Score = f(uptime, quality, burn_amount)
Burn-Multiplier = log_φ(burned + 1)
Vote-Weight = E-Score × Burn-Multiplier × Uptime
```

### 🔷 LOI 9: Kabbalistic Routing
- Dogs mappés aux Sefirot (Keter → Malkhut)
- Decisions fluides selon l'arbre de la vie (Seder Hishtalshelut)

### 🔷 LOI 10: Hybrid RAG (PageIndex)
```
tree_search (precision) + vector_search (recall) = PageIndex
```

---

## 6. INTERCONNEXIONS CRITIQUES

### Flow Principal:
```
Perception → Memory → Learning → Orchestration → Dogs → Judgment → Consensus → Blockchain
     ↓            ↓          ↓            ↓           ↓          ↓          ↓
  Watchers   Tiered    Thompson    Kabbalistic  11 Dogs   36 dims    φ-BFT
            Memory    Sampling    Router                  Oracle
                                  ↓
                              Intelligent
                                Switch
                                  ↓
                             PageIndex
                                  ↓
                              Pricing
                               Oracle
```

### Circuits de Feedback:
1. **Learning Circuit**: Judgment → SONA → Behavior → Future Judgment
2. **Economic Circuit**: Usage → Pricing → Budget → Model Selection
3. **Reputation Circuit**: Quality → E-Score → Voting Weight → Consensus
4. **Memory Circuit**: Perception → Encoding → Storage → Retrieval → Action

---

## 7. CE QUI EXISTE EN JS vs PYTHON

| Composant | JS (cynic-v1) | Python (cynic-v1-python) |
|-----------|---------------|---------------------------|
| **LLM Adapters** | ✅ Complete | ✅ Ollama, Anthropic |
| **Orchestrator** | ✅ Unified | ✅ Core |
| **Dogs** | ✅ 11 Sefirot | ✅ Base, Guardian, Scout |
| **Judgment** | ✅ 36 dimensions | ❌ |
| **Learning** | ✅ Thompson, SONA | ✅ Thompson |
| **Memory** | ✅ Tiered, Hilbert | ❌ |
| **Consensus** | ✅ φ-BFT | ❌ |
| **PageIndex** | ✅ Hybrid RAG | ❌ |
| **Pricing Oracle** | ✅ Real costs | ❌ |
| **E-Score** | ✅ Reputation | ❌ |
| **Blockchain** | ✅ Solana PoJ | ❌ |
| **Perception** | ✅ 7 watchers | ❌ |
| **Network** | ✅ P2P, Gossip | ❌ |

---

## 8. GAPS IDENTIFIÉS POUR CYNIC-v3

### Gaps Python (par rapport à JS):
1. ❌ Système de jugement 36 dimensions
2. ❌ φ-BFT Consensus protocol
3. ❌ Tiered memory avec Hilbert indexing
4. ❌ PageIndex hybrid RAG
5. ❌ Pricing Oracle temps réel
6. ❌ E-Score reputation system
7. ❌ 11 Dogs avec Kabbalistic routing
8. ❌ Perception layer (watchers)
9. ❌ P2P networking
10. ❌ Solana blockchain anchoring

---

## 9. PROCHAINES ÉTAPES RECOMMANDÉES

1. **Valider cette cartographie** - Est-ce que tout est correct?
2. **Prioriser les gaps** - Lequel implémenter en premier?
3. **Choisir architecture** - Comment connecter Python à l'écosystème existant?

---

*Document généré par analyse approfondie de l'écosystème CYNIC JS*
*Dernière mise à jour: 2026-02-14*
