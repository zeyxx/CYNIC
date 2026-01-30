/**
 * Autonomous Goals Repository
 *
 * Persistent goals that CYNIC pursues autonomously.
 * Part of CYNIC's Full Autonomy system.
 *
 * @module @cynic/persistence/repositories/autonomous-goals
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Goal types
 * @enum {string}
 */
export const GoalType = {
  QUALITY: 'quality',
  LEARNING: 'learning',
  MAINTENANCE: 'maintenance',
  MONITORING: 'monitoring',
  SECURITY: 'security',
  DOCUMENTATION: 'documentation',
  PERFORMANCE: 'performance',
  CUSTOM: 'custom',
};

/**
 * Goal status
 * @enum {string}
 */
export const GoalStatus = {
  ACTIVE: 'active',
  PAUSED: 'paused',
  COMPLETED: 'completed',
  ABANDONED: 'abandoned',
  BLOCKED: 'blocked',
};

/**
 * Autonomous Goals Repository
 *
 * @extends BaseRepository
 */
export class AutonomousGoalsRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Create a new goal
   *
   * @param {Object} goal - Goal data
   * @param {string} goal.userId - User ID
   * @param {string} goal.goalType - Type of goal
   * @param {string} goal.title - Goal title
   * @param {string} [goal.description] - Goal description
   * @param {Object[]} [goal.successCriteria] - Success criteria
   * @param {number} [goal.priority=50] - Priority (0-100)
   * @param {Object} [goal.config] - Goal configuration
   * @param {Date} [goal.dueAt] - Optional deadline
   * @returns {Promise<Object>} Created goal
   */
  async create(goal) {
    const { rows } = await this.db.query(`
      INSERT INTO autonomous_goals (
        user_id, goal_type, title, description,
        success_criteria, priority, config, due_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      RETURNING *
    `, [
      goal.userId,
      goal.goalType,
      goal.title,
      goal.description || null,
      JSON.stringify(goal.successCriteria || []),
      goal.priority ?? 50,
      JSON.stringify(goal.config || {}),
      goal.dueAt || null,
    ]);
    return this._formatRow(rows[0]);
  }

  /**
   * Find goal by ID
   * @param {string} id - Goal UUID
   * @returns {Promise<Object|null>}
   */
  async findById(id) {
    const { rows } = await this.db.query(
      'SELECT * FROM autonomous_goals WHERE id = $1',
      [id]
    );
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Get active goals for user
   * @param {string} userId - User ID
   * @param {number} [limit=10] - Maximum results
   * @returns {Promise<Object[]>}
   */
  async findActive(userId, limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM autonomous_goals
      WHERE user_id = $1 AND status = 'active'
      ORDER BY priority DESC, created_at ASC
      LIMIT $2
    `, [userId, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get goals by type
   * @param {string} userId - User ID
   * @param {string} goalType - Goal type
   * @param {Object} [options] - Query options
   * @returns {Promise<Object[]>}
   */
  async findByType(userId, goalType, options = {}) {
    const { status = 'active', limit = 10 } = options;

    const { rows } = await this.db.query(`
      SELECT * FROM autonomous_goals
      WHERE user_id = $1 AND goal_type = $2 AND status = $3
      ORDER BY priority DESC, created_at ASC
      LIMIT $4
    `, [userId, goalType, status, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get goals nearing deadline
   * @param {string} userId - User ID
   * @param {number} [withinDays=7] - Days until deadline
   * @returns {Promise<Object[]>}
   */
  async findDueSoon(userId, withinDays = 7) {
    const { rows } = await this.db.query(`
      SELECT * FROM autonomous_goals
      WHERE user_id = $1
        AND status = 'active'
        AND due_at IS NOT NULL
        AND due_at <= NOW() + $2 * INTERVAL '1 day'
      ORDER BY due_at ASC
    `, [userId, withinDays]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Update goal progress
   * @param {string} goalId - Goal UUID
   * @param {number} progress - Progress value (0-1)
   * @param {string} [note] - Progress note
   * @returns {Promise<Object|null>} Updated goal
   */
  async updateProgress(goalId, progress, note = null) {
    const { rows } = await this.db.query(`
      UPDATE autonomous_goals
      SET progress = $2,
          updated_at = NOW()
      WHERE id = $1
      RETURNING *
    `, [goalId, progress]);
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Update goal status
   * @param {string} goalId - Goal UUID
   * @param {string} status - New status
   * @returns {Promise<Object|null>}
   */
  async updateStatus(goalId, status) {
    const { rows } = await this.db.query(`
      UPDATE autonomous_goals
      SET status = $2,
          completed_at = CASE WHEN $2 = 'completed' THEN NOW() ELSE completed_at END
      WHERE id = $1
      RETURNING *
    `, [goalId, status]);
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Update a goal
   * @param {string} id - Goal UUID
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
    if (data.successCriteria !== undefined) {
      updates.push(`success_criteria = $${paramIndex++}`);
      params.push(JSON.stringify(data.successCriteria));
    }
    if (data.progress !== undefined) {
      updates.push(`progress = $${paramIndex++}`);
      params.push(data.progress);
    }
    if (data.status !== undefined) {
      updates.push(`status = $${paramIndex++}`);
      params.push(data.status);
    }
    if (data.priority !== undefined) {
      updates.push(`priority = $${paramIndex++}`);
      params.push(data.priority);
    }
    if (data.config !== undefined) {
      updates.push(`config = $${paramIndex++}`);
      params.push(JSON.stringify(data.config));
    }
    if (data.dueAt !== undefined) {
      updates.push(`due_at = $${paramIndex++}`);
      params.push(data.dueAt);
    }

    if (updates.length === 0) {
      return this.findById(id);
    }

    // updated_at handled by trigger
    const { rows } = await this.db.query(`
      UPDATE autonomous_goals
      SET ${updates.join(', ')}
      WHERE id = $1
      RETURNING *
    `, params);

    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Delete a goal
   * @param {string} id - Goal UUID
   * @returns {Promise<boolean>}
   */
  async delete(id) {
    const { rowCount } = await this.db.query(
      'DELETE FROM autonomous_goals WHERE id = $1',
      [id]
    );
    return rowCount > 0;
  }

  /**
   * List goals with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0, userId, status } = options;

    let sql = 'SELECT * FROM autonomous_goals WHERE 1=1';
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

    sql += ` ORDER BY priority DESC, created_at DESC LIMIT $${paramIndex++} OFFSET $${paramIndex}`;
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
        COUNT(*) FILTER (WHERE status = 'completed') as completed,
        COUNT(*) FILTER (WHERE status = 'paused') as paused,
        AVG(progress) FILTER (WHERE status = 'active') as avg_progress
      FROM autonomous_goals
    `;
    const params = [];

    if (userId) {
      sql += ' WHERE user_id = $1';
      params.push(userId);
    }

    const { rows: statsRows } = await this.db.query(sql, params);

    // Get counts by type
    let typeSql = 'SELECT goal_type, COUNT(*) as count FROM autonomous_goals';
    if (userId) {
      typeSql += ' WHERE user_id = $1';
    }
    typeSql += ' GROUP BY goal_type ORDER BY count DESC';

    const { rows: typeRows } = await this.db.query(typeSql, params);

    const stats = statsRows[0];
    return {
      total: parseInt(stats.total),
      active: parseInt(stats.active),
      completed: parseInt(stats.completed),
      paused: parseInt(stats.paused),
      avgProgress: parseFloat(stats.avg_progress) || 0,
      byType: Object.fromEntries(typeRows.map(r => [r.goal_type, parseInt(r.count)])),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // v1.1: MILESTONE TRACKING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * v1.1: Add a milestone to a goal
   *
   * @param {string} goalId - Goal UUID
   * @param {Object} milestone - Milestone data
   * @param {string} milestone.title - Milestone title
   * @param {number} milestone.targetProgress - Progress when milestone is reached (0-1)
   * @param {string} [milestone.description] - Milestone description
   * @returns {Promise<Object|null>} Updated goal
   */
  async addMilestone(goalId, milestone) {
    const goal = await this.findById(goalId);
    if (!goal) return null;

    const milestones = goal.config?.milestones || [];
    milestones.push({
      id: `ms_${Date.now().toString(36)}`,
      title: milestone.title,
      description: milestone.description || null,
      targetProgress: milestone.targetProgress,
      reachedAt: null,
      createdAt: new Date().toISOString(),
    });

    // Sort by target progress
    milestones.sort((a, b) => a.targetProgress - b.targetProgress);

    return this.update(goalId, {
      config: { ...goal.config, milestones },
    });
  }

  /**
   * v1.1: Mark a milestone as reached
   *
   * @param {string} goalId - Goal UUID
   * @param {string} milestoneId - Milestone ID
   * @returns {Promise<Object|null>} Updated goal
   */
  async completeMilestone(goalId, milestoneId) {
    const goal = await this.findById(goalId);
    if (!goal) return null;

    const milestones = goal.config?.milestones || [];
    const milestone = milestones.find(m => m.id === milestoneId);
    if (!milestone) return goal;

    milestone.reachedAt = new Date().toISOString();

    return this.update(goalId, {
      config: { ...goal.config, milestones },
    });
  }

  /**
   * v1.1: Get goals with pending milestones
   *
   * @param {string} userId - User ID
   * @returns {Promise<Object[]>} Goals with unreached milestones at or below current progress
   */
  async findWithPendingMilestones(userId) {
    const activeGoals = await this.findActive(userId, 50);

    return activeGoals.filter(goal => {
      const milestones = goal.config?.milestones || [];
      return milestones.some(m =>
        !m.reachedAt && m.targetProgress <= goal.progress,
      );
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // v1.1: PROGRESS HISTORY
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * v1.1: Record progress with note (history tracking)
   *
   * @param {string} goalId - Goal UUID
   * @param {number} progress - Progress value (0-1)
   * @param {string} [note] - Progress note
   * @returns {Promise<Object|null>} Updated goal
   */
  async recordProgress(goalId, progress, note = null) {
    const goal = await this.findById(goalId);
    if (!goal) return null;

    // Add to progress history
    const progressNotes = goal.progressNotes || [];
    progressNotes.push({
      timestamp: new Date().toISOString(),
      progress,
      previousProgress: goal.progress,
      note,
    });

    // Keep only last 50 entries
    if (progressNotes.length > 50) {
      progressNotes.splice(0, progressNotes.length - 50);
    }

    // Check for milestone completions
    const milestones = goal.config?.milestones || [];
    for (const m of milestones) {
      if (!m.reachedAt && m.targetProgress <= progress) {
        m.reachedAt = new Date().toISOString();
      }
    }

    const { rows } = await this.db.query(`
      UPDATE autonomous_goals
      SET progress = $2,
          progress_notes = $3,
          config = $4
      WHERE id = $1
      RETURNING *
    `, [
      goalId,
      progress,
      JSON.stringify(progressNotes),
      JSON.stringify({ ...goal.config, milestones }),
    ]);

    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * v1.1: Get progress history for a goal
   *
   * @param {string} goalId - Goal UUID
   * @returns {Promise<Object[]>} Progress history entries
   */
  async getProgressHistory(goalId) {
    const goal = await this.findById(goalId);
    return goal?.progressNotes || [];
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // v1.1: AUTO-SUGGESTIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * v1.1: Suggest goals based on patterns and history
   *
   * @param {string} userId - User ID
   * @returns {Promise<Object[]>} Suggested goals
   */
  async suggestGoals(userId) {
    const suggestions = [];

    // Check for goal types without active goals
    const stats = await this.getStats(userId);
    const activeByType = {};

    // Get active goals by type
    const { rows: activeRows } = await this.db.query(`
      SELECT goal_type, COUNT(*) as count
      FROM autonomous_goals
      WHERE user_id = $1 AND status = 'active'
      GROUP BY goal_type
    `, [userId]);

    for (const row of activeRows) {
      activeByType[row.goal_type] = parseInt(row.count);
    }

    // Suggest quality goal if none active
    if (!activeByType[GoalType.QUALITY]) {
      suggestions.push({
        type: 'missing_type',
        goalType: GoalType.QUALITY,
        title: 'Maintain Code Quality',
        description: 'No active quality goals. Consider adding one.',
        priority: 70,
      });
    }

    // Suggest learning goal if none active
    if (!activeByType[GoalType.LEARNING]) {
      suggestions.push({
        type: 'missing_type',
        goalType: GoalType.LEARNING,
        title: 'Learn New Skills',
        description: 'No active learning goals. Consider adding one.',
        priority: 60,
      });
    }

    // Check for stalled goals (no progress in 7 days)
    const { rows: stalledRows } = await this.db.query(`
      SELECT * FROM autonomous_goals
      WHERE user_id = $1
        AND status = 'active'
        AND progress < 1
        AND updated_at < NOW() - INTERVAL '7 days'
      ORDER BY priority DESC
      LIMIT 5
    `, [userId]);

    for (const row of stalledRows) {
      const goal = this._formatRow(row);
      suggestions.push({
        type: 'stalled',
        goalId: goal.id,
        title: goal.title,
        description: `Goal "${goal.title}" has not progressed in 7+ days.`,
        priority: 80,
        suggestedAction: 'Review and update progress, or pause/abandon if no longer relevant.',
      });
    }

    // Check for overdue goals
    const { rows: overdueRows } = await this.db.query(`
      SELECT * FROM autonomous_goals
      WHERE user_id = $1
        AND status = 'active'
        AND due_at IS NOT NULL
        AND due_at < NOW()
      ORDER BY due_at ASC
      LIMIT 5
    `, [userId]);

    for (const row of overdueRows) {
      const goal = this._formatRow(row);
      suggestions.push({
        type: 'overdue',
        goalId: goal.id,
        title: goal.title,
        description: `Goal "${goal.title}" is past its deadline.`,
        priority: 90,
        suggestedAction: 'Complete urgently, extend deadline, or abandon.',
      });
    }

    // Sort by priority descending
    suggestions.sort((a, b) => b.priority - a.priority);

    return suggestions;
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
      goalType: row.goal_type,
      title: row.title,
      description: row.description,
      successCriteria: row.success_criteria || [],
      progress: parseFloat(row.progress),
      progressNotes: row.progress_notes || [],
      status: row.status,
      priority: row.priority,
      config: row.config || {},
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      completedAt: row.completed_at,
      dueAt: row.due_at,
    };
  }
}

export default AutonomousGoalsRepository;
