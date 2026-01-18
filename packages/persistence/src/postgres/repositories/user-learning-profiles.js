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

// φ constants
const PHI_INV_CUBED = Math.pow(0.618033988749895, 3); // φ⁻³ ≈ 0.236

export class UserLearningProfilesRepository {
  constructor(db = null) {
    this.db = db || getPool();
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
}

export default UserLearningProfilesRepository;
