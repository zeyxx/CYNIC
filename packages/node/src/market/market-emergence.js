/**
 * Market Emergence — C3.7 (MARKET × EMERGE)
 *
 * Detects emergent patterns in market behavior:
 * - Pump & dump signals
 * - Whale accumulation/distribution
 * - Coordinated trading patterns
 * - Market manipulation
 *
 * "Patterns emerge from chaos" - κυνικός
 *
 * @module @cynic/node/market/market-emergence
 */

'use strict';

import { createLogger, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

const log = createLogger('MarketEmergence');

/**
 * Emergent pattern types
 * @readonly
 * @enum {string}
 */
export const PatternType = {
  PUMP_AND_DUMP: 'pump_and_dump',
  WHALE_ACCUMULATION: 'whale_accumulation',
  WHALE_DISTRIBUTION: 'whale_distribution',
  COORDINATED_BUY: 'coordinated_buy',
  COORDINATED_SELL: 'coordinated_sell',
  MANIPULATION: 'manipulation',
  ORGANIC_GROWTH: 'organic_growth',
};

/**
 * MarketEmergence — Detects emergent market patterns
 *
 * Stub implementation for C3.7.
 * Future: ML-based pattern recognition, graph analysis.
 */
export class MarketEmergence {
  constructor() {
    // Event buffer for pattern detection
    this.eventWindow = []; // Last 89 events (F(11))
    this.detectedPatterns = [];

    // Stats
    this.stats = {
      patternsDetected: 0,
      pumpsDetected: 0,
      dumpsDetected: 0,
      whaleActivity: 0,
    };
  }

  /**
   * Process market event for pattern detection
   *
   * @param {Object} event - Market event
   * @returns {Object|null} Detected pattern or null
   */
  processEvent(event) {
    this.eventWindow.push({ ...event, timestamp: Date.now() });

    // Keep last 89 events (F(11))
    if (this.eventWindow.length > 89) {
      this.eventWindow.shift();
    }

    // Stub: Simple pump detection
    // Real implementation would use ML + graph analysis
    const pattern = this._detectSimplePattern();

    if (pattern) {
      this.detectedPatterns.push(pattern);
      this.stats.patternsDetected++;

      if (pattern.type === PatternType.PUMP_AND_DUMP) {
        this.stats.pumpsDetected++;
      }

      log.warn('Market pattern detected', {
        type: pattern.type,
        confidence: pattern.confidence.toFixed(2),
      });

      return pattern;
    }

    return null;
  }

  /**
   * Advanced pattern detection
   * @private
   */
  _detectSimplePattern() {
    if (this.eventWindow.length < 8) return null;

    // Try multiple detection methods
    const pumpPattern = this._detectPumpAndDump();
    const whalePattern = this._detectWhaleActivity();
    const volatilityPattern = this._detectVolatilityClustering();
    const coordPattern = this._detectCoordinatedTrading();

    // Return highest confidence pattern
    const patterns = [pumpPattern, whalePattern, volatilityPattern, coordPattern].filter(Boolean);
    if (patterns.length === 0) return null;

    patterns.sort((a, b) => b.confidence - a.confidence);
    return patterns[0];
  }

  /**
   * Detect pump-and-dump pattern
   * @private
   */
  _detectPumpAndDump() {
    if (this.eventWindow.length < 13) return null;

    const recent = this.eventWindow.slice(-13); // F(7) = 13
    const changes = recent.map(e => e.priceChangePercent || 0);

    // Phase 1: Rapid rise (first 8 events)
    const phase1 = changes.slice(0, 8);
    const avgRise = phase1.reduce((a, b) => a + b, 0) / phase1.length;
    const riseConsistency = phase1.filter(c => c > 0).length / phase1.length;

    // Phase 2: Rapid fall (last 5 events)
    const phase2 = changes.slice(-5);
    const avgFall = phase2.reduce((a, b) => a + b, 0) / phase2.length;
    const fallConsistency = phase2.filter(c => c < 0).length / phase2.length;

    // Pump-dump criteria:
    // 1. Strong rise (>φ⁻² avg, 75%+ positive)
    // 2. Strong fall (<-φ⁻³ avg, 60%+ negative)
    if (
      avgRise > PHI_INV_2 * 100 &&
      riseConsistency > 0.75 &&
      avgFall < -PHI_INV_3 * 100 &&
      fallConsistency > 0.6
    ) {
      return {
        type: PatternType.PUMP_AND_DUMP,
        confidence: Math.min(PHI_INV, (riseConsistency + fallConsistency) / 2),
        avgRise,
        avgFall,
        riseConsistency,
        fallConsistency,
        timestamp: Date.now(),
      };
    }

    return null;
  }

  /**
   * Detect whale accumulation/distribution
   * @private
   */
  _detectWhaleActivity() {
    if (this.eventWindow.length < 5) return null;

    const recent = this.eventWindow.slice(-5);
    const volumes = recent.map(e => e.volume || 0).filter(v => v > 0);
    if (volumes.length === 0) return null;

    const avgVolume = volumes.reduce((a, b) => a + b, 0) / volumes.length;
    const priceChanges = recent.map(e => Math.abs(e.priceChangePercent || 0));
    const avgPriceChange = priceChanges.reduce((a, b) => a + b, 0) / priceChanges.length;

    // Whale signal: High volume + low price movement
    // (accumulation/distribution without moving price)
    const volumeSpike = volumes.some(v => v > avgVolume * 3);
    const lowVolatility = avgPriceChange < 5; // <5% average move

    if (volumeSpike && lowVolatility) {
      const direction = recent.slice(-3).every(e => (e.volume || 0) > avgVolume) ? 'accumulation' : 'distribution';

      return {
        type: direction === 'accumulation' ? PatternType.WHALE_ACCUMULATION : PatternType.WHALE_DISTRIBUTION,
        confidence: Math.min(PHI_INV, 0.5),
        avgVolume,
        avgPriceChange,
        timestamp: Date.now(),
      };
    }

    return null;
  }

  /**
   * Detect volatility clustering
   * @private
   */
  _detectVolatilityClustering() {
    if (this.eventWindow.length < 21) return null;

    const changes = this.eventWindow.slice(-21).map(e => Math.abs(e.priceChangePercent || 0));

    // Split into 3 periods of 7 (F(5))
    const period1 = changes.slice(0, 7);
    const period2 = changes.slice(7, 14);
    const period3 = changes.slice(14, 21);

    const vol1 = period1.reduce((a, b) => a + b, 0) / period1.length;
    const vol2 = period2.reduce((a, b) => a + b, 0) / period2.length;
    const vol3 = period3.reduce((a, b) => a + b, 0) / period3.length;

    // Clustering: volatility increasing over time
    const isIncreasing = vol3 > vol2 * 1.5 && vol2 > vol1 * 1.2;

    if (isIncreasing && vol3 > PHI_INV_2 * 100) {
      return {
        type: PatternType.MANIPULATION, // High volatility clustering suggests manipulation
        confidence: Math.min(PHI_INV, vol3 / 100),
        vol1,
        vol2,
        vol3,
        timestamp: Date.now(),
      };
    }

    return null;
  }

  /**
   * Detect coordinated trading
   * @private
   */
  _detectCoordinatedTrading() {
    if (this.eventWindow.length < 8) return null;

    const recent = this.eventWindow.slice(-8);
    const changes = recent.map(e => e.priceChangePercent || 0);

    // Coordinated buy: 5+ consecutive positive moves, similar magnitude
    const positiveMoves = changes.filter(c => c > 0);
    if (positiveMoves.length >= 5) {
      const avgPositive = positiveMoves.reduce((a, b) => a + b, 0) / positiveMoves.length;
      const stdDev = Math.sqrt(
        positiveMoves.map(c => Math.pow(c - avgPositive, 2)).reduce((a, b) => a + b, 0) / positiveMoves.length
      );

      // Low std dev = similar magnitudes = coordinated
      if (stdDev < avgPositive * 0.3 && avgPositive > 10) {
        return {
          type: PatternType.COORDINATED_BUY,
          confidence: Math.min(PHI_INV, 0.55),
          avgChange: avgPositive,
          consistency: 1 - (stdDev / avgPositive),
          timestamp: Date.now(),
        };
      }
    }

    // Coordinated sell: 5+ consecutive negative moves
    const negativeMoves = changes.filter(c => c < 0);
    if (negativeMoves.length >= 5) {
      const avgNegative = Math.abs(negativeMoves.reduce((a, b) => a + b, 0) / negativeMoves.length);
      const stdDev = Math.sqrt(
        negativeMoves.map(c => Math.pow(Math.abs(c) - avgNegative, 2)).reduce((a, b) => a + b, 0) / negativeMoves.length
      );

      if (stdDev < avgNegative * 0.3 && avgNegative > 10) {
        return {
          type: PatternType.COORDINATED_SELL,
          confidence: Math.min(PHI_INV, 0.55),
          avgChange: -avgNegative,
          consistency: 1 - (stdDev / avgNegative),
          timestamp: Date.now(),
        };
      }
    }

    return null;
  }

  /**
   * Get detected patterns
   *
   * @param {number} [limit=21] - Max patterns to return
   * @returns {Array} Recent patterns
   */
  getPatterns(limit = 21) {
    return this.detectedPatterns.slice(-limit);
  }

  /**
   * Get emergence stats
   *
   * @returns {Object} Stats
   */
  getStats() {
    return { ...this.stats };
  }
}

// Singleton
let _instance = null;

export function getMarketEmergence() {
  if (!_instance) _instance = new MarketEmergence();
  return _instance;
}

export function resetMarketEmergence() {
  _instance = null;
}
