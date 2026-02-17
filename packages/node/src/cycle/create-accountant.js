/**
 * Accountant Factory — creates domain-specific Accountant classes from config
 *
 * Template code (~60% of each Accountant) lives here ONCE.
 * Domain logic (~40%) lives in config objects.
 *
 * Usage:
 *   const { Class, getInstance, resetInstance } = createAccountant(cynicAccountantConfig);
 *
 * "Le chien compte ses ressources" — one factory, N domains
 *
 * @module @cynic/node/cycle/create-accountant
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus } from '@cynic/core';
import { createStatsTracker, updateStats, pushHistory, roundTo } from '@cynic/core';

/**
 * Create an Accountant class from a domain config.
 *
 * @param {Object} config - Domain-specific configuration
 * @param {string} config.name - Accountant name (e.g. 'CynicAccountant')
 * @param {string} config.cell - Matrix cell (e.g. 'C6.6')
 * @param {string} config.dimension - Domain name (e.g. 'CYNIC')
 * @param {string} config.eventPrefix - Event name prefix (e.g. 'cynic')
 * @param {Object} config.categories - Enum of accounting categories
 * @param {Function} config.account - (category, data, context) → cost record
 * @param {Function} [config.enrichRecord] - (record, category, data) → void [optional]
 * @param {Function} [config.healthCheck] - (stats) → health object [optional]
 * @param {number} [config.maxHistory=500] - Max history size
 * @param {string[]} [config.extraStatFields=[]] - Additional stat counter fields
 * @param {Function} [config.init] - (instance, options) → void [optional]
 * @param {Object} [config.prototype] - Extra methods to add to the class prototype
 * @returns {{ Class, getInstance, resetInstance }}
 */
export function createAccountant(config) {
  const log = createLogger(config.name);
  const maxHistory = config.maxHistory || 500;
  const categoryValues = Object.values(config.categories);

  class DomainAccountant extends EventEmitter {
    constructor(options = {}) {
      super();

      this._history = [];
      this._maxHistory = maxHistory;

      // Stats tracking
      this._stats = createStatsTracker(config.categories, [
        'totalCost',
        'totalValue',
        ...(config.extraStatFields || []),
      ]);

      // Summary accumulators
      this._summary = {
        totalOperations: 0,
        totalCostUSD: 0,
        totalTokens: 0,
        totalComputeMs: 0,
        totalAPIcalls: 0,
        byCategory: {},
      };

      for (const cat of categoryValues) {
        this._summary.byCategory[cat] = {
          operations: 0,
          costUSD: 0,
          tokens: 0,
        };
      }

      if (config.init) config.init(this, options);
    }

    /**
     * Record a cost or value event.
     *
     * @param {Object} event - What to account
     * @param {string} [event.category] - Category from config.categories
     * @param {Object} [event.data] - Category-specific data
     * @param {Object} [context] - Additional context
     * @returns {Object} Accounting record
     */
    recordCost(event, context = {}) {
      const category = event.category || categoryValues[0];
      const data = event.data || event;

      // Delegate to domain accounting logic
      const costRecord = config.account(category, data, context);

      // Default structure if domain doesn't provide
      const record = {
        timestamp: Date.now(),
        category,
        cell: config.cell,
        dimension: config.dimension,
        analysis: 'ACCOUNT',
        costUSD: costRecord.costUSD || 0,
        tokens: costRecord.tokens || 0,
        computeMs: costRecord.computeMs || 0,
        apiCalls: costRecord.apiCalls || 0,
        valueScore: costRecord.valueScore || 0,
        ...costRecord,
      };

      // Optional enrichment
      if (config.enrichRecord) config.enrichRecord(record, category, data);

      // Update stats
      this._stats.total++;
      if (this._stats.byType[category] !== undefined) {
        this._stats.byType[category]++;
      }
      this._stats.lastTimestamp = record.timestamp;
      this._stats.totalCost += record.costUSD;
      this._stats.totalValue += record.valueScore || 0;

      // Update summary
      this._summary.totalOperations++;
      this._summary.totalCostUSD += record.costUSD;
      this._summary.totalTokens += record.tokens;
      this._summary.totalComputeMs += record.computeMs;
      this._summary.totalAPIcalls += record.apiCalls;

      const catSummary = this._summary.byCategory[category];
      if (catSummary) {
        catSummary.operations++;
        catSummary.costUSD += record.costUSD;
        catSummary.tokens += record.tokens;
      }

      // History
      pushHistory(this._history, record, this._maxHistory);

      // Emit locally
      this.emit('cost_recorded', record);

      // Publish to global bus
      if (typeof globalEventBus.publish === 'function') {
        globalEventBus.publish(`${config.eventPrefix}:cost`, record, { source: config.name });
      } else {
        globalEventBus.emit(`${config.eventPrefix}:cost`, record);
      }

      log.debug(`${config.name} cost recorded`, {
        category,
        costUSD: roundTo(record.costUSD, 4),
        tokens: record.tokens,
      });

      return record;
    }

    getStats() { return { ...this._stats }; }

    getSummary() {
      return {
        ...this._summary,
        totalCostUSD: roundTo(this._summary.totalCostUSD, 4),
        avgCostPerOperation: this._summary.totalOperations > 0
          ? roundTo(this._summary.totalCostUSD / this._summary.totalOperations, 6)
          : 0,
      };
    }

    getHistory(limit = 21) {
      return this._history.slice(-limit);
    }

    getHealth() {
      if (config.healthCheck) return config.healthCheck(this._stats, this._summary);

      const total = this._stats.total;
      const avgCost = total > 0 ? this._summary.totalCostUSD / total : 0;

      return {
        status: total > 0 ? 'tracking' : 'idle',
        score: Math.min(PHI_INV, 1 - Math.min(avgCost * 100, 1)), // Lower cost = higher health
        totalRecords: total,
        avgCostUSD: roundTo(avgCost, 6),
      };
    }

    clear() {
      this._history = [];
      this._stats.total = 0;
      this._stats.totalCost = 0;
      this._stats.totalValue = 0;
      this._stats.lastTimestamp = null;
      for (const k of Object.keys(this._stats.byType)) this._stats.byType[k] = 0;

      this._summary.totalOperations = 0;
      this._summary.totalCostUSD = 0;
      this._summary.totalTokens = 0;
      this._summary.totalComputeMs = 0;
      this._summary.totalAPIcalls = 0;
      for (const cat of categoryValues) {
        this._summary.byCategory[cat] = { operations: 0, costUSD: 0, tokens: 0 };
      }
    }
  }

  // Attach extra prototype methods from config
  if (config.prototype) {
    for (const [name, fn] of Object.entries(config.prototype)) {
      DomainAccountant.prototype[name] = fn;
    }
  }

  // Singleton management
  let _instance = null;

  function getInstance(options = {}) {
    if (!_instance) _instance = new DomainAccountant(options);
    return _instance;
  }

  function resetInstance() {
    if (_instance) _instance.removeAllListeners();
    _instance = null;
  }

  return {
    Class: DomainAccountant,
    getInstance,
    resetInstance,
  };
}
