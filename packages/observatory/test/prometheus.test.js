/**
 * PrometheusExporter Tests
 *
 * Tests for Prometheus metrics export.
 * Uses mock PostgreSQL pool.
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import { PrometheusExporter } from '../src/exporters/prometheus.js';

// ═══════════════════════════════════════════════════════════════════════════
// MOCK POOL
// ═══════════════════════════════════════════════════════════════════════════

function createMockPool(responses = {}) {
  return {
    query(sql) {
      for (const [pattern, response] of Object.entries(responses)) {
        if (sql.includes(pattern)) {
          if (response instanceof Error) return Promise.reject(response);
          return Promise.resolve(response);
        }
      }
      return Promise.resolve({ rows: [] });
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('PrometheusExporter', () => {
  it('includes header with timestamp', async () => {
    const pool = createMockPool({
      qlearning_state: { rows: [] },
      patterns: { rows: [{ total: 0, locked: 0, important: 0 }] },
      tool_usage: { rows: [{ total_calls: 0, successes: 0, avg_duration: 0 }] },
      frictions: { rows: [{ total: 0, critical: 0 }] },
    });
    const exporter = new PrometheusExporter(pool);
    const metrics = await exporter.getMetrics();

    assert.ok(metrics.includes('# CYNIC Observatory Metrics'));
    assert.ok(metrics.includes('# Exported at'));
  });

  it('exports φ constants', async () => {
    const pool = createMockPool({
      qlearning_state: { rows: [] },
      patterns: { rows: [{ total: 0, locked: 0, important: 0 }] },
      tool_usage: { rows: [{ total_calls: 0, successes: 0, avg_duration: 0 }] },
      frictions: { rows: [{ total: 0, critical: 0 }] },
    });
    const exporter = new PrometheusExporter(pool);
    const metrics = await exporter.getMetrics();

    assert.ok(metrics.includes('cynic_phi_constant{name="phi_inv"}'));
    assert.ok(metrics.includes('cynic_phi_constant{name="phi_inv_2"}'));
  });

  it('exports Q-learning metrics', async () => {
    const pool = createMockPool({
      qlearning_state: {
        rows: [{
          service_id: 'test-service',
          exploration_rate: 0.3,
          total_episodes: 150,
          total_reward: 45.5,
        }],
      },
      patterns: { rows: [{ total: 0, locked: 0, important: 0 }] },
      tool_usage: { rows: [{ total_calls: 0, successes: 0, avg_duration: 0 }] },
      frictions: { rows: [{ total: 0, critical: 0 }] },
    });
    const exporter = new PrometheusExporter(pool);
    const metrics = await exporter.getMetrics();

    assert.ok(metrics.includes('cynic_qlearning_exploration_rate{service="test-service"} 0.3'));
    assert.ok(metrics.includes('cynic_qlearning_episodes_total{service="test-service"} 150'));
    assert.ok(metrics.includes('cynic_qlearning_reward_total{service="test-service"} 45.5'));
  });

  it('exports pattern metrics', async () => {
    const pool = createMockPool({
      qlearning_state: { rows: [] },
      patterns: { rows: [{ total: 25, locked: 5, important: 10 }] },
      tool_usage: { rows: [{ total_calls: 0, successes: 0, avg_duration: 0 }] },
      frictions: { rows: [{ total: 0, critical: 0 }] },
    });
    const exporter = new PrometheusExporter(pool);
    const metrics = await exporter.getMetrics();

    assert.ok(metrics.includes('cynic_patterns_total 25'));
    assert.ok(metrics.includes('cynic_patterns_locked 5'));
  });

  it('exports tool metrics', async () => {
    const pool = createMockPool({
      qlearning_state: { rows: [] },
      patterns: { rows: [{ total: 0, locked: 0, important: 0 }] },
      tool_usage: { rows: [{ total_calls: 100, successes: 95, avg_duration: 250.5 }] },
      frictions: { rows: [{ total: 0, critical: 0 }] },
    });
    const exporter = new PrometheusExporter(pool);
    const metrics = await exporter.getMetrics();

    assert.ok(metrics.includes('cynic_tool_calls_hourly 100'));
    assert.ok(metrics.includes('cynic_success_rate_hourly 0.9500'));
    assert.ok(metrics.includes('cynic_avg_latency_ms_hourly 250.50'));
  });

  it('exports friction metrics', async () => {
    const pool = createMockPool({
      qlearning_state: { rows: [] },
      patterns: { rows: [{ total: 0, locked: 0, important: 0 }] },
      tool_usage: { rows: [{ total_calls: 10, successes: 10, avg_duration: 100 }] },
      frictions: { rows: [{ total: 8, critical: 2 }] },
    });
    const exporter = new PrometheusExporter(pool);
    const metrics = await exporter.getMetrics();

    assert.ok(metrics.includes('cynic_frictions_hourly 8'));
    assert.ok(metrics.includes('cynic_critical_frictions_hourly 2'));
  });

  it('handles DB errors gracefully', async () => {
    const pool = createMockPool({
      qlearning_state: new Error('connection refused'),
    });
    const exporter = new PrometheusExporter(pool);
    const metrics = await exporter.getMetrics();

    // Should include error line instead of crashing
    assert.ok(metrics.includes('# Error fetching metrics'));
  });

  it('outputs valid Prometheus format lines', async () => {
    const pool = createMockPool({
      qlearning_state: { rows: [] },
      patterns: { rows: [{ total: 0, locked: 0, important: 0 }] },
      tool_usage: { rows: [{ total_calls: 0, successes: 0, avg_duration: 0 }] },
      frictions: { rows: [{ total: 0, critical: 0 }] },
    });
    const exporter = new PrometheusExporter(pool);
    const metrics = await exporter.getMetrics();
    const lines = metrics.split('\n').filter(l => l && !l.startsWith('#'));

    // Each metric line should match pattern: metric_name{labels} value
    for (const line of lines) {
      // Should have at least a metric name and a numeric value
      assert.ok(line.match(/^[a-z_]+/), `Invalid metric line: ${line}`);
    }
  });
});
