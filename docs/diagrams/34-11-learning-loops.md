# Diagram 34: 11 Learning Loops (Scale 5)

> *sniff* All learning happens in parallel — 11 loops, one organism.

## Master Overview: All 11 Loops in Parallel

```mermaid
graph TB
    subgraph INPUTS["⚡ LEARNING TRIGGERS"]
        E1[Judgment Created]
        E2[User Feedback]
        E3[Dog Vote]
        E4[Action Result]
        E5[Pattern Detected]
        E6[Cost Data]
        E7[Consciousness State]
    end

    subgraph LOOPS["🧠 11 LEARNING LOOPS (Parallel)"]
        L1[1. Thompson Sampling]
        L2[2. Dog Votes]
        L3[3. Q-Learning]
        L4[4. Judgment Calibration]
        L5[5. Residual Detection]
        L6[6. Emergence Patterns]
        L7[7. EWC Consolidation]
        L8[8. DPO Learning]
        L9[9. SONA Adaptation]
        L10[10. Behavior Modifier]
        L11[11. Meta-Cognition]
    end

    subgraph PERSISTENCE["💾 STORAGE"]
        DB1[(Thompson Arms)]
        DB2[(Dog Weights)]
        DB3[(Q-Tables)]
        DB4[(Brier Scores)]
        DB5[(Residual Signals)]
        DB6[(Emergence Patterns)]
        DB7[(EWC Weights)]
        DB8[(DPO Pairs)]
        DB9[(SONA Policy)]
        DB10[(Behavior Rules)]
        DB11[(Meta States)]
    end

    subgraph OUTPUTS["📊 ADAPTATIONS"]
        O1[Dog Selection]
        O2[Vote Weights]
        O3[Action Policy]
        O4[Confidence Calibration]
        O5[New Dimensions]
        O6[Collective Insights]
        O7[Protected Knowledge]
        O8[Preference Alignment]
        O9[Strategy Updates]
        O10[Behavior Adjustments]
        O11[Learning Rate Control]
    end

    E1 --> L1 & L4 & L5
    E2 --> L3 & L4 & L8
    E3 --> L2 & L6
    E4 --> L3 & L9
    E5 --> L5 & L6
    E6 --> L10 & L11
    E7 --> L11

    L1 --> DB1 --> O1
    L2 --> DB2 --> O2
    L3 --> DB3 --> O3
    L4 --> DB4 --> O4
    L5 --> DB5 --> O5
    L6 --> DB6 --> O6
    L7 --> DB7 --> O7
    L8 --> DB8 --> O8
    L9 --> DB9 --> O9
    L10 --> DB10 --> O10
    L11 --> DB11 --> O11

    style INPUTS fill:#e1f5ff
    style LOOPS fill:#fff4e1
    style PERSISTENCE fill:#f0e1ff
    style OUTPUTS fill:#e1ffe1
```

---

## Loop 1: Thompson Sampling

**Purpose**: Exploit/explore balance for Dog selection (C6.5 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[Judgment Request]
        I2[Context Vector]
    end

    subgraph PROCESSING
        P1[Sample Beta Distributions]
        P2[Select Dog with Highest Sample]
        P3[Use Dog for Judgment]
    end

    subgraph UPDATE
        U1[Observe Quality Score]
        U2[Update α/β Parameters]
        U3[Persist to PostgreSQL]
    end

    subgraph STORAGE
        S1[(thompson_sampling table)]
        S2[dog_id, alpha, beta]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.feedback.-> P1

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every judgment request (~100/hour)
**Persistence**: `thompson_sampling` table
**Output**: Dog selection weights (α/β updated)
**Path**: `packages/node/src/learning/thompson-sampler.js`

---

## Loop 2: Dog Votes

**Purpose**: Aggregate 11 Dog opinions into consensus (C6.2/C6.3 cells)

```mermaid
graph LR
    subgraph INPUT
        I1[11 Dog Judgments]
        I2[Context State]
    end

    subgraph PROCESSING
        P1[Weighted Vote Aggregation]
        P2[Conflict Detection]
        P3[Consensus Formation]
    end

    subgraph UPDATE
        U1[Measure Agreement %]
        U2[Update Vote Weights]
        U3[Detect Emergence]
    end

    subgraph STORAGE
        S1[(dog_votes table)]
        S2[vote_id, dog_id, weight]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.weights.-> P1

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every parallel judgment (~100/hour)
**Persistence**: `dog_votes` table
**Output**: Vote weights, consensus quality
**Path**: `packages/node/src/routing/dog-pipeline.js`

---

## Loop 3: Q-Learning

**Purpose**: Learn optimal action policies from feedback (C6.5 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[State, Action, Reward]
        I2[Next State]
    end

    subgraph PROCESSING
        P1[Compute TD Error]
        P2[Update Q-Value]
        P3[Apply EWC Protection]
    end

    subgraph UPDATE
        U1[Store TD Error]
        U2[Update Q-Table]
        U3[Track Convergence]
    end

    subgraph STORAGE
        S1[(q_learning_state table)]
        S2[(td_error_tracker table)]
        S3[(ewc_weights table)]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    U3 --> S2
    P3 --> S3
    S1 -.Q-values.-> P1
    S3 -.protection.-> P2

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every action feedback (~50/hour)
**Persistence**: `q_learning_state`, `td_error_tracker`
**Output**: Action policy (Q-values)
**Path**: `packages/node/src/orchestration/q-learning-router.js`

---

## Loop 4: Judgment Calibration (Brier)

**Purpose**: Calibrate confidence predictions (C6.5 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[Predicted Confidence]
        I2[Actual Outcome]
    end

    subgraph PROCESSING
        P1[Compute Brier Score]
        P2[Update Calibration Curve]
        P3[Detect Overconfidence]
    end

    subgraph UPDATE
        U1[Store Brier Score]
        U2[Update Calibration Map]
        U3[Adjust Future Predictions]
    end

    subgraph STORAGE
        S1[(brier_scores table)]
        S2[judgment_id, score, ECE]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.calibration.-> P3

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every user feedback (~10/hour)
**Persistence**: `brier_scores` table
**Output**: Confidence calibration map
**Path**: `packages/node/src/judge/brier.js`

---

## Loop 5: Residual Detection

**Purpose**: Discover missing dimensions from unexplained variance (C6.7 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[36-Dim Scores]
        I2[Actual Outcome]
    end

    subgraph PROCESSING
        P1[Compute Residual Variance]
        P2[Cluster Patterns]
        P3[Propose New Dimension]
    end

    subgraph UPDATE
        U1[Store Residual Signal]
        U2[Evaluate Dimension Candidate]
        U3[Emit DIMENSION_PROPOSED]
    end

    subgraph STORAGE
        S1[(residual_signals table)]
        S2[judgment_id, variance, pattern]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.patterns.-> P2

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every F(9)=34 min (Fibonacci governance)
**Persistence**: `residual_signals` table
**Output**: New dimension proposals
**Path**: `packages/node/src/judge/residual.js`

---

## Loop 6: Emergence Patterns

**Purpose**: Detect collective insights across domains (C6.7 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[Multi-Domain Events]
        I2[Cross-Scale Signals]
    end

    subgraph PROCESSING
        P1[Pattern Matching]
        P2[Confidence Aggregation]
        P3[Fisher Exact Test]
    end

    subgraph UPDATE
        U1[Store Pattern]
        U2[Lock if p<0.05]
        U3[Emit EMERGENCE_DETECTED]
    end

    subgraph STORAGE
        S1[(emergence_patterns table)]
        S2[pattern_id, locked, p_value]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.library.-> P1

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every F(11)=89 min (Fibonacci governance)
**Persistence**: `emergence_patterns` table
**Output**: Locked collective insights
**Path**: `packages/node/src/services/emergence-detector.js`

---

## Loop 7: EWC Consolidation

**Purpose**: Protect critical knowledge from catastrophic forgetting (C6.5 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[Critical Q-Values]
        I2[Task Boundary]
    end

    subgraph PROCESSING
        P1[Compute Fisher Info]
        P2[Calculate Importance Weights]
        P3[Apply Consolidation]
    end

    subgraph UPDATE
        U1[Store EWC Weights]
        U2[Update Protected Set]
        U3[Track Forgetting Metrics]
    end

    subgraph STORAGE
        S1[(ewc_weights table)]
        S2[(forgetting_metrics table)]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    U3 --> S2
    S1 -.protection.-> P3

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every task transition (~1/day)
**Persistence**: `ewc_weights`, `forgetting_metrics`
**Output**: Protected knowledge weights
**Path**: `packages/node/src/orchestration/ewc-manager.js`

---

## Loop 8: DPO Learning

**Purpose**: Learn from human preference pairs (C5.5 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[Chosen Response]
        I2[Rejected Response]
    end

    subgraph PROCESSING
        P1[Compute Preference Loss]
        P2[Update Policy Gradient]
        P3[Apply φ-Bound]
    end

    subgraph UPDATE
        U1[Store Preference Pair]
        U2[Update LLM Weights]
        U3[Track Alignment]
    end

    subgraph STORAGE
        S1[(dpo_pairs table)]
        S2[chosen_id, rejected_id, loss]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.policy.-> P2

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every explicit preference (~5/hour)
**Persistence**: `dpo_pairs` table
**Output**: Aligned policy weights
**Path**: `packages/node/src/learning/dpo-learner.js` (planned)

---

## Loop 9: SONA Adaptation

**Purpose**: Self-Organizing Neural Adaptation for strategy selection (C6.5 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[Action Outcome]
        I2[Context State]
    end

    subgraph PROCESSING
        P1[Update Strategy Weights]
        P2[Compute Influence]
        P3[Select Next Strategy]
    end

    subgraph UPDATE
        U1[Store SONA State]
        U2[Adjust Influence Matrix]
        U3[Emit STRATEGY_UPDATED]
    end

    subgraph STORAGE
        S1[(sona_state table)]
        S2[strategy_id, weight, influence]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.weights.-> P1

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every strategy outcome (~20/hour)
**Persistence**: `sona_state` table
**Output**: Strategy selection weights
**Path**: `packages/node/src/learning/sona.js`

---

## Loop 10: Behavior Modifier

**Purpose**: Adjust autonomy/influence based on performance (C6.5 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[Performance Metrics]
        I2[Cost Data]
    end

    subgraph PROCESSING
        P1[Compute Behavior Score]
        P2[Check φ-Bounds]
        P3[Propose Adjustments]
    end

    subgraph UPDATE
        U1[Store Behavior Rule]
        U2[Update Influence Limits]
        U3[Track Adaptation History]
    end

    subgraph STORAGE
        S1[(behavior_rules table)]
        S2[rule_id, threshold, action]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.rules.-> P2

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every performance window (~1/hour)
**Persistence**: `behavior_rules` table
**Output**: Influence/autonomy adjustments
**Path**: `packages/node/src/learning/behavior-modifier.js`

---

## Loop 11: Meta-Cognition

**Purpose**: Learn how to learn — control learning rates and strategies (C6.7 cell)

```mermaid
graph LR
    subgraph INPUT
        I1[All Loop Performance]
        I2[Consciousness State]
    end

    subgraph PROCESSING
        P1[Analyze Learning Velocity]
        P2[Detect Convergence/Divergence]
        P3[Adjust Meta-Parameters]
    end

    subgraph UPDATE
        U1[Store Meta-State]
        U2[Update Learning Rates]
        U3[Emit META_INSIGHT]
    end

    subgraph STORAGE
        S1[(meta_cognition table)]
        S2[state_id, learning_rate, strategy]
    end

    I1 --> P1
    I2 --> P1
    P1 --> P2
    P2 --> P3
    P3 --> U1
    U1 --> U2
    U2 --> U3
    U3 --> S1
    S1 -.meta.-> P2

    style INPUT fill:#e1f5ff
    style PROCESSING fill:#fff4e1
    style UPDATE fill:#ffe1e1
    style STORAGE fill:#f0e1ff
```

**Frequency**: Every consciousness cycle (~1/hour)
**Persistence**: `meta_cognition` table
**Output**: Learning rate control signals
**Path**: `packages/node/src/learning/meta-cognition.js`

---

## Integration: All Loops Working Together

```mermaid
graph TB
    subgraph PARALLEL["11 LOOPS (Parallel Execution)"]
        L1[Thompson<br/>100/hr]
        L2[Dog Votes<br/>100/hr]
        L3[Q-Learning<br/>50/hr]
        L4[Brier<br/>10/hr]
        L5[Residual<br/>34min]
        L6[Emergence<br/>89min]
        L7[EWC<br/>1/day]
        L8[DPO<br/>5/hr]
        L9[SONA<br/>20/hr]
        L10[Behavior<br/>1/hr]
        L11[Meta<br/>1/hr]
    end

    subgraph COORDINATION["🎯 UNIFIED BRIDGE"]
        UB[UnifiedBridge]
        MC[Meta-Cognition]
    end

    subgraph PERSISTENCE["💾 POSTGRES"]
        DB[(cynic_db)]
    end

    L1 & L2 & L3 & L4 & L5 & L6 & L7 & L8 & L9 & L10 & L11 --> UB
    UB --> MC
    MC -.controls.-> L1 & L2 & L3 & L4 & L5 & L6 & L7 & L8 & L9 & L10
    UB <--> DB

    style PARALLEL fill:#fff4e1
    style COORDINATION fill:#e1ffe1
    style PERSISTENCE fill:#f0e1ff
```

**Key Integration Points**:
- **UnifiedBridge** (`unified-bridge.js`): Coordinates all 11 loops
- **Meta-Cognition** controls learning rates for all loops
- **Shared PostgreSQL** ensures atomic consistency
- **Event-driven**: All loops listen to relevant events
- **φ-bounded**: All confidence outputs stay ≤61.8%

---

## Loop Interaction Matrix

| Loop | Feeds Into | Fed By | Shared Table |
|------|-----------|--------|--------------|
| 1. Thompson | Dog selection | Judgment quality | `thompson_sampling` |
| 2. Dog Votes | Consensus | Individual judgments | `dog_votes` |
| 3. Q-Learning | Action policy | User feedback | `q_learning_state` |
| 4. Brier | Confidence calibration | Actual outcomes | `brier_scores` |
| 5. Residual | New dimensions | Judgment variance | `residual_signals` |
| 6. Emergence | Collective insights | Cross-domain patterns | `emergence_patterns` |
| 7. EWC | Knowledge protection | Task transitions | `ewc_weights` |
| 8. DPO | Preference alignment | Human choices | `dpo_pairs` |
| 9. SONA | Strategy selection | Action outcomes | `sona_state` |
| 10. Behavior | Influence control | Performance metrics | `behavior_rules` |
| 11. Meta | Learning rates | All loop performance | `meta_cognition` |

---

## Frequency Timeline (24-Hour View)

```mermaid
gantt
    title 11 Learning Loops - 24 Hour Timeline
    dateFormat HH:mm
    axisFormat %H:%M

    section High Frequency
    Thompson (100/hr)    :00:00, 24h
    Dog Votes (100/hr)   :00:00, 24h
    Q-Learning (50/hr)   :00:00, 24h

    section Medium Frequency
    DPO (5/hr)          :00:00, 24h
    SONA (20/hr)        :00:00, 24h
    Brier (10/hr)       :00:00, 24h

    section Low Frequency
    Behavior (1/hr)     :00:00, 24h
    Meta-Cog (1/hr)     :00:00, 24h
    Residual (34min)    :00:00, 00:34
    Residual (34min)    :00:34, 01:08

    section Very Low Frequency
    Emergence (89min)   :00:00, 01:29
    Emergence (89min)   :01:29, 02:58
    EWC (1/day)         :12:00, 12:01
```

---

## Performance Characteristics

| Loop | Latency | Throughput | Storage/day | Criticality |
|------|---------|------------|-------------|-------------|
| Thompson | <5ms | 2400 ops | 50KB | High |
| Dog Votes | <20ms | 2400 ops | 200KB | High |
| Q-Learning | <10ms | 1200 ops | 100KB | Critical |
| Brier | <5ms | 240 ops | 20KB | Medium |
| Residual | 100ms | 42 ops | 10KB | Low |
| Emergence | 200ms | 16 ops | 15KB | Low |
| EWC | 500ms | 1 op | 5KB | Critical |
| DPO | <15ms | 120 ops | 30KB | Medium |
| SONA | <10ms | 480 ops | 40KB | High |
| Behavior | 50ms | 24 ops | 5KB | Medium |
| Meta-Cog | 100ms | 24 ops | 8KB | High |
| **TOTAL** | - | **~6946 ops/day** | **~483KB/day** | - |

---

## φ-Bounded Learning

All 11 loops respect φ constraints:

```javascript
// Loop output bounds (enforced in each loop)
const PHI_INV = 0.618;
const PHI_INV_SQ = 0.382;

// Thompson Sampling: β/(α+β) ≤ φ⁻¹
if (beta / (alpha + beta) > PHI_INV) {
  // Rebalance parameters
}

// Q-Learning: confidence ≤ φ⁻¹
qValue = Math.min(qValue, PHI_INV);

// Brier: predicted confidence ≤ φ⁻¹
prediction = Math.min(prediction, PHI_INV);

// Residual: trigger threshold = φ⁻²
if (residualVariance > PHI_INV_SQ) {
  // Propose new dimension
}

// All loops: respect φ homeostasis
```

---

## Health Monitoring

**What to watch** (see `docs/architecture/completion-criteria.md`):

```
LEARNING MATURITY:
  ├─ Thompson: α/β convergence (target >100 samples/dog)
  ├─ Q-Learning: TD error <0.05 (convergence)
  ├─ Brier: ECE <0.08 (well-calibrated)
  ├─ Residual: variance <φ⁻² (18.2%)
  ├─ Emergence: 12+ Fisher-locked patterns
  └─ Meta: learning velocity >0 (improving)

LOOP HEALTH:
  ├─ Execution rate: actual vs expected
  ├─ Error rate: <1% failures
  ├─ Latency: p95 within budget
  └─ Storage: growth within forecast
```

---

## Key Files

| Component | Path |
|-----------|------|
| Thompson Sampling | `packages/node/src/learning/thompson-sampler.js` |
| Dog Pipeline | `packages/node/src/routing/dog-pipeline.js` |
| Q-Learning Router | `packages/node/src/orchestration/q-learning-router.js` |
| Brier Scoring | `packages/node/src/judge/brier.js` |
| Residual Detector | `packages/node/src/judge/residual.js` |
| Emergence Detector | `packages/node/src/services/emergence-detector.js` |
| EWC Manager | `packages/node/src/orchestration/ewc-manager.js` |
| DPO Learner | `packages/node/src/learning/dpo-learner.js` (planned) |
| SONA Learning | `packages/node/src/learning/sona.js` |
| Behavior Modifier | `packages/node/src/learning/behavior-modifier.js` |
| Meta-Cognition | `packages/node/src/learning/meta-cognition.js` |
| Unified Bridge | `packages/node/src/learning/unified-bridge.js` |

---

## References

- **Scale 5 Definition**: `docs/philosophy/fractal-matrix.md` (LEARN dimension)
- **Learning Service**: `packages/node/src/orchestration/learning-service.js`
- **Database Schema**: `packages/persistence/src/postgres/migrations/`
- **Event Types**: `packages/node/src/agents/events.js` (39 agent event types)
- **φ Constants**: `packages/core/src/axioms/constants.js`

---

*sniff* All 11 loops documented — parallel learning, φ-bounded, event-driven.

**Confidence: 58%** (φ⁻¹ limit — comprehensive but untested in production)
