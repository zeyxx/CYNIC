/**
 * OracleWatchlist Tests
 *
 * Tests for autonomous token monitoring and alerting.
 * Uses mock pool, fetcher, scorer, and memory.
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert';

import { OracleWatchlist } from '../src/oracle/watchlist.js';

// ═══════════════════════════════════════════════════════════════════════════
// MOCKS
// ═══════════════════════════════════════════════════════════════════════════

function createMockPool(queryResponses = {}) {
  const calls = [];
  return {
    calls,
    query(sql, params) {
      calls.push({ sql: sql.trim(), params });
      // Match by first significant SQL keyword
      for (const [pattern, response] of Object.entries(queryResponses)) {
        if (sql.includes(pattern)) {
          if (response instanceof Error) return Promise.reject(response);
          return Promise.resolve(response);
        }
      }
      return Promise.resolve({ rows: [] });
    },
  };
}

function createMockMemory() {
  const stored = [];
  return {
    stored,
    async store(judgment) { stored.push(judgment); },
  };
}

function createMockFetcher() {
  return {
    async getTokenData(mint) {
      return {
        mint,
        name: 'MockToken',
        symbol: 'MOCK',
        isNative: false,
        supply: { total: 1000000 },
        distribution: { topHolders: [], holderCount: 100 },
        authorities: { mintAuthorityActive: false, freezeAuthorityActive: false },
        ageInDays: 30,
        priceInfo: { pricePerToken: 1.0 },
        metadataIntegrity: { hasName: true, hasSymbol: true, hasUri: true, hasImage: true },
      };
    },
  };
}

function createMockScorer() {
  return {
    score() {
      return {
        qScore: 55,
        kScore: 40,
        verdict: 'WAG',
        confidence: 0.5,
        dimensions: {},
        weaknesses: [],
      };
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('OracleWatchlist', () => {
  let watchlist;

  describe('ensureSchema()', () => {
    it('creates tables on first call', async () => {
      const pool = createMockPool({ 'CREATE TABLE': { rows: [] } });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());

      await wl.ensureSchema();
      assert.ok(pool.calls.length > 0);
      assert.ok(pool.calls[0].sql.includes('CREATE TABLE'));
    });

    it('skips on subsequent calls', async () => {
      const pool = createMockPool({ 'CREATE TABLE': { rows: [] } });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());

      await wl.ensureSchema();
      const callCount = pool.calls.length;
      await wl.ensureSchema();
      assert.strictEqual(pool.calls.length, callCount);
    });
  });

  describe('add()', () => {
    it('inserts or updates watchlist entry', async () => {
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'INSERT INTO oracle_watchlist': { rows: [] },
      });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());

      await wl.add('MintAAA', 'My Token');
      const insertCall = pool.calls.find(c => c.sql.includes('INSERT INTO oracle_watchlist'));
      assert.ok(insertCall);
      assert.deepStrictEqual(insertCall.params, ['MintAAA', 'My Token']);
    });

    it('works without label', async () => {
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'INSERT INTO oracle_watchlist': { rows: [] },
      });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());

      await wl.add('MintBBB');
      const insertCall = pool.calls.find(c => c.sql.includes('INSERT INTO oracle_watchlist'));
      assert.deepStrictEqual(insertCall.params, ['MintBBB', null]);
    });
  });

  describe('remove()', () => {
    it('deletes watchlist entry', async () => {
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'DELETE FROM oracle_watchlist': { rows: [] },
      });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());

      await wl.remove('MintCCC');
      const deleteCall = pool.calls.find(c => c.sql.includes('DELETE FROM'));
      assert.ok(deleteCall);
      assert.deepStrictEqual(deleteCall.params, ['MintCCC']);
    });
  });

  describe('list()', () => {
    it('returns formatted watchlist', async () => {
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'SELECT mint': {
          rows: [
            { mint: 'A', label: 'Alpha', last_verdict: 'WAG', last_q_score: 55, last_k_score: 40, added_at: '2026-01-01', last_checked_at: '2026-01-20' },
          ],
        },
      });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());
      const list = await wl.list();

      assert.strictEqual(list.length, 1);
      assert.strictEqual(list[0].mint, 'A');
      assert.strictEqual(list[0].label, 'Alpha');
      assert.strictEqual(list[0].lastVerdict, 'WAG');
    });
  });

  describe('getAlerts()', () => {
    it('returns formatted alerts', async () => {
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'FROM oracle_alerts': {
          rows: [{
            mint: 'A', label: 'Alpha', alert_type: 'verdict_change',
            old_verdict: 'GROWL', new_verdict: 'WAG',
            old_q_score: 35, new_q_score: 55,
            message: 'Alpha: GROWL → WAG', created_at: '2026-01-20',
          }],
        },
      });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());
      const alerts = await wl.getAlerts(10);

      assert.strictEqual(alerts.length, 1);
      assert.strictEqual(alerts[0].alertType, 'verdict_change');
      assert.strictEqual(alerts[0].oldVerdict, 'GROWL');
      assert.strictEqual(alerts[0].newVerdict, 'WAG');
    });
  });

  describe('checkAll()', () => {
    it('returns zero checked when watchlist empty', async () => {
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'SELECT mint, label': { rows: [] },
      });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());
      const result = await wl.checkAll();

      assert.strictEqual(result.checked, 0);
      assert.strictEqual(result.alerts, 0);
    });

    it('checks watched tokens and stores judgments', async () => {
      const memory = createMockMemory();
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'SELECT mint, label': {
          rows: [
            { mint: 'MintA', label: 'TokenA', last_verdict: null, last_q_score: null },
          ],
        },
        'UPDATE oracle_watchlist': { rows: [] },
      });
      const wl = new OracleWatchlist(pool, memory, createMockFetcher(), createMockScorer());
      const result = await wl.checkAll();

      assert.strictEqual(result.checked, 1);
      assert.strictEqual(memory.stored.length, 1);
    });

    it('generates alert on verdict change', async () => {
      const memory = createMockMemory();
      const scorer = {
        score() {
          return { qScore: 85, kScore: 70, verdict: 'HOWL', confidence: 0.6, dimensions: {}, weaknesses: [] };
        },
      };
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'SELECT mint, label': {
          rows: [
            { mint: 'MintA', label: 'TokenA', last_verdict: 'GROWL', last_q_score: 35 },
          ],
        },
        'INSERT INTO oracle_alerts': { rows: [] },
        'UPDATE oracle_watchlist': { rows: [] },
      });
      const wl = new OracleWatchlist(pool, memory, createMockFetcher(), scorer);
      const result = await wl.checkAll();

      assert.strictEqual(result.alerts, 1);
    });

    it('generates alert on large score change', async () => {
      const memory = createMockMemory();
      const scorer = {
        score() {
          return { qScore: 65, kScore: 50, verdict: 'WAG', confidence: 0.5, dimensions: {}, weaknesses: [] };
        },
      };
      const pool = createMockPool({
        'CREATE TABLE': { rows: [] },
        'SELECT mint, label': {
          rows: [
            { mint: 'MintA', label: 'TokenA', last_verdict: 'WAG', last_q_score: 50 },
          ],
        },
        'INSERT INTO oracle_alerts': { rows: [] },
        'UPDATE oracle_watchlist': { rows: [] },
      });
      const wl = new OracleWatchlist(pool, memory, createMockFetcher(), scorer);
      const result = await wl.checkAll();

      assert.strictEqual(result.alerts, 1);
    });
  });

  describe('monitoring', () => {
    it('startMonitoring sets interval', () => {
      const pool = createMockPool({ 'CREATE TABLE': { rows: [] } });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());

      wl.startMonitoring(60000);
      assert.ok(wl._interval !== null);
      wl.stopMonitoring();
    });

    it('stopMonitoring clears interval', () => {
      const pool = createMockPool({ 'CREATE TABLE': { rows: [] } });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());

      wl.startMonitoring(60000);
      wl.stopMonitoring();
      assert.strictEqual(wl._interval, null);
    });

    it('double start does not create duplicate intervals', () => {
      const pool = createMockPool({ 'CREATE TABLE': { rows: [] } });
      const wl = new OracleWatchlist(pool, createMockMemory(), createMockFetcher(), createMockScorer());

      wl.startMonitoring(60000);
      const first = wl._interval;
      wl.startMonitoring(60000);
      assert.strictEqual(wl._interval, first);
      wl.stopMonitoring();
    });
  });
});
