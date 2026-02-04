/**
 * TelemetryQueries Tests
 *
 * Tests for usage metrics, frictions, and system health queries.
 * Uses mock PostgreSQL pool.
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import { TelemetryQueries } from '../src/queries/telemetry.js';

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

describe('TelemetryQueries', () => {
  describe('getToolUsage()', () => {
    it('returns usage data with defaults', async () => {
      const pool = createMockPool({
        rows: [
          { time_bucket: '2026-01-20T10:00:00Z', tool: 'brain_search', calls: 25, successes: 24, failures: 1, avg_duration_ms: 150 },
        ],
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getToolUsage();

      assert.strictEqual(result.usage.length, 1);
      assert.strictEqual(result.usage[0].tool, 'brain_search');
      assert.ok(result.timestamp);
    });

    it('sanitizes interval', async () => {
      const pool = createMockPool((sql, params) => {
        assert.strictEqual(params[0], 'hour');
        return Promise.resolve({ rows: [] });
      });
      const q = new TelemetryQueries(pool);
      await q.getToolUsage({ interval: 'malicious' });
    });
  });

  describe('getFrictions()', () => {
    it('returns friction data', async () => {
      const pool = createMockPool({
        rows: [{
          id: 1, friction_type: 'timeout', severity: 'high',
          tool: 'brain_search', error_type: 'TIMEOUT', message: 'Request timed out',
          metadata: {}, created_at: '2026-01-20',
        }],
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getFrictions();

      assert.strictEqual(result.count, 1);
      assert.strictEqual(result.frictions[0].friction_type, 'timeout');
    });

    it('filters by severity', async () => {
      const pool = createMockPool((sql, params) => {
        assert.ok(sql.includes('AND severity = $2'));
        assert.strictEqual(params[1], 'critical');
        return Promise.resolve({ rows: [] });
      });
      const q = new TelemetryQueries(pool);
      await q.getFrictions({ severity: 'critical' });
    });
  });

  describe('getFrictionHotspots()', () => {
    it('returns top friction sources', async () => {
      const pool = createMockPool({
        rows: [
          { tool: 'brain_search', error_type: 'TIMEOUT', friction_count: 15, max_severity: 'high', messages: ['timeout', 'slow'] },
        ],
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getFrictionHotspots();

      assert.strictEqual(result.hotspots.length, 1);
      assert.strictEqual(result.hotspots[0].friction_count, 15);
    });
  });

  describe('getSessionMetrics()', () => {
    it('returns session data', async () => {
      const pool = createMockPool({
        rows: [{
          session_id: 'sess1', user_id: 'user1',
          started_at: '2026-01-20T10:00:00Z', ended_at: '2026-01-20T11:00:00Z',
          duration_ms: 3600000, tool_calls: 50, errors: 2,
          tokens_used: 15000, patterns_detected: 5, judgments: 3,
        }],
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getSessionMetrics();

      assert.strictEqual(result.count, 1);
      assert.strictEqual(result.sessions[0].session_id, 'sess1');
    });
  });

  describe('getHealthMetrics()', () => {
    it('returns healthy status when success rate is high', async () => {
      const pool = createMockPool((sql) => {
        if (sql.includes('tool_usage')) {
          return Promise.resolve({ rows: [{ total_calls: 100, successes: 95, avg_latency_ms: 200 }] });
        }
        if (sql.includes('frictions')) {
          return Promise.resolve({ rows: [{ friction_count: 2, critical_count: 0 }] });
        }
        if (sql.includes('session_summary')) {
          return Promise.resolve({ rows: [{ active_sessions: 3, avg_session_duration: 1800000 }] });
        }
        return Promise.resolve({ rows: [] });
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getHealthMetrics();

      assert.strictEqual(result.status, 'healthy');
      assert.strictEqual(result.statusEmoji, '*tail wag*');
      assert.ok(result.successRate > 0.618);
      assert.strictEqual(result.toolCalls, 100);
    });

    it('returns degraded status when success rate is medium', async () => {
      const pool = createMockPool((sql) => {
        if (sql.includes('tool_usage')) {
          return Promise.resolve({ rows: [{ total_calls: 100, successes: 50, avg_latency_ms: 500 }] });
        }
        if (sql.includes('frictions')) {
          return Promise.resolve({ rows: [{ friction_count: 20, critical_count: 5 }] });
        }
        if (sql.includes('session_summary')) {
          return Promise.resolve({ rows: [{ active_sessions: 1, avg_session_duration: 600000 }] });
        }
        return Promise.resolve({ rows: [] });
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getHealthMetrics();

      assert.strictEqual(result.status, 'degraded');
      assert.strictEqual(result.statusEmoji, '*sniff*');
    });

    it('returns critical status when success rate is very low', async () => {
      const pool = createMockPool((sql) => {
        if (sql.includes('tool_usage')) {
          return Promise.resolve({ rows: [{ total_calls: 100, successes: 20, avg_latency_ms: 1000 }] });
        }
        if (sql.includes('frictions')) {
          return Promise.resolve({ rows: [{ friction_count: 50, critical_count: 20 }] });
        }
        if (sql.includes('session_summary')) {
          return Promise.resolve({ rows: [{ active_sessions: 0, avg_session_duration: 0 }] });
        }
        return Promise.resolve({ rows: [] });
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getHealthMetrics();

      assert.strictEqual(result.status, 'critical');
      assert.strictEqual(result.statusEmoji, '*GROWL*');
    });

    it('returns healthy when no tool calls', async () => {
      const pool = createMockPool((sql) => {
        if (sql.includes('tool_usage')) {
          return Promise.resolve({ rows: [{ total_calls: 0, successes: 0, avg_latency_ms: 0 }] });
        }
        if (sql.includes('frictions')) {
          return Promise.resolve({ rows: [{ friction_count: 0, critical_count: 0 }] });
        }
        if (sql.includes('session_summary')) {
          return Promise.resolve({ rows: [{ active_sessions: 0, avg_session_duration: 0 }] });
        }
        return Promise.resolve({ rows: [] });
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getHealthMetrics();

      // successRate defaults to 1 when no calls
      assert.strictEqual(result.status, 'healthy');
    });

    it('includes φ thresholds in response', async () => {
      const pool = createMockPool(() => Promise.resolve({ rows: [{ total_calls: 0, successes: 0, avg_latency_ms: 0, friction_count: 0, critical_count: 0, active_sessions: 0, avg_session_duration: 0 }] }));
      const q = new TelemetryQueries(pool);
      const result = await q.getHealthMetrics();

      assert.ok(result.thresholds.phi > 0.6);
      assert.ok(result.thresholds.phi2 > 0.3);
    });
  });

  describe('getLLMMetrics()', () => {
    it('returns LLM usage data', async () => {
      const pool = createMockPool({
        rows: [{
          time_bucket: '2026-01-20T10:00:00Z', provider: 'claude-opus',
          calls: 15, total_input_tokens: 50000, total_output_tokens: 20000,
          avg_latency_ms: 3500, total_cost_usd: 0,
        }],
      });
      const q = new TelemetryQueries(pool);
      const result = await q.getLLMMetrics();

      assert.strictEqual(result.metrics.length, 1);
      assert.strictEqual(result.metrics[0].provider, 'claude-opus');
    });

    it('sanitizes interval', async () => {
      const pool = createMockPool((sql, params) => {
        assert.strictEqual(params[0], 'hour');
        return Promise.resolve({ rows: [] });
      });
      const q = new TelemetryQueries(pool);
      await q.getLLMMetrics('invalid');
    });
  });
});
