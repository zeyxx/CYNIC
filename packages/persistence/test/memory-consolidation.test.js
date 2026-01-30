/**
 * @cynic/persistence - Memory Consolidation Tests
 *
 * v1.1: Tests for memory consolidation service
 *
 * @module @cynic/persistence/test/memory-consolidation
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  MemoryConsolidation,
  createMemoryConsolidation,
  DEFAULT_CONSOLIDATION_CONFIG,
} from '../src/services/memory-consolidation.js';

// =============================================================================
// MOCK HELPERS
// =============================================================================

function createMockPool(queryResults = {}) {
  return {
    query: mock.fn(async (sql) => {
      // Return empty by default
      return queryResults[sql] || { rows: [] };
    }),
  };
}

function createMockEmbedder() {
  return {
    embed: mock.fn(async (text) => {
      // Generate deterministic embedding based on text
      const hash = text.split('').reduce((a, c) => a + c.charCodeAt(0), 0);
      return Array(128).fill(0).map((_, i) => Math.sin(hash + i) * 0.5);
    }),
  };
}

// =============================================================================
// DEFAULT CONFIG TESTS
// =============================================================================

describe('DEFAULT_CONSOLIDATION_CONFIG', () => {
  it('should have φ-derived values', () => {
    assert.ok(DEFAULT_CONSOLIDATION_CONFIG.similarityThreshold > 0.6);
    assert.ok(DEFAULT_CONSOLIDATION_CONFIG.similarityThreshold < 0.62);
    assert.ok(DEFAULT_CONSOLIDATION_CONFIG.importanceDecay > 0.38);
    assert.ok(DEFAULT_CONSOLIDATION_CONFIG.pruneThreshold > 0.23);
    assert.ok(DEFAULT_CONSOLIDATION_CONFIG.pruneThreshold < 0.24);
  });

  it('should have Fibonacci batch sizes', () => {
    assert.strictEqual(DEFAULT_CONSOLIDATION_CONFIG.maxMergePerRun, 21);
    assert.strictEqual(DEFAULT_CONSOLIDATION_CONFIG.maxPrunePerRun, 34);
    assert.strictEqual(DEFAULT_CONSOLIDATION_CONFIG.staleAfterDays, 13);
    assert.strictEqual(DEFAULT_CONSOLIDATION_CONFIG.batchSize, 55);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(DEFAULT_CONSOLIDATION_CONFIG));
  });
});

// =============================================================================
// MEMORY CONSOLIDATION TESTS
// =============================================================================

describe('MemoryConsolidation', () => {
  describe('Construction', () => {
    it('should require a pool', () => {
      assert.throws(() => new MemoryConsolidation(), /requires.*pool/i);
    });

    it('should create with pool', () => {
      const pool = createMockPool();
      const consolidation = new MemoryConsolidation({ pool });

      assert.ok(consolidation);
      const stats = consolidation.getStats();
      assert.strictEqual(stats.totalRuns, 0);
    });

    it('should accept custom config', () => {
      const pool = createMockPool();
      const consolidation = new MemoryConsolidation({
        pool,
        config: { staleAfterDays: 7 },
      });

      const stats = consolidation.getStats();
      assert.strictEqual(stats.config.staleAfterDays, 7);
    });
  });

  describe('Consolidation Run', () => {
    it('should run consolidation (dry run)', async () => {
      const pool = createMockPool();
      const consolidation = new MemoryConsolidation({ pool });

      const results = await consolidation.consolidate({ dryRun: true });

      assert.ok(results.timestamp);
      assert.strictEqual(results.dryRun, true);
      assert.ok('merged' in results);
      assert.ok('decayed' in results);
      assert.ok('pruned' in results);
    });

    it('should track run statistics', async () => {
      const pool = createMockPool();
      const consolidation = new MemoryConsolidation({ pool });

      await consolidation.consolidate({ dryRun: true });

      const stats = consolidation.getStats();
      assert.strictEqual(stats.totalRuns, 1);
      assert.ok(stats.lastRun);
      assert.ok(stats.lastRunDuration >= 0);
    });
  });

  describe('Decay Stale Memories', () => {
    it('should identify stale memories for decay', async () => {
      const staleMemory = {
        id: 'mem1',
        user_id: 'user1',
        importance: 0.8,
        access_count: 0,
        last_accessed_at: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000), // 30 days ago
      };

      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('SELECT id')) {
          return { rows: [staleMemory] };
        }
        return { rows: [] };
      });

      const consolidation = new MemoryConsolidation({ pool });
      const results = await consolidation.consolidate({ dryRun: true });

      assert.strictEqual(results.decayed.count, 1);
      assert.strictEqual(results.decayed.memories[0].id, 'mem1');
      assert.ok(results.decayed.memories[0].newImportance < 0.8);
    });
  });

  describe('Prune Low-Value Memories', () => {
    it('should identify low-value memories for pruning', async () => {
      const lowValueMemory = {
        id: 'mem2',
        user_id: 'user1',
        importance: 0.1,
        memory_type: 'observation',
        created_at: new Date(),
      };

      const pool = createMockPool();
      let callCount = 0;
      pool.query = mock.fn(async (sql) => {
        callCount++;
        // First call is decay (no results), second is prune
        if (callCount === 2 && sql.includes('importance <=')) {
          return { rows: [lowValueMemory] };
        }
        return { rows: [] };
      });

      const consolidation = new MemoryConsolidation({ pool });
      const results = await consolidation.consolidate({ dryRun: true });

      assert.strictEqual(results.pruned.count, 1);
      assert.strictEqual(results.pruned.memories[0].id, 'mem2');
    });
  });

  describe('Merge Similar Memories', () => {
    it('should skip merge without embedder', async () => {
      const pool = createMockPool();
      const consolidation = new MemoryConsolidation({ pool });

      const results = await consolidation.consolidate({ dryRun: true });

      assert.strictEqual(results.merged.count, 0);
    });

    it('should identify similar memories for merge', async () => {
      const similarMemories = [
        {
          id: 'mem1',
          user_id: 'user1',
          memory_type: 'insight',
          content: 'Hello world',
          embedding: Array(128).fill(0.5),
          importance: 0.7,
          access_count: 2,
        },
        {
          id: 'mem2',
          user_id: 'user1',
          memory_type: 'insight',
          content: 'Hello world again',
          embedding: Array(128).fill(0.5), // Same embedding = 100% similar
          importance: 0.5,
          access_count: 1,
        },
      ];

      const pool = createMockPool();
      let queryType = 0;
      pool.query = mock.fn(async (sql) => {
        queryType++;
        // Query 1: decay (no results)
        // Query 2: merge candidates
        if (queryType === 2 && sql.includes('embedding IS NOT NULL')) {
          return { rows: similarMemories };
        }
        return { rows: [] };
      });

      const embedder = createMockEmbedder();
      const consolidation = new MemoryConsolidation({ pool, embedder });

      const results = await consolidation.consolidate({ dryRun: true });

      assert.strictEqual(results.merged.count, 1);
      assert.strictEqual(results.merged.pairs[0].memory1, 'mem1');
      assert.strictEqual(results.merged.pairs[0].memory2, 'mem2');
      // Identical embeddings = ~1.0 similarity (allow floating point tolerance)
      assert.ok(results.merged.pairs[0].similarity > 0.99);
    });
  });

  describe('Importance Calculation', () => {
    it('should calculate φ-weighted importance', () => {
      const pool = createMockPool();
      const consolidation = new MemoryConsolidation({ pool });

      const memory = {
        importance: 0.8,
        access_count: 5,
        content: 'A'.repeat(500),
        created_at: new Date(),
        last_accessed_at: new Date(),
      };

      const calculated = consolidation.calculateImportance(memory);

      assert.ok(calculated > 0);
      assert.ok(calculated <= 1);
      // High importance + good access + recent = should be high
      assert.ok(calculated > 0.5);
    });

    it('should decay importance for old memories', () => {
      const pool = createMockPool();
      const consolidation = new MemoryConsolidation({ pool });

      const oldDate = new Date();
      oldDate.setDate(oldDate.getDate() - 30);

      const oldMemory = {
        importance: 0.5,
        access_count: 0,
        content: 'test',
        created_at: oldDate,
        last_accessed_at: oldDate,
      };

      const newMemory = {
        importance: 0.5,
        access_count: 0,
        content: 'test',
        created_at: new Date(),
        last_accessed_at: new Date(),
      };

      const oldScore = consolidation.calculateImportance(oldMemory);
      const newScore = consolidation.calculateImportance(newMemory);

      assert.ok(oldScore < newScore, 'Old memory should have lower score');
    });
  });

  describe('Boost Importance', () => {
    it('should boost memory importance', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({
        rows: [{ id: 'mem1', importance: 0.6, access_count: 2 }],
      }));

      const consolidation = new MemoryConsolidation({ pool });
      const result = await consolidation.boostImportance('mem1', 0.1);

      assert.ok(result);
      assert.strictEqual(pool.query.mock.calls.length, 1);
    });
  });

  describe('Health Metrics', () => {
    it('should calculate health metrics', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({
        rows: [{
          total: '100',
          avg_importance: '0.6',
          avg_access_count: '3.5',
          low_value_count: '10',
          stale_count: '15',
        }],
      }));

      const consolidation = new MemoryConsolidation({ pool });
      const metrics = await consolidation.getHealthMetrics();

      assert.strictEqual(metrics.total, 100);
      assert.ok(metrics.avgImportance > 0);
      assert.ok(metrics.healthScore > 0);
      assert.ok(metrics.healthScore <= 1);
      assert.ok('thresholds' in metrics);
    });
  });
});

// =============================================================================
// FACTORY FUNCTION TESTS
// =============================================================================

describe('createMemoryConsolidation', () => {
  it('should create instance', () => {
    const pool = createMockPool();
    const consolidation = createMemoryConsolidation({ pool });

    assert.ok(consolidation instanceof MemoryConsolidation);
  });
});
