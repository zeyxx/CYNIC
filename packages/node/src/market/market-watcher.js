/**
 * CYNIC Market Watcher - C3.1 (MARKET × PERCEIVE)
 *
 * Watches $asdfasdfa price, liquidity, and market sentiment.
 * Part of the 7×7 Fractal Matrix perception layer.
 *
 * "The market is truth, but φ-bounded truth" - κυνικός
 *
 * Perceives:
 * - Token price (SOL, USDC)
 * - Liquidity depth
 * - Volume (24h, 7d)
 * - Market cap
 * - Holder count
 * - Price velocity (rate of change)
 *
 * Data sources:
 * - Jupiter aggregator API (primary)
 * - Raydium pool data (fallback)
 * - Birdeye API (analytics)
 *
 * @module @cynic/node/market/market-watcher
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

const log = createLogger('MarketWatcher');

/**
 * $asdfasdfa token mint address (mainnet)
 */
export const ASDFASDFA_MINT = '9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump';

/**
 * Market event types emitted to EventBus
 * @readonly
 * @enum {string}
 */
export const MarketEventType = {
  PRICE_UPDATE: 'perception:market:price',
  LIQUIDITY_CHANGE: 'perception:market:liquidity',
  VOLUME_SPIKE: 'perception:market:volume_spike',
  HOLDER_CHANGE: 'perception:market:holders',
  MARKETCAP_UPDATE: 'perception:market:marketcap',
  PRICE_ALERT: 'perception:market:price_alert',
  ERROR: 'perception:market:error',
};

/**
 * API endpoints
 */
const API_ENDPOINTS = {
  // Jupiter aggregator - most reliable price data
  jupiter: 'https://price.jup.ag/v6',

  // Birdeye - analytics and holder data
  birdeye: 'https://public-api.birdeye.so',

  // DexScreener - backup price source
  dexscreener: 'https://api.dexscreener.com/latest/dex',
};

/**
 * φ-aligned polling intervals (Fibonacci milliseconds)
 */
const POLLING_INTERVALS = {
  price: 13 * 1000,      // F(7) = 13 seconds
  liquidity: 34 * 1000,  // F(9) = 34 seconds
  holders: 144 * 1000,   // F(12) = 144 seconds (~2.4 min)
};

/**
 * Price change thresholds for alerts (φ-bounded)
 */
const PRICE_THRESHOLDS = {
  // Percentage changes (as decimals)
  microMove: 0.01,        // 1% - noise
  smallMove: PHI_INV_3,   // 23.6% - noteworthy
  mediumMove: PHI_INV_2,  // 38.2% - significant
  largeMove: PHI_INV,     // 61.8% - major event

  // Velocity thresholds (% per minute)
  slowVelocity: 0.001,    // 0.1%/min
  fastVelocity: 0.01,     // 1%/min
  extremeVelocity: 0.05,  // 5%/min
};

/**
 * MarketWatcher - Watches $asdfasdfa market data
 *
 * Implements the perception layer for market state (C3.1).
 * Polls APIs at φ-aligned intervals, emits events on changes.
 */
export class MarketWatcher extends EventEmitter {
  /**
   * Create a market watcher
   *
   * @param {Object} options
   * @param {string} options.tokenMint - Token mint address (default: $asdfasdfa)
   * @param {boolean} options.autoStart - Start polling immediately (default: false)
   * @param {Object} options.intervals - Custom polling intervals
   */
  constructor(options = {}) {
    super();

    this.tokenMint = options.tokenMint || ASDFASDFA_MINT;
    this.intervals = { ...POLLING_INTERVALS, ...options.intervals };

    // State
    this.running = false;
    this.lastPrice = null;
    this.lastLiquidity = null;
    this.lastHolderCount = null;
    this.priceHistory = []; // Last 144 price points (F(12))

    // Timers
    this._priceTimer = null;
    this._liquidityTimer = null;
    this._holderTimer = null;

    // Stats
    this.stats = {
      priceUpdates: 0,
      liquidityUpdates: 0,
      holderUpdates: 0,
      errors: 0,
      lastUpdate: null,
      uptime: 0,
    };

    this._startTime = null;

    if (options.autoStart) {
      this.start();
    }
  }

  /**
   * Start watching the market
   */
  start() {
    if (this.running) {
      log.debug('MarketWatcher already running');
      return;
    }

    this.running = true;
    this._startTime = Date.now();

    log.info('MarketWatcher started', {
      tokenMint: this.tokenMint,
      intervals: this.intervals,
    });

    // Initial fetch (immediate)
    this._fetchPrice();
    this._fetchLiquidity();
    this._fetchHolders();

    // Start polling
    this._priceTimer = setInterval(() => this._fetchPrice(), this.intervals.price);
    this._liquidityTimer = setInterval(() => this._fetchLiquidity(), this.intervals.liquidity);
    this._holderTimer = setInterval(() => this._fetchHolders(), this.intervals.holders);

    // Unref timers so they don't block process exit
    this._priceTimer.unref();
    this._liquidityTimer.unref();
    this._holderTimer.unref();

    this.emit('started');
  }

  /**
   * Stop watching the market
   */
  stop() {
    if (!this.running) return;

    this.running = false;

    if (this._priceTimer) {
      clearInterval(this._priceTimer);
      this._priceTimer = null;
    }
    if (this._liquidityTimer) {
      clearInterval(this._liquidityTimer);
      this._liquidityTimer = null;
    }
    if (this._holderTimer) {
      clearInterval(this._holderTimer);
      this._holderTimer = null;
    }

    if (this._startTime) {
      this.stats.uptime = Date.now() - this._startTime;
    }

    log.info('MarketWatcher stopped', { stats: this.stats });
    this.emit('stopped');
  }

  /**
   * Fetch current price from Jupiter
   * @private
   */
  async _fetchPrice() {
    try {
      const url = `${API_ENDPOINTS.jupiter}/price?ids=${this.tokenMint}`;
      const response = await fetch(url);

      if (!response.ok) {
        throw new Error(`Jupiter API error: ${response.status}`);
      }

      const data = await response.json();
      const priceData = data.data?.[this.tokenMint];

      if (!priceData) {
        throw new Error('No price data returned');
      }

      const price = priceData.price; // Price in USD
      const timestamp = Date.now();

      // Calculate price change
      const priceChange = this.lastPrice ? (price - this.lastPrice) / this.lastPrice : 0;
      const priceChangePercent = priceChange * 100;

      // Update history (keep last F(12) = 144 points)
      this.priceHistory.push({ price, timestamp });
      if (this.priceHistory.length > 144) {
        this.priceHistory.shift();
      }

      // Calculate velocity (price change per minute)
      const velocity = this._calculateVelocity();

      // Emit price update event
      const event = {
        type: MarketEventType.PRICE_UPDATE,
        token: this.tokenMint,
        price,
        priceUSD: price,
        priceChange,
        priceChangePercent,
        velocity,
        timestamp,
      };

      this.emit(MarketEventType.PRICE_UPDATE, event);

      // Check for price alerts
      if (Math.abs(priceChange) >= PRICE_THRESHOLDS.mediumMove) {
        const alert = {
          type: MarketEventType.PRICE_ALERT,
          token: this.tokenMint,
          severity: Math.abs(priceChange) >= PRICE_THRESHOLDS.largeMove ? 'high' : 'medium',
          direction: priceChange > 0 ? 'up' : 'down',
          priceChange,
          priceChangePercent,
          price,
          timestamp,
        };

        this.emit(MarketEventType.PRICE_ALERT, alert);
        log.info('Price alert', alert);
      }

      this.lastPrice = price;
      this.stats.priceUpdates++;
      this.stats.lastUpdate = timestamp;

    } catch (err) {
      this.stats.errors++;
      log.debug('Price fetch failed', { error: err.message });

      const errorEvent = {
        type: MarketEventType.ERROR,
        source: 'price',
        error: err.message,
        timestamp: Date.now(),
      };

      this.emit(MarketEventType.ERROR, errorEvent);
    }
  }

  /**
   * Fetch liquidity data from DexScreener
   * @private
   */
  async _fetchLiquidity() {
    try {
      const url = `${API_ENDPOINTS.dexscreener}/tokens/${this.tokenMint}`;
      const response = await fetch(url);

      if (!response.ok) {
        throw new Error(`DexScreener API error: ${response.status}`);
      }

      const data = await response.json();
      const pair = data.pairs?.[0]; // Get the main pair

      if (!pair) {
        throw new Error('No pair data returned');
      }

      const liquidity = {
        usd: pair.liquidity?.usd || 0,
        base: pair.liquidity?.base || 0,
        quote: pair.liquidity?.quote || 0,
      };

      const volume24h = pair.volume?.h24 || 0;
      const marketCap = pair.fdv || 0; // Fully diluted valuation
      const timestamp = Date.now();

      // Emit liquidity event
      const event = {
        type: MarketEventType.LIQUIDITY_CHANGE,
        token: this.tokenMint,
        liquidity,
        volume24h,
        marketCap,
        pairAddress: pair.pairAddress,
        dexId: pair.dexId,
        timestamp,
      };

      this.emit(MarketEventType.LIQUIDITY_CHANGE, event);

      // Check for volume spike
      if (this.lastLiquidity && volume24h > this.lastLiquidity.volume24h * 2) {
        this.emit(MarketEventType.VOLUME_SPIKE, {
          type: MarketEventType.VOLUME_SPIKE,
          token: this.tokenMint,
          volume24h,
          previousVolume: this.lastLiquidity.volume24h,
          multiplier: volume24h / this.lastLiquidity.volume24h,
          timestamp,
        });
      }

      this.lastLiquidity = { liquidity, volume24h, marketCap };
      this.stats.liquidityUpdates++;
      this.stats.lastUpdate = timestamp;

    } catch (err) {
      this.stats.errors++;
      log.debug('Liquidity fetch failed', { error: err.message });

      const errorEvent = {
        type: MarketEventType.ERROR,
        source: 'liquidity',
        error: err.message,
        timestamp: Date.now(),
      };

      this.emit(MarketEventType.ERROR, errorEvent);
    }
  }

  /**
   * Fetch holder count from Birdeye (requires API key)
   * @private
   */
  async _fetchHolders() {
    // Birdeye requires API key - skip if not configured
    const apiKey = process.env.BIRDEYE_API_KEY;

    if (!apiKey) {
      log.debug('Birdeye API key not configured - skipping holder fetch');
      return;
    }

    try {
      const url = `${API_ENDPOINTS.birdeye}/token/holder?address=${this.tokenMint}`;
      const response = await fetch(url, {
        headers: { 'X-API-KEY': apiKey },
      });

      if (!response.ok) {
        throw new Error(`Birdeye API error: ${response.status}`);
      }

      const data = await response.json();
      const holderCount = data.data?.holderCount || 0;
      const timestamp = Date.now();

      // Emit holder change event
      const event = {
        type: MarketEventType.HOLDER_CHANGE,
        token: this.tokenMint,
        holderCount,
        holderChange: this.lastHolderCount ? holderCount - this.lastHolderCount : 0,
        timestamp,
      };

      this.emit(MarketEventType.HOLDER_CHANGE, event);

      this.lastHolderCount = holderCount;
      this.stats.holderUpdates++;
      this.stats.lastUpdate = timestamp;

    } catch (err) {
      this.stats.errors++;
      log.debug('Holder fetch failed', { error: err.message });

      const errorEvent = {
        type: MarketEventType.ERROR,
        source: 'holders',
        error: err.message,
        timestamp: Date.now(),
      };

      this.emit(MarketEventType.ERROR, errorEvent);
    }
  }

  /**
   * Calculate price velocity (rate of change)
   * @private
   * @returns {number} Velocity in %/minute
   */
  _calculateVelocity() {
    if (this.priceHistory.length < 2) return 0;

    // Use last 13 points (F(7)) for velocity calculation
    const window = this.priceHistory.slice(-13);
    if (window.length < 2) return 0;

    const first = window[0];
    const last = window[window.length - 1];

    const priceChange = (last.price - first.price) / first.price;
    const timeDiffMinutes = (last.timestamp - first.timestamp) / (1000 * 60);

    if (timeDiffMinutes === 0) return 0;

    return (priceChange / timeDiffMinutes) * 100; // %/minute
  }

  /**
   * Get current market state snapshot
   * @returns {Object} Current state
   */
  getState() {
    return {
      running: this.running,
      tokenMint: this.tokenMint,
      lastPrice: this.lastPrice,
      lastLiquidity: this.lastLiquidity,
      lastHolderCount: this.lastHolderCount,
      velocity: this._calculateVelocity(),
      stats: { ...this.stats },
      uptime: this._startTime ? Date.now() - this._startTime : 0,
    };
  }

  /**
   * Get recent price history
   * @param {number} count - Number of recent points (default: 21 = F(8))
   * @returns {Array<{price: number, timestamp: number}>}
   */
  getPriceHistory(count = 21) {
    return this.priceHistory.slice(-count);
  }
}

/**
 * Create a market watcher singleton
 */
let _instance = null;

/**
 * Get or create the market watcher singleton
 *
 * @param {Object} [options] - Options for first initialization
 * @returns {MarketWatcher}
 */
export function getMarketWatcher(options = {}) {
  if (!_instance) {
    _instance = new MarketWatcher(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetMarketWatcher() {
  if (_instance) {
    _instance.stop();
    _instance = null;
  }
}

export default MarketWatcher;
