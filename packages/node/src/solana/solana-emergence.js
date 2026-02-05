/**
 * CYNIC Solana Emergence - C2.7 (SOLANA × EMERGE)
 *
 * Detects emergent patterns in Solana activity.
 * Part of the 7×7 Fractal Matrix emergence layer.
 *
 * "La chaîne révèle ses mystères" - κυνικός
 *
 * Emerges:
 * - Activity pattern shifts
 * - Anomalous transaction clusters
 * - Network behavior changes
 * - Program popularity trends
 *
 * @module @cynic/node/solana/solana-emergence
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('SolanaEmergence');

/**
 * Emergent pattern types
 */
export const SolanaPatternType = {
  ACTIVITY_SPIKE: 'activity_spike',
  ACTIVITY_DROP: 'activity_drop',
  FEE_ANOMALY: 'fee_anomaly',
  WHALE_MOVEMENT: 'whale_movement',
  PROGRAM_SURGE: 'program_surge',
  ERROR_CLUSTER: 'error_cluster',
  TIMING_SHIFT: 'timing_shift',
  VOLUME_TREND: 'volume_trend',
};

/**
 * Pattern significance levels
 */
export const SignificanceLevel = {
  LOW: 'low',           // Interesting but not actionable
  MEDIUM: 'medium',     // Worth noting
  HIGH: 'high',         // Should trigger attention
  CRITICAL: 'critical', // Immediate action needed
};

/**
 * φ-aligned detection thresholds
 */
const EMERGENCE_THRESHOLDS = {
  // Activity spike/drop threshold (multiplier from baseline)
  activitySpike: PHI_INV + 1,     // 1.618x above baseline
  activityDrop: PHI_INV_2,        // 0.382x below baseline

  // Fee anomaly threshold (std deviations)
  feeAnomalyStdDev: 2.0,

  // Whale threshold (in SOL)
  whaleThreshold: 1000,           // 1000 SOL

  // Minimum pattern occurrences
  minPatternOccurrences: 3,

  // Window size for analysis (ms)
  analysisWindow: 24 * 60 * 60 * 1000, // 24 hours
};

/**
 * SolanaEmergence - Detects emergent patterns
 */
export class SolanaEmergence extends EventEmitter {
  /**
   * Create a new SolanaEmergence
   *
   * @param {Object} [options] - Configuration
   */
  constructor(options = {}) {
    super();

    // Data stores for analysis
    this._activityHistory = [];
    this._feeHistory = [];
    this._volumeHistory = [];
    this._errorHistory = [];

    // Detected patterns
    this._patterns = [];
    this._maxPatterns = 100;

    // Baseline metrics (rolling averages)
    this._baselines = {
      activityRate: null,      // Transactions per hour
      avgFee: null,            // Average fee
      avgVolume: null,         // Average volume per tx
      errorRate: null,         // Errors per hour
    };

    // Stats
    this._stats = {
      patternsDetected: 0,
      byType: {},
      lastAnalysis: null,
    };

    // Initialize pattern counters
    for (const type of Object.values(SolanaPatternType)) {
      this._stats.byType[type] = 0;
    }
  }

  /**
   * Record a data point for emergence analysis
   *
   * @param {Object} data - Activity data
   */
  recordActivity(data) {
    const timestamp = Date.now();

    // Store activity
    this._activityHistory.push({
      timestamp,
      count: data.transactionCount || 1,
      success: data.success !== false,
    });

    // Store fee data
    if (data.fee !== undefined) {
      this._feeHistory.push({
        timestamp,
        fee: data.fee,
      });
    }

    // Store volume data
    if (data.volume !== undefined) {
      this._volumeHistory.push({
        timestamp,
        volume: data.volume,
      });
    }

    // Store error data
    if (data.error) {
      this._errorHistory.push({
        timestamp,
        error: data.error,
        errorCode: data.errorCode,
      });
    }

    // Cleanup old data
    this._cleanupHistory(timestamp);

    // Check for immediate patterns
    this._checkImmediatePatterns(data, timestamp);
  }

  /**
   * Run full emergence analysis
   *
   * @returns {Object} Analysis results with detected patterns
   */
  analyze() {
    const timestamp = Date.now();
    this._stats.lastAnalysis = timestamp;

    const patterns = [];

    // Update baselines
    this._updateBaselines();

    // Detect various pattern types
    const activityPattern = this._detectActivityPattern();
    if (activityPattern) patterns.push(activityPattern);

    const feePattern = this._detectFeeAnomaly();
    if (feePattern) patterns.push(feePattern);

    const volumePattern = this._detectVolumeTrend();
    if (volumePattern) patterns.push(volumePattern);

    const errorPattern = this._detectErrorCluster();
    if (errorPattern) patterns.push(errorPattern);

    // Store detected patterns
    for (const pattern of patterns) {
      this._recordPattern(pattern);
    }

    const result = {
      cell: 'C2.7',
      dimension: 'SOLANA',
      analysis: 'EMERGE',
      patterns,
      baselines: this._baselines,
      timestamp,
    };

    // Emit events
    if (patterns.length > 0) {
      this.emit('patterns_detected', result);
      globalEventBus.emit('solana:emergence', result);
    }

    return result;
  }

  /**
   * Update baseline metrics
   * @private
   */
  _updateBaselines() {
    const now = Date.now();
    const windowStart = now - EMERGENCE_THRESHOLDS.analysisWindow;

    // Activity rate (transactions per hour)
    const recentActivity = this._activityHistory.filter(a => a.timestamp >= windowStart);
    if (recentActivity.length > 0) {
      const hours = EMERGENCE_THRESHOLDS.analysisWindow / (60 * 60 * 1000);
      this._baselines.activityRate = recentActivity.length / hours;
    }

    // Average fee
    const recentFees = this._feeHistory.filter(f => f.timestamp >= windowStart);
    if (recentFees.length > 0) {
      this._baselines.avgFee = recentFees.reduce((sum, f) => sum + f.fee, 0) / recentFees.length;
    }

    // Average volume
    const recentVolume = this._volumeHistory.filter(v => v.timestamp >= windowStart);
    if (recentVolume.length > 0) {
      this._baselines.avgVolume = recentVolume.reduce((sum, v) => sum + v.volume, 0) / recentVolume.length;
    }

    // Error rate
    const recentErrors = this._errorHistory.filter(e => e.timestamp >= windowStart);
    const hours = EMERGENCE_THRESHOLDS.analysisWindow / (60 * 60 * 1000);
    this._baselines.errorRate = recentErrors.length / hours;
  }

  /**
   * Detect activity pattern changes
   * @private
   */
  _detectActivityPattern() {
    if (!this._baselines.activityRate || this._activityHistory.length < 10) {
      return null;
    }

    // Compare last hour to baseline
    const now = Date.now();
    const lastHour = this._activityHistory.filter(a => a.timestamp >= now - 60 * 60 * 1000);
    const currentRate = lastHour.length;

    const ratio = currentRate / this._baselines.activityRate;

    if (ratio >= EMERGENCE_THRESHOLDS.activitySpike) {
      return {
        type: SolanaPatternType.ACTIVITY_SPIKE,
        significance: ratio > 2.618 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        ratio,
        currentRate,
        baseline: this._baselines.activityRate,
        description: `Activity ${(ratio * 100 - 100).toFixed(0)}% above baseline`,
      };
    }

    if (ratio <= EMERGENCE_THRESHOLDS.activityDrop) {
      return {
        type: SolanaPatternType.ACTIVITY_DROP,
        significance: ratio < 0.1 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        ratio,
        currentRate,
        baseline: this._baselines.activityRate,
        description: `Activity ${(100 - ratio * 100).toFixed(0)}% below baseline`,
      };
    }

    return null;
  }

  /**
   * Detect fee anomalies
   * @private
   */
  _detectFeeAnomaly() {
    if (!this._baselines.avgFee || this._feeHistory.length < 10) {
      return null;
    }

    // Calculate std dev
    const fees = this._feeHistory.slice(-50).map(f => f.fee);
    const avg = fees.reduce((a, b) => a + b, 0) / fees.length;
    const stdDev = Math.sqrt(fees.map(f => Math.pow(f - avg, 2)).reduce((a, b) => a + b, 0) / fees.length);

    // Check last few transactions
    const recentFees = this._feeHistory.slice(-5);
    for (const f of recentFees) {
      const zScore = (f.fee - avg) / stdDev;
      if (Math.abs(zScore) > EMERGENCE_THRESHOLDS.feeAnomalyStdDev) {
        return {
          type: SolanaPatternType.FEE_ANOMALY,
          significance: zScore > 3 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
          fee: f.fee,
          avgFee: avg,
          zScore,
          description: `Fee ${zScore > 0 ? 'spike' : 'drop'}: ${f.fee} lamports (${zScore.toFixed(1)} std devs)`,
        };
      }
    }

    return null;
  }

  /**
   * Detect volume trends
   * @private
   */
  _detectVolumeTrend() {
    if (this._volumeHistory.length < 20) {
      return null;
    }

    // Compare recent 10 to previous 10
    const recent = this._volumeHistory.slice(-10);
    const previous = this._volumeHistory.slice(-20, -10);

    const recentAvg = recent.reduce((sum, v) => sum + v.volume, 0) / recent.length;
    const prevAvg = previous.reduce((sum, v) => sum + v.volume, 0) / previous.length;

    const change = (recentAvg - prevAvg) / prevAvg;

    if (Math.abs(change) > 0.5) {
      const direction = change > 0 ? 'increasing' : 'decreasing';
      return {
        type: SolanaPatternType.VOLUME_TREND,
        significance: Math.abs(change) > 1 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        change,
        recentAvg,
        previousAvg: prevAvg,
        description: `Volume ${direction} by ${(Math.abs(change) * 100).toFixed(0)}%`,
      };
    }

    // Check for whale movement
    const maxRecent = Math.max(...recent.map(v => v.volume));
    if (maxRecent >= EMERGENCE_THRESHOLDS.whaleThreshold * 1e9) {
      return {
        type: SolanaPatternType.WHALE_MOVEMENT,
        significance: SignificanceLevel.HIGH,
        volume: maxRecent,
        volumeSOL: maxRecent / 1e9,
        description: `Whale movement detected: ${(maxRecent / 1e9).toFixed(2)} SOL`,
      };
    }

    return null;
  }

  /**
   * Detect error clusters
   * @private
   */
  _detectErrorCluster() {
    if (this._errorHistory.length < 5) {
      return null;
    }

    // Check for recent error cluster
    const now = Date.now();
    const recentErrors = this._errorHistory.filter(e => e.timestamp >= now - 5 * 60 * 1000); // Last 5 minutes

    if (recentErrors.length >= EMERGENCE_THRESHOLDS.minPatternOccurrences) {
      // Group by error type
      const errorCounts = new Map();
      for (const e of recentErrors) {
        const key = e.errorCode || 'unknown';
        errorCounts.set(key, (errorCounts.get(key) || 0) + 1);
      }

      const mostCommon = Array.from(errorCounts.entries())
        .sort((a, b) => b[1] - a[1])[0];

      return {
        type: SolanaPatternType.ERROR_CLUSTER,
        significance: recentErrors.length > 10 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        errorCount: recentErrors.length,
        dominantError: mostCommon[0],
        dominantCount: mostCommon[1],
        description: `Error cluster: ${recentErrors.length} errors in 5 min (${mostCommon[0]}: ${mostCommon[1]}x)`,
      };
    }

    return null;
  }

  /**
   * Check for immediate patterns
   * @private
   */
  _checkImmediatePatterns(data, timestamp) {
    // Check for whale movement immediately
    if (data.volume && data.volume >= EMERGENCE_THRESHOLDS.whaleThreshold * 1e9) {
      const pattern = {
        type: SolanaPatternType.WHALE_MOVEMENT,
        significance: SignificanceLevel.HIGH,
        volume: data.volume,
        volumeSOL: data.volume / 1e9,
        timestamp,
        description: `Whale movement: ${(data.volume / 1e9).toFixed(2)} SOL`,
      };

      this._recordPattern(pattern);
      this.emit('pattern_detected', pattern);
      globalEventBus.emit('solana:emergence:immediate', pattern);
    }
  }

  /**
   * Record a detected pattern
   * @private
   */
  _recordPattern(pattern) {
    pattern.timestamp = pattern.timestamp || Date.now();

    this._patterns.push(pattern);
    while (this._patterns.length > this._maxPatterns) {
      this._patterns.shift();
    }

    this._stats.patternsDetected++;
    this._stats.byType[pattern.type] = (this._stats.byType[pattern.type] || 0) + 1;

    log.info('Pattern detected', { type: pattern.type, significance: pattern.significance });
  }

  /**
   * Cleanup old history data
   * @private
   */
  _cleanupHistory(now) {
    const cutoff = now - EMERGENCE_THRESHOLDS.analysisWindow * 2;

    this._activityHistory = this._activityHistory.filter(a => a.timestamp >= cutoff);
    this._feeHistory = this._feeHistory.filter(f => f.timestamp >= cutoff);
    this._volumeHistory = this._volumeHistory.filter(v => v.timestamp >= cutoff);
    this._errorHistory = this._errorHistory.filter(e => e.timestamp >= cutoff);
  }

  /**
   * Get detected patterns
   *
   * @param {number} [limit] - Max patterns to return
   * @returns {Array} Recent patterns
   */
  getPatterns(limit = 20) {
    return this._patterns.slice(-limit);
  }

  /**
   * Get statistics
   *
   * @returns {Object}
   */
  getStats() {
    return {
      ...this._stats,
      activityHistorySize: this._activityHistory.length,
      patternsStored: this._patterns.length,
      baselines: this._baselines,
    };
  }

  /**
   * Get health assessment
   *
   * @returns {Object}
   */
  getHealth() {
    const hasEnoughData = this._activityHistory.length >= 20;
    const recentCritical = this._patterns
      .slice(-10)
      .filter(p => p.significance === SignificanceLevel.CRITICAL).length;

    let status = 'healthy';
    let score = PHI_INV;

    if (!hasEnoughData) {
      status = 'bootstrapping';
      score = PHI_INV_2;
    } else if (recentCritical > 0) {
      status = 'alert';
      score = PHI_INV_3;
    }

    return {
      status,
      score,
      patternsDetected: this._stats.patternsDetected,
      recentCriticalPatterns: recentCritical,
      hasEnoughData,
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this._activityHistory = [];
    this._feeHistory = [];
    this._volumeHistory = [];
    this._errorHistory = [];
    this._patterns = [];
    this._baselines = {
      activityRate: null,
      avgFee: null,
      avgVolume: null,
      errorRate: null,
    };
    this._stats = {
      patternsDetected: 0,
      byType: {},
      lastAnalysis: null,
    };
    for (const type of Object.values(SolanaPatternType)) {
      this._stats.byType[type] = 0;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create SolanaEmergence singleton
 *
 * @param {Object} [options] - Options
 * @returns {SolanaEmergence}
 */
export function getSolanaEmergence(options = {}) {
  if (!_instance) {
    _instance = new SolanaEmergence(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetSolanaEmergence() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  SolanaEmergence,
  SolanaPatternType,
  SignificanceLevel,
  getSolanaEmergence,
  resetSolanaEmergence,
};
