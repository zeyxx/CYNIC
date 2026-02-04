/**
 * OracleMemory Tests
 *
 * Tests for judgment persistence and trajectory calculation.
 * Uses mock PostgreSQL pool.
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import { OracleMemory } from '../src/oracle/memory.js';

// ═══════════════════════════════════════════════════════════════════════════
// MOCK POOL
// ═══════════════════════════════════════════════════════════════════════════

function createMockPool(queryResponses = []) {
  let callIndex = 0;
  const calls = [];

  return {
    calls,
    query(sql, params) {
      calls.push({ sql, params });
      if (callIndex < queryResponses.length) {
        const response = queryResponses[callIndex++];
        if (response instanceof Error) return Promise.reject(response);
        return Promise.resolve(response);
      }
      return Promise.resolve({ rows: [] });
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('OracleMemory', () => {
  describe('ensureSchema()', () => {
    it('initializes schema on first call', async () => {
      const pool = createMockPool([
        { rows: [] }, // INIT_SQL
        { rows: [] }, // MIGRATE_SQL
      ]);
      const mem = new OracleMemory(pool);

      await mem.ensureSchema();
      assert.strictEqual(pool.calls.length, 2);
      assert.ok(pool.calls[0].sql.includes('CREATE TABLE'));
    });

    it('skips on subsequent calls', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
      ]);
      const mem = new OracleMemory(pool);

      await mem.ensureSchema();
      await mem.ensureSchema();
      // Only 2 calls (init + migrate), not 4
      assert.strictEqual(pool.calls.length, 2);
    });

    it('handles schema init failure gracefully', async () => {
      const pool = createMockPool([new Error('connection refused')]);
      const mem = new OracleMemory(pool);

      // Should not throw
      await mem.ensureSchema();
      assert.strictEqual(mem._initialized, false);
    });
  });

  describe('store()', () => {
    it('inserts judgment into DB', async () => {
      const pool = createMockPool([
        { rows: [] }, // init
        { rows: [] }, // migrate
        { rows: [] }, // insert
      ]);
      const mem = new OracleMemory(pool);

      await mem.store({
        mint: 'TestMint111',
        name: 'Test',
        symbol: 'TST',
        verdict: 'WAG',
        qScore: 55,
        kScore: 45,
        confidence: 0.55,
        tier: 'Copper',
        dimensions: { a: 1 },
        weaknesses: [],
      });

      const insertCall = pool.calls[2];
      assert.ok(insertCall.sql.includes('INSERT INTO oracle_judgments'));
      assert.strictEqual(insertCall.params[0], 'TestMint111');
      assert.strictEqual(insertCall.params[3], 'WAG');
      assert.strictEqual(insertCall.params[4], 55);
    });

    it('handles store failure gracefully', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        new Error('disk full'),
      ]);
      const mem = new OracleMemory(pool);

      // Should not throw
      await mem.store({ mint: 'x', verdict: 'BARK', qScore: 10, kScore: 5, confidence: 0.3 });
    });
  });

  describe('getHistory()', () => {
    it('returns formatted history', async () => {
      const pool = createMockPool([
        { rows: [] }, // init
        { rows: [] }, // migrate
        {
          rows: [
            { verdict: 'WAG', q_score: 55, k_score: 40, confidence: 0.5, tier: 'Copper', dimensions: {}, weaknesses: [], judged_at: '2026-01-01' },
            { verdict: 'GROWL', q_score: 35, k_score: 30, confidence: 0.4, tier: 'Iron', dimensions: {}, weaknesses: [], judged_at: '2025-12-01' },
          ],
        },
      ]);
      const mem = new OracleMemory(pool);
      const history = await mem.getHistory('TestMint', 20);

      assert.strictEqual(history.length, 2);
      assert.strictEqual(history[0].verdict, 'WAG');
      assert.strictEqual(history[0].qScore, 55);
      assert.strictEqual(history[1].verdict, 'GROWL');
    });
  });

  describe('getTrajectory()', () => {
    it('returns "new" for no history', async () => {
      const pool = createMockPool([
        { rows: [] }, // init
        { rows: [] }, // migrate
        { rows: [] }, // getHistory query
      ]);
      const mem = new OracleMemory(pool);
      const traj = await mem.getTrajectory('EmptyMint');

      assert.strictEqual(traj.direction, 'new');
      assert.strictEqual(traj.delta, 0);
      assert.strictEqual(traj.previousJudgments, 0);
    });

    it('returns "new" for single history entry', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        {
          rows: [
            { verdict: 'WAG', q_score: 55, k_score: 40, confidence: 0.5, tier: 'Copper', dimensions: {}, weaknesses: [], judged_at: '2026-01-15' },
          ],
        },
      ]);
      const mem = new OracleMemory(pool);
      const traj = await mem.getTrajectory('SingleMint');

      assert.strictEqual(traj.direction, 'new');
      assert.strictEqual(traj.previousJudgments, 1);
      assert.strictEqual(traj.lastVerdict, 'WAG');
    });

    it('detects improving trajectory', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        {
          rows: [
            { verdict: 'WAG', q_score: 60, k_score: 50, confidence: 0.5, tier: 'Bronze', dimensions: {}, weaknesses: [], judged_at: '2026-01-20' },
            { verdict: 'GROWL', q_score: 35, k_score: 30, confidence: 0.4, tier: 'Iron', dimensions: {}, weaknesses: [], judged_at: '2026-01-10' },
          ],
        },
      ]);
      const mem = new OracleMemory(pool);
      const traj = await mem.getTrajectory('ImprovingMint');

      assert.strictEqual(traj.direction, 'improving');
      assert.ok(traj.delta > 0);
      assert.strictEqual(traj.verdictChanged, true);
    });

    it('detects declining trajectory', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        {
          rows: [
            { verdict: 'GROWL', q_score: 30, k_score: 25, confidence: 0.3, tier: 'Iron', dimensions: {}, weaknesses: [], judged_at: '2026-01-20' },
            { verdict: 'WAG', q_score: 55, k_score: 45, confidence: 0.5, tier: 'Copper', dimensions: {}, weaknesses: [], judged_at: '2026-01-10' },
          ],
        },
      ]);
      const mem = new OracleMemory(pool);
      const traj = await mem.getTrajectory('DecliningMint');

      assert.strictEqual(traj.direction, 'declining');
      assert.ok(traj.delta < 0);
    });

    it('detects stable trajectory (small delta)', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        {
          rows: [
            { verdict: 'WAG', q_score: 56, k_score: 45, confidence: 0.5, tier: 'Copper', dimensions: {}, weaknesses: [], judged_at: '2026-01-20' },
            { verdict: 'WAG', q_score: 55, k_score: 44, confidence: 0.5, tier: 'Copper', dimensions: {}, weaknesses: [], judged_at: '2026-01-10' },
          ],
        },
      ]);
      const mem = new OracleMemory(pool);
      const traj = await mem.getTrajectory('StableMint');

      assert.strictEqual(traj.direction, 'stable');
      assert.ok(Math.abs(traj.delta) < 3);
    });
  });

  describe('getFirstPrice()', () => {
    it('returns first recorded price', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        { rows: [{ price_per_token: 0.05, judged_at: '2026-01-01' }] },
      ]);
      const mem = new OracleMemory(pool);
      const result = await mem.getFirstPrice('TestMint');

      assert.strictEqual(result.price, 0.05);
      assert.ok(result.judgedAt);
    });

    it('returns null when no price recorded', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        { rows: [] },
      ]);
      const mem = new OracleMemory(pool);
      const result = await mem.getFirstPrice('NoPriceMint');

      assert.strictEqual(result, null);
    });
  });

  describe('getRecentMints()', () => {
    it('returns unique mints sorted by date', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        {
          rows: [
            { mint: 'A', name: 'Alpha', symbol: 'ALP', verdict: 'WAG', q_score: 55, k_score: 40, tier: 'Copper', judged_at: '2026-01-20' },
            { mint: 'B', name: 'Beta', symbol: 'BET', verdict: 'HOWL', q_score: 85, k_score: 70, tier: 'Gold', judged_at: '2026-01-15' },
          ],
        },
      ]);
      const mem = new OracleMemory(pool);
      const mints = await mem.getRecentMints(10);

      assert.strictEqual(mints.length, 2);
      assert.strictEqual(mints[0].mint, 'A');
      assert.strictEqual(mints[0].verdict, 'WAG');
    });
  });

  describe('getStats()', () => {
    it('returns aggregate statistics', async () => {
      const pool = createMockPool([
        { rows: [] },
        { rows: [] },
        {
          rows: [{
            total_judgments: '42',
            unique_tokens: '15',
            avg_q_score: '52.3',
            avg_k_score: '38.7',
            first_judgment: '2026-01-01',
            last_judgment: '2026-01-20',
          }],
        },
      ]);
      const mem = new OracleMemory(pool);
      const stats = await mem.getStats();

      assert.strictEqual(stats.totalJudgments, 42);
      assert.strictEqual(stats.uniqueTokens, 15);
      assert.strictEqual(stats.avgQScore, 52.3);
    });
  });
});
