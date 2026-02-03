/**
 * Prometheus Exporter
 *
 * Export CYNIC metrics in Prometheus format.
 * "φ mesure tout" - Prometheus scrapes, CYNIC provides
 *
 * @module @cynic/observatory/exporters/prometheus
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Prometheus metrics exporter
 */
export class PrometheusExporter {
  /**
   * @param {Object} pool - PostgreSQL pool
   */
  constructor(pool) {
    this.pool = pool;
  }

  /**
   * Get metrics in Prometheus format
   * @returns {Promise<string>} Prometheus metrics
   */
  async getMetrics() {
    const lines = [];

    // Header
    lines.push('# CYNIC Observatory Metrics');
    lines.push('# Exported at ' + new Date().toISOString());
    lines.push('');

    // φ constants
    lines.push('# HELP cynic_phi_constant Golden ratio inverse constant');
    lines.push('# TYPE cynic_phi_constant gauge');
    lines.push(`cynic_phi_constant{name="phi_inv"} ${PHI_INV}`);
    lines.push(`cynic_phi_constant{name="phi_inv_2"} ${PHI_INV_2}`);
    lines.push('');

    try {
      // Q-Learning metrics
      const qstats = await this.pool.query(`
        SELECT
          service_id,
          exploration_rate,
          (stats->>'totalEpisodes')::int as total_episodes,
          (stats->>'totalReward')::float as total_reward
        FROM qlearning_state
      `);

      lines.push('# HELP cynic_qlearning_exploration_rate Current exploration rate');
      lines.push('# TYPE cynic_qlearning_exploration_rate gauge');
      for (const row of qstats.rows) {
        lines.push(`cynic_qlearning_exploration_rate{service="${row.service_id}"} ${row.exploration_rate}`);
      }
      lines.push('');

      lines.push('# HELP cynic_qlearning_episodes_total Total Q-learning episodes');
      lines.push('# TYPE cynic_qlearning_episodes_total counter');
      for (const row of qstats.rows) {
        lines.push(`cynic_qlearning_episodes_total{service="${row.service_id}"} ${row.total_episodes || 0}`);
      }
      lines.push('');

      lines.push('# HELP cynic_qlearning_reward_total Total accumulated reward');
      lines.push('# TYPE cynic_qlearning_reward_total counter');
      for (const row of qstats.rows) {
        lines.push(`cynic_qlearning_reward_total{service="${row.service_id}"} ${row.total_reward || 0}`);
      }
      lines.push('');

      // Pattern metrics
      const patterns = await this.pool.query(`
        SELECT
          count(*) as total,
          count(CASE WHEN fisher_importance >= $1 THEN 1 END) as locked,
          count(CASE WHEN fisher_importance >= $2 THEN 1 END) as important
        FROM patterns
      `, [PHI_INV, PHI_INV_2]);

      const p = patterns.rows[0] || {};
      lines.push('# HELP cynic_patterns_total Total patterns stored');
      lines.push('# TYPE cynic_patterns_total gauge');
      lines.push(`cynic_patterns_total ${p.total || 0}`);
      lines.push('');

      lines.push('# HELP cynic_patterns_locked Patterns locked by EWC++');
      lines.push('# TYPE cynic_patterns_locked gauge');
      lines.push(`cynic_patterns_locked ${p.locked || 0}`);
      lines.push('');

      // Tool metrics (last hour)
      const tools = await this.pool.query(`
        SELECT
          count(*) as total_calls,
          sum(CASE WHEN success THEN 1 ELSE 0 END) as successes,
          avg(latency_ms) as avg_duration
        FROM tool_usage
        WHERE created_at > NOW() - INTERVAL '1 hour'
      `);

      const t = tools.rows[0] || {};
      lines.push('# HELP cynic_tool_calls_hourly Tool calls in last hour');
      lines.push('# TYPE cynic_tool_calls_hourly gauge');
      lines.push(`cynic_tool_calls_hourly ${t.total_calls || 0}`);
      lines.push('');

      const successRate = t.total_calls > 0 ? (t.successes / t.total_calls) : 1;
      lines.push('# HELP cynic_success_rate_hourly Success rate in last hour');
      lines.push('# TYPE cynic_success_rate_hourly gauge');
      lines.push(`cynic_success_rate_hourly ${successRate.toFixed(4)}`);
      lines.push('');

      lines.push('# HELP cynic_avg_latency_ms_hourly Average latency in last hour');
      lines.push('# TYPE cynic_avg_latency_ms_hourly gauge');
      lines.push(`cynic_avg_latency_ms_hourly ${parseFloat(t.avg_duration || 0).toFixed(2)}`);
      lines.push('');

      // Friction metrics
      const frictions = await this.pool.query(`
        SELECT
          count(*) as total,
          count(CASE WHEN severity = 'critical' THEN 1 END) as critical
        FROM frictions
        WHERE created_at > NOW() - INTERVAL '1 hour'
      `);

      const f = frictions.rows[0] || {};
      lines.push('# HELP cynic_frictions_hourly Frictions in last hour');
      lines.push('# TYPE cynic_frictions_hourly gauge');
      lines.push(`cynic_frictions_hourly ${f.total || 0}`);
      lines.push('');

      lines.push('# HELP cynic_critical_frictions_hourly Critical frictions in last hour');
      lines.push('# TYPE cynic_critical_frictions_hourly gauge');
      lines.push(`cynic_critical_frictions_hourly ${f.critical || 0}`);

    } catch (e) {
      lines.push(`# Error fetching metrics: ${e.message}`);
    }

    return lines.join('\n');
  }
}
