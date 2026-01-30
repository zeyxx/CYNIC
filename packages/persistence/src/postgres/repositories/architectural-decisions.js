/**
 * Architectural Decisions Repository
 *
 * Tracks design choices, technology decisions, and patterns with rationale.
 * Part of CYNIC's Total Memory system.
 *
 * @module @cynic/persistence/repositories/architectural-decisions
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Decision types
 * @enum {string}
 */
export const DecisionType = {
  PATTERN: 'pattern',
  TECHNOLOGY: 'technology',
  STRUCTURE: 'structure',
  NAMING: 'naming',
  API: 'api',
  DATA: 'data',
  SECURITY: 'security',
  PERFORMANCE: 'performance',
  TESTING: 'testing',
  DEPLOYMENT: 'deployment',
  OTHER: 'other',
};

/**
 * Decision status
 * @enum {string}
 */
export const DecisionStatus = {
  ACTIVE: 'active',
  SUPERSEDED: 'superseded',
  DEPRECATED: 'deprecated',
  PENDING: 'pending',
};

/**
 * Architectural Decisions Repository
 *
 * @extends BaseRepository
 */
export class ArchitecturalDecisionsRepository extends BaseRepository {
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
   * Create a new decision
   *
   * @param {Object} decision - Decision data
   * @param {string} decision.userId - User ID
   * @param {string} [decision.projectPath] - Project path
   * @param {string} decision.decisionType - Type of decision
   * @param {string} decision.title - Decision title
   * @param {string} decision.description - Decision description
   * @param {string} [decision.rationale] - Why this decision was made
   * @param {Object[]} [decision.alternatives] - Alternatives considered
   * @param {Object} [decision.consequences] - Consequences of decision
   * @param {number[]} [decision.embedding] - Vector embedding
   * @param {string} [decision.status='active'] - Decision status
   * @returns {Promise<Object>} Created decision
   */
  async create(decision) {
    const { rows } = await this.db.query(`
      INSERT INTO architectural_decisions (
        user_id, project_path, decision_type, title, description,
        rationale, alternatives, consequences, embedding, status
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
      RETURNING *
    `, [
      decision.userId,
      decision.projectPath || null,
      decision.decisionType,
      decision.title,
      decision.description,
      decision.rationale || null,
      JSON.stringify(decision.alternatives || []),
      JSON.stringify(decision.consequences || {}),
      decision.embedding ? JSON.stringify(decision.embedding) : null,
      decision.status || 'active',
    ]);
    return this._formatRow(rows[0]);
  }

  /**
   * Find decision by ID
   * @param {string} id - Decision UUID
   * @returns {Promise<Object|null>}
   */
  async findById(id) {
    const { rows } = await this.db.query(
      'SELECT * FROM architectural_decisions WHERE id = $1',
      [id]
    );
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Search decisions using FTS or hybrid search
   *
   * @param {string} userId - User ID
   * @param {string} query - Search query
   * @param {Object} [options] - Search options
   * @param {number[]} [options.embedding] - Query embedding for vector search
   * @param {string} [options.projectPath] - Filter by project path
   * @param {string[]} [options.decisionTypes] - Filter by decision types
   * @param {string} [options.status='active'] - Filter by status
   * @param {number} [options.limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async search(userId, query, options = {}) {
    const {
      embedding = null,
      projectPath = null,
      decisionTypes = null,
      status = 'active',
      limit = 10,
    } = options;

    const PHI_FTS = 0.382;
    const PHI_VECTOR = 0.618;

    let sql;
    const params = [userId, query];
    let paramIndex = 3;

    if (embedding) {
      // Hybrid search with vector
      sql = `
        WITH fts_results AS (
          SELECT id,
            ts_rank(to_tsvector('english', title || ' ' || COALESCE(description, '')),
                    plainto_tsquery('english', $2)) as fts_score
          FROM architectural_decisions
          WHERE user_id = $1
        ),
        vector_results AS (
          SELECT id, 1 - (embedding <=> $${paramIndex}::vector) as vector_score
          FROM architectural_decisions
          WHERE user_id = $1 AND embedding IS NOT NULL
        )
        SELECT d.*, f.fts_score, v.vector_score,
               (COALESCE(f.fts_score, 0) * ${PHI_FTS} + COALESCE(v.vector_score, 0) * ${PHI_VECTOR}) as combined_score
        FROM architectural_decisions d
        LEFT JOIN fts_results f ON d.id = f.id
        LEFT JOIN vector_results v ON d.id = v.id
        WHERE d.user_id = $1
      `;
      params.push(JSON.stringify(embedding));
      paramIndex++;
    } else {
      // FTS-only search
      sql = `
        SELECT *,
          ts_rank(to_tsvector('english', title || ' ' || COALESCE(description, '')),
                  plainto_tsquery('english', $2)) as fts_score,
          ts_rank(to_tsvector('english', title || ' ' || COALESCE(description, '')),
                  plainto_tsquery('english', $2)) * ${PHI_FTS} as combined_score
        FROM architectural_decisions
        WHERE user_id = $1
          AND to_tsvector('english', title || ' ' || COALESCE(description, '')) @@ plainto_tsquery('english', $2)
      `;
    }

    if (projectPath) {
      sql += ` AND project_path = $${paramIndex++}`;
      params.push(projectPath);
    }

    if (decisionTypes && decisionTypes.length > 0) {
      sql += ` AND decision_type = ANY($${paramIndex++})`;
      params.push(decisionTypes);
    }

    if (status) {
      sql += ` AND status = $${paramIndex++}`;
      params.push(status);
    }

    sql += ` ORDER BY combined_score DESC, created_at DESC LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get decisions by project
   * @param {string} userId - User ID
   * @param {string} projectPath - Project path
   * @param {Object} [options] - Query options
   * @returns {Promise<Object[]>}
   */
  async findByProject(userId, projectPath, options = {}) {
    const { status = 'active', limit = 20 } = options;

    const { rows } = await this.db.query(`
      SELECT * FROM architectural_decisions
      WHERE user_id = $1 AND project_path = $2 AND status = $3
      ORDER BY updated_at DESC
      LIMIT $4
    `, [userId, projectPath, status, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * v1.1: Get decisions from a specific session
   * @param {string} userId - User ID
   * @param {string} sessionId - Session ID
   * @param {number} [limit=20] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findBySession(userId, sessionId, limit = 20) {
    // Note: Decisions may not have session_id, search by metadata or time range
    // For now, return empty as decisions are typically project-scoped
    return [];
  }

  /**
   * Get decisions by type
   * @param {string} userId - User ID
   * @param {string} decisionType - Decision type
   * @param {number} [limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findByType(userId, decisionType, limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM architectural_decisions
      WHERE user_id = $1 AND decision_type = $2 AND status = 'active'
      ORDER BY created_at DESC
      LIMIT $3
    `, [userId, decisionType, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get recent decisions
   * @param {string} userId - User ID
   * @param {number} [limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findRecent(userId, limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM architectural_decisions
      WHERE user_id = $1 AND status = 'active'
      ORDER BY updated_at DESC
      LIMIT $2
    `, [userId, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Supersede a decision with a new one
   * @param {string} oldDecisionId - ID of decision being superseded
   * @param {Object} newDecision - New decision data
   * @returns {Promise<Object>} New decision
   */
  async supersede(oldDecisionId, newDecision) {
    // Create new decision
    const created = await this.create({
      ...newDecision,
      relatedDecisions: [oldDecisionId, ...(newDecision.relatedDecisions || [])],
    });

    // Mark old as superseded
    await this.db.query(`
      UPDATE architectural_decisions
      SET status = 'superseded', superseded_by = $2, updated_at = NOW()
      WHERE id = $1
    `, [oldDecisionId, created.id]);

    return created;
  }

  /**
   * Update a decision
   * @param {string} id - Decision UUID
   * @param {Object} data - Update data
   * @returns {Promise<Object|null>}
   */
  async update(id, data) {
    const updates = [];
    const params = [id];
    let paramIndex = 2;

    if (data.title !== undefined) {
      updates.push(`title = $${paramIndex++}`);
      params.push(data.title);
    }
    if (data.description !== undefined) {
      updates.push(`description = $${paramIndex++}`);
      params.push(data.description);
    }
    if (data.rationale !== undefined) {
      updates.push(`rationale = $${paramIndex++}`);
      params.push(data.rationale);
    }
    if (data.alternatives !== undefined) {
      updates.push(`alternatives = $${paramIndex++}`);
      params.push(JSON.stringify(data.alternatives));
    }
    if (data.consequences !== undefined) {
      updates.push(`consequences = $${paramIndex++}`);
      params.push(JSON.stringify(data.consequences));
    }
    if (data.embedding !== undefined) {
      updates.push(`embedding = $${paramIndex++}`);
      params.push(data.embedding ? JSON.stringify(data.embedding) : null);
    }
    if (data.status !== undefined) {
      updates.push(`status = $${paramIndex++}`);
      params.push(data.status);
    }

    if (updates.length === 0) {
      return this.findById(id);
    }

    // updated_at is handled by trigger
    const { rows } = await this.db.query(`
      UPDATE architectural_decisions
      SET ${updates.join(', ')}
      WHERE id = $1
      RETURNING *
    `, params);

    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Delete a decision
   * @param {string} id - Decision UUID
   * @returns {Promise<boolean>}
   */
  async delete(id) {
    const { rowCount } = await this.db.query(
      'DELETE FROM architectural_decisions WHERE id = $1',
      [id]
    );
    return rowCount > 0;
  }

  /**
   * List decisions with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0, userId, status = 'active' } = options;

    let sql = 'SELECT * FROM architectural_decisions WHERE 1=1';
    const params = [];
    let paramIndex = 1;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }
    if (status) {
      sql += ` AND status = $${paramIndex++}`;
      params.push(status);
    }

    sql += ` ORDER BY updated_at DESC LIMIT $${paramIndex++} OFFSET $${paramIndex}`;
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
        COUNT(*) FILTER (WHERE status = 'active') as active,
        COUNT(*) FILTER (WHERE status = 'superseded') as superseded,
        COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '7 days') as recent
      FROM architectural_decisions
    `;
    const params = [];

    if (userId) {
      sql += ' WHERE user_id = $1';
      params.push(userId);
    }

    const { rows: statsRows } = await this.db.query(sql, params);

    // Get counts by type
    let typeSql = `
      SELECT decision_type, COUNT(*) as count
      FROM architectural_decisions
      WHERE status = 'active'
    `;
    if (userId) {
      typeSql += ' AND user_id = $1';
    }
    typeSql += ' GROUP BY decision_type ORDER BY count DESC';

    const { rows: typeRows } = await this.db.query(typeSql, params);

    const stats = statsRows[0];
    return {
      total: parseInt(stats.total),
      active: parseInt(stats.active),
      superseded: parseInt(stats.superseded),
      recent: parseInt(stats.recent),
      byType: Object.fromEntries(typeRows.map(r => [r.decision_type, parseInt(r.count)])),
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
      projectPath: row.project_path,
      decisionType: row.decision_type,
      title: row.title,
      description: row.description,
      rationale: row.rationale,
      alternatives: row.alternatives || [],
      consequences: row.consequences || {},
      embedding: row.embedding,
      status: row.status,
      supersededBy: row.superseded_by,
      relatedDecisions: row.related_decisions || [],
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      // Search result fields
      combinedScore: row.combined_score,
      ftsScore: row.fts_score,
      vectorScore: row.vector_score,
    };
  }
}

export default ArchitecturalDecisionsRepository;
