/**
 * LearningProofQueries Tests
 *
 * Tests for learning proof system — proves CYNIC is actually learning.
 * Uses mock PostgreSQL pool.
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import { LearningProofQueries } from '../src/queries/learning-proof.js';

// ═══════════════════════════════════════════════════════════════════════════
// MOCK POOL
// ═══════════════════════════════════════════════════════════════════════════

function createMockPool(queryHandler) {
  return {
    query(sql, params) {
      if (typeof queryHandler === 'function') return queryHandler(sql, params);
      return Promise.resolve({ rows: [] });
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('LearningProofQueries', () => {
  describe('getLearningProof()', () => {
    it('returns complete proof structure', async () => {
      const pool = createMockPool(() => Promise.resolve({ rows: [{}] }));
      const q = new LearningProofQueries(pool);
      const result = await q.getLearningProof();

      assert.ok(result.verdict);
      assert.ok(result.proofs);
      assert.ok(result.proofs.rewardTrend);
      assert.ok(result.proofs.explorationDecay);
      assert.ok(result.proofs.qValueConvergence);
      assert.ok(result.proofs.patternRetention);
      assert.ok(result.proofs.errorReduction);
      assert.ok(result.thresholds);
      assert.ok(result.timestamp);
    });

    it('returns "unknown" when no data', async () => {
      const pool = createMockPool(() => Promise.resolve({ rows: [{}] }));
      const q = new LearningProofQueries(pool);
      const result = await q.getLearningProof();

      // All scores null → verdict unknown
      assert.strictEqual(result.verdict, 'unknown');
      assert.ok(result.interpretation.includes('head tilt'));
    });

    it('returns "learning" with high scores', async () => {
      const pool = createMockPool((sql) => {
        // Reward trend: positive slope
        if (sql.includes('regr_slope') && sql.includes('qlearning_episodes')) {
          return Promise.resolve({
            rows: [{ slope: 0.001, correlation: 0.7, data_points: 10 }],
          });
        }
        // Exploration decay
        if (sql.includes('exploration_rate')) {
          return Promise.resolve({
            rows: [{ exploration_rate: 0.15, total_episodes: 200, updated_at: '2026-01-20' }],
          });
        }
        // Q-value convergence
        if (sql.includes('q_table') && !sql.includes('qlearning_state')) {
          return Promise.resolve({
            rows: [{ q_table: { entries: Array(20).fill({ values: { a: 0.5, b: 0.6 } }) }, version: 5 }],
          });
        }
        // Pattern retention
        if (sql.includes('fisher_importance')) {
          return Promise.resolve({
            rows: [{ total_patterns: 50, locked_patterns: 8, important_patterns: 15, avg_fisher: 0.4, max_fisher: 0.9 }],
          });
        }
        // Error reduction
        if (sql.includes('frictions')) {
          return Promise.resolve({
            rows: [{ slope: -0.5, correlation: -0.6, data_points: 10, avg_errors: 3 }],
          });
        }
        return Promise.resolve({ rows: [{}] });
      });
      const q = new LearningProofQueries(pool);
      const result = await q.getLearningProof();

      assert.strictEqual(result.verdict, 'learning');
      assert.ok(result.interpretation.includes('tail wag'));
    });

    it('returns "not_learning" with poor scores', async () => {
      const pool = createMockPool((sql) => {
        if (sql.includes('regr_slope') && sql.includes('qlearning_episodes')) {
          return Promise.resolve({
            rows: [{ slope: -0.01, correlation: -0.8, data_points: 10 }],
          });
        }
        if (sql.includes('exploration_rate')) {
          return Promise.resolve({
            rows: [{ exploration_rate: 0.95, total_episodes: 200, updated_at: '2026-01-20' }],
          });
        }
        if (sql.includes('q_table') && !sql.includes('qlearning_state')) {
          // Small Q-table — not enough data
          return Promise.resolve({ rows: [{ q_table: { entries: [] }, version: 1 }] });
        }
        if (sql.includes('fisher_importance')) {
          return Promise.resolve({
            rows: [{ total_patterns: 50, locked_patterns: 0, important_patterns: 1, avg_fisher: 0.05, max_fisher: 0.1 }],
          });
        }
        if (sql.includes('frictions')) {
          return Promise.resolve({
            rows: [{ slope: 2.0, correlation: 0.8, data_points: 10, avg_errors: 20 }],
          });
        }
        return Promise.resolve({ rows: [{}] });
      });
      const q = new LearningProofQueries(pool);
      const result = await q.getLearningProof();

      assert.strictEqual(result.verdict, 'not_learning');
      assert.ok(result.interpretation.includes('GROWL'));
    });
  });

  describe('_getRewardTrend()', () => {
    it('handles DB errors gracefully', async () => {
      const pool = createMockPool(() => Promise.reject(new Error('connection lost')));
      const q = new LearningProofQueries(pool);
      const result = await q._getRewardTrend();

      assert.ok(result.error);
      assert.strictEqual(result.score, null);
    });

    it('returns null score with insufficient data points', async () => {
      const pool = createMockPool(() => Promise.resolve({
        rows: [{ slope: 0.1, correlation: 0.5, data_points: 2 }],
      }));
      const q = new LearningProofQueries(pool);
      const result = await q._getRewardTrend();

      assert.strictEqual(result.score, null);
    });
  });

  describe('_getExplorationDecay()', () => {
    it('returns null score with too few episodes', async () => {
      const pool = createMockPool(() => Promise.resolve({
        rows: [{ exploration_rate: 0.9, total_episodes: 10, updated_at: '2026-01-20' }],
      }));
      const q = new LearningProofQueries(pool);
      const result = await q._getExplorationDecay();

      assert.strictEqual(result.score, null);
      assert.ok(result.interpretation.includes('Not enough'));
    });

    it('scores high for low exploration with many episodes', async () => {
      const pool = createMockPool(() => Promise.resolve({
        rows: [{ exploration_rate: 0.1, total_episodes: 500, updated_at: '2026-01-20' }],
      }));
      const q = new LearningProofQueries(pool);
      const result = await q._getExplorationDecay();

      assert.ok(result.score > 0.8);
      assert.ok(result.interpretation.includes('exploiting'));
    });
  });

  describe('_getQValueConvergence()', () => {
    it('returns null score with too few Q-values', async () => {
      const pool = createMockPool(() => Promise.resolve({
        rows: [{ q_table: { entries: [{ values: { a: 1 } }] }, version: 1 }],
      }));
      const q = new LearningProofQueries(pool);
      const result = await q._getQValueConvergence();

      assert.strictEqual(result.score, null);
    });
  });

  describe('_getPatternRetention()', () => {
    it('scores based on locked pattern count', async () => {
      const pool = createMockPool(() => Promise.resolve({
        rows: [{ total_patterns: 100, locked_patterns: 10, important_patterns: 20, avg_fisher: 0.5, max_fisher: 0.95 }],
      }));
      const q = new LearningProofQueries(pool);
      const result = await q._getPatternRetention();

      assert.ok(result.score > 0);
      assert.ok(result.interpretation.includes('locked'));
    });
  });

  describe('_getErrorReduction()', () => {
    it('scores high for declining errors', async () => {
      const pool = createMockPool(() => Promise.resolve({
        rows: [{ slope: -3.0, correlation: -0.9, data_points: 10, avg_errors: 5 }],
      }));
      const q = new LearningProofQueries(pool);
      const result = await q._getErrorReduction();

      assert.ok(result.score > 0.5);
      assert.ok(result.interpretation.includes('DOWN'));
    });
  });

  describe('getLearningTimeline()', () => {
    it('returns timeline data', async () => {
      const pool = createMockPool({
        rows: [
          { day: '2026-01-18', episodes: 10, avg_reward: 0.5, new_patterns: 3, errors: 2 },
          { day: '2026-01-19', episodes: 15, avg_reward: 0.6, new_patterns: 5, errors: 1 },
        ],
      });
      const q = new LearningProofQueries(pool);
      const result = await q.getLearningTimeline(7);

      assert.ok(result.timeline);
      assert.strictEqual(result.days, 7);
    });
  });
});
