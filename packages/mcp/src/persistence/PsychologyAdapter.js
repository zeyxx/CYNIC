/**
 * Psychology Persistence Adapter
 *
 * ISP: Only psychology/user-state operations.
 * "Comprendre l'humain pour mieux l'aider"
 *
 * @module @cynic/mcp/persistence/PsychologyAdapter
 */

'use strict';

/**
 * @typedef {Object} PsychologyState
 * @property {string} user_id
 * @property {Object} dimensions - Energy, focus, etc.
 * @property {Object} emotions
 * @property {Object} calibration
 * @property {Date} updated_at
 */

export class PsychologyAdapter {
  /**
   * @param {Object} repository - PsychologyRepository from @cynic/persistence
   */
  constructor(repository) {
    this._repo = repository;
    // No fallback - psychology requires PostgreSQL for cross-session persistence
  }

  /**
   * Sync psychology state from local to database
   * Called at session end via sleep.cjs
   * @param {string} userId
   * @param {Object} data
   * @returns {Promise<Object|null>}
   */
  async sync(userId, data) {
    if (this._repo) {
      try {
        return await this._repo.syncPsychology(userId, data);
      } catch (err) {
        console.error('Error syncing psychology:', err.message);
      }
    }
    return null;
  }

  /**
   * Load psychology state from database
   * Called at session start via awaken.cjs
   * @param {string} userId
   * @returns {Promise<PsychologyState|null>}
   */
  async load(userId) {
    if (this._repo) {
      try {
        return await this._repo.loadPsychology(userId);
      } catch (err) {
        console.error('Error loading psychology:', err.message);
      }
    }
    return null;
  }

  /**
   * Record intervention outcome for learning
   * @param {string} userId
   * @param {Object} intervention
   * @returns {Promise<void>}
   */
  async recordIntervention(userId, intervention) {
    if (this._repo) {
      try {
        await this._repo.recordIntervention(userId, intervention);
      } catch (err) {
        console.error('Error recording intervention:', err.message);
      }
    }
  }

  /**
   * Get intervention effectiveness stats
   * @param {string} userId
   * @returns {Promise<Object|null>}
   */
  async getInterventionEffectiveness(userId) {
    if (this._repo) {
      try {
        return await this._repo.getInterventionEffectiveness(userId);
      } catch (err) {
        console.error('Error getting intervention effectiveness:', err.message);
      }
    }
    return null;
  }

  /**
   * Record learning observation for calibration
   * @param {string} userId
   * @param {Object} observation
   * @returns {Promise<void>}
   */
  async recordLearningObservation(userId, observation) {
    if (this._repo) {
      try {
        await this._repo.recordLearningObservation(userId, observation);
      } catch (err) {
        console.error('Error recording learning observation:', err.message);
      }
    }
  }

  /**
   * Get calibration stats from learning observations
   * @param {string} userId
   * @returns {Promise<Object|null>}
   */
  async getCalibrationStats(userId) {
    if (this._repo) {
      try {
        return await this._repo.getCalibrationStats(userId);
      } catch (err) {
        console.error('Error getting calibration stats:', err.message);
      }
    }
    return null;
  }

  /**
   * Get aggregate psychology stats across all users
   * @returns {Promise<Object>}
   */
  async getStats() {
    if (this._repo) {
      try {
        return await this._repo.getStats();
      } catch (err) {
        console.error('Error getting psychology stats:', err.message);
      }
    }
    return { totalUsers: 0, totalSessions: 0, avgAccuracy: 0 };
  }

  /**
   * Get top performers (highest calibration accuracy)
   * @param {number} limit
   * @returns {Promise<Array>}
   */
  async getTopPerformers(limit = 10) {
    if (this._repo) {
      try {
        return await this._repo.getTopPerformers(limit);
      } catch (err) {
        console.error('Error getting top performers:', err.message);
      }
    }
    return [];
  }

  /**
   * Check if adapter is available
   */
  get isAvailable() {
    return !!this._repo;
  }
}
