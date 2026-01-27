/**
 * Lessons Learned Repository
 *
 * Tracks mistakes and corrections for self-improvement.
 * Part of CYNIC's Total Memory system.
 *
 * @module @cynic/persistence/repositories/lessons-learned
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Lesson categories
 * @enum {string}
 */
export const LessonCategory = {
  BUG: 'bug',
  ARCHITECTURE: 'architecture',
  PROCESS: 'process',
  COMMUNICATION: 'communication',
  PERFORMANCE: 'performance',
  SECURITY: 'security',
  TESTING: 'testing',
  DESIGN: 'design',
  JUDGMENT: 'judgment',
  OTHER: 'other',
};

/**
 * Severity levels
 * @enum {string}
 */
export const LessonSeverity = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};

/**
 * Lessons Learned Repository
 *
 * @extends BaseRepository
 */
export class LessonsLearnedRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Supports full-text search
   * @returns {boolean}
   */
  supportsFTS() {
    return true;
  }

  /**
   * Create a new lesson
   *
   * @param {Object} lesson - Lesson data
   * @param {string} lesson.userId - User ID
   * @param {string} lesson.category - Lesson category
   * @param {string} lesson.mistake - What went wrong
   * @param {string} lesson.correction - What was done to fix it
   * @param {string} [lesson.prevention] - How to prevent in future
   * @param {string} [lesson.severity='medium'] - Severity level
   * @param {number[]} [lesson.embedding] - Vector embedding
   * @param {string} [lesson.sourceJudgmentId] - Related judgment ID
   * @param {string} [lesson.sourceSessionId] - Session where learned
   * @returns {Promise<Object>} Created lesson
   */
  async create(lesson) {
    const { rows } = await this.db.query(`
      INSERT INTO lessons_learned (
        user_id, category, mistake, correction, prevention,
        severity, embedding, source_judgment_id, source_session_id
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
      RETURNING *
    `, [
      lesson.userId,
      lesson.category,
      lesson.mistake,
      lesson.correction,
      lesson.prevention || null,
      lesson.severity || 'medium',
      lesson.embedding ? JSON.stringify(lesson.embedding) : null,
      lesson.sourceJudgmentId || null,
      lesson.sourceSessionId || null,
    ]);
    return this._formatRow(rows[0]);
  }

  /**
   * Find lesson by ID
   * @param {string} id - Lesson UUID
   * @returns {Promise<Object|null>}
   */
  async findById(id) {
    const { rows } = await this.db.query(
      'SELECT * FROM lessons_learned WHERE id = $1',
      [id]
    );
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Search lessons using FTS or hybrid search
   *
   * @param {string} userId - User ID
   * @param {string} query - Search query
   * @param {Object} [options] - Search options
   * @param {number[]} [options.embedding] - Query embedding for vector search
   * @param {string[]} [options.categories] - Filter by categories
   * @param {string[]} [options.severities] - Filter by severities
   * @param {number} [options.limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async search(userId, query, options = {}) {
    const {
      embedding = null,
      categories = null,
      severities = null,
      limit = 10,
    } = options;

    const PHI_FTS = 0.382;
    const PHI_VECTOR = 0.618;

    let sql;
    const params = [userId, query];
    let paramIndex = 3;

    if (embedding) {
      // Hybrid search
      sql = `
        WITH fts_results AS (
          SELECT id,
            ts_rank(to_tsvector('english', mistake || ' ' || correction || ' ' || COALESCE(prevention, '')),
                    plainto_tsquery('english', $2)) as fts_score
          FROM lessons_learned
          WHERE user_id = $1
        ),
        vector_results AS (
          SELECT id, 1 - (embedding <=> $${paramIndex}::vector) as vector_score
          FROM lessons_learned
          WHERE user_id = $1 AND embedding IS NOT NULL
        )
        SELECT l.*, f.fts_score, v.vector_score,
               (COALESCE(f.fts_score, 0) * ${PHI_FTS} + COALESCE(v.vector_score, 0) * ${PHI_VECTOR}) as combined_score
        FROM lessons_learned l
        LEFT JOIN fts_results f ON l.id = f.id
        LEFT JOIN vector_results v ON l.id = v.id
        WHERE l.user_id = $1
      `;
      params.push(JSON.stringify(embedding));
      paramIndex++;
    } else {
      // FTS-only search
      sql = `
        SELECT *,
          ts_rank(to_tsvector('english', mistake || ' ' || correction || ' ' || COALESCE(prevention, '')),
                  plainto_tsquery('english', $2)) as fts_score,
          ts_rank(to_tsvector('english', mistake || ' ' || correction || ' ' || COALESCE(prevention, '')),
                  plainto_tsquery('english', $2)) * ${PHI_FTS} as combined_score
        FROM lessons_learned
        WHERE user_id = $1
          AND to_tsvector('english', mistake || ' ' || correction || ' ' || COALESCE(prevention, ''))
              @@ plainto_tsquery('english', $2)
      `;
    }

    if (categories && categories.length > 0) {
      sql += ` AND category = ANY($${paramIndex++})`;
      params.push(categories);
    }

    if (severities && severities.length > 0) {
      sql += ` AND severity = ANY($${paramIndex++})`;
      params.push(severities);
    }

    sql += ` ORDER BY combined_score DESC, occurrence_count DESC, last_occurred DESC LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Find similar lessons to avoid repeating mistakes
   *
   * This is the key method for self-correction - checks if we've made
   * similar mistakes before.
   *
   * @param {string} userId - User ID
   * @param {string} context - Current context/action description
   * @param {Object} [options] - Search options
   * @param {number[]} [options.embedding] - Context embedding
   * @param {number} [options.limit=5] - Maximum results
   * @returns {Promise<Object[]>} Relevant lessons
   */
  async findSimilar(userId, context, options = {}) {
    return this.search(userId, context, {
      ...options,
      limit: options.limit || 5,
    });
  }

  /**
   * Get lessons by category
   * @param {string} userId - User ID
   * @param {string} category - Lesson category
   * @param {number} [limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findByCategory(userId, category, limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM lessons_learned
      WHERE user_id = $1 AND category = $2
      ORDER BY occurrence_count DESC, last_occurred DESC
      LIMIT $3
    `, [userId, category, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get critical lessons (should never repeat)
   * @param {string} userId - User ID
   * @param {number} [limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findCritical(userId, limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM lessons_learned
      WHERE user_id = $1 AND severity IN ('critical', 'high')
      ORDER BY severity DESC, occurrence_count DESC
      LIMIT $2
    `, [userId, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get frequently occurring lessons (recurring mistakes)
   * @param {string} userId - User ID
   * @param {number} [minOccurrences=2] - Minimum occurrence count
   * @param {number} [limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findRecurring(userId, minOccurrences = 2, limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM lessons_learned
      WHERE user_id = $1 AND occurrence_count >= $2
      ORDER BY occurrence_count DESC, last_occurred DESC
      LIMIT $3
    `, [userId, minOccurrences, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Record another occurrence of a lesson
   * @param {string} lessonId - Lesson UUID
   * @returns {Promise<void>}
   */
  async recordOccurrence(lessonId) {
    await this.db.query('SELECT record_lesson_occurrence($1)', [lessonId]);
  }

  /**
   * Update a lesson
   * @param {string} id - Lesson UUID
   * @param {Object} data - Update data
   * @returns {Promise<Object|null>}
   */
  async update(id, data) {
    const updates = [];
    const params = [id];
    let paramIndex = 2;

    if (data.mistake !== undefined) {
      updates.push(`mistake = $${paramIndex++}`);
      params.push(data.mistake);
    }
    if (data.correction !== undefined) {
      updates.push(`correction = $${paramIndex++}`);
      params.push(data.correction);
    }
    if (data.prevention !== undefined) {
      updates.push(`prevention = $${paramIndex++}`);
      params.push(data.prevention);
    }
    if (data.severity !== undefined) {
      updates.push(`severity = $${paramIndex++}`);
      params.push(data.severity);
    }
    if (data.embedding !== undefined) {
      updates.push(`embedding = $${paramIndex++}`);
      params.push(data.embedding ? JSON.stringify(data.embedding) : null);
    }

    if (updates.length === 0) {
      return this.findById(id);
    }

    const { rows } = await this.db.query(`
      UPDATE lessons_learned
      SET ${updates.join(', ')}
      WHERE id = $1
      RETURNING *
    `, params);

    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Delete a lesson
   * @param {string} id - Lesson UUID
   * @returns {Promise<boolean>}
   */
  async delete(id) {
    const { rowCount } = await this.db.query(
      'DELETE FROM lessons_learned WHERE id = $1',
      [id]
    );
    return rowCount > 0;
  }

  /**
   * List lessons with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0, userId, category } = options;

    let sql = 'SELECT * FROM lessons_learned WHERE 1=1';
    const params = [];
    let paramIndex = 1;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }
    if (category) {
      sql += ` AND category = $${paramIndex++}`;
      params.push(category);
    }

    sql += ` ORDER BY last_occurred DESC LIMIT $${paramIndex++} OFFSET $${paramIndex}`;
    params.push(limit, offset);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get repository statistics
   * @param {string} [userId] - Optional user ID filter
   * @returns {Promise<Object>}
   */
  async getStats(userId = null) {
    let sql = `
      SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE severity = 'critical') as critical,
        COUNT(*) FILTER (WHERE severity = 'high') as high,
        COUNT(*) FILTER (WHERE occurrence_count > 1) as recurring,
        SUM(occurrence_count) as total_occurrences
      FROM lessons_learned
    `;
    const params = [];

    if (userId) {
      sql += ' WHERE user_id = $1';
      params.push(userId);
    }

    const { rows: statsRows } = await this.db.query(sql, params);

    // Get counts by category
    let catSql = 'SELECT category, COUNT(*) as count FROM lessons_learned';
    if (userId) {
      catSql += ' WHERE user_id = $1';
    }
    catSql += ' GROUP BY category ORDER BY count DESC';

    const { rows: catRows } = await this.db.query(catSql, params);

    const stats = statsRows[0];
    return {
      total: parseInt(stats.total),
      critical: parseInt(stats.critical),
      high: parseInt(stats.high),
      recurring: parseInt(stats.recurring),
      totalOccurrences: parseInt(stats.total_occurrences) || 0,
      byCategory: Object.fromEntries(catRows.map(r => [r.category, parseInt(r.count)])),
    };
  }

  /**
   * Format database row to consistent object
   * @private
   */
  _formatRow(row) {
    if (!row) return null;
    return {
      id: row.id,
      userId: row.user_id,
      category: row.category,
      mistake: row.mistake,
      correction: row.correction,
      prevention: row.prevention,
      severity: row.severity,
      embedding: row.embedding,
      occurrenceCount: row.occurrence_count,
      lastOccurred: row.last_occurred,
      sourceJudgmentId: row.source_judgment_id,
      sourceSessionId: row.source_session_id,
      createdAt: row.created_at,
      // Search result fields
      combinedScore: row.combined_score,
      ftsScore: row.fts_score,
      vectorScore: row.vector_score,
    };
  }
}

export default LessonsLearnedRepository;
