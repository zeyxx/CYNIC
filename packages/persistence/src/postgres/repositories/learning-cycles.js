/**
 * Learning Cycles Repository
 *
 * Records each RLHF learning cycle for analysis.
 * Tracks patterns updated, weights adjusted, and performance metrics.
 *
 * @module @cynic/persistence/repositories/learning-cycles
 */

'use strict';

import crypto from 'crypto';
import { getPool } from '../client.js';

/**
 * Generate short cycle ID
 */
function generateCycleId() {
  return 'lrn_' + crypto.randomBytes(8).toString('hex');
}

export class LearningCyclesRepository {
  constructor(db = null) {
    this.db = db || getPool();
  }

  /**
   * Record a learning cycle
   */
  async record(cycle) {
    const cycleId = cycle.cycleId || generateCycleId();

    const { rows } = await this.db.query(`
      INSERT INTO learning_cycles (
        cycle_id,
        feedback_processed,
        patterns_updated,
        patterns_merged,
        weights_adjusted,
        thresholds_adjusted,
        avg_weight_delta,
        avg_threshold_delta,
        duration_ms
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
      RETURNING *
    `, [
      cycleId,
      cycle.feedbackProcessed || 0,
      cycle.patternsUpdated || 0,
      cycle.patternsMerged || 0,
      cycle.weightsAdjusted || 0,
      cycle.thresholdsAdjusted || 0,
      cycle.avgWeightDelta || null,
      cycle.avgThresholdDelta || null,
      cycle.durationMs || null,
    ]);

    return rows[0];
  }

  /**
   * Get recent learning cycles
   */
  async getRecent(limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM learning_cycles
      ORDER BY completed_at DESC
      LIMIT $1
    `, [limit]);

    return rows;
  }

  /**
   * Get learning cycle by ID
   */
  async findById(cycleId) {
    const { rows } = await this.db.query(
      'SELECT * FROM learning_cycles WHERE cycle_id = $1',
      [cycleId]
    );
    return rows[0] || null;
  }

  /**
   * Get aggregate statistics
   */
  async getStats(days = 7) {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total_cycles,
        SUM(feedback_processed) as total_feedback,
        SUM(patterns_updated) as total_patterns_updated,
        SUM(patterns_merged) as total_patterns_merged,
        SUM(weights_adjusted) as total_weights_adjusted,
        SUM(thresholds_adjusted) as total_thresholds_adjusted,
        AVG(avg_weight_delta) as avg_weight_delta,
        AVG(avg_threshold_delta) as avg_threshold_delta,
        AVG(duration_ms) as avg_duration_ms
      FROM learning_cycles
      WHERE completed_at >= NOW() - ($1 || ' days')::INTERVAL
    `, [days.toString()]);

    const stats = rows[0];
    return {
      totalCycles: parseInt(stats.total_cycles) || 0,
      totalFeedback: parseInt(stats.total_feedback) || 0,
      totalPatternsUpdated: parseInt(stats.total_patterns_updated) || 0,
      totalPatternsMerged: parseInt(stats.total_patterns_merged) || 0,
      totalWeightsAdjusted: parseInt(stats.total_weights_adjusted) || 0,
      totalThresholdsAdjusted: parseInt(stats.total_thresholds_adjusted) || 0,
      avgWeightDelta: parseFloat(stats.avg_weight_delta) || 0,
      avgThresholdDelta: parseFloat(stats.avg_threshold_delta) || 0,
      avgDurationMs: parseFloat(stats.avg_duration_ms) || 0,
    };
  }

  /**
   * Get cycles per day for trend analysis
   */
  async getCyclesPerDay(days = 30) {
    const { rows } = await this.db.query(`
      SELECT
        DATE(completed_at) as day,
        COUNT(*) as cycles,
        SUM(feedback_processed) as feedback,
        SUM(patterns_updated) as patterns
      FROM learning_cycles
      WHERE completed_at >= NOW() - ($1 || ' days')::INTERVAL
      GROUP BY DATE(completed_at)
      ORDER BY day DESC
    `, [days.toString()]);

    return rows.map((r) => ({
      day: r.day,
      cycles: parseInt(r.cycles),
      feedback: parseInt(r.feedback) || 0,
      patterns: parseInt(r.patterns) || 0,
    }));
  }
}

export default LearningCyclesRepository;
