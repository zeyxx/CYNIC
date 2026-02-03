/**
 * Learning Proof Queries
 *
 * Queries that PROVE CYNIC is learning - not just collecting data.
 * "Le chien apprend, pas juste mémorise"
 *
 * These queries answer the fundamental question:
 * "Is CYNIC actually getting better over time?"
 *
 * @module @cynic/observatory/queries/learning-proof
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Learning proof queries
 */
export class LearningProofQueries {
  /**
   * @param {Object} pool - PostgreSQL pool from @cynic/persistence
   */
  constructor(pool) {
    this.pool = pool;
  }

  /**
   * Get comprehensive learning proof
   * This is THE query that proves learning is working.
   * @returns {Promise<Object>} Learning proof
   */
  async getLearningProof() {
    // Run all proof queries in parallel
    const [
      rewardTrend,
      explorationDecay,
      qValueConvergence,
      patternRetention,
      errorReduction,
    ] = await Promise.all([
      this._getRewardTrend(),
      this._getExplorationDecay(),
      this._getQValueConvergence(),
      this._getPatternRetention(),
      this._getErrorReduction(),
    ]);

    // Calculate overall learning score
    const scores = [
      rewardTrend.score,
      explorationDecay.score,
      qValueConvergence.score,
      patternRetention.score,
      errorReduction.score,
    ].filter(s => s !== null);

    const overallScore = scores.length > 0
      ? scores.reduce((a, b) => a + b, 0) / scores.length
      : null;

    // Determine verdict
    let verdict = 'unknown';
    let interpretation = '*head tilt* Not enough data to determine learning status';

    if (overallScore !== null) {
      if (overallScore >= PHI_INV) {
        verdict = 'learning';
        interpretation = '*tail wag* CYNIC IS LEARNING - metrics show improvement';
      } else if (overallScore >= PHI_INV_2) {
        verdict = 'stable';
        interpretation = '*sniff* Learning is stable but not improving';
      } else {
        verdict = 'not_learning';
        interpretation = '*GROWL* WARNING: Learning may be broken';
      }
    }

    return {
      verdict,
      overallScore,
      interpretation,
      proofs: {
        rewardTrend,
        explorationDecay,
        qValueConvergence,
        patternRetention,
        errorReduction,
      },
      thresholds: {
        learning: PHI_INV,
        stable: PHI_INV_2,
      },
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Proof 1: Rewards are increasing over time
   * If learning works, later episodes should have higher rewards.
   * @private
   */
  async _getRewardTrend() {
    try {
      const result = await this.pool.query(`
        WITH weekly_rewards AS (
          SELECT
            date_trunc('day', created_at) as day,
            avg(reward) as avg_reward,
            count(*) as episode_count
          FROM qlearning_episodes
          WHERE created_at > NOW() - INTERVAL '14 days'
          GROUP BY 1
          HAVING count(*) >= 3
          ORDER BY 1
        ),
        trend AS (
          SELECT
            regr_slope(avg_reward, EXTRACT(epoch FROM day)) as slope,
            corr(avg_reward, EXTRACT(epoch FROM day)) as correlation,
            count(*) as data_points
          FROM weekly_rewards
        )
        SELECT * FROM trend
      `);

      const row = result.rows[0] || {};
      const slope = parseFloat(row.slope || 0);
      const correlation = parseFloat(row.correlation || 0);
      const dataPoints = parseInt(row.data_points || 0);

      // Score: positive slope = good, negative = bad
      let score = null;
      if (dataPoints >= 5) {
        score = correlation > 0 ? Math.min(correlation + 0.5, 1) : Math.max(correlation + 0.5, 0);
      }

      return {
        name: 'Reward Trend',
        description: 'Are rewards increasing over time?',
        slope,
        correlation,
        dataPoints,
        score,
        interpretation: score === null
          ? 'Not enough data'
          : slope > 0
          ? 'Rewards trending UP - good!'
          : 'Rewards trending DOWN - investigate',
      };
    } catch (e) {
      return { name: 'Reward Trend', error: e.message, score: null };
    }
  }

  /**
   * Proof 2: Exploration rate is decaying
   * As system learns, it should explore less (exploit more).
   * @private
   */
  async _getExplorationDecay() {
    try {
      const result = await this.pool.query(`
        SELECT
          exploration_rate,
          (stats->>'totalEpisodes')::int as total_episodes,
          updated_at
        FROM qlearning_state
        ORDER BY updated_at DESC
        LIMIT 1
      `);

      const row = result.rows[0] || {};
      const explorationRate = parseFloat(row.exploration_rate || 1);
      const totalEpisodes = parseInt(row.total_episodes || 0);

      // Score: lower exploration (with many episodes) = good
      let score = null;
      if (totalEpisodes > 50) {
        score = 1 - explorationRate; // Lower exploration = higher score
      }

      return {
        name: 'Exploration Decay',
        description: 'Is exploration rate decreasing as system learns?',
        explorationRate,
        totalEpisodes,
        score,
        interpretation: score === null
          ? 'Not enough episodes'
          : explorationRate < 0.3
          ? 'System is exploiting learned knowledge'
          : 'System is still exploring heavily',
      };
    } catch (e) {
      return { name: 'Exploration Decay', error: e.message, score: null };
    }
  }

  /**
   * Proof 3: Q-values are converging
   * Stable Q-values indicate the system has learned.
   * @private
   */
  async _getQValueConvergence() {
    try {
      const result = await this.pool.query(`
        SELECT
          q_table,
          version
        FROM qlearning_state
        ORDER BY updated_at DESC
        LIMIT 1
      `);

      const row = result.rows[0] || {};
      const qTable = row.q_table || { entries: [] };
      const entries = qTable.entries || [];

      // Calculate Q-value variance
      const allQValues = entries.flatMap(e =>
        Object.values(e.values || {}).filter(v => typeof v === 'number')
      );

      if (allQValues.length < 10) {
        return {
          name: 'Q-Value Convergence',
          description: 'Are Q-values stabilizing?',
          score: null,
          interpretation: 'Not enough Q-values to analyze',
        };
      }

      const mean = allQValues.reduce((a, b) => a + b, 0) / allQValues.length;
      const variance = allQValues.reduce((s, v) => s + Math.pow(v - mean, 2), 0) / allQValues.length;
      const stddev = Math.sqrt(variance);
      const cv = mean !== 0 ? stddev / Math.abs(mean) : Infinity; // Coefficient of variation

      // Score: lower CV = more converged
      const score = cv < 2 ? Math.max(1 - cv / 2, 0) : 0;

      return {
        name: 'Q-Value Convergence',
        description: 'Are Q-values stabilizing?',
        mean,
        stddev,
        coefficientOfVariation: cv,
        entryCount: entries.length,
        qValueCount: allQValues.length,
        score,
        interpretation: score > PHI_INV
          ? 'Q-values are converging - learning stabilized'
          : 'Q-values are volatile - still learning or unstable',
      };
    } catch (e) {
      return { name: 'Q-Value Convergence', error: e.message, score: null };
    }
  }

  /**
   * Proof 4: Important patterns are retained (EWC++)
   * High-Fisher patterns should persist over time.
   * @private
   */
  async _getPatternRetention() {
    try {
      const result = await this.pool.query(`
        SELECT
          count(*) as total_patterns,
          count(CASE WHEN fisher_importance >= $1 THEN 1 END) as locked_patterns,
          count(CASE WHEN fisher_importance >= $2 THEN 1 END) as important_patterns,
          avg(fisher_importance) as avg_fisher,
          max(fisher_importance) as max_fisher
        FROM patterns
      `, [PHI_INV, PHI_INV_2]);

      const row = result.rows[0] || {};
      const totalPatterns = parseInt(row.total_patterns || 0);
      const lockedPatterns = parseInt(row.locked_patterns || 0);
      const importantPatterns = parseInt(row.important_patterns || 0);

      // Score: having locked patterns = good
      let score = null;
      if (totalPatterns > 10) {
        score = Math.min(lockedPatterns / 5, 1); // 5+ locked = perfect score
      }

      return {
        name: 'Pattern Retention (EWC++)',
        description: 'Are important patterns being preserved?',
        totalPatterns,
        lockedPatterns,
        importantPatterns,
        avgFisher: parseFloat(row.avg_fisher || 0),
        maxFisher: parseFloat(row.max_fisher || 0),
        score,
        interpretation: lockedPatterns > 0
          ? `${lockedPatterns} patterns locked - catastrophic forgetting prevented`
          : 'No locked patterns yet - EWC++ not active or too few patterns',
      };
    } catch (e) {
      return { name: 'Pattern Retention', error: e.message, score: null };
    }
  }

  /**
   * Proof 5: Errors are reducing over time
   * Fewer errors = system learned from mistakes.
   * @private
   */
  async _getErrorReduction() {
    try {
      const result = await this.pool.query(`
        WITH weekly_errors AS (
          SELECT
            date_trunc('day', created_at) as day,
            count(*) as error_count
          FROM frictions
          WHERE severity IN ('high', 'critical')
            AND created_at > NOW() - INTERVAL '14 days'
          GROUP BY 1
          ORDER BY 1
        ),
        trend AS (
          SELECT
            regr_slope(error_count, EXTRACT(epoch FROM day)) as slope,
            corr(error_count, EXTRACT(epoch FROM day)) as correlation,
            count(*) as data_points,
            avg(error_count) as avg_errors
          FROM weekly_errors
        )
        SELECT * FROM trend
      `);

      const row = result.rows[0] || {};
      const slope = parseFloat(row.slope || 0);
      const correlation = parseFloat(row.correlation || 0);
      const dataPoints = parseInt(row.data_points || 0);
      const avgErrors = parseFloat(row.avg_errors || 0);

      // Score: negative slope (fewer errors) = good
      let score = null;
      if (dataPoints >= 5) {
        score = slope < 0 ? Math.min(Math.abs(correlation) + 0.5, 1) : Math.max(0.5 - correlation, 0);
      }

      return {
        name: 'Error Reduction',
        description: 'Are errors decreasing over time?',
        slope,
        correlation,
        dataPoints,
        avgErrorsPerDay: avgErrors,
        score,
        interpretation: score === null
          ? 'Not enough data'
          : slope < 0
          ? 'Errors trending DOWN - learning from mistakes!'
          : 'Errors not decreasing - may need attention',
      };
    } catch (e) {
      return { name: 'Error Reduction', error: e.message, score: null };
    }
  }

  /**
   * Get learning timeline visualization data
   * @param {number} [days=30] - Days to analyze
   * @returns {Promise<Object>} Timeline data
   */
  async getLearningTimeline(days = 30) {
    const result = await this.pool.query(`
      SELECT
        date_trunc('day', created_at) as day,
        -- Q-Learning
        (SELECT count(*) FROM qlearning_episodes WHERE date_trunc('day', created_at) = date_trunc('day', t.created_at)) as episodes,
        (SELECT avg(reward) FROM qlearning_episodes WHERE date_trunc('day', created_at) = date_trunc('day', t.created_at)) as avg_reward,
        -- Patterns
        (SELECT count(*) FROM patterns WHERE date_trunc('day', created_at) = date_trunc('day', t.created_at)) as new_patterns,
        -- Errors
        (SELECT count(*) FROM frictions WHERE severity IN ('high', 'critical') AND date_trunc('day', created_at) = date_trunc('day', t.created_at)) as errors
      FROM generate_series(
        NOW() - INTERVAL '${days} days',
        NOW(),
        INTERVAL '1 day'
      ) as t(created_at)
      ORDER BY day
    `);

    return {
      timeline: result.rows,
      days,
      timestamp: new Date().toISOString(),
    };
  }
}
