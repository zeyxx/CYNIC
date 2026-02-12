/**
 * Market Learner — C3.5 (MARKET × LEARN)
 *
 * Learns market patterns: typical volatility, price cycles, whale behavior.
 * Feeds predictions back to MarketJudge for adaptive thresholds.
 *
 * "The market teaches — we learn slowly" - κυνικός
 *
 * @module @cynic/node/market/market-learner
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('MarketLearner');

/**
 * MarketLearner — Learns from market outcomes
 *
 * Stub implementation for C3.5.
 * Future: EMA tracking, volatility bands, pattern recognition.
 */
export class MarketLearner {
  constructor() {
    // Historical data (rolling window)
    this.priceHistory = []; // Last 144 prices (F(12))
    this.volatilityEMA = null; // Exponential moving average
    this.learningRate = PHI_INV; // α = 0.618

    // Stats
    this.stats = {
      samplesProcessed: 0,
      patternsDetected: 0,
      predictionsCorrect: 0,
      predictionsFailed: 0,
    };
  }

  /**
   * Learn from a price update
   *
   * @param {Object} event - Price update event
   * @param {number} event.price - Current price
   * @param {number} event.priceChangePercent - Change %
   */
  learnFromPrice(event) {
    const { price, priceChangePercent } = event;

    this.priceHistory.push({ price, timestamp: Date.now() });

    // Keep only last 144 (F(12))
    if (this.priceHistory.length > 144) {
      this.priceHistory.shift();
    }

    // Update volatility EMA
    const volatility = Math.abs(priceChangePercent);
    if (this.volatilityEMA === null) {
      this.volatilityEMA = volatility;
    } else {
      this.volatilityEMA = this.learningRate * volatility + (1 - this.learningRate) * this.volatilityEMA;
    }

    this.stats.samplesProcessed++;

    log.debug('Market learned from price', {
      price,
      volatilityEMA: this.volatilityEMA.toFixed(2),
      samples: this.stats.samplesProcessed,
    });
  }

  /**
   * Predict if next move will be significant
   *
   * @returns {Object} Prediction (stub)
   */
  predictNextMove() {
    // Stub: Random prediction with φ-bounded confidence
    return {
      direction: Math.random() > 0.5 ? 'up' : 'down',
      magnitude: this.volatilityEMA || 5, // % expected change
      confidence: Math.min(PHI_INV, (this.stats.samplesProcessed / 144) * PHI_INV),
      timestamp: Date.now(),
    };
  }

  /**
   * Get learned volatility baseline
   *
   * @returns {number} Average volatility %
   */
  getVolatilityBaseline() {
    return this.volatilityEMA || 0;
  }

  /**
   * Get learning stats
   *
   * @returns {Object} Stats
   */
  getStats() {
    return { ...this.stats, volatilityEMA: this.volatilityEMA };
  }
}

// Singleton
let _instance = null;

export function getMarketLearner() {
  if (!_instance) _instance = new MarketLearner();
  return _instance;
}

export function resetMarketLearner() {
  _instance = null;
}
