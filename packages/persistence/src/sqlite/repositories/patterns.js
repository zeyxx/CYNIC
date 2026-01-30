/**
 * SQLite Patterns Repository
 *
 * SQLite-compatible implementation of PatternRepository.
 *
 * @module @cynic/persistence/sqlite/repositories/patterns
 */

'use strict';

import crypto from 'crypto';
import { BaseRepository } from '../../interfaces/IRepository.js';

function generatePatternId() {
  return 'pat_' + crypto.randomBytes(8).toString('hex');
}

/**
 * SQLite Patterns Repository
 */
export class SQLitePatternRepository extends BaseRepository {
  constructor(db) {
    super(db);
  }

  /**
   * Create a new pattern
   */
  async create(pattern) {
    const patternId = pattern.pattern_id || generatePatternId();

    const { rows } = await this.db.query(`
      INSERT INTO patterns (
        pattern_id, user_id, session_id, pattern_type, name, description,
        frequency, weight, axiom, context, metadata
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      RETURNING *
    `, [
      patternId,
      pattern.userId || pattern.user_id || null,
      pattern.sessionId || pattern.session_id || null,
      pattern.patternType || pattern.pattern_type || 'unknown',
      pattern.name || null,
      pattern.description || null,
      pattern.frequency || 1,
      pattern.weight || 1.0,
      pattern.axiom || 'VERIFY',
      JSON.stringify(pattern.context || {}),
      JSON.stringify(pattern.metadata || {}),
    ]);

    return this._parseRow(rows[0]);
  }

  /**
   * Find pattern by ID
   */
  async findById(patternId) {
    const { rows } = await this.db.query(
      'SELECT * FROM patterns WHERE pattern_id = ?',
      [patternId]
    );
    return rows[0] ? this._parseRow(rows[0]) : null;
  }

  /**
   * Update a pattern
   */
  async update(patternId, data) {
    const sets = [];
    const params = [];

    if (data.frequency !== undefined) {
      sets.push('frequency = ?');
      params.push(data.frequency);
    }
    if (data.weight !== undefined) {
      sets.push('weight = ?');
      params.push(data.weight);
    }
    if (data.lastSeen !== undefined || data.last_seen !== undefined) {
      sets.push('last_seen = ?');
      params.push(data.lastSeen || data.last_seen);
    }
    if (data.metadata !== undefined) {
      sets.push('metadata = ?');
      params.push(JSON.stringify(data.metadata));
    }

    sets.push("updated_at = datetime('now')");

    if (sets.length === 1) {
      return this.findById(patternId);
    }

    params.push(patternId);

    const { rows } = await this.db.query(`
      UPDATE patterns SET ${sets.join(', ')} WHERE pattern_id = ?
      RETURNING *
    `, params);

    return rows[0] ? this._parseRow(rows[0]) : null;
  }

  /**
   * Increment pattern frequency
   */
  async incrementFrequency(patternId, boost = 1) {
    const { rows } = await this.db.query(`
      UPDATE patterns
      SET frequency = frequency + ?,
          last_seen = datetime('now'),
          updated_at = datetime('now')
      WHERE pattern_id = ?
      RETURNING *
    `, [boost, patternId]);

    return rows[0] ? this._parseRow(rows[0]) : null;
  }

  /**
   * Update pattern weight (for path reinforcement)
   */
  async updateWeight(patternId, weight) {
    const { rows } = await this.db.query(`
      UPDATE patterns
      SET weight = ?,
          updated_at = datetime('now')
      WHERE pattern_id = ?
      RETURNING *
    `, [weight, patternId]);

    return rows[0] ? this._parseRow(rows[0]) : null;
  }

  /**
   * Delete a pattern
   */
  async delete(patternId) {
    const { rowCount } = await this.db.query(
      'DELETE FROM patterns WHERE pattern_id = ?',
      [patternId]
    );
    return rowCount > 0;
  }

  /**
   * Search patterns
   */
  async search(query, options = {}) {
    const { limit = 10, offset = 0, userId, patternType, axiom } = options;

    let sql = 'SELECT * FROM patterns WHERE 1=1';
    const params = [];

    if (userId) {
      sql += ' AND user_id = ?';
      params.push(userId);
    }

    if (patternType) {
      sql += ' AND pattern_type = ?';
      params.push(patternType);
    }

    if (axiom) {
      sql += ' AND axiom = ?';
      params.push(axiom);
    }

    if (query) {
      sql += ' AND (name LIKE ? OR description LIKE ?)';
      const likeQuery = `%${query}%`;
      params.push(likeQuery, likeQuery);
    }

    sql += ' ORDER BY frequency DESC, weight DESC LIMIT ? OFFSET ?';
    params.push(limit, offset);

    const { rows } = await this.db.query(sql, params);
    return rows.map(row => this._parseRow(row));
  }

  /**
   * Find by type
   */
  async findByType(patternType, options = {}) {
    const { limit = 10, userId } = options;

    let sql = 'SELECT * FROM patterns WHERE pattern_type = ?';
    const params = [patternType];

    if (userId) {
      sql += ' AND user_id = ?';
      params.push(userId);
    }

    sql += ' ORDER BY frequency DESC LIMIT ?';
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(row => this._parseRow(row));
  }

  /**
   * Find top reinforced patterns (for path reinforcement)
   */
  async findTopReinforced(limit = 10, options = {}) {
    const { userId, minWeight = 1.0 } = options;

    let sql = 'SELECT * FROM patterns WHERE weight >= ?';
    const params = [minWeight];

    if (userId) {
      sql += ' AND user_id = ?';
      params.push(userId);
    }

    sql += ' ORDER BY weight DESC, frequency DESC LIMIT ?';
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(row => this._parseRow(row));
  }

  /**
   * Find patterns needing decay (not seen recently)
   */
  async findStalePatterns(hoursThreshold = 168, limit = 50) {
    const { rows } = await this.db.query(`
      SELECT * FROM patterns
      WHERE datetime(last_seen) < datetime('now', '-${hoursThreshold} hours')
        AND weight > 0.236
      ORDER BY last_seen ASC
      LIMIT ?
    `, [limit]);

    return rows.map(row => this._parseRow(row));
  }

  /**
   * List patterns
   */
  async list(options = {}) {
    return this.search('', options);
  }

  /**
   * Get statistics
   */
  async getStats(options = {}) {
    const { userId } = options;

    let sql = `
      SELECT
        COUNT(*) as total,
        AVG(frequency) as avg_frequency,
        AVG(weight) as avg_weight,
        COUNT(DISTINCT pattern_type) as type_count
      FROM patterns WHERE 1=1
    `;
    const params = [];

    if (userId) {
      sql += ' AND user_id = ?';
      params.push(userId);
    }

    const { rows } = await this.db.query(sql, params);
    const stats = rows[0];

    return {
      total: parseInt(stats.total) || 0,
      avgFrequency: parseFloat(stats.avg_frequency) || 0,
      avgWeight: parseFloat(stats.avg_weight) || 0,
      typeCount: parseInt(stats.type_count) || 0,
    };
  }

  /**
   * Parse JSON fields
   * @private
   */
  _parseRow(row) {
    if (!row) return null;

    return {
      ...row,
      context: row.context ? JSON.parse(row.context) : {},
      metadata: row.metadata ? JSON.parse(row.metadata) : {},
    };
  }
}

export default SQLitePatternRepository;
