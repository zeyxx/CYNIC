/**
 * Psychology Repository
 *
 * Cross-session persistence for human psychology state and learning loop.
 * "Comprendre l'humain pour mieux l'aider" - κυνικός
 *
 * Persists:
 *   - Psychological state (dimensions, emotions, composites)
 *   - Learning loop calibration (accuracy, patterns)
 *   - Intervention history (effectiveness tracking)
 *
 * Implements: BaseRepository
 *
 * @module @cynic/persistence/repositories/psychology
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * Psychology Repository
 *
 * LSP compliant - implements standard repository interface.
 *
 * @extends BaseRepository
 */
export class PsychologyRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Sync psychology state from local to database
   * Called at session end via sleep.cjs
   *
   * @param {string} userId - User ID
   * @param {Object} data - Psychology data
   * @param {Object} data.dimensions - Psychological dimensions
   * @param {Object} data.emotions - Emotional spectrum
   * @param {Object} data.temporal - Temporal tracking
   * @param {Object} data.calibration - Learning loop calibration
   * @param {Object} data.userPatterns - Learned user patterns
   * @param {Object} data.interventionStats - Intervention statistics
   * @returns {Promise<Object>} Synced result
   */
  async syncPsychology(userId, data) {
    const userIdUUID = await this._ensureUserExists(userId);

    const { rows } = await this.db.query(`
      INSERT INTO user_psychology (
        user_id,
        dimensions,
        emotions,
        temporal,
        calibration,
        user_patterns,
        intervention_stats,
        session_count,
        updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, 1, NOW())
      ON CONFLICT (user_id) DO UPDATE SET
        dimensions = COALESCE(
          jsonb_deep_merge(user_psychology.dimensions, $2),
          $2
        ),
        emotions = COALESCE(
          jsonb_deep_merge(user_psychology.emotions, $3),
          $3
        ),
        temporal = $4,
        calibration = jsonb_deep_merge(user_psychology.calibration, $5),
        user_patterns = jsonb_deep_merge(user_psychology.user_patterns, $6),
        intervention_stats = jsonb_deep_merge(user_psychology.intervention_stats, $7),
        session_count = user_psychology.session_count + 1,
        updated_at = NOW()
      RETURNING *
    `, [
      userIdUUID,
      JSON.stringify(data.dimensions || {}),
      JSON.stringify(data.emotions || {}),
      JSON.stringify(data.temporal || {}),
      JSON.stringify(data.calibration || {}),
      JSON.stringify(data.userPatterns || {}),
      JSON.stringify(data.interventionStats || {}),
    ]);

    return rows[0] || null;
  }

  /**
   * Load psychology state from database
   * Called at session start via awaken.cjs
   *
   * @param {string} userId - User ID
   * @returns {Promise<Object|null>} Psychology data or null
   */
  async loadPsychology(userId) {
    const userIdUUID = await this._findUserUUID(userId);
    if (!userIdUUID) return null;

    const { rows } = await this.db.query(`
      SELECT
        dimensions,
        emotions,
        temporal,
        calibration,
        user_patterns,
        intervention_stats,
        session_count,
        updated_at
      FROM user_psychology
      WHERE user_id = $1
    `, [userIdUUID]);

    if (!rows[0]) return null;

    const row = rows[0];
    return {
      dimensions: row.dimensions,
      emotions: row.emotions,
      temporal: row.temporal,
      calibration: row.calibration,
      userPatterns: row.user_patterns,
      interventionStats: row.intervention_stats,
      sessionCount: row.session_count,
      lastUpdated: row.updated_at,
    };
  }

  /**
   * Record intervention outcome for learning
   *
   * @param {string} userId - User ID
   * @param {Object} intervention - Intervention data
   */
  async recordIntervention(userId, intervention) {
    const userIdUUID = await this._ensureUserExists(userId);

    await this.db.query(`
      INSERT INTO psychology_interventions (
        user_id,
        intervention_type,
        intensity,
        message,
        response,
        was_effective,
        context
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
    `, [
      userIdUUID,
      intervention.type,
      intervention.intensity,
      intervention.message,
      intervention.response,
      intervention.wasEffective,
      JSON.stringify(intervention.context || {}),
    ]);
  }

  /**
   * Get intervention effectiveness stats
   *
   * @param {string} userId - User ID
   * @returns {Promise<Object>} Effectiveness stats
   */
  async getInterventionEffectiveness(userId) {
    const userIdUUID = await this._findUserUUID(userId);
    if (!userIdUUID) return null;

    const { rows } = await this.db.query(`
      SELECT
        intervention_type,
        COUNT(*) as total,
        SUM(CASE WHEN was_effective THEN 1 ELSE 0 END) as effective,
        AVG(CASE WHEN was_effective THEN 1 ELSE 0 END) as effectiveness_rate
      FROM psychology_interventions
      WHERE user_id = $1
      GROUP BY intervention_type
    `, [userIdUUID]);

    const byType = {};
    for (const row of rows) {
      byType[row.intervention_type] = {
        total: parseInt(row.total),
        effective: parseInt(row.effective),
        rate: parseFloat(row.effectiveness_rate),
      };
    }

    return byType;
  }

  /**
   * Record learning observation
   *
   * @param {string} userId - User ID
   * @param {Object} observation - Learning observation
   */
  async recordLearningObservation(userId, observation) {
    const userIdUUID = await this._ensureUserExists(userId);

    await this.db.query(`
      INSERT INTO psychology_observations (
        user_id,
        module,
        prediction,
        actual,
        correct,
        context
      ) VALUES ($1, $2, $3, $4, $5, $6)
    `, [
      userIdUUID,
      observation.module,
      observation.prediction,
      observation.actual,
      observation.correct,
      JSON.stringify(observation.context || {}),
    ]);
  }

  /**
   * Get learning calibration from observations
   *
   * @param {string} userId - User ID
   * @returns {Promise<Object>} Calibration stats by module
   */
  async getCalibrationStats(userId) {
    const userIdUUID = await this._findUserUUID(userId);
    if (!userIdUUID) return null;

    const { rows } = await this.db.query(`
      SELECT
        module,
        COUNT(*) as total,
        SUM(CASE WHEN correct THEN 1 ELSE 0 END) as correct_count,
        AVG(CASE WHEN correct THEN 1 ELSE 0 END) as accuracy
      FROM psychology_observations
      WHERE user_id = $1
        AND created_at > NOW() - INTERVAL '7 days'
      GROUP BY module
    `, [userIdUUID]);

    const byModule = {};
    for (const row of rows) {
      byModule[row.module] = {
        total: parseInt(row.total),
        correct: parseInt(row.correct_count),
        accuracy: parseFloat(row.accuracy),
      };
    }

    return byModule;
  }

  /**
   * Get aggregate psychology stats across all users
   *
   * @returns {Promise<Object>} Aggregate stats
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total_users,
        SUM(session_count) as total_sessions,
        AVG(
          (calibration->'overall'->>'accuracy')::float
        ) as avg_accuracy,
        MAX(updated_at) as last_activity
      FROM user_psychology
    `);

    const stats = rows[0];
    return {
      totalUsers: parseInt(stats.total_users) || 0,
      totalSessions: parseInt(stats.total_sessions) || 0,
      avgAccuracy: parseFloat(stats.avg_accuracy) || 0,
      lastActivity: stats.last_activity,
    };
  }

  /**
   * Get top performers (high calibration accuracy)
   *
   * @param {number} limit - Max results
   * @returns {Promise<Array>} Top users
   */
  async getTopPerformers(limit = 10) {
    const { rows } = await this.db.query(`
      SELECT
        p.user_id,
        u.username,
        p.session_count,
        p.calibration->'overall'->>'accuracy' as accuracy,
        p.updated_at
      FROM user_psychology p
      JOIN users u ON u.id = p.user_id
      WHERE (p.calibration->'overall'->>'accuracy')::float > 0
      ORDER BY (p.calibration->'overall'->>'accuracy')::float DESC
      LIMIT $1
    `, [limit]);

    return rows;
  }

  /**
   * Ensure user exists in users table
   * @private
   */
  async _ensureUserExists(hookUserId) {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    if (uuidRegex.test(hookUserId)) {
      return hookUserId;
    }

    const { rows: existing } = await this.db.query(
      'SELECT id FROM users WHERE username = $1',
      [hookUserId]
    );

    if (existing.length > 0) {
      return existing[0].id;
    }

    const { rows: created } = await this.db.query(`
      INSERT INTO users (username, e_score)
      VALUES ($1, 0.5)
      ON CONFLICT (username) DO UPDATE SET updated_at = NOW()
      RETURNING id
    `, [hookUserId]);

    return created[0].id;
  }

  /**
   * Find UUID for a hook userId
   * @private
   */
  async _findUserUUID(hookUserId) {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    if (uuidRegex.test(hookUserId)) {
      return hookUserId;
    }

    const { rows } = await this.db.query(
      'SELECT id FROM users WHERE username = $1',
      [hookUserId]
    );

    return rows[0]?.id || null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // BaseRepository Interface Methods (LSP compliance)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create psychology entry (via syncPsychology)
   * @param {Object} data - Psychology data with userId
   * @returns {Promise<Object>}
   */
  async create(data) {
    return this.syncPsychology(data.userId, data);
  }

  /**
   * Find psychology by user ID
   * @param {string} userId - User ID
   * @returns {Promise<Object|null>}
   */
  async findById(userId) {
    return this.loadPsychology(userId);
  }

  /**
   * List psychology entries with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0 } = options;

    const { rows } = await this.db.query(`
      SELECT
        p.*,
        u.username
      FROM user_psychology p
      JOIN users u ON u.id = p.user_id
      ORDER BY p.updated_at DESC
      LIMIT $1 OFFSET $2
    `, [limit, offset]);

    return rows;
  }

  /**
   * Update psychology entry
   * @param {string} userId - User ID
   * @param {Object} data - Update data
   * @returns {Promise<Object|null>}
   */
  async update(userId, data) {
    return this.syncPsychology(userId, data);
  }

  /**
   * Delete psychology entry
   * @param {string} userId - User ID
   * @returns {Promise<boolean>}
   */
  async delete(userId) {
    const userIdUUID = await this._findUserUUID(userId);
    if (!userIdUUID) return false;

    const { rowCount } = await this.db.query(
      'DELETE FROM user_psychology WHERE user_id = $1',
      [userIdUUID]
    );
    return rowCount > 0;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// v1.1: BURNOUT DETECTION METHODS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Record a psychology snapshot for trend analysis
 *
 * @param {string} userId - User ID
 * @param {Object} dimensions - Psychology dimensions
 * @param {Object} [context={}] - Additional context
 * @returns {Promise<Object>} Recorded snapshot
 */
PsychologyRepository.prototype.recordSnapshot = async function(userId, dimensions, context = {}) {
  const userIdUUID = await this._ensureUserExists(userId);

  // Calculate burnout score
  const energy = dimensions.energy ?? 0.618;
  const frustration = dimensions.frustration ?? 0.382;
  const creativity = dimensions.creativity ?? 0.5;
  const focus = dimensions.focus ?? 0.618;

  const burnoutScore = (1 - energy) * frustration * (1 + ((1 - creativity) * 0.309));
  const flowScore = Math.min(1, energy * 0.618 + focus * 0.618 + creativity * 0.382 - frustration * 0.618);

  const { rows } = await this.db.query(`
    INSERT INTO psychology_snapshots (
      user_id,
      session_id,
      energy,
      focus,
      creativity,
      frustration,
      burnout_score,
      flow_score,
      work_done,
      heat_generated,
      error_count
    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    RETURNING *
  `, [
    userIdUUID,
    context.sessionId || null,
    energy,
    focus,
    creativity,
    frustration,
    Math.min(1, Math.max(0, burnoutScore)),
    Math.min(1, Math.max(0, flowScore)),
    context.workDone || 0,
    context.heatGenerated || 0,
    context.errorCount || 0,
  ]);

  return rows[0];
};

/**
 * Get burnout trends for a user
 *
 * @param {string} userId - User ID
 * @param {number} [limit=13] - Snapshots to analyze
 * @returns {Promise<Object>} Trend data
 */
PsychologyRepository.prototype.getBurnoutTrends = async function(userId, limit = 13) {
  const userIdUUID = await this._findUserUUID(userId);
  if (!userIdUUID) return { hasSufficientData: false };

  const { rows: snapshots } = await this.db.query(`
    SELECT
      energy,
      focus,
      frustration,
      burnout_score,
      flow_score,
      created_at
    FROM psychology_snapshots
    WHERE user_id = $1
    ORDER BY created_at DESC
    LIMIT $2
  `, [userIdUUID, limit]);

  if (snapshots.length < 3) {
    return {
      hasSufficientData: false,
      snapshotCount: snapshots.length,
    };
  }

  // Calculate simple trend (slope)
  const burnoutValues = snapshots.map(s => s.burnout_score).reverse();
  const n = burnoutValues.length;
  let sumX = 0, sumY = 0, sumXY = 0, sumX2 = 0;
  for (let i = 0; i < n; i++) {
    sumX += i;
    sumY += burnoutValues[i];
    sumXY += i * burnoutValues[i];
    sumX2 += i * i;
  }
  const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);

  return {
    hasSufficientData: true,
    snapshotCount: snapshots.length,
    current: {
      energy: snapshots[0].energy,
      frustration: snapshots[0].frustration,
      burnout: snapshots[0].burnout_score,
      flow: snapshots[0].flow_score,
    },
    trend: {
      slope,
      direction: slope > 0.01 ? 'rising' : slope < -0.01 ? 'declining' : 'stable',
    },
    timespan: {
      from: snapshots[snapshots.length - 1].created_at,
      to: snapshots[0].created_at,
    },
  };
};

/**
 * Get active burnout episode for a user
 *
 * @param {string} userId - User ID
 * @returns {Promise<Object|null>} Active episode or null
 */
PsychologyRepository.prototype.getActiveBurnoutEpisode = async function(userId) {
  const userIdUUID = await this._findUserUUID(userId);
  if (!userIdUUID) return null;

  const { rows } = await this.db.query(`
    SELECT *
    FROM burnout_episodes
    WHERE user_id = $1 AND ended_at IS NULL
    ORDER BY started_at DESC
    LIMIT 1
  `, [userIdUUID]);

  return rows[0] || null;
};

/**
 * Get burnout episode history for a user
 *
 * @param {string} userId - User ID
 * @param {number} [limit=10] - Max episodes
 * @returns {Promise<Object[]>} Episodes
 */
PsychologyRepository.prototype.getBurnoutHistory = async function(userId, limit = 10) {
  const userIdUUID = await this._findUserUUID(userId);
  if (!userIdUUID) return [];

  const { rows } = await this.db.query(`
    SELECT *
    FROM burnout_episodes
    WHERE user_id = $1
    ORDER BY started_at DESC
    LIMIT $2
  `, [userIdUUID, limit]);

  return rows;
};

/**
 * Record a burnout warning
 *
 * @param {string} userId - User ID
 * @param {Object} warning - Warning data
 * @returns {Promise<Object>} Created warning
 */
PsychologyRepository.prototype.recordBurnoutWarning = async function(userId, warning) {
  const userIdUUID = await this._ensureUserExists(userId);

  const { rows } = await this.db.query(`
    INSERT INTO burnout_warnings (
      user_id,
      warning_type,
      severity,
      message,
      burnout_score,
      energy,
      frustration
    ) VALUES ($1, $2, $3, $4, $5, $6, $7)
    RETURNING *
  `, [
    userIdUUID,
    warning.type,
    warning.severity,
    warning.message,
    warning.burnoutScore,
    warning.energy,
    warning.frustration,
  ]);

  return rows[0];
};

export default PsychologyRepository;
