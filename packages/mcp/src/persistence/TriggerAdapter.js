/**
 * Trigger Persistence Adapter
 *
 * ISP: Only trigger-related operations.
 * "Les watchdogs doivent persister leurs règles"
 *
 * @module @cynic/mcp/persistence/TriggerAdapter
 */

'use strict';

/**
 * @typedef {Object} Trigger
 * @property {string} trigger_id
 * @property {string} name
 * @property {string} type
 * @property {Object} conditions
 * @property {Object} actions
 * @property {boolean} enabled
 * @property {Date} created_at
 */

export class TriggerAdapter {
  /**
   * @param {Object} repository - TriggerRepository from @cynic/persistence
   * @param {Object} fallback - MemoryStore or FileStore
   */
  constructor(repository, fallback) {
    this._repo = repository;
    this._fallback = fallback;
  }

  /**
   * Get all enabled triggers
   * @returns {Promise<Trigger[]>}
   */
  async getEnabled() {
    if (this._repo) {
      try {
        return await this._repo.findEnabled();
      } catch (err) {
        console.error('Error getting enabled triggers:', err.message);
      }
    }
    if (this._fallback?.triggersState?.triggers) {
      return this._fallback.triggersState.triggers.filter(t => t.enabled);
    }
    return [];
  }

  /**
   * Get all triggers
   * @returns {Promise<Trigger[]>}
   */
  async getAll() {
    if (this._repo) {
      try {
        return await this._repo.findAll();
      } catch (err) {
        console.error('Error getting all triggers:', err.message);
      }
    }
    if (this._fallback?.triggersState?.triggers) {
      return this._fallback.triggersState.triggers;
    }
    return [];
  }

  /**
   * Create a new trigger
   * @param {Trigger} trigger
   * @returns {Promise<Trigger|null>}
   */
  async create(trigger) {
    if (this._repo) {
      try {
        return await this._repo.create(trigger);
      } catch (err) {
        console.error('Error creating trigger:', err.message);
      }
    }
    return null;
  }

  /**
   * Update a trigger
   * @param {string} triggerId
   * @param {Object} updates
   * @returns {Promise<Trigger|null>}
   */
  async update(triggerId, updates) {
    if (this._repo) {
      try {
        return await this._repo.update(triggerId, updates);
      } catch (err) {
        console.error('Error updating trigger:', err.message);
      }
    }
    return null;
  }

  /**
   * Enable a trigger
   * @param {string} triggerId
   * @returns {Promise<Trigger|null>}
   */
  async enable(triggerId) {
    if (this._repo) {
      try {
        return await this._repo.enable(triggerId);
      } catch (err) {
        console.error('Error enabling trigger:', err.message);
      }
    }
    return null;
  }

  /**
   * Disable a trigger
   * @param {string} triggerId
   * @returns {Promise<Trigger|null>}
   */
  async disable(triggerId) {
    if (this._repo) {
      try {
        return await this._repo.disable(triggerId);
      } catch (err) {
        console.error('Error disabling trigger:', err.message);
      }
    }
    return null;
  }

  /**
   * Delete a trigger
   * @param {string} triggerId
   * @returns {Promise<boolean>}
   */
  async delete(triggerId) {
    if (this._repo) {
      try {
        return await this._repo.delete(triggerId);
      } catch (err) {
        console.error('Error deleting trigger:', err.message);
      }
    }
    return false;
  }

  /**
   * Record a trigger execution
   * @param {Object} execution
   * @returns {Promise<Object|null>}
   */
  async recordExecution(execution) {
    if (this._repo) {
      try {
        return await this._repo.recordExecution(execution);
      } catch (err) {
        console.error('Error recording trigger execution:', err.message);
      }
    }
    return null;
  }

  /**
   * Store a trigger event
   * @param {Object} event
   * @returns {Promise<Object|null>}
   */
  async storeEvent(event) {
    if (this._repo) {
      try {
        return await this._repo.storeEvent(event);
      } catch (err) {
        console.error('Error storing trigger event:', err.message);
      }
    }
    return null;
  }

  /**
   * Get trigger statistics
   * @returns {Promise<Object>}
   */
  async getStats() {
    if (this._repo) {
      try {
        return await this._repo.getStats();
      } catch (err) {
        console.error('Error getting trigger stats:', err.message);
      }
    }
    return { totalTriggers: 0, enabledTriggers: 0 };
  }

  /**
   * Get active triggers summary
   * @returns {Promise<Object[]>}
   */
  async getActiveSummary() {
    if (this._repo) {
      try {
        return await this._repo.getActiveSummary();
      } catch (err) {
        console.error('Error getting active triggers summary:', err.message);
      }
    }
    return [];
  }

  /**
   * Count executions in last minute (rate limiting)
   * @param {string} triggerId
   * @returns {Promise<number>}
   */
  async countExecutionsLastMinute(triggerId) {
    if (this._repo) {
      try {
        return await this._repo.countExecutionsLastMinute(triggerId);
      } catch (err) {
        console.error('Error checking trigger rate limit:', err.message);
      }
    }
    return 0;
  }

  /**
   * Legacy: Get triggers state (backward compatibility)
   * @deprecated Use getEnabled() instead
   */
  async getState() {
    if (this._fallback) {
      return await this._fallback.getTriggersState();
    }
    return null;
  }

  /**
   * Legacy: Save triggers state (backward compatibility)
   * @deprecated Use create()/update() instead
   */
  async saveState(state) {
    if (this._fallback) {
      return await this._fallback.saveTriggersState(state);
    }
    return null;
  }

  /**
   * Check if adapter is available
   */
  get isAvailable() {
    return !!this._repo || !!this._fallback;
  }
}
