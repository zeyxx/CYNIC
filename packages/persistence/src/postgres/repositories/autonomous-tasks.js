/**
 * Autonomous Tasks Repository
 *
 * Durable task queue that survives restarts.
 * Part of CYNIC's Full Autonomy system.
 *
 * @module @cynic/persistence/repositories/autonomous-tasks
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Task status
 * @enum {string}
 */
export const TaskStatus = {
  PENDING: 'pending',
  RUNNING: 'running',
  COMPLETED: 'completed',
  FAILED: 'failed',
  RETRY: 'retry',
  CANCELLED: 'cancelled',
};

/**
 * Common task types
 * @enum {string}
 */
export const TaskType = {
  ANALYZE_PATTERNS: 'analyze_patterns',
  RUN_TESTS: 'run_tests',
  SECURITY_SCAN: 'security_scan',
  CODE_REVIEW: 'code_review',
  GENERATE_SUMMARY: 'generate_summary',
  UPDATE_DOCS: 'update_docs',
  CLEANUP: 'cleanup',
  NOTIFY: 'notify',
  SYNC: 'sync',
  CUSTOM: 'custom',
};

/**
 * Autonomous Tasks Repository
 *
 * @extends BaseRepository
 */
export class AutonomousTasksRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Create a new task
   *
   * @param {Object} task - Task data
   * @param {string} task.userId - User ID
   * @param {string} [task.goalId] - Associated goal ID
   * @param {string} task.taskType - Type of task
   * @param {Object} task.payload - Task-specific data
   * @param {number} [task.priority=50] - Priority (0-100)
   * @param {Date} [task.scheduledFor] - When to execute (default: now)
   * @param {number} [task.maxRetries=3] - Maximum retry attempts
   * @param {string} [task.createdBy='daemon'] - Who created this task
   * @returns {Promise<Object>} Created task
   */
  async create(task) {
    const { rows } = await this.db.query(`
      INSERT INTO autonomous_tasks (
        user_id, goal_id, task_type, payload,
        priority, scheduled_for, max_retries, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      RETURNING *
    `, [
      task.userId,
      task.goalId || null,
      task.taskType,
      JSON.stringify(task.payload || {}),
      task.priority ?? 50,
      task.scheduledFor || new Date(),
      task.maxRetries ?? 3,
      task.createdBy || 'daemon',
    ]);
    return this._formatRow(rows[0]);
  }

  /**
   * Find task by ID
   * @param {string} id - Task UUID
   * @returns {Promise<Object|null>}
   */
  async findById(id) {
    const { rows } = await this.db.query(
      'SELECT * FROM autonomous_tasks WHERE id = $1',
      [id]
    );
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Get pending tasks ready for execution
   * @param {number} [limit=10] - Maximum tasks to fetch
   * @returns {Promise<Object[]>}
   */
  async getPending(limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM get_pending_tasks($1)
    `, [limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Claim a task for execution (atomically set status to running)
   * @param {string} taskId - Task UUID
   * @returns {Promise<Object|null>} Task if successfully claimed
   */
  async claim(taskId) {
    const { rows } = await this.db.query(`
      UPDATE autonomous_tasks
      SET status = 'running', started_at = NOW()
      WHERE id = $1 AND status = 'pending'
      RETURNING *
    `, [taskId]);
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Mark task as completed
   * @param {string} taskId - Task UUID
   * @param {Object} [result] - Task result
   * @returns {Promise<Object|null>}
   */
  async complete(taskId, result = null) {
    const { rows } = await this.db.query(`
      UPDATE autonomous_tasks
      SET status = 'completed',
          completed_at = NOW(),
          result = $2
      WHERE id = $1
      RETURNING *
    `, [taskId, result ? JSON.stringify(result) : null]);
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Mark task as failed
   * @param {string} taskId - Task UUID
   * @param {string} errorMessage - Error message
   * @returns {Promise<Object|null>}
   */
  async fail(taskId, errorMessage) {
    // Check if we should retry
    const task = await this.findById(taskId);
    if (!task) return null;

    const shouldRetry = task.retryCount < task.maxRetries;
    const newStatus = shouldRetry ? 'retry' : 'failed';

    // Fibonacci backoff for retry delay: 1, 2, 3, 5, 8, 13... minutes
    const FIBONACCI = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89];
    const delayMinutes = FIBONACCI[Math.min(task.retryCount, FIBONACCI.length - 1)];

    const { rows } = await this.db.query(`
      UPDATE autonomous_tasks
      SET status = $2,
          error_message = $3,
          retry_count = retry_count + 1,
          scheduled_for = CASE
            WHEN $2 = 'retry' THEN NOW() + $4 * INTERVAL '1 minute'
            ELSE scheduled_for
          END,
          completed_at = CASE
            WHEN $2 = 'failed' THEN NOW()
            ELSE NULL
          END
      WHERE id = $1
      RETURNING *
    `, [taskId, newStatus, errorMessage, delayMinutes]);
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Cancel a task
   * @param {string} taskId - Task UUID
   * @returns {Promise<Object|null>}
   */
  async cancel(taskId) {
    const { rows } = await this.db.query(`
      UPDATE autonomous_tasks
      SET status = 'cancelled', completed_at = NOW()
      WHERE id = $1 AND status IN ('pending', 'retry')
      RETURNING *
    `, [taskId]);
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Get tasks by goal
   * @param {string} goalId - Goal UUID
   * @returns {Promise<Object[]>}
   */
  async findByGoal(goalId) {
    const { rows } = await this.db.query(`
      SELECT * FROM autonomous_tasks
      WHERE goal_id = $1
      ORDER BY created_at DESC
    `, [goalId]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get tasks by user
   * @param {string} userId - User ID
   * @param {Object} [options] - Query options
   * @returns {Promise<Object[]>}
   */
  async findByUser(userId, options = {}) {
    const { status, limit = 20 } = options;

    let sql = 'SELECT * FROM autonomous_tasks WHERE user_id = $1';
    const params = [userId];
    let paramIndex = 2;

    if (status) {
      sql += ` AND status = $${paramIndex++}`;
      params.push(status);
    }

    sql += ` ORDER BY created_at DESC LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get running tasks (for monitoring)
   * @returns {Promise<Object[]>}
   */
  async getRunning() {
    const { rows } = await this.db.query(`
      SELECT * FROM autonomous_tasks
      WHERE status = 'running'
      ORDER BY started_at ASC
    `);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get stuck tasks (running for too long)
   * @param {number} [timeoutMinutes=30] - Timeout threshold
   * @returns {Promise<Object[]>}
   */
  async getStuck(timeoutMinutes = 30) {
    const { rows } = await this.db.query(`
      SELECT * FROM autonomous_tasks
      WHERE status = 'running'
        AND started_at < NOW() - $1 * INTERVAL '1 minute'
      ORDER BY started_at ASC
    `, [timeoutMinutes]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Reset stuck tasks to pending
   * @param {number} [timeoutMinutes=30] - Timeout threshold
   * @returns {Promise<number>} Number of tasks reset
   */
  async resetStuck(timeoutMinutes = 30) {
    const { rowCount } = await this.db.query(`
      UPDATE autonomous_tasks
      SET status = 'retry',
          error_message = 'Task timed out',
          retry_count = retry_count + 1,
          scheduled_for = NOW()
      WHERE status = 'running'
        AND started_at < NOW() - $1 * INTERVAL '1 minute'
    `, [timeoutMinutes]);
    return rowCount;
  }

  /**
   * Update a task
   * @param {string} id - Task UUID
   * @param {Object} data - Update data
   * @returns {Promise<Object|null>}
   */
  async update(id, data) {
    const updates = [];
    const params = [id];
    let paramIndex = 2;

    if (data.status !== undefined) {
      updates.push(`status = $${paramIndex++}`);
      params.push(data.status);
    }
    if (data.priority !== undefined) {
      updates.push(`priority = $${paramIndex++}`);
      params.push(data.priority);
    }
    if (data.scheduledFor !== undefined) {
      updates.push(`scheduled_for = $${paramIndex++}`);
      params.push(data.scheduledFor);
    }
    if (data.payload !== undefined) {
      updates.push(`payload = $${paramIndex++}`);
      params.push(JSON.stringify(data.payload));
    }
    if (data.result !== undefined) {
      updates.push(`result = $${paramIndex++}`);
      params.push(JSON.stringify(data.result));
    }

    if (updates.length === 0) {
      return this.findById(id);
    }

    const { rows } = await this.db.query(`
      UPDATE autonomous_tasks
      SET ${updates.join(', ')}
      WHERE id = $1
      RETURNING *
    `, params);

    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Delete a task
   * @param {string} id - Task UUID
   * @returns {Promise<boolean>}
   */
  async delete(id) {
    const { rowCount } = await this.db.query(
      'DELETE FROM autonomous_tasks WHERE id = $1',
      [id]
    );
    return rowCount > 0;
  }

  /**
   * List tasks with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0, userId, status } = options;

    let sql = 'SELECT * FROM autonomous_tasks WHERE 1=1';
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
        COUNT(*) FILTER (WHERE status = 'pending') as pending,
        COUNT(*) FILTER (WHERE status = 'running') as running,
        COUNT(*) FILTER (WHERE status = 'completed') as completed,
        COUNT(*) FILTER (WHERE status = 'failed') as failed,
        COUNT(*) FILTER (WHERE status = 'retry') as retrying
      FROM autonomous_tasks
    `;
    const params = [];

    if (userId) {
      sql += ' WHERE user_id = $1';
      params.push(userId);
    }

    const { rows: statsRows } = await this.db.query(sql, params);

    // Get counts by type
    let typeSql = 'SELECT task_type, COUNT(*) as count FROM autonomous_tasks';
    if (userId) {
      typeSql += ' WHERE user_id = $1';
    }
    typeSql += ' GROUP BY task_type ORDER BY count DESC';

    const { rows: typeRows } = await this.db.query(typeSql, params);

    const stats = statsRows[0];
    return {
      total: parseInt(stats.total),
      pending: parseInt(stats.pending),
      running: parseInt(stats.running),
      completed: parseInt(stats.completed),
      failed: parseInt(stats.failed),
      retrying: parseInt(stats.retrying),
      byType: Object.fromEntries(typeRows.map(r => [r.task_type, parseInt(r.count)])),
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
      goalId: row.goal_id,
      userId: row.user_id,
      taskType: row.task_type,
      payload: row.payload || {},
      status: row.status,
      priority: row.priority,
      scheduledFor: row.scheduled_for,
      startedAt: row.started_at,
      completedAt: row.completed_at,
      retryCount: row.retry_count,
      maxRetries: row.max_retries,
      errorMessage: row.error_message,
      result: row.result,
      createdBy: row.created_by,
      createdAt: row.created_at,
    };
  }
}

export default AutonomousTasksRepository;
