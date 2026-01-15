/**
 * Judgments Repository
 *
 * Append-only storage for all CYNIC judgments.
 * Core of the PoJ (Proof of Judgment) system.
 *
 * @module @cynic/persistence/repositories/judgments
 */

'use strict';

import crypto from 'crypto';
import { getPool } from '../client.js';

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

export class JudgmentRepository {
  constructor(db = null) {
    this.db = db || getPool();
  }

  /**
   * Store a new judgment
   */
  async create(judgment) {
    const judgmentId = generateJudgmentId();
    const itemHash = hashContent(judgment.item?.content || judgment.itemContent);

    // Build searchable content: include description + content for better search
    const item = judgment.item || {};
    const contentParts = [];
    if (item.description) contentParts.push(item.description);
    if (item.content) contentParts.push(item.content);
    if (item.name) contentParts.push(item.name);
    const searchableContent = contentParts.join('\n') || judgment.itemContent || '';

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
      JSON.stringify(judgment.dimensionScores || judgment.dimension_scores || null),
      JSON.stringify(judgment.weaknesses || []),
      JSON.stringify(judgment.context || {}),
    ]);

    return rows[0];
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
}

export default JudgmentRepository;
