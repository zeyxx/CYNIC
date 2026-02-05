/**
 * CYNIC Solana Learner - C2.5 (SOLANA × LEARN)
 *
 * Learns from Solana transaction outcomes.
 * Part of the 7×7 Fractal Matrix learning layer.
 *
 * "Le chien apprend de la chaîne" - κυνικός
 *
 * Learns:
 * - Optimal fee estimation
 * - Best transaction timing
 * - RPC endpoint reliability
 * - Program interaction patterns
 *
 * @module @cynic/node/solana/solana-learner
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('SolanaLearner');

/**
 * Learning categories for Solana
 */
export const SolanaLearningCategory = {
  FEE_ESTIMATION: 'fee_estimation',
  TIMING: 'timing',
  ENDPOINT_RELIABILITY: 'endpoint_reliability',
  PROGRAM_PATTERNS: 'program_patterns',
  ERROR_PATTERNS: 'error_patterns',
};

/**
 * Learning rate config (φ-aligned)
 */
const LEARNING_CONFIG = {
  learningRate: PHI_INV_2,        // 38.2% - gradual learning
  decayRate: 0.95,                 // Memory decay per day
  minObservations: 5,              // Before forming belief
  maxHistory: 500,                 // Max observations stored
};

/**
 * SolanaLearner - Learns from Solana outcomes
 */
export class SolanaLearner extends EventEmitter {
  /**
   * Create a new SolanaLearner
   *
   * @param {Object} [options] - Configuration
   */
  constructor(options = {}) {
    super();

    // Fee learning
    this._feeHistory = [];
    this._feeModel = {
      baseFee: 5000,
      congestionMultiplier: 1.0,
      successRate: 1.0,
    };

    // Timing learning
    this._timingHistory = [];
    this._timingModel = {
      bestHours: [],           // Hours with highest success
      worstHours: [],          // Hours to avoid
      avgSlotTime: 400,        // ms
    };

    // Endpoint learning
    this._endpointHistory = new Map();
    this._endpointModel = new Map();

    // Error pattern learning
    this._errorHistory = [];
    this._errorPatterns = new Map();

    // Stats
    this._stats = {
      observationsRecorded: 0,
      beliefsFormed: 0,
      predictionsCorrect: 0,
      predictionsMade: 0,
      lastObservation: null,
    };
  }

  /**
   * Record a transaction outcome for learning
   *
   * @param {Object} outcome - Transaction outcome
   * @returns {Object} Learning update
   */
  recordOutcome(outcome) {
    const timestamp = Date.now();
    this._stats.observationsRecorded++;
    this._stats.lastObservation = timestamp;

    const updates = [];

    // Learn from fee
    if (outcome.fee !== undefined) {
      updates.push(this._learnFee(outcome));
    }

    // Learn timing
    updates.push(this._learnTiming(outcome, timestamp));

    // Learn endpoint reliability
    if (outcome.endpoint) {
      updates.push(this._learnEndpoint(outcome));
    }

    // Learn from errors
    if (outcome.error) {
      updates.push(this._learnError(outcome));
    }

    // Emit update event
    const result = {
      cell: 'C2.5',
      dimension: 'SOLANA',
      analysis: 'LEARN',
      updates,
      timestamp,
    };

    this.emit('learning_update', result);
    globalEventBus.emit('solana:learning', result);

    return result;
  }

  /**
   * Predict optimal fee
   *
   * @param {Object} [context] - Current context
   * @returns {Object} Fee prediction
   */
  predictFee(context = {}) {
    this._stats.predictionsMade++;

    const { congestion = 0.5 } = context;

    // Base prediction from learned model
    const baseFee = this._feeModel.baseFee;
    const multiplier = 1 + (congestion * (this._feeModel.congestionMultiplier - 1));
    const predictedFee = Math.round(baseFee * multiplier);

    return {
      predictedFee,
      confidence: Math.min(PHI_INV, this._feeModel.successRate),
      baseFee,
      multiplier,
      source: this._feeHistory.length >= LEARNING_CONFIG.minObservations ? 'learned' : 'default',
    };
  }

  /**
   * Predict best timing
   *
   * @returns {Object} Timing prediction
   */
  predictTiming() {
    this._stats.predictionsMade++;

    const currentHour = new Date().getHours();
    const isBestTime = this._timingModel.bestHours.includes(currentHour);
    const isWorstTime = this._timingModel.worstHours.includes(currentHour);

    return {
      currentHour,
      recommendation: isWorstTime ? 'wait' : isBestTime ? 'proceed' : 'neutral',
      bestHours: this._timingModel.bestHours,
      worstHours: this._timingModel.worstHours,
      confidence: this._timingHistory.length >= LEARNING_CONFIG.minObservations ? PHI_INV : PHI_INV_2,
    };
  }

  /**
   * Predict endpoint reliability
   *
   * @param {string} url - Endpoint URL
   * @returns {Object} Reliability prediction
   */
  predictEndpoint(url) {
    this._stats.predictionsMade++;

    const model = this._endpointModel.get(url);

    if (!model) {
      return {
        reliability: 0.5,
        avgLatency: 1000,
        confidence: 0,
        source: 'unknown',
      };
    }

    return {
      reliability: model.successRate,
      avgLatency: model.avgLatency,
      confidence: Math.min(PHI_INV, model.observations / 10),
      source: 'learned',
    };
  }

  /**
   * Record prediction outcome (for calibration)
   *
   * @param {string} category - Learning category
   * @param {boolean} wasCorrect - Was prediction correct?
   */
  recordPredictionOutcome(category, wasCorrect) {
    if (wasCorrect) {
      this._stats.predictionsCorrect++;
    }

    this.emit('prediction_outcome', { category, wasCorrect });
  }

  /**
   * Learn from fee outcome
   * @private
   */
  _learnFee(outcome) {
    const { fee, success, congestion = 0.5 } = outcome;

    this._feeHistory.push({
      fee,
      success,
      congestion,
      timestamp: Date.now(),
    });

    // Trim history
    while (this._feeHistory.length > LEARNING_CONFIG.maxHistory) {
      this._feeHistory.shift();
    }

    // Update model if enough data
    if (this._feeHistory.length >= LEARNING_CONFIG.minObservations) {
      const successfulFees = this._feeHistory
        .filter(h => h.success)
        .map(h => h.fee);

      if (successfulFees.length > 0) {
        // Use median successful fee as base
        successfulFees.sort((a, b) => a - b);
        this._feeModel.baseFee = successfulFees[Math.floor(successfulFees.length / 2)];

        // Calculate success rate
        this._feeModel.successRate = successfulFees.length / this._feeHistory.length;

        this._stats.beliefsFormed++;
      }
    }

    return {
      category: SolanaLearningCategory.FEE_ESTIMATION,
      historySize: this._feeHistory.length,
      currentModel: this._feeModel,
    };
  }

  /**
   * Learn from timing
   * @private
   */
  _learnTiming(outcome, timestamp) {
    const hour = new Date(timestamp).getHours();
    const { success, latency } = outcome;

    this._timingHistory.push({
      hour,
      success,
      latency,
      timestamp,
    });

    // Trim history
    while (this._timingHistory.length > LEARNING_CONFIG.maxHistory) {
      this._timingHistory.shift();
    }

    // Update timing model
    if (this._timingHistory.length >= LEARNING_CONFIG.minObservations) {
      const hourStats = new Map();

      for (const h of this._timingHistory) {
        const stats = hourStats.get(h.hour) || { success: 0, total: 0 };
        stats.total++;
        if (h.success) stats.success++;
        hourStats.set(h.hour, stats);
      }

      // Find best and worst hours
      const hourRates = Array.from(hourStats.entries())
        .map(([hour, stats]) => ({ hour, rate: stats.success / stats.total }))
        .sort((a, b) => b.rate - a.rate);

      this._timingModel.bestHours = hourRates.slice(0, 3).map(h => h.hour);
      this._timingModel.worstHours = hourRates.slice(-3).map(h => h.hour);

      this._stats.beliefsFormed++;
    }

    return {
      category: SolanaLearningCategory.TIMING,
      historySize: this._timingHistory.length,
      currentModel: this._timingModel,
    };
  }

  /**
   * Learn from endpoint
   * @private
   */
  _learnEndpoint(outcome) {
    const { endpoint, success, latency } = outcome;

    // Get or create history
    let history = this._endpointHistory.get(endpoint) || [];
    history.push({ success, latency, timestamp: Date.now() });

    // Trim history
    while (history.length > 100) {
      history.shift();
    }
    this._endpointHistory.set(endpoint, history);

    // Update model
    if (history.length >= 3) {
      const successCount = history.filter(h => h.success).length;
      const avgLatency = history.reduce((sum, h) => sum + (h.latency || 1000), 0) / history.length;

      this._endpointModel.set(endpoint, {
        successRate: successCount / history.length,
        avgLatency,
        observations: history.length,
      });
    }

    return {
      category: SolanaLearningCategory.ENDPOINT_RELIABILITY,
      endpoint,
      historySize: history.length,
    };
  }

  /**
   * Learn from error
   * @private
   */
  _learnError(outcome) {
    const { error, errorCode } = outcome;

    this._errorHistory.push({
      error,
      errorCode,
      timestamp: Date.now(),
    });

    // Trim history
    while (this._errorHistory.length > LEARNING_CONFIG.maxHistory) {
      this._errorHistory.shift();
    }

    // Count error patterns
    const key = errorCode || error?.slice(0, 50);
    const count = (this._errorPatterns.get(key) || 0) + 1;
    this._errorPatterns.set(key, count);

    return {
      category: SolanaLearningCategory.ERROR_PATTERNS,
      errorKey: key,
      occurrences: count,
    };
  }

  /**
   * Get learning statistics
   *
   * @returns {Object}
   */
  getStats() {
    return {
      ...this._stats,
      feeHistorySize: this._feeHistory.length,
      timingHistorySize: this._timingHistory.length,
      endpointsTracked: this._endpointModel.size,
      errorPatternsDetected: this._errorPatterns.size,
      predictionAccuracy: this._stats.predictionsMade > 0
        ? this._stats.predictionsCorrect / this._stats.predictionsMade
        : null,
    };
  }

  /**
   * Get health assessment
   *
   * @returns {Object}
   */
  getHealth() {
    const hasEnoughData = this._feeHistory.length >= LEARNING_CONFIG.minObservations;

    return {
      status: hasEnoughData ? 'learning' : 'bootstrapping',
      score: hasEnoughData ? PHI_INV : PHI_INV_2,
      observationsTotal: this._stats.observationsRecorded,
      beliefsFormed: this._stats.beliefsFormed,
      predictionAccuracy: this._stats.predictionsMade > 0
        ? this._stats.predictionsCorrect / this._stats.predictionsMade
        : null,
    };
  }

  /**
   * Clear all learning
   */
  clear() {
    this._feeHistory = [];
    this._feeModel = { baseFee: 5000, congestionMultiplier: 1.0, successRate: 1.0 };
    this._timingHistory = [];
    this._timingModel = { bestHours: [], worstHours: [], avgSlotTime: 400 };
    this._endpointHistory.clear();
    this._endpointModel.clear();
    this._errorHistory = [];
    this._errorPatterns.clear();
    this._stats = {
      observationsRecorded: 0,
      beliefsFormed: 0,
      predictionsCorrect: 0,
      predictionsMade: 0,
      lastObservation: null,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create SolanaLearner singleton
 *
 * @param {Object} [options] - Options
 * @returns {SolanaLearner}
 */
export function getSolanaLearner(options = {}) {
  if (!_instance) {
    _instance = new SolanaLearner(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetSolanaLearner() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  SolanaLearner,
  SolanaLearningCategory,
  getSolanaLearner,
  resetSolanaLearner,
};
