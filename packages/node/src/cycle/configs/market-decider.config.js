/**
 * Market Decider Configuration (C3.3 - MARKET × DECIDE)
 *
 * Analyzes market data and generates trading signals.
 * Phase 1: Hypothetical decisions only (NO real trades)
 *
 * Decision types: BUY_SIGNAL, SELL_SIGNAL, ACCUMULATE, DISTRIBUTE, ALERT, HOLD, WAIT
 *
 * @module @cynic/node/cycle/configs/market-decider
 */

'use strict';

import { phiBound } from '@cynic/core/axioms/phi-utils.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { createLogger } from '@cynic/core';

const log = createLogger('MarketDecider');

/**
 * Decision types (7 types, Fibonacci-aligned)
 */
export const MarketDecisionType = {
  BUY_SIGNAL: 'buy_signal',
  SELL_SIGNAL: 'sell_signal',
  ACCUMULATE: 'accumulate',
  DISTRIBUTE: 'distribute',
  ALERT: 'alert',
  HOLD: 'hold',
  WAIT: 'wait',
};

/**
 * Severity levels
 */
const Severity = {
  CRITICAL: 'critical',
  HIGH: 'high',
  MEDIUM: 'medium',
  LOW: 'low',
};

/**
 * φ-window size for averaging (F6 = 8)
 */
const PHI_WINDOW = 8;

/**
 * Decision history size (F11 = 89)
 */
const HISTORY_SIZE = 89;

/**
 * Cooldown periods (Fibonacci minutes)
 */
const COOLDOWNS = {
  ALERT: 5 * 60 * 1000,      // 5 min
  BUY_SIGNAL: 3 * 60 * 1000,  // 3 min
  SELL_SIGNAL: 3 * 60 * 1000, // 3 min
};

/**
 * Market Decider Config
 *
 * 40% base logic (utilities, state management)
 * 60% delegated logic (core decision algorithm)
 */
export const marketDeciderConfig = {
  // ============================================================================
  // BASE LOGIC (40% - utilities, state, validation)
  // ============================================================================
  baseLogicFunctions: {
    /**
     * Initialize state
     */
    _initState() {
      this._priceHistory = [];
      this._volumeHistory = [];
      this._decisionHistory = [];
      this._lastAlertTime = 0;
      this._lastBuyTime = 0;
      this._lastSellTime = 0;
    },

    /**
     * Update price history (φ-window)
     */
    _updateHistory(price, volume) {
      this._priceHistory.push(price);
      this._volumeHistory.push(volume);

      // Keep only PHI_WINDOW items
      if (this._priceHistory.length > PHI_WINDOW) {
        this._priceHistory.shift();
      }
      if (this._volumeHistory.length > PHI_WINDOW) {
        this._volumeHistory.shift();
      }
    },

    /**
     * Calculate average price over window
     */
    _getAveragePrice() {
      if (this._priceHistory.length === 0) return null;
      const sum = this._priceHistory.reduce((a, b) => a + b, 0);
      return sum / this._priceHistory.length;
    },

    /**
     * Calculate average volume over window
     */
    _getAverageVolume() {
      if (this._volumeHistory.length === 0) return null;
      const sum = this._volumeHistory.reduce((a, b) => a + b, 0);
      return sum / this._volumeHistory.length;
    },

    /**
     * Check if on cooldown
     */
    _isOnCooldown(decisionType) {
      const now = Date.now();
      const cooldown = COOLDOWNS[decisionType];
      if (!cooldown) return false;

      let lastTime;
      if (decisionType === MarketDecisionType.ALERT) {
        lastTime = this._lastAlertTime;
      } else if (decisionType === MarketDecisionType.BUY_SIGNAL) {
        lastTime = this._lastBuyTime;
      } else if (decisionType === MarketDecisionType.SELL_SIGNAL) {
        lastTime = this._lastSellTime;
      }

      return lastTime && (now - lastTime) < cooldown;
    },

    /**
     * Update cooldown timestamp
     */
    _updateCooldown(decisionType) {
      const now = Date.now();
      if (decisionType === MarketDecisionType.ALERT) {
        this._lastAlertTime = now;
      } else if (decisionType === MarketDecisionType.BUY_SIGNAL) {
        this._lastBuyTime = now;
      } else if (decisionType === MarketDecisionType.SELL_SIGNAL) {
        this._lastSellTime = now;
      }
    },

    /**
     * Record decision in history
     */
    _recordDecision(decision) {
      this._decisionHistory.push({
        ...decision,
        timestamp: Date.now(),
      });

      // Keep only HISTORY_SIZE decisions
      if (this._decisionHistory.length > HISTORY_SIZE) {
        this._decisionHistory.shift();
      }
    },

    /**
     * Get decision statistics
     */
    getStats() {
      return {
        priceHistorySize: this._priceHistory.length,
        volumeHistorySize: this._volumeHistory.length,
        decisionHistorySize: this._decisionHistory.length,
        averagePrice: this._getAveragePrice(),
        averageVolume: this._getAverageVolume(),
      };
    },
  },

  // ============================================================================
  // DELEGATED LOGIC (60% - core decision algorithm)
  // ============================================================================
  delegatedLogic: {
    /**
     * Make trading decision based on market data
     *
     * @param {Object} marketData - Current market state
     * @param {number} marketData.price - Current price
     * @param {number} marketData.priceChange24h - 24h price change (decimal, e.g., -0.10 = -10%)
     * @param {number} marketData.volume - Current volume
     * @param {number} marketData.liquidity - Current liquidity
     * @param {string} marketData.sentiment - Sentiment ('positive', 'neutral', 'negative')
     * @param {Object} [context] - Additional context
     * @returns {Object} Decision object
     */
    async decide(marketData, context = {}) {
      const { price, priceChange24h, volume, liquidity, sentiment } = marketData;

      // Validate inputs
      if (!price || price <= 0) {
        return this._createDecision(MarketDecisionType.WAIT, Severity.LOW, 0.38, 'Invalid price data');
      }

      // Update history
      this._updateHistory(price, volume || 0);

      // Wait for sufficient history
      if (this._priceHistory.length < PHI_WINDOW) {
        return this._createDecision(
          MarketDecisionType.WAIT,
          Severity.LOW,
          0.38,
          `Building price history (${this._priceHistory.length}/${PHI_WINDOW})`
        );
      }

      // Calculate averages
      const avgPrice = this._getAveragePrice();
      const avgVolume = this._getAverageVolume();

      // Calculate factors
      const priceDeviation = (price - avgPrice) / avgPrice;
      const volumeRatio = volume / avgVolume;

      // Sentiment score (-1 to +1)
      const sentimentScore = sentiment === 'positive' ? 1 : sentiment === 'negative' ? -1 : 0;

      // Decision logic
      let decision = MarketDecisionType.HOLD;
      let severity = Severity.MEDIUM;
      let confidence = PHI_INV_3; // Default: φ⁻³ = 0.236
      let reason = '';

      // ALERT: Price crash >61.8% (φ⁻¹ threshold)
      if (priceChange24h < -PHI_INV && !this._isOnCooldown(MarketDecisionType.ALERT)) {
        decision = MarketDecisionType.ALERT;
        severity = Severity.CRITICAL;
        confidence = phiBound(0.70); // High confidence in crash detection
        reason = `Price crash detected: ${(priceChange24h * 100).toFixed(1)}% drop`;
        this._updateCooldown(MarketDecisionType.ALERT);
      }
      // BUY_SIGNAL: Price drop >23.6% (φ⁻²) + volume spike + positive sentiment
      else if (
        priceChange24h < -PHI_INV_2 &&
        volumeRatio > 1.5 &&
        sentimentScore > 0 &&
        !this._isOnCooldown(MarketDecisionType.BUY_SIGNAL)
      ) {
        decision = MarketDecisionType.BUY_SIGNAL;
        severity = Severity.HIGH;
        confidence = phiBound(0.50 + (sentimentScore * 0.08)); // Sentiment boost
        reason = `Buy opportunity: ${(priceChange24h * 100).toFixed(1)}% drop, ${volumeRatio.toFixed(1)}x volume, positive sentiment`;
        this._updateCooldown(MarketDecisionType.BUY_SIGNAL);
      }
      // ACCUMULATE: Gradual price decline + steady volume
      else if (priceChange24h < -0.10 && priceChange24h > -PHI_INV_2 && volumeRatio > 0.8 && volumeRatio < 1.5) {
        decision = MarketDecisionType.ACCUMULATE;
        severity = Severity.MEDIUM;
        confidence = phiBound(0.45);
        reason = `Gradual accumulation zone: ${(priceChange24h * 100).toFixed(1)}% decline, steady volume`;
      }
      // SELL_SIGNAL: Price pump >38.2% (φ⁻¹ × φ⁻²) + volume spike
      else if (
        priceChange24h > (PHI_INV * PHI_INV_2) &&
        volumeRatio > 2.0 &&
        !this._isOnCooldown(MarketDecisionType.SELL_SIGNAL)
      ) {
        decision = MarketDecisionType.SELL_SIGNAL;
        severity = Severity.HIGH;
        confidence = phiBound(0.52);
        reason = `Sell signal: ${(priceChange24h * 100).toFixed(1)}% pump, ${volumeRatio.toFixed(1)}x volume spike`;
        this._updateCooldown(MarketDecisionType.SELL_SIGNAL);
      }
      // DISTRIBUTE: Gradual price rise + high volume
      else if (priceChange24h > 0.15 && priceChange24h < (PHI_INV * PHI_INV_2) && volumeRatio > 1.3) {
        decision = MarketDecisionType.DISTRIBUTE;
        severity = Severity.MEDIUM;
        confidence = phiBound(0.47);
        reason = `Distribution zone: ${(priceChange24h * 100).toFixed(1)}% rise, elevated volume`;
      }
      // HOLD: No strong signal
      else {
        decision = MarketDecisionType.HOLD;
        severity = Severity.LOW;
        confidence = phiBound(0.40);
        reason = `No strong signal: ${(priceChange24h * 100).toFixed(1)}% change, ${volumeRatio.toFixed(1)}x volume`;
      }

      // Create decision object
      const decisionObj = this._createDecision(decision, severity, confidence, reason, {
        price,
        priceChange24h,
        volume,
        liquidity,
        sentiment,
        avgPrice,
        avgVolume,
        priceDeviation,
        volumeRatio,
      });

      // Record in history
      this._recordDecision(decisionObj);

      log.debug('Market decision made', {
        decision: decisionObj.decision,
        confidence: decisionObj.confidence,
        reason: decisionObj.reason,
      });

      return decisionObj;
    },

    /**
     * Create decision object
     */
    _createDecision(decision, severity, confidence, reason, factors = {}) {
      return {
        decision,
        severity,
        confidence: phiBound(confidence),
        reason,
        factors,
        timestamp: Date.now(),
      };
    },
  },

  // ============================================================================
  // METADATA
  // ============================================================================
  decisionTypes: MarketDecisionType,
  singletonName: 'MarketDecider',
  domain: 'MARKET',
  cell: 'C3.3',
};

export default marketDeciderConfig;
