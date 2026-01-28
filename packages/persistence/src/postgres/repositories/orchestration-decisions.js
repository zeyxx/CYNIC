/**
 * OrchestrationDecisionRepository - Persists orchestration decisions
 *
 * Stores full DecisionEvent data for:
 * - Learning from routing outcomes
 * - Analyzing which domains work best
 * - Correlating judgments with skills
 *
 * "φ traces every step" - κυνικός
 *
 * @module @cynic/persistence/postgres/repositories/orchestration-decisions
 */

'use strict';

import { createLogger } from '@cynic/core';
import crypto from 'crypto';

const log = createLogger('OrchestrationDecisionRepository');

/**
 * Repository for orchestration decision persistence
 */
export class OrchestrationDecisionRepository {
  /**
   * Create repository
   * @param {Object} pool - PostgreSQL pool
   */
  constructor(pool) {
    this.pool = pool;
  }

  /**
   * Record a decision event
   *
   * @param {Object} decision - Decision data
   * @param {string} decision.id - Decision ID
   * @param {string} decision.eventType - Event type
   * @param {string} decision.userId - User ID
   * @param {string} decision.outcome - Outcome (ALLOW, BLOCK, MODIFIED, ERROR)
   * @param {Object} decision.routing - Routing info (sefirah, domain, suggestedAgent)
   * @param {Object} decision.intervention - Intervention info (level, actionRisk)
   * @param {Object} [decision.judgment] - Judgment data if available
   * @param {string} [decision.skillInvoked] - Skill that was invoked
   * @param {boolean} [decision.skillSuccess] - Whether skill succeeded
   * @param {Array} [decision.trace] - Decision trace array
   * @param {string} [decision.contentHash] - Hash of content for correlation
   * @param {number} [decision.durationMs] - Processing duration
   * @param {string} [decision.sessionId] - Session ID
   * @returns {Promise<Object>} Created record
   */
  async record(decision) {
    const {
      id,
      eventType,
      userId,
      outcome,
      routing = {},
      intervention = {},
      judgment,
      skillInvoked,
      skillSuccess,
      trace = [],
      contentHash,
      durationMs,
      sessionId,
    } = decision;

    const query = `
      INSERT INTO orchestration_log (
        id, event_type, user_id, outcome,
        sefirah, domain, suggested_agent, intervention, risk_level,
        judgment_id, judgment_qscore, judgment_verdict,
        skill_invoked, skill_success,
        trace, content_hash, duration_ms, session_id,
        created_at
      ) VALUES (
        COALESCE($1, gen_random_uuid()), $2, $3, $4,
        $5, $6, $7, $8, $9,
        $10, $11, $12,
        $13, $14,
        $15, $16, $17, $18,
        NOW()
      )
      RETURNING *
    `;

    const values = [
      id,
      eventType,
      userId,
      outcome || 'ALLOW',
      routing.sefirah || 'Keter',
      routing.domain,
      routing.suggestedAgent,
      intervention.level,
      intervention.actionRisk || 'low',
      judgment?.id,
      judgment?.qScore,
      judgment?.verdict,
      skillInvoked,
      skillSuccess,
      JSON.stringify(trace),
      contentHash,
      durationMs,
      sessionId,
    ];

    const result = await this.pool.query(query, values);
    log.debug('Decision recorded', { id: result.rows[0].id, outcome });
    return result.rows[0];
  }

  /**
   * Record from a DecisionEvent object
   *
   * @param {Object} event - DecisionEvent instance
   * @returns {Promise<Object>} Created record
   */
  async recordEvent(event) {
    // Hash content if present (don't store actual content)
    let contentHash = null;
    if (event.content) {
      contentHash = crypto.createHash('sha256')
        .update(event.content.slice(0, 1000))
        .digest('hex')
        .slice(0, 16);
    }

    return this.record({
      id: event.id,
      eventType: event.eventType,
      userId: event.userContext?.userId,
      outcome: event.outcome,
      routing: event.routing,
      intervention: event.intervention,
      judgment: event.judgment,
      skillInvoked: event.skillResult?.skill,
      skillSuccess: event.skillResult?.success,
      trace: event.trace,
      contentHash,
      durationMs: event.timestamp ? Date.now() - event.timestamp : null,
      sessionId: event.userContext?.sessionId,
    });
  }

  /**
   * Get recent decisions
   *
   * @param {number} [limit=50] - Max results
   * @returns {Promise<Array>} Recent decisions
   */
  async getRecent(limit = 50) {
    const result = await this.pool.query(
      `SELECT * FROM orchestration_log ORDER BY created_at DESC LIMIT $1`,
      [limit]
    );
    return result.rows;
  }

  /**
   * Get decisions by user
   *
   * @param {string} userId - User ID
   * @param {number} [limit=50] - Max results
   * @returns {Promise<Array>} User decisions
   */
  async getByUser(userId, limit = 50) {
    const result = await this.pool.query(
      `SELECT * FROM orchestration_log WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2`,
      [userId, limit]
    );
    return result.rows;
  }

  /**
   * Get decisions by outcome
   *
   * @param {string} outcome - Outcome filter
   * @param {number} [limit=50] - Max results
   * @returns {Promise<Array>} Filtered decisions
   */
  async getByOutcome(outcome, limit = 50) {
    const result = await this.pool.query(
      `SELECT * FROM orchestration_log WHERE outcome = $1 ORDER BY created_at DESC LIMIT $2`,
      [outcome, limit]
    );
    return result.rows;
  }

  /**
   * Get blocked decisions
   *
   * @param {number} [limit=50] - Max results
   * @returns {Promise<Array>} Blocked decisions
   */
  async getBlocked(limit = 50) {
    return this.getByOutcome('BLOCK', limit);
  }

  /**
   * Get decisions by domain
   *
   * @param {string} domain - Domain filter
   * @param {number} [limit=50] - Max results
   * @returns {Promise<Array>} Domain decisions
   */
  async getByDomain(domain, limit = 50) {
    const result = await this.pool.query(
      `SELECT * FROM orchestration_log WHERE domain = $1 ORDER BY created_at DESC LIMIT $2`,
      [domain, limit]
    );
    return result.rows;
  }

  /**
   * Get learning analysis (domain performance)
   *
   * @returns {Promise<Array>} Domain performance stats
   */
  async getLearningAnalysis() {
    const result = await this.pool.query(
      `SELECT * FROM orchestration_learning_view`
    );
    return result.rows;
  }

  /**
   * Get statistics
   *
   * @param {string} [userId] - Optional user filter
   * @returns {Promise<Object>} Statistics
   */
  async getStats(userId = null) {
    const whereClause = userId ? 'WHERE user_id = $1' : '';
    const params = userId ? [userId] : [];

    const result = await this.pool.query(
      `SELECT
        COUNT(*) as total,
        COUNT(CASE WHEN outcome = 'ALLOW' THEN 1 END) as allowed,
        COUNT(CASE WHEN outcome = 'BLOCK' THEN 1 END) as blocked,
        COUNT(CASE WHEN outcome = 'MODIFIED' THEN 1 END) as modified,
        COUNT(CASE WHEN outcome = 'ERROR' THEN 1 END) as errors,
        AVG(duration_ms) as avg_duration_ms,
        AVG(judgment_qscore) as avg_qscore,
        COUNT(DISTINCT domain) as unique_domains
      FROM orchestration_log ${whereClause}`,
      params
    );

    return result.rows[0];
  }

  /**
   * Cleanup old decisions (keep 30 days)
   *
   * @returns {Promise<number>} Deleted count
   */
  async cleanup() {
    const result = await this.pool.query(
      `SELECT cleanup_orchestration_logs() as deleted`
    );
    const deleted = result.rows[0].deleted;
    log.debug('Cleanup completed', { deleted });
    return deleted;
  }
}

/**
 * Create repository instance
 *
 * @param {Object} pool - PostgreSQL pool
 * @returns {OrchestrationDecisionRepository}
 */
export function createOrchestrationDecisionRepository(pool) {
  return new OrchestrationDecisionRepository(pool);
}

export default OrchestrationDecisionRepository;
