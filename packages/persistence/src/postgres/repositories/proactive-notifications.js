/**
 * Proactive Notifications Repository
 *
 * Notifications delivered at session start.
 * Part of CYNIC's Full Autonomy system.
 *
 * @module @cynic/persistence/repositories/proactive-notifications
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Notification types
 * @enum {string}
 */
export const NotificationType = {
  INSIGHT: 'insight',
  WARNING: 'warning',
  REMINDER: 'reminder',
  ACHIEVEMENT: 'achievement',
  PATTERN: 'pattern',
  SUGGESTION: 'suggestion',
  LEARNING: 'learning',
  QUESTION: 'question',
};

/**
 * Proactive Notifications Repository
 *
 * @extends BaseRepository
 */
export class ProactiveNotificationsRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Create a new notification
   *
   * @param {Object} notification - Notification data
   * @param {string} notification.userId - User ID
   * @param {string} notification.notificationType - Type of notification
   * @param {string} notification.title - Notification title
   * @param {string} notification.message - Notification message
   * @param {number} [notification.priority=50] - Priority (0-100)
   * @param {Object} [notification.context] - Additional context
   * @param {Date} [notification.expiresAt] - Expiration date
   * @returns {Promise<Object>} Created notification
   */
  async create(notification) {
    const { rows } = await this.db.query(`
      INSERT INTO proactive_notifications (
        user_id, notification_type, title, message,
        priority, context, expires_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
      RETURNING *
    `, [
      notification.userId,
      notification.notificationType,
      notification.title,
      notification.message,
      notification.priority ?? 50,
      JSON.stringify(notification.context || {}),
      notification.expiresAt || null,
    ]);
    return this._formatRow(rows[0]);
  }

  /**
   * Find notification by ID
   * @param {string} id - Notification UUID
   * @returns {Promise<Object|null>}
   */
  async findById(id) {
    const { rows } = await this.db.query(
      'SELECT * FROM proactive_notifications WHERE id = $1',
      [id]
    );
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Get pending notifications for user (for session start)
   * @param {string} userId - User ID
   * @param {number} [limit=5] - Maximum notifications
   * @returns {Promise<Object[]>}
   */
  async getPending(userId, limit = 5) {
    const { rows } = await this.db.query(`
      SELECT * FROM get_pending_notifications($1, $2)
    `, [userId, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Mark notification as delivered
   * @param {string} notificationId - Notification UUID
   * @returns {Promise<Object|null>}
   */
  async markDelivered(notificationId) {
    const { rows } = await this.db.query(`
      UPDATE proactive_notifications
      SET delivered = TRUE, delivered_at = NOW()
      WHERE id = $1
      RETURNING *
    `, [notificationId]);
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Mark multiple notifications as delivered
   * @param {string[]} notificationIds - Notification UUIDs
   * @returns {Promise<number>} Number of notifications updated
   */
  async markMultipleDelivered(notificationIds) {
    if (!notificationIds || notificationIds.length === 0) return 0;

    const { rowCount } = await this.db.query(`
      UPDATE proactive_notifications
      SET delivered = TRUE, delivered_at = NOW()
      WHERE id = ANY($1)
    `, [notificationIds]);
    return rowCount;
  }

  /**
   * Dismiss a notification
   * @param {string} notificationId - Notification UUID
   * @param {string} [actionTaken] - What action user took
   * @returns {Promise<Object|null>}
   */
  async dismiss(notificationId, actionTaken = null) {
    const { rows } = await this.db.query(`
      UPDATE proactive_notifications
      SET dismissed = TRUE,
          dismissed_at = NOW(),
          action_taken = $2
      WHERE id = $1
      RETURNING *
    `, [notificationId, actionTaken]);
    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Get notifications by type
   * @param {string} userId - User ID
   * @param {string} notificationType - Notification type
   * @param {Object} [options] - Query options
   * @returns {Promise<Object[]>}
   */
  async findByType(userId, notificationType, options = {}) {
    const { includeDelivered = false, limit = 10 } = options;

    let sql = `
      SELECT * FROM proactive_notifications
      WHERE user_id = $1 AND notification_type = $2
    `;
    const params = [userId, notificationType];
    let paramIndex = 3;

    if (!includeDelivered) {
      sql += ' AND delivered = FALSE';
    }

    sql += ` AND (expires_at IS NULL OR expires_at > NOW())
             ORDER BY priority DESC, created_at DESC
             LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get recent notifications (including delivered)
   * @param {string} userId - User ID
   * @param {number} [limit=20] - Maximum notifications
   * @returns {Promise<Object[]>}
   */
  async findRecent(userId, limit = 20) {
    const { rows } = await this.db.query(`
      SELECT * FROM proactive_notifications
      WHERE user_id = $1
      ORDER BY created_at DESC
      LIMIT $2
    `, [userId, limit]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Get notification history (delivered only)
   * @param {string} userId - User ID
   * @param {number} [days=7] - Days of history
   * @returns {Promise<Object[]>}
   */
  async getHistory(userId, days = 7) {
    const { rows } = await this.db.query(`
      SELECT * FROM proactive_notifications
      WHERE user_id = $1
        AND delivered = TRUE
        AND delivered_at > NOW() - $2 * INTERVAL '1 day'
      ORDER BY delivered_at DESC
    `, [userId, days]);
    return rows.map(r => this._formatRow(r));
  }

  /**
   * Update a notification
   * @param {string} id - Notification UUID
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
    if (data.message !== undefined) {
      updates.push(`message = $${paramIndex++}`);
      params.push(data.message);
    }
    if (data.priority !== undefined) {
      updates.push(`priority = $${paramIndex++}`);
      params.push(data.priority);
    }
    if (data.context !== undefined) {
      updates.push(`context = $${paramIndex++}`);
      params.push(JSON.stringify(data.context));
    }
    if (data.expiresAt !== undefined) {
      updates.push(`expires_at = $${paramIndex++}`);
      params.push(data.expiresAt);
    }

    if (updates.length === 0) {
      return this.findById(id);
    }

    const { rows } = await this.db.query(`
      UPDATE proactive_notifications
      SET ${updates.join(', ')}
      WHERE id = $1
      RETURNING *
    `, params);

    return rows[0] ? this._formatRow(rows[0]) : null;
  }

  /**
   * Delete a notification
   * @param {string} id - Notification UUID
   * @returns {Promise<boolean>}
   */
  async delete(id) {
    const { rowCount } = await this.db.query(
      'DELETE FROM proactive_notifications WHERE id = $1',
      [id]
    );
    return rowCount > 0;
  }

  /**
   * Delete expired notifications
   * @returns {Promise<number>} Number deleted
   */
  async cleanupExpired() {
    const { rowCount } = await this.db.query(`
      DELETE FROM proactive_notifications
      WHERE expires_at < NOW() AND delivered = FALSE
    `);
    return rowCount;
  }

  /**
   * List notifications with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0, userId, notificationType } = options;

    let sql = 'SELECT * FROM proactive_notifications WHERE 1=1';
    const params = [];
    let paramIndex = 1;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }
    if (notificationType) {
      sql += ` AND notification_type = $${paramIndex++}`;
      params.push(notificationType);
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
        COUNT(*) FILTER (WHERE delivered = FALSE) as pending,
        COUNT(*) FILTER (WHERE delivered = TRUE) as delivered,
        COUNT(*) FILTER (WHERE dismissed = TRUE) as dismissed,
        COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '24 hours') as recent
      FROM proactive_notifications
    `;
    const params = [];

    if (userId) {
      sql += ' WHERE user_id = $1';
      params.push(userId);
    }

    const { rows: statsRows } = await this.db.query(sql, params);

    // Get counts by type
    let typeSql = `
      SELECT notification_type, COUNT(*) as count
      FROM proactive_notifications
    `;
    if (userId) {
      typeSql += ' WHERE user_id = $1';
    }
    typeSql += ' GROUP BY notification_type ORDER BY count DESC';

    const { rows: typeRows } = await this.db.query(typeSql, params);

    const stats = statsRows[0];
    return {
      total: parseInt(stats.total),
      pending: parseInt(stats.pending),
      delivered: parseInt(stats.delivered),
      dismissed: parseInt(stats.dismissed),
      recent: parseInt(stats.recent),
      byType: Object.fromEntries(typeRows.map(r => [r.notification_type, parseInt(r.count)])),
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
      notificationType: row.notification_type,
      title: row.title,
      message: row.message,
      priority: row.priority,
      context: row.context || {},
      delivered: row.delivered,
      deliveredAt: row.delivered_at,
      expiresAt: row.expires_at,
      dismissed: row.dismissed,
      dismissedAt: row.dismissed_at,
      actionTaken: row.action_taken,
      createdAt: row.created_at,
    };
  }
}

export default ProactiveNotificationsRepository;
