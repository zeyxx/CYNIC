/**
 * φ-Utility Library — Recurring patterns consolidated
 *
 * "Don't extract, burn" — these patterns existed in 150+ files.
 * Now they exist ONCE.
 *
 * @module @cynic/core/axioms/phi-utils
 */

'use strict';

import { PHI_INV, PHI_INV_2, THRESHOLDS } from './constants.js';

// =============================================================================
// BOUNDING
// =============================================================================

/**
 * Clamp value to [0, PHI_INV] (0 to 61.8%)
 * The most repeated pattern in the codebase (58+ files).
 *
 * Replaces: Math.min(PHI_INV, Math.max(0, value))
 *
 * @param {number} value - Value to bound
 * @returns {number} Value clamped to [0, 0.618]
 */
export function phiBound(value) {
  if (typeof value !== 'number' || Number.isNaN(value)) return 0;
  return Math.min(PHI_INV, Math.max(0, value));
}

/**
 * Clamp value to [0, max] where max defaults to PHI_INV.
 * For cases where the ceiling differs from 61.8%.
 *
 * @param {number} value - Value to bound
 * @param {number} [max=PHI_INV] - Upper bound
 * @returns {number} Clamped value
 */
export function phiBoundTo(value, max = PHI_INV) {
  if (typeof value !== 'number' || Number.isNaN(value)) return 0;
  return Math.min(max, Math.max(0, value));
}

/**
 * General clamp between min and max.
 *
 * Replaces: Math.min(max, Math.max(min, value))
 *
 * @param {number} value - Value to clamp
 * @param {number} min - Floor
 * @param {number} max - Ceiling
 * @returns {number} Clamped value
 */
export function clamp(value, min, max) {
  if (typeof value !== 'number' || Number.isNaN(value)) return min;
  return Math.min(max, Math.max(min, value));
}

// =============================================================================
// CLASSIFICATION
// =============================================================================

/**
 * Classify a score into HOWL/WAG/GROWL/BARK verdict.
 * Uses THRESHOLDS from constants.js (HOWL=80, WAG=50, GROWL=38, BARK=0).
 *
 * Replaces: repeated if/else chains in 3+ judge files.
 *
 * @param {number} score - Score 0-100
 * @returns {string} 'HOWL' | 'WAG' | 'GROWL' | 'BARK'
 */
export function phiClassify(score) {
  if (score >= THRESHOLDS.HOWL) return 'HOWL';
  if (score >= THRESHOLDS.WAG) return 'WAG';
  if (score >= THRESHOLDS.GROWL) return 'GROWL';
  return 'BARK';
}

/**
 * Classify a decimal confidence into health status.
 *
 * @param {number} confidence - Confidence 0-1
 * @returns {string} 'healthy' | 'warning' | 'critical' | 'failing'
 */
export function phiHealthStatus(confidence) {
  if (confidence >= PHI_INV) return 'healthy';
  if (confidence >= PHI_INV_2) return 'warning';
  if (confidence > 0) return 'critical';
  return 'failing';
}

// =============================================================================
// ROUNDING
// =============================================================================

/**
 * Round to N decimal places.
 *
 * Replaces: Math.round(x * 1000) / 1000  (27+ files)
 *
 * @param {number} value - Value to round
 * @param {number} [decimals=3] - Decimal places
 * @returns {number} Rounded value
 */
export function roundTo(value, decimals = 3) {
  if (typeof value !== 'number' || Number.isNaN(value)) return 0;
  const factor = Math.pow(10, decimals);
  return Math.round(value * factor) / factor;
}

// =============================================================================
// HISTORY MANAGEMENT
// =============================================================================

/**
 * Push item to array, trim to maxSize (FIFO eviction).
 *
 * Replaces:
 *   this._history.push(item);
 *   while (this._history.length > this._maxHistory) this._history.shift();
 *
 * Found in 12+ files with identical pattern.
 *
 * @param {Array} array - Target array (mutated in place)
 * @param {*} item - Item to push
 * @param {number} maxSize - Maximum array length
 * @returns {Array} The same array (for chaining)
 */
export function pushHistory(array, item, maxSize) {
  array.push(item);
  while (array.length > maxSize) array.shift();
  return array;
}

// =============================================================================
// SINGLETON
// =============================================================================

/**
 * Create a singleton accessor for a class.
 *
 * Replaces the 27-instance pattern:
 *   let _instance = null;
 *   export function getX(opts) { if (!_instance) _instance = new X(opts); return _instance; }
 *   export function resetX() { if (_instance) _instance.removeAllListeners(); _instance = null; }
 *
 * @param {Function} ClassConstructor - The class to singleton-ify
 * @param {Object} [options] - Configuration
 * @param {Function} [options.cleanup] - Custom cleanup (default: removeAllListeners)
 * @returns {{ getInstance: Function, resetInstance: Function }}
 */
export function createSingleton(ClassConstructor, options = {}) {
  let _instance = null;
  const cleanup = options.cleanup || ((inst) => {
    if (typeof inst.removeAllListeners === 'function') inst.removeAllListeners();
  });

  return {
    getInstance(opts = {}) {
      if (!_instance) _instance = new ClassConstructor(opts);
      return _instance;
    },
    resetInstance() {
      if (_instance) {
        cleanup(_instance);
        _instance = null;
      }
    },
  };
}

// =============================================================================
// STATS TRACKING
// =============================================================================

/**
 * Initialize stats object from an enum/object of types.
 *
 * Replaces:
 *   this._stats = { total: 0, byType: {} };
 *   for (const type of Object.values(TypeEnum)) stats.byType[type] = 0;
 *
 * Found in 20+ files.
 *
 * @param {Object} typeEnum - Object whose values are the type strings
 * @param {string[]} [extraFields=[]] - Additional counter fields
 * @returns {Object} Stats object with total, byType, lastTimestamp, and extra fields
 */
export function createStatsTracker(typeEnum, extraFields = []) {
  const stats = {
    total: 0,
    byType: {},
    lastTimestamp: null,
  };
  for (const type of Object.values(typeEnum)) {
    stats.byType[type] = 0;
  }
  for (const field of extraFields) {
    stats[field] = 0;
  }
  return stats;
}

/**
 * Update stats with a result. Increments total, byType[result.type], sets lastTimestamp.
 *
 * @param {Object} stats - Stats object from createStatsTracker
 * @param {Object} result - Result with .type field
 * @returns {Object} The same stats object
 */
export function updateStats(stats, result) {
  stats.total++;
  if (result.type && stats.byType[result.type] !== undefined) {
    stats.byType[result.type]++;
  }
  stats.lastTimestamp = result.timestamp || Date.now();
  return stats;
}

// =============================================================================
// COOLDOWN
// =============================================================================

/**
 * Create a cooldown tracker.
 *
 * Replaces:
 *   this._lastAction = new Map();
 *   _isOnCooldown(type) { const last = this._lastAction.get(type); ... }
 *
 * Found in 6+ files.
 *
 * @param {Object} cooldowns - Map of type → cooldown ms
 * @param {number} [defaultMs=300000] - Default cooldown (5 minutes)
 * @returns {{ isOnCooldown: Function, record: Function, reset: Function }}
 */
export function createCooldownTracker(cooldowns = {}, defaultMs = 300000) {
  const _lastAction = new Map();

  return {
    /**
     * Check if an action type is on cooldown.
     * @param {string} actionType
     * @returns {boolean}
     */
    isOnCooldown(actionType) {
      const last = _lastAction.get(actionType);
      if (!last) return false;
      return (Date.now() - last) < (cooldowns[actionType] || defaultMs);
    },

    /**
     * Record that an action was taken (starts cooldown).
     * @param {string} actionType
     */
    record(actionType) {
      _lastAction.set(actionType, Date.now());
    },

    /**
     * Reset all cooldowns.
     */
    reset() {
      _lastAction.clear();
    },

    /**
     * Get the underlying map (for serialization/debugging).
     * @returns {Map}
     */
    getState() {
      return new Map(_lastAction);
    },
  };
}

// =============================================================================
// CALIBRATION
// =============================================================================

/**
 * Apply calibration adjustment to confidence based on outcome history.
 *
 * Replaces the _applyCalibration() method duplicated in 4+ Decider files:
 *   successRate = outcomes.filter(success).length / total
 *   gap = successRate - baseConfidence
 *   adjustment = clamp(gap * PHI_INV, -maxAdjust, +maxAdjust)
 *   return phiBound(baseConfidence + adjustment)
 *
 * @param {Array<{result: string}>} outcomes - Array with .result field ('success' or other)
 * @param {number} baseConfidence - The uncalibrated confidence (0-1)
 * @param {Object} [opts] - Options
 * @param {number} [opts.minSamples=5] - Minimum outcomes before calibration kicks in
 * @param {number} [opts.maxAdjust=0.2] - Maximum adjustment magnitude
 * @param {string} [opts.successValue='success'] - What counts as success
 * @returns {number} Calibrated confidence, φ-bounded
 */
export function applyCalibration(outcomes, baseConfidence, opts = {}) {
  const { minSamples = 5, maxAdjust = 0.2, successValue = 'success' } = opts;

  if (!outcomes || outcomes.length < minSamples) return phiBound(baseConfidence);

  const successCount = outcomes.filter(o => o.result === successValue).length;
  const successRate = successCount / outcomes.length;
  const gap = successRate - baseConfidence;
  const adjustment = clamp(gap * PHI_INV, -maxAdjust, maxAdjust);

  return phiBound(baseConfidence + adjustment);
}

// =============================================================================
// DEFAULT EXPORT
// =============================================================================

export default {
  phiBound,
  phiBoundTo,
  clamp,
  phiClassify,
  phiHealthStatus,
  roundTo,
  pushHistory,
  createSingleton,
  createStatsTracker,
  updateStats,
  createCooldownTracker,
  applyCalibration,
};
