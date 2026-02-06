/**
 * @cynic/agent - Perceiver Module
 *
 * Monitors Solana for trading opportunities and signals.
 * Uses Jupiter Price API v2 + DexScreener for real market data.
 * "Les yeux du chien ne dorment jamais" - κυνικός
 *
 * @module @cynic/agent/perceiver
 */

'use strict';

import { EventEmitter } from 'eventemitter3';
import { createSolanaRpc, address } from '@solana/kit';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('Perceiver');

// Dynamic import of SolanaWatcher (handle if not available)
let SolanaWatcherModule = null;
try {
  SolanaWatcherModule = await import('@cynic/node/perception/solana-watcher.js');
} catch (e) {
  log.debug('SolanaWatcher not available - API-only mode');
}

// Lazy-load inference modules for signal enrichment
let _inferenceModules = null;
async function loadInferenceModules() {
  if (_inferenceModules) return _inferenceModules;
  try {
    const { EventRateTracker, computeStats, zScore } = await import('@cynic/node/inference');
    _inferenceModules = { EventRateTracker, computeStats, zScore };
    return _inferenceModules;
  } catch (e) {
    log.debug('Inference modules not available', { error: e.message });
    return null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API Endpoints
// ═══════════════════════════════════════════════════════════════════════════════

const JUPITER_PRICE_API = 'https://api.jup.ag/price/v2';
const DEXSCREENER_API = 'https://api.dexscreener.com/latest/dex';

// Well-known token mints for default monitoring
const KNOWN_TOKENS = {
  SOL: 'So11111111111111111111111111111111111111112',
  USDC: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  BONK: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263',
  WIF: 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm',
  JUP: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN',
  PYTH: 'HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3',
  RAY: '4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R',
};

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

const DEFAULT_CONFIG = {
  rpcUrl: process.env.SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com',
  pollInterval: 15000, // 15s (respect rate limits)
  watchedTokens: Object.values(KNOWN_TOKENS), // Default to known tokens
  volumeThreshold: 1000, // Minimum 24h volume in USD
  priceChangeThreshold: 0.03, // 3% price change triggers signal
  liquidityThreshold: 10000, // Minimum liquidity in USD
  useRealAPIs: true, // Toggle for real vs synthetic
  // SolanaWatcher config
  useSolanaWatcher: true, // Enable WebSocket monitoring
  cluster: process.env.SOLANA_CLUSTER || 'mainnet',
  whaleThreshold: 100, // SOL amount to consider "whale"
};

// ═══════════════════════════════════════════════════════════════════════════════
// Signal Types
// ═══════════════════════════════════════════════════════════════════════════════

export const SignalType = {
  PRICE_SPIKE: 'price_spike',
  PRICE_DROP: 'price_drop',
  VOLUME_SURGE: 'volume_surge',
  WHALE_MOVEMENT: 'whale_movement',
  NEW_TOKEN: 'new_token',
  LIQUIDITY_ADDED: 'liquidity_added',
  LIQUIDITY_REMOVED: 'liquidity_removed',
  // SolanaWatcher signals
  SWAP_DETECTED: 'swap_detected',
  ACCOUNT_CHANGE: 'account_change',
};

// ═══════════════════════════════════════════════════════════════════════════════
// Perceiver Class
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Perceiver - Monitors Solana for opportunities
 */
export class Perceiver extends EventEmitter {
  constructor(options = {}) {
    super();

    this.config = { ...DEFAULT_CONFIG, ...options };
    this.rpc = null;
    this.pollTimer = null;
    this.isRunning = false;

    // State
    this.latestSignal = null;
    this.signalHistory = [];
    this.maxHistory = 100;

    // Price/volume cache for change detection
    this.priceCache = new Map();
    this.volumeCache = new Map();

    // Metrics
    this.metrics = {
      pollCount: 0,
      signalsDetected: 0,
      opportunitiesFound: 0,
      errors: 0,
      solanaEvents: 0, // SolanaWatcher events
    };

    // SolanaWatcher state
    this.solanaWatcher = null;
    this._watcherSubscriptions = [];

    // Inference enrichment (wired lazily)
    this._rateTracker = null;
    this._priceHistory = [];
    this._inferenceReady = false;
  }

  /**
   * Start monitoring
   */
  async start() {
    if (this.isRunning) return;

    log.info('Starting perceiver...', { rpcUrl: this.config.rpcUrl });

    this.rpc = createSolanaRpc(this.config.rpcUrl);
    this.isRunning = true;

    // Initialize inference modules for signal enrichment
    await this._ensureInference();

    // Initialize SolanaWatcher for WebSocket monitoring
    if (this.config.useSolanaWatcher && SolanaWatcherModule) {
      await this._initSolanaWatcher();
    }

    // Initial poll
    await this._poll();

    // Start polling loop
    this.pollTimer = setInterval(() => this._poll(), this.config.pollInterval);

    log.info('Perceiver started', {
      polling: true,
      solanaWatcher: !!this.solanaWatcher,
    });
  }

  /**
   * Stop monitoring
   */
  async stop() {
    if (!this.isRunning) return;

    if (this.pollTimer) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }

    // Stop SolanaWatcher
    if (this.solanaWatcher) {
      try {
        await this.solanaWatcher.stop();
        this.solanaWatcher = null;
        this._watcherSubscriptions = [];
      } catch (e) {
        log.warn('Error stopping SolanaWatcher', { error: e.message });
      }
    }

    this.isRunning = false;
    log.info('Perceiver stopped');
  }

  // ═══════════════════════════════════════════════════════════════════════════════
  // SolanaWatcher Integration
  // ═══════════════════════════════════════════════════════════════════════════════

  /**
   * Initialize SolanaWatcher for WebSocket-based monitoring
   * @private
   */
  async _initSolanaWatcher() {
    try {
      const { SolanaWatcher } = SolanaWatcherModule;

      this.solanaWatcher = new SolanaWatcher({
        rpcUrl: this.config.rpcUrl,
        cluster: this.config.cluster,
      });

      await this.solanaWatcher.start();

      // Watch major swap programs
      const swapPrograms = [
        'JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4', // Jupiter v6
        '675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8', // Raydium AMM
        'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc', // Orca Whirlpool
      ];

      for (const program of swapPrograms) {
        try {
          const subId = await this.solanaWatcher.watchLogs(program);
          this._watcherSubscriptions.push(subId);
        } catch (e) {
          log.debug('Could not watch program', { program, error: e.message });
        }
      }

      // Wire event handlers
      this.solanaWatcher.on('log', (event) => this._handleSwapLog(event));
      this.solanaWatcher.on('account_change', (event) => this._handleAccountChange(event));

      log.info('SolanaWatcher initialized', {
        subscriptions: this._watcherSubscriptions.length,
        cluster: this.config.cluster,
      });
    } catch (error) {
      log.warn('SolanaWatcher init failed - continuing with API-only mode', {
        error: error.message,
      });
      this.solanaWatcher = null;
    }
  }

  /**
   * Handle swap logs from SolanaWatcher
   * @private
   */
  _handleSwapLog(event) {
    this.metrics.solanaEvents++;

    const logs = event.logs || [];
    const isSwap = logs.some(log =>
      log.includes('Swap') || log.includes('swap') || log.includes('Instruction: Route')
    );

    if (isSwap) {
      const signal = {
        type: SignalType.SWAP_DETECTED,
        token: 'Unknown', // Would need parsing from logs
        mint: null,
        data: {
          signature: event.signature,
          slot: event.slot,
          logs: logs.slice(0, 5), // First 5 logs for context
        },
        timestamp: event.timestamp || Date.now(),
        source: 'solana_watcher',
        confidence: PHI_INV_2, // 38.2% - low confidence until verified
      };

      this._recordSignal(signal);

      // Emit as potential opportunity
      if (this._isOpportunity(signal)) {
        this.metrics.opportunitiesFound++;
        this.emit('opportunity', this._signalToOpportunity(signal));
      }
    }
  }

  /**
   * Handle account changes (whale movements)
   * @private
   */
  _handleAccountChange(event) {
    this.metrics.solanaEvents++;

    const solChange = (event.lamports || 0) / 1e9;
    const absChange = Math.abs(solChange);

    // Only signal for whale movements
    if (absChange >= this.config.whaleThreshold) {
      const signal = {
        type: SignalType.WHALE_MOVEMENT,
        token: 'SOL',
        mint: KNOWN_TOKENS.SOL,
        data: {
          pubkey: event.pubkey,
          solChange,
          slot: event.slot,
          direction: solChange > 0 ? 'inflow' : 'outflow',
        },
        timestamp: event.timestamp || Date.now(),
        source: 'solana_watcher',
        confidence: PHI_INV, // 61.8% - high confidence for on-chain data
      };

      this._recordSignal(signal);

      // Whale movements are always interesting
      this.metrics.opportunitiesFound++;
      this.emit('opportunity', this._signalToOpportunity(signal));
    }
  }

  /**
   * Main polling loop
   * @private
   */
  async _poll() {
    if (!this.isRunning) return;

    this.metrics.pollCount++;

    try {
      // Get market data (simplified - in production use Jupiter/Birdeye API)
      const signals = await this._detectSignals();

      for (const signal of signals) {
        this._recordSignal(signal);

        // Check if it's an actionable opportunity
        if (this._isOpportunity(signal)) {
          this.metrics.opportunitiesFound++;
          this.emit('opportunity', this._signalToOpportunity(signal));
        }
      }

    } catch (err) {
      this.metrics.errors++;
      log.error('Poll error', { error: err.message });
    }
  }

  /**
   * Detect signals from market data
   * Uses Jupiter Price API v2 + DexScreener
   * @private
   */
  async _detectSignals() {
    if (!this.config.useRealAPIs) {
      return this._detectSyntheticSignals();
    }

    const signals = [];

    try {
      // 1. Fetch prices from Jupiter
      const priceData = await this._fetchJupiterPrices();

      // 2. Detect price changes
      for (const [mint, data] of Object.entries(priceData)) {
        const prevPrice = this.priceCache.get(mint);

        if (prevPrice && data.price) {
          const changePercent = (data.price - prevPrice) / prevPrice;

          if (Math.abs(changePercent) >= this.config.priceChangeThreshold) {
            signals.push({
              type: changePercent > 0 ? SignalType.PRICE_SPIKE : SignalType.PRICE_DROP,
              token: data.symbol || mint.slice(0, 8),
              mint,
              data: {
                currentPrice: data.price,
                previousPrice: prevPrice,
                changePercent,
                confidence: data.confidence || 'medium',
              },
              timestamp: Date.now(),
            });
          }
        }

        // Update cache
        if (data.price) {
          this.priceCache.set(mint, data.price);
        }
      }

      // 3. Fetch DexScreener data for volume/liquidity
      const dexData = await this._fetchDexScreenerData();

      for (const pair of dexData) {
        const prevVolume = this.volumeCache.get(pair.pairAddress);

        if (prevVolume && pair.volume24h) {
          const volumeChange = (pair.volume24h - prevVolume) / prevVolume;

          if (volumeChange > 0.5) { // 50% volume surge
            signals.push({
              type: SignalType.VOLUME_SURGE,
              token: pair.baseToken?.symbol || 'Unknown',
              mint: pair.baseToken?.address,
              data: {
                currentVolume: pair.volume24h,
                previousVolume: prevVolume,
                changePercent: volumeChange,
                liquidity: pair.liquidity?.usd || 0,
                priceUsd: pair.priceUsd,
              },
              timestamp: Date.now(),
              pairAddress: pair.pairAddress,
            });
          }
        }

        // Update volume cache
        if (pair.volume24h) {
          this.volumeCache.set(pair.pairAddress, pair.volume24h);
        }

        // Detect liquidity changes
        if (pair.liquidity?.usd < this.config.liquidityThreshold) {
          signals.push({
            type: SignalType.LIQUIDITY_REMOVED,
            token: pair.baseToken?.symbol || 'Unknown',
            mint: pair.baseToken?.address,
            data: {
              liquidity: pair.liquidity?.usd || 0,
              threshold: this.config.liquidityThreshold,
            },
            timestamp: Date.now(),
            severity: 'warning',
          });
        }
      }

    } catch (err) {
      log.warn('Real API detection failed, falling back to synthetic', { error: err.message });
      return this._detectSyntheticSignals();
    }

    return signals;
  }

  /**
   * Fetch prices from Jupiter Price API v2
   * @private
   */
  async _fetchJupiterPrices() {
    const mints = this.config.watchedTokens.join(',');
    const url = `${JUPITER_PRICE_API}?ids=${mints}&showExtraInfo=true`;

    try {
      const response = await fetch(url, {
        headers: { 'Accept': 'application/json' },
        signal: AbortSignal.timeout(5000),
      });

      if (!response.ok) {
        throw new Error(`Jupiter API error: ${response.status}`);
      }

      const json = await response.json();
      return json.data || {};

    } catch (err) {
      log.debug('Jupiter API fetch failed', { error: err.message });
      return {};
    }
  }

  /**
   * Fetch data from DexScreener API
   * @private
   */
  async _fetchDexScreenerData() {
    // Query top Solana pairs
    const url = `${DEXSCREENER_API}/search?q=solana`;

    try {
      const response = await fetch(url, {
        headers: { 'Accept': 'application/json' },
        signal: AbortSignal.timeout(5000),
      });

      if (!response.ok) {
        throw new Error(`DexScreener API error: ${response.status}`);
      }

      const json = await response.json();

      // Filter for Solana pairs only
      const solanaPairs = (json.pairs || []).filter(p => p.chainId === 'solana');

      // Return top pairs by volume
      return solanaPairs
        .sort((a, b) => (b.volume24h || 0) - (a.volume24h || 0))
        .slice(0, 20);

    } catch (err) {
      log.debug('DexScreener API fetch failed', { error: err.message });
      return [];
    }
  }

  /**
   * Fallback synthetic signal detection (for testing/demo)
   * @private
   */
  async _detectSyntheticSignals() {
    const signals = [];

    try {
      const { value: slot } = await this.rpc.getSlot().send();

      // Simulate signal detection based on slot
      if (slot % 100 === 0) {
        signals.push({
          type: SignalType.VOLUME_SURGE,
          token: 'DemoToken',
          mint: 'Demo111111111111111111111111111111111111111',
          data: {
            currentVolume: Math.random() * 10000,
            previousVolume: Math.random() * 5000,
            changePercent: (Math.random() - 0.3) * 0.5,
          },
          timestamp: Date.now(),
          slot,
          synthetic: true,
        });
      }

    } catch (err) {
      log.debug('Synthetic signal detection error', { error: err.message });
    }

    return signals;
  }

  /**
   * Record a signal
   * @private
   */
  _recordSignal(signal) {
    signal.id = `sig_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;

    this.latestSignal = signal;
    this.signalHistory.push(signal);
    this.metrics.signalsDetected++;

    // Trim history
    while (this.signalHistory.length > this.maxHistory) {
      this.signalHistory.shift();
    }

    this.emit('signal', signal);

    // Emit to globalEventBus for collective visibility
    globalEventBus.emit(EventType.PATTERN_DETECTED, {
      id: signal.id,
      payload: {
        signalType: signal.type,
        token: signal.token,
        mint: signal.mint,
        source: signal.source || 'perceiver',
        confidence: signal.confidence || PHI_INV_2,
        timestamp: signal.timestamp,
      },
    });

    log.debug('Signal detected', { type: signal.type, token: signal.token });
  }

  /**
   * Ensure inference modules are loaded
   * @private
   */
  async _ensureInference() {
    if (this._inferenceReady) return;
    this._inferenceReady = true;

    try {
      const modules = await loadInferenceModules();
      if (!modules) return;

      this._rateTracker = new modules.EventRateTracker();
      this._inferenceModules = modules;
      log.debug('Inference modules wired for signal enrichment');
    } catch (e) {
      log.debug('Inference init failed', { error: e.message });
    }
  }

  /**
   * Check if signal is an actionable opportunity
   * Enriched with Poisson anomaly detection and Gaussian z-scores.
   * @private
   */
  _isOpportunity(signal) {
    if (!signal.data) return false;

    const { changePercent } = signal.data;
    if (changePercent === undefined) return false;

    // Must exceed threshold
    if (Math.abs(changePercent) < this.config.priceChangeThreshold) {
      return false;
    }

    // Enrich with inference statistics (non-blocking)
    this._enrichWithInference(signal);

    return true;
  }

  /**
   * Enrich signal with inference statistics (Poisson + Gaussian)
   * @private
   */
  _enrichWithInference(signal) {
    if (!this._inferenceModules) return;

    const { computeStats, zScore } = this._inferenceModules;
    const statistics = {};

    try {
      // Track signal rate for Poisson anomaly detection
      if (this._rateTracker) {
        this._rateTracker.record(signal.type || 'signal', 1);
        const anomalies = this._rateTracker.checkAllAnomalies?.();
        if (anomalies) {
          const typeAnomaly = anomalies[signal.type || 'signal'];
          statistics.isRateAnomaly = typeAnomaly?.isAnomaly || false;
          statistics.rateSeverity = typeAnomaly?.severity || 0;
        }
      }

      // Z-score on price change magnitude
      const magnitude = Math.abs(signal.data.changePercent || 0);
      this._priceHistory.push(magnitude);

      // Keep history bounded
      if (this._priceHistory.length > 200) {
        this._priceHistory = this._priceHistory.slice(-100);
      }

      if (this._priceHistory.length >= 5) {
        const stats = computeStats(this._priceHistory);
        statistics.zScore = zScore(magnitude, stats.mean, stats.std);
        statistics.sampleSize = stats.n;
        statistics.mean = stats.mean;
        statistics.std = stats.std;
      }
    } catch (e) {
      log.debug('Inference enrichment error', { error: e.message });
    }

    // Attach statistics to signal
    signal.statistics = statistics;
  }

  /**
   * Convert signal to opportunity format
   * @private
   */
  _signalToOpportunity(signal) {
    const { changePercent } = signal.data;

    return {
      id: `opp_${signal.id}`,
      signal,
      direction: changePercent > 0 ? 'LONG' : 'SHORT',
      magnitude: Math.abs(changePercent),
      token: signal.token,
      mint: signal.mint,
      timestamp: signal.timestamp,
      confidence: Math.min(Math.abs(changePercent) * 2, PHI_INV), // Cap at φ⁻¹
    };
  }

  /**
   * Get latest perception
   */
  async getLatest() {
    return this.latestSignal;
  }

  /**
   * Get signal history
   */
  getHistory(limit = 20) {
    return this.signalHistory.slice(-limit);
  }

  /**
   * Manually trigger signal check (for testing)
   */
  async check() {
    await this._poll();
    return this.latestSignal;
  }

  /**
   * Get status
   */
  getStatus() {
    return {
      isRunning: this.isRunning,
      metrics: { ...this.metrics },
      config: {
        pollInterval: this.config.pollInterval,
        watchedTokens: this.config.watchedTokens.length,
        useRealAPIs: this.config.useRealAPIs,
      },
      lastSignal: this.latestSignal ? {
        type: this.latestSignal.type,
        timestamp: this.latestSignal.timestamp,
      } : null,
      cacheSize: {
        prices: this.priceCache.size,
        volumes: this.volumeCache.size,
      },
    };
  }

  /**
   * Query price for a specific token
   * @param {string} mint - Token mint address
   * @returns {Promise<Object|null>} Price data
   */
  async queryTokenPrice(mint) {
    try {
      const url = `${JUPITER_PRICE_API}?ids=${mint}&showExtraInfo=true`;
      const response = await fetch(url, {
        headers: { 'Accept': 'application/json' },
        signal: AbortSignal.timeout(5000),
      });

      if (!response.ok) return null;

      const json = await response.json();
      return json.data?.[mint] || null;

    } catch (err) {
      log.debug('Token price query failed', { mint, error: err.message });
      return null;
    }
  }

  /**
   * Query DexScreener for a specific token
   * @param {string} mint - Token mint address
   * @returns {Promise<Array>} Pairs data
   */
  async queryTokenPairs(mint) {
    try {
      const url = `${DEXSCREENER_API}/tokens/${mint}`;
      const response = await fetch(url, {
        headers: { 'Accept': 'application/json' },
        signal: AbortSignal.timeout(5000),
      });

      if (!response.ok) return [];

      const json = await response.json();
      return json.pairs || [];

    } catch (err) {
      log.debug('Token pairs query failed', { mint, error: err.message });
      return [];
    }
  }

  /**
   * Add token to watch list
   * @param {string} mint - Token mint address
   */
  addWatchedToken(mint) {
    if (!this.config.watchedTokens.includes(mint)) {
      this.config.watchedTokens.push(mint);
      log.info('Added token to watch list', { mint });
    }
  }

  /**
   * Remove token from watch list
   * @param {string} mint - Token mint address
   */
  removeWatchedToken(mint) {
    const idx = this.config.watchedTokens.indexOf(mint);
    if (idx !== -1) {
      this.config.watchedTokens.splice(idx, 1);
      log.info('Removed token from watch list', { mint });
    }
  }
}

// Export known tokens for external use
export { KNOWN_TOKENS };

export default Perceiver;
