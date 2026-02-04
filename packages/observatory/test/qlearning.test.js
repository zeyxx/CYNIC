/**
 * QLearningQueries Tests
 *
 * Tests for Q-Table visualization and learning metric queries.
 * Uses mock PostgreSQL pool.
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import { QLearningQueries } from '../src/queries/qlearning.js';

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

describe('QLearningQueries', () => {
  describe('getQTableStats()', () => {
    it('returns services with stats', async () => {
      const pool = createMockPool({
        rows: [{
          service_id: 'default',
          entry_count: 42,
          exploration_rate: 0.3,
          total_episodes: 150,
          total_reward: 45.5,
          version: 3,
          updated_at: '2026-01-20',
        }],
      });
      const q = new QLearningQueries(pool);
      const result = await q.getQTableStats();

      assert.strictEqual(result.services.length, 1);
      assert.strictEqual(result.services[0].service_id, 'default');
      assert.strictEqual(result.services[0].entry_count, 42);
      assert.ok(result.timestamp);
    });

    it('returns empty services when no data', async () => {
      const q = new QLearningQueries(createMockPool());
      const result = await q.getQTableStats();

      assert.strictEqual(result.services.length, 0);
    });
  });

  describe('getEpisodeHistory()', () => {
    it('returns episodes with defaults', async () => {
      const pool = createMockPool((sql, params) => {
        assert.strictEqual(params[0], 100); // default limit
        return Promise.resolve({
          rows: [
            { episode_id: 'ep1', service_id: 'default', reward: 0.8, created_at: '2026-01-20' },
          ],
        });
      });
      const q = new QLearningQueries(pool);
      const result = await q.getEpisodeHistory();

      assert.strictEqual(result.count, 1);
      assert.strictEqual(result.episodes[0].episode_id, 'ep1');
    });

    it('filters by serviceId', async () => {
      const pool = createMockPool((sql, params) => {
        assert.ok(sql.includes('WHERE service_id = $2'));
        assert.strictEqual(params[1], 'my-service');
        return Promise.resolve({ rows: [] });
      });
      const q = new QLearningQueries(pool);
      await q.getEpisodeHistory({ serviceId: 'my-service' });
    });

    it('respects limit parameter', async () => {
      const pool = createMockPool((sql, params) => {
        assert.strictEqual(params[0], 50);
        return Promise.resolve({ rows: [] });
      });
      const q = new QLearningQueries(pool);
      await q.getEpisodeHistory({ limit: 50 });
    });
  });

  describe('getLearningCurve()', () => {
    it('detects improving trend', async () => {
      // Rows in DESC order (as query returns), code reverses to ASC
      const pool = createMockPool({
        rows: [
          { time_bucket: '2026-01-18', avg_reward: 0.7, episode_count: 18 },
          { time_bucket: '2026-01-17', avg_reward: 0.5, episode_count: 15 },
          { time_bucket: '2026-01-16', avg_reward: 0.35, episode_count: 12 },
          { time_bucket: '2026-01-15', avg_reward: 0.3, episode_count: 10 },
        ],
      });
      const q = new QLearningQueries(pool);
      const result = await q.getLearningCurve();

      assert.strictEqual(result.trend, 'improving');
      assert.ok(result.interpretation.includes('tail wag'));
    });

    it('detects degrading trend', async () => {
      // Rows in DESC order (as query returns), code reverses to ASC
      const pool = createMockPool({
        rows: [
          { time_bucket: '2026-01-18', avg_reward: 0.1, episode_count: 18 },
          { time_bucket: '2026-01-17', avg_reward: 0.3, episode_count: 15 },
          { time_bucket: '2026-01-16', avg_reward: 0.6, episode_count: 12 },
          { time_bucket: '2026-01-15', avg_reward: 0.8, episode_count: 10 },
        ],
      });
      const q = new QLearningQueries(pool);
      const result = await q.getLearningCurve();

      assert.strictEqual(result.trend, 'degrading');
      assert.ok(result.interpretation.includes('GROWL'));
    });

    it('defaults to stable with equal halves', async () => {
      const pool = createMockPool({
        rows: [
          { time_bucket: '2026-01-15', avg_reward: 0.5, episode_count: 10 },
          { time_bucket: '2026-01-16', avg_reward: 0.5, episode_count: 10 },
          { time_bucket: '2026-01-17', avg_reward: 0.5, episode_count: 10 },
          { time_bucket: '2026-01-18', avg_reward: 0.5, episode_count: 10 },
        ],
      });
      const q = new QLearningQueries(pool);
      const result = await q.getLearningCurve();

      assert.strictEqual(result.trend, 'stable');
      assert.ok(result.interpretation.includes('sniff'));
    });

    it('sanitizes invalid interval', async () => {
      const pool = createMockPool((sql, params) => {
        // Should fallback to 'hour' for invalid interval
        assert.strictEqual(params[0], 'hour');
        return Promise.resolve({ rows: [] });
      });
      const q = new QLearningQueries(pool);
      await q.getLearningCurve({ interval: 'DROP TABLE; --' });
    });
  });

  describe('getQValuesHeatmap()', () => {
    it('returns heatmap from Q-table', async () => {
      const pool = createMockPool({
        rows: [{
          q_table: {
            entries: [
              { features: 'state1', values: { action1: 0.8, action2: 0.3 }, visits: { action1: 10, action2: 5 } },
              { features: 'state2', values: { action1: 0.5 }, visits: { action1: 7 } },
            ],
          },
        }],
      });
      const q = new QLearningQueries(pool);
      const result = await q.getQValuesHeatmap('default');

      assert.strictEqual(result.entryCount, 2);
      assert.strictEqual(result.heatmap.length, 2);
      assert.strictEqual(result.heatmap[0].features, 'state1');
      assert.strictEqual(result.heatmap[0].actions.length, 2);
    });

    it('returns empty when no Q-table found', async () => {
      const q = new QLearningQueries(createMockPool());
      const result = await q.getQValuesHeatmap('nonexistent');

      assert.deepStrictEqual(result.heatmap, []);
      assert.ok(result.message.includes('No Q-Table'));
    });
  });

  describe('getTopActions()', () => {
    it('returns top performing actions', async () => {
      const pool = createMockPool({
        rows: [
          { action: 'brain_search', usage_count: 50, avg_reward: 0.85, success_rate: 0.9 },
          { action: 'brain_judge', usage_count: 30, avg_reward: 0.72, success_rate: 0.8 },
        ],
      });
      const q = new QLearningQueries(pool);
      const result = await q.getTopActions(10);

      assert.strictEqual(result.actions.length, 2);
      assert.strictEqual(result.actions[0].action, 'brain_search');
    });
  });
});
