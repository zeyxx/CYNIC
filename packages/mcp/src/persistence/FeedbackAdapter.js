/**
 * Feedback Persistence Adapter
 *
 * ISP: Only feedback-related operations.
 * "Le chien apprend de chaque correction"
 *
 * @module @cynic/mcp/persistence/FeedbackAdapter
 */

'use strict';

/**
 * @typedef {Object} Feedback
 * @property {string} feedback_id
 * @property {string} judgment_id
 * @property {string} outcome - 'correct' | 'incorrect' | 'partial'
 * @property {string} userId
 * @property {string} itemType
 * @property {Date} created_at
 */

export class FeedbackAdapter {
  /**
   * @param {Object} repository - FeedbackRepository from @cynic/persistence
   * @param {Object} fallback - MemoryStore or FileStore
   * @param {Object} userLearningProfiles - UserLearningProfilesRepository (optional)
   */
  constructor(repository, fallback, userLearningProfiles = null) {
    this._repo = repository;
    this._fallback = fallback;
    this._profiles = userLearningProfiles;
  }

  /**
   * Store feedback
   * Also updates user_learning_profiles if available
   * @param {Feedback} feedback
   * @returns {Promise<Feedback|null>}
   */
  async store(feedback) {
    let result = null;

    if (this._repo) {
      try {
        result = await this._repo.create(feedback);

        // Update user learning profile if available
        if (this._profiles && feedback.userId) {
          try {
            const wasCorrect = feedback.outcome === 'correct';
            await this._profiles.recordFeedback(feedback.userId, wasCorrect);
            await this._profiles.recordActivity(feedback.userId);

            if (feedback.itemType) {
              await this._profiles.updateJudgmentPatterns(feedback.userId, feedback.itemType);
            }
          } catch (profileErr) {
            console.error('Error updating user learning profile:', profileErr.message);
          }
        }
      } catch (err) {
        console.error('Error storing feedback:', err.message);
      }
    }

    if (!result && this._fallback) {
      result = await this._fallback.storeFeedback(feedback);
    }

    return result;
  }

  /**
   * Check if adapter is available
   */
  get isAvailable() {
    return !!this._repo || !!this._fallback;
  }
}
