/**
 * E-Score - Node Reputation Calculator
 *
 * "Reputation is earned through sacrifice" - κυνικός
 *
 * E-Score (Ecosystem Score) measures node trustworthiness:
 * - Burns: How much $asdfasdfa has been sacrificed
 * - Uptime: How reliably the node has been online
 * - Quality: Quality of judgments (consensus agreement)
 *
 * Formula (φ-weighted):
 *   E-Score = (Burns^φ⁻¹ × Uptime^φ⁻² × Quality^φ⁻³) × 100
 *
 * All components normalized to [0, 1], output is [0, 100]
 *
 * @module @cynic/identity/escore
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

/**
 * E-Score component weights (φ-aligned)
 *
 * Burns is most important (φ⁻¹ = 0.618) - commitment via sacrifice
 * Uptime is second (φ⁻² = 0.382) - reliability
 * Quality is third (φ⁻³ = 0.236) - accuracy
 */
export const WEIGHTS = {
  BURNS: PHI_INV,      // 0.618 - Most important
  UPTIME: PHI_INV_2,   // 0.382 - Second
  QUALITY: PHI_INV_3,  // 0.236 - Third
};

/**
 * E-Score thresholds for status levels
 */
export const THRESHOLDS = {
  TRUSTED: 61.8,       // φ⁻¹ × 100 - Fully trusted
  VERIFIED: 38.2,      // φ⁻² × 100 - Verified participant
  NEWCOMER: 23.6,      // φ⁻³ × 100 - New but participating
  UNKNOWN: 0,          // No reputation yet
};

/**
 * E-Score status from score
 * @param {number} score - E-Score (0-100)
 * @returns {string} Status
 */
export function getStatus(score) {
  if (score >= THRESHOLDS.TRUSTED) return 'TRUSTED';
  if (score >= THRESHOLDS.VERIFIED) return 'VERIFIED';
  if (score >= THRESHOLDS.NEWCOMER) return 'NEWCOMER';
  return 'UNKNOWN';
}

/**
 * Normalize burn amount to [0, 1]
 *
 * Uses logarithmic scaling so diminishing returns apply:
 * - 0 burns = 0
 * - 1M burns = ~0.5
 * - 1B burns = ~0.75
 * - ∞ burns → 1 (asymptotic)
 *
 * @param {number} burns - Total burn amount (in smallest unit)
 * @param {number} [scale=1e9] - Scale factor (1B by default)
 * @returns {number} Normalized burns [0, 1]
 */
export function normalizeBurns(burns, scale = 1e9) {
  if (burns <= 0) return 0;
  // Log scaling with φ as base for aesthetic harmony
  const normalized = Math.log(1 + burns / scale) / Math.log(PHI + burns / scale);
  return Math.min(1, Math.max(0, normalized));
}

/**
 * Normalize uptime to [0, 1]
 *
 * Uptime is measured as ratio of actual uptime to expected uptime.
 * Values above 1.0 are capped at 1.0.
 *
 * @param {number} uptimeSeconds - Actual uptime in seconds
 * @param {number} expectedSeconds - Expected uptime in seconds
 * @returns {number} Normalized uptime [0, 1]
 */
export function normalizeUptime(uptimeSeconds, expectedSeconds) {
  if (expectedSeconds <= 0) return 0;
  const ratio = uptimeSeconds / expectedSeconds;
  return Math.min(1, Math.max(0, ratio));
}

/**
 * Normalize quality to [0, 1]
 *
 * Quality is based on how often node's judgments match consensus.
 * Requires minimum number of judgments to be meaningful.
 *
 * @param {number} agreementCount - Number of judgments matching consensus
 * @param {number} totalJudgments - Total judgments made
 * @param {number} [minJudgments=10] - Minimum for meaningful quality
 * @returns {number} Normalized quality [0, 1]
 */
export function normalizeQuality(agreementCount, totalJudgments, minJudgments = 10) {
  if (totalJudgments < minJudgments) {
    // Not enough data - return neutral quality scaled by participation
    return 0.5 * (totalJudgments / minJudgments);
  }
  return Math.min(1, Math.max(0, agreementCount / totalJudgments));
}

/**
 * Calculate E-Score
 *
 * @param {Object} params - Score parameters
 * @param {number} params.burns - Total burns (raw amount)
 * @param {number} params.uptimeSeconds - Actual uptime
 * @param {number} params.expectedUptimeSeconds - Expected uptime
 * @param {number} params.agreementCount - Judgments matching consensus
 * @param {number} params.totalJudgments - Total judgments made
 * @param {Object} [options] - Calculation options
 * @param {number} [options.burnScale] - Burn normalization scale
 * @param {number} [options.minJudgments] - Minimum judgments for quality
 * @returns {Object} E-Score result
 */
export function calculateEScore(params, options = {}) {
  const {
    burns = 0,
    uptimeSeconds = 0,
    expectedUptimeSeconds = 0,
    agreementCount = 0,
    totalJudgments = 0,
  } = params;

  const {
    burnScale = 1e9,
    minJudgments = 10,
  } = options;

  // Normalize components
  const normalizedBurns = normalizeBurns(burns, burnScale);
  const normalizedUptime = normalizeUptime(uptimeSeconds, expectedUptimeSeconds);
  const normalizedQuality = normalizeQuality(agreementCount, totalJudgments, minJudgments);

  // Calculate weighted geometric mean
  // E = Burns^w₁ × Uptime^w₂ × Quality^w₃
  const burnComponent = Math.pow(Math.max(0.001, normalizedBurns), WEIGHTS.BURNS);
  const uptimeComponent = Math.pow(Math.max(0.001, normalizedUptime), WEIGHTS.UPTIME);
  const qualityComponent = Math.pow(Math.max(0.001, normalizedQuality), WEIGHTS.QUALITY);

  const rawScore = burnComponent * uptimeComponent * qualityComponent;

  // Scale to 0-100
  const score = Math.round(rawScore * 100 * 10) / 10; // One decimal place

  return {
    score: Math.min(100, Math.max(0, score)),
    status: getStatus(score),
    components: {
      burns: {
        raw: burns,
        normalized: normalizedBurns,
        weight: WEIGHTS.BURNS,
        contribution: burnComponent,
      },
      uptime: {
        raw: uptimeSeconds,
        expected: expectedUptimeSeconds,
        normalized: normalizedUptime,
        weight: WEIGHTS.UPTIME,
        contribution: uptimeComponent,
      },
      quality: {
        agreements: agreementCount,
        total: totalJudgments,
        normalized: normalizedQuality,
        weight: WEIGHTS.QUALITY,
        contribution: qualityComponent,
      },
    },
    formula: 'Burns^φ⁻¹ × Uptime^φ⁻² × Quality^φ⁻³',
    timestamp: Date.now(),
  };
}

/**
 * E-Score Calculator class
 *
 * Tracks and calculates E-Score over time for a node.
 */
export class EScoreCalculator {
  /**
   * @param {Object} options - Configuration
   * @param {number} [options.burnScale] - Burn normalization scale
   * @param {number} [options.minJudgments] - Minimum judgments for quality
   */
  constructor(options = {}) {
    this.burnScale = options.burnScale || 1e9;
    this.minJudgments = options.minJudgments || 10;

    // Tracking state
    this.totalBurns = 0;
    this.burnHistory = []; // { amount, timestamp, txSignature }

    this.startTime = Date.now();
    this.totalUptimeMs = 0;
    this.lastHeartbeat = Date.now();
    this.isOnline = true;

    this.agreementCount = 0;
    this.totalJudgments = 0;
    this.judgmentHistory = []; // { judgmentId, matchedConsensus, timestamp }

    // Cached score
    this._cachedScore = null;
    this._cacheTime = 0;
    this._cacheTtl = 60000; // 1 minute cache
  }

  /**
   * Record a burn
   * @param {number} amount - Burn amount
   * @param {string} [txSignature] - Transaction signature
   */
  recordBurn(amount, txSignature = null) {
    this.totalBurns += amount;
    this.burnHistory.push({
      amount,
      txSignature,
      timestamp: Date.now(),
    });
    this._invalidateCache();
  }

  /**
   * Record heartbeat (node is alive)
   */
  heartbeat() {
    const now = Date.now();
    if (this.isOnline) {
      this.totalUptimeMs += now - this.lastHeartbeat;
    }
    this.lastHeartbeat = now;
    this.isOnline = true;
  }

  /**
   * Mark node as offline
   */
  markOffline() {
    if (this.isOnline) {
      this.totalUptimeMs += Date.now() - this.lastHeartbeat;
    }
    this.isOnline = false;
    this._invalidateCache();
  }

  /**
   * Record a judgment
   * @param {string} judgmentId - Judgment ID
   * @param {boolean} matchedConsensus - Whether it matched consensus
   */
  recordJudgment(judgmentId, matchedConsensus) {
    this.totalJudgments++;
    if (matchedConsensus) {
      this.agreementCount++;
    }
    this.judgmentHistory.push({
      judgmentId,
      matchedConsensus,
      timestamp: Date.now(),
    });
    this._invalidateCache();
  }

  /**
   * Get expected uptime (time since start)
   * @returns {number} Expected uptime in ms
   */
  getExpectedUptimeMs() {
    return Date.now() - this.startTime;
  }

  /**
   * Get actual uptime
   * @returns {number} Actual uptime in ms
   */
  getActualUptimeMs() {
    let uptime = this.totalUptimeMs;
    if (this.isOnline) {
      uptime += Date.now() - this.lastHeartbeat;
    }
    return uptime;
  }

  /**
   * Calculate current E-Score
   * @param {boolean} [skipCache=false] - Skip cache
   * @returns {Object} E-Score result
   */
  calculate(skipCache = false) {
    // Check cache
    if (!skipCache && this._cachedScore && Date.now() - this._cacheTime < this._cacheTtl) {
      return this._cachedScore;
    }

    const expectedUptime = this.getExpectedUptimeMs();
    const actualUptime = this.getActualUptimeMs();

    const result = calculateEScore({
      burns: this.totalBurns,
      uptimeSeconds: actualUptime / 1000,
      expectedUptimeSeconds: expectedUptime / 1000,
      agreementCount: this.agreementCount,
      totalJudgments: this.totalJudgments,
    }, {
      burnScale: this.burnScale,
      minJudgments: this.minJudgments,
    });

    // Cache result
    this._cachedScore = result;
    this._cacheTime = Date.now();

    return result;
  }

  /**
   * Invalidate cached score
   * @private
   */
  _invalidateCache() {
    this._cachedScore = null;
  }

  /**
   * Export state for persistence
   * @returns {Object}
   */
  export() {
    return {
      totalBurns: this.totalBurns,
      burnHistory: this.burnHistory,
      startTime: this.startTime,
      totalUptimeMs: this.totalUptimeMs,
      lastHeartbeat: this.lastHeartbeat,
      isOnline: this.isOnline,
      agreementCount: this.agreementCount,
      totalJudgments: this.totalJudgments,
      judgmentHistory: this.judgmentHistory,
    };
  }

  /**
   * Import state from persistence
   * @param {Object} state - Exported state
   */
  import(state) {
    this.totalBurns = state.totalBurns || 0;
    this.burnHistory = state.burnHistory || [];
    this.startTime = state.startTime || Date.now();
    this.totalUptimeMs = state.totalUptimeMs || 0;
    this.lastHeartbeat = state.lastHeartbeat || Date.now();
    this.isOnline = state.isOnline ?? true;
    this.agreementCount = state.agreementCount || 0;
    this.totalJudgments = state.totalJudgments || 0;
    this.judgmentHistory = state.judgmentHistory || [];
    this._invalidateCache();
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    const expectedUptime = this.getExpectedUptimeMs();
    const actualUptime = this.getActualUptimeMs();

    return {
      burns: {
        total: this.totalBurns,
        count: this.burnHistory.length,
      },
      uptime: {
        expected: expectedUptime,
        actual: actualUptime,
        ratio: expectedUptime > 0 ? actualUptime / expectedUptime : 0,
        isOnline: this.isOnline,
      },
      judgments: {
        total: this.totalJudgments,
        agreements: this.agreementCount,
        accuracy: this.totalJudgments > 0 ? this.agreementCount / this.totalJudgments : 0,
      },
      currentScore: this.calculate(),
    };
  }
}

/**
 * Create an E-Score calculator
 * @param {Object} [options] - Configuration
 * @returns {EScoreCalculator}
 */
export function createEScoreCalculator(options = {}) {
  return new EScoreCalculator(options);
}

export default {
  // Constants
  WEIGHTS,
  THRESHOLDS,

  // Functions
  calculateEScore,
  normalizeBurns,
  normalizeUptime,
  normalizeQuality,
  getStatus,

  // Class
  EScoreCalculator,
  createEScoreCalculator,
};
