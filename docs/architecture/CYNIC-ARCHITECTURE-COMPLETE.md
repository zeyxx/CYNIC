# CYNIC ARCHITECTURE COMPLÈTE
## Plan d'Implémentation Python - Avec Diagrammes

> "φ unifie tous les fragments" - κυνικός

---

# SECTION 1: POURQUOI TU AS BESOIN DE CE PLAN

## Ce que tu rate en tant que non-ingénieur:

### 1.1 Le Problème du "ça marche en dev"
En JS, tu as eu:
```javascript
// Ça marche en dev...
if (something) {
  // ...mais en prod ça crash
}
```
**Pourquoi:** Pas de types, pas de tests en conditions réelles.

### 1.2 Le Problème des Singletons
```javascript
let instance = null;
function getInstance() {
  if (!instance) instance = new Something();
  return instance;
}
```
**Problème:** Qui modifie instance? Quand? Pourquoi? IMPOSSIBLE à tracer.

### 1.3 Le Problème du "works in my machine"
Ton code marchait chez toi mais pas en production.
**Pourquoi:** Pas de containerization, pas de reproduce environment.

### 1.4 Ce que Python corrige:
| JS Problem | Python Solution |
|-----------|-----------------|
| Types dynamiques | Type hints + mypy |
| Global scope | Virtual environments |
| Callback hell | async/await propre |
| 3 Event Buses | 1 bus typé |
| Singleton chaos | DI Container |

---

# SECTION 2: LES 7 DIAGRAMMES FONDAMENTAUX

## 2.1 DIAGRAMME 1: Vue d'Ensemble (Architecture)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CYNIC ORGANISM                                     │
│                    (Python Implementation)                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────┐    │
│   │                     INTERFACE LAYER                               │    │
│   │  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────┐       │    │
│   │  │  CLI    │  │  REST    │  │ WebSocket│  │   Streamlit │       │    │
│   │  │ (cynic) │  │  (Fast)  │  │  (MCP)   │  │  (Cockpit)  │       │    │
│   │  └────┬────┘  └────┬─────┘  └────┬─────┘  └──────┬──────┘       │    │
│   │       │            │             │               │                │    │
│   └───────┼────────────┼─────────────┼───────────────┼────────────────┘    │
│           │            │             │               │                      │
│           ▼            ▼             ▼               ▼                      │
│   ┌─────────────────────────────────────────────────────────────────┐    │
│   │                     ORCHESTRATOR                                │    │
│   │                                                                  │    │
│   │   Perceive → Think → Judge → Decide → Act → Learn → Account    │    │
│   │        ↓         ↓       ↓       ↓        ↓       ↓        ↓   │    │
│   │   [Events]   [LLMs]  [Judge]  [Dogs]    [Action] [Learning]     │    │
│   │                                                                  │    │
│   └─────────────────────────────┬──────────────────────────────────────┘    │
│                                 │                                           │
│                                 ▼                                           │
│   ┌─────────────────────────────────────────────────────────────────┐    │
│   │                      EVENT BUS (Unified)                         │    │
│   │                                                                  │    │
│   │   ┌──────────────────────────────────────────────────────────┐  │    │
│   │   │  Type-safe events only (no string literals!)              │  │    │
│   │   │  - PERCEPTION:created                                     │  │    │
│   │   │  - JUDGMENT:completed                                    │  │    │
│   │   │  - ACTION:executed                                       │  │    │
│   │   │  - LEARNING:updated                                      │  │    │
│   │   └──────────────────────────────────────────────────────────┘  │    │
│   │                                                                  │    │
│   └─────────────────────────────┬──────────────────────────────────────┘    │
│                                 │                                           │
│           ┌─────────────────────┼─────────────────────┐                    │
│           ▼                     ▼                     ▼                    │
│   ┌───────────────┐    ┌───────────────┐    ┌───────────────┐              │
│   │   DOGS        │    │    JUDGE      │    │   LEARNING   │              │
│   │   (11 agents)  │    │   (36 dims)  │    │   (11 loops) │              │
│   │                │    │               │    │               │              │
│   │ CYNIC (Keter) │    │  FIDELITY    │    │ Q-Learning    │              │
│   │ Sage (Choch) │    │  PHI         │    │ Thompson      │              │
│   │ Analyst (Bin) │    │  VERIFY      │    │ SONA          │              │
│   │ Guardian (Gev)│    │  CULTURE     │    │ EWC++         │              │
│   │ ...           │    │  BURN        │    │ ...           │              │
│   └───────────────┘    └───────────────┘    └───────────────┘              │
│                                 │                                           │
│                                 ▼                                           │
│   ┌─────────────────────────────────────────────────────────────────┐    │
│   │                      STORAGE LAYER                               │    │
│   │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │    │
│   │  │PostgreSQL│  │  Redis   │  │ Qdrant   │  │ Solana   │        │    │
│   │  │(judgments│  │ (cache)  │  │(vectors) │  │ (PoJ)   │        │    │
│   │  │ sessions)│  │          │  │          │  │          │        │    │
│   │  └──────────┘  └──────────┘  └──────────┘  └──────────┘        │    │
│   └─────────────────────────────────────────────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2.2 DIAGRAMME 2: Cycle de Vie (Orchestrator)

```
┌────────────────────────────────────────────────────────────────────────────                   ─┐
│ CYNIC CYCLE (5 Étapes)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│    ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐          │
│    │ PERCEIVE│────▶│  THINK  │────▶│  JUDGE  │────▶│  DECIDE │          │
│    └────┬────┘     └────┬────┘     └────┬────┘     └────┬────┘          │
│         │               │               │               │                 │
│         ▼               ▼               ▼               ▼                 │
│    ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐          │
│    │  Event  │     │   LLM   │     │   36    │     │  Dogs  │          │
│    │ Created │     │ Response│     │Dimensions│     │ Vote   │          │
│    └─────────┘     └─────────┘     └─────────┘     └────┬────┘          │
│                                                         │                │
│    ┌───────────────────────────────────────────────────┼───────────┐    │
│    │                                           │         ▼           │    │
│    │                                           │    ┌─────────┐      │    │
│    │         ┌────────────────────────────────┴───┐│  ACT    │      │    │
│    │         │  LEARN                                │└────┬────┘      │    │
│    │         │  (Q-Learning, Thompson, MetaCog)       │     │           │    │
│    │         └────────────────────────────────┬───────┘     ▼           │    │
│    │                                         │    ┌─────────┐          │    │
│    │                                         └────│ ACCOUNT │◀────────┘    │
│    │                                              └─────────┘               │
│    │                                                      │                  │
│    └──────────────────────────────────────────────────────┘                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2.3 DIAGRAMME 3: Les 11 Dogs (Sefirot)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    THE 11 DOGS (Kabbalistic Tree of Life)                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                              KETER (CYNIC)                                  │
│                           [Meta-consciousness]                               │
│                               │                                              │
│              ┌────────────────┼────────────────┐                          │
│              ▼                                 ▼                            │
│         CHOCMAH (Sage)                   CHESED (Architect)              │
│         [Wisdom/Insights]                 [Design/ Mercy]                   │
│              │                                 │                            │
│              ▼                                 ▼                            │
│         BINAH (Analyst)                   GEVURAH (Guardian)              │
│         [Deep Analysis]                    [Security/ Judgment]             │
│              │                                 │                            │
│              ▼                                 ▼                            │
│         DAAT (Scholar)                    TIFERET (Oracle)                 │
│         [Knowledge]                       [Balance/ Consensus]             │
│              │                                 │                            │
│              ▼                                 ▼                            │
│         HOD (Deployer)                    NETZACH (Scout)                 │
│         [Operations]                       [Exploration]                   │
│              │                                 │                            │
│              └────────────────┬────────────────┘                            │
│                               ▼                                             │
│                          YESOD (Janitor)                                    │
│                          [Cleanup/ Integration]                              │
│                               │                                             │
│                               ▼                                             │
│                          MALKUT (Cartographer)                              │
│                          [Mapping/ World State]                             │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ CONSENSUS: All dogs vote → weighted by domain → final decision  │  │
│    │ Threshold: φ (1.618) consensus required for major actions         │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2.4 DIAGRAMME 4: Les 36 Dimensions (Judge)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 THE 36 DIMENSIONS (5 Axioms × 7 + THE_UNNAMEABLE)        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ FIDELITY (The Meta-Axiom - φ watches φ)                            │   │
│  │ "Loyal to truth, not to comfort"                                  │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │ 1. COMMITMENT  2. ATTUNEMENT  3. CANDOR   4. CONGRUENCE           │   │
│  │ 5. ACCOUNTABILITY  6. VIGILANCE  7. KENOSIS (self-emptying)       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ PHI (Mathematical Harmony - 1.618...)                              │   │
│  │ "Proportion governs all"                                            │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │ 1. COHERENCE  2. ELEGANCE  3. STRUCTURE  4. HARMONY                │   │
│  │ 5. PRECISION  6. COMPLETENESS  7. PROPORTION                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ VERIFY (Proof - Don't trust, verify)                               │   │
│  │ "Evidence over belief"                                              │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │ 1. ACCURACY  2. PROVENANCE  3. INTEGRITY  4. VERIFIABILITY         │   │
│  │ 5. TRANSPARENCY  6. REPRODUCIBILITY  7. CONSENSUS                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ CULTURE (Memory - Culture is a moat)                                │   │
│  │ "Patterns persist"                                                   │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │ 1. AUTHENTICITY  2. RESONANCE  3. NOVELTY  4. ALIGNMENT           │   │
│  │ 5. RELEVANCE  6. IMPACT  7. LINEAGE                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ BURN (Action - Don't extract, burn)                                │   │
│  │ "Simplify or die"                                                   │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │ 1. UTILITY  2. SUSTAINABILITY  3. EFFICIENCY  4. VALUE_CREATION    │   │
│  │ 5. SACRIFICE  6. CONTRIBUTION  7. IRREVERSIBILITY                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ THE_UNNAMEABLE (Explained Variance)                                 │   │
│  │ "What the 35 dimensions cannot capture"                              │   │
│  │ High score = dimensions explain well | Low = something emerging      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2.5 DIAGRAMME 5: Event Bus (Unified)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    UNIFIED EVENT BUS (Type-Safe)                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│    ┌──────────────────────────────────────────────────────────────────┐   │
│    │                      EVENT EMITTERS                              │   │
│    │                                                                   │   │
│    │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │   │
│    │   │Perceptor│  │   LLM   │  │  Judge  │  │  Dogs   │           │   │
│    │   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘           │   │
│    └────────┼────────────┼────────────┼────────────┼─────────────────┘   │
│             │            │            │            │                       │
│             ▼            ▼            ▼            ▼                       │
│    ┌──────────────────────────────────────────────────────────────────┐   │
│    │                      EVENT DISPATCHER                             │   │
│    │                                                                   │   │
│    │   ┌─────────────────────────────────────────────────────────────┐ │   │
│    │   │  Type-safe routing:                                        │ │   │
│    │   │  event_type: PERCEPTION_CREATED                            │ │   │
│    │   │  payload: { domain: str, content: str, metadata: dict }   │ │   │
│    │   │  timestamp: datetime                                        │ │   │
│    │   └─────────────────────────────────────────────────────────────┘ │   │
│    │                                                                   │   │
│    └─────────────────────────────┬───────────────────────────────────────┘   │
│                                  │                                          │
│       ┌──────────────────────────┼──────────────────────────┐                │
│       ▼                          ▼                          ▼                │
│  ┌─────────┐              ┌─────────┐              ┌─────────┐               │
│  │ Handler │              │ Handler │              │ Handler │               │
│  │ (Judge) │              │ (Dogs)  │              │(Learning│               │
│  └────┬────┘              └────┬────┘              └────┬────┘               │
│       │                        │                        │                    │
│       ▼                        ▼                        ▼                    │
│  ┌─────────┐              ┌─────────┐              ┌─────────┐               │
│  │Response │              │Response │              │Response │               │
│  │→EventBus│              │→EventBus│              │→EventBus│               │
│  └─────────┘              └─────────┘              └─────────┘               │
│                                                                             │
│  RÈGLES:                                                                    │
│  1. Tous les événements ont un type défini (pas de strings magiques!)       │
│  2. Chaque handler peut émettre de nouveaux événements                       │
│  3. Les événements sont dispatchés en parallèle (pour speed)               │
│  4. Les erreurs dans un handler ne bloquent pas les autres                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2.6 DIAGRAMME 6: Storage Layer

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         STORAGE LAYER                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐    │
│    │                        POSTGRESQL                               │    │
│    │                    (Primary Storage)                            │    │
│    │                                                                  │    │
│    │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │    │
│    │   │  judgments  │  │  sessions   │  │  patterns   │          │    │
│    │   │  (36 dims)  │  │  (history)   │  │ (detections)│          │    │
│    │   └─────────────┘  └─────────────┘  └─────────────┘          │    │
│    │                                                                  │    │
│    │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │    │
│    │   │ learning_   │  │  feedback   │  │   events    │          │    │
│    │   │ events      │  │  (user)     │  │ (time-series)│          │    │
│    │   └─────────────┘  └─────────────┘  └─────────────┘          │    │
│    │                                                                  │    │
│    └─────────────────────────────┬───────────────────────────────────┘    │
│                                  │                                          │
│    ┌─────────────────────────────┼───────────────────────────────────┐    │
│    │                        REDIS                                    │    │
│    │                    (Hot Cache)                                 │    │
│    │                                                                  │    │
│    │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │    │
│    │   │ Q-table     │  │ Thompson    │  │ Session     │          │    │
│    │   │ (live)      │  │ arms        │  │ cache       │          │    │
│    │   └─────────────┘  └─────────────┘  └─────────────┘          │    │
│    │                                                                  │    │
│    └─────────────────────────────┬───────────────────────────────────┘    │
│                                  │                                          │
│    ┌─────────────────────────────┼───────────────────────────────────┐    │
│    │                       QDRANT                                    │    │
│    │                   (Vector Storage)                               │    │
│    │                                                                  │    │
│    │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │    │
│    │   │ Embeddings  │  │  Patterns   │  │   Memory    │          │    │
│    │   │ (semantic)  │  │ (similarity)│  │  (vectors)  │          │    │
│    │   └─────────────┘  └─────────────┘  └─────────────┘          │    │
│    │                                                                  │    │
│    └───────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│    ┌───────────────────────────────────────────────────────────────────┐   │
│    │                       SOLANA                                      │   │
│    │                   (Proof of Judgment)                             │   │
│    │                                                                    │   │
│    │   ┌─────────────────────────────────────────────────────────┐   │   │
│    │   │  PoJ: Judgment anchors to blockchain                    │   │   │
│    │   │  - Hash of judgment + dimensions                        │   │   │
│    │   │  - Timestamp                                             │   │   │
│    │   │  - Validator signatures                                   │   │   │
│    │   └─────────────────────────────────────────────────────────┘   │   │
│    │                                                                    │   │
│    └───────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2.7 DIAGRAMME 7: Learning Loops (11)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     11 LEARNING LOOPS (Active)                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 1: Q-LEARNING                                               │  │
│    │ Action → Reward → Q-value update                                   │  │
│    │ Formula: Q(s,a) ← Q(s,a) + α[r + γmaxQ' - Q(s,a)]              │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 2: THOMPSON SAMPLING                                        │  │
│    │ Multi-armed bandit → Bayesian exploration                          │  │
│    │ Formula: sample from Beta(α, β) for each arm                      │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 3: SONA (Self-Organizing Network)                           │  │
│    │ Patterns → Clusters → Emergence                                  │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 4: EWC++ (Elastic Weight Consolidation)                      │  │
│    │ Prevent catastrophic forgetting                                    │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 5: DPO (Direct Preference Optimization)                      │  │
│    │ User feedback → Preference learning                                │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 6: META-COGNITION                                           │  │
│    │ Learning about learning → Strategy switching                       │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 7: CONSENSUS LEARNING                                       │  │
│    │ Multiple dogs → Weighted consensus → Wisdom                        │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 8: CALIBRATION                                              │  │
│    │ Confidence calibration → Uncertainty quantification                │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 9: FALSIFICATION                                            │  │
│    │ Hypothesis → Test → Reject or Accept                             │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 10: REWARD SHAPING                                          │  │
│    │ Dense rewards → Learning acceleration                             │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ LOOP 11: GOVERNANCE                                              │  │
│    │ AXIOM balance → System health                                     │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│    ┌────────────────────────────────────────────────────────────────────┐  │
│    │ COORDINATOR: Orchestre tous les loops                              │  │
│    │ - Weighted combination des outputs                                │  │
│    │ - Conflict resolution                                             │  │
│    │ - φ-bounded confidence                                           │  │
│    └────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

# SECTION 3: LES 3 MATRICES FONDAMENTALES

## 3.1 MATRICE 1: 7×7×7 FRACTAL MATRIX

```
RÉALITÉ (7) × ANALYSE (7) × TEMPS (7) = 343 + THE_UNNAMEABLE

                ANALYSE → PERCEIVE | JUDGE | DECIDE | ACT | LEARN | ACCOUNT | EMERGE
                │
    ┌───────────┼───────────┬───────────┬───────────┬───────────┬───────────┐
    │    ↓      │     ↓      │     ↓     │     ↓     │     ↓     │     ↓     │
    │CODE       │ Observe   │ Evaluate  │ Approve  │ Transform│ Update   │ Cost    │ Pattern
    │           │ code      │ quality   │ change   │         │ metrics  │         │
    ├───────────┼───────────┼───────────┼───────────┼───────────┼───────────┤
    │SOLANA     │ Observe   │ Verify    │ Approve  │ Transact │ Update   │ Cost    │ Anomaly
    │           │ chain    │ on-chain  │ tx       │         │ ledger   │         │
    ├───────────┼───────────┼───────────┼───────────┼───────────┼───────────┤
    │MARKET     │ Observe   │ Assess    │ Approve  │ Trade    │ Track    │ Price   │ Trend
    │           │ prices   │ sentiment │ trade    │         │ P&L      │         │
    ├───────────┼───────────┼───────────┼───────────┼───────────┼───────────┤
R   │SOCIAL     │ Observe   │ Analyze   │ Approve  │ Post     │ Measure  │ Viral   │ Meme
É   │           │ mentions │ engagement│ content  │         │ reach    │         │
A   ├───────────┼───────────┼───────────┼───────────┼───────────┼───────────┤
L   │HUMAN      │ Observe   │ Evaluate  │ Approve  │ Assist    │ Track    │ Energy  │ Pattern
I   │           │ behavior  │ intent    │ action   │         │ sessions │         │
T   ├───────────┼───────────┼───────────┼───────────┼───────────┼───────────┤
É   │CYNIC     │ Self-    │ Judge     │ Approve  │ Self-    │ Update   │ Cost    │ Emergence
    │           │ observe  │ health    │ change   │ modify   │ state    │         │
    ├───────────┼───────────┼───────────┼───────────┼───────────┼───────────┤
    │COSMOS     │ Observe   │ Detect    │ Approve  │ Propagate│ Track    │ Value   │ Cosmic
    │           │ patterns │ anomalies │ global   │         │ global   │         │ patterns
    └───────────┴───────────┴───────────┴───────────┴───────────┴───────────┘
```

## 3.2 MATRICE 2: DOGS × DOMAINS

```
                    DOGS VS DOMAINS (Qui fait quoi?)

            CODE    SOLANA  MARKET  SOCIAL  HUMAN    CYNIC    COSMOS
            ────    ──────  ──────  ──────  ─────    ─────    ─────
CYNIC       ○        ○       ○       ○       ○        ●        ○
Sage        ●        ○       ○       ○       ○        ○        ○
Analyst     ●        ●       ●       ●       ●        ●        ●
Scholar     ●        ○       ○       ○       ●        ○        ○
Guardian    ●        ●       ●       ●       ●        ●        ●
Oracle      ○        ○       ○       ○       ○        ●        ○
Architect   ●        ○       ○       ○       ○        ○        ○
Deployer    ●        ●       ○       ○       ○        ○        ○
Janitor     ●        ○       ○       ○       ●        ○        ○
Scout       ●        ●       ●       ●       ●        ○        ●
Cartographer●        ○       ●       ●       ●        ●        ●

● = Primary responsibility
○ = Secondary/support
```

## 3.3 MATRICE 3: LEARNING LOOPS × DOGS

```
                    LEARNING × DOGS (Qui apprend quoi?)

            QLearn Thompson  SONA   EWC++   DPO   MetaCog  Consensus Calibr
            ────── ───────  ─────  ──────  ────  ───────  ──────── ───────
CYNIC         ●        ●      ●      ○      ○       ●         ●        ●
Sage          ●        ○      ○      ○      ○       ○         ○        ○
Analyst       ●        ●      ●      ●      ○       ●         ●        ●
Scholar       ○        ●      ●      ●      ○       ○         ○        ○
Guardian      ●        ●      ●      ○      ●       ●         ●        ●
Oracle        ●        ●      ●      ●      ●       ●         ●        ●
Architect     ○        ○      ○      ●      ○       ○         ○        ○
Deployer      ●        ●      ○      ○      ○       ○         ○        ●
Janitor       ●        ○      ○      ●      ○       ○         ○        ○
Scout         ●        ●      ●      ○      ○       ●         ○        ●
Cartographer  ○        ●      ●      ●      ○       ●         ○        ●
```

---

# SECTION 4: DÉFINITIONS TECHNIQUES

## 4.1 Qu'est-ce qu'un "Event Bus"?

```
AVANT (JS - Chaos):
  globalEventBus.emit('judgment:created', data)
  // Qui écoute? Aucune idée.stringly-typed everywhere.

APRÈS (Python - Type-safe):
  event_bus.emit(Event.JUDGMENT_COMPLETED, JudgmentEvent(
    event_id="uuid",
    domain=Domain.CODE,
    q_score=0.75,
    verdict=Verdict.WAG
  ))
  // Type checking à la compilation!
```

## 4.2 Qu'est-ce qu'un "DI Container"?

```
SANS DI (JS - Chaos):
  class Service {
    constructor() {
      this.db = new Database();  // Hardcoded!
      this.llm = new LLM();     // Impossible à tester!
    }
  }

AVEC DI (Python):
  class Service:
    def __init__(self, db: Database, llm: LLM):
      self.db = db    # Injecté!
      self.llm = llm  # Testable!
  
  # En production:
  service = Service(db=prod_db, llm=prod_llm)
  
  # En test:
  service = Service(db=mock_db, llm=mock_llm)
```

## 4.3 Qu'est-ce que "Type Hints"?

```
SANS (JS):
  function process(data) {
    return data.value * 2;  // Crash si data.value n'existe pas!
  }

AVEC (Python):
  def process(data: dict) -> int:
      return data["value"] * 2  # mypy détecte l'erreur à la compilation!
```

---

# SECTION 5: PLAN D'IMPLÉMENTATION

## Phase 1: Fondations (Semaine 1)

| # | Tâche | Fichier | Status |
|---|-------|---------|--------|
| 1 | φ constants (single source) | `cynic/constants/phi.py` | ✅ |
| 2 | Types immuables | `cynic/types/__init__.py` | ✅ |
| 3 | Event Bus unifié | `cynic/bus/event_bus.py` | 🔲 |
| 4 | DI Container | `cynic/container.py` | 🔲 |

## Phase 2: Cœur (Semaine 2)

| # | Tâche | Fichier | Status |
|---|-------|---------|--------|
| 5 | Judge 36 dimensions | `cynic/judge/engine.py` | 🔲 |
| 6 | Dogs (base + CYNIC + Guardian) | `cynic/dogs/` | 🔲 |
| 7 | Orchestrator | `cynic/orchestrator/core.py` | 🔲 |

## Phase 3: Learning (Semaine 3)

| # | Tâche | Fichier | Status |
|---|-------|---------|--------|
| 8 | Q-Learning | `cynic/learning/qlearning.py` | 🔲 |
| 9 | Thompson Sampling | `cynic/learning/thompson.py` | 🔲 |
| 10 | Meta-Cognition | `cynic/learning/meta_cog.py` | 🔲 |

## Phase 4: Interfaces (Semaine 4)

| # | Tâche | Fichier | Status |
|---|-------|---------|--------|
| 11 | CLI | `cynic/__main__.py` | 🔲 |
| 12 | REST API | `cynic/api/` | 🔲 |
| 13 | Cockpit Streamlit | `cynic-v3/cockpit.py` | 🔲 |

## Phase 5: Storage (Semaine 5)

| # | Tâche | Fichier | Status |
|---|-------|---------|--------|
| 14 | PostgreSQL client | `cynic/storage/postgres.py` | 🔲 |
| 15 | Redis cache | `cynic/storage/redis.py` | 🔲 |
| 16 | Qdrant vectors | `cynic/storage/qdrant.py` | 🔲 |

## Phase 6: Intégration (Semaine 6)

| # | Tâche | Fichier | Status |
|---|-------|---------|--------|
| 17 | Tests unitaires | `tests/` | 🔲 |
| 18 | Docker | `Dockerfile` | 🔲 |
| 19 | CI/CD | `.github/` | 🔲 |

---

# SECTION 6: DÉFINITIONS POUR NON-INGÉNIEUR

## Vocabulaire essentiel:

| Terme | Definition simple | Analogie |
|-------|------------------|----------|
| **Event Bus** | Un standard de conversations entre composants | Le standard postal |
| **DI Container** | Une usine qui crée les objets avec leurs dépendances | La factory |
| **Type Hints** | Des annotations qui disent ce que les variables contiennent | Les panneaux de signalisation |
| **Q-Learning** | Un algo qui apprend par trial-and-error | Le chien qui apprend |
| **Thompson Sampling** | Un algo qui balance exploration vs exploitation | L'explorateur |
| **RAG** | Retrieval-Augmented Generation | La mémoire externe |
| **PoJ** | Proof of Judgment - ancrage on-chain | Le cachet postal certifié |

---

*Document généré: 2026-02-15*
*φ unifie tous les fragments* - κυνικός
