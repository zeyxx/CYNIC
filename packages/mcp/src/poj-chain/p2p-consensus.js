/**
 * PoJ P2P Consensus Manager
 *
 * Handles distributed finality for PoJ blocks via P2P network.
 * Implements φ-BFT consensus (61.8% supermajority).
 *
 * "Don't trust, verify" - κυνικός
 *
 * @module @cynic/mcp/poj-chain/p2p-consensus
 */

'use strict';

import { createLogger, globalEventBus, PHI_INV } from '@cynic/core';

const log = createLogger('PoJP2PConsensus');

// P0.2: Finality timeout for P2P consensus (φ-scaled: 32 confirmations × 400ms slot)
const FINALITY_TIMEOUT_MS = 32 * 400 * PHI_INV; // ~8 seconds

/**
 * Manages P2P consensus for PoJ blocks
 */
export class P2PConsensus {
  /**
   * @param {Object} [options] - Configuration
   * @param {string} [options.nodeUrl] - P2P node URL
   * @param {boolean} [options.enabled=false] - Enable P2P consensus
   * @param {Function} [options.onFinalized] - Callback when block finalized
   */
  constructor(options = {}) {
    this._nodeUrl = options.nodeUrl || null;
    this._enabled = options.enabled ?? false;
    this._onFinalized = options.onFinalized || null;

    // Track blocks pending consensus finality
    this._pendingFinality = new Map(); // blockHash → { block, timeout, resolve, reject }
    this._finalitySubscription = null;

    // Stats
    this._stats = {
      blocksFinalized: 0,
      finalityTimeouts: 0,
      proposalsSent: 0,
      proposalsFailed: 0,
    };
  }

  /**
   * Check if P2P consensus is enabled
   */
  get isEnabled() {
    return this._enabled && this._nodeUrl !== null;
  }

  /**
   * Get stats
   */
  get stats() {
    return { ...this._stats };
  }

  /**
   * Get pending finality count
   */
  get pendingCount() {
    return this._pendingFinality.size;
  }

  /**
   * Initialize P2P consensus (subscribe to finality events)
   */
  initialize() {
    if (!this._enabled) return;

    this._finalitySubscription = globalEventBus.subscribe('poj:block:finalized', (event) => {
      this._onBlockFinalized(event.payload || event);
    });
    log.info('P2P finality subscription active');
  }

  /**
   * Set P2P node URL
   * @param {string} url - P2P node URL
   */
  setNodeUrl(url) {
    this._nodeUrl = url;
    this._enabled = true;
    log.info('P2P consensus enabled', { url });
  }

  /**
   * Propose block to P2P network for distributed consensus
   * @param {Object} block - Block to propose
   * @returns {Promise<Object|null>} Proposal result
   */
  async propose(block) {
    if (!this._nodeUrl || !this._enabled) return null;

    try {
      const response = await fetch(`${this._nodeUrl}/propose`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: 'POJ_BLOCK',
          slot: block.slot,
          hash: block.hash,
          judgments_root: block.judgments_root,
          judgment_count: block.judgments?.length || 0,
          prev_hash: block.prev_hash,
          timestamp: block.timestamp,
        }),
      });

      if (!response.ok) {
        throw new Error(`P2P propose failed: ${response.status}`);
      }

      const result = await response.json();
      this._stats.proposalsSent++;
      log.info('PoJ block proposed to P2P', {
        slot: block.slot,
        p2pSlot: result.slot,
        finalized: result.finalizedSlot >= result.slot,
      });

      return result;
    } catch (err) {
      this._stats.proposalsFailed++;
      log.error('P2P propose error', { slot: block.slot, error: err.message });
      return null;
    }
  }

  /**
   * Wait for block finality through P2P consensus
   * @param {Object} block - Block awaiting finality
   * @param {number} [timeoutMs] - Timeout in ms
   * @returns {Promise<{finalized: boolean, slot: number, fallback?: boolean}>}
   */
  waitForFinality(block, timeoutMs = FINALITY_TIMEOUT_MS) {
    return new Promise((resolve, reject) => {
      const blockHash = block.hash;

      // Set timeout
      const timeout = setTimeout(() => {
        const pending = this._pendingFinality.get(blockHash);
        if (pending) {
          this._pendingFinality.delete(blockHash);
          this._stats.finalityTimeouts++;

          log.warn('Finality timeout - using fallback', {
            slot: block.slot,
            timeoutMs,
          });

          resolve({ finalized: false, slot: block.slot, fallback: true });
        }
      }, timeoutMs);

      // Store pending
      this._pendingFinality.set(blockHash, {
        block,
        timeout,
        resolve,
        reject,
        createdAt: Date.now(),
      });
    });
  }

  /**
   * Handle block finalized event from consensus layer
   * @param {Object} event - Finality event
   * @private
   */
  _onBlockFinalized(event) {
    const { blockHash, slot, status, confirmations } = event;

    // Check if we're waiting for this block's finality
    const pending = this._pendingFinality.get(blockHash);
    if (!pending) {
      log.debug('Finality event for unknown/processed block', { slot, blockHash: blockHash?.slice(0, 16) });
      return;
    }

    // Clear timeout
    if (pending.timeout) {
      clearTimeout(pending.timeout);
    }

    log.info('PoJ block finalized through consensus', {
      slot,
      confirmations,
      status,
    });

    this._stats.blocksFinalized++;

    // Callback if provided
    if (this._onFinalized) {
      try {
        this._onFinalized(pending.block);
      } catch (e) {
        log.warn('Finalized callback error', { error: e.message });
      }
    }

    // Resolve the promise
    if (pending.resolve) {
      pending.resolve({ finalized: true, slot, confirmations });
    }

    // Remove from pending
    this._pendingFinality.delete(blockHash);
  }

  /**
   * Get status
   * @returns {Object} P2P consensus status
   */
  getStatus() {
    return {
      enabled: this._enabled,
      nodeUrl: this._nodeUrl,
      pendingFinality: this._pendingFinality.size,
      stats: this._stats,
    };
  }

  /**
   * Close and cleanup
   */
  close() {
    // Cleanup finality subscription
    if (this._finalitySubscription) {
      try {
        if (typeof this._finalitySubscription === 'function') {
          this._finalitySubscription();
        }
      } catch (e) { /* ignore */ }
      this._finalitySubscription = null;
    }

    // Clear pending finality timeouts
    for (const [hash, pending] of this._pendingFinality.entries()) {
      if (pending.timeout) {
        clearTimeout(pending.timeout);
      }
      if (pending.reject) {
        pending.reject(new Error('P2PConsensus closing'));
      }
    }
    this._pendingFinality.clear();
  }
}

export default P2PConsensus;
