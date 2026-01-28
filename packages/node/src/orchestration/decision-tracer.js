/**
 * DecisionTracer - Decision Path Recording and Visualization
 *
 * Records all orchestration decisions for:
 * - Debugging: See exactly what happened
 * - Learning: Feed outcomes to improve
 * - Transparency: Show users why decisions were made
 *
 * "φ traces every step" - κυνικός
 *
 * @module @cynic/node/orchestration/decision-tracer
 */

'use strict';

import { createLogger } from '@cynic/core';
import { DecisionEvent, DecisionOutcome } from './decision-event.js';

const log = createLogger('DecisionTracer');

/**
 * Trace storage modes
 */
export const StorageMode = {
  MEMORY: 'memory',       // In-memory only (default)
  PERSISTENCE: 'persist', // Persist to database
  HYBRID: 'hybrid',       // Memory + async persist
};

/**
 * DecisionTracer - Records and retrieves decision traces
 */
export class DecisionTracer {
  /**
   * Create the tracer
   *
   * @param {Object} options - Options
   * @param {string} [options.mode] - Storage mode (memory, persist, hybrid)
   * @param {number} [options.maxTraces] - Max traces to keep in memory
   * @param {Object} [options.persistence] - PersistenceManager for persist mode
   */
  constructor(options = {}) {
    this.mode = options.mode || StorageMode.MEMORY;
    this.maxTraces = options.maxTraces || 1000;
    this.persistence = options.persistence || null;

    // In-memory trace storage (circular buffer approach)
    this._traces = new Map();
    this._traceOrder = []; // Maintain insertion order for LRU

    // Index by various keys
    this._byUser = new Map();     // userId → [traceIds]
    this._byOutcome = new Map();  // outcome → [traceIds]
    this._byDomain = new Map();   // domain → [traceIds]

    // Statistics
    this.stats = {
      recorded: 0,
      retrieved: 0,
      evicted: 0,
      persisted: 0,
    };

    log.debug('DecisionTracer created', { mode: this.mode, maxTraces: this.maxTraces });
  }

  /**
   * Record a decision event
   *
   * @param {DecisionEvent} event - The decision event to record
   */
  async record(event) {
    if (!(event instanceof DecisionEvent)) {
      log.warn('Invalid event type, expected DecisionEvent');
      return;
    }

    const traceId = event.id;

    // Store in memory
    this._traces.set(traceId, event);
    this._traceOrder.push(traceId);

    // Update indexes
    this._indexTrace(event);

    // Evict old traces if needed
    this._evictIfNeeded();

    this.stats.recorded++;

    // Persist if hybrid/persist mode
    if (this.mode !== StorageMode.MEMORY && this.persistence) {
      await this._persist(event);
    }

    log.trace('Decision recorded', { traceId, outcome: event.outcome });
  }

  /**
   * Get a decision trace by ID
   *
   * @param {string} traceId - Decision ID
   * @returns {DecisionEvent|null}
   */
  get(traceId) {
    this.stats.retrieved++;
    return this._traces.get(traceId) || null;
  }

  /**
   * Get recent decisions
   *
   * @param {number} [limit=10] - Max decisions to return
   * @returns {DecisionEvent[]}
   */
  getRecent(limit = 10) {
    const ids = this._traceOrder.slice(-limit).reverse();
    return ids.map(id => this._traces.get(id)).filter(Boolean);
  }

  /**
   * Get decisions by user
   *
   * @param {string} userId - User ID
   * @param {number} [limit=10] - Max decisions
   * @returns {DecisionEvent[]}
   */
  getByUser(userId, limit = 10) {
    const ids = this._byUser.get(userId) || [];
    return ids.slice(-limit).reverse()
      .map(id => this._traces.get(id))
      .filter(Boolean);
  }

  /**
   * Get decisions by outcome
   *
   * @param {DecisionOutcome} outcome - Outcome filter
   * @param {number} [limit=10] - Max decisions
   * @returns {DecisionEvent[]}
   */
  getByOutcome(outcome, limit = 10) {
    const ids = this._byOutcome.get(outcome) || [];
    return ids.slice(-limit).reverse()
      .map(id => this._traces.get(id))
      .filter(Boolean);
  }

  /**
   * Get decisions by domain
   *
   * @param {string} domain - Domain filter
   * @param {number} [limit=10] - Max decisions
   * @returns {DecisionEvent[]}
   */
  getByDomain(domain, limit = 10) {
    const ids = this._byDomain.get(domain) || [];
    return ids.slice(-limit).reverse()
      .map(id => this._traces.get(id))
      .filter(Boolean);
  }

  /**
   * Get blocked decisions
   *
   * @param {number} [limit=10] - Max decisions
   * @returns {DecisionEvent[]}
   */
  getBlocked(limit = 10) {
    return this.getByOutcome(DecisionOutcome.BLOCK, limit);
  }

  /**
   * Search decisions
   *
   * @param {Object} filters - Search filters
   * @param {string} [filters.userId] - User ID
   * @param {string} [filters.outcome] - Outcome
   * @param {string} [filters.domain] - Domain
   * @param {number} [filters.since] - Timestamp (since)
   * @param {number} [filters.until] - Timestamp (until)
   * @param {number} [filters.limit] - Max results
   * @returns {DecisionEvent[]}
   */
  search(filters = {}) {
    let results = Array.from(this._traces.values());

    // Apply filters
    if (filters.userId) {
      results = results.filter(e => e.userContext?.userId === filters.userId);
    }
    if (filters.outcome) {
      results = results.filter(e => e.outcome === filters.outcome);
    }
    if (filters.domain) {
      results = results.filter(e => e.routing?.domain === filters.domain);
    }
    if (filters.since) {
      results = results.filter(e => e.timestamp >= filters.since);
    }
    if (filters.until) {
      results = results.filter(e => e.timestamp <= filters.until);
    }

    // Sort by timestamp descending
    results.sort((a, b) => b.timestamp - a.timestamp);

    // Apply limit
    if (filters.limit) {
      results = results.slice(0, filters.limit);
    }

    return results;
  }

  /**
   * Get formatted trace for display
   *
   * @param {string} traceId - Decision ID
   * @returns {string|null} Formatted trace
   */
  getFormattedTrace(traceId) {
    const event = this.get(traceId);
    if (!event) return null;
    return event.getFormattedTrace();
  }

  /**
   * Get summary statistics
   *
   * @returns {Object} Summary
   */
  getSummary() {
    const outcomes = {};
    const domains = {};

    for (const event of this._traces.values()) {
      // Count outcomes
      outcomes[event.outcome] = (outcomes[event.outcome] || 0) + 1;

      // Count domains
      const domain = event.routing?.domain || 'unknown';
      domains[domain] = (domains[domain] || 0) + 1;
    }

    return {
      total: this._traces.size,
      outcomes,
      domains,
      stats: { ...this.stats },
    };
  }

  /**
   * Clear all traces
   */
  clear() {
    this._traces.clear();
    this._traceOrder = [];
    this._byUser.clear();
    this._byOutcome.clear();
    this._byDomain.clear();
    log.debug('Traces cleared');
  }

  /**
   * Export traces to JSON
   *
   * @param {Object} [filters] - Optional filters
   * @returns {Object[]} JSON-serializable traces
   */
  export(filters = {}) {
    const events = filters ? this.search(filters) : Array.from(this._traces.values());
    return events.map(e => e.toJSON());
  }

  /**
   * Import traces from JSON
   *
   * @param {Object[]} traces - JSON traces
   */
  async import(traces) {
    for (const json of traces) {
      const event = DecisionEvent.fromJSON(json);
      await this.record(event);
    }
    log.debug('Traces imported', { count: traces.length });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRIVATE METHODS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Index a trace for fast lookup
   * @private
   */
  _indexTrace(event) {
    const traceId = event.id;

    // Index by user
    const userId = event.userContext?.userId;
    if (userId) {
      if (!this._byUser.has(userId)) {
        this._byUser.set(userId, []);
      }
      this._byUser.get(userId).push(traceId);
    }

    // Index by outcome
    const outcome = event.outcome;
    if (!this._byOutcome.has(outcome)) {
      this._byOutcome.set(outcome, []);
    }
    this._byOutcome.get(outcome).push(traceId);

    // Index by domain
    const domain = event.routing?.domain;
    if (domain) {
      if (!this._byDomain.has(domain)) {
        this._byDomain.set(domain, []);
      }
      this._byDomain.get(domain).push(traceId);
    }
  }

  /**
   * Evict old traces if over limit
   * @private
   */
  _evictIfNeeded() {
    while (this._traces.size > this.maxTraces) {
      const oldestId = this._traceOrder.shift();
      if (oldestId) {
        const event = this._traces.get(oldestId);
        this._traces.delete(oldestId);

        // Clean up indexes
        this._removeFromIndexes(oldestId, event);

        this.stats.evicted++;
      }
    }
  }

  /**
   * Remove trace from indexes
   * @private
   */
  _removeFromIndexes(traceId, event) {
    // Remove from user index
    const userId = event?.userContext?.userId;
    if (userId && this._byUser.has(userId)) {
      const arr = this._byUser.get(userId);
      const idx = arr.indexOf(traceId);
      if (idx >= 0) arr.splice(idx, 1);
    }

    // Remove from outcome index
    const outcome = event?.outcome;
    if (outcome && this._byOutcome.has(outcome)) {
      const arr = this._byOutcome.get(outcome);
      const idx = arr.indexOf(traceId);
      if (idx >= 0) arr.splice(idx, 1);
    }

    // Remove from domain index
    const domain = event?.routing?.domain;
    if (domain && this._byDomain.has(domain)) {
      const arr = this._byDomain.get(domain);
      const idx = arr.indexOf(traceId);
      if (idx >= 0) arr.splice(idx, 1);
    }
  }

  /**
   * Persist trace to database
   * @private
   */
  async _persist(event) {
    if (!this.persistence) return;

    try {
      // Use OrchestrationDecisionRepository if available
      if (this.persistence.orchestrationDecisions) {
        await this.persistence.orchestrationDecisions.recordEvent(event);
        this.stats.persisted++;
        log.trace('Decision persisted', { traceId: event.id });
      } else if (this.persistence.query) {
        // Fallback to direct query (basic fields only)
        await this.persistence.query(
          `INSERT INTO orchestration_log (event_type, user_id, sefirah, intervention, risk_level, outcome, domain, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())`,
          [
            event.eventType,
            event.userContext?.userId,
            event.routing?.sefirah || 'Keter',
            event.intervention?.level,
            event.intervention?.actionRisk || 'low',
            event.outcome,
            event.routing?.domain,
          ]
        );
        this.stats.persisted++;
      }
    } catch (err) {
      log.warn('Failed to persist trace', { traceId: event.id, error: err.message });
    }
  }

  /**
   * Set the persistence manager
   *
   * @param {Object} persistence - PersistenceManager with orchestrationDecisions repository
   */
  setPersistence(persistence) {
    this.persistence = persistence;
    log.debug('Persistence set', { hasRepo: !!persistence?.orchestrationDecisions });
  }
}

/**
 * Create a DecisionTracer instance
 *
 * @param {Object} options - Options
 * @returns {DecisionTracer}
 */
export function createDecisionTracer(options) {
  return new DecisionTracer(options);
}

export default DecisionTracer;
