/**
 * Judgments Repository
 *
 * Append-only storage for all CYNIC judgments.
 * Core of the PoJ (Proof of Judgment) system.
 *
 * Implements: BaseRepository, Searchable
 *
 * @module @cynic/persistence/repositories/judgments
 */

'use strict';

import crypto from 'crypto';
import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Generate short judgment ID
 */
function generateJudgmentId() {
  return 'jdg_' + crypto.randomBytes(8).toString('hex');
}

/**
 * Hash item content for deduplication
 */
function hashContent(content) {
  return crypto.createHash('sha256')
    .update(typeof content === 'string' ? content : JSON.stringify(content))
    .digest('hex');
}

/**
 * Judgments Repository
 *
 * LSP compliant - implements standard repository interface.
 *
 * @extends BaseRepository
 */
export class JudgmentRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Supports full-text search via PostgreSQL ILIKE
   * @returns {boolean}
   */
  supportsFTS() {
    return true;
  }

  /**
   * Store a new judgment
   * @param {Object} judgment - Judgment data
   * @param {Object[]} [judgment.reasoningPath] - Reasoning trajectory steps
   */
  async create(judgment) {
    // CRITICAL: Preserve original judgment ID for PoJ chain traceability
    // If judgment.id exists (from DogOrchestrator), use it. Never overwrite.
    const judgmentId = judgment.id || judgment.judgmentId || judgment.judgment_id || generateJudgmentId();
    const itemHash = hashContent(judgment.item?.content || judgment.itemContent);

    // Build searchable content: include description + content for better search
    const item = judgment.item || {};
    const contentParts = [];
    if (item.description) contentParts.push(item.description);
    if (item.content) contentParts.push(item.content);
    if (item.name) contentParts.push(item.name);
    const searchableContent = contentParts.join('\n') || judgment.itemContent || '';

    // Normalize reasoning path (add step numbers if missing)
    const reasoningPath = (judgment.reasoningPath || judgment.reasoning_path || [])
      .map((step, idx) => ({
        step: step.step ?? idx + 1,
        ...step,
      }));

    // P2-A: Extract queryType from context for DPO per-context learning
    const queryType = judgment.queryType
      || judgment.context?.queryType
      || judgment.context?.type
      || judgment.item?.type
      || 'general';

    // Try with reasoning_path + query_type first, fall back if columns don't exist
    try {
      const { rows } = await this.db.query(`
        INSERT INTO judgments (
          judgment_id, user_id, session_id,
          item_type, item_content, item_hash,
          q_score, global_score, confidence, verdict,
          axiom_scores, dimension_scores, weaknesses,
          context, reasoning_path, query_type
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING *
      `, [
        judgmentId,
        judgment.userId || null,
        judgment.sessionId || null,
        judgment.item?.type || judgment.itemType || 'unknown',
        searchableContent,
        itemHash,
        judgment.qScore || judgment.q_score,
        judgment.globalScore || judgment.global_score || judgment.qScore || judgment.q_score,
        judgment.confidence,
        judgment.verdict,
        JSON.stringify(judgment.axiomScores || judgment.axiom_scores || {}),
        JSON.stringify(judgment.dimensions || judgment.dimensionScores || judgment.dimension_scores || null),
        JSON.stringify(judgment.weaknesses || []),
        JSON.stringify(judgment.context || {}),
        JSON.stringify(reasoningPath),
        queryType,
      ]);

      return rows[0];
    } catch (err) {
      // Fallback: reasoning_path/query_type columns may not exist (pre-migration)
      if (err.message?.includes('reasoning_path') || err.message?.includes('query_type')) {
        const { rows } = await this.db.query(`
          INSERT INTO judgments (
            judgment_id, user_id, session_id,
            item_type, item_content, item_hash,
            q_score, global_score, confidence, verdict,
            axiom_scores, dimension_scores, weaknesses,
            context
          ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
          RETURNING *
        `, [
          judgmentId,
          judgment.userId || null,
          judgment.sessionId || null,
          judgment.item?.type || judgment.itemType || 'unknown',
          searchableContent,
          itemHash,
          judgment.qScore || judgment.q_score,
          judgment.globalScore || judgment.global_score || judgment.qScore || judgment.q_score,
          judgment.confidence,
          judgment.verdict,
          JSON.stringify(judgment.axiomScores || judgment.axiom_scores || {}),
          JSON.stringify(judgment.dimensions || judgment.dimensionScores || judgment.dimension_scores || null),
          JSON.stringify(judgment.weaknesses || []),
          JSON.stringify(judgment.context || {}),
        ]);

        return rows[0];
      }
      throw err;
    }
  }

  /**
   * Get judgment by ID
   */
  async findById(judgmentId) {
    const { rows } = await this.db.query(
      'SELECT * FROM judgments WHERE judgment_id = $1',
      [judgmentId]
    );
    return rows[0] || null;
  }

  /**
   * Search judgments
   */
  async search(query, options = {}) {
    const { limit = 10, offset = 0, userId, sessionId, verdict, itemType } = options;

    let sql = 'SELECT * FROM judgments WHERE 1=1';
    const params = [];
    let paramIndex = 1;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }

    if (sessionId) {
      sql += ` AND session_id = $${paramIndex++}`;
      params.push(sessionId);
    }

    if (verdict) {
      sql += ` AND verdict = $${paramIndex++}`;
      params.push(verdict);
    }

    if (itemType) {
      sql += ` AND item_type = $${paramIndex++}`;
      params.push(itemType);
    }

    if (query) {
      sql += ` AND (
        item_content ILIKE $${paramIndex} OR
        item_type ILIKE $${paramIndex} OR
        verdict ILIKE $${paramIndex} OR
        context::text ILIKE $${paramIndex}
      )`;
      params.push(`%${query}%`);
      paramIndex++;
    }

    sql += ` ORDER BY created_at DESC LIMIT $${paramIndex++} OFFSET $${paramIndex}`;
    params.push(limit, offset);

    const { rows } = await this.db.query(sql, params);
    return rows;
  }

  /**
   * Get recent judgments
   */
  async findRecent(limit = 10) {
    const { rows } = await this.db.query(
      'SELECT * FROM judgments ORDER BY created_at DESC LIMIT $1',
      [limit]
    );
    return rows;
  }

  /**
   * Get judgment statistics
   */
  async getStats(options = {}) {
    const { userId, sessionId, since } = options;

    let sql = `
      SELECT
        COUNT(*) as total,
        AVG(q_score) as avg_score,
        AVG(confidence) as avg_confidence,
        COUNT(*) FILTER (WHERE verdict = 'HOWL') as howl_count,
        COUNT(*) FILTER (WHERE verdict = 'WAG') as wag_count,
        COUNT(*) FILTER (WHERE verdict = 'GROWL') as growl_count,
        COUNT(*) FILTER (WHERE verdict = 'BARK') as bark_count
      FROM judgments WHERE 1=1
    `;
    const params = [];
    let paramIndex = 1;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }

    if (sessionId) {
      sql += ` AND session_id = $${paramIndex++}`;
      params.push(sessionId);
    }

    if (since) {
      sql += ` AND created_at >= $${paramIndex++}`;
      params.push(since);
    }

    const { rows } = await this.db.query(sql, params);
    const stats = rows[0];

    return {
      total: parseInt(stats.total),
      avgScore: parseFloat(stats.avg_score) || 0,
      avgConfidence: parseFloat(stats.avg_confidence) || 0,
      verdicts: {
        HOWL: parseInt(stats.howl_count),
        WAG: parseInt(stats.wag_count),
        GROWL: parseInt(stats.growl_count),
        BARK: parseInt(stats.bark_count),
      },
    };
  }

  /**
   * Find similar judgments by content hash
   */
  async findSimilar(content, limit = 5) {
    const hash = hashContent(content);

    const { rows } = await this.db.query(`
      SELECT * FROM judgments
      WHERE item_hash = $1
      ORDER BY created_at DESC
      LIMIT $2
    `, [hash, limit]);

    return rows;
  }

  /**
   * Count total judgments
   */
  async count(options = {}) {
    const { userId, sessionId } = options;

    let sql = 'SELECT COUNT(*) FROM judgments WHERE 1=1';
    const params = [];
    let paramIndex = 1;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }

    if (sessionId) {
      sql += ` AND session_id = $${paramIndex++}`;
      params.push(sessionId);
    }

    const { rows } = await this.db.query(sql, params);
    return parseInt(rows[0].count);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // BaseRepository Interface Methods (LSP compliance)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * List judgments with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    return this.search('', options);
  }

  /**
   * Update is not supported for append-only judgments
   * @throws {Error} Judgments are immutable
   */
  async update(id, data) {
    throw new Error('Judgments are append-only and cannot be updated');
  }

  /**
   * Delete is not supported for append-only judgments
   * @throws {Error} Judgments are immutable
   */
  async delete(id) {
    throw new Error('Judgments are append-only and cannot be deleted');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Reasoning Trajectory Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get reasoning trajectory for a judgment
   * @param {string} judgmentId - Judgment ID
   * @returns {Promise<Object[]|null>} Reasoning path or null
   */
  async getReasoningPath(judgmentId) {
    const { rows } = await this.db.query(
      'SELECT reasoning_path FROM judgments WHERE judgment_id = $1',
      [judgmentId]
    );
    return rows[0]?.reasoning_path || null;
  }

  /**
   * Find judgments with similar reasoning patterns
   * @param {string} itemType - Item type to search
   * @param {Object} [options={}] - Search options
   * @returns {Promise<Object[]>} Judgments with reasoning paths
   */
  async findWithReasoningPath(itemType, options = {}) {
    const { limit = 10, verdict, minSteps = 1 } = options;

    const { rows } = await this.db.query(`
      SELECT judgment_id, item_type, verdict, q_score, confidence, reasoning_path
      FROM judgments
      WHERE item_type = $1
        AND reasoning_path IS NOT NULL
        AND jsonb_array_length(reasoning_path) >= $2
        ${verdict ? 'AND verdict = $4' : ''}
      ORDER BY created_at DESC
      LIMIT $3
    `, verdict ? [itemType, minSteps, limit, verdict] : [itemType, minSteps, limit]);

    return rows;
  }

  /**
   * Get trajectory statistics for learning
   * @param {string} [itemType] - Filter by item type
   * @returns {Promise<Object>} Trajectory stats
   */
  async getTrajectoryStats(itemType = null) {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) FILTER (WHERE reasoning_path IS NOT NULL AND jsonb_array_length(reasoning_path) > 0) as with_trajectory,
        COUNT(*) as total,
        AVG(jsonb_array_length(reasoning_path)) FILTER (WHERE reasoning_path IS NOT NULL) as avg_steps,
        MAX(jsonb_array_length(reasoning_path)) as max_steps
      FROM judgments
      ${itemType ? 'WHERE item_type = $1' : ''}
    `, itemType ? [itemType] : []);

    const stats = rows[0];
    return {
      totalJudgments: parseInt(stats.total) || 0,
      withTrajectory: parseInt(stats.with_trajectory) || 0,
      avgSteps: parseFloat(stats.avg_steps) || 0,
      maxSteps: parseInt(stats.max_steps) || 0,
      coverageRate: stats.total > 0
        ? (parseInt(stats.with_trajectory) / parseInt(stats.total))
        : 0,
    };
  }
}

export default JudgmentRepository;
