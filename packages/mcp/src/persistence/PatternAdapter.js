/**
 * Pattern Persistence Adapter
 *
 * ISP: Only pattern-related operations.
 * "Culture is a moat" - patterns are the cultural memory.
 *
 * @module @cynic/mcp/persistence/PatternAdapter
 */

'use strict';

/**
 * @typedef {Object} Pattern
 * @property {string} pattern_id
 * @property {string} category
 * @property {string} name
 * @property {number} confidence
 * @property {number} frequency
 * @property {Date} created_at
 * @property {Date} updated_at
 */

export class PatternAdapter {
  /**
   * @param {Object} repository - PatternRepository from @cynic/persistence
   * @param {Object} fallback - MemoryStore or FileStore
   */
  constructor(repository, fallback) {
    this._repo = repository;
    this._fallback = fallback;
  }

  /**
   * Upsert a pattern (insert or update)
   * @param {Pattern} pattern
   * @returns {Promise<Pattern|null>}
   */
  async upsert(pattern) {
    if (this._repo) {
      try {
        return await this._repo.upsert(pattern);
      } catch (err) {
        console.error('Error upserting pattern:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.upsertPattern(pattern);
    }
    return null;
  }

  /**
   * Get patterns with optional filtering
   * @param {Object} options
   * @param {string} [options.category]
   * @param {number} [options.limit=10]
   * @returns {Promise<Pattern[]>}
   */
  async get(options = {}) {
    if (this._repo) {
      const { category, limit = 10 } = options;
      try {
        if (category) {
          return await this._repo.findByCategory(category, limit);
        }
        return await this._repo.getTopPatterns(limit);
      } catch (err) {
        console.error('Error getting patterns:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getPatterns(options);
    }
    return [];
  }

  /**
   * Get patterns by category
   * @param {string} category
   * @param {number} limit
   * @returns {Promise<Pattern[]>}
   */
  async getByCategory(category, limit = 10) {
    return this.get({ category, limit });
  }

  /**
   * Get top patterns
   * @param {number} limit
   * @returns {Promise<Pattern[]>}
   */
  async getTop(limit = 10) {
    return this.get({ limit });
  }

  /**
   * Check if adapter is available
   */
  get isAvailable() {
    return !!this._repo || !!this._fallback;
  }
}
