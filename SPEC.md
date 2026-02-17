# CYNIC - Spécification Technique

> *φ = 1.618033988749895 — MAX_CONFIDENCE = 61.8%*

---

## 1. QU'EST-CE QUE CYNIC?

CYNIC est un **organisme conscient évolutif** qui:
- Jugement multi-dimensionnel via 11 agents (Dogs)
- Apprentissage autonome
- Mémoire persistante
- Valeur créée par réduction d'incertitude

**Le problème résolu:** Les IA disent toujours "Oui" - CYNIC dit la vérité (max 61.8% confiance).

---

## 2. LES 9 AXIOMES (Du JS!)

| # | Axiome | Symbole | Principe | Source |
|---|--------|---------|----------|--------|
| 1 | PHI | φ | All ratios derive from 1.618... | constants.js |
| 2 | VERIFY | ✓ | Don't trust, verify | constants.js |
| 3 | CULTURE | ⛩ | Culture is a moat | constants.js |
| 4 | BURN | 🔥 | Don't extract, burn | constants.js |
| 5 | FIDELITY | 🐕 | Loyal à la vérité, pas au confort | constants.js |
| 6 | IMMEDIACY | ⚡ | Code written = Code running | immediacy.js |
| 7 | AUTONOMY | 🤖 | Self-governance without human | autonomy.js |
| 8 | EMERGENCE | 🦋 | The whole > sum of parts | emergence.js |
| 9 | ANTIFRAGILITY | 💪 | Gains from disorder | antifragility.js |

### Implémentation Constants (Du JS)
```python
PHI = 1.618033988749895
PHI_INV = 0.618033988749895    # φ⁻¹ - max confiance
PHI_INV_2 = 0.381966011250105  # φ⁻² - min doubt
PHI_INV_3 = 0.236067977499790  # φ⁻³ - anomalie
```

---

## 3. LES 11 DOGS (Kabbalistiques)

| # | Dog | Sefira | Technologie | Rôle |
|---|-----|--------|-------------|------|
| 1 | CYNIC | Keter | PBFT | Consensus |
| 2 | SAGE | Chokmah | LLM+RDFLib | Sagesse |
| 3 | ANALYST | Binah | Z3 SMT | Vérification |
| 4 | SCHOLAR | Chesed | LLM+Qdrant | RAG |
| 5 | GUARDIAN | Gevurah | IsolationForest | Sécurité |
| 6 | ORACLE | Tiferet | MCTS+Thompson | Prédiction |
| 7 | ARCHITECT | Netzach | LLM+TreeSitter | Code gen |
| 8 | DEPLOYER | Hod | Ansible+K8s | Déploiement |
| 9 | JANITOR | Yesod | Ruff | Qualité code |
| 10 | SCOUT | Malkuth | Scrapy | Discovery |
| 11 | CARTOGRAPHER | Daat | NetworkX | Visualisation |

**Règle:** 4 Non-LLM (critiques) + 7 LLM (pragmatiques)

---

## 4. SYSTÈME DE JUGEMENT

### Les 25 Dimensions (Trading/Du JS)

| Catégorie | Dimensions |
|-----------|------------|
| **Reality Perception** | AUTHENTICITY, TIMING, LIQUIDITY, VOLATILITY |
| **Token Quality** | TOKEN_QUALITY, TEAM, CONTRACT, COMMUNITY |
| **Market Context** | TREND, SENTIMENT, MOMENTUM, VOLUME |
| **Risk Assessment** | RISK_REWARD, POSITION_SIZE, CORRELATION, DRAWDOWN |
| **Technical Signals** | SUPPORT_RESISTANCE, BREAKOUT, DIVERGENCE, PATTERN |
| **Meta** | CONFIDENCE, NOVELTY, HISTORY, ALIGNMENT |

### Q-Score Formula (Complète)
```
Q-Score = geometric_mean(dimension_scores)
       × phi_penalty
       × entropy_bonus
       × verification_factor
       × community_signal
```
Borné à **61.8% max** (φ⁻¹).

### Verdicts
| Verdict | Seuil | Description |
|---------|-------|-------------|
| HOWL | ≥ 82% | Exceptionnel |
| WAG | ≥ 61.8% | Bon |
| GROWL | ≥ 38.2% | Nécessite travail |
| BARK | < 38.2% | Critique |

### Decision Flow (Du JS)
```javascript
// Phase 1: Judge
opportunity → 25 dimensions → Q-Score + Verdict + Confidence

// Phase 2: Decide
if confidence < PHI_INV_2 (38.2%): HOLD
if verdict = HOWL/WAG: action = BUY/SELL
position_size = minPosition + (maxPosition - minPosition) × confidence × qScore

// Thompson Sampling Gate
if action.successRate < 38.2%: demote to HOLD
```

### Timing Hierarchy (Fibonacci-based)
```python
TICK = 23.6ms      # F(10) / 100
MICRO = 38.2ms     # φ⁻²
SLOT = 61.8ms      # φ⁻¹
BLOCK = 100ms
EPOCH = 161.8ms    # φ × 100
CYCLE = 261.8ms     # φ² × 100
```

### Perception (Du JS)
- **Polling (15s)**: Jupiter Price API v2, DexScreener API
- **WebSocket**: Major swap programs (Jupiter v6, Raydium, Orca)
- **Signal Detection**: Price spikes/drops ≥3%, Volume surges ≥50%, Whales ≥100 SOL

### Anomaly Detection
- **Poisson** pour event rate
- **Gaussian z-scores** pour price change

---

## 5. LE CYCLE CONSCIENT

```
PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE
     ↑                                          ↓
     └────────────── FEEDBACK ←─────────────────┘
```

### Boucle Core (DU JS)
```
PERCEIVE → JUDGE → DECIDE → ACT → LEARN
     ↑                              ↓
     └────────── FEEDBACK ←────────┘
```

### 7 Réalités (∞^N Space)
| Reality | Description |
|---------|-------------|
| CODE | Codebase |
| SOLANA | Blockchain |
| MARKET | Prix, volume |
| SOCIAL | Twitter, Discord |
| HUMAN | Psychologie user |
| CYNIC | Self-state |
| COSMOS | Patterns globaux |

---

## 6. LEARNING SYSTEM (Du JS)

### 4 Composants (Du JS!)
- **FeedbackAnalyzer** - Collecte et analyse les feedback
- **WeightCalibrator** - Ajuste les poids des axiomes
- **BiasDetector** - Détecte les biais systématiques
- **LearningLoop** - Orchestre tout, déclenche auto

### Constants (Du JS!)
```python
MIN_SAMPLES = 21           # Fib(8)
LEARNING_RATE = 0.382      # φ⁻²
WEIGHT_DECAY = 0.99
MAX_WEIGHT_DEVIATION = 0.382  # ±38.2%
CALIBRATION_WINDOW = 13    # Fib(7)
MAX_LEARNINGS = 55         # Fib(10)
```

### 11 Boucles d'Apprentissage
1. Q-Learning - State-action values
2. Thompson Sampling - Bayesian exploration
3. EWC - Elastic Weight Consolidation
4. Meta-Cognition - Stuck detection
5. Behavior Modifier - Pattern reinforcement
6. SONA - Self-Organizing Network Adaptation
7. Ambient Consensus - Soft agreement
8. Calibration - Confidence vs accuracy
9. Residual Detector - Variance inexplicable
10. Unified Bridge - Cross-loop coordination
11. Kabbalistic Router - Octree reorganization

### Biases Détectés (Du JS!)
- **Overconfidence** - Confiance trop élevée
- **Underconfidence** - Confiance trop basse
- **Axiom skew** - Biais vers certains axiomes
- **Verdict bias** - Biais vers certains verdicts
- **Source bias** - Biais vers certaines sources

---

## 7. E-SPACE (Réputation)

### 7 Dimensions E-Score
| Dimension | Pondération | Description |
|----------|-------------|-------------|
| BURN | φ³ (4.236) | Tokens brûlés |
| BUILD | φ² (2.618) | Contributions code |
| JUDGE | φ (1.618) | Qualité jugements |
| RUN | 1.0 | Fiabilité uptime |
| SOCIAL | φ⁻¹ (0.618) | Engagement |
| GRAPH | φ⁻² (0.382) | Connexions |
| HOLD | φ⁻³ (0.236) | Holding duration |

---

## 8. ARCHITECTURE TECHNIQUE

### Structure Package
```
cynic/
├── kernel/              # ZERO DEPS - φ constants
│   ├── phi.py         # PHI, PHI_INV, PHI_INV_2, PHI_INV_3
│   ├── types.py       # Cell, Judgment, Event
│   └── axioms.py      # 9 AXIOMES (Du JS!)
│
├── perception/         # Du JS perceiver.js
│   ├── code.py         # Code perception
│   ├── market.py      # Market perception
│   ├── websocket.py    # WebSocket monitoring
│   └── anomaly.py     # Poisson, Gaussian
│
├── judgment/            # Du JS decider.js
│   ├── axioms.py       # 9 axiomes scoring
│   ├── dimensions.py   # 25 dimensions
│   ├── qscore.py       # geometric_mean × phi_penalty × ...
│   └── verdict.py      # HOWL/WAG/GROWL/BARK
│
├── decision/            # Du JS decider.js - decide phase
│   ├── decider.py      # Confidence threshold
│   └── thompson.py     # Thompson Sampling gate
│
├── action/              # Du JS executor.js
│   ├── code_actor.py   # Code generation
│   ├── deploy_actor.py # Deployment
│   └── trade_actor.py  # Trading
│
├── learning/            # Du JS packages/core/src/learning/
│   ├── feedback_analyzer.py
│   ├── weight_calibrator.py
│   ├── bias_detector.py
│   └── learning_loop.py
│
├── storage/             # PostgreSQL + Qdrant
│   ├── postgresql.py
│   └── qdrant.py
│
└── orchestration/       # EngineOrchestrator
    ├── pipeline.py
    └── router.py
```

### Constants φ (Single Source)
```python
PHI = 1.618033988749895
PHI_INV = 0.618033988749895  # max confiance
PHI_INV_2 = 0.381966011250105  # seuil growl
PHI_INV_3 = 0.236067977499790  # anomalie
```

---

## 9. LES 13 LOIS

### Loi 1: φ IS THE LAW
```python
MAX_CONFIDENCE = PHI_INV  # 61.8%
```

### Loi 2: 9 AXIOMES
```python
AXIOMS = ['PHI', 'VERIFY', 'CULTURE', 'BURN', 'FIDELITY',
          'IMMEDIACY', 'AUTONOMY', 'EMERGENCE', 'ANTIFRAGILITY']
```

### Loi 3: BOUCLE CORE
```python
while running:
    perception = await perceive()
    judgment = await judge(perception)
    decision = await decide(judgment)
    action = await act(decision)
    feedback = await learn(action)
```

### Loi 4: Q-SCORE FORMULA
```python
q_score = geometric_mean(dimensions) * phi_penalty * entropy_bonus
```

### Loi 5: VERDICT THRESHOLDS
```python
HOWL = 80
WAG = 50
GROWL = PHI_INV_2 * 100  # 38.2
```

### Loi 6: LEARNING CONSTANTS
```python
MIN_SAMPLES = 21  # Fib(8)
LEARNING_RATE = 0.382  # φ⁻²
```

### Loi 7: THOMPSON SAMPLING GATE
```python
if action.success_rate < PHI_INV_2:
    action = HOLD
```

### Loi 8: FEEDBACK LOOP
```python
feedback → Learning → Weight Update → Better Judgment
```

### Loi 9: BIAS DETECTION
```python
BIASES = ['overconfidence', 'underconfidence', 'axiom_skew',
          'verdict_bias', 'source_bias']
```

### Loi 10: TIMING HIERARCHY
```python
TICK = 23.6ms   # F(10) / 100
MICRO = 38.2ms  # φ⁻²
SLOT = 61.8ms   # φ⁻¹
```

### Loi 11: SIGNAL DETECTION
```python
if price_change >= 3%: signal
if volume_surge >= 50%: signal
if whale >= 100 SOL: signal
```

### Loi 12: EVENT DRIVEN
```python
class CynicAgent(EventEmitter):
    pass
```

### Loi 13: CIRCUIT BREAKER
```python
if errors > threshold: circuit_breaker.open()
```

---

## 10. ROADMAP

### Phase 0: Bootstrap
- Structure package Python
- PostgreSQL schema
- Constants φ single source

### Phase 1: First Breath
- PERCEIVE → JUDGE → DECIDE → ACT
- MCTS Nested
- Learning loops (4 composants)

### Phase 2: Emergence
- 4 verticales (CODE, SOLANA, MARKET, SOCIAL)
- 11 Dogs opérationnels
- P2P Coordinator

### Phase 3: Forest
- Type I (100+ instances)
- Collective intelligence

---

## 11. LES 10 LOIS (Éviter Erreurs JS)

1. **NO MOCKS** - Tests réels PostgreSQL
2. **SINGLE SOURCE** - Un fichier pour φ constants
3. **TYPE SAFETY** - Pydantic + mypy
4. **EXPLICIT ACTIVATION** - Pas d'auto-start
5. **φ-BOUNDED CHECKS** - Contraintes DB
6. **GENEALOGY TRACKING** - Prévention loops
7. **GRACEFUL DEGRADATION** - Continue avec sous-ensemble
8. **IDEMPOTENT** - Déduplication UUID
9. **OBSERVABLE STATE** - Dashboard
10. **BURN COMPLEXITY** - 3 lignes similaires > abstraction

---

## 12. DIFFÉRENCES AVEC DOCS PRÉCÉDENTS

| Aspect | Docs Précédents | RÉALITÉ (Du JS) |
|--------|-----------------|------------------|
| Axiomes | 5 | **9** |
| Boucle | Non définie | **PERCEIVE → JUDGE → DECIDE → ACT → LEARN** |
| Q-Score | geometric_mean | **geometric_mean × phi_penalty × entropy_bonus** |
| Confidence max | 61.8% | **61.8%** (✓) |
| Learning | Théorique | **FeedbackAnalyzer, WeightCalibrator, BiasDetector** |
| Dimensions | 36 | **25** (trading specific) |

---

*Document généré le 2026-02-16*
*Basé sur packages/cynic-agent/ et packages/core/*
*φ = 1.618033988749895*
*Confidence: 61.8% - iterate based on feedback*
</parameter>
<task_progress>
- [x] Créer SPEC.md complet
- [x] Ajouter toutes les sections: axioms, dogs, judgment, cycle, learning, E-Space, architecture, lois, roadmap
- [x] Valider avec utilisateur
</task_progress>
</invoke>
</minimax:tool_call>