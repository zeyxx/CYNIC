/**
 * Growth - Adaptation and Learning Rate
 *
 * "Le chien grandit en chassant, pas en dormant" - κυνικός
 *
 * Measures how CYNIC grows and adapts:
 * - Pattern acquisition rate
 * - Knowledge accumulation
 * - Skill improvement
 * - Behavioral adaptation
 *
 * Positive growth = learning, improving
 * Negative growth = forgetting, regressing
 * Zero growth = stagnation
 *
 * @module @cynic/node/organism/growth
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { computeStats } from '../inference/gaussian.js';
import { estimateRate } from '../inference/poisson.js';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

export const GROWTH_CONFIG = {
  // Growth rate thresholds (per hour)
  RATE_RAPID: 10,           // > 10 patterns/hour = rapid growth
  RATE_HEALTHY: 3,          // > 3 patterns/hour = healthy growth
  RATE_SLOW: 1,             // > 1 pattern/hour = slow growth
  RATE_STAGNANT: 0,         // = 0 = stagnation

  // Growth types
  TYPES: {
    PATTERN: 'pattern',     // New pattern discovered
    KNOWLEDGE: 'knowledge', // Knowledge acquired
    SKILL: 'skill',         // Skill improved
    ADAPTATION: 'adaptation', // Behavior adapted
    INSIGHT: 'insight',     // Cross-domain insight
  },

  // Time window for rate calculation (ms)
  RATE_WINDOW: 3600000,     // 1 hour

  // Maximum history size
  MAX_EVENTS: 1000,

  // Growth momentum (exponential smoothing)
  MOMENTUM_ALPHA: PHI_INV_3, // 23.6%
};

// ═══════════════════════════════════════════════════════════════════════════════
// GROWTH EVENT
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Single growth event
 */
export class GrowthEvent {
  /**
   * @param {string} type - Growth type
   * @param {Object} data - Event data
   */
  constructor(type, data = {}) {
    this.timestamp = Date.now();
    this.type = type;
    this.magnitude = data.magnitude || 1;
    this.domain = data.domain || 'general';
    this.description = data.description || '';
    this.metadata = data.metadata || {};

    // Derived: is this a regression (negative growth)?
    this.isRegression = this.magnitude < 0;
  }

  toJSON() {
    return {
      timestamp: this.timestamp,
      type: this.type,
      magnitude: this.magnitude,
      domain: this.domain,
      description: this.description,
      isRegression: this.isRegression,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GROWTH TRACKER
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Tracks growth over time
 */
export class GrowthTracker {
  constructor(options = {}) {
    /** @type {GrowthEvent[]} */
    this.events = [];
    this.maxEvents = options.maxEvents || GROWTH_CONFIG.MAX_EVENTS;

    // Domain-specific tracking
    this.domainGrowth = new Map();

    // Momentum (smoothed growth rate)
    this.momentum = 0;
    this.lastMomentumUpdate = Date.now();

    // Session
    this.sessionStart = Date.now();

    // Totals
    this.totalGrowth = 0;
    this.totalRegressions = 0;
  }

  /**
   * Record a growth event
   * @param {string} type - Growth type
   * @param {Object} [data] - Event data
   * @returns {GrowthEvent}
   */
  record(type, data = {}) {
    const event = new GrowthEvent(type, data);

    this.events.push(event);
    this.totalGrowth += Math.max(0, event.magnitude);
    this.totalRegressions += Math.abs(Math.min(0, event.magnitude));

    // Update domain tracking
    const domainTotal = this.domainGrowth.get(event.domain) || 0;
    this.domainGrowth.set(event.domain, domainTotal + event.magnitude);

    // Update momentum
    this._updateMomentum(event.magnitude);

    // Prune
    while (this.events.length > this.maxEvents) {
      this.events.shift();
    }

    return event;
  }

  /**
   * Update growth momentum (exponentially smoothed)
   * @private
   */
  _updateMomentum(magnitude) {
    const alpha = GROWTH_CONFIG.MOMENTUM_ALPHA;
    this.momentum = alpha * magnitude + (1 - alpha) * this.momentum;
    this.lastMomentumUpdate = Date.now();
  }

  // Convenience methods
  recordPattern(data = {}) {
    return this.record(GROWTH_CONFIG.TYPES.PATTERN, data);
  }

  recordKnowledge(data = {}) {
    return this.record(GROWTH_CONFIG.TYPES.KNOWLEDGE, data);
  }

  recordSkill(data = {}) {
    return this.record(GROWTH_CONFIG.TYPES.SKILL, data);
  }

  recordAdaptation(data = {}) {
    return this.record(GROWTH_CONFIG.TYPES.ADAPTATION, data);
  }

  recordInsight(data = {}) {
    return this.record(GROWTH_CONFIG.TYPES.INSIGHT, { ...data, magnitude: 2 });
  }

  /**
   * Record regression (negative growth)
   * @param {string} type
   * @param {Object} data
   */
  recordRegression(type, data = {}) {
    return this.record(type, { ...data, magnitude: -(data.magnitude || 1) });
  }

  /**
   * Get events within time window
   * @param {number} [windowMs]
   * @returns {GrowthEvent[]}
   */
  getRecentEvents(windowMs = GROWTH_CONFIG.RATE_WINDOW) {
    const cutoff = Date.now() - windowMs;
    return this.events.filter(e => e.timestamp >= cutoff);
  }

  /**
   * Calculate growth rate (events per hour)
   * @param {number} [windowMs]
   * @returns {number}
   */
  getGrowthRate(windowMs = GROWTH_CONFIG.RATE_WINDOW) {
    const recent = this.getRecentEvents(windowMs);
    if (recent.length === 0) return 0;

    // Use Poisson rate estimation
    const timestamps = recent.map(e => e.timestamp);
    const timeSpanMs = Math.max(1, timestamps[timestamps.length - 1] - timestamps[0]);
    const timeSpanHours = timeSpanMs / 3600000;

    // Sum magnitudes
    const totalMagnitude = recent.reduce((sum, e) => sum + e.magnitude, 0);

    return totalMagnitude / Math.max(0.1, timeSpanHours);
  }

  /**
   * Get net growth (growth - regressions)
   * @param {number} [windowMs]
   * @returns {number}
   */
  getNetGrowth(windowMs = GROWTH_CONFIG.RATE_WINDOW) {
    const recent = this.getRecentEvents(windowMs);
    return recent.reduce((sum, e) => sum + e.magnitude, 0);
  }

  /**
   * Get growth by domain
   * @returns {Object}
   */
  getGrowthByDomain() {
    const result = {};
    for (const [domain, total] of this.domainGrowth) {
      result[domain] = total;
    }
    return result;
  }

  /**
   * Get growth by type
   * @param {number} [windowMs]
   * @returns {Object}
   */
  getGrowthByType(windowMs = GROWTH_CONFIG.RATE_WINDOW) {
    const recent = this.getRecentEvents(windowMs);
    const byType = {};

    for (const event of recent) {
      byType[event.type] = (byType[event.type] || 0) + event.magnitude;
    }

    return byType;
  }

  /**
   * Get growth status
   * @returns {string}
   */
  getStatus() {
    const rate = this.getGrowthRate();

    if (rate >= GROWTH_CONFIG.RATE_RAPID) return 'rapid';
    if (rate >= GROWTH_CONFIG.RATE_HEALTHY) return 'healthy';
    if (rate >= GROWTH_CONFIG.RATE_SLOW) return 'slow';
    if (rate > 0) return 'minimal';
    if (rate === 0) return 'stagnant';
    return 'regressing';
  }

  /**
   * Get growth health score
   * @returns {Object}
   */
  getHealth() {
    const rate = this.getGrowthRate();
    const netGrowth = this.getNetGrowth();
    const status = this.getStatus();

    // Score based on rate relative to healthy threshold
    const rawScore = Math.min(1, rate / GROWTH_CONFIG.RATE_RAPID);
    const score = Math.min(PHI_INV, rawScore);

    // Penalize regressions
    const regressionRatio = this.totalGrowth > 0
      ? this.totalRegressions / this.totalGrowth
      : 0;

    return {
      score: Math.max(0, score * (1 - regressionRatio)),
      status,
      details: {
        rate,
        netGrowth,
        momentum: this.momentum,
        totalGrowth: this.totalGrowth,
        totalRegressions: this.totalRegressions,
        byType: this.getGrowthByType(),
        byDomain: this.getGrowthByDomain(),
      },
    };
  }

  /**
   * Check if experiencing growth spurt
   * @returns {boolean}
   */
  isGrowthSpurt() {
    return this.getGrowthRate() >= GROWTH_CONFIG.RATE_RAPID;
  }

  /**
   * Check if stagnating
   * @returns {boolean}
   */
  isStagnant() {
    const rate = this.getGrowthRate();
    return rate <= 0;
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    return {
      rate: this.getGrowthRate(),
      netGrowth: this.getNetGrowth(),
      momentum: this.momentum,
      status: this.getStatus(),
      totalGrowth: this.totalGrowth,
      totalRegressions: this.totalRegressions,
      eventCount: this.events.length,
      byType: this.getGrowthByType(),
      byDomain: this.getGrowthByDomain(),
      session: {
        durationMs: Date.now() - this.sessionStart,
      },
    };
  }

  /**
   * Reset tracker
   */
  reset() {
    this.events = [];
    this.domainGrowth.clear();
    this.momentum = 0;
    this.totalGrowth = 0;
    this.totalRegressions = 0;
    this.sessionStart = Date.now();
  }

  /**
   * Serialize
   */
  toJSON() {
    return {
      events: this.events.map(e => e.toJSON()),
      domainGrowth: Object.fromEntries(this.domainGrowth),
      momentum: this.momentum,
      totalGrowth: this.totalGrowth,
      totalRegressions: this.totalRegressions,
      sessionStart: this.sessionStart,
    };
  }

  /**
   * Restore
   */
  static fromJSON(data) {
    const tracker = new GrowthTracker();
    tracker.events = (data.events || []).map(e => {
      const event = new GrowthEvent(e.type, e);
      event.timestamp = e.timestamp;
      return event;
    });
    tracker.domainGrowth = new Map(Object.entries(data.domainGrowth || {}));
    tracker.momentum = data.momentum || 0;
    tracker.totalGrowth = data.totalGrowth || 0;
    tracker.totalRegressions = data.totalRegressions || 0;
    tracker.sessionStart = data.sessionStart || Date.now();
    return tracker;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY & SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

export function createGrowthTracker(options = {}) {
  return new GrowthTracker(options);
}

let _growthTracker = null;

export function getGrowthTracker() {
  if (!_growthTracker) {
    _growthTracker = new GrowthTracker();
  }
  return _growthTracker;
}

export function resetGrowthTracker() {
  _growthTracker = null;
}

export default {
  GrowthEvent,
  GrowthTracker,
  createGrowthTracker,
  getGrowthTracker,
  resetGrowthTracker,
  GROWTH_CONFIG,
};
