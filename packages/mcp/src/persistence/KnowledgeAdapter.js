/**
 * Knowledge Persistence Adapter
 *
 * ISP: Only knowledge-related operations.
 * "La mémoire collective du chien"
 *
 * @module @cynic/mcp/persistence/KnowledgeAdapter
 */

'use strict';

/**
 * @typedef {Object} Knowledge
 * @property {string} knowledge_id
 * @property {string} summary
 * @property {string} content
 * @property {string[]} insights
 * @property {string} source_type
 * @property {Date} created_at
 */

export class KnowledgeAdapter {
  /**
   * @param {Object} repository - KnowledgeRepository from @cynic/persistence
   * @param {Object} fallback - MemoryStore or FileStore
   */
  constructor(repository, fallback) {
    this._repo = repository;
    this._fallback = fallback;
  }

  /**
   * Store knowledge
   * @param {Knowledge} knowledge
   * @returns {Promise<Knowledge|null>}
   */
  async store(knowledge) {
    if (this._repo) {
      try {
        return await this._repo.create(knowledge);
      } catch (err) {
        console.error('Error storing knowledge:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.storeKnowledge(knowledge);
    }
    return null;
  }

  /**
   * Search knowledge
   * @param {string} query
   * @param {Object} options
   * @returns {Promise<Knowledge[]>}
   */
  async search(query, options = {}) {
    if (this._repo) {
      try {
        // Try FTS search first
        return await this._repo.search(query, options);
      } catch (err) {
        // If FTS fails (migration not run), try fallback search
        if (err.message.includes('search_vector') || err.message.includes('websearch_to_tsquery')) {
          console.error('FTS not available, using fallback search');
          try {
            return await this._repo.searchFallback(query, options);
          } catch (fallbackErr) {
            console.error('Error in fallback search:', fallbackErr.message);
          }
        } else {
          console.error('Error searching knowledge:', err.message);
        }
      }
    }
    if (this._fallback) {
      return await this._fallback.searchKnowledge(query, options);
    }
    return [];
  }

  /**
   * Check if adapter is available
   */
  get isAvailable() {
    return !!this._repo || !!this._fallback;
  }
}
