/**
 * CYNIC Solana Watcher - C2.1 (SOLANA × PERCEIVE)
 *
 * Watches Solana blockchain events and emits to EventBus.
 * Part of the 7×7 Fractal Matrix perception layer.
 *
 * "On-chain is truth" - cynic
 *
 * Perceives:
 * - Account changes (balances, data)
 * - Program state changes
 * - Slot progression
 * - Transaction logs
 * - Network health
 *
 * @module @cynic/node/perception/solana-watcher
 */

'use strict';

import { EventEmitter } from 'events';
import { Connection, PublicKey } from '@solana/web3.js';
import { createLogger, PHI_INV, PHI_INV_2, PHI_INV_3, globalEventBus, EventType as CoreEventType } from '@cynic/core';
import { getEventBus } from '../services/event-bus.js';

const log = createLogger('SolanaWatcher');

/**
 * Solana event types emitted to EventBus
 * @readonly
 * @enum {string}
 */
export const SolanaEventType = {
  ACCOUNT_CHANGE: 'perception:solana:account',
  PROGRAM_CHANGE: 'perception:solana:program',
  SLOT_CHANGE: 'perception:solana:slot',
  SIGNATURE: 'perception:solana:signature',
  LOG: 'perception:solana:log',
  ERROR: 'perception:solana:error',
  CONNECTED: 'perception:solana:connected',
  DISCONNECTED: 'perception:solana:disconnected',
};

/**
 * Default RPC endpoints by cluster
 */
const RPC_ENDPOINTS = {
  mainnet: 'https://api.mainnet-beta.solana.com',
  devnet: 'https://api.devnet.solana.com',
  testnet: 'https://api.testnet.solana.com',
  localhost: 'http://localhost:8899',
};

/**
 * φ-aligned health thresholds
 */
const HEALTH_THRESHOLDS = {
  // Slot lag acceptable limits (in slots)
  slotLagExcellent: 5,           // < 5 slots behind
  slotLagAcceptable: 13,         // < 13 slots behind (φ⁻¹ × 21)
  slotLagDegraded: 34,           // < 34 slots behind (Fib)

  // Event rate limits
  minEventsPerMinute: 1,         // Minimum to be "active"
  healthyEventsPerMinute: 10,    // Good activity level

  // Error rate thresholds (as percentage)
  errorRateExcellent: PHI_INV_3, // < 23.6%
  errorRateAcceptable: PHI_INV_2, // < 38.2%
  errorRateCritical: PHI_INV,    // > 61.8% is critical
};

/**
 * SolanaWatcher - Watches Solana blockchain and emits to EventBus
 *
 * Implements the perception layer for on-chain events (C2.1).
 * Uses @solana/web3.js WebSocket subscriptions.
 */
export class SolanaWatcher extends EventEmitter {
  /**
   * Create a new SolanaWatcher
   *
   * @param {Object} [options] - Configuration options
   * @param {string} [options.rpcUrl] - Solana RPC URL
   * @param {string} [options.cluster='mainnet'] - Cluster name
   * @param {EventBus} [options.eventBus] - EventBus instance
   * @param {string} [options.commitment='confirmed'] - Commitment level
   */
  constructor(options = {}) {
    super();

    const cluster = options.cluster || 'mainnet';
    this.rpcUrl = options.rpcUrl || process.env.SOLANA_RPC_URL || RPC_ENDPOINTS[cluster];
    this.eventBus = options.eventBus || getEventBus();
    this.commitment = options.commitment || 'confirmed';
    this._cluster = cluster;

    this.connection = null;
    this._subscriptions = new Map(); // id -> { type, pubkey?, programId? }
    this._isRunning = false;

    // Enhanced stats for perception layer
    this._stats = {
      eventsEmitted: 0,
      accountsWatched: 0,
      programsWatched: 0,
      errors: 0,
      startedAt: null,
      lastEventAt: null,
      lastSlot: null,
      slotHistory: [], // Last 10 slot updates for health tracking
    };

    // Event rate tracking (per-minute window)
    this._eventWindow = [];
    this._windowSize = 60000; // 1 minute
  }

  /**
   * Start the Solana watcher
   *
   * @returns {Promise<SolanaWatcher>} this (for chaining)
   */
  async start() {
    if (this._isRunning) {
      log.warn('SolanaWatcher already running');
      return this;
    }

    log.info('Starting Solana watcher', { rpcUrl: this._maskUrl(this.rpcUrl) });

    try {
      this.connection = new Connection(this.rpcUrl, {
        commitment: this.commitment,
        wsEndpoint: this._getWsEndpoint(this.rpcUrl),
      });

      // Test connection
      const slot = await this.connection.getSlot();
      log.info('Connected to Solana', { slot });

      this._isRunning = true;
      this._stats.startedAt = Date.now();

      this.eventBus.publish(SolanaEventType.CONNECTED, {
        rpcUrl: this._maskUrl(this.rpcUrl),
        slot,
        timestamp: Date.now(),
      }, {
        source: 'SolanaWatcher',
      });

      return this;
    } catch (error) {
      log.error('Failed to connect to Solana', { error: error.message });
      this._emitError(error);
      throw error;
    }
  }

  /**
   * Stop the Solana watcher
   *
   * @returns {Promise<void>}
   */
  async stop() {
    if (!this._isRunning) {
      return;
    }

    log.info('Stopping Solana watcher');

    // Remove all subscriptions
    for (const [subId, sub] of this._subscriptions.entries()) {
      try {
        if (sub.type === 'account') {
          await this.connection.removeAccountChangeListener(subId);
        } else if (sub.type === 'program') {
          await this.connection.removeProgramAccountChangeListener(subId);
        } else if (sub.type === 'slot') {
          await this.connection.removeSlotChangeListener(subId);
        } else if (sub.type === 'logs') {
          await this.connection.removeOnLogsListener(subId);
        }
      } catch (error) {
        log.warn('Error removing subscription', { subId, error: error.message });
      }
    }

    this._subscriptions.clear();
    this._isRunning = false;

    this.eventBus.publish(SolanaEventType.DISCONNECTED, {
      timestamp: Date.now(),
    }, {
      source: 'SolanaWatcher',
    });
  }

  /**
   * Watch an account for changes
   *
   * @param {PublicKey|string} pubkey - Account public key
   * @returns {Promise<number>} Subscription ID
   */
  async watchAccount(pubkey) {
    this._ensureRunning();

    const pk = typeof pubkey === 'string' ? new PublicKey(pubkey) : pubkey;

    const subId = this.connection.onAccountChange(
      pk,
      (accountInfo, context) => {
        this._emitAccountChange(pk, accountInfo, context);
      },
      this.commitment,
    );

    this._subscriptions.set(subId, { type: 'account', pubkey: pk.toString() });
    this._stats.accountsWatched++;

    log.info('Watching account', { pubkey: pk.toString() });

    return subId;
  }

  /**
   * Watch a program for account changes
   *
   * @param {PublicKey|string} programId - Program public key
   * @param {Object} [filters] - Optional filters
   * @returns {Promise<number>} Subscription ID
   */
  async watchProgram(programId, filters = []) {
    this._ensureRunning();

    const pk = typeof programId === 'string' ? new PublicKey(programId) : programId;

    const subId = this.connection.onProgramAccountChange(
      pk,
      (keyedAccountInfo, context) => {
        this._emitProgramChange(pk, keyedAccountInfo, context);
      },
      this.commitment,
      filters,
    );

    this._subscriptions.set(subId, { type: 'program', programId: pk.toString() });
    this._stats.programsWatched++;

    log.info('Watching program', { programId: pk.toString() });

    return subId;
  }

  /**
   * Watch for slot changes
   *
   * @returns {Promise<number>} Subscription ID
   */
  async watchSlots() {
    this._ensureRunning();

    const subId = this.connection.onSlotChange((slotInfo) => {
      this._emitSlotChange(slotInfo);
    });

    this._subscriptions.set(subId, { type: 'slot' });

    log.info('Watching slot changes');

    return subId;
  }

  /**
   * Watch logs from a program or all
   *
   * @param {PublicKey|string|'all'|'allWithVotes'} filter - Program ID or 'all'
   * @returns {Promise<number>} Subscription ID
   */
  async watchLogs(filter = 'all') {
    this._ensureRunning();

    let logsFilter;
    if (filter === 'all' || filter === 'allWithVotes') {
      logsFilter = filter;
    } else {
      const pk = typeof filter === 'string' ? new PublicKey(filter) : filter;
      logsFilter = { mentions: [pk.toString()] };
    }

    const subId = this.connection.onLogs(
      logsFilter,
      (logs, context) => {
        this._emitLogs(logs, context);
      },
      this.commitment,
    );

    this._subscriptions.set(subId, { type: 'logs', filter: logsFilter });

    log.info('Watching logs', { filter: typeof filter === 'string' ? filter : filter.toString() });

    return subId;
  }

  /**
   * Wait for a transaction signature confirmation
   *
   * @param {string} signature - Transaction signature
   * @param {number} [timeout=30000] - Timeout in ms
   * @returns {Promise<Object>} Confirmation result
   */
  async waitForSignature(signature, timeout = 30000) {
    this._ensureRunning();

    try {
      const result = await this.connection.confirmTransaction(signature, this.commitment);

      this.eventBus.publish(SolanaEventType.SIGNATURE, {
        signature,
        confirmed: !result.value?.err,
        error: result.value?.err,
        slot: result.context?.slot,
        timestamp: Date.now(),
      }, {
        source: 'SolanaWatcher',
      });

      this._stats.eventsEmitted++;

      return result;
    } catch (error) {
      this._emitError(error);
      throw error;
    }
  }

  /**
   * Remove a subscription
   *
   * @param {number} subId - Subscription ID
   * @returns {Promise<void>}
   */
  async unwatch(subId) {
    const sub = this._subscriptions.get(subId);
    if (!sub) {
      return;
    }

    try {
      if (sub.type === 'account') {
        await this.connection.removeAccountChangeListener(subId);
        this._stats.accountsWatched--;
      } else if (sub.type === 'program') {
        await this.connection.removeProgramAccountChangeListener(subId);
        this._stats.programsWatched--;
      } else if (sub.type === 'slot') {
        await this.connection.removeSlotChangeListener(subId);
      } else if (sub.type === 'logs') {
        await this.connection.removeOnLogsListener(subId);
      }

      this._subscriptions.delete(subId);
      log.debug('Removed subscription', { subId, type: sub.type });
    } catch (error) {
      log.warn('Error removing subscription', { subId, error: error.message });
    }
  }

  /**
   * Check if watcher is running
   *
   * @returns {boolean}
   */
  isRunning() {
    return this._isRunning;
  }

  /**
   * Get watcher statistics
   *
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      ...this._stats,
      isRunning: this._isRunning,
      subscriptionCount: this._subscriptions.size,
      uptime: this._stats.startedAt ? Date.now() - this._stats.startedAt : 0,
    };
  }

  /**
   * Get active subscriptions
   *
   * @returns {Array} Subscription info
   */
  getSubscriptions() {
    return Array.from(this._subscriptions.entries()).map(([id, sub]) => ({
      id,
      ...sub,
    }));
  }

  /**
   * Emit account change event
   *
   * @private
   */
  _emitAccountChange(pubkey, accountInfo, context) {
    const event = {
      pubkey: pubkey.toString(),
      slot: context.slot,
      lamports: accountInfo.lamports,
      owner: accountInfo.owner.toString(),
      dataLength: accountInfo.data.length,
      executable: accountInfo.executable,
      rentEpoch: accountInfo.rentEpoch,
      timestamp: Date.now(),
    };

    this.eventBus.publish(SolanaEventType.ACCOUNT_CHANGE, event, {
      source: 'SolanaWatcher',
    });

    // Emit to local EventEmitter
    this.emit('account_change', event);

    this._stats.eventsEmitted++;
    this._recordEvent();
    log.debug('Account change', { pubkey: pubkey.toString(), slot: context.slot });
  }

  /**
   * Emit program account change event
   *
   * @private
   */
  _emitProgramChange(programId, keyedAccountInfo, context) {
    const event = {
      programId: programId.toString(),
      pubkey: keyedAccountInfo.accountId.toString(),
      slot: context.slot,
      lamports: keyedAccountInfo.accountInfo.lamports,
      dataLength: keyedAccountInfo.accountInfo.data.length,
      timestamp: Date.now(),
    };

    this.eventBus.publish(SolanaEventType.PROGRAM_CHANGE, event, {
      source: 'SolanaWatcher',
    });

    // Emit to local EventEmitter
    this.emit('program_change', event);

    this._stats.eventsEmitted++;
    this._recordEvent();
    log.debug('Program change', { programId: programId.toString(), slot: context.slot });
  }

  /**
   * Emit slot change event
   *
   * @private
   */
  _emitSlotChange(slotInfo) {
    const event = {
      slot: slotInfo.slot,
      parent: slotInfo.parent,
      root: slotInfo.root,
      timestamp: Date.now(),
    };

    this.eventBus.publish(SolanaEventType.SLOT_CHANGE, event, {
      source: 'SolanaWatcher',
    });

    // Emit to local EventEmitter
    this.emit('slot_change', event);

    // Track slot for health monitoring
    this._stats.lastSlot = slotInfo.slot;
    this._stats.slotHistory.push({ slot: slotInfo.slot, timestamp: event.timestamp });
    while (this._stats.slotHistory.length > 10) {
      this._stats.slotHistory.shift();
    }

    this._stats.eventsEmitted++;
    this._recordEvent();
  }

  /**
   * Emit logs event
   *
   * @private
   */
  _emitLogs(logs, context) {
    const event = {
      signature: logs.signature,
      slot: context.slot,
      logs: logs.logs,
      error: logs.err,
      timestamp: Date.now(),
    };

    this.eventBus.publish(SolanaEventType.LOG, event, {
      source: 'SolanaWatcher',
    });

    // Emit to local EventEmitter
    this.emit('log', event);

    this._stats.eventsEmitted++;
    this._recordEvent();
    log.debug('Log event', { signature: logs.signature?.slice(0, 12) });
  }

  /**
   * Emit error event
   *
   * @private
   */
  _emitError(error) {
    log.error('Solana watcher error', { error: error.message });

    this.eventBus.publish(SolanaEventType.ERROR, {
      error: error.message,
      stack: error.stack,
      timestamp: Date.now(),
    }, {
      source: 'SolanaWatcher',
    });

    this._stats.errors++;
  }

  /**
   * Ensure watcher is running
   *
   * @private
   */
  _ensureRunning() {
    if (!this._isRunning || !this.connection) {
      throw new Error('SolanaWatcher not running. Call start() first.');
    }
  }

  /**
   * Get WebSocket endpoint from HTTP URL
   *
   * @private
   */
  _getWsEndpoint(httpUrl) {
    return httpUrl
      .replace('https://', 'wss://')
      .replace('http://', 'ws://');
  }

  /**
   * Mask URL for logging (hide API keys)
   *
   * @private
   */
  _maskUrl(url) {
    try {
      const u = new URL(url);
      if (u.password) {
        u.password = '***';
      }
      // Mask common API key patterns
      if (u.pathname.includes('/v1/')) {
        const parts = u.pathname.split('/');
        if (parts[parts.length - 1].length > 20) {
          parts[parts.length - 1] = '***';
          u.pathname = parts.join('/');
        }
      }
      return u.toString();
    } catch {
      return url.substring(0, 30) + '...';
    }
  }

  /**
   * Get perception health assessment
   *
   * @returns {Object} Health assessment with φ-aligned status
   */
  getHealth() {
    const now = Date.now();

    // Calculate event rate (events per minute)
    this._pruneEventWindow(now);
    const eventsPerMinute = this._eventWindow.length;

    // Calculate error rate
    const totalOperations = this._stats.eventsEmitted + this._stats.errors;
    const errorRate = totalOperations > 0
      ? this._stats.errors / totalOperations
      : 0;

    // Calculate uptime
    const uptime = this._stats.startedAt
      ? now - this._stats.startedAt
      : 0;

    // Calculate time since last event
    const timeSinceLastEvent = this._stats.lastEventAt
      ? now - this._stats.lastEventAt
      : null;

    // Determine overall health status
    let status = 'healthy';
    let score = 1.0;

    if (!this._isRunning) {
      status = 'offline';
      score = 0;
    } else if (errorRate > HEALTH_THRESHOLDS.errorRateCritical) {
      status = 'critical';
      score = PHI_INV_3; // 23.6%
    } else if (errorRate > HEALTH_THRESHOLDS.errorRateAcceptable) {
      status = 'degraded';
      score = PHI_INV_2; // 38.2%
    } else if (eventsPerMinute < HEALTH_THRESHOLDS.minEventsPerMinute && this._subscriptions.size > 0) {
      status = 'stale';
      score = PHI_INV_2; // 38.2%
    } else if (errorRate < HEALTH_THRESHOLDS.errorRateExcellent && eventsPerMinute >= HEALTH_THRESHOLDS.healthyEventsPerMinute) {
      status = 'excellent';
      score = PHI_INV; // 61.8% (φ⁻¹ max)
    }

    return {
      status,
      score,
      cluster: this._cluster,
      isRunning: this._isRunning,
      uptime,
      uptimeMinutes: Math.round(uptime / 60000),
      eventsPerMinute,
      errorRate,
      totalEvents: this._stats.eventsEmitted,
      totalErrors: this._stats.errors,
      subscriptions: this._subscriptions.size,
      accountsWatched: this._stats.accountsWatched,
      programsWatched: this._stats.programsWatched,
      lastSlot: this._stats.lastSlot,
      timeSinceLastEvent,
      thresholds: HEALTH_THRESHOLDS,
    };
  }

  /**
   * Get current perception state for emission
   *
   * @returns {Object} Current perception state
   */
  getPerceptionState() {
    const health = this.getHealth();

    return {
      dimension: 'SOLANA',
      analysis: 'PERCEIVE',
      cell: 'C2.1',
      health,
      subscriptions: this.getSubscriptions(),
      lastSlot: this._stats.lastSlot,
      timestamp: Date.now(),
    };
  }

  /**
   * Emit perception state to globalEventBus
   * Call this periodically or on significant changes
   */
  emitPerceptionState() {
    const state = this.getPerceptionState();

    // Emit to local EventEmitter
    this.emit('perception_state', state);

    // Emit to globalEventBus
    globalEventBus.emit('perception:solana:state', state);

    return state;
  }

  /**
   * Prune old events from rate window
   * @private
   */
  _pruneEventWindow(now = Date.now()) {
    const cutoff = now - this._windowSize;
    while (this._eventWindow.length > 0 && this._eventWindow[0] < cutoff) {
      this._eventWindow.shift();
    }
  }

  /**
   * Record event timestamp for rate tracking
   * @private
   */
  _recordEvent() {
    const now = Date.now();
    this._eventWindow.push(now);
    this._stats.lastEventAt = now;
  }
}

/**
 * Create a new SolanaWatcher
 *
 * @param {Object} [options] - Options
 * @returns {SolanaWatcher}
 */
export function createSolanaWatcher(options) {
  return new SolanaWatcher(options);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create SolanaWatcher singleton
 *
 * @param {Object} [options] - Options (only used on first call)
 * @returns {SolanaWatcher}
 */
export function getSolanaWatcher(options = {}) {
  if (!_instance) {
    _instance = new SolanaWatcher(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetSolanaWatcher() {
  if (_instance) {
    _instance.stop().catch(() => {});
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  SolanaWatcher,
  SolanaEventType,
  createSolanaWatcher,
  getSolanaWatcher,
  resetSolanaWatcher,
};
