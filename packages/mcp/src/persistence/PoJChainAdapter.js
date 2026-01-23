/**
 * PoJ Chain Persistence Adapter
 *
 * ISP: Only Proof-of-Judgment chain operations.
 * "φ distrusts φ" - the chain is how CYNIC verifies itself.
 *
 * @module @cynic/mcp/persistence/PoJChainAdapter
 */

'use strict';

/**
 * @typedef {Object} PoJBlock
 * @property {string} block_hash
 * @property {number} slot
 * @property {string} prev_hash
 * @property {number} judgment_count
 * @property {string[]} judgment_ids
 * @property {Date} created_at
 */

export class PoJChainAdapter {
  /**
   * @param {Object} repository - PoJBlockRepository from @cynic/persistence
   * @param {Object} fallback - MemoryStore or FileStore
   */
  constructor(repository, fallback) {
    this._repo = repository;
    this._fallback = fallback;
  }

  /**
   * Store a PoJ block
   * @param {PoJBlock} block
   * @returns {Promise<PoJBlock|null>}
   */
  async store(block) {
    if (this._repo) {
      try {
        return await this._repo.create(block);
      } catch (err) {
        console.error('Error storing PoJ block to PostgreSQL:', err.message);
      }
    }
    if (this._fallback) {
      try {
        return await this._fallback.storePoJBlock(block);
      } catch (err) {
        console.error('Error storing PoJ block to fallback:', err.message);
      }
    }
    return null;
  }

  /**
   * Get the head (latest) block
   * @returns {Promise<PoJBlock|null>}
   */
  async getHead() {
    if (this._repo) {
      try {
        return await this._repo.getHead();
      } catch (err) {
        console.error('Error getting PoJ head:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getPoJHead();
    }
    return null;
  }

  /**
   * Get chain statistics
   * @returns {Promise<Object>}
   */
  async getStats() {
    if (this._repo) {
      try {
        return await this._repo.getStats();
      } catch (err) {
        console.error('Error getting PoJ stats:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getPoJStats();
    }
    return { totalBlocks: 0, headSlot: 0, totalJudgments: 0 };
  }

  /**
   * Get recent blocks
   * @param {number} limit
   * @returns {Promise<PoJBlock[]>}
   */
  async getRecent(limit = 10) {
    if (this._repo) {
      try {
        return await this._repo.findRecent(limit);
      } catch (err) {
        console.error('Error getting recent PoJ blocks:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getRecentPoJBlocks(limit);
    }
    return [];
  }

  /**
   * Get block by slot number
   * @param {number} slot
   * @returns {Promise<PoJBlock|null>}
   */
  async getBySlot(slot) {
    if (this._repo) {
      try {
        return await this._repo.findByNumber(slot);
      } catch (err) {
        console.error('Error getting PoJ block by slot:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getPoJBlockBySlot(slot);
    }
    return null;
  }

  /**
   * Verify chain integrity
   * @returns {Promise<Object>}
   */
  async verifyIntegrity() {
    if (this._repo) {
      try {
        return await this._repo.verifyIntegrity();
      } catch (err) {
        console.error('Error verifying PoJ chain:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.verifyPoJChain();
    }
    return { valid: true, blocksChecked: 0, errors: [] };
  }

  /**
   * Check if adapter is available
   */
  get isAvailable() {
    return !!this._repo || !!this._fallback;
  }
}
