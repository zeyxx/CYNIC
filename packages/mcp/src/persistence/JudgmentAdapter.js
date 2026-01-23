/**
 * Judgment Persistence Adapter
 *
 * ISP: Only judgment-related operations.
 * "φ distrusts φ" - judgments must persist for verification.
 *
 * @module @cynic/mcp/persistence/JudgmentAdapter
 */

'use strict';

/**
 * @typedef {Object} Judgment
 * @property {string} judgment_id
 * @property {number} q_score
 * @property {string} verdict
 * @property {number} confidence
 * @property {Object} axiom_scores
 * @property {Date} created_at
 */

export class JudgmentAdapter {
  /**
   * @param {Object} repository - JudgmentRepository from @cynic/persistence
   * @param {Object} fallback - MemoryStore or FileStore
   */
  constructor(repository, fallback) {
    this._repo = repository;
    this._fallback = fallback;
  }

  /**
   * Store a judgment
   * @param {Judgment} judgment
   * @returns {Promise<Judgment|null>}
   */
  async store(judgment) {
    if (this._repo) {
      try {
        return await this._repo.create(judgment);
      } catch (err) {
        console.error('Error storing judgment to PostgreSQL:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.storeJudgment(judgment);
    }
    return null;
  }

  /**
   * Get a judgment by ID
   * @param {string} judgmentId
   * @returns {Promise<Judgment|null>}
   */
  async getById(judgmentId) {
    if (this._repo?.findById) {
      try {
        return await this._repo.findById(judgmentId);
      } catch (err) {
        console.error('Error getting judgment from PostgreSQL:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getJudgment(judgmentId);
    }
    return null;
  }

  /**
   * Search judgments
   * @param {string} query
   * @param {Object} options
   * @returns {Promise<Judgment[]>}
   */
  async search(query, options = {}) {
    if (this._repo) {
      try {
        return await this._repo.search(query, options);
      } catch (err) {
        console.error('Error searching judgments:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.searchJudgments(query, options);
    }
    return [];
  }

  /**
   * Get recent judgments
   * @param {number} limit
   * @returns {Promise<Judgment[]>}
   */
  async getRecent(limit = 10) {
    if (this._repo) {
      try {
        return await this._repo.findRecent(limit);
      } catch (err) {
        console.error('Error getting recent judgments:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.findRecentJudgments(limit);
    }
    return [];
  }

  /**
   * Get judgment statistics
   * @param {Object} options
   * @returns {Promise<Object>}
   */
  async getStats(options = {}) {
    if (this._repo) {
      try {
        return await this._repo.getStats(options);
      } catch (err) {
        console.error('Error getting judgment stats:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getJudgmentStats();
    }
    return { total: 0, avgScore: 0, avgConfidence: 0, verdicts: {} };
  }

  /**
   * Check if adapter is available
   */
  get isAvailable() {
    return !!this._repo || !!this._fallback;
  }
}
