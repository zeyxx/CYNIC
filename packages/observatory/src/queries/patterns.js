/**
 * Patterns Queries
 *
 * Queries for pattern detection and EWC++ Fisher scores.
 * "Le chien se souvient des patterns importants"
 *
 * @module @cynic/observatory/queries/patterns
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Patterns observatory queries
 */
export class PatternsQueries {
  /**
   * @param {Object} pool - PostgreSQL pool from @cynic/persistence
   */
  constructor(pool) {
    this.pool = pool;
  }

  /**
   * Get recent patterns
   * @param {Object} options - Query options
   * @param {number} [options.limit=50] - Max patterns
   * @param {string} [options.category] - Filter by category
   * @returns {Promise<Object>} Recent patterns
   */
  async getRecentPatterns({ limit = 50, category } = {}) {
    const params = [limit];
    let whereClause = '';

    if (category) {
      whereClause = 'WHERE category = $2';
      params.push(category);
    }

    const result = await this.pool.query(`
      SELECT
        id,
        pattern_id,
        category as type,
        name as signature,
        description,
        confidence,
        frequency as occurrences,
        fisher_importance,
        data as metadata,
        created_at,
        updated_at
      FROM patterns
      ${whereClause}
      ORDER BY updated_at DESC
      LIMIT $1
    `, params);

    return {
      patterns: result.rows,
      count: result.rows.length,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get patterns by Fisher importance (EWC++)
   * High Fisher = important patterns that should not be forgotten
   * @param {number} [limit=20] - Max patterns
   * @returns {Promise<Object>} Important patterns
   */
  async getImportantPatterns(limit = 20) {
    const result = await this.pool.query(`
      SELECT
        id,
        pattern_id,
        category as type,
        name as signature,
        description,
        confidence,
        frequency as occurrences,
        fisher_importance,
        CASE
          WHEN fisher_importance >= $2 THEN 'locked'
          WHEN fisher_importance >= $3 THEN 'important'
          ELSE 'normal'
        END as ewc_status,
        data as metadata,
        created_at
      FROM patterns
      WHERE fisher_importance > 0
      ORDER BY fisher_importance DESC
      LIMIT $1
    `, [limit, PHI_INV, PHI_INV_2]);

    return {
      patterns: result.rows,
      lockedCount: result.rows.filter(p => p.ewc_status === 'locked').length,
      importantCount: result.rows.filter(p => p.ewc_status === 'important').length,
      ewcThreshold: PHI_INV,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get pattern category distribution
   * @returns {Promise<Object>} Pattern categories
   */
  async getPatternDistribution() {
    const result = await this.pool.query(`
      SELECT
        category as type,
        count(*) as count,
        avg(confidence) as avg_confidence,
        avg(fisher_importance) as avg_fisher,
        sum(frequency) as total_occurrences
      FROM patterns
      GROUP BY category
      ORDER BY count DESC
    `);

    return {
      distribution: result.rows,
      totalTypes: result.rows.length,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get pattern timeline (when patterns are detected)
   * @param {string} [interval='hour'] - Time bucket
   * @param {number} [limit=168] - Max data points
   * @returns {Promise<Object>} Pattern timeline
   */
  async getPatternTimeline({ interval = 'hour', limit = 168 } = {}) {
    const validIntervals = ['hour', 'day', 'week'];
    const safeInterval = validIntervals.includes(interval) ? interval : 'hour';

    const result = await this.pool.query(`
      SELECT
        date_trunc($1, created_at) as time_bucket,
        count(*) as new_patterns,
        avg(confidence) as avg_confidence,
        count(DISTINCT category) as unique_types
      FROM patterns
      WHERE created_at > NOW() - INTERVAL '7 days'
      GROUP BY 1
      ORDER BY 1 DESC
      LIMIT $2
    `, [safeInterval, limit]);

    return {
      timeline: result.rows.reverse(),
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get anomaly patterns
   * @param {number} [limit=20] - Max anomalies
   * @returns {Promise<Object>} Anomaly patterns
   */
  async getAnomalies(limit = 20) {
    const result = await this.pool.query(`
      SELECT
        id,
        pattern_id,
        category as type,
        name as signature,
        description,
        confidence,
        data as metadata,
        created_at
      FROM patterns
      WHERE category LIKE '%anomaly%'
         OR category LIKE '%error%'
         OR category LIKE '%warning%'
      ORDER BY created_at DESC
      LIMIT $1
    `, [limit]);

    return {
      anomalies: result.rows,
      count: result.rows.length,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get pattern co-occurrence matrix
   * Which patterns appear together?
   * @param {number} [limit=10] - Max patterns to analyze
   * @returns {Promise<Object>} Co-occurrence data
   */
  async getPatternCoOccurrence(limit = 10) {
    // Get patterns that occur in similar time windows
    const result = await this.pool.query(`
      WITH pattern_windows AS (
        SELECT
          id,
          category,
          name,
          date_trunc('hour', created_at) as window
        FROM patterns
        WHERE created_at > NOW() - INTERVAL '7 days'
      )
      SELECT
        p1.category as type1,
        p2.category as type2,
        count(*) as co_occurrences
      FROM pattern_windows p1
      JOIN pattern_windows p2
        ON p1.window = p2.window
        AND p1.category < p2.category
      GROUP BY p1.category, p2.category
      HAVING count(*) >= 2
      ORDER BY co_occurrences DESC
      LIMIT $1
    `, [limit * 3]);

    return {
      coOccurrences: result.rows,
      timestamp: new Date().toISOString(),
    };
  }
}
