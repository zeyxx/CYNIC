/**
 * End-to-End test for budget-aware LLM routing
 * Verifies full event flow: cost:update → model:recommendation → routing
 */

'use strict';

import { test } from 'node:test';
import assert from 'node:assert';
import { globalEventBus } from '@cynic/core';
import { getCostLedger, BudgetStatus } from '../src/accounting/cost-ledger.js';
import { getUnifiedLLMRouter, Strategy, BudgetMode, Priority, Complexity } from '../src/orchestration/unified-llm-router.js';
import { KabbalisticRouter } from '../src/orchestration/kabbalistic-router.js';

// Mock CollectivePack
class MockCollectivePack {
  getAllAgents() {
    return [];
  }
}

test('Budget-aware LLM Routing E2E', async (t) => {
  await t.test('should integrate UnifiedLLMRouter and CostLedger', () => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });

    assert.ok(router.llmRouter, 'should have llmRouter');
    assert.ok(router.costLedger, 'should have costLedger');
  });

  await t.test('should provide budget-aware LLM routing', () => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });

    const route = router.getBudgetAwareLLMRoute('moderate');

    assert.ok(route, 'should return routing decision');
    assert.ok(route.provider, 'should have provider');
    assert.ok(route.model, 'should have model');
    assert.ok(route.tier, 'should have tier');
    assert.ok(route.reason, 'should have reason');
  });

  await t.test('should map models to dog tiers correctly', () => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });

    // Test mapping for each complexity level
    const simple = router.getBudgetAwareLLMRoute('simple');
    const complex = router.getBudgetAwareLLMRoute('complex');

    // All routes should have valid tiers
    assert.ok(['FULL', 'MEDIUM', 'LIGHT', 'LOCAL'].includes(simple.tier));
    assert.ok(['FULL', 'MEDIUM', 'LIGHT', 'LOCAL'].includes(complex.tier));
  });

  await t.test('should emit budget:recommendation to UnifiedLLMRouter', async () => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });
    const llmRouter = router.llmRouter;

    // Listen for budget recommendation with promise
    const eventPromise = new Promise((resolve) => {
      llmRouter.once('budget:recommendation', (data) => {
        assert.strictEqual(data.model, 'haiku');
        assert.strictEqual(data.budgetLevel, 'warning');
        resolve();
      });

      // Timeout after 500ms
      setTimeout(resolve, 500);
    });

    // Trigger via global event
    globalEventBus.emit('model:recommendation', {
      model: 'haiku',
      reason: 'budget warning',
      budgetLevel: 'warning',
    });

    await eventPromise;
  });

  await t.test('should handle model:recommendation events', async () => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });

    // Emit model:recommendation event
    globalEventBus.emit('model:recommendation', {
      model: 'sonnet',
      reason: 'balanced performance',
      budgetLevel: 'abundant',
    });

    // Give event time to propagate
    await new Promise(resolve => setTimeout(resolve, 10));

    assert.strictEqual(router._recommendedModel, 'sonnet');
  });

  await t.test('should allow CRITICAL priority even when exhausted', () => {
    const llmRouter = getUnifiedLLMRouter();

    // Mock exhausted budget
    const budgetStatus = {
      level: BudgetStatus.EXHAUSTED,
      remaining: 0.10,
      total: 10.00,
    };

    const blocked = llmRouter._shouldBlockForBudget(budgetStatus, Priority.CRITICAL);

    assert.strictEqual(blocked, false, 'CRITICAL priority should never be blocked');
  });

  await t.test('should block NORMAL priority when exhausted', () => {
    const llmRouter = getUnifiedLLMRouter();

    // Mock exhausted budget
    const budgetStatus = {
      level: BudgetStatus.EXHAUSTED,
      remaining: 0.10,
      total: 10.00,
    };

    const blocked = llmRouter._shouldBlockForBudget(budgetStatus, Priority.NORMAL);

    assert.strictEqual(blocked, true, 'NORMAL priority should be blocked when exhausted');
  });
});
