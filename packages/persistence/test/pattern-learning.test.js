/**
 * @cynic/persistence - Pattern Learning Tests
 *
 * v1.1: Tests for pattern learning service
 *
 * @module @cynic/persistence/test/pattern-learning
 */

import { describe, it, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  PatternLearning,
  createPatternLearning,
  DEFAULT_PATTERN_CONFIG,
} from '../src/services/pattern-learning.js';

// =============================================================================
// MOCK HELPERS
// =============================================================================

function createMockPool(queryResults = {}) {
  return {
    query: mock.fn(async (sql) => queryResults[sql] || { rows: [], rowCount: 0 }),
  };
}

// =============================================================================
// DEFAULT CONFIG TESTS
// =============================================================================

describe('DEFAULT_PATTERN_CONFIG', () => {
  it('should have φ-derived values', () => {
    assert.ok(DEFAULT_PATTERN_CONFIG.extractionThreshold > 0.6);
    assert.ok(DEFAULT_PATTERN_CONFIG.similarityThreshold > 0.6);
    assert.ok(DEFAULT_PATTERN_CONFIG.minConfidence > 0.2);
    assert.ok(DEFAULT_PATTERN_CONFIG.confidenceDecay < 0.1);
  });

  it('should have Fibonacci batch sizes', () => {
    assert.strictEqual(DEFAULT_PATTERN_CONFIG.maxPatternsPerCategory, 21);
    assert.strictEqual(DEFAULT_PATTERN_CONFIG.decayPeriodDays, 13);
    assert.strictEqual(DEFAULT_PATTERN_CONFIG.batchSize, 34);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(DEFAULT_PATTERN_CONFIG));
  });
});

// =============================================================================
// PATTERN LEARNING TESTS
// =============================================================================

describe('PatternLearning', () => {
  describe('Construction', () => {
    it('should require a pool', () => {
      assert.throws(() => new PatternLearning(), /requires.*pool/i);
    });

    it('should create with pool', () => {
      const pool = createMockPool();
      const service = new PatternLearning({ pool });

      assert.ok(service);
      const stats = service.getStats();
      assert.strictEqual(stats.totalExtracted, 0);
    });

    it('should accept custom config', () => {
      const pool = createMockPool();
      const service = new PatternLearning({
        pool,
        config: { decayPeriodDays: 7 },
      });

      const stats = service.getStats();
      assert.strictEqual(stats.config.decayPeriodDays, 7);
    });
  });

  describe('Pattern Extraction', () => {
    it('should extract patterns from high-quality judgments', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('FROM judgments')) {
          return {
            rows: [{
              judgment_id: 'jud_123',
              q_score: 80,
              verdict: 'WAG',
              subject_type: 'code',
              subject_name: 'Clean Code Pattern',
              reasoning: 'Well structured code with good patterns',
              data: '{}',
            }],
          };
        }
        if (sql.includes('INSERT INTO patterns')) {
          return { rows: [] };
        }
        return { rows: [] };
      });

      const service = new PatternLearning({ pool });
      const results = await service.extractFromJudgments({ dryRun: true });

      assert.strictEqual(results.processed, 1);
      assert.strictEqual(results.extracted.length, 1);
      assert.strictEqual(results.extracted[0].category, 'code_quality');
    });

    it('should skip low-quality judgments', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({ rows: [] }));

      const service = new PatternLearning({ pool });
      const results = await service.extractFromJudgments();

      assert.strictEqual(results.extracted.length, 0);
    });
  });

  describe('Confidence Decay', () => {
    it('should decay stale patterns', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('SELECT pattern_id')) {
          return {
            rows: [{
              pattern_id: 'pat_123',
              name: 'Old Pattern',
              confidence: 0.8,
              frequency: 5,
            }],
          };
        }
        if (sql.includes('DELETE FROM patterns')) {
          return { rowCount: 0 };
        }
        return { rows: [] };
      });

      const service = new PatternLearning({ pool });
      const results = await service.applyConfidenceDecay({ dryRun: true });

      assert.strictEqual(results.decayed, 1);
      assert.ok(results.patterns[0].newConfidence < results.patterns[0].oldConfidence);
    });
  });

  describe('Pattern Reinforcement', () => {
    it('should reinforce patterns', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({
        rows: [{ pattern_id: 'pat_123', confidence: 0.85, frequency: 6 }],
      }));

      const service = new PatternLearning({ pool });
      const result = await service.reinforcePattern('pat_123', 0.05);

      assert.ok(result);
      assert.strictEqual(pool.query.mock.calls.length, 1);
    });

    it('should weaken patterns', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({
        rows: [{ pattern_id: 'pat_123', confidence: 0.65, frequency: 5 }],
      }));

      const service = new PatternLearning({ pool });
      const result = await service.weakenPattern('pat_123', 0.1);

      assert.ok(result);
    });
  });

  describe('Full Cycle', () => {
    it('should run full learning cycle', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('DISTINCT category')) {
          return { rows: [] };
        }
        return { rows: [], rowCount: 0 };
      });

      const service = new PatternLearning({ pool });
      const results = await service.runCycle({ dryRun: true });

      assert.ok(results.timestamp);
      assert.ok('extraction' in results);
      assert.ok('decay' in results);
      assert.ok(results.duration >= 0);
    });
  });
});

// =============================================================================
// FACTORY FUNCTION TESTS
// =============================================================================

describe('createPatternLearning', () => {
  it('should create instance', () => {
    const pool = createMockPool();
    const service = createPatternLearning({ pool });

    assert.ok(service instanceof PatternLearning);
  });
});
