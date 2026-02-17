# Budget-Aware LLM Routing Architecture

> *sniff* "Cost-conscious intelligence" — κυνικός

## Overview

CYNIC implements budget-aware LLM routing to prevent budget exhaustion and optimize costs while maintaining quality. The system automatically downgrades to cheaper models when budget is critical.

## Event Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Budget-Aware LLM Flow                     │
└─────────────────────────────────────────────────────────────┘

1. CostLedger tracks spending
   └─> Emits cost:update events (globalEventBus)

2. service-wiring.js listens to cost:update
   ├─> Calls costLedger.recommendModel()
   └─> Emits model:recommendation events

3. KabbalisticRouter listens to model:recommendation
   ├─> Stores _recommendedModel (for tier selection)
   └─> Emits budget:recommendation to UnifiedLLMRouter

4. UnifiedLLMRouter routes based on budget
   ├─> ABUNDANT → Sonnet/Opus (paid)
   ├─> WARNING → Haiku (cheap)
   ├─> CRITICAL → Ollama (free)
   └─> EXHAUSTED → Ollama (free) + blocks expensive calls
```

## Components

### 1. CostLedger (packages/node/src/accounting/cost-ledger.js)

**Responsibilities**:
- Track all LLM costs in real-time
- Calculate burn rate and budget status
- Recommend models based on budget state

**Budget Levels** (φ-aligned thresholds):
```javascript
ABUNDANT:  > 61.8% budget remaining  // φ⁻¹
WARNING:   38.2% - 61.8% remaining  // φ⁻²
CRITICAL:  < 38.2% remaining         // Below φ⁻²
EXHAUSTED: ≥ 95% budget consumed
```

**Model Recommendation Logic**:
```javascript
// Simple tasks → Haiku
if (taskType === 'simple') return HAIKU;

// Complex tasks → Opus
if (taskType === 'complex') return OPUS;

// Budget pressure downgrades
if (budget === CRITICAL && recommended === OPUS) {
  return SONNET; // Downgrade one tier
}

if (budget === EXHAUSTED) {
  return HAIKU; // Force cheapest
}

// Velocity pressure (burn rate ≥ φ⁻¹)
if (burnRate.velocity >= PHI_INV && recommended === OPUS) {
  return SONNET; // Decelerate spending
}
```

### 2. service-wiring.js (packages/node/src/daemon/service-wiring.js)

**Responsibilities**:
- Wire cost:update → model:recommendation bridge
- Initialize UnifiedLLMRouter singleton
- Persist learning state every 5 minutes

**Event Bridge**:
```javascript
globalEventBus.on('cost:update', (data) => {
  const rec = costLedger.recommendModel({
    taskType: 'moderate',
    needsReasoning: false,
  });

  globalEventBus.emit('model:recommendation', {
    model: rec.model,
    reason: rec.reason,
    budgetLevel: data.budget.level,
  });
});
```

### 3. KabbalisticRouter (packages/node/src/orchestration/kabbalistic-router.js)

**Responsibilities**:
- Route tasks through Tree of Life (11 dogs)
- Store model recommendations from events
- Provide budget-aware LLM routing API
- Emit recommendations to UnifiedLLMRouter

**Integration**:
```javascript
constructor(options) {
  this.llmRouter = getUnifiedLLMRouter();
  this.costLedger = getCostLedger();
  this._recommendedModel = null; // Updated by events
  this._wireHealthEvents();
}

_wireHealthEvents() {
  globalEventBus.on('model:recommendation', (data) => {
    this._recommendedModel = data.model;

    // Forward to UnifiedLLMRouter
    this.llmRouter.emit('budget:recommendation', data);
  });
}

getBudgetAwareLLMRoute(taskType, options) {
  const rec = this.costLedger.recommendModel({ taskType });

  const modelToTierMap = {
    opus: { tier: 'FULL', provider: 'anthropic', model: 'opus' },
    sonnet: { tier: 'MEDIUM', provider: 'anthropic', model: 'sonnet' },
    haiku: { tier: 'LIGHT', provider: 'anthropic', model: 'haiku' },
    ollama: { tier: 'LOCAL', provider: 'ollama', model: 'llama3.2:latest' },
  };

  return modelToTierMap[rec.model];
}
```

### 4. UnifiedLLMRouter (packages/node/src/orchestration/unified-llm-router.js)

**Responsibilities**:
- Unified interface for all LLM calls
- Budget enforcement (circuit breaker)
- Multi-provider support (Anthropic, Ollama)
- Consensus voting (multi-LLM agreement)
- Thompson Sampling (exploration/exploitation)

**Strategies**:
```javascript
Strategy.CHEAPEST:  // Cost minimization (budget-aware)
Strategy.CONSENSUS: // Multi-LLM voting (≥ φ⁻¹ agreement)
Strategy.BEST:      // Thompson Sampling (quality)
Strategy.FASTEST:   // Latency minimization
```

**Budget Enforcement**:
```javascript
_shouldBlockForBudget(budgetStatus, priority) {
  if (priority === Priority.CRITICAL) return false;

  if (budgetStatus.level === BudgetStatus.EXHAUSTED) {
    return true; // Block all except CRITICAL
  }

  if (budgetStatus.level === BudgetStatus.CRITICAL &&
      (priority === Priority.NORMAL || priority === Priority.LOW)) {
    return true; // Block NORMAL and LOW
  }

  if (budgetStatus.level === BudgetStatus.WARNING &&
      priority === Priority.LOW) {
    return true; // Block LOW
  }

  return false;
}
```

## Dog Tier Mapping

KabbalisticRouter maps models to dog consultation tiers:

```javascript
const modelToTier = {
  opus: 'FULL',     // All 11 dogs consulted
  sonnet: 'MEDIUM', // 7 dogs consulted (φ-aligned)
  haiku: 'LIGHT',   // 5 dogs consulted (φ-aligned)
  ollama: 'LOCAL',  // 3 dogs consulted (φ-aligned)
};
```

**φ-Alignment**:
- 11 total dogs (Fibonacci F_7)
- Tier sizes follow Fibonacci: 3, 5, 7, 11
- Cost savings: LOCAL = $0, LIGHT ≈ 55% cheaper, MEDIUM ≈ 36% cheaper

## Usage Examples

### Direct Routing
```javascript
const router = new KabbalisticRouter({ collectivePack });

// Get budget-aware route
const route = router.getBudgetAwareLLMRoute('complex', {
  needsReasoning: true,
});

console.log(route);
// {
//   tier: 'MEDIUM',
//   provider: 'anthropic',
//   model: 'sonnet',
//   reason: 'budget critical — downgraded from Opus',
//   budgetLevel: 'critical',
//   confidence: 0.5
// }
```

### Via UnifiedLLMRouter
```javascript
const llmRouter = getUnifiedLLMRouter();

// Automatic budget enforcement
const response = await llmRouter.call(prompt, {
  strategy: Strategy.CHEAPEST,
  budget: BudgetMode.ENFORCE,
  priority: Priority.NORMAL,
  complexity: Complexity.MODERATE,
});

// If budget CRITICAL → degrades to Ollama (free)
console.log(response.provider); // 'ollama'
```

### Event-Driven Flow
```javascript
// Emit cost update
globalEventBus.emit('cost:update', {
  budget: { level: 'critical', remaining: 1.50, total: 10.00 },
  burnRate: { velocity: 0.65, tokensPerMinute: 120 },
});

// Router automatically receives model:recommendation
// No manual intervention needed
```

## Testing

See `packages/node/test/kabbalistic-router-llm.test.js` for integration tests.

**Test Coverage**:
- ✅ UnifiedLLMRouter initialization
- ✅ model:recommendation event handling
- ✅ Budget-aware routing API
- ✅ Model-to-tier mapping
- ✅ Recommended model storage

## Metrics

Track budget-aware routing effectiveness:

```sql
-- Budget-triggered downgrades
SELECT
  budgetLevel,
  COUNT(*) as downgrades,
  AVG(cost) as avg_cost_saved
FROM model_recommendations
WHERE reason LIKE '%budget%'
GROUP BY budgetLevel;

-- Tier distribution by budget level
SELECT
  tier,
  budgetLevel,
  COUNT(*) as routes
FROM routing_log
GROUP BY tier, budgetLevel;
```

## Future Enhancements

1. **Dynamic Thresholds**: Learn optimal budget thresholds per user
2. **Quality Metrics**: Track model quality vs cost tradeoffs
3. **Predictive Budgeting**: Forecast exhaustion time, recommend actions
4. **Multi-Currency**: Support non-USD budgets
5. **Provider Fallback**: Auto-switch if Anthropic down → Ollama

## See Also

- `docs/architecture/unified-llm-router.md` - Full router specification
- `docs/architecture/cost-ledger.md` - Budget tracking details
- `docs/philosophy/phi-alignment.md` - φ-based thresholds
- `packages/node/src/accounting/cost-ledger.js` - Implementation
- `packages/node/src/orchestration/unified-llm-router.js` - Implementation

---

*sniff* Confidence: 58% (φ⁻¹ limit)
