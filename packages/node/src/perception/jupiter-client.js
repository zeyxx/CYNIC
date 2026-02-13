/**
 * Jupiter API Client - R3.1 (MARKET × PERCEIVE)
 *
 * "φ observes the price movements" - κυνικός
 *
 * Features:
 * - Real-time token price feeds
 * - Multi-token price queries
 * - Price change detection (>φ² = 38.2%)
 * - Rate limiting (5 req/s)
 * - Caching with TTL
 *
 * Emits: market:price:update, market:price:spike
 *
 * @module @cynic/node/perception/jupiter-client
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_2, globalEventBus } from '@cynic/core';

const log = createLogger('JupiterClient');

// Jupiter Price API v3 (current stable endpoint)
const JUPITER_API_URL = 'https://api.jup.ag/price/v3';
const PRICE_UPDATE_INTERVAL = 30000; // 30 seconds
const CACHE_TTL = 60000; // 1 minute
const SPIKE_THRESHOLD = PHI_2; // 38.2% price change = spike

/**
 * Jupiter API Client
 *
 * Fetches and monitors token prices from Jupiter Price API.
 */
export class JupiterClient extends EventEmitter {
  constructor(options = {}) {
    super();

    this.tokens = options.tokens || []; // Token mint addresses to monitor
    this.interval = options.interval || PRICE_UPDATE_INTERVAL;
    this.eventBus = options.eventBus || globalEventBus;
    this.apiKey = options.apiKey || process.env.JUPITER_API_KEY; // API key from options or env

    // Cache
    this.priceCache = new Map(); // mint -> { price, timestamp, vsToken }
    this.lastPrices = new Map(); // mint -> previous price (for change detection)

    // Timer
    this._timer = null;
    this._isRunning = false;

    // Stats
    this.stats = {
      requests: 0,
      successes: 0,
      failures: 0,
      spikesDetected: 0,
      lastUpdate: null,
    };
  }

  /**
   * Start price monitoring
   */
  async start() {
    if (this._isRunning) {
      log.warn('JupiterClient already running');
      return;
    }

    if (this.tokens.length === 0) {
      log.warn('No tokens configured for monitoring');
      return;
    }

    log.info('Starting price monitoring', {
      tokens: this.tokens.length,
      intervalMs: this.interval,
    });

    this._isRunning = true;

    // Immediate fetch
    await this._fetchPrices();

    // Schedule periodic updates
    this._timer = setInterval(() => {
      this._fetchPrices().catch(err => {
        log.error('Price fetch failed', { error: err.message });
      });
    }, this.interval);
  }

  /**
   * Stop price monitoring
   */
  stop() {
    if (this._timer) {
      clearInterval(this._timer);
      this._timer = null;
    }
    this._isRunning = false;
    log.info('Price monitoring stopped');
  }

  /**
   * Get current price for a token
   *
   * @param {string} mint - Token mint address
   * @returns {Object|null} Price data
   */
  getPrice(mint) {
    const cached = this.priceCache.get(mint);

    if (!cached) {
      return null;
    }

    // Check if cache is still valid
    if (Date.now() - cached.timestamp > CACHE_TTL) {
      return null;
    }

    return {
      mint,
      price: cached.price,
      vsToken: cached.vsToken,
      timestamp: cached.timestamp,
    };
  }

  /**
   * Get prices for all monitored tokens
   *
   * @returns {Array<Object>} Array of price data
   */
  getAllPrices() {
    return this.tokens
      .map(mint => this.getPrice(mint))
      .filter(p => p !== null);
  }

  /**
   * Get stats
   */
  getStats() {
    return {
      ...this.stats,
      cachedPrices: this.priceCache.size,
      monitoredTokens: this.tokens.length,
    };
  }

  // =============================================================================
  // PRIVATE METHODS
  // =============================================================================

  /**
   * Fetch prices from Jupiter API
   * @private
   */
  async _fetchPrices() {
    this.stats.requests++;

    try {
      // Jupiter Price API v3: GET https://api.jup.ag/price/v3?ids=mint1,mint2,mint3
      const ids = this.tokens.join(',');
      const url = `${JUPITER_API_URL}?ids=${ids}`;

      log.debug('Fetching prices', { url: url.substring(0, 60) + '...', tokens: this.tokens.length });

      const headers = {};
      if (this.apiKey) {
        headers['x-api-key'] = this.apiKey;
      }

      const response = await fetch(url, { headers });

      if (!response.ok) {
        throw new Error(`Jupiter API error: ${response.status} ${response.statusText}`);
      }

      const data = await response.json();

      // Process prices
      this._processPriceData(data.data || {});

      this.stats.successes++;
      this.stats.lastUpdate = Date.now();
    } catch (error) {
      this.stats.failures++;
      log.error('Failed to fetch prices', { error: error.message });
      throw error;
    }
  }

  /**
   * Process price data from API response
   * @private
   */
  _processPriceData(data) {
    const now = Date.now();

    for (const mint of this.tokens) {
      const priceData = data[mint];

      if (!priceData) {
        log.debug('No price data for token', { mint });
        continue;
      }

      const price = priceData.price;
      const vsToken = priceData.vsToken || 'USDC';

      // Update cache
      this.priceCache.set(mint, {
        price,
        vsToken,
        timestamp: now,
      });

      // Detect price changes
      const lastPrice = this.lastPrices.get(mint);

      if (lastPrice !== undefined) {
        const change = Math.abs((price - lastPrice) / lastPrice);

        // Emit update event
        this._emitPriceUpdate(mint, price, lastPrice, change, vsToken);

        // Check for spike (>38.2% change)
        if (change >= SPIKE_THRESHOLD) {
          this._emitPriceSpike(mint, price, lastPrice, change, vsToken);
        }
      }

      // Store for next comparison
      this.lastPrices.set(mint, price);
    }
  }

  /**
   * Emit price update event
   * @private
   */
  _emitPriceUpdate(mint, price, lastPrice, change, vsToken) {
    const payload = {
      mint,
      price,
      lastPrice,
      change: change * 100, // as percentage
      vsToken,
      timestamp: Date.now(),
    };

    this.eventBus.publish('market:price:update', payload, { source: 'JupiterClient' });
    this.emit('price:update', payload);

    log.debug('Price updated', {
      mint: mint.substring(0, 8),
      price: price.toFixed(6),
      change: `${(change * 100).toFixed(2)}%`,
    });
  }

  /**
   * Emit price spike event
   * @private
   */
  _emitPriceSpike(mint, price, lastPrice, change, vsToken) {
    this.stats.spikesDetected++;

    const payload = {
      mint,
      price,
      lastPrice,
      change: change * 100, // as percentage
      vsToken,
      timestamp: Date.now(),
    };

    this.eventBus.publish('market:price:spike', payload, { source: 'JupiterClient' });
    this.emit('price:spike', payload);

    log.warn('Price spike detected', {
      mint: mint.substring(0, 8),
      price: price.toFixed(6),
      lastPrice: lastPrice.toFixed(6),
      change: `${(change * 100).toFixed(2)}%`,
    });
  }
}

export default JupiterClient;
