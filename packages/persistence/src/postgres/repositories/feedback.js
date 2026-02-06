/**
 * Feedback Repository
 *
 * Learning from user corrections.
 * Feedback burns CYNIC's ego - every correction makes it smarter.
 *
 * Implements: BaseRepository
 *
 * @module @cynic/persistence/repositories/feedback
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Feedback Repository
 *
 * LSP compliant - implements standard repository interface.
 *
 * @extends BaseRepository
 */
export class FeedbackRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Create feedback for a judgment
   * Supports orphan feedback (feedback without linked judgment)
   * @param {Object} feedback - Feedback data
   * @param {string} [feedback.judgmentId] - Optional judgment ID (nullable for orphan feedback)
   * @param {string} [feedback.userId] - User ID
   * @param {string} feedback.outcome - 'correct', 'incorrect', 'partial'
   * @param {number} [feedback.actualScore] - Actual score (if known)
   * @param {string} [feedback.reason] - Feedback reason
   * @param {string} [feedback.sourceType] - Source type (manual, test_result, commit, build, etc.)
   * @param {Object} [feedback.sourceContext] - Additional context from source
   */
  async create(feedback) {
    const { rows } = await this.db.query(`
      INSERT INTO feedback (judgment_id, user_id, outcome, actual_score, reason, source_type, source_context)
      VALUES ($1, $2, $3, $4, $5, $6, $7)
      RETURNING *
    `, [
      feedback.judgmentId || null,  // Now nullable for orphan feedback
      feedback.userId || null,
      feedback.outcome,
      feedback.actualScore || null,
      feedback.reason || null,
      feedback.sourceType || 'manual',
      JSON.stringify(feedback.sourceContext || {}),
    ]);
    return rows[0];
  }

  /**
   * Find feedback by judgment ID
   */
  async findByJudgment(judgmentId) {
    const { rows } = await this.db.query(
      'SELECT * FROM feedback WHERE judgment_id = $1 ORDER BY created_at DESC',
      [judgmentId]
    );
    return rows;
  }

  /**
   * Get unapplied feedback
   */
  async findUnapplied(limit = 100) {
    const { rows } = await this.db.query(`
      SELECT f.*,
        COALESCE(j.q_score, f.actual_score) AS q_score,
        j.verdict,
        j.item_type
      FROM feedback f
      LEFT JOIN judgments j ON f.judgment_id = j.judgment_id
      WHERE f.applied = FALSE
      ORDER BY f.created_at ASC
      LIMIT $1
    `, [limit]);
    return rows;
  }

  /**
   * Mark feedback as applied
   */
  async markApplied(feedbackId) {
    const { rows } = await this.db.query(`
      UPDATE feedback SET
        applied = TRUE,
        applied_at = NOW()
      WHERE id = $1
      RETURNING *
    `, [feedbackId]);
    return rows[0];
  }

  /**
   * Get feedback statistics
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE outcome = 'correct') as correct,
        COUNT(*) FILTER (WHERE outcome = 'incorrect') as incorrect,
        COUNT(*) FILTER (WHERE outcome = 'partial') as partial,
        COUNT(*) FILTER (WHERE applied = TRUE) as applied,
        AVG(ABS(actual_score - (
          SELECT q_score FROM judgments WHERE judgment_id = feedback.judgment_id
        ))) FILTER (WHERE actual_score IS NOT NULL) as avg_score_diff
      FROM feedback
    `);

    const stats = rows[0];
    const total = parseInt(stats.total);
    const correct = parseInt(stats.correct);

    return {
      total,
      correct,
      incorrect: parseInt(stats.incorrect),
      partial: parseInt(stats.partial),
      applied: parseInt(stats.applied),
      accuracy: total > 0 ? (correct / total) : 0,
      avgScoreDiff: parseFloat(stats.avg_score_diff) || 0,
    };
  }

  /**
   * Get learning insights from feedback
   */
  async getLearningInsights() {
    const { rows } = await this.db.query(`
      SELECT
        j.item_type,
        COUNT(*) as feedback_count,
        COUNT(*) FILTER (WHERE f.outcome = 'incorrect') as incorrect_count,
        AVG(j.q_score) as avg_original_score,
        AVG(f.actual_score) FILTER (WHERE f.actual_score IS NOT NULL) as avg_actual_score
      FROM feedback f
      JOIN judgments j ON f.judgment_id = j.judgment_id
      GROUP BY j.item_type
      HAVING COUNT(*) >= 3
      ORDER BY incorrect_count DESC
    `);
    return rows;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // BaseRepository Interface Methods (LSP compliance)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Find feedback by ID
   * @param {number} id - Feedback ID
   * @returns {Promise<Object|null>}
   */
  async findById(id) {
    const { rows } = await this.db.query(
      'SELECT * FROM feedback WHERE id = $1',
      [id]
    );
    return rows[0] || null;
  }

  /**
   * List feedback with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0, outcome, applied } = options;

    let sql = 'SELECT * FROM feedback WHERE 1=1';
    const params = [];
    let paramIndex = 1;

    if (outcome) {
      sql += ` AND outcome = $${paramIndex++}`;
      params.push(outcome);
    }

    if (applied !== undefined) {
      sql += ` AND applied = $${paramIndex++}`;
      params.push(applied);
    }

    sql += ` ORDER BY created_at DESC LIMIT $${paramIndex++} OFFSET $${paramIndex}`;
    params.push(limit, offset);

    const { rows } = await this.db.query(sql, params);
    return rows;
  }

  /**
   * Update feedback
   * @param {number} id - Feedback ID
   * @param {Object} data - Update data
   * @returns {Promise<Object|null>}
   */
  async update(id, data) {
    const updates = [];
    const params = [id];
    let paramIndex = 2;

    if (data.outcome !== undefined) {
      updates.push(`outcome = $${paramIndex++}`);
      params.push(data.outcome);
    }
    if (data.actualScore !== undefined) {
      updates.push(`actual_score = $${paramIndex++}`);
      params.push(data.actualScore);
    }
    if (data.reason !== undefined) {
      updates.push(`reason = $${paramIndex++}`);
      params.push(data.reason);
    }
    if (data.applied !== undefined) {
      updates.push(`applied = $${paramIndex++}`);
      params.push(data.applied);
      if (data.applied) {
        updates.push('applied_at = NOW()');
      }
    }

    if (updates.length === 0) {
      return this.findById(id);
    }

    const { rows } = await this.db.query(`
      UPDATE feedback
      SET ${updates.join(', ')}
      WHERE id = $1
      RETURNING *
    `, params);

    return rows[0] || null;
  }

  /**
   * Delete feedback
   * @param {number} id - Feedback ID
   * @returns {Promise<boolean>}
   */
  async delete(id) {
    const { rowCount } = await this.db.query(
      'DELETE FROM feedback WHERE id = $1',
      [id]
    );
    return rowCount > 0;
  }
}

export default FeedbackRepository;
