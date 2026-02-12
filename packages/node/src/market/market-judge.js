/**
 * Market Judge — C3.2 (MARKET × JUDGE)
 *
 * Scores market events: price moves, liquidity changes, volume spikes.
 * Applies CYNIC's judgment framework to market data.
 *
 * "The market speaks — we judge what it says" - κυνικός
 *
 * @module @cynic/node/market/market-judge
 */

'use strict';

import { createLogger, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

const log = createLogger('MarketJudge');

/**
 * Verdict levels for market events
 * @readonly
 * @enum {string}
 */
export const MarketVerdict = {
  NOISE: 'noise',         // < 1% move — ignore
  NOTEWORTHY: 'noteworthy', // 1-23.6% — track
  SIGNIFICANT: 'significant', // 23.6-38.2% — alert
  MAJOR: 'major',         // 38.2-61.8% — investigate
  CRITICAL: 'critical',   // > 61.8% — emergency
};

/**
 * MarketJudge — Scores and judges market events
 *
 * Stub implementation for C3.2.
 * Future: Integrate with CYNIC Judge's 36-dimension scoring.
 */
export class MarketJudge {
  /**
   * Judge a price movement
   *
   * @param {Object} event - Market event from MarketWatcher
   * @param {number} event.price - Current price
   * @param {number} event.priceChange - Absolute change
   * @param {number} event.priceChangePercent - Percentage change
   * @returns {Object} Judgment
   */
  judgePriceMove(event) {
    const { price, priceChange, priceChangePercent } = event;

    // φ-bounded confidence (max 61.8%)
    let confidence = Math.min(PHI_INV, Math.abs(priceChangePercent) / 100);

    // Classify verdict
    let verdict = MarketVerdict.NOISE;
    if (Math.abs(priceChangePercent) >= PHI_INV * 100) {
      verdict = MarketVerdict.CRITICAL;
    } else if (Math.abs(priceChangePercent) >= PHI_INV_2 * 100) {
      verdict = MarketVerdict.MAJOR;
    } else if (Math.abs(priceChangePercent) >= PHI_INV_3 * 100) {
      verdict = MarketVerdict.SIGNIFICANT;
    } else if (Math.abs(priceChangePercent) >= 1) {
      verdict = MarketVerdict.NOTEWORTHY;
    }

    return {
      verdict,
      confidence,
      price,
      priceChange,
      priceChangePercent,
      direction: priceChange > 0 ? 'up' : 'down',
      timestamp: Date.now(),
    };
  }

  /**
   * Judge liquidity change
   *
   * @param {Object} event - Liquidity event
   * @returns {Object} Judgment (stub)
   */
  judgeLiquidityChange(event) {
    // Stub: Always return "noteworthy"
    return {
      verdict: MarketVerdict.NOTEWORTHY,
      confidence: 0.3,
      ...event,
      timestamp: Date.now(),
    };
  }

  /**
   * Judge volume spike
   *
   * @param {Object} event - Volume spike event
   * @returns {Object} Judgment (stub)
   */
  judgeVolumeSpike(event) {
    // Stub: Always return "significant"
    return {
      verdict: MarketVerdict.SIGNIFICANT,
      confidence: 0.4,
      ...event,
      timestamp: Date.now(),
    };
  }
}

// Singleton
let _instance = null;

export function getMarketJudge() {
  if (!_instance) _instance = new MarketJudge();
  return _instance;
}

export function resetMarketJudge() {
  _instance = null;
}
