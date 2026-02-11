/**
 * Decider Factory — creates domain-specific Decider classes from config
 *
 * Template code (~40% of each Decider) lives here ONCE.
 * Domain logic (~60%) lives in config objects.
 *
 * Unlike Actors (uniform act() signature), Deciders have divergent
 * decide() methods — so the factory delegates decide() fully to config.
 *
 * Usage:
 *   const { Class, getInstance, resetInstance } = createDecider(cosmosDeciderConfig);
 *
 * "Le code factorise le jugement" — one factory, N decision domains
 *
 * @module @cynic/node/cycle/create-decider
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, createLogger, globalEventBus } from '@cynic/core';

/**
 * Create a Decider class from a domain config.
 *
 * @param {Object} config - Domain-specific configuration
 * @param {string} config.name - Decider name (e.g. 'CosmosDecider')
 * @param {string} config.cell - Matrix cell (e.g. 'C7.3')
 * @param {string} config.dimension - Domain name (e.g. 'COSMOS')
 * @param {string} config.eventPrefix - Event name prefix (e.g. 'cosmos')
 * @param {Object} config.decisionTypes - Enum of decision types
 * @param {number} [config.maxHistory=89] - Max history size (Fibonacci)
 * @param {string[]} [config.extraStatFields=[]] - Additional stat counter fields
 * @param {number} [config.minCalibrationSamples=5] - Min outcomes before calibration
 * @param {number} [config.calibrationClamp=0.15] - Max calibration adjustment
 * @param {string} [config.calibrationGroupBy='decisionType'] - Group calibration by this result field
 * @param {Function} config.decide - (input, context, decider) → result or null
 * @param {Function} [config.init] - (decider, options) → void — domain-specific init
 * @param {Function} [config.updateExtraStats] - (stats, result) → void
 * @param {Function} config.getHealth - (decider) → health object
 * @returns {{ Class, getInstance, resetInstance }}
 */
export function createDecider(config) {
  const log = createLogger(config.name);
  const maxHistory = config.maxHistory || 89;
  const minCalibrationSamples = config.minCalibrationSamples || 5;
  const calibrationClamp = config.calibrationClamp || 0.15;
  const calibrationGroupBy = config.calibrationGroupBy || 'decisionType';

  class DomainDecider extends EventEmitter {
    constructor(options = {}) {
      super();

      this._history = [];
      this._maxHistory = maxHistory;
      this._outcomes = [];

      this._stats = {
        decisionsTotal: 0,
        byType: {},
        outcomesRecorded: 0,
        lastDecision: null,
      };

      for (const type of Object.values(config.decisionTypes)) {
        this._stats.byType[type] = 0;
      }

      for (const field of (config.extraStatFields || [])) {
        this._stats[field] = 0;
      }

      // Domain-specific init
      if (config.init) config.init(this, options);
    }

    /**
     * Execute a decision. Fully delegated to config.decide().
     *
     * @param {Object} input - Domain-specific input (change, judgment, pattern, etc.)
     * @param {Object} [context] - Additional context
     * @returns {Object|null} Decision result or null
     */
    decide(input, context = {}) {
      return config.decide.call(this, input, context, this);
    }

    /**
     * Record and publish a decision result (called by config.decide).
     * Handles history, stats, events — so config doesn't have to.
     *
     * @param {Object} result - The decision result to record
     * @returns {Object} The same result, enriched with cell/dimension/analysis
     */
    recordDecision(result) {
      // Enrich with matrix coordinates
      result.cell = result.cell || config.cell;
      result.dimension = result.dimension || config.dimension;
      result.analysis = result.analysis || 'DECIDE';
      result.timestamp = result.timestamp || Date.now();

      // Update stats
      this._stats.decisionsTotal++;
      const typeField = result.type || result.decision;
      this._stats.byType[typeField] = (this._stats.byType[typeField] || 0) + 1;
      this._stats.lastDecision = Date.now();

      if (config.updateExtraStats) config.updateExtraStats(this._stats, result);

      // History
      this._history.push(result);
      while (this._history.length > this._maxHistory) this._history.shift();

      // Emit locally
      this.emit('decision', result);

      // Publish to global bus
      if (typeof globalEventBus.publish === 'function') {
        globalEventBus.publish(`${config.eventPrefix}:decision`, result, { source: config.name });
      } else {
        globalEventBus.emit(`${config.eventPrefix}:decision`, result);
      }

      log.debug(`${config.name} decision`, {
        type: typeField,
        confidence: result.confidence,
      });

      return result;
    }

    /**
     * Record the outcome of a previous decision for calibration.
     *
     * @param {Object} outcome
     * @param {string} outcome.decisionType - Type of decision
     * @param {string} outcome.result - 'success', 'partial', or 'failure'
     * @param {string} [outcome.reason] - Why it succeeded/failed
     */
    recordOutcome(outcome) {
      const entry = {
        decisionType: outcome.decisionType || 'unknown',
        riskLevel: outcome.riskLevel,
        result: outcome.result || 'unknown',
        reason: outcome.reason || null,
        confidence: outcome.confidence,
        ts: Date.now(),
      };

      this._outcomes.push(entry);
      while (this._outcomes.length > this._maxHistory) this._outcomes.shift();
      this._stats.outcomesRecorded++;

      this.emit('outcome_recorded', entry);
    }

    /**
     * Apply outcome-based calibration adjustment.
     *
     * @param {string} groupKey - Value to group outcomes by (e.g. 'approve', 'trivial')
     * @param {number} baseConfidence - Starting confidence (0-1)
     * @returns {number} Adjusted confidence, phi-bounded
     */
    applyCalibration(groupKey, baseConfidence) {
      const relevant = this._outcomes.filter(o => o[calibrationGroupBy] === groupKey);
      if (relevant.length < minCalibrationSamples) return baseConfidence;

      const successRate = relevant.filter(o => o.result === 'success').length / relevant.length;
      const gap = successRate - baseConfidence;

      const adjustment = Math.max(-calibrationClamp, Math.min(calibrationClamp, gap * PHI_INV));
      return Math.max(0, Math.min(PHI_INV, baseConfidence + adjustment));
    }

    getCalibrationSummary() {
      const summary = {};
      for (const type of Object.values(config.decisionTypes)) {
        const relevant = this._outcomes.filter(o => o[calibrationGroupBy] === type);
        if (relevant.length === 0) continue;
        summary[type] = {
          outcomes: relevant.length,
          successRate: relevant.filter(o => o.result === 'success').length / relevant.length,
        };
      }
      return summary;
    }

    getStats() {
      return {
        ...this._stats,
        historySize: this._history.length,
        outcomesSize: this._outcomes.length,
        calibration: this.getCalibrationSummary(),
      };
    }

    getHistory(limit = 21) {
      return this._history.slice(-limit);
    }

    getHealth() {
      return config.getHealth(this);
    }
  }

  // Singleton management
  let _instance = null;

  function getInstance(options = {}) {
    if (!_instance) _instance = new DomainDecider(options);
    return _instance;
  }

  function resetInstance() {
    if (_instance) _instance.removeAllListeners();
    _instance = null;
  }

  return {
    Class: DomainDecider,
    getInstance,
    resetInstance,
  };
}
