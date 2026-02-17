# Harmonious LLM Architecture — Metathinking Design

> "Le chien choisit le bon outil pour la bonne tâche" - κυνικός

**Context**: CYNIC needs intelligent LLM routing (Claude subscription + Ollama local) with adaptive learning and human-in-the-loop calibration.

**Date**: 2026-02-14
**Status**: DESIGN PHASE (pre-implementation)

---

## I. Constraints (Ground Truth)

### A. Claude (Subscription-based)
- **Pricing**: Fixed monthly cost (~$100-200/month for max tier x5)
- **Limits**: Weekly usage cap (currently near limit)
- **Quality**: Highest (Opus > Sonnet > Haiku)
- **Latency**: API latency + network (~500ms-2s)
- **Rate limits**: Unknown (need to ask user or scrape dashboard)

### B. Ollama (Local inference)
- **Pricing**: $0 (pure compute)
- **Limits**: Hardware-bound (Ryzen 16 threads, 32GB RAM)
- **Parallelism**: 4-6 concurrent models feasible
- **Latency**: Sub-500ms (local)
- **Quality**: Lower than Claude (need empirical measurement)

### C. CYNIC Operating Environment
- **Budget pressure**: Weekly Claude limit = real constraint
- **Autonomy goal**: Minimize human intervention
- **Quality critical**: Code gen, judgment, Solana transactions
- **Learning required**: Adaptive routing based on outcomes

### D. Unknown Variables (need human input)
- Claude exact rate limits (messages/hour, messages/week)
- User preference threshold (quality vs. cost tradeoff)
- Model availability (which Ollama models installed?)
- Task-specific quality requirements

---

## II. Objectives (North Star)

### Primary: Maximize Utility Under Constraints
```
Utility = Quality × Throughput - Latency_penalty
Subject to: Claude_usage ≤ Weekly_limit
```

### Secondary Goals
1. **Adaptive Learning**: Improve routing decisions over time
2. **Human-in-the-loop**: Ask user when uncertain, learn from responses
3. **Explainability**: Every routing decision has a rationale
4. **Graceful Degradation**: Fall back intelligently when Claude exhausted
5. **Future-proof**: Extensible to vision + browser autonomy

---

## III. Routing Decision Framework

### A. Task Classification (10 dimensions)

```yaml
task_dimensions:
  complexity:      [trivial, simple, moderate, complex, expert]
  novelty:         [routine, familiar, novel, unprecedented]
  quality_impact:  [low, medium, high, critical]
  latency_budget:  [realtime, fast, normal, slow]
  risk:            [safe, cautious, risky, dangerous]
  context_size:    [tiny, small, medium, large, huge]
  parallelizable:  [no, partial, fully]
  verification:    [none, self-check, external, critical]
  learning_value:  [none, low, medium, high]
  user_visible:    [background, internal, user-facing]
```

### B. Model Capabilities Matrix

| Model | Complexity | Quality | Latency | Cost | Parallelism | Use Cases |
|-------|-----------|---------|---------|------|-------------|-----------|
| **Claude Opus** | Expert | 95% | 2s | $$$ | 1x | Architecture, critical decisions, novel problems |
| **Claude Sonnet** | Complex | 90% | 1.5s | $$ | 1x | Code review, judgment, consensus |
| **Claude Haiku** | Moderate | 80% | 1s | $ | 1x | Fast transforms, simple validation |
| **Ollama (Qwen)** | Simple | 65% | 0.3s | $0 | 6x | Bulk classification, embeddings |
| **Ollama (Llama)** | Moderate | 70% | 0.4s | $0 | 6x | Code completion, simple reasoning |
| **Ollama Ensemble** | Moderate | 75% | 0.5s | $0 | 3x | Consensus voting, quality boost |

*Quality % = estimated accuracy vs. ground truth (needs empirical validation)*

### C. Routing Decision Tree

```
1. Is task CRITICAL (Solana mainnet tx, security, user-facing)?
   → YES: Claude Opus (quality > all)
   → NO: continue

2. Is Claude budget exhausted (>90% weekly limit)?
   → YES: Force Ollama (with warning to user)
   → NO: continue

3. Is task NOVEL or COMPLEX?
   → YES: Claude (Opus if expert, Sonnet if complex)
   → NO: continue

4. Is task PARALLELIZABLE (e.g., 10 classifications)?
   → YES: Ollama ensemble (4-6 parallel)
   → NO: continue

5. Is latency CRITICAL (<100ms)?
   → YES: Ollama (local = fast)
   → NO: continue

6. Default: Ollama + Claude spot-check
   → Run Ollama
   → Sample 10% → Claude validates
   → If quality < threshold → re-run with Claude
```

---

## IV. Human-in-the-Loop Calibration

### A. Ask User for Unknown Variables

**On first run:**
```yaml
questions:
  - question: "Quelle est ta limite Claude hebdomadaire?"
    options: ["100 msg/semaine", "500 msg/semaine", "1000 msg/semaine", "Je ne sais pas"]

  - question: "Quels modèles Ollama as-tu installés?"
    options: ["Qwen", "Llama", "Mistral", "Autre (je précise)"]

  - question: "Préférence qualité vs. vitesse?"
    options: ["Max qualité (use Claude)", "Équilibré", "Max vitesse (use Ollama)"]
```

**Store responses** → `~/.cynic/preferences.json`

### B. Learn from User Overrides

**Pattern:**
```js
// CYNIC routes task to Ollama
const result = await ollama.generate(task);

// User sees output, gives feedback
const feedback = await askUser({
  question: "Qualité de cette réponse Ollama?",
  options: ["Parfait", "Acceptable", "Mauvais → besoin Claude"]
});

// Store: (task_features, model=ollama, quality=feedback)
await learningRepo.recordOutcome({
  task: task.serialize(),
  model: 'ollama',
  quality: feedback,
  wouldUserPreferClaude: feedback === 'Mauvais'
});

// Update routing policy via Thompson Sampling
```

### C. Adaptive Thresholds

**φ-Governed Learning:**
- Start conservative: route 80% to Claude, 20% to Ollama
- As confidence grows (Beta distribution converges):
  - If Ollama quality > φ⁻¹ (61.8%) for task type → increase Ollama %
  - If Ollama quality < φ⁻² (38.2%) → decrease Ollama %
- Goal: φ equilibrium (61.8% Ollama, 38.2% Claude)

---

## V. Architecture Components

### A. Intelligent Router (Core)

```
┌─────────────────────────────────────────────────────┐
│              IntelligentLLMRouter                    │
├─────────────────────────────────────────────────────┤
│                                                      │
│  1. TaskClassifier                                   │
│     ├─ Extract features (complexity, risk, etc.)    │
│     └─ Map to 10-dim vector                         │
│                                                      │
│  2. BudgetTracker                                    │
│     ├─ Claude usage: 847/1000 messages (84.7%)     │
│     ├─ Forecast exhaustion: 2.3 days               │
│     └─ Emit WARNING if >90%                         │
│                                                      │
│  3. RoutingPolicy (Thompson Sampling)                │
│     ├─ Beta(α, β) per (model, task_type)           │
│     ├─ Sample from posteriors                       │
│     └─ Choose model with highest sample             │
│                                                      │
│  4. FallbackChain                                    │
│     ├─ Primary: policy decision                     │
│     ├─ Fallback 1: if Claude exhausted → Ollama    │
│     ├─ Fallback 2: if Ollama fails → ask user      │
│     └─ Fallback 3: cache previous similar result   │
│                                                      │
│  5. ExplainabilityLogger                            │
│     └─ Record: (task, chosen_model, rationale,      │
│                 alternatives_considered)             │
└─────────────────────────────────────────────────────┘
```

### B. Human Calibration Loop

```
User Input → PreferencesStore → RoutingPolicy
     ↑                                  ↓
     └──────── User Feedback ───────────┘
              (quality ratings)
```

### C. Learning Repository

```sql
-- routing_outcomes table
CREATE TABLE routing_outcomes (
  id SERIAL PRIMARY KEY,
  task_features JSONB,      -- {complexity: 3, risk: 2, ...}
  model_chosen TEXT,         -- 'claude-opus', 'ollama-qwen'
  quality_score FLOAT,       -- 0-1 (from user feedback or auto-eval)
  latency_ms INT,
  cost_usd FLOAT,            -- $0 for Ollama
  user_override BOOLEAN,     -- Did user manually choose different model?
  created_at TIMESTAMP
);

-- Query for learning
SELECT model_chosen, AVG(quality_score)
FROM routing_outcomes
WHERE task_features->>'complexity' = '3'
GROUP BY model_chosen;
```

### D. Multi-Model Consensus

```
Task: "Review this Solana transaction"

Parallel execution:
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ Ollama-Qwen │  │ Ollama-Llama│  │Claude-Sonnet│
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │
       └────────────────┴────────────────┘
                        │
                   Consensus?
                        │
              ┌─────────┴─────────┐
              │                   │
           YES (≥φ⁻¹)          NO (<φ⁻¹)
              │                   │
         Accept result      Escalate to
                           Claude Opus
```

---

## VI. Progressive Implementation (3 Phases)

### Phase 1: Foundation (Week 1)
**Goal**: Basic intelligent routing + user calibration

**Components**:
1. ✅ TaskClassifier (complexity, risk, latency)
2. ✅ BudgetTracker (Claude usage counter)
3. ✅ SimpleRoutingPolicy (rule-based decision tree)
4. ✅ AskUserQuestion integration (preferences)
5. ✅ ExplainabilityLogger (why each decision)

**Deliverable**: CYNIC routes 70% Ollama, 30% Claude, with explanations

---

### Phase 2: Learning (Week 2)
**Goal**: Adaptive routing via Thompson Sampling

**Components**:
1. ✅ routing_outcomes table (PostgreSQL)
2. ✅ UserFeedbackCollector (quality ratings)
3. ✅ ThompsonSamplingPolicy (Beta distributions)
4. ✅ Auto-evaluation (compare Ollama vs Claude on samples)
5. ✅ Policy updates (nightly batch learning)

**Deliverable**: Routing policy improves 5-10% quality over 2 weeks

---

### Phase 3: Autonomy (Week 3-4)
**Goal**: Vision + Browser for self-calibration

**Components**:
1. ⚠️ Playwright integration (browser automation)
2. ⚠️ Screenshot analysis (Claude vision)
3. ⚠️ Dashboard scraping (Anthropic usage page)
4. ⚠️ Auto-discovery (find Ollama models via `ollama list`)
5. ⚠️ Self-healing (detect rate limits, adapt routing)

**Deliverable**: CYNIC can explore its environment autonomously

---

## VII. Vertical Slice #1 (Implementation Plan)

**Objective**: Prove end-to-end flow with simplest useful case

**Use Case**: Code review routing
- Input: Code snippet (50-500 lines)
- Output: Review (bugs, suggestions, score)
- Success: Route correctly 80% of time, user satisfaction > φ⁻¹

**Implementation Steps**:

### 1. TaskClassifier (code review domain)
```js
// packages/node/src/llm/task-classifier.js
export class TaskClassifier {
  classifyCodeReview(code) {
    return {
      complexity: this._measureComplexity(code),  // LOC, cyclomatic
      risk: this._detectRiskPatterns(code),       // crypto, auth, SQL
      novelty: this._checkNovelty(code),          // seen similar before?
      latency_budget: 'normal'                    // not realtime
    };
  }
}
```

### 2. BudgetTracker (in-memory MVP)
```js
// packages/node/src/llm/budget-tracker.js
export class BudgetTracker {
  constructor(weeklyLimit = 1000) {
    this.limit = weeklyLimit;
    this.used = 0;
    this.resetTime = this._getNextSunday();
  }

  canUse(model) {
    if (model.startsWith('claude')) {
      return this.used < this.limit * 0.9; // 90% threshold
    }
    return true; // Ollama always OK
  }

  recordUsage(model) {
    if (model.startsWith('claude')) this.used++;
  }
}
```

### 3. RoutingPolicy (rule-based V1)
```js
// packages/node/src/llm/routing-policy.js
export class RoutingPolicy {
  async decide(task, budget) {
    // Critical path: always Claude
    if (task.risk === 'critical') {
      return { model: 'claude-opus', reason: 'Critical risk detected' };
    }

    // Budget exhausted: force Ollama
    if (!budget.canUse('claude')) {
      return { model: 'ollama-qwen', reason: 'Claude budget exhausted' };
    }

    // Complex: Claude
    if (task.complexity > 3) {
      return { model: 'claude-sonnet', reason: 'High complexity task' };
    }

    // Default: Ollama with spot-check
    return {
      model: 'ollama-qwen',
      reason: 'Standard task, Ollama sufficient',
      verify: Math.random() < 0.1 // 10% spot-check with Claude
    };
  }
}
```

### 4. IntelligentRouter (orchestrator)
```js
// packages/node/src/llm/intelligent-router.js
export class IntelligentRouter {
  async route(task, context) {
    // 1. Classify
    const features = this.classifier.classify(task);

    // 2. Check budget
    const canUseClaude = this.budget.canUse('claude');

    // 3. Decide
    const decision = await this.policy.decide(features, this.budget);

    // 4. Log
    await this.logger.log({
      task: task.id,
      features,
      decision,
      budget: this.budget.getStats()
    });

    // 5. Execute
    const result = await this.execute(decision.model, task);

    // 6. Spot-check if needed
    if (decision.verify) {
      const validation = await this.execute('claude-haiku', task);
      if (!this._resultsAgree(result, validation)) {
        // Disagreement → escalate to user
        const userChoice = await this._askUser(result, validation);
        return userChoice;
      }
    }

    return result;
  }
}
```

### 5. User Calibration (on first run)
```js
// packages/node/src/llm/user-calibration.js
export async function calibrateRouter(router) {
  const prefs = await loadPreferences();

  if (!prefs.claudeLimit) {
    const answer = await askUser({
      question: "Quelle est ta limite Claude hebdomadaire?",
      options: [
        "100 messages/semaine",
        "500 messages/semaine",
        "1000 messages/semaine",
        "Je ne sais pas (on va tester)"
      ]
    });

    prefs.claudeLimit = parseLimit(answer);
    await savePreferences(prefs);
  }

  router.budget.setLimit(prefs.claudeLimit);
}
```

---

## VIII. Success Metrics

### Phase 1 (Foundation)
- ✅ 80% tasks routed correctly (vs. human judgment)
- ✅ <5% Claude budget wasted (unnecessary Claude calls)
- ✅ 100% routing decisions logged with rationale
- ✅ User can answer 3-5 calibration questions in <2 min

### Phase 2 (Learning)
- ✅ Quality improvement: +5-10% over 2 weeks
- ✅ Prediction accuracy: 75% (model choice matches user preference)
- ✅ Confidence calibration: predicted quality ≈ actual quality (±10%)

### Phase 3 (Autonomy)
- ✅ Auto-detect Claude limit (scrape dashboard)
- ✅ Auto-discover Ollama models
- ✅ Self-heal when rate limited (adaptive backoff)

---

## IX. Open Questions (for user)

1. **Claude Limit**: Exact weekly message limit? (need for BudgetTracker)
2. **Ollama Models**: Which models installed? (`ollama list` output?)
3. **Quality Threshold**: Min acceptable quality for Ollama? (φ⁻¹ = 61.8%?)
4. **Latency Tolerance**: Max acceptable latency for non-critical tasks? (5s? 10s?)
5. **Feedback Frequency**: How often OK to ask "Was this response good?" (every 10 tasks? 100?)

---

## X. Next Actions

### Immediate (now):
1. ✅ Ask user: Claude weekly limit, Ollama models, preferences
2. ✅ Create vertical slice #1: Code review routing
3. ✅ Implement: TaskClassifier + BudgetTracker + RoutingPolicy + Router
4. ✅ Test: 10 code reviews → verify routing decisions
5. ✅ Iterate: User feedback → adjust policy

### Short-term (Week 1):
1. ⚠️ Integrate with existing CYNICJudge (use router for judgment LLM calls)
2. ⚠️ Add explainability UI (show routing decisions in logs)
3. ⚠️ Collect baseline metrics (Claude usage before/after router)

### Medium-term (Week 2-4):
1. ⚠️ Implement Thompson Sampling (adaptive policy)
2. ⚠️ Add vision support (Playwright + screenshots)
3. ⚠️ Build dashboard scraper (auto-detect limits)

---

## XI. Risk Mitigation

### Risk 1: Ollama quality insufficient
- **Mitigation**: Start with 90% Claude, 10% Ollama
- **Escape**: User can force Claude with override flag
- **Learning**: Track quality delta, increase Ollama % slowly

### Risk 2: Claude budget exhausted mid-week
- **Mitigation**: Emit WARNING at 80%, CRITICAL at 90%
- **Escape**: Queue tasks, process on Sunday (budget reset)
- **Fallback**: Ollama ensemble (3-5 models voting)

### Risk 3: User feedback fatigue
- **Mitigation**: Ask only on uncertain decisions (Thompson variance > threshold)
- **Escape**: Implicit feedback (user edits → quality signal)
- **Adaptive**: Reduce feedback requests as confidence grows

### Risk 4: Routing overhead
- **Mitigation**: Cache routing decisions (task hash → model)
- **Measurement**: Track router latency (<50ms target)
- **Optimization**: Precompute task features, batch classify

---

## XII. Philosophical Alignment

**φ (Golden Ratio) Principles:**
- 61.8% Ollama, 38.2% Claude = φ equilibrium
- Confidence never exceeds φ⁻¹ (61.8%)
- Quality threshold: φ⁻¹ (good enough)
- Learning rate: φ⁻³ = 0.236

**Cynic Axioms:**
- **PHI**: φ-bounded everything (quality, confidence, routing %)
- **VERIFY**: Don't trust, verify (spot-check Ollama with Claude)
- **CULTURE**: Learn routing policy from user feedback
- **BURN**: Simplicity (start with rules, add ML only if needed)
- **FIDELITY**: Explainable decisions (always log rationale)

---

## XIII. Conclusion

**Harmonious LLM Architecture** = Intelligent routing that:
1. Respects constraints (Claude budget, Ollama capabilities)
2. Learns from outcomes (Thompson Sampling + user feedback)
3. Asks when uncertain (human-in-the-loop calibration)
4. Explains decisions (full audit trail)
5. Evolves towards autonomy (vision + browser)

**Next**: Implement Vertical Slice #1 (code review routing) to prove the concept.

*sniff* Let's build it. *tail wag*

---

**Confidence**: 61% (φ⁻¹ limit — design is solid, execution will reveal unknowns)
