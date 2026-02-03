/**
 * Telemetry Queries
 *
 * Queries for usage metrics, frictions, and system health.
 * "φ mesure tout"
 *
 * @module @cynic/observatory/queries/telemetry
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Telemetry observatory queries
 */
export class TelemetryQueries {
  /**
   * @param {Object} pool - PostgreSQL pool from @cynic/persistence
   */
  constructor(pool) {
    this.pool = pool;
  }

  /**
   * Get tool usage stats
   * @param {Object} options - Query options
   * @param {string} [options.interval='hour'] - Time bucket
   * @param {number} [options.limit=168] - Max data points
   * @returns {Promise<Object>} Tool usage stats
   */
  async getToolUsage({ interval = 'hour', limit = 168 } = {}) {
    const validIntervals = ['hour', 'day', 'week'];
    const safeInterval = validIntervals.includes(interval) ? interval : 'hour';

    const result = await this.pool.query(`
      SELECT
        date_trunc($1, created_at) as time_bucket,
        tool_name as tool,
        count(*) as calls,
        sum(CASE WHEN success THEN 1 ELSE 0 END) as successes,
        sum(CASE WHEN NOT success THEN 1 ELSE 0 END) as failures,
        avg(latency_ms) as avg_duration_ms
      FROM tool_usage
      WHERE created_at > NOW() - INTERVAL '7 days'
      GROUP BY 1, 2
      ORDER BY 1 DESC, calls DESC
      LIMIT $2
    `, [safeInterval, limit * 10]);

    return {
      usage: result.rows,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get friction points (errors, retries, timeouts)
   * @param {Object} options - Query options
   * @param {string} [options.severity] - Filter by severity
   * @param {number} [options.limit=50] - Max frictions
   * @returns {Promise<Object>} Friction points
   */
  async getFrictions({ severity, limit = 50 } = {}) {
    const params = [limit];
    let whereClause = "WHERE created_at > NOW() - INTERVAL '7 days'";

    if (severity) {
      whereClause += ' AND severity = $2';
      params.push(severity);
    }

    const result = await this.pool.query(`
      SELECT
        id,
        name as friction_type,
        severity,
        details->>'tool' as tool,
        details->>'errorType' as error_type,
        details->>'error' as message,
        details as metadata,
        created_at
      FROM frictions
      ${whereClause}
      ORDER BY created_at DESC
      LIMIT $1
    `, params);

    return {
      frictions: result.rows,
      count: result.rows.length,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get friction hotspots (most problematic areas)
   * @param {number} [limit=10] - Max hotspots
   * @returns {Promise<Object>} Friction hotspots
   */
  async getFrictionHotspots(limit = 10) {
    const result = await this.pool.query(`
      SELECT
        details->>'tool' as tool,
        details->>'errorType' as error_type,
        count(*) as friction_count,
        max(severity) as max_severity,
        array_agg(DISTINCT details->>'error') as messages
      FROM frictions
      WHERE created_at > NOW() - INTERVAL '7 days'
      GROUP BY details->>'tool', details->>'errorType'
      ORDER BY friction_count DESC
      LIMIT $1
    `, [limit]);

    return {
      hotspots: result.rows,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get session metrics
   * @param {number} [limit=20] - Max sessions
   * @returns {Promise<Object>} Session metrics
   */
  async getSessionMetrics(limit = 20) {
    const result = await this.pool.query(`
      SELECT
        session_id,
        (metadata->>'user_id') as user_id,
        start_time as started_at,
        end_time as ended_at,
        duration_ms,
        action_count as tool_calls,
        error_count as errors,
        (llm_tokens_in + llm_tokens_out) as tokens_used,
        patterns_detected,
        judgments
      FROM session_summary
      ORDER BY start_time DESC
      LIMIT $1
    `, [limit]);

    return {
      sessions: result.rows,
      count: result.rows.length,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get system health metrics
   * @returns {Promise<Object>} Health metrics
   */
  async getHealthMetrics() {
    // Multiple queries in parallel
    const [toolsResult, frictionsResult, sessionsResult] = await Promise.all([
      this.pool.query(`
        SELECT
          count(*) as total_calls,
          sum(CASE WHEN success THEN 1 ELSE 0 END) as successes,
          avg(latency_ms) as avg_latency_ms
        FROM tool_usage
        WHERE created_at > NOW() - INTERVAL '1 hour'
      `),
      this.pool.query(`
        SELECT
          count(*) as friction_count,
          count(CASE WHEN severity = 'critical' THEN 1 END) as critical_count
        FROM frictions
        WHERE created_at > NOW() - INTERVAL '1 hour'
      `),
      this.pool.query(`
        SELECT
          count(*) as active_sessions,
          avg(duration_ms) as avg_session_duration
        FROM session_summary
        WHERE end_time IS NULL OR end_time > NOW() - INTERVAL '1 hour'
      `),
    ]);

    const tools = toolsResult.rows[0] || {};
    const frictions = frictionsResult.rows[0] || {};
    const sessions = sessionsResult.rows[0] || {};

    const successRate = tools.total_calls > 0
      ? parseFloat(tools.successes) / parseFloat(tools.total_calls)
      : 1;

    // Health status based on φ thresholds
    let status = 'healthy';
    let statusEmoji = '*tail wag*';
    if (successRate < PHI_INV_2) {
      status = 'critical';
      statusEmoji = '*GROWL*';
    } else if (successRate < PHI_INV) {
      status = 'degraded';
      statusEmoji = '*sniff*';
    }

    return {
      status,
      statusEmoji,
      successRate,
      toolCalls: parseInt(tools.total_calls || 0),
      avgLatencyMs: parseFloat(tools.avg_latency_ms || 0),
      frictionCount: parseInt(frictions.friction_count || 0),
      criticalFrictions: parseInt(frictions.critical_count || 0),
      activeSessions: parseInt(sessions.active_sessions || 0),
      avgSessionDurationMs: parseFloat(sessions.avg_session_duration || 0),
      thresholds: {
        phi: PHI_INV,
        phi2: PHI_INV_2,
      },
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get LLM usage metrics
   * @param {string} [interval='hour'] - Time bucket
   * @returns {Promise<Object>} LLM metrics
   */
  async getLLMMetrics(interval = 'hour') {
    const validIntervals = ['hour', 'day', 'week'];
    const safeInterval = validIntervals.includes(interval) ? interval : 'hour';

    const result = await this.pool.query(`
      SELECT
        date_trunc($1, created_at) as time_bucket,
        model as provider,
        count(*) as calls,
        sum(input_tokens) as total_input_tokens,
        sum(output_tokens) as total_output_tokens,
        avg(latency_ms) as avg_latency_ms,
        0 as total_cost_usd
      FROM llm_usage
      WHERE created_at > NOW() - INTERVAL '7 days'
      GROUP BY 1, 2
      ORDER BY 1 DESC
    `, [safeInterval]);

    return {
      metrics: result.rows,
      timestamp: new Date().toISOString(),
    };
  }
}
