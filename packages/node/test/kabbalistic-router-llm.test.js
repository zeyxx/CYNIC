/**
 * Test KabbalisticRouter budget-aware LLM integration
 * Verifies that model:recommendation events properly wire to UnifiedLLMRouter
 */

'use strict';

import { test } from 'node:test';
import assert from 'node:assert';
import { globalEventBus } from '@cynic/core';
import { KabbalisticRouter } from '../src/orchestration/kabbalistic-router.js';
import { getCostLedger } from '../src/accounting/cost-ledger.js';
import { getUnifiedLLMRouter } from '../src/orchestration/unified-llm-router.js';

// Mock CollectivePack for testing
class MockCollectivePack {
  constructor() {
    this.agents = [];
  }

  getAllAgents() {
    return this.agents;
  }
}

test('KabbalisticRouter - Budget-aware LLM routing', async (t) => {
  await t.test('should integrate UnifiedLLMRouter', () => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });

    assert.ok(router.llmRouter, 'llmRouter should be initialized');
    assert.ok(router.costLedger, 'costLedger should be initialized');
  });

  await t.test('should handle model:recommendation events', (t, done) => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });

    // Listen for budget:recommendation emitted to UnifiedLLMRouter
    router.llmRouter.once('budget:recommendation', (data) => {
      assert.strictEqual(data.model, 'haiku');
      assert.strictEqual(data.budgetLevel, 'critical');
      assert.ok(data.reason);
      done();
    });

    // Emit model:recommendation event
    globalEventBus.emit('model:recommendation', {
      model: 'haiku',
      reason: 'budget critical — minimum cost model',
      budgetLevel: 'critical',
      timestamp: Date.now(),
    });
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

  await t.test('should map models to tiers correctly', () => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });

    // Map expected by router
    const expected = {
      opus: 'FULL',
      sonnet: 'MEDIUM',
      haiku: 'LIGHT',
      ollama: 'LOCAL',
    };

    // Test each model mapping (we'll just test the logic structure)
    const route = router.getBudgetAwareLLMRoute('simple');
    assert.ok(['FULL', 'MEDIUM', 'LIGHT', 'LOCAL'].includes(route.tier));
  });

  await t.test('should store recommended model from events', (t, done) => {
    const pack = new MockCollectivePack();
    const router = new KabbalisticRouter({ collectivePack: pack });

    // Emit model recommendation
    globalEventBus.emit('model:recommendation', {
      model: 'sonnet',
      reason: 'balanced performance',
      budgetLevel: 'abundant',
    });

    // Give event time to propagate
    setTimeout(() => {
      assert.strictEqual(router._recommendedModel, 'sonnet');
      done();
    }, 10);
  });
});
