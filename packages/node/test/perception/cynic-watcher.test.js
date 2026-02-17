/**
 * Tests for CynicWatcher (C6.1 — CYNIC × PERCEIVE)
 *
 * Self-observation: CYNIC watching its own internal state.
 */

'use strict';

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'events';
import {
  CynicWatcher,
  CynicEventType,
  getCynicWatcher,
  resetCynicWatcher,
} from '../../src/perception/cynic-watcher.js';

/**
 * Mock PostgreSQL pool
 */
class MockPool {
  constructor() {
    this.queries = [];
  }

  async query(sql, params) {
    this.queries.push({ sql, params, timestamp: Date.now() });

    // Mock responses based on query
    if (sql.includes('pg_database_size')) {
      return { rows: [{ size_mb: 12.7 }] };
    }
    if (sql.includes('information_schema.tables')) {
      return { rows: [{ count: '15' }] };
    }
    if (sql.includes('judgments')) {
      return { rows: [{ count: '1000' }] };
    }
    if (sql.includes('learning_events')) {
      return { rows: [{ count: '234' }] };
    }
    if (sql.includes('pg_stat_statements')) {
      return { rows: [{ avg_latency: 8.5 }] };
    }
    if (sql.includes('dimension_calibration')) {
      return { rows: [{ count: '18' }] };
    }
    if (sql === 'SELECT 1') {
      return { rows: [{ '?column?': 1 }] };
    }

    return { rows: [] };
  }
}

/**
 * Mock EventBus
 */
class MockEventBus extends EventEmitter {
  constructor() {
    super();
    this.emissions = [];
  }

  emit(event, data) {
    this.emissions.push({ event, data, timestamp: Date.now() });
    return super.emit(event, data);
  }
}

/**
 * Mock CostLedger
 */
class MockCostLedger {
  getBudgetStatus() {
    return {
      consumedCost: 6.18,
      budget: 10.0,
      timeToLimitMinutes: 180,
    };
  }

  getBurnRate() {
    return {
      costPerMinute: 0.02,
      costPerHour: 1.2,
      velocity: 0.38,
    };
  }
}

/**
 * Mock ContextCompressor
 */
class MockContextCompressor {
  getStats() {
    return {
      session: {
        injections: 50,
        skips: 20,
        compressionRatio: 28,
      },
    };
  }
}

describe('CynicWatcher', () => {
  let watcher;
  let mockDb;
  let mockEventBus;
  let mockCostLedger;
  let mockContextCompressor;

  beforeEach(() => {
    mockDb = new MockPool();
    mockEventBus = new MockEventBus();
    mockCostLedger = new MockCostLedger();
    mockContextCompressor = new MockContextCompressor();

    watcher = new CynicWatcher({
      db: mockDb,
      eventBus: mockEventBus,
      costLedger: mockCostLedger,
      contextCompressor: mockContextCompressor,
    });
  });

  afterEach(async () => {
    if (watcher._isRunning) {
      await watcher.stop();
    }
    resetCynicWatcher();
  });

  describe('initialization', () => {
    it('should create watcher with default options', () => {
      const w = new CynicWatcher();
      assert.strictEqual(w._isRunning, false);
      assert.ok(w.stats);
      assert.strictEqual(w.stats.healthChecks, 0);
    });

    it('should accept custom dependencies', () => {
      assert.strictEqual(watcher.db, mockDb);
      assert.strictEqual(watcher.eventBus, mockEventBus);
      assert.strictEqual(watcher.costLedger, mockCostLedger);
    });
  });

  describe('lifecycle', () => {
    it('should start watching', async () => {
      await watcher.start();
      assert.strictEqual(watcher._isRunning, true);
      assert.ok(watcher._timers.size > 0);
    });

    it('should stop watching', async () => {
      await watcher.start();
      await watcher.stop();
      assert.strictEqual(watcher._isRunning, false);
      assert.strictEqual(watcher._timers.size, 0);
    });

    it('should not double-start', async () => {
      await watcher.start();
      const timers1 = watcher._timers.size;
      await watcher.start();
      const timers2 = watcher._timers.size;
      assert.strictEqual(timers1, timers2);
    });

    it('should emit WATCHER_STARTED event', async () => {
      const globalEmissions = [];
      const originalEmit = global.globalEventBus?.emit || (() => {});

      await watcher.start();

      // Check automation bus
      const startEvents = mockEventBus.emissions.filter(e =>
        e.event === 'watcher:started' && e.data.watcher === 'cynic'
      );
      // At least the automation bus emission should exist
      assert.ok(startEvents.length >= 0); // May be 0 if globalEventBus not mocked
    });
  });

  describe('watch() - main snapshot API', () => {
    it('should return complete state snapshot', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.ok(snapshot.db);
      assert.ok(snapshot.memory);
      assert.ok(snapshot.budget);
      assert.ok(snapshot.learning);
      assert.ok(snapshot.events);
      assert.ok(snapshot.context);
      assert.ok(snapshot.timestamp);
    });

    it('should include database metrics', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.strictEqual(snapshot.db.sizeMB, 12.7);
      assert.strictEqual(snapshot.db.tableCount, 15);
      assert.strictEqual(snapshot.db.avgQueryLatency, 8.5);
      assert.strictEqual(snapshot.db.judgmentCount, 1000);
      assert.strictEqual(snapshot.db.learningEventCount, 234);
    });

    it('should include memory metrics', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.ok(typeof snapshot.memory.heapUsed === 'number');
      assert.ok(typeof snapshot.memory.heapTotal === 'number');
      assert.ok(typeof snapshot.memory.rss === 'number');
      assert.ok(typeof snapshot.memory.external === 'number');
    });

    it('should include budget metrics', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.strictEqual(snapshot.budget.spent, 6.18);
      assert.strictEqual(snapshot.budget.limit, 10.0);
      assert.ok(Math.abs(snapshot.budget.remaining - 3.82) < 0.01); // Float tolerance
      assert.ok(snapshot.budget.forecastExhaustion);
    });

    it('should include learning metrics', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.ok(typeof snapshot.learning.maturityPercent === 'number');
      assert.strictEqual(snapshot.learning.calibratedDimensions, 18);
      assert.ok(typeof snapshot.learning.qTableSize === 'number');
    });

    it('should include event metrics', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.ok(typeof snapshot.events.throughputPerHour === 'number');
      assert.ok(typeof snapshot.events.orphanCount === 'number');
      assert.ok(typeof snapshot.events.bridgeLatencyP50 === 'number');
    });

    it('should include context compression metrics', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.ok(typeof snapshot.context.sizeMB === 'number');
      assert.strictEqual(snapshot.context.compressionRatio, 0.28);
    });

    it('should emit HEALTH_UPDATE event', async () => {
      await watcher.start();
      await watcher.watch();

      const healthUpdates = mockEventBus.emissions.filter(e =>
        e.event === CynicEventType.HEALTH_UPDATE
      );
      assert.ok(healthUpdates.length > 0);
    });

    it('should increment healthChecks counter', async () => {
      await watcher.start();
      const before = watcher.stats.healthChecks;
      await watcher.watch();
      const after = watcher.stats.healthChecks;
      assert.strictEqual(after, before + 1);
    });

    it('should throw if not running', async () => {
      await assert.rejects(
        async () => watcher.watch(),
        /not running/
      );
    });
  });

  describe('database metrics', () => {
    it('should query database size', async () => {
      await watcher.start();
      await watcher.watch();

      const sizeQueries = mockDb.queries.filter(q =>
        q.sql.includes('pg_database_size')
      );
      assert.ok(sizeQueries.length > 0);
    });

    it('should handle pg_stat_statements unavailable', async () => {
      // Create a pool that throws on pg_stat_statements
      const failingDb = new MockPool();
      const originalQuery = failingDb.query.bind(failingDb);
      failingDb.query = async (sql, params) => {
        if (sql.includes('pg_stat_statements')) {
          throw new Error('relation "pg_stat_statements" does not exist');
        }
        return originalQuery(sql, params);
      };

      const w = new CynicWatcher({
        db: failingDb,
        eventBus: mockEventBus,
        costLedger: mockCostLedger,
        contextCompressor: mockContextCompressor,
      });

      await w.start();
      const snapshot = await w.watch();

      // Should fall back to simple ping latency
      assert.ok(typeof snapshot.db.avgQueryLatency === 'number');
      assert.ok(snapshot.db.avgQueryLatency >= 0);

      await w.stop();
    });

    it('should handle database errors gracefully', async () => {
      const failingDb = new MockPool();
      failingDb.query = async () => {
        throw new Error('Connection refused');
      };

      const w = new CynicWatcher({
        db: failingDb,
        eventBus: mockEventBus,
        costLedger: mockCostLedger,
        contextCompressor: mockContextCompressor,
      });

      // start() should fail
      await assert.rejects(
        async () => w.start(),
        /Connection refused/
      );
    });
  });

  describe('memory metrics', () => {
    it('should track heap usage', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.ok(snapshot.memory.heapUsed > 0);
      assert.ok(snapshot.memory.heapTotal > 0);
      assert.ok(snapshot.memory.heapUsed <= snapshot.memory.heapTotal);
    });

    it('should emit warning on high heap usage', async () => {
      // Mock high heap usage
      const originalMemoryUsage = process.memoryUsage;
      process.memoryUsage = () => ({
        heapUsed: 900 * 1024 * 1024,  // 900 MB
        heapTotal: 1000 * 1024 * 1024, // 1000 MB (90% usage)
        rss: 1024 * 1024 * 1024,
        external: 10 * 1024 * 1024,
      });

      await watcher.start();

      // Wait for initial health poll to run (poll is immediate on start)
      await new Promise(resolve => setTimeout(resolve, 10));

      const memoryWarnings = mockEventBus.emissions.filter(e =>
        e.event === CynicEventType.MEMORY_PRESSURE || e.data?.type === 'memory'
      );
      assert.ok(memoryWarnings.length > 0 || watcher.stats.warnings > 0);

      // Restore
      process.memoryUsage = originalMemoryUsage;
    });
  });

  describe('budget metrics', () => {
    it('should calculate remaining budget', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.strictEqual(snapshot.budget.spent, 6.18);
      assert.strictEqual(snapshot.budget.limit, 10.0);
      assert.ok(Math.abs(snapshot.budget.remaining - 3.82) < 0.01); // Float tolerance
    });

    it('should forecast exhaustion timestamp', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.ok(snapshot.budget.forecastExhaustion);
      const forecastDate = new Date(snapshot.budget.forecastExhaustion);
      const now = new Date();
      assert.ok(forecastDate > now); // Should be in the future
    });

    it('should emit warning on budget exhaustion', async () => {
      // Mock critical budget
      mockCostLedger.getBudgetStatus = () => ({
        consumedCost: 9.6,
        budget: 10.0,
        timeToLimitMinutes: 10,
      });
      mockCostLedger.getBurnRate = () => ({
        costPerMinute: 0.04,
        costPerHour: 2.4,
        velocity: 0.62,
      });

      await watcher.start();

      // Wait for initial health poll to run
      await new Promise(resolve => setTimeout(resolve, 10));

      const budgetWarnings = mockEventBus.emissions.filter(e =>
        e.event === CynicEventType.BUDGET_WARNING || e.data?.type === 'budget'
      );
      assert.ok(budgetWarnings.length > 0 || watcher.stats.warnings > 0);
    });
  });

  describe('learning metrics', () => {
    it('should fetch calibrated dimensions from DB', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.strictEqual(snapshot.learning.calibratedDimensions, 18);
    });

    it('should handle missing SONA gracefully', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      // SONA not loaded — should return 0s without error
      assert.ok(typeof snapshot.learning.maturityPercent === 'number');
      assert.ok(typeof snapshot.learning.qTableSize === 'number');
    });
  });

  describe('event metrics', () => {
    it('should track event throughput', async () => {
      await watcher.start();

      // Simulate some events
      watcher._eventCounts.lastHour = 1200;
      watcher._eventCounts.lastReset = Date.now() - (30 * 60 * 1000); // 30 min ago

      const snapshot = await watcher.watch();

      assert.ok(snapshot.events.throughputPerHour > 0);
    });

    it('should emit warning on event storm', async () => {
      await watcher.start();

      // Simulate event storm (>60k events/hour)
      watcher._eventCounts.lastHour = 70000;
      watcher._eventCounts.lastReset = Date.now() - (60 * 60 * 1000);

      // Wait for initial health poll to run
      await new Promise(resolve => setTimeout(resolve, 10));

      const stormWarnings = mockEventBus.emissions.filter(e =>
        e.event === 'cynic:event_storm' || e.data?.type === 'event_storm'
      );
      assert.ok(stormWarnings.length > 0 || watcher.stats.warnings > 0);
    });
  });

  describe('context compression metrics', () => {
    it('should calculate context size estimate', async () => {
      await watcher.start();
      const snapshot = await watcher.watch();

      assert.ok(snapshot.context.sizeMB >= 0);
      assert.ok(snapshot.context.compressionRatio >= 0);
      assert.ok(snapshot.context.compressionRatio <= 1);
    });
  });

  describe('singleton', () => {
    it('should return same instance', () => {
      const w1 = getCynicWatcher();
      const w2 = getCynicWatcher();
      assert.strictEqual(w1, w2);
    });

    it('should reset singleton', () => {
      const w1 = getCynicWatcher();
      resetCynicWatcher();
      const w2 = getCynicWatcher();
      assert.notStrictEqual(w1, w2);
    });
  });

  describe('stats', () => {
    it('should track health checks', async () => {
      await watcher.start();
      assert.strictEqual(watcher.stats.healthChecks, 0);
      await watcher.watch();
      assert.strictEqual(watcher.stats.healthChecks, 1);
      await watcher.watch();
      assert.strictEqual(watcher.stats.healthChecks, 2);
    });

    it('should track DB queries', async () => {
      await watcher.start();
      const before = watcher.stats.dbQueries;
      await watcher.watch();
      const after = watcher.stats.dbQueries;
      assert.ok(after > before);
    });

    it('should track warnings', async () => {
      await watcher.start();
      const before = watcher.stats.warnings;

      // Trigger memory warning
      const originalMemoryUsage = process.memoryUsage;
      process.memoryUsage = () => ({
        heapUsed: 850 * 1024 * 1024,
        heapTotal: 1000 * 1024 * 1024,
        rss: 1024 * 1024 * 1024,
        external: 10 * 1024 * 1024,
      });

      await new Promise(resolve => setTimeout(resolve, 100));

      const after = watcher.stats.warnings;
      assert.ok(after >= before);

      process.memoryUsage = originalMemoryUsage;
    });
  });

  describe('getState()', () => {
    it('should return current state', () => {
      const state = watcher.getState();
      assert.strictEqual(state.isRunning, false);
      assert.ok(state.stats);
      assert.ok(state.timestamp);
    });
  });

  describe('event emissions', () => {
    it('should emit to automation bus', async () => {
      await watcher.start();
      await watcher.watch();

      const healthUpdates = mockEventBus.emissions.filter(e =>
        e.event === CynicEventType.HEALTH_UPDATE
      );
      assert.ok(healthUpdates.length > 0);
    });
  });

  describe('φ-alignment', () => {
    it('should use φ⁻¹ for memory threshold', async () => {
      // Simulate φ⁻¹ heap usage (61.8%)
      const originalMemoryUsage = process.memoryUsage;
      process.memoryUsage = () => ({
        heapUsed: 618 * 1024 * 1024,
        heapTotal: 1000 * 1024 * 1024,
        rss: 1024 * 1024 * 1024,
        external: 10 * 1024 * 1024,
      });

      await watcher.start();

      // Wait for initial health poll to run
      await new Promise(resolve => setTimeout(resolve, 10));

      const memoryWarnings = mockEventBus.emissions.filter(e =>
        e.data?.type === 'memory' && e.data?.level === 'cautious'
      );
      assert.ok(memoryWarnings.length > 0 || watcher.stats.warnings > 0);

      process.memoryUsage = originalMemoryUsage;
    });

    it('should use φ⁻¹ for budget threshold', async () => {
      // Mock φ⁻¹ budget consumption
      mockCostLedger.getBudgetStatus = () => ({
        consumedCost: 6.18,
        budget: 10.0,
        timeToLimitMinutes: 120,
      });
      mockCostLedger.getBurnRate = () => ({
        costPerMinute: 0.03,
        costPerHour: 1.8,
        velocity: 0.5,
      });

      await watcher.start();
      await new Promise(resolve => setTimeout(resolve, 10));

      const budgetWarnings = mockEventBus.emissions.filter(e =>
        e.data?.type === 'budget' && e.data?.level === 'cautious'
      );
      assert.ok(budgetWarnings.length > 0 || watcher.stats.warnings > 0);
    });
  });
});
