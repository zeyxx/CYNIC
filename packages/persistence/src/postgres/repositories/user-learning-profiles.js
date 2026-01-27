/**
 * User Learning Profiles Repository
 *
 * Per-user learning preferences and behavior patterns.
 * φ-bounded learning rates and feedback analysis.
 *
 * @module @cynic/persistence/repositories/user-learning-profiles
 */

'use strict';

import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

// φ constants
const PHI_INV_CUBED = Math.pow(0.618033988749895, 3); // φ⁻³ ≈ 0.236

/**
 * @extends BaseRepository
 */
export class UserLearningProfilesRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Get or create user learning profile
   */
  async getOrCreate(userId) {
    // Try to get existing
    const existing = await this.findByUserId(userId);
    if (existing) return existing;

    // Create new profile
    const { rows } = await this.db.query(`
      INSERT INTO user_learning_profiles (
        user_id, learning_rate
      ) VALUES ($1, $2)
      ON CONFLICT (user_id) DO UPDATE SET
        updated_at = NOW()
      RETURNING *
    `, [userId, PHI_INV_CUBED]);

    return rows[0];
  }

  /**
   * Find profile by user ID
   */
  async findByUserId(userId) {
    const { rows } = await this.db.query(
      'SELECT * FROM user_learning_profiles WHERE user_id = $1',
      [userId]
    );
    return rows[0] || null;
  }

  /**
   * Record feedback from user
   */
  async recordFeedback(userId, wasCorrect) {
    const { rows } = await this.db.query(`
      UPDATE user_learning_profiles SET
        total_feedback = total_feedback + 1,
        correct_feedback = correct_feedback + $2,
        feedback_bias = (
          (feedback_bias * total_feedback + $3) / (total_feedback + 1)
        ),
        updated_at = NOW()
      WHERE user_id = $1
      RETURNING *
    `, [
      userId,
      wasCorrect ? 1 : 0,
      wasCorrect ? 0.1 : -0.1, // Positive/negative tendency
    ]);

    return rows[0];
  }

  /**
   * Update dimension preferences
   */
  async updateDimensionPreferences(userId, preferences) {
    const { rows } = await this.db.query(`
      UPDATE user_learning_profiles SET
        preferred_dimensions = preferred_dimensions || $2,
        updated_at = NOW()
      WHERE user_id = $1
      RETURNING *
    `, [userId, JSON.stringify(preferences)]);

    return rows[0];
  }

  /**
   * Update judgment patterns
   */
  async updateJudgmentPatterns(userId, itemType) {
    const { rows } = await this.db.query(`
      UPDATE user_learning_profiles SET
        judgment_patterns = jsonb_set(
          judgment_patterns,
          $2,
          COALESCE(judgment_patterns->$3, '0')::int + 1
        ),
        updated_at = NOW()
      WHERE user_id = $1
      RETURNING *
    `, [userId, `{${itemType}}`, itemType]);

    return rows[0];
  }

  /**
   * Update activity times
   */
  async recordActivity(userId) {
    const hour = new Date().getHours().toString();

    const { rows } = await this.db.query(`
      UPDATE user_learning_profiles SET
        activity_times = jsonb_set(
          activity_times,
          $2,
          (COALESCE(activity_times->$3, '0')::int + 1)::text::jsonb
        ),
        updated_at = NOW()
      WHERE user_id = $1
      RETURNING *
    `, [userId, `{${hour}}`, hour]);

    return rows[0];
  }

  /**
   * Update learning rate based on performance
   */
  async updateLearningRate(userId, newRate) {
    // Clamp to φ-bounded range [0.1, 0.382]
    const clampedRate = Math.min(0.382, Math.max(0.1, newRate));

    const { rows } = await this.db.query(`
      UPDATE user_learning_profiles SET
        learning_rate = $2,
        updated_at = NOW()
      WHERE user_id = $1
      RETURNING *
    `, [userId, clampedRate]);

    return rows[0];
  }

  /**
   * Update E-Score feedback correlation
   */
  async updateEScoreCorrelation(userId, correlation) {
    const { rows } = await this.db.query(`
      UPDATE user_learning_profiles SET
        escore_feedback_correlation = $2,
        updated_at = NOW()
      WHERE user_id = $1
      RETURNING *
    `, [userId, correlation]);

    return rows[0];
  }

  /**
   * Get profile summary for learning
   */
  async getSummary(userId) {
    const profile = await this.findByUserId(userId);
    if (!profile) return null;

    const accuracy = profile.total_feedback > 0
      ? profile.correct_feedback / profile.total_feedback
      : 0;

    // Find most active hour
    const activityTimes = profile.activity_times || {};
    let peakHour = null;
    let peakCount = 0;
    for (const [hour, count] of Object.entries(activityTimes)) {
      if (count > peakCount) {
        peakHour = parseInt(hour);
        peakCount = count;
      }
    }

    // Find most common judgment type
    const judgmentPatterns = profile.judgment_patterns || {};
    let topItemType = null;
    let topCount = 0;
    for (const [type, count] of Object.entries(judgmentPatterns)) {
      if (count > topCount) {
        topItemType = type;
        topCount = count;
      }
    }

    return {
      userId,
      learningRate: parseFloat(profile.learning_rate),
      totalFeedback: profile.total_feedback,
      feedbackAccuracy: Math.round(accuracy * 1000) / 1000,
      feedbackBias: parseFloat(profile.feedback_bias) || 0,
      peakActivityHour: peakHour,
      topItemType,
      preferredDimensions: profile.preferred_dimensions || {},
      escoreCorrelation: parseFloat(profile.escore_feedback_correlation) || null,
    };
  }

  /**
   * Aggregate session stats into user profile
   * Called at session end to update lifetime counters
   * @param {string} userId - User ID
   * @param {Object} sessionStats - Stats from the completed session
   * @returns {Promise<Object>} Updated profile
   */
  async aggregateSessionStats(userId, sessionStats) {
    const userIdUUID = await this._ensureUserExists(userId);

    const { rows } = await this.db.query(`
      UPDATE user_learning_profiles SET
        session_count = session_count + 1,
        total_tool_calls = total_tool_calls + $2,
        total_errors = total_errors + $3,
        total_danger_blocked = total_danger_blocked + $4,
        last_session_at = NOW(),
        updated_at = NOW()
      WHERE user_id = $1
      RETURNING *
    `, [
      userIdUUID,
      sessionStats.toolCalls || 0,
      sessionStats.errors || 0,
      sessionStats.dangerBlocked || 0,
    ]);

    // If no row was updated (user doesn't have a profile yet), create one
    if (rows.length === 0) {
      const { rows: newRows } = await this.db.query(`
        INSERT INTO user_learning_profiles (
          user_id,
          session_count,
          total_tool_calls,
          total_errors,
          total_danger_blocked,
          last_session_at,
          learning_rate
        ) VALUES ($1, 1, $2, $3, $4, NOW(), $5)
        ON CONFLICT (user_id) DO UPDATE SET
          session_count = user_learning_profiles.session_count + 1,
          total_tool_calls = user_learning_profiles.total_tool_calls + $2,
          total_errors = user_learning_profiles.total_errors + $3,
          total_danger_blocked = user_learning_profiles.total_danger_blocked + $4,
          last_session_at = NOW(),
          updated_at = NOW()
        RETURNING *
      `, [
        userIdUUID,
        sessionStats.toolCalls || 0,
        sessionStats.errors || 0,
        sessionStats.dangerBlocked || 0,
        PHI_INV_CUBED,
      ]);
      return newRows[0];
    }

    return rows[0];
  }

  /**
   * Get aggregate statistics
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total_profiles,
        AVG(total_feedback) as avg_feedback,
        AVG(learning_rate) as avg_learning_rate,
        AVG(
          CASE WHEN total_feedback > 0
          THEN correct_feedback::float / total_feedback
          ELSE 0 END
        ) as avg_accuracy
      FROM user_learning_profiles
    `);

    const stats = rows[0];
    return {
      totalProfiles: parseInt(stats.total_profiles),
      avgFeedback: parseFloat(stats.avg_feedback) || 0,
      avgLearningRate: parseFloat(stats.avg_learning_rate) || PHI_INV_CUBED,
      avgAccuracy: parseFloat(stats.avg_accuracy) || 0,
    };
  }

  /**
   * Get most active users by feedback
   */
  async getMostActive(limit = 10) {
    const { rows } = await this.db.query(`
      SELECT
        ulp.*,
        u.username,
        u.wallet_address,
        u.e_score
      FROM user_learning_profiles ulp
      JOIN users u ON u.id = ulp.user_id
      ORDER BY ulp.total_feedback DESC
      LIMIT $1
    `, [limit]);

    return rows;
  }

  // ===========================================================================
  // CROSS-SESSION PROFILE SYNC (Migration 008)
  // ===========================================================================

  /**
   * Sync full user profile from hooks to database
   * Called at session end to persist cross-session memory
   * @param {string} userId - User ID (can be hook-generated usr_xxx or UUID)
   * @param {Object} profile - Full profile from cynic-core.cjs
   * @returns {Promise<Object>} Updated profile row
   */
  async syncProfile(userId, profile) {
    // Ensure user exists first (create if needed with hook userId)
    const userIdUUID = await this._ensureUserExists(userId, profile.identity);

    const { rows } = await this.db.query(`
      SELECT sync_user_profile($1, $2, $3, $4, $5, $6) as result
    `, [
      userIdUUID,
      JSON.stringify(profile.identity || {}),
      JSON.stringify(profile.stats || {}),
      JSON.stringify(profile.patterns || {}),
      JSON.stringify(profile.preferences || {}),
      JSON.stringify(profile.memory || {}),
    ]);

    return rows[0]?.result || null;
  }

  /**
   * Load full user profile from database
   * Called at session start to restore cross-session memory
   * @param {string} userId - User ID (hook-generated usr_xxx or UUID)
   * @returns {Promise<Object|null>} Full profile or null if not found
   */
  async loadProfile(userId) {
    // Try to find user by hook userId or UUID
    const userIdUUID = await this._findUserUUID(userId);
    if (!userIdUUID) return null;

    const { rows } = await this.db.query(
      'SELECT load_user_profile($1) as profile',
      [userIdUUID]
    );

    return rows[0]?.profile || null;
  }

  /**
   * Ensure user exists in users table, creating if needed
   * @private
   */
  async _ensureUserExists(hookUserId, identity = {}) {
    // First check if hookUserId is already a UUID
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    if (uuidRegex.test(hookUserId)) {
      return hookUserId;
    }

    // Check if user exists by username (hook userId stored as username)
    const { rows: existing } = await this.db.query(
      'SELECT id FROM users WHERE username = $1',
      [hookUserId]
    );

    if (existing.length > 0) {
      return existing[0].id;
    }

    // Create new user with hook userId as username
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
    // If already UUID, return as-is
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    if (uuidRegex.test(hookUserId)) {
      return hookUserId;
    }

    // Find by username
    const { rows } = await this.db.query(
      'SELECT id FROM users WHERE username = $1',
      [hookUserId]
    );

    return rows[0]?.id || null;
  }

  /**
   * Merge remote profile with local profile
   * Remote (DB) is source of truth for accumulated stats
   * Local has latest session data
   * @param {Object} remoteProfile - Profile from database
   * @param {Object} localProfile - Profile from local JSON
   * @returns {Object} Merged profile
   */
  mergeProfiles(remoteProfile, localProfile) {
    if (!remoteProfile) return localProfile;
    if (!localProfile) return remoteProfile;

    return {
      userId: localProfile.userId,
      identity: {
        ...remoteProfile.identity,
        ...localProfile.identity,
        // Keep the earliest firstSeen
        firstSeen: remoteProfile.identity?.firstSeen || localProfile.identity?.firstSeen,
        lastSeen: localProfile.identity?.lastSeen || new Date().toISOString(),
      },
      stats: {
        // Use max of remote/local for accumulated stats
        sessions: Math.max(
          remoteProfile.stats?.sessions || 0,
          localProfile.stats?.sessions || 0
        ),
        toolCalls: Math.max(
          remoteProfile.stats?.toolCalls || 0,
          localProfile.stats?.toolCalls || 0
        ),
        errorsEncountered: Math.max(
          remoteProfile.stats?.errorsEncountered || 0,
          localProfile.stats?.errorsEncountered || 0
        ),
        dangerBlocked: Math.max(
          remoteProfile.stats?.dangerBlocked || 0,
          localProfile.stats?.dangerBlocked || 0
        ),
      },
      patterns: {
        preferredLanguages: [
          ...new Set([
            ...(remoteProfile.patterns?.preferredLanguages || []),
            ...(localProfile.patterns?.preferredLanguages || []),
          ])
        ],
        commonTools: this._mergeToolCounts(
          remoteProfile.patterns?.commonTools || {},
          localProfile.patterns?.commonTools || {}
        ),
        workingHours: this._mergeToolCounts(
          remoteProfile.patterns?.workingHours || {},
          localProfile.patterns?.workingHours || {}
        ),
        projectTypes: [
          ...new Set([
            ...(remoteProfile.patterns?.projectTypes || []),
            ...(localProfile.patterns?.projectTypes || []),
          ])
        ],
      },
      preferences: {
        ...remoteProfile.preferences,
        ...localProfile.preferences,
      },
      memory: {
        recentProjects: [
          ...new Set([
            ...(localProfile.memory?.recentProjects || []),
            ...(remoteProfile.memory?.recentProjects || []),
          ])
        ].slice(0, 20),
        ongoingTasks: localProfile.memory?.ongoingTasks || [],
        decisions: [
          ...(localProfile.memory?.decisions || []),
          ...(remoteProfile.memory?.decisions || []),
        ].slice(-100),
      },
      // Include learning data from remote
      learning: remoteProfile.learning || {},
      meta: remoteProfile.meta || {},
    };
  }

  /**
   * Merge tool/hour counts (take max of each key)
   * @private
   */
  _mergeToolCounts(remote, local) {
    const merged = { ...remote };
    for (const [key, value] of Object.entries(local)) {
      merged[key] = Math.max(merged[key] || 0, value);
    }
    return merged;
  }
}

export default UserLearningProfilesRepository;
