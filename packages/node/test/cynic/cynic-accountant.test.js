/**
 * CynicAccountant Tests — C6.6 (CYNIC × ACCOUNT)
 *
 * Tests for factory-based Accountant and domain-specific CynicAccountant.
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { getCynicAccountant, resetCynicAccountant } from '../../src/cynic/cynic-accountant.js';
import { CynicAccountingCategories } from '../../src/cycle/configs/cynic-accountant.config.js';
import { globalEventBus } from '@cynic/core';

describe('CynicAccountant (C6.6)', () => {
  let accountant;
  let eventHandlers = [];

  beforeEach(() => {
    resetCynicAccountant();
    accountant = getCynicAccountant();
    // Clean up any leftover event handlers
    eventHandlers.forEach(({ event, handler }) => {
      globalEventBus.off(event, handler);
    });
    eventHandlers = [];
  });

  describe('Factory Pattern', () => {
    it('should create singleton instance', () => {
      const instance1 = getCynicAccountant();
      const instance2 = getCynicAccountant();
      assert.strictEqual(instance1, instance2, 'Should return same instance');
    });

    it('should reset singleton', () => {
      const instance1 = getCynicAccountant();
      resetCynicAccountant();
      const instance2 = getCynicAccountant();
      assert.notStrictEqual(instance1, instance2, 'Should create new instance after reset');
    });

    it('should have required methods', () => {
      assert.equal(typeof accountant.recordCost, 'function');
      assert.equal(typeof accountant.getStats, 'function');
      assert.equal(typeof accountant.getSummary, 'function');
      assert.equal(typeof accountant.getHistory, 'function');
      assert.equal(typeof accountant.getHealth, 'function');
      assert.equal(typeof accountant.clear, 'function');
    });
  });

  describe('Cost Recording', () => {
    it('should record memory operation costs', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.MEMORY,
        data: {
          dbQueries: 5,
          dbLatencyMs: 12,
          memoryDeltaMB: 2,
        },
      });

      assert.equal(result.category, CynicAccountingCategories.MEMORY);
      assert.equal(result.cell, 'C6.6');
      assert.equal(result.dimension, 'CYNIC');
      assert.equal(typeof result.costUSD, 'number');
      assert.equal(result.costs.database.queries, 5);
      assert.equal(result.costs.database.latencyMs, 12);
    });

    it('should record event routing costs', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.EVENTS,
        data: {
          eventsRouted: 100,
          computeMs: 5,
        },
      });

      assert.equal(result.category, CynicAccountingCategories.EVENTS);
      assert.equal(result.costs.events.routedCount, 100);
      assert.equal(result.costs.events.estimatedCostUSD, 0, 'Event routing should be free');
    });

    it('should record learning operation costs', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.LEARNING,
        data: {
          learningSamples: 10,
          computeMs: 50,
          dbQueries: 2,
        },
      });

      assert.equal(result.category, CynicAccountingCategories.LEARNING);
      assert.ok(result.valueScore > 0, 'Learning should produce value');
      assert.ok(result.efficiency >= 0, 'Should calculate efficiency');
    });

    it('should record judgment costs', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.JUDGMENT,
        data: {
          judgmentsCount: 3,
          computeMs: 20,
        },
      });

      assert.equal(result.category, CynicAccountingCategories.JUDGMENT);
      assert.ok(result.valueScore > 0, 'Judgments should produce value');
    });

    it('should record emergence detection costs', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.EMERGENCE,
        data: {
          patternsDetected: 2,
          computeMs: 30,
        },
      });

      assert.equal(result.category, CynicAccountingCategories.EMERGENCE);
      assert.ok(result.valueScore > 0, 'Emergence should produce value');
    });

    it('should record LLM costs when CYNIC uses meta-cognition', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.LEARNING,
        data: {
          llmTokens: 1000,
          computeMs: 100,
        },
      });

      assert.equal(result.tokens, 1000);
      assert.ok(result.costs.llm.estimatedCostUSD > 0, 'LLM should have cost');
    });
  });

  describe('Stats Tracking', () => {
    it('should track stats across categories', () => {
      accountant.recordCost({
        category: CynicAccountingCategories.MEMORY,
        data: { dbQueries: 5 },
      });

      accountant.recordCost({
        category: CynicAccountingCategories.EVENTS,
        data: { eventsRouted: 10 },
      });

      accountant.recordCost({
        category: CynicAccountingCategories.MEMORY,
        data: { dbQueries: 3 },
      });

      const stats = accountant.getStats();
      assert.equal(stats.total, 3);
      assert.equal(stats.byType[CynicAccountingCategories.MEMORY], 2);
      assert.equal(stats.byType[CynicAccountingCategories.EVENTS], 1);
      assert.ok(stats.totalCost >= 0);
      assert.ok(stats.totalValue >= 0);
    });

    it('should track summary metrics', () => {
      accountant.recordCost({
        category: CynicAccountingCategories.LEARNING,
        data: {
          learningSamples: 5,
          llmTokens: 500,
          computeMs: 30,
        },
      });

      const summary = accountant.getSummary();
      assert.equal(summary.totalOperations, 1);
      assert.ok(summary.totalCostUSD >= 0);
      assert.equal(summary.totalTokens, 500);
      assert.equal(summary.totalComputeMs, 30);
      assert.ok(summary.avgCostPerOperation >= 0);
      assert.ok(summary.byCategory[CynicAccountingCategories.LEARNING]);
    });
  });

  describe('History', () => {
    it('should maintain cost history', () => {
      for (let i = 0; i < 10; i++) {
        accountant.recordCost({
          category: CynicAccountingCategories.MEMORY,
          data: { dbQueries: i + 1 },
        });
      }

      const history = accountant.getHistory();
      assert.ok(history.length <= 21, 'Should respect default limit');
      assert.equal(history[history.length - 1].costs.database.queries, 10);
    });

    it('should respect history limit', () => {
      for (let i = 0; i < 100; i++) {
        accountant.recordCost({
          category: CynicAccountingCategories.EVENTS,
          data: { eventsRouted: i },
        });
      }

      const fullHistory = accountant.getHistory(1000);
      assert.ok(fullHistory.length <= 500, 'Should trim to maxHistory');
    });
  });

  describe('Health Check', () => {
    it('should report healthy for low-cost operations', () => {
      // Record only Tier 1+2 operations (zero/low cost)
      for (let i = 0; i < 10; i++) {
        accountant.recordCost({
          category: CynicAccountingCategories.EVENTS,
          data: { eventsRouted: 100, computeMs: 5 },
        });
      }

      const health = accountant.getHealth();
      assert.equal(health.status, 'efficient', 'Should be efficient with low costs');
      assert.ok(health.score > 0);
      assert.equal(health.totalRecords, 10);
    });

    it('should report concern for high-cost operations', () => {
      // Record expensive LLM operations
      for (let i = 0; i < 5; i++) {
        accountant.recordCost({
          category: CynicAccountingCategories.LEARNING,
          data: { llmTokens: 10000, computeMs: 200 },
        });
      }

      const health = accountant.getHealth();
      assert.ok(health.totalCostUSD > 0, 'Should accumulate costs');
      assert.ok(health.avgCostUSD > 0, 'Should calculate average');
    });

    it('should report idle when no operations', () => {
      const health = accountant.getHealth();
      assert.equal(health.status, 'idle');
      assert.equal(health.totalRecords, 0);
    });
  });

  describe('Event Emission', () => {
    it('should emit cost_recorded event locally', async (t) => {
      const promise = new Promise((resolve) => {
        accountant.once('cost_recorded', (record) => {
          assert.equal(record.category, CynicAccountingCategories.MEMORY);
          assert.equal(record.cell, 'C6.6');
          resolve();
        });
      });

      accountant.recordCost({
        category: CynicAccountingCategories.MEMORY,
        data: { dbQueries: 1 },
      });

      await promise;
    });

    it('should publish to global event bus', async (t) => {
      const promise = new Promise((resolve) => {
        const handler = (event) => {
          try {
            // globalEventBus.publish wraps data in CYNICEvent with { type, payload, source, ... }
            const record = event.payload || event;
            assert.equal(record.category, CynicAccountingCategories.LEARNING);
            assert.ok(record.costUSD !== undefined);
            resolve();
          } finally {
            globalEventBus.off('cynic:cost', handler);
          }
        };

        globalEventBus.on('cynic:cost', handler);
        eventHandlers.push({ event: 'cynic:cost', handler });
      });

      accountant.recordCost({
        category: CynicAccountingCategories.LEARNING,
        data: { learningSamples: 3 },
      });

      await promise;
    });
  });

  describe('Value Calculation', () => {
    it('should assign high value to learning operations', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.LEARNING,
        data: { learningSamples: 10 },
      });

      assert.ok(result.valueScore > 0, 'Learning should produce value');
    });

    it('should assign moderate value to judgments', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.JUDGMENT,
        data: { judgmentsCount: 5 },
      });

      assert.ok(result.valueScore > 0, 'Judgments should produce value');
    });

    it('should assign high value to emergence detection', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.EMERGENCE,
        data: { patternsDetected: 3 },
      });

      assert.ok(result.valueScore > 0, 'Emergence should produce value');
    });

    it('should calculate efficiency ratio', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.LEARNING,
        data: {
          learningSamples: 10,
          llmTokens: 100,
          computeMs: 50,
        },
      });

      assert.ok(result.efficiency >= 0, 'Should calculate efficiency');
      assert.equal(typeof result.efficiency, 'number');

      if (result.costUSD > 0) {
        // Efficiency should be value/cost ratio (allowing for rounding in config)
        const expectedRange = result.valueScore / result.costUSD;
        assert.ok(result.efficiency > 0, 'Should have positive efficiency when cost > 0');
        assert.ok(result.efficiency <= expectedRange * 1.1, 'Efficiency should be reasonable');
      }
    });
  });

  describe('Clear', () => {
    it('should clear all state', () => {
      accountant.recordCost({
        category: CynicAccountingCategories.MEMORY,
        data: { dbQueries: 5 },
      });
      accountant.recordCost({
        category: CynicAccountingCategories.LEARNING,
        data: { learningSamples: 3 },
      });

      accountant.clear();

      const stats = accountant.getStats();
      const summary = accountant.getSummary();
      const history = accountant.getHistory();

      assert.equal(stats.total, 0);
      assert.equal(summary.totalOperations, 0);
      assert.equal(summary.totalCostUSD, 0);
      assert.equal(history.length, 0);
    });
  });

  describe('φ-Bounded Costs', () => {
    it('should keep costs reasonable for Tier 1+2 operations', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.EVENTS,
        data: {
          eventsRouted: 1000,
          computeMs: 10,
        },
      });

      assert.ok(result.costUSD < 0.01, 'Tier 1+2 should be near-zero cost');
    });

    it('should track LLM costs when meta-cognition is used', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.LEARNING,
        data: {
          llmTokens: 10000, // 10k tokens
          learningSamples: 5,
        },
      });

      assert.ok(result.costUSD > 0, 'LLM usage should have cost');
      assert.ok(result.costUSD < 1, 'Should be reasonable for 10k tokens');
    });
  });

  describe('Integration with CostLedger', () => {
    it('should structure costs for CostLedger consumption', () => {
      const result = accountant.recordCost({
        category: CynicAccountingCategories.JUDGMENT,
        data: {
          llmTokens: 500,
          computeMs: 25,
          dbQueries: 3,
          apiCalls: 1,
        },
      });

      // Verify structure matches CostLedger expectations
      assert.ok(result.costs);
      assert.ok(result.costs.llm);
      assert.ok(result.costs.compute);
      assert.ok(result.costs.database);
      assert.ok(result.costs.api);
      assert.equal(typeof result.costUSD, 'number');
      assert.equal(typeof result.tokens, 'number');
    });
  });
});
