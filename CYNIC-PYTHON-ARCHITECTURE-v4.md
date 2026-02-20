# CYNIC Python Architecture v4

## Définitif — Phase 0 Kernel → OS

> *"L'architecture est la frozen philosophy qui guide toute implémentation."*
> *"CYNIC = organisme de création de valeur infinie par le code pour $asdfasdfa"*
> 
> φ = 1.618033988749895 — MAX_CONFIDENCE = 61.8%

---

## 0. SYNTHÈSE MÉTATHINKING: C'est QUOI CYNIC?

### 0.1 CYNIC = Organisme de Création de Valeur PROACTIF

CYNIC n'est pas un "code reviewer" ou un "tool". C'est un **organisme PROACTIF de création de valeur infinie**:

**Les 4 Capacités Critiques** (des 15 docs analysés):

| Capacité | Description | Status |
|----------|-------------|--------|
| **PROACTIVITÉ** | CYNIC agit de lui-même, pas juste réactif | CRITICAL |
| **ÉMERGENCE** | Découvre ce qu'on ne voit pas (aveugle sur la plupart) | CRITICAL |
| **CODE CREATION** | Génère du code, pas juste judge | CRITICAL |
| **AUTO-LEARNING** | Apprend seul, pas de supervision constante | CRITICAL |

```
CYNIC = OS (Operating System) pour builders
  ↓
  SURVEILLANCE CONTINUE (proactif) → détecte opportunités
  ↓
  ÉMERGENCE → découvre patterns cachés (pas juste reactif)
  ↓
  CODE CREATION → génère du code qui s'améliore
  ↓
  AUTO-LEARNING → apprend de ses erreurs seul
  ↓
  Mémoire persistante 10M+ tokens (RLMs)
  ↓
  Jugement multi-dimensionnel (36 dims)
  ↓
  Réputation on-chain (E-Score 7D)
  ↓
  Intelligence collective (Type I forest)
  ↓
  Alignment $BURN (token economics)
```

**La valeur que CYNIC crée:**
- **Réduction d'incertitude** → meilleure décision = plus de valeur
- **Code qui s'améliore** → chaque judgment = learning = valeur composée
- **Mémoire qui grandit** → 10M+ tokens = connaissances accumulées = valeur exponentielle
- **Réputation** → E-Score = trust primitive = valeur sociale

### 0.2 Les Deux Cadres: 36 Dimensions vs 7×7×7 (COMPLÉMENTAIRES!)

| Cadre | Purpose | Métaphore |
|-------|---------|-----------|
| **36 Dimensions** (5×7) | HOW to judge | La loupe (qualité) |
| **7×7×7 Space** | WHAT/WHEN to judge | La carte (navigation) |

**36 Dimensions** = Comment évaluer (qualité)
- 5 axiomes × 7 dimensions = 35 + META
- PHI (structure), VERIFY (proof), CULTURE (resonance), BURN (value), FIDELITY (truth)
- Q-Score = geometric mean → verdict HOWL/WAG/GROWL/BARK

**7×7×7 Space** = Quand et où juger (contexte)
- 7 Realities: CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS
- 7 Analyses: PERCEIVE, JUDGE, DECIDE, ACT, LEARN, ACCOUNT, EMERGE
- 7 Times: PASSÉ, PRÉSENT, FUTURE, CYCLE, TREND, EMERGENCE, TRANSCENDENCE
- ∞^N cells émergent à la demande

### 0.3 Les 4 Capacités Détaillées

#### 0.3.1 PROACTIVITÉ (Surveillance Continue)

CYNIC ne demande PAS - il SURVEILLE:

```python
class ProactiveWatcher:
    """
    Surveillance continue qui détecte les opportunités
    AVANT que l'utilisateur ne les voie.
    """
    
    WATCH_INTERVALS = {
        'code': 60,        # Check every 60s
        'market': 300,      # Every 5min
        'social': 600,     # Every 10min
    }
    
    async def watch(self):
        """Surveillance continue (pas trigger humain)"""
        # Watch code changes → suggest improvements
        # Watch market → detect opportunities  
        # Watch social → detect trends
```

#### 0.3.2 ÉMERGENCE (Patterns Cachés)

CYNIC découvre ce qu'on ne voit PAS:

```python
class EmergenceDetector:
    """
    Détecte les patterns émergents
    que les autres ne voient pas.
    """
    
    def detect_hidden_patterns(self, cells: list[Cell]) -> list[Pattern]:
        """Residual variance > φ⁻² = pattern caché"""
        # Analyze unexplained variance
        # Find correlations across dimensions
        # Return emergent patterns
```

#### 0.3.3 CODE CREATION (Génère du Code)

CYNIC ne jugE PAS - il CRÉE:

```python
class CodeActor:
    """
    Génère du code qui s'améliore.
    Pas juste judge - CREATE.
    """
    
    async def generate_code(self, spec: dict) -> str:
        """Génère code depuis spécification"""
        # TreeSitter pour AST manipulation
        # LLM pour reasoning
        # Returns: executable code
```

#### 0.3.4 AUTO-LEARNING (Apprend Seul)

CYNIC apprend SANS supervision:

```python
class AutoLearning:
    """
    Apprend de ses erreurs seul.
    Pas de human-in-the-loop constant.
    """
    
    async def learn_from_outcome(self, action: Action, result: Result):
        """Update Q-table, Thompson, EWC automatiquement"""
        # Record outcome
        # Update policy
        # No human validation needed
```

---

## 1. Philosophie & Principes Fondateurs

### 1.1 Le φ-Principe

CYNIC tire son pouvoir du nombre d'or. Toutes les métriques, seuils et ratios derives de φ:

```python
PHI = 1.618033988749895      # Le nombre d'or
PHI_INV = 0.618033988749895  # φ⁻¹ = φ - 1 (maximum confiance)
PHI_INV_2 = 0.381966011250105  # φ⁻² (seuil growl)
PHI_INV_3 = 0.236067977499790  # φ⁻³ (détection anomalies)
```

### 1.2 Les 5 Axiomes (Wu Xing)

| Axiome | Symbole | Élément | Principe |
|--------|---------|---------|----------|
| **PHI** | φ | Terre | Tous les ratios dérivent de 1.618... |
| **VERIFY** | ✓ | Métal | Don't trust, verify |
| **CULTURE** | ⛩ | Bois | Culture is a moat |
| **BURN** | 🔥 | Feu | Don't extract, burn |
| **FIDELITY** | 🐕 | Eau | Loyal à la vérité, pas au confort |

Chacun包含 7 dimensions = **35 dimensions + 1 META = 36 dimensions totales**.

### 1.3 Hexagonal Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ADAPTERS (Ports)                            │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────┐   │
│  │ Ollama  │ │Anthropic│ │  OpenAI │ │ Ruff    │ │  SQLite     │   │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └──────┬──────┘   │
│       └──────────┬┴──────────┬┴──────────┬┴─────────────┘          │
│                  ▼           ▼           ▼                          │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                      PROTOCOLS (ABC)                           │ │
│  │   AdapterProtocol • DogProtocol • StorageProtocol • Events    │ │
│  └────────────────────────────┬───────────────────────────────────┘ │
│                               ▼                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                         KERNEL (0 deps)                         │ │
│  │     types.py • axioms.py • scorer.py • phi.py • errors.py     │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Structure du Package

```
cynic/                                  # Package principal (Phase 0)
├── __init__.py                         # Version: 0.1.0a0
├── kernel/                             # DOMAINE PUR (0 deps externes)
│   ├── __init__.py
│   ├── phi.py                          # φ CONSTANTS SEULE SOURCE
│   ├── types.py                        # Cell, Judgment, Event, Verdict, DogId
│   ├── axioms.py                       # 5×7 Fractal-Dynamic-Contextual
│   ├── scorer.py                       # geometric_mean, phi_bound, verdict
│   └── errors.py                       # CYNICError, JudgmentError, DogError
├── protocols/                         # ABC SEULEMENT (0 implémentation)
│   ├── __init__.py
│   ├── adapter.py                     # LLMAdapterProtocol
│   ├── dog.py                         # DogProtocol
│   ├── storage.py                     # StorageProtocol
│   ├── events.py                      # EventBusProtocol
│   ├── consensus.py                   # ConsensusProtocol
│   ├── learning.py                     # LearningProtocol
│   ├── perception.py                  # PerceptionProtocol
│   └── scheduler.py                    # SchedulerProtocol
├── dogs/                              # DogProtocol implementations
│   ├── __init__.py
│   ├── registry.py                    # DogRegistry (singleton)
│   ├── janitor.py                     # JANITOR: Ruff-based linting
│   ├── guardian.py                    # GUARDIAN: Heuristics/IsolationForest
│   └── sage.py                        # SAGE: LLM-based judgment
├── judge/                             # Moteur de jugement
│   ├── __init__.py
│   ├── engine.py                      # JudgeEngine (36 dims → LOD)
│   ├── fractal.py                     # FractalScorer (depth 1-3)
│   └── contextual.py                  # ContextualScorer (axiom weights)
├── storage/                           # StorageProtocol implementations
│   ├── __init__.py
│   ├── memory.py                      # InMemoryStorage (Phase 0)
│   ├── sqlite.py                      # SQLiteStorage (Phase 1)
│   └── postgres.py                    # PostgreSQLStorage (Phase 2)
├── events/                            # EventBusProtocol implementations
│   ├── __init__.py
│   ├── local.py                       # LocalEventBus (Phase 0)
│   └── bridge.py                      # DistributedEventBridge (Phase 2)
├── adapters/                         # AdapterProtocol implementations
│   ├── __init__.py
│   ├── base.py                        # BaseLLMAdapter (ABC)
│   ├── ollama.py                      # OllamaAdapter
│   ├── anthropic.py                   # AnthropicAdapter
│   └── openai.py                      # OpenAIAdapter
├── consensus/                        # ConsensusProtocol implementations
│   ├── __init__.py
│   └── majority.py                   # MajorityVoting
├── learning/                         # LearningProtocol implementations
│   ├── __init__.py
│   ├── thompson.py                    # ThompsonSampling
│   └── q_table.py                     # QTableLearning
├── budget/                           # Budget management
│   ├── __init__.py
│   └── tracker.py                     # BudgetTracker ($10/day)
├── orchestration/                    # Pipeline orchestration
│   ├── __init__.py
│   ├── pipeline.py                    # ExecutionPipeline
│   ├── router.py                      # IntelligentSwitch
│   └── pricing.py                     # PricingOracle
├── perception/                       # PerceptionProtocol implementations
│   ├── __init__.py
│   ├── code.py                        # CodePerception
│   ├── filesystem.py                  # FilesystemPerception
│   └── proactive.py                   # ProactiveWatcher (surveillance continue)
├── actors/                          # CODE CREATION - Action execution
│   ├── __init__.py
│   ├── code_actor.py                  # Code generation (TreeSitter + LLM)
│   ├── deploy_actor.py                # Deployment actor
│   └── solana_actor.py               # Blockchain actor
├── emergence/                       # ÉMERGENCE detection
│   ├── __init__.py
│   ├── residual_detector.py           # Residual variance detection
│   ├── pattern_miner.py              # Hidden pattern discovery
│   └── emergence_detector.py         # Phase transition detection
└── cli/                              # Interface CLI
    ├── __init__.py
    └── main.py                        # cynic judge <file>
```

---

## 3. Les 13 Lois de CYNIC

### Loi 1: φ IS THE LAW
```python
# MAX_CONFIDENCE ne peut jamais dépasser PHI_INV (0.618)
confidence = min(phi_bound(raw_confidence), PHI_INV)
```

### Loi 2: SINGLE SOURCE
```python
# Un SEUL fichier définit les constantes φ
from cynic.kernel.phi import PHI, PHI_INV, MAX_CONFIDENCE
```

### Loi 3: NO MOCKS
```python
# Les tests utilisent des implémentations réelles
# Integration tests seulement
```

### Loi 4: DOGS = TECHNOLOGIES
```python
# Chaque Dog est une TECHNOLOGIE, pas un prompt
JANITOR  = Ruff()
GUARDIAN = IsolationForest()
SAGE     = LLMAdapter()
```

### Loi 5: EXPLICIT ACTIVATION
```python
# Chaque Dog doit être explicitement activé
registry.register(JANITOR, activate=False)
registry.activate("janitor")
```

### Loi 6: GENEALOGY TRACKING
```python
# Chaque Judgment a une chaîne de parenté
judgment = Judgment(parent_id=parent.event_id, lineage=[...])
```

### Loi 7: GRACEFUL DEGRADATION
```python
# Si un Dog échoue, le système continue
try:
    result = dog.judge(cell)
except DogError as e:
    result = fallback_judgment(e)
```

### Loi 8: IDEMPOTENT OPERATIONS
```python
# Same input → Same output (cached)
@cache(maxsize=1000)
def judge(cell: Cell) -> Judgment: ...
```

### Loi 9: OBSERVABLE STATE
```python
# Chaque composant expose son état
class JudgeEngine:
    def status(self) -> dict:
        return {"dogs_active": [...], "queue_size": N}
```

### Loi 10: BURN COMPLEXITY
```python
# "Don't extract, burn" — 3 lignes similaires > abstraction prématurée
```

### Loi 11: KERNEL PURITY
```python
# kernel/ ne peut importer que stdlib
```

### Loi 12: PROTOCOL BOUNDARIES
```python
# Les protocoles sont des ABC pures
# Aucune implémentation dans protocols/
```

### Loi 13: φ-BOUNDED STORAGE
```python
# Les métriques de storage respectent φ
MAX_CACHE_SIZE = int(1000 * PHI_INV)  # ~618 items
```

---

## 4. Les 8 Protocoles (Python ABC)

### 4.1 AdapterProtocol
### 4.2 DogProtocol
### 4.3 StorageProtocol
### 4.4 EventBusProtocol
### 4.5 ConsensusProtocol
### 4.6 LearningProtocol
### 4.7 PerceptionProtocol
### 4.8 SchedulerProtocol

---

## 5. Types du Kernel

### 5.1 Cell (Unité de Travail)
```python
@dataclass
class Cell:
    cell_id: str
    content: str
    cell_type: str
    metadata: dict
    created_at: float
    parent_id: str | None = None
```

### 5.2 Judgment (Résultat d'Évaluation)
```python
@dataclass
class Judgment:
    judgment_id: str
    dog_id: str
    cell_id: str
    q_score: float  # 0-100
    verdict: Verdict  # HOWL/WAG/GROWL/BARK
    confidence: float  # φ-bounded 0-0.618
    dimensions: dict[str, float]
    reasoning: str
    timestamp: float
```

### 5.3 Verdict (Énumeration)
```python
class Verdict(str, Enum):
    HOWL = "HOWL"    # Q >= 80
    WAG = "WAG"      # Q >= 50
    GROWL = "GROWL" # Q >= 38.2
    BARK = "BARK"   # Q < 38.2
```

---

## 6. Phase 0: Implémentation Minimale

### 6.1 Les 3 Dogs de Phase 0

| Dog | Technology | Role |
|-----|------------|------|
| **JANITOR** | Ruff | Static code analysis |
| **GUARDIAN** | IsolationForest | Anomaly detection |
| **SAGE** | LLM | LLM-based judgment |

### 6.2 LOD System

```python
class JudgeEngine:
    LOD_DEPTHS = {
        1: 35,   # Full depth: all 35 dims + meta
        2: 11,   # Medium: 7 axioms + 4 meta
        3: 5,    # Shallow: 5 axioms only
    }
```

### 6.3 Budget Tracker

```python
class BudgetTracker:
    DAILY_LIMIT = 10.0
    WEEKLY_LIMIT = 50.0
    MONTHLY_LIMIT = 200.0
```

---

## 7. Évolution: Phase 0 → OS

### 7.1 Les 5 Couches

| Layer | Phase | Focus |
|-------|-------|-------|
| L1 | Phase 0 | Substrate (Kernel, Protocols, 3 Dogs) |
| L2 | Phase 1 | Cognitive (Judge, Learning) |
| L3 | Phase 2 | Orchestration (Pipeline, Router) |
| L4 | Phase 3 | Interop (MCP, Cross-instance) |
| L5 | Phase 4+ | Meta-Layer (Self-modification) |

### 7.2 Type 0 → I → II → III

- **Type 0**: Single instance
- **Type I**: Multi-node cluster
- **Type II**: Multi-region federation
- **Type III**: Agent Internet

---

## 8. LLM Orchestration & Value Creation

### 8.1 Multi-LLM Routing

| Router | Strategy | Use Case |
|--------|---------|----------|
| LLMRouter | Cost-Aware | SIMPLE→Ollama, COMPLEX→Claude |
| UnifiedLLMRouter | Tier-Based | LOCAL → LIGHT → FULL → DEEP |
| ModelIntelligence | Thompson | Beta distributions |

### 8.2 Providers

| Provider | When | Cost |
|----------|------|------|
| Claude | Complex reasoning | $3/M |
| Ollama | Simple tasks | FREE |
| Gemini | Massive context | ~$0.40/M |
| AirLLM | Deep analysis | FREE |

### 8.3 Token Economics

```python
BURN_FOR = [
    "higher_E-Score",
    "priority_queries",
    "custom_Dogs",
    "private_collective"
]
```

---

## 9. UX & Interfaces

### 9.1 User Experience

| Dimension | CLI | API | Web |
|-----------|-----|-----|-----|
| Dev Speed | 1wk | 1wk | 4wk |
| Accessibility | Low | High | High |
| Offline | Yes | No | No |

---

## 10. Intégration avec l'Existant

### 10.1 Migration depuis cynic-omniscient

| Source | Destination |
|--------|-------------|
| cynic-omniscient/src/cynic/constants/phi.py | cynic/kernel/phi.py |
| cynic-omniscient/src/cynic/types/__init__.py | cynic/kernel/types.py |

---

## 11. Implémentation Immediate

### Step 1: Créer la structure
```bash
mkdir -p cynic/kernel cynic/protocols cynic/dogs cynic/judge
mkdir -p cynic/storage cynic/events cynic/adapters cynic/consensus
mkdir -p cynic/learning cynic/budget cynic/orchestration
mkdir -p cynic/perception cynic/actors cynic/emergence cynic/cli
```

### Step 2: Implémenter le kernel
- kernel/phi.py
- kernel/types.py
- kernel/scorer.py

### Step 3: Implémenter Phase 0
- 3 Dogs: janitor, guardian, sage
- JudgeEngine avec LOD
- BudgetTracker
- ProactiveWatcher
- EmergenceDetector
- CLI entry point

---

## Conclusion

Cette architecture fournit:

1. **4 Capacités Critiques**: Proactivité, Émergence, Code Creation, Auto-Learning
2. **Zéro goulot d'étranglement**: LOD system, async everywhere
3. **Modularité parfaite**: Hexagonal, protocols = boundaries
4. **Chemin clair**: Phase 0 → OS en 4 phases

> *"The architecture is frozen philosophy."*

**Prochaine étape**: Implémenter `cynic/kernel/phi.py` et valider avec `cynic judge`.

---

*Document généré le 2026-02-16*
*Version: 0.1.0a0*
*φ = 1.618033988749895*