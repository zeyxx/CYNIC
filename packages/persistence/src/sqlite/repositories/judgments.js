/**
 * SQLite Judgments Repository
 *
 * SQLite-compatible implementation of JudgmentRepository.
 * Matches PostgreSQL interface for seamless fallback.
 *
 * @module @cynic/persistence/sqlite/repositories/judgments
 */

'use strict';

import crypto from 'crypto';
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
 * SQLite Judgments Repository
 *
 * @extends BaseRepository
 */
export class SQLiteJudgmentRepository extends BaseRepository {
  constructor(db) {
    super(db);
  }

  /**
   * Supports text search via SQLite LIKE
   */
  supportsFTS() {
    return true;
  }

  /**
   * Store a new judgment
   */
  async create(judgment) {
    const judgmentId = generateJudgmentId();
    const itemHash = hashContent(judgment.item?.content || judgment.itemContent);

    const item = judgment.item || {};
    const contentParts = [];
    if (item.description) contentParts.push(item.description);
    if (item.content) contentParts.push(item.content);
    if (item.name) contentParts.push(item.name);
    const searchableContent = contentParts.join('\n') || judgment.itemContent || '';

    const reasoningPath = (judgment.reasoningPath || judgment.reasoning_path || [])
      .map((step, idx) => ({
        step: step.step ?? idx + 1,
        ...step,
      }));

    const { rows } = await this.db.query(`
      INSERT INTO judgments (
        judgment_id, user_id, session_id,
        item_type, item_content, item_hash,
        q_score, global_score, confidence, verdict,
        axiom_scores, dimension_scores, weaknesses,
        context, reasoning_path
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
      JSON.stringify(reasoningPath),
    ]);

    return this._parseRow(rows[0]);
  }

  /**
   * Get judgment by ID
   */
  async findById(judgmentId) {
    const { rows } = await this.db.query(
      'SELECT * FROM judgments WHERE judgment_id = ?',
      [judgmentId]
    );
    return rows[0] ? this._parseRow(rows[0]) : null;
  }

  /**
   * Search judgments
   */
  async search(query, options = {}) {
    const { limit = 10, offset = 0, userId, sessionId, verdict, itemType } = options;

    let sql = 'SELECT * FROM judgments WHERE 1=1';
    const params = [];

    if (userId) {
      sql += ' AND user_id = ?';
      params.push(userId);
    }

    if (sessionId) {
      sql += ' AND session_id = ?';
      params.push(sessionId);
    }

    if (verdict) {
      sql += ' AND verdict = ?';
      params.push(verdict);
    }

    if (itemType) {
      sql += ' AND item_type = ?';
      params.push(itemType);
    }

    if (query) {
      sql += ` AND (
        item_content LIKE ? OR
        item_type LIKE ? OR
        verdict LIKE ? OR
        context LIKE ?
      )`;
      const likeQuery = `%${query}%`;
      params.push(likeQuery, likeQuery, likeQuery, likeQuery);
    }

    sql += ' ORDER BY created_at DESC LIMIT ? OFFSET ?';
    params.push(limit, offset);

    const { rows } = await this.db.query(sql, params);
    return rows.map(row => this._parseRow(row));
  }

  /**
   * Get recent judgments
   */
  async findRecent(limit = 10) {
    const { rows } = await this.db.query(
      'SELECT * FROM judgments ORDER BY created_at DESC LIMIT ?',
      [limit]
    );
    return rows.map(row => this._parseRow(row));
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
        SUM(CASE WHEN verdict = 'HOWL' THEN 1 ELSE 0 END) as howl_count,
        SUM(CASE WHEN verdict = 'WAG' THEN 1 ELSE 0 END) as wag_count,
        SUM(CASE WHEN verdict = 'GROWL' THEN 1 ELSE 0 END) as growl_count,
        SUM(CASE WHEN verdict = 'BARK' THEN 1 ELSE 0 END) as bark_count
      FROM judgments WHERE 1=1
    `;
    const params = [];

    if (userId) {
      sql += ' AND user_id = ?';
      params.push(userId);
    }

    if (sessionId) {
      sql += ' AND session_id = ?';
      params.push(sessionId);
    }

    if (since) {
      sql += ' AND created_at >= ?';
      params.push(since);
    }

    const { rows } = await this.db.query(sql, params);
    const stats = rows[0];

    return {
      total: parseInt(stats.total) || 0,
      avgScore: parseFloat(stats.avg_score) || 0,
      avgConfidence: parseFloat(stats.avg_confidence) || 0,
      verdicts: {
        HOWL: parseInt(stats.howl_count) || 0,
        WAG: parseInt(stats.wag_count) || 0,
        GROWL: parseInt(stats.growl_count) || 0,
        BARK: parseInt(stats.bark_count) || 0,
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
      WHERE item_hash = ?
      ORDER BY created_at DESC
      LIMIT ?
    `, [hash, limit]);

    return rows.map(row => this._parseRow(row));
  }

  /**
   * Count total judgments
   */
  async count(options = {}) {
    const { userId, sessionId } = options;

    let sql = 'SELECT COUNT(*) as count FROM judgments WHERE 1=1';
    const params = [];

    if (userId) {
      sql += ' AND user_id = ?';
      params.push(userId);
    }

    if (sessionId) {
      sql += ' AND session_id = ?';
      params.push(sessionId);
    }

    const { rows } = await this.db.query(sql, params);
    return parseInt(rows[0].count) || 0;
  }

  /**
   * List judgments with pagination
   */
  async list(options = {}) {
    return this.search('', options);
  }

  /**
   * Update is not supported (append-only)
   */
  async update(id, data) {
    throw new Error('Judgments are append-only and cannot be updated');
  }

  /**
   * Delete is not supported (append-only)
   */
  async delete(id) {
    throw new Error('Judgments are append-only and cannot be deleted');
  }

  /**
   * Get reasoning trajectory for a judgment
   */
  async getReasoningPath(judgmentId) {
    const { rows } = await this.db.query(
      'SELECT reasoning_path FROM judgments WHERE judgment_id = ?',
      [judgmentId]
    );
    if (!rows[0]?.reasoning_path) return null;
    return JSON.parse(rows[0].reasoning_path);
  }

  /**
   * Find judgments with reasoning paths
   */
  async findWithReasoningPath(itemType, options = {}) {
    const { limit = 10, verdict, minSteps = 1 } = options;

    let sql = `
      SELECT judgment_id, item_type, verdict, q_score, confidence, reasoning_path
      FROM judgments
      WHERE item_type = ?
        AND reasoning_path IS NOT NULL
        AND reasoning_path != '[]'
    `;
    const params = [itemType];

    if (verdict) {
      sql += ' AND verdict = ?';
      params.push(verdict);
    }

    sql += ' ORDER BY created_at DESC LIMIT ?';
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows
      .map(row => ({
        ...row,
        reasoning_path: JSON.parse(row.reasoning_path || '[]'),
      }))
      .filter(row => row.reasoning_path.length >= minSteps);
  }

  /**
   * Get trajectory statistics
   */
  async getTrajectoryStats(itemType = null) {
    let sql = `
      SELECT
        COUNT(*) as total,
        SUM(CASE WHEN reasoning_path IS NOT NULL AND reasoning_path != '[]' THEN 1 ELSE 0 END) as with_trajectory
      FROM judgments
    `;
    const params = [];

    if (itemType) {
      sql += ' WHERE item_type = ?';
      params.push(itemType);
    }

    const { rows } = await this.db.query(sql, params);
    const stats = rows[0];

    return {
      totalJudgments: parseInt(stats.total) || 0,
      withTrajectory: parseInt(stats.with_trajectory) || 0,
      avgSteps: 0, // Would need to parse all JSON to calculate
      maxSteps: 0,
      coverageRate: stats.total > 0
        ? (parseInt(stats.with_trajectory) / parseInt(stats.total))
        : 0,
    };
  }

  /**
   * Parse JSON fields in row
   * @private
   */
  _parseRow(row) {
    if (!row) return null;

    return {
      ...row,
      axiom_scores: row.axiom_scores ? JSON.parse(row.axiom_scores) : {},
      dimension_scores: row.dimension_scores ? JSON.parse(row.dimension_scores) : null,
      weaknesses: row.weaknesses ? JSON.parse(row.weaknesses) : [],
      context: row.context ? JSON.parse(row.context) : {},
      reasoning_path: row.reasoning_path ? JSON.parse(row.reasoning_path) : [],
    };
  }
}

export default SQLiteJudgmentRepository;
