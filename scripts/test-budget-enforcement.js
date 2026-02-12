/**
 * Test Budget Enforcement (Wiring Gap 7)
 *
 * Validates:
 * 1. Budget circuit breaker blocks Anthropic when exhausted
 * 2. Graceful degradation at different budget levels
 * 3. Priority override works for critical tasks
 * 4. Budget reset mechanism
 */

import { CostLedger, BudgetStatus } from '../packages/node/src/accounting/cost-ledger.js';
import { LLMRouter, PRIORITY, COMPLEXITY, BudgetExhaustedError } from '../packages/node/src/orchestration/llm-router.js';
import { getModelIntelligence } from '../packages/node/src/learning/model-intelligence.js';

console.log('🧪 Testing Budget Enforcement Circuit Breaker\n');

// Create cost ledger with small budget for testing
const costLedger = new CostLedger({
  sessionBudget: 100000, // 100k tokens
});

// Create router
const router = new LLMRouter({
  costLedger,
  modelIntelligence: getModelIntelligence(),
});

console.log('Initial budget status:', costLedger.getBudgetStatus());

// Test 1: Normal operation (budget abundant)
console.log('\n--- Test 1: ABUNDANT budget ---');
let result = await router.route({
  type: 'code',
  complexity: COMPLEXITY.MODERATE,
  priority: PRIORITY.NORMAL,
  estimatedTokens: 1000,
});
console.log('Route decision:', {
  provider: result.provider,
  reason: result.reason,
  degraded: result.degraded,
  budgetLevel: result.budgetLevel,
});

// Simulate token consumption to reach WARNING level
console.log('\n--- Simulating token consumption ---');
for (let i = 0; i < 20; i++) {
  costLedger.record({
    type: 'test',
    model: 'sonnet',
    inputTokens: 2000,
    outputTokens: 500,
  });
}

const budgetAfterBurn = costLedger.getBudgetStatus();
console.log('Budget after burn:', {
  consumed: budgetAfterBurn.consumed,
  consumedRatio: budgetAfterBurn.consumedRatio,
  level: budgetAfterBurn.level,
});

// Test 2: CAUTIOUS budget (should start preferring Ollama)
console.log('\n--- Test 2: CAUTIOUS budget ---');
result = await router.route({
  type: 'chat',
  complexity: COMPLEXITY.MODERATE,
  priority: PRIORITY.LOW, // Low priority should be blocked
  estimatedTokens: 1000,
});
console.log('Route decision (LOW priority):', {
  provider: result.provider,
  reason: result.reason,
  degraded: result.degraded,
});

// Test 3: Burn more to reach CRITICAL
console.log('\n--- Burning to CRITICAL level ---');
for (let i = 0; i < 10; i++) {
  costLedger.record({
    type: 'test',
    model: 'sonnet',
    inputTokens: 2000,
    outputTokens: 500,
  });
}

const budgetCritical = costLedger.getBudgetStatus();
console.log('Budget at CRITICAL:', {
  consumed: budgetCritical.consumed,
  consumedRatio: budgetCritical.consumedRatio,
  level: budgetCritical.level,
});

// Test 4: CRITICAL budget blocks NORMAL priority
console.log('\n--- Test 4: CRITICAL budget ---');
result = await router.route({
  type: 'analysis',
  complexity: COMPLEXITY.MODERATE,
  priority: PRIORITY.NORMAL,
  estimatedTokens: 1000,
});
console.log('Route decision (NORMAL priority, should be blocked):', {
  provider: result.provider,
  reason: result.reason,
  degraded: result.degraded,
  circuitBreaker: result.circuitBreakerActive,
});

// Test 5: HIGH priority still allowed at CRITICAL
result = await router.route({
  type: 'analysis',
  complexity: COMPLEXITY.COMPLEX,
  priority: PRIORITY.HIGH,
  estimatedTokens: 1000,
});
console.log('Route decision (HIGH priority, should be allowed):', {
  provider: result.provider,
  reason: result.reason,
  degraded: result.degraded,
});

// Test 6: Burn to EXHAUSTED
console.log('\n--- Burning to EXHAUSTED level ---');
for (let i = 0; i < 5; i++) {
  costLedger.record({
    type: 'test',
    model: 'sonnet',
    inputTokens: 3000,
    outputTokens: 1000,
  });
}

const budgetExhausted = costLedger.getBudgetStatus();
console.log('Budget at EXHAUSTED:', {
  consumed: budgetExhausted.consumed,
  consumedRatio: budgetExhausted.consumedRatio,
  level: budgetExhausted.level,
});

// Test 7: EXHAUSTED blocks everything except CRITICAL
console.log('\n--- Test 7: EXHAUSTED budget ---');
result = await router.route({
  type: 'code',
  complexity: COMPLEXITY.COMPLEX,
  priority: PRIORITY.HIGH,
  estimatedTokens: 1000,
});
console.log('Route decision (HIGH priority, EXHAUSTED → blocked):', {
  provider: result.provider,
  reason: result.reason,
  degraded: result.degraded,
  circuitBreaker: result.circuitBreakerActive,
});

// Test 8: CRITICAL priority override
result = await router.route({
  type: 'error_handling',
  complexity: COMPLEXITY.SIMPLE,
  priority: PRIORITY.CRITICAL, // Should override
  estimatedTokens: 1000,
});
console.log('Route decision (CRITICAL priority, override):', {
  provider: result.provider,
  reason: result.reason,
  degraded: result.degraded,
});

// Test 9: throwOnBlock mode
console.log('\n--- Test 8: throwOnBlock mode ---');
try {
  result = await router.route({
    type: 'code',
    complexity: COMPLEXITY.COMPLEX,
    priority: PRIORITY.NORMAL,
    estimatedTokens: 1000,
    throwOnBlock: true,
  });
  console.log('ERROR: Should have thrown BudgetExhaustedError!');
} catch (err) {
  if (err instanceof BudgetExhaustedError) {
    console.log('✓ BudgetExhaustedError thrown as expected:', {
      message: err.message,
      level: err.budgetStatus.level,
      suggestion: err.suggestion,
    });
  } else {
    console.log('ERROR: Wrong error type:', err);
  }
}

// Test 10: Budget reset
console.log('\n--- Test 9: Budget reset ---');
costLedger.resetBudget('manual');
const budgetAfterReset = costLedger.getBudgetStatus();
console.log('Budget after reset:', {
  consumed: budgetAfterReset.consumed,
  consumedRatio: budgetAfterReset.consumedRatio,
  level: budgetAfterReset.level,
});

// Circuit breaker should reset
result = await router.route({
  type: 'code',
  complexity: COMPLEXITY.COMPLEX,
  priority: PRIORITY.NORMAL,
  estimatedTokens: 1000,
});
console.log('Route decision (after reset, should allow Anthropic):', {
  provider: result.provider,
  reason: result.reason,
  degraded: result.degraded,
  circuitBreaker: result.circuitBreakerActive,
});

// Final stats
console.log('\n--- Final Router Stats ---');
const stats = router.getStats();
console.log(stats);

console.log('\n✓ Budget enforcement circuit breaker test complete');
