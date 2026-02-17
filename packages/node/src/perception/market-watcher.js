/**
 * CYNIC Market Watcher - C3.1 (MARKET × PERCEIVE)
 *
 * Watches $BURN token market data on Solana:
 * - Price changes
 * - Volume spikes
 * - Liquidity changes
 * - Holder distribution
 *
 * Feeds MarketEmergence for pattern detection (pump/dump, whale activity).
 *
 * "Markets reveal truth through chaos" - κυνικός
 *
 * @module @cynic/node/perception/market-watcher
 */

'use strict';

import { EventEmitter } from 'events';
import { Connection, PublicKey } from '@solana/web3.js';
import { createLogger, PHI_INV, globalEventBus, EventType } from '@cynic/core';
import { getMarketEmergence } from '../market/market-emergence.js';
import { getEventBus } from '../services/event-bus.js';

const log = createLogger('MarketWatcher');

/**
 * Market event types
 * @readonly
 * @enum {string}
 */
export const MarketEventType = {
  PRICE_CHANGE: 'perception:market:price',
  VOLUME_SPIKE: 'perception:market:volume',
  LIQUIDITY_CHANGE: 'perception:market:liquidity',
  HOLDER_CHANGE: 'perception:market:holders',
  PATTERN_DETECTED: 'perception:market:pattern',
};

/**
 * Default poll intervals (φ-aligned)
 */
const POLL_INTERVALS = {
  price: 60 * 1000,        // 1 min (fast data)
  volume: 60 * 1000,       // 1 min
  liquidity: 5 * 60 * 1000, // 5 min (slower data)
  holders: 15 * 60 * 1000,  // 15 min (slowest)
};

/**
 * Rate limit backoff (exponential)
 */
const RATE_LIMIT_BACKOFF_MS = [1000, 2000, 5000, 10000, 30000]; // Up to 30s

/**
 * MarketWatcher - Polls Solana for $BURN market data
 *
 * Implements C3.1 (MARKET × PERCEIVE) in 7×7 matrix.
 * Feeds MarketEmergence (C3.7) for pattern detection.
 */
export class MarketWatcher extends EventEmitter {
  /**
   * Create MarketWatcher
   *
   * @param {Object} [options]
   * @param {string} [options.rpcUrl] - Solana RPC URL
   * @param {string} [options.tokenMint] - $BURN token mint address
   * @param {EventBus} [options.eventBus] - EventBus instance
   * @param {Object} [options.db] - PostgreSQL pool for heartbeats
   */
  constructor(options = {}) {
    super();

    this.rpcUrl = options.rpcUrl || process.env.SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com';
    this.tokenMint = options.tokenMint || process.env.BURN_TOKEN_MINT || null;
    this.eventBus = options.eventBus || getEventBus();
    this.db = options.db || null;

    this.connection = null;
    this._timers = new Map(); // type -> timer
    this._isRunning = false;
    this._rateLimitBackoffIndex = 0;

    // Market state
    this._lastPrice = null;
    this._lastVolume = null;
    this._lastLiquidity = null;
    this._lastHolderCount = null;

    // Stats
    this.stats = {
      priceUpdates: 0,
      volumeUpdates: 0,
      patternsDetected: 0,
      rateLimitHits: 0,
      errors: 0,
      lastPollAt: null,
    };

    // MarketEmergence integration
    this.marketEmergence = getMarketEmergence();
  }

  /**
   * Start watching market data
   */
  async start() {
    if (this._isRunning) {
      log.debug('MarketWatcher already running');
      return;
    }

    if (!this.tokenMint) {
      log.warn('No BURN_TOKEN_MINT configured — MarketWatcher disabled');
      return;
    }

    try {
      this.connection = new Connection(this.rpcUrl, {
        commitment: 'confirmed',
        confirmTransactionInitialTimeout: 30000,
      });

      // Test connection
      await this.connection.getSlot();
      log.info('MarketWatcher connected to Solana', { rpcUrl: this.rpcUrl });

      this._isRunning = true;

      // Start polling loops
      this._startPricePolling();
      this._startVolumePolling();

      // Emit startup event
      globalEventBus.emit(EventType.WATCHER_STARTED || 'watcher:started', {
        watcher: 'market',
        timestamp: Date.now(),
      });

      // Record heartbeat
      await this._recordHeartbeat('active');

      log.info('MarketWatcher started');
    } catch (error) {
      log.error('Failed to start MarketWatcher', { error: error.message });
      this._isRunning = false;
      throw error;
    }
  }

  /**
   * Stop watching
   */
  async stop() {
    if (!this._isRunning) return;

    this._isRunning = false;

    // Clear all timers
    for (const timer of this._timers.values()) {
      clearInterval(timer);
    }
    this._timers.clear();

    // Record final heartbeat
    await this._recordHeartbeat('stopped');

    globalEventBus.emit(EventType.WATCHER_STOPPED || 'watcher:stopped', {
      watcher: 'market',
      timestamp: Date.now(),
    });

    log.info('MarketWatcher stopped');
  }

  /**
   * Get current stats
   */
  getStats() {
    return { ...this.stats };
  }

  /**
   * Get current market state (for concurrent polling)
   */
  getState() {
    return {
      isRunning: this._isRunning,
      price: this._lastPrice,
      volume: this._lastVolume,
      liquidity: this._lastLiquidity,
      holderCount: this._lastHolderCount,
      stats: this.getStats(),
      timestamp: Date.now(),
    };
  }

  // =============================================================================
  // PRIVATE POLLING LOOPS
  // =============================================================================

  /**
   * Start price polling loop
   * @private
   */
  _startPricePolling() {
    const poll = async () => {
      if (!this._isRunning) return;

      try {
        const price = await this._fetchPrice();
        if (price !== null) {
          const priceChangePercent = this._lastPrice
            ? ((price - this._lastPrice) / this._lastPrice) * 100
            : 0;

          // Feed to MarketEmergence
          const pattern = this.marketEmergence.processEvent({
            type: 'price',
            price,
            priceChangePercent,
            timestamp: Date.now(),
          });

          if (pattern) {
            this.stats.patternsDetected++;
            this.eventBus.emit(MarketEventType.PATTERN_DETECTED, pattern);
          }

          // Emit price change
          this.eventBus.emit(MarketEventType.PRICE_CHANGE, {
            price,
            priceChangePercent,
            pattern: pattern?.type || null,
            timestamp: Date.now(),
          });

          this._lastPrice = price;
          this.stats.priceUpdates++;
          this.stats.lastPollAt = Date.now();

          // Reset rate limit backoff on success
          this._rateLimitBackoffIndex = 0;

          await this._recordHeartbeat('active', 1);
        }
      } catch (error) {
        this._handlePollError('price', error);
      }
    };

    // Initial poll
    poll();

    // Recurring poll
    const timer = setInterval(poll, POLL_INTERVALS.price);
    this._timers.set('price', timer);
  }

  /**
   * Start volume polling loop
   * @private
   */
  _startVolumePolling() {
    const poll = async () => {
      if (!this._isRunning) return;

      try {
        const volume = await this._fetchVolume();
        if (volume !== null) {
          // Feed to MarketEmergence
          this.marketEmergence.processEvent({
            type: 'volume',
            volume,
            timestamp: Date.now(),
          });

          this._lastVolume = volume;
          this.stats.volumeUpdates++;

          await this._recordHeartbeat('active', 1);
        }
      } catch (error) {
        this._handlePollError('volume', error);
      }
    };

    // Initial poll (offset by 30s to avoid collisions)
    setTimeout(poll, 30000);

    // Recurring poll
    const timer = setInterval(poll, POLL_INTERVALS.volume);
    this._timers.set('volume', timer);
  }

  // =============================================================================
  // DATA FETCHING (STUB IMPLEMENTATIONS)
  // =============================================================================

  /**
   * Fetch current $BURN price
   * @private
   * @returns {Promise<number|null>} Price in USD or null
   */
  async _fetchPrice() {
    try {
      // Try DexScreener first (free, no API key needed)
      try {
        const dexUrl = `https://api.dexscreener.com/latest/dex/tokens/${this.tokenMint}`;
        const dexResponse = await fetch(dexUrl, {
          timeout: 5000,
          headers: { 'User-Agent': 'CYNIC/0.1.0' }
        });

        if (dexResponse.ok) {
          const dexData = await dexResponse.json();
          const price = dexData.pairs?.[0]?.priceUsd;

          if (price) {
            log.debug('DexScreener price fetched', {
              price: parseFloat(price).toFixed(8),
              pair: dexData.pairs[0]?.pairAddress?.substring(0, 8)
            });

            // Emit MARKET_PRICE_UPDATED to globalEventBus
            globalEventBus.emit(EventType.MARKET_PRICE_UPDATED || 'market:price:updated', {
              source: 'DexScreener',
              price: parseFloat(price),
              tokenMint: this.tokenMint,
              timestamp: Date.now()
            });

            return parseFloat(price);
          }
        }
      } catch (err) {
        log.debug('DexScreener failed, trying Birdeye', { error: err.message });
      }

      // Fallback 1: Birdeye (requires API key)
      const birdeyeKey = process.env.BIRDEYE_API_KEY;
      if (birdeyeKey) {
        try {
          const birdeyeUrl = `https://public-api.birdeye.so/defi/price?address=${this.tokenMint}`;
          const birdeyeResponse = await fetch(birdeyeUrl, {
            timeout: 5000,
            headers: {
              'X-API-KEY': birdeyeKey,
              'User-Agent': 'CYNIC/0.1.0'
            }
          });

          if (birdeyeResponse.ok) {
            const birdeyeData = await birdeyeResponse.json();
            const price = birdeyeData.data?.value;

            if (price) {
              log.debug('Birdeye price fetched', { price: price.toFixed(8) });

              globalEventBus.emit(EventType.MARKET_PRICE_UPDATED || 'market:price:updated', {
                source: 'Birdeye',
                price,
                tokenMint: this.tokenMint,
                timestamp: Date.now()
              });

              return price;
            }
          }
        } catch (err) {
          log.debug('Birdeye failed, trying Jupiter', { error: err.message });
        }
      }

      // Fallback 2: Jupiter Price API v4
      try {
        const jupiterUrl = `https://price.jup.ag/v4/price?ids=${this.tokenMint}`;
        const jupiterResponse = await fetch(jupiterUrl, {
          timeout: 5000,
          headers: { 'User-Agent': 'CYNIC/0.1.0' }
        });

        if (jupiterResponse.ok) {
          const jupiterData = await jupiterResponse.json();
          const price = jupiterData.data?.[this.tokenMint]?.price;

          if (price) {
            log.debug('Jupiter price fetched', { price: price.toFixed(8) });

            globalEventBus.emit(EventType.MARKET_PRICE_UPDATED || 'market:price:updated', {
              source: 'Jupiter',
              price,
              tokenMint: this.tokenMint,
              timestamp: Date.now()
            });

            return price;
          }
        }
      } catch (err) {
        log.warn('All price sources failed', { error: err.message });
      }

      // All sources failed
      log.error('Failed to fetch price from all sources (DexScreener, Birdeye, Jupiter)');
      return null;
    } catch (error) {
      log.error('Price fetch error', { error: error.message });
      throw error;
    }
  }

  /**
   * Fetch 24h volume
   * @private
   * @returns {Promise<number|null>} Volume in USD or null
   */
  async _fetchVolume() {
    try {
      // DexScreener provides volume data in the same endpoint
      const dexUrl = `https://api.dexscreener.com/latest/dex/tokens/${this.tokenMint}`;
      const response = await fetch(dexUrl, {
        timeout: 5000,
        headers: { 'User-Agent': 'CYNIC/0.1.0' }
      });

      if (response.ok) {
        const data = await response.json();
        const pair = data.pairs?.[0];

        if (pair?.volume?.h24) {
          const volume = parseFloat(pair.volume.h24);
          log.debug('24h volume fetched', {
            volume: volume.toFixed(2),
            pair: pair.pairAddress?.substring(0, 8)
          });

          // Emit MARKET_VOLUME_UPDATED to globalEventBus
          globalEventBus.emit(EventType.MARKET_VOLUME_UPDATED || 'market:volume:updated', {
            source: 'DexScreener',
            volume24h: volume,
            tokenMint: this.tokenMint,
            timestamp: Date.now()
          });

          return volume;
        }
      }

      log.warn('Failed to fetch volume from DexScreener');
      return null;
    } catch (error) {
      log.debug('Volume fetch error', { error: error.message });
      throw error;
    }
  }

  // =============================================================================
  // ERROR HANDLING & HEARTBEATS
  // =============================================================================

  /**
   * Handle poll error with rate limit backoff
   * @private
   */
  _handlePollError(pollType, error) {
    this.stats.errors++;

    // Check for 429 rate limit
    if (error.message?.includes('429') || error.message?.includes('rate limit')) {
      this.stats.rateLimitHits++;

      // Exponential backoff
      const backoffMs = RATE_LIMIT_BACKOFF_MS[this._rateLimitBackoffIndex] || RATE_LIMIT_BACKOFF_MS[RATE_LIMIT_BACKOFF_MS.length - 1];
      this._rateLimitBackoffIndex = Math.min(this._rateLimitBackoffIndex + 1, RATE_LIMIT_BACKOFF_MS.length - 1);

      log.warn('MarketWatcher rate limited', {
        pollType,
        backoffMs,
        backoffIndex: this._rateLimitBackoffIndex,
      });

      // Pause this poll type temporarily
      const timer = this._timers.get(pollType);
      if (timer) {
        clearInterval(timer);
        setTimeout(() => {
          if (this._isRunning) {
            if (pollType === 'price') this._startPricePolling();
            else if (pollType === 'volume') this._startVolumePolling();
          }
        }, backoffMs);
      }

      this._recordHeartbeat('idle', 0, 'rate_limited');
    } else {
      log.debug('MarketWatcher poll error', {
        pollType,
        error: error.message,
      });

      this._recordHeartbeat('error', 0, error.message);
    }
  }

  /**
   * Record heartbeat to database
   * @private
   */
  async _recordHeartbeat(status, eventsPolled = 0, errorMessage = null) {
    if (!this.db) return;

    try {
      await this.db.query(`
        INSERT INTO watcher_heartbeats (watcher_name, status, events_polled, error_message)
        VALUES ($1, $2, $3, $4)
      `, ['market', status, eventsPolled, errorMessage]);
    } catch (err) {
      // Non-blocking
      log.debug('Failed to record heartbeat', { error: err.message });
    }
  }
}

// Singleton
let _instance = null;

/**
 * Get singleton instance
 */
export function getMarketWatcher() {
  if (!_instance) {
    _instance = new MarketWatcher();
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetMarketWatcher() {
  if (_instance) {
    _instance.stop();
  }
  _instance = null;
}

export default MarketWatcher;
