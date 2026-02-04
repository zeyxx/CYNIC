/**
 * PatternsQueries Tests
 *
 * Tests for pattern detection and EWC++ Fisher score queries.
 * Uses mock PostgreSQL pool.
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import { PatternsQueries } from '../src/queries/patterns.js';

// ═══════════════════════════════════════════════════════════════════════════
// MOCK POOL
// ═══════════════════════════════════════════════════════════════════════════

function createMockPool(queryResult) {
  return {
    query(sql, params) {
      if (typeof queryResult === 'function') return queryResult(sql, params);
      return Promise.resolve(queryResult || { rows: [] });
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('PatternsQueries', () => {
  describe('getRecentPatterns()', () => {
    it('returns formatted patterns', async () => {
      const pool = createMockPool({
        rows: [{
          id: 1, pattern_id: 'pat_1', type: 'tool_usage', signature: 'search_then_judge',
          description: 'Search before judging', confidence: 0.8, occurrences: 15,
          fisher_importance: 0.7, metadata: {}, created_at: '2026-01-20', updated_at: '2026-01-20',
        }],
      });
      const q = new PatternsQueries(pool);
      const result = await q.getRecentPatterns();

      assert.strictEqual(result.count, 1);
      assert.strictEqual(result.patterns[0].pattern_id, 'pat_1');
      assert.ok(result.timestamp);
    });

    it('filters by category', async () => {
      const pool = createMockPool((sql, params) => {
        assert.ok(sql.includes('WHERE category = $2'));
        assert.strictEqual(params[1], 'error');
        return Promise.resolve({ rows: [] });
      });
      const q = new PatternsQueries(pool);
      await q.getRecentPatterns({ category: 'error' });
    });

    it('respects limit', async () => {
      const pool = createMockPool((sql, params) => {
        assert.strictEqual(params[0], 10);
        return Promise.resolve({ rows: [] });
      });
      const q = new PatternsQueries(pool);
      await q.getRecentPatterns({ limit: 10 });
    });
  });

  describe('getImportantPatterns()', () => {
    it('returns patterns sorted by Fisher importance', async () => {
      const pool = createMockPool({
        rows: [
          { id: 1, pattern_id: 'p1', type: 'a', signature: 's1', fisher_importance: 0.9, ewc_status: 'locked', confidence: 0.8, occurrences: 20 },
          { id: 2, pattern_id: 'p2', type: 'b', signature: 's2', fisher_importance: 0.5, ewc_status: 'important', confidence: 0.6, occurrences: 10 },
        ],
      });
      const q = new PatternsQueries(pool);
      const result = await q.getImportantPatterns();

      assert.strictEqual(result.lockedCount, 1);
      assert.strictEqual(result.importantCount, 1);
      assert.ok(result.ewcThreshold > 0.5);
    });
  });

  describe('getPatternDistribution()', () => {
    it('returns category breakdown', async () => {
      const pool = createMockPool({
        rows: [
          { type: 'tool_usage', count: 45, avg_confidence: 0.7, avg_fisher: 0.3, total_occurrences: 200 },
          { type: 'error', count: 12, avg_confidence: 0.5, avg_fisher: 0.2, total_occurrences: 50 },
        ],
      });
      const q = new PatternsQueries(pool);
      const result = await q.getPatternDistribution();

      assert.strictEqual(result.totalTypes, 2);
      assert.strictEqual(result.distribution[0].type, 'tool_usage');
    });
  });

  describe('getPatternTimeline()', () => {
    it('returns time-bucketed pattern data', async () => {
      const pool = createMockPool({
        rows: [
          { time_bucket: '2026-01-19', new_patterns: 5, avg_confidence: 0.6, unique_types: 3 },
          { time_bucket: '2026-01-20', new_patterns: 8, avg_confidence: 0.7, unique_types: 4 },
        ],
      });
      const q = new PatternsQueries(pool);
      const result = await q.getPatternTimeline();

      assert.ok(result.timeline.length > 0);
    });

    it('sanitizes interval input', async () => {
      const pool = createMockPool((sql, params) => {
        assert.strictEqual(params[0], 'hour');
        return Promise.resolve({ rows: [] });
      });
      const q = new PatternsQueries(pool);
      await q.getPatternTimeline({ interval: '; DROP TABLE patterns;' });
    });
  });

  describe('getAnomalies()', () => {
    it('returns anomaly and error patterns', async () => {
      const pool = createMockPool({
        rows: [
          { id: 1, pattern_id: 'a1', type: 'anomaly', signature: 'spike', description: 'Unusual spike', confidence: 0.9, metadata: {}, created_at: '2026-01-20' },
        ],
      });
      const q = new PatternsQueries(pool);
      const result = await q.getAnomalies();

      assert.strictEqual(result.count, 1);
      assert.strictEqual(result.anomalies[0].type, 'anomaly');
    });
  });

  describe('getPatternCoOccurrence()', () => {
    it('returns co-occurrence pairs', async () => {
      const pool = createMockPool({
        rows: [
          { type1: 'tool_usage', type2: 'error', co_occurrences: 12 },
        ],
      });
      const q = new PatternsQueries(pool);
      const result = await q.getPatternCoOccurrence();

      assert.strictEqual(result.coOccurrences.length, 1);
      assert.strictEqual(result.coOccurrences[0].co_occurrences, 12);
    });
  });
});
