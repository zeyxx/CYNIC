/**
 * E-Score History Repository
 *
 * Tracks E-Score evolution over time for trend analysis.
 * φ-aligned retention: hourly (24h), daily (30d), weekly (1y).
 *
 * @module @cynic/persistence/repositories/escore-history
 */

'use strict';

import { getPool } from '../client.js';

export class EScoreHistoryRepository {
  constructor(db = null) {
    this.db = db || getPool();
  }

  /**
   * Record an E-Score snapshot
   */
  async recordSnapshot(userId, eScore, breakdown, trigger = 'manual') {
    // Get previous score for delta
    const previous = await this.getLatest(userId);
    const delta = previous ? eScore - previous.e_score : 0;

    const { rows } = await this.db.query(`
      INSERT INTO escore_history (
        user_id, e_score,
        hold_score, burn_score, use_score, build_score,
        run_score, refer_score, time_score,
        trigger_event, delta
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
      RETURNING *
    `, [
      userId,
      eScore,
      breakdown?.hold || 0,
      breakdown?.burn || 0,
      breakdown?.use || 0,
      breakdown?.build || 0,
      breakdown?.run || 0,
      breakdown?.refer || 0,
      breakdown?.time || 0,
      trigger,
      delta,
    ]);

    return rows[0];
  }

  /**
   * Get latest E-Score snapshot for user
   */
  async getLatest(userId) {
    const { rows } = await this.db.query(`
      SELECT * FROM escore_history
      WHERE user_id = $1
      ORDER BY recorded_at DESC
      LIMIT 1
    `, [userId]);

    return rows[0] || null;
  }

  /**
   * Get E-Score history for user
   */
  async getHistory(userId, days = 7, limit = 100) {
    const { rows } = await this.db.query(`
      SELECT * FROM escore_history
      WHERE user_id = $1
        AND recorded_at >= NOW() - ($2 || ' days')::INTERVAL
      ORDER BY recorded_at DESC
      LIMIT $3
    `, [userId, days.toString(), limit]);

    return rows;
  }

  /**
   * Get E-Score trend for user
   */
  async getTrend(userId, days = 7) {
    const history = await this.getHistory(userId, days);

    if (history.length < 2) {
      return {
        direction: 'stable',
        velocity: 0,
        avgScore: history[0]?.e_score || 0,
        minScore: history[0]?.e_score || 0,
        maxScore: history[0]?.e_score || 0,
        dataPoints: history.length,
      };
    }

    const scores = history.map((h) => parseFloat(h.e_score));
    const first = scores[scores.length - 1]; // oldest
    const last = scores[0]; // newest
    const velocity = (last - first) / Math.max(days, 1);

    let direction = 'stable';
    if (velocity > 0.01) direction = 'up';
    else if (velocity < -0.01) direction = 'down';

    return {
      direction,
      velocity: Math.round(velocity * 10000) / 10000,
      avgScore: Math.round((scores.reduce((a, b) => a + b, 0) / scores.length) * 100) / 100,
      minScore: Math.min(...scores),
      maxScore: Math.max(...scores),
      dataPoints: scores.length,
    };
  }

  /**
   * Get dimension breakdown trends
   */
  async getDimensionTrends(userId, days = 7) {
    const history = await this.getHistory(userId, days);

    if (history.length < 2) {
      return { hasTrend: false };
    }

    const dimensions = ['hold', 'burn', 'use', 'build', 'run', 'refer', 'time'];
    const trends = {};

    for (const dim of dimensions) {
      const key = `${dim}_score`;
      const scores = history.map((h) => parseFloat(h[key] || 0));
      const first = scores[scores.length - 1];
      const last = scores[0];
      const velocity = (last - first) / Math.max(days, 1);

      let direction = 'stable';
      if (velocity > 0.005) direction = 'up';
      else if (velocity < -0.005) direction = 'down';

      trends[dim] = {
        direction,
        velocity: Math.round(velocity * 10000) / 10000,
        current: last,
        previous: first,
        change: Math.round((last - first) * 100) / 100,
      };
    }

    return {
      hasTrend: true,
      period: days,
      dimensions: trends,
    };
  }

  /**
   * Get aggregate statistics
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total_snapshots,
        COUNT(DISTINCT user_id) as unique_users,
        AVG(e_score) as avg_score,
        MAX(e_score) as max_score,
        AVG(ABS(delta)) as avg_change
      FROM escore_history
    `);

    const stats = rows[0];
    return {
      totalSnapshots: parseInt(stats.total_snapshots),
      uniqueUsers: parseInt(stats.unique_users),
      avgScore: parseFloat(stats.avg_score) || 0,
      maxScore: parseFloat(stats.max_score) || 0,
      avgChange: parseFloat(stats.avg_change) || 0,
    };
  }

  /**
   * Clean up old snapshots (retention policy)
   * Keep hourly for 24h, daily for 30d
   */
  async cleanup() {
    // Delete hourly snapshots older than 24h, keep only on-the-hour
    const hourlyResult = await this.db.query(`
      DELETE FROM escore_history
      WHERE recorded_at < NOW() - INTERVAL '24 hours'
        AND EXTRACT(MINUTE FROM recorded_at) != 0
    `);

    // Delete daily snapshots older than 30d, keep only start of day
    const dailyResult = await this.db.query(`
      DELETE FROM escore_history
      WHERE recorded_at < NOW() - INTERVAL '30 days'
        AND EXTRACT(HOUR FROM recorded_at) != 0
    `);

    return {
      hourlyDeleted: hourlyResult.rowCount,
      dailyDeleted: dailyResult.rowCount,
    };
  }
}

export default EScoreHistoryRepository;
