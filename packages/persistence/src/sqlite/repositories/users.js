/**
 * SQLite Users Repository
 *
 * SQLite-compatible implementation of UserRepository.
 *
 * @module @cynic/persistence/sqlite/repositories/users
 */

'use strict';

import crypto from 'crypto';
import { BaseRepository } from '../../interfaces/IRepository.js';

function generateUserId() {
  return crypto.randomUUID();
}

/**
 * SQLite Users Repository
 */
export class SQLiteUserRepository extends BaseRepository {
  constructor(db) {
    super(db);
  }

  /**
   * Create a new user
   */
  async create(user) {
    const userId = user.id || generateUserId();

    const { rows } = await this.db.query(`
      INSERT INTO users (
        id, email, username, display_name,
        burn_amount, e_score, uptime_hours
      ) VALUES (?, ?, ?, ?, ?, ?, ?)
      RETURNING *
    `, [
      userId,
      user.email || null,
      user.username || null,
      user.displayName || user.display_name || null,
      user.burnAmount || user.burn_amount || 0,
      user.eScore || user.e_score || 0,
      user.uptimeHours || user.uptime_hours || 0,
    ]);

    return rows[0];
  }

  /**
   * Find user by ID
   */
  async findById(userId) {
    const { rows } = await this.db.query(
      'SELECT * FROM users WHERE id = ?',
      [userId]
    );
    return rows[0] || null;
  }

  /**
   * Find user by email
   */
  async findByEmail(email) {
    const { rows } = await this.db.query(
      'SELECT * FROM users WHERE email = ?',
      [email]
    );
    return rows[0] || null;
  }

  /**
   * Find user by username
   */
  async findByUsername(username) {
    const { rows } = await this.db.query(
      'SELECT * FROM users WHERE username = ?',
      [username]
    );
    return rows[0] || null;
  }

  /**
   * Update user
   */
  async update(userId, data) {
    const sets = [];
    const params = [];

    if (data.email !== undefined) {
      sets.push('email = ?');
      params.push(data.email);
    }
    if (data.username !== undefined) {
      sets.push('username = ?');
      params.push(data.username);
    }
    if (data.displayName !== undefined || data.display_name !== undefined) {
      sets.push('display_name = ?');
      params.push(data.displayName || data.display_name);
    }
    if (data.burnAmount !== undefined || data.burn_amount !== undefined) {
      sets.push('burn_amount = ?');
      params.push(data.burnAmount || data.burn_amount);
    }
    if (data.eScore !== undefined || data.e_score !== undefined) {
      sets.push('e_score = ?');
      params.push(data.eScore || data.e_score);
    }
    if (data.uptimeHours !== undefined || data.uptime_hours !== undefined) {
      sets.push('uptime_hours = ?');
      params.push(data.uptimeHours || data.uptime_hours);
    }

    sets.push("updated_at = datetime('now')");

    if (sets.length === 1) {
      return this.findById(userId);
    }

    params.push(userId);

    const { rows } = await this.db.query(`
      UPDATE users SET ${sets.join(', ')} WHERE id = ?
      RETURNING *
    `, params);

    return rows[0] || null;
  }

  /**
   * Update last active timestamp
   */
  async updateLastActive(userId) {
    const { rows } = await this.db.query(`
      UPDATE users
      SET last_active = datetime('now'),
          updated_at = datetime('now')
      WHERE id = ?
      RETURNING *
    `, [userId]);

    return rows[0] || null;
  }

  /**
   * Delete user
   */
  async delete(userId) {
    const { rowCount } = await this.db.query(
      'DELETE FROM users WHERE id = ?',
      [userId]
    );
    return rowCount > 0;
  }

  /**
   * List users
   */
  async list(options = {}) {
    const { limit = 10, offset = 0 } = options;

    const { rows } = await this.db.query(
      'SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?',
      [limit, offset]
    );

    return rows;
  }

  /**
   * Get statistics
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total,
        SUM(burn_amount) as total_burn,
        AVG(e_score) as avg_e_score
      FROM users
    `);

    const stats = rows[0];
    return {
      total: parseInt(stats.total) || 0,
      totalBurn: parseFloat(stats.total_burn) || 0,
      avgEScore: parseFloat(stats.avg_e_score) || 0,
    };
  }

  /**
   * Find or create user by email
   */
  async findOrCreate(email, defaults = {}) {
    let user = await this.findByEmail(email);
    if (!user) {
      user = await this.create({ email, ...defaults });
    }
    return user;
  }
}

export default SQLiteUserRepository;
