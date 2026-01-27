/**
 * Conversation Memories Repository
 *
 * Stores summaries, key moments, decisions, and preferences from conversations.
 * Supports hybrid FTS + vector search for semantic retrieval.
 *
 * Part of CYNIC's Total Memory system.
 *
 * @module @cynic/persistence/repositories/conversation-memories
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Memory types
 * @enum {string}
 */
export const MemoryType = {
  SUMMARY: 'summary',
  KEY_MOMENT: 'key_moment',
  DECISION: 'decision',
  PREFERENCE: 'preference',
  CORRECTION: 'correction',
  INSIGHT: 'insight',
};

/**
 * Conversation Memories Repository
 *
 * Implements hybrid search combining full-text search and vector similarity.
 * Uses φ-weighted scoring: 0.382 FTS + 0.618 vector.
 *
 * @extends BaseRepository
 */
export class ConversationMemoriesRepository extends BaseRepository {
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
   * Supports vector search (if pgvector available)
   * @returns {boolean}
   */
  supportsVector() {
    return true;
  }

  /**
   * Create a new memory
   *
   * @param {Object} memory - Memory data
   * @param {string} memory.userId - User ID
   * @param {string} [memory.sessionId] - Session ID
   * @param {string} memory.memoryType - Type of memory
   * @param {string} memory.content - Memory content
   * @param {number[]} [memory.embedding] - Vector embedding (1536 dimensions)
   * @param {number} [memory.importance=0.5] - Importance score (0-1)
   * @param {Object} [memory.context={}] - Additional context
   * @returns {Promise<Object>} Created memory
   */
  async create(memory) {
    const { rows } = await this.db.query(`
      INSERT INTO conversation_memories (
        user_id, session_id, memory_type, content,
        embedding, importance, context
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
      RETURNING *
    `, [
      memory.userId,
      memory.sessionId || null,
      memory.memoryType,
      memory.content,
      memory.embedding ? JSON.stringify(memory.embedding) : null,
      memory.importance ?? 0.5,
      JSON.stringify(memory.context || {}),
    ]);
    return this._formatRow(rows[0]);
  }

  /**
   * Find memory by ID
   * @param {string} id - Memory UUID
   * @returns {Promise<Object|null>}
   */
  async findById(id) {
    const { rows } = await this.db.query(
      'SELECT * FROM conversation_memories WHERE id = $1',
      [id]
    );
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Search memories using hybrid FTS + vector scoring
   *
   * φ-weighted: 0.382 FTS + 0.618 vector
   *
   * @param {string} userId - User ID
   * @param {string} query - Search query text
   * @param {Object} [options] - Search options
   * @param {number[]} [options.embedding] - Query embedding for vector search
   * @param {string[]} [options.memoryTypes] - Filter by memory types
   * @param {number} [options.minRelevance=0.236] - Minimum relevance score (φ⁻³)
   * @param {number} [options.limit=10] - Maximum results
   * @returns {Promise<Object[]>} Memories with combined scores
   */
  async search(userId, query, options = {}) {
    const {
      embedding = null,
      memoryTypes = ['summary', 'key_moment', 'decision', 'preference'],
      minRelevance = 0.236,
      limit = 10,
    } = options;

    // Use the hybrid search function if embedding provided
    if (embedding) {
      const { rows } = await this.db.query(`
        SELECT * FROM search_memories_hybrid($1, $2, $3, $4, $5, $6)
      `, [
        userId,
        query,
        JSON.stringify(embedding),
        memoryTypes,
        minRelevance,
        limit,
      ]);
      return rows.map(r => this._formatRow(r));
    }

    // Fallback to FTS-only search
    const { rows } = await this.db.query(`
      SELECT *,
        ts_rank(to_tsvector('english', content), plainto_tsquery('english', $2)) as fts_score
      FROM conversation_memories
      WHERE user_id = $1
        AND memory_type = ANY($3)
        AND to_tsvector('english', content) @@ plainto_tsquery('english', $2)
      ORDER BY fts_score DESC, importance DESC, created_at DESC
      LIMIT $4
    `, [userId, query, memoryTypes, limit]);

    return rows.map(r => ({
      ...this._formatRow(r),
      combined_score: r.fts_score * 0.382, // FTS-only uses FTS weight
    }));
  }

  /**
   * Get memories by user
   * @param {string} userId - User ID
   * @param {Object} [options] - Query options
   * @returns {Promise<Object[]>}
   */
  async findByUser(userId, options = {}) {
    const { memoryType, limit = 10, offset = 0 } = options;

    let sql = 'SELECT * FROM conversation_memories WHERE user_id = $1';
    const params = [userId];
    let paramIndex = 2;

    if (memoryType) {
      sql += ` AND memory_type = $${paramIndex++}`;
      params.push(memoryType);
    }

    sql += ` ORDER BY created_at DESC LIMIT $${paramIndex++} OFFSET $${paramIndex}`;
    params.push(limit, offset);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get memories by session
   * @param {string} sessionId - Session ID
   * @param {number} [limit=20] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findBySession(sessionId, limit = 20) {
    const { rows } = await this.db.query(`
      SELECT * FROM conversation_memories
      WHERE session_id = $1
      ORDER BY created_at ASC
      LIMIT $2
    `, [sessionId, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get recent memories by importance
   * @param {string} userId - User ID
   * @param {number} [minImportance=0.5] - Minimum importance
   * @param {number} [limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findImportant(userId, minImportance = 0.5, limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM conversation_memories
      WHERE user_id = $1 AND importance >= $2
      ORDER BY importance DESC, created_at DESC
      LIMIT $3
    `, [userId, minImportance, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Record access to memories (updates last_accessed and access_count)
   * @param {string[]} memoryIds - Array of memory UUIDs
   * @returns {Promise<number>} Number of memories updated
   */
  async recordAccess(memoryIds) {
    if (!memoryIds || memoryIds.length === 0) return 0;

    const { rows } = await this.db.query(
      'SELECT record_memory_access($1)',
      [memoryIds]
    );
    return rows[0]?.record_memory_access || 0;
  }

  /**
   * Update a memory
   * @param {string} id - Memory UUID
   * @param {Object} data - Update data
   * @returns {Promise<Object|null>}
   */
  async update(id, data) {
    const updates = [];
    const params = [id];
    let paramIndex = 2;

    if (data.content !== undefined) {
      updates.push(`content = $${paramIndex++}`);
      params.push(data.content);
    }
    if (data.embedding !== undefined) {
      updates.push(`embedding = $${paramIndex++}`);
      params.push(data.embedding ? JSON.stringify(data.embedding) : null);
    }
    if (data.importance !== undefined) {
      updates.push(`importance = $${paramIndex++}`);
      params.push(data.importance);
    }
    if (data.context !== undefined) {
      updates.push(`context = $${paramIndex++}`);
      params.push(JSON.stringify(data.context));
    }

    if (updates.length === 0) {
      return this.findById(id);
    }

    const { rows } = await this.db.query(`
      UPDATE conversation_memories
      SET ${updates.join(', ')}
      WHERE id = $1
      RETURNING *
    `, params);

    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Delete a memory
   * @param {string} id - Memory UUID
   * @returns {Promise<boolean>}
   */
  async delete(id) {
    const { rowCount } = await this.db.query(
      'DELETE FROM conversation_memories WHERE id = $1',
      [id]
    );
    return rowCount > 0;
  }

  /**
   * List memories with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0, userId, memoryType } = options;

    let sql = 'SELECT * FROM conversation_memories WHERE 1=1';
    const params = [];
    let paramIndex = 1;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }
    if (memoryType) {
      sql += ` AND memory_type = $${paramIndex++}`;
      params.push(memoryType);
    }

    sql += ` ORDER BY created_at DESC LIMIT $${paramIndex++} OFFSET $${paramIndex}`;
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
        COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '24 hours') as recent,
        AVG(importance) as avg_importance,
        AVG(access_count) as avg_access_count
      FROM conversation_memories
    `;
    const params = [];

    if (userId) {
      sql += ' WHERE user_id = $1';
      params.push(userId);
    }

    const { rows: statsRows } = await this.db.query(sql, params);

    // Get counts by type
    let typeSql = `
      SELECT memory_type, COUNT(*) as count
      FROM conversation_memories
    `;
    if (userId) {
      typeSql += ' WHERE user_id = $1';
    }
    typeSql += ' GROUP BY memory_type ORDER BY count DESC';

    const { rows: typeRows } = await this.db.query(typeSql, params);

    const stats = statsRows[0];
    return {
      total: parseInt(stats.total),
      recent: parseInt(stats.recent),
      avgImportance: parseFloat(stats.avg_importance) || 0,
      avgAccessCount: parseFloat(stats.avg_access_count) || 0,
      byType: Object.fromEntries(typeRows.map(r => [r.memory_type, parseInt(r.count)])),
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
      sessionId: row.session_id,
      memoryType: row.memory_type,
      content: row.content,
      embedding: row.embedding,
      importance: parseFloat(row.importance),
      context: row.context || {},
      createdAt: row.created_at,
      lastAccessed: row.last_accessed,
      accessCount: row.access_count,
      // Search result fields
      combinedScore: row.combined_score,
      ftsScore: row.fts_score,
    };
  }
}

export default ConversationMemoriesRepository;
