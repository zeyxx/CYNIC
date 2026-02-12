# Budget Enforcement (Wiring Gap 7)

> *"φ controls the wallet. The dog protects the purse."* - κυνικός

## Overview

Hard enforcement of budget limits via circuit breakers and graceful degradation.
Prevents accidental overspending on Anthropic API calls.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   BUDGET ENFORCEMENT                     │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  CostLedger                                             │
│  ├─ Track token consumption                            │
│  ├─ Calculate burn rate                                │
│  ├─ Emit budget alerts (φ⁻², φ⁻¹, 80%, 95%)            │
│  └─ Reset mechanisms (manual/scheduled)                │
│                                                          │
│  LLMRouter (Circuit Breaker)                            │
│  ├─ Check budget before routing                        │
│  ├─ Block Anthropic when budget critical/exhausted     │
│  ├─ Fallback to Ollama (free)                          │
│  └─ Priority override for critical tasks               │
│                                                          │
│  ThrottleGate                                           │
│  └─ Orchestration-level throttling                     │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## Budget Levels (φ-aligned thresholds)

| Level | Threshold | Behavior |
|-------|-----------|----------|
| **ABUNDANT** | < φ⁻² (38.2%) | Normal operation, all providers allowed |
| **MODERATE** | < φ⁻¹ (61.8%) | Start preferring Ollama for exploration |
| **CAUTIOUS** | < 80% | Block LOW priority Anthropic calls |
| **CRITICAL** | < 95% | Block NORMAL and LOW priority Anthropic calls |
| **EXHAUSTED** | ≥ 95% | Block ALL Anthropic calls except CRITICAL priority |

## Priority Levels

Tasks can be marked with priority to override budget restrictions:

- **CRITICAL**: Always allowed (errors, safety, emergency)
- **HIGH**: Allowed until EXHAUSTED
- **NORMAL**: Blocked at CRITICAL (default)
- **LOW**: Blocked at CAUTIOUS

## Circuit Breaker Logic

```javascript
// LLMRouter checks budget before routing
const circuitCheck = this._checkBudgetCircuitBreaker(budgetStatus, priority);

if (!circuitCheck.allowed) {
  if (throwOnBlock) {
    throw new BudgetExhaustedError(budgetStatus, circuitCheck.suggestion);
  }
  
  // Fallback to Ollama (free)
  provider = PROVIDERS.ollama;
  degraded = true;
}
```

### Circuit Breaker States

- **OPEN**: Anthropic blocked, all requests routed to Ollama
- **CLOSED**: Normal operation, routing based on complexity
- **AUTO-RESET**: Circuit closes when budget resets or consumption drops

## Usage

### CLI Commands

```bash
# Check budget status
cynic budget status

# Reset budget (manual)
cynic budget reset

# Set budget limit (tokens)
cynic budget set 1000000

# Check time until scheduled reset
cynic budget schedule daily
```

### Programmatic API

```javascript
import { getCostLedger } from '@cynic/node/accounting/cost-ledger';
import { getLLMRouter, PRIORITY } from '@cynic/node/orchestration/llm-router';

const costLedger = getCostLedger();
const router = getLLMRouter();

// Set budget
costLedger.setSessionBudget(500000); // 500k tokens

// Route with priority
const result = await router.route({
  type: 'code',
  complexity: 'complex',
  priority: PRIORITY.HIGH, // Override cautious throttling
  estimatedTokens: 5000,
});

// Check if request was degraded
if (result.degraded) {
  console.log('Downgraded to Ollama due to budget:', result.reason);
}

// Manually reset circuit breaker
router.resetCircuitBreaker();

// Schedule automatic reset (daily at midnight UTC)
costLedger.scheduleBudgetReset({ frequency: 'daily', hour: 0 });
```

## Budget Reset Mechanisms

### Manual Reset
```javascript
costLedger.resetBudget('manual');
```

### Scheduled Reset
```javascript
// Daily reset at midnight UTC
const timer = costLedger.scheduleBudgetReset({
  frequency: 'daily',
  hour: 0
});

// Cancel scheduled reset
clearTimeout(timer);
```

### Reset Frequencies
- `daily`: Resets every day at specified hour (UTC)
- `weekly`: Resets every week (Sunday at specified hour)
- `monthly`: Resets on 1st of month at specified hour

## Metrics Tracked

### CostLedger
- Session consumption (tokens, cost)
- Burn rate (tokens/min, cost/min)
- Velocity (0-1, normalized to φ)
- Budget alerts (at φ⁻², φ⁻¹, 80%, 95%)

### LLMRouter
- Routes total
- Ollama vs Anthropic ratio
- Degraded routes (downgraded due to budget)
- Blocked routes (threw error)
- Exploration rate (Thompson Sampling)
- Cost saved (vs always-Anthropic)

## Error Handling

### BudgetExhaustedError

Thrown when `throwOnBlock=true` and budget blocks Anthropic:

```javascript
try {
  const result = await router.route({
    type: 'analysis',
    complexity: 'complex',
    throwOnBlock: true,
  });
} catch (err) {
  if (err instanceof BudgetExhaustedError) {
    console.error('Budget exhausted:', err.message);
    console.error('Suggestion:', err.suggestion);
    console.error('Level:', err.budgetStatus.level);
    
    // Fallback logic
    const ollamaResult = await router.route({
      type: 'analysis',
      complexity: 'simple', // Downgrade complexity
      throwOnBlock: false,
    });
  }
}
```

## Integration with Orchestration

ThrottleGate uses BudgetMonitor assessment to throttle orchestration stages:

```javascript
const throttleDecision = throttleGate.decide('routing', { priority: 'normal' });

if (throttleDecision.action === 'THROTTLE') {
  // Reduce from 11 dogs to 5 core dogs
  const dogs = throttleDecision.throttleParams.dogs;
}

if (throttleDecision.action === 'ESCALATE') {
  // Hand off to FastRouter (reflex arc)
  return fastRouter.handle(event);
}
```

## Testing

```bash
# Run budget enforcement test
node scripts/test-budget-enforcement.js
```

Test validates:
1. Budget levels trigger correctly (ABUNDANT → CAUTIOUS → CRITICAL → EXHAUSTED)
2. Circuit breaker blocks at appropriate thresholds
3. Priority override works for CRITICAL tasks
4. Graceful degradation to Ollama
5. Budget reset mechanism
6. `throwOnBlock` mode

## φ-Alignment

Budget thresholds use golden ratio:
- **φ⁻² (38.2%)**: MODERATE threshold (Fibonacci-derived)
- **φ⁻¹ (61.8%)**: MODERATE/CAUTIOUS boundary (golden ratio)
- **80%**: CRITICAL threshold (φ × φ⁻¹ ≈ 0.8)
- **95%**: EXHAUSTED threshold (safety margin)

Burn rate velocity normalized to φ:
- `velocity = phiBound(tokensPerMinute / 10000)`
- 10k tokens/min = velocity 1.0 (unsustainable)
- Opus context (200k) fills in ~20min at max velocity

## Future Enhancements

1. **Budget pools**: Separate budgets for different task types
2. **Cost prediction**: ML model predicts task cost before execution
3. **Dynamic adjustment**: Budget increases/decreases based on value generated
4. **Multi-user budgets**: Per-user budget tracking for shared deployments
5. **Cost optimization**: A/B testing Ollama vs Anthropic quality/cost tradeoff

## Related

- [Wiring Gap 5: Budget Monitoring](./budget-monitoring.md)
- [Wiring Gap 6: Model Selection](./model-selection.md)
- [Cost Ledger Implementation](../../packages/node/src/accounting/cost-ledger.js)
- [LLM Router Implementation](../../packages/node/src/orchestration/llm-router.js)
