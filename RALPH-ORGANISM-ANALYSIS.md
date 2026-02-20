# CYNIC Organism Analysis
## Ralph Loop Iteration 2 - Complete Mapping

---

## 🎯 FULL PICTURE: Organisme + Processus UNIFIÉS

### ORGANISM (11 Dogs - Sefirot Agents)

| Dog | Sefira | Role | L1 Heuristics |
|-----|--------|------|---------------|
| Guardian | Gevurah | Security/validation | ✅ patterns.json + rules.js |
| Scout | Netzach | Exploration/discovery | ✅ |
| Analyst | Binah | Deep analysis | ✅ |
| Janitor | Yesod | Cleanup/maintenance | ✅ |
| Architect | Chesed | Architecture/planning | ✅ |
| Scholar | Daat | Knowledge/research | ✅ |
| Sage | Chochmah | Wisdom/synthesis | ❌ |
| Oracle | Tiferet | Predictions/future | ❌ |
| Deployer | Hod | Deployment/execution | ❌ |
| Cartographer | Malkhut | Mapping/tracking | ❌ |
| CYNIC | Keter | Meta-cognition | ❌ |

### PROCESSUS (Learning Loops)

#### 1. **Q-Learning Loop** (Route Optimization)
- **Used by**: KabbalisticRouter, QLearningRouter
- **Purpose**: Learn optimal Dog selection
- **Persistence**: PostgreSQL `qlearning_state` table
- **Hyperparameters**: α=0.618 (φ⁻¹), γ=0.382 (φ⁻²)

#### 2. **Thompson Sampling Loop** (Model Selection)
- **Used by**: ModelIntelligence (LLM selection)
- **Purpose**: Explore/exploit LLM models (Opus, Sonnet, Haiku, Ollama)
- **Persistence**: `~/.cynic/thompson/state.json`
- **Exploration rate**: φ⁻³ (23.6%)

#### 3. **SONA Loop** (Pattern Adaptation)
- **Used by**: Real-time judgment observation
- **Purpose**: Adapt dimension weights
- **Persistence**: learning_events table

#### 4. **BehaviorModifier Loop** (Feedback Processing)
- **Used by**: User feedback → behavior changes
- **Purpose**: Close feedback loop

#### 5. **MetaCognition Loop** (Self-Monitoring)
- **Used by**: Strategy switching
- **Purpose**: Optimize learning parameters

#### 6. **LearningPipeline** (5-Stage Orchestration)
```
PERCEIVE → JUDGE → DECIDE → ACT → LEARN
```

---

## 🔗 HOW ORGANISM + PROCESSUS CONNECT

### entry.js (Boot Sequence)
```javascript
1. bootDaemon() → Load components
2. wireEventAdapter() → Bridge old → unified events
3. wireDaemonServices() → ModelIntelligence + CostLedger
4. wireLearningSystem() → Collective + SONA + MetaCognition
5. wireOrchestrator() → KabbalisticRouter → Dogs → Consensus
6. wireWatchers() → FileWatcher + SolanaWatcher
7. wireConsciousnessReflection() → 60 min self-reflection
8. wireCynicHeartbeat() → 5 min autonomous cycle
```

### Event Flow
```
globalEventBus → KabbalisticRouter → Dogs → Consensus → Q-Learning
                      ↓
                 Thompson (model selection)
                      ↓
                 learning_events (DB)
```

---

## 📊 GAPS IDENTIFIED

| Gap | Severity | Current State | Solution |
|-----|----------|---------------|----------|
| **No Web UI** | 🔴 Critical | CLI only | Build Frontend (Vibe Companion inspired) |
| **Python not integrated** | 🔴 Critical | cynic-v1-python exists | Connect to Node.js |
| **Embeddings missing** | 🔴 Critical | No real embeddings | sentence-transformers |
| **Vector DB missing** | 🔴 Critical | No vector store | Qdrant integration |
| **6 Dogs without L1** | 🟠 High | patterns.json missing | Implement heuristics |

---

## 🧬 UNIFIED ARCHITECTURE (Full Picture)

```
┌─────────────────────────────────────────────────────────────────────┐
│                        FRONTEND (Web UI)                            │
│         Multi-session Claude Code (Vibe Companion inspired)         │
└────────────────────────────┬────────────────────────────────────────┘
                             │ WebSocket
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    ORCHESTRATOR (Node.js)                          │
├─────────────────────────────────────────────────────────────────────┤
│  KabbalisticRouter → Dogs (11 Sefirot) → Consensus                │
│         ↓                                                            │
│  Q-Learning (route weights)                                          │
│  Thompson (model selection)                                          │
│  SONA (pattern adaptation)                                           │
│  MetaCognition (self-optimization)                                   │
└────────────────────────────┬────────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        ▼                    ▼                    ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   PYTHON     │     │  CLAUDE      │     │   SOLANA    │
│   (ML)       │     │   CODE       │     │   CHAIN     │
├──────────────┤     ├──────────────┤     ├──────────────┤
│ embeddings   │     │ --sdk-url    │     │   Anchor     │
│ vector DB    │     │ WebSocket    │     │   PoJ        │
│ Q-Learning   │     │ 3 models     │     │   Consensus  │
│ Thompson     │     │ Sonnet       │     │              │
│ DPO/EWC      │     │ Haiku        │     │              │
│              │     │ Opus         │     │              │
└──────────────┘     └──────────────┘     └──────────────┘
```

---

## ✅ WHAT'S WORKING

- [x] 11 Dogs (organism) - Full system wired
- [x] Q-Learning loop - Persisted to DB
- [x] Thompson Sampling - Model selection works
- [x] SONA + BehaviorModifier - Learning loops active
- [x] MetaCognition - Self-optimization active
- [x] LearningPipeline - 5-stage orchestration
- [x] Service wiring - All connected at boot

---

## ❌ WHAT'S MISSING

- [ ] Web UI (front-end)
- [ ] Python ML integration (embeddings, vector DB)
- [ ] 6 Dogs need L1 heuristics
- [ ] Deployment pipeline

---

<promise>ORGANISM ANALYSIS COMPLETE</promise>
