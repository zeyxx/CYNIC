/**
 * SolanaAnchoringManager - Onchain truth anchoring
 *
 * Extracted from CYNICNetworkNode monolith (BURN)
 *
 * Anchors finalized blocks to Solana, verifies anchors,
 * manages pending/anchored/failed states.
 *
 * "Onchain is truth" - κυνικός
 *
 * @module @cynic/node/network/solana-anchoring
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('SolanaAnchoring');

export class SolanaAnchoringManager extends EventEmitter {
  /**
   * @param {Object} [options]
   * @param {boolean} [options.enabled=false]
   * @param {string} [options.cluster='devnet']
   * @param {Object} [options.wallet=null]
   * @param {boolean} [options.dryRun=false]
   * @param {number} [options.anchorInterval=500]
   * @param {string} [options.selfPublicKey=null] - Node's public key (only proposer anchors)
   */
  constructor(options = {}) {
    super();

    this._enabled = options.enabled ?? false;
    this._cluster = options.cluster || 'devnet';
    this._wallet = options.wallet || null;
    this._dryRun = options.dryRun ?? false;
    this._selfPublicKey = options.selfPublicKey || null;
    this._anchorer = null;
    this._pendingAnchors = new Map();  // blockHash -> { slot, merkleRoot, status, ... }
    this._lastAnchorSlot = 0;
    this._anchorInterval = options.anchorInterval || 500;

    this._stats = {
      blocksAnchored: 0,
      anchorsFailed: 0,
      anchorsRetried: 0,
      lastAnchorSignature: null,
      lastAnchorTimestamp: null,
    };

    // Retry infrastructure
    this._blockStore = null;
    this._retryInterval = null;
  }

  // Fibonacci backoff delays in ms: F(6)-F(10) = [8, 13, 21, 34, 55] seconds
  static FIBONACCI_DELAYS = [8000, 13000, 21000, 34000, 55000];
  static RETRY_SWEEP_MS = 21000; // F(8) seconds
  static MAX_RETRY_COUNT = 8;

  /**
   * Enable Solana anchoring
   * @param {Object} [options]
   * @param {string} [options.cluster]
   * @param {Object} [options.wallet]
   * @param {number} [options.interval]
   * @param {boolean} [options.dryRun]
   */
  async enable(options = {}) {
    this._enabled = true;
    if (options.cluster) this._cluster = options.cluster;
    if (options.wallet) this._wallet = options.wallet;
    if (options.interval) this._anchorInterval = options.interval;
    if (options.dryRun !== undefined) this._dryRun = options.dryRun;
    if (options.blockStore) this._blockStore = options.blockStore;

    if (!this._anchorer) {
      try {
        const { SolanaAnchorer } = await import('@cynic/anchor');
        this._anchorer = new SolanaAnchorer({
          cluster: this._cluster,
          wallet: this._dryRun ? null : this._wallet,
          useAnchorProgram: true,
          onAnchor: (record) => log.info('Root anchored', { sig: record.signature?.slice(0, 32) }),
          onError: (record, err) => log.warn('Anchor failed', { error: err.message }),
        });
      } catch (error) {
        log.warn('SolanaAnchorer init failed, using simulation', { error: error.message });
      }
    }

    log.info('Solana anchoring enabled', {
      cluster: this._cluster,
      interval: this._anchorInterval,
      hasWallet: !!this._wallet,
      hasAnchorer: !!this._anchorer,
      dryRun: this._dryRun,
    });

    this.emit('anchoring:enabled', {
      cluster: this._cluster,
      interval: this._anchorInterval,
    });

    // Start retry timer for failed anchors
    this._startRetryTimer();
  }

  /** Disable anchoring */
  disable() {
    this._enabled = false;
    this._stopRetryTimer();
    log.info('Solana anchoring disabled');
    this.emit('anchoring:disabled');
  }

  /**
   * Anchor a finalized block to Solana
   * @param {Object} block - { slot, hash, merkleRoot, ... }
   * @returns {Promise<Object|null>} Anchor result or null
   */
  async anchorBlock(block, retryCount = 0) {
    if (!this._enabled) return null;

    if (!this._anchorer && !this._wallet) {
      log.warn('Cannot anchor - no wallet or anchorer configured');
      return null;
    }

    const { slot, hash } = block;
    // Resolve merkle root early for logging (handles snake_case/camelCase)
    const resolvedRoot = this.resolveMerkleRoot(block);

    // Preserve existing retryCount if re-anchoring
    const existing = this._pendingAnchors.get(hash);
    const currentRetryCount = retryCount || existing?.retryCount || 0;

    this._pendingAnchors.set(hash, {
      slot,
      merkleRoot: resolvedRoot,
      status: 'pending',
      retryCount: currentRetryCount,
      queuedAt: existing?.queuedAt || Date.now(),
    });

    log.info('Anchoring block to Solana', {
      slot,
      hash: hash?.slice(0, 16),
      merkleRoot: resolvedRoot?.slice(0, 16),
      cluster: this._cluster,
    });

    try {
      const result = await this._createAnchorTransaction(block);

      if (result.success) {
        this._pendingAnchors.set(hash, {
          slot,
          merkleRoot: resolvedRoot,
          status: 'anchored',
          signature: result.signature,
          anchoredAt: Date.now(),
        });

        this._lastAnchorSlot = slot;
        this._stats.blocksAnchored++;
        this._stats.lastAnchorSignature = result.signature;
        this._stats.lastAnchorTimestamp = Date.now();

        log.info('Block anchored to Solana', {
          slot,
          signature: result.signature?.slice(0, 32),
        });

        this.emit('block:anchored', {
          slot, hash, merkleRoot: resolvedRoot,
          signature: result.signature,
          cluster: this._cluster,
        });

        globalEventBus.publish(EventType.BLOCK_ANCHORED, {
          slot, hash, merkleRoot: resolvedRoot,
          signature: result.signature,
          cluster: this._cluster,
          timestamp: Date.now(),
        });

        return result;
      } else {
        throw new Error(result.error || 'Anchor failed');
      }
    } catch (error) {
      const nextRetryCount = currentRetryCount + 1;

      this._pendingAnchors.set(hash, {
        slot,
        merkleRoot: resolvedRoot,
        status: 'failed',
        retryCount: nextRetryCount,
        error: error.message,
        failedAt: Date.now(),
      });

      this._stats.anchorsFailed++;

      log.error('Failed to anchor block', { slot, retryCount: nextRetryCount, error: error.message });

      const failureData = { slot, hash, error: error.message, retryCount: nextRetryCount };
      this.emit('anchor:failed', failureData);

      // Publish to globalEventBus for event-listeners persistence
      globalEventBus.publish('anchor:failed', {
        ...failureData,
        timestamp: Date.now(),
      });

      return { success: false, error: error.message };
    }
  }

  /**
   * Resolve a valid 64-char hex merkle root from a block
   * @param {Object} block
   * @returns {string|null}
   */
  resolveMerkleRoot(block) {
    const ZERO_ROOT = '0'.repeat(64);
    // Prefer real merkle root (non-zero = block has judgments)
    for (const key of ['judgments_root', 'judgmentsRoot', 'merkle_root', 'merkleRoot']) {
      const val = block[key];
      if (val && /^[a-f0-9]{64}$/i.test(val) && val !== ZERO_ROOT) return val;
    }
    // Fall back to block hash (always unique per slot)
    const hash = block.hash || block.block_hash;
    if (hash && /^[a-f0-9]{64}$/i.test(hash)) return hash;
    return null;
  }

  /**
   * Create anchor transaction.
   *
   * FIDELITY: When the anchorer exists (real wallet configured) but fails,
   * we return failure — NOT a simulated success. Simulation is only for
   * dev/test mode when no anchorer is available at all.
   *
   * @private
   */
  async _createAnchorTransaction(block) {
    const merkleRoot = this.resolveMerkleRoot(block);
    if (!merkleRoot) {
      return { success: false, error: 'No valid merkle root for block' };
    }

    if (this._anchorer) {
      try {
        const result = await this._anchorer.anchor(merkleRoot, []);
        if (result.success) {
          log.debug('Anchor TX path', {
            simulated: !!result.simulated,
            hasWallet: !!this._anchorer.wallet,
            walletPub: this._anchorer.wallet?.publicKey?.slice?.(0, 16) || 'none',
          });
          return {
            success: true,
            signature: result.signature,
            slot: result.slot || block.slot,
            merkleRoot,
            cluster: this._cluster,
            timestamp: result.timestamp || Date.now(),
            simulated: result.simulated || this._dryRun,
          };
        }
        // Real failure — do NOT simulate. "Don't trust, verify" means don't lie.
        log.warn('Anchor failed (no simulation fallback)', { error: result.error });
        return { success: false, error: result.error || 'Anchor program returned failure' };
      } catch (error) {
        log.warn('Anchor error (no simulation fallback)', { error: error.message });
        return { success: false, error: error.message };
      }
    }

    // No anchorer available → simulation mode (dev/test only)
    return {
      success: true,
      signature: `sim_${merkleRoot.slice(0, 16)}_${Date.now()}`,
      slot: block.slot,
      merkleRoot,
      cluster: this._cluster,
      timestamp: Date.now(),
      simulated: true,
    };
  }

  /**
   * Check if a block should be anchored (based on interval)
   * @param {number} slot
   * @returns {boolean}
   */
  shouldAnchor(slot) {
    if (!this._enabled) return false;
    return slot % this._anchorInterval === 0;
  }

  /**
   * Called when a block is finalized.
   * Only the block PROPOSER anchors — prevents N nodes × same wallet × same slot.
   *
   * @param {Object} block
   * @param {Function} recordBlockHash - ForkDetector's recordBlockHash
   */
  async onBlockFinalized(block, recordBlockHash) {
    recordBlockHash?.(block.slot, block.hash);

    if (!this.shouldAnchor(block.slot)) return;

    // Only the proposer anchors their own blocks.
    // If proposer is unknown (undefined/null), anchor anyway (backwards compat).
    if (block.proposer && this._selfPublicKey && block.proposer !== this._selfPublicKey) {
      log.debug('Skipping anchor — not the proposer', {
        slot: block.slot,
        proposer: block.proposer?.slice(0, 16),
        self: this._selfPublicKey?.slice(0, 16),
      });
      return;
    }

    await this.anchorBlock(block);
  }

  /**
   * Get anchor status for a block hash
   * @param {string} hash
   * @returns {Object|null}
   */
  getAnchorStatus(hash) {
    return this._pendingAnchors.get(hash) || null;
  }

  /**
   * Get full anchoring status
   * @returns {Object}
   */
  getAnchoringStatus() {
    const pending = Array.from(this._pendingAnchors.values()).filter(a => a.status === 'pending');
    const anchored = Array.from(this._pendingAnchors.values()).filter(a => a.status === 'anchored');
    const failed = Array.from(this._pendingAnchors.values()).filter(a => a.status === 'failed');

    return {
      enabled: this._enabled,
      cluster: this._cluster,
      hasWallet: !!this._wallet,
      dryRun: this._dryRun,
      hasAnchorer: !!this._anchorer,
      anchorInterval: this._anchorInterval,
      lastAnchorSlot: this._lastAnchorSlot,
      pending: pending.length,
      anchored: anchored.length,
      failed: failed.length,
      anchorerStats: this._anchorer?.getStats?.() || null,
      stats: {
        blocksAnchored: this._stats.blocksAnchored,
        anchorsFailed: this._stats.anchorsFailed,
        lastSignature: this._stats.lastAnchorSignature?.slice(0, 32),
        lastTimestamp: this._stats.lastAnchorTimestamp,
      },
    };
  }

  /**
   * Verify an anchor on Solana
   * @param {string} signatureOrMerkleRoot
   * @returns {Promise<Object>}
   */
  async verifyAnchor(signatureOrMerkleRoot) {
    // Check local cache first
    for (const [hash, anchor] of this._pendingAnchors) {
      if (anchor.signature === signatureOrMerkleRoot) {
        return {
          verified: true,
          slot: anchor.slot,
          hash,
          merkleRoot: anchor.merkleRoot,
          anchoredAt: anchor.anchoredAt,
          source: 'cache',
        };
      }
    }

    // Fallback to on-chain verification
    if (this._anchorer) {
      try {
        const result = await this._anchorer.verifyAnchor(signatureOrMerkleRoot);
        return { ...result, source: 'onchain' };
      } catch (error) {
        log.warn('On-chain verification failed', { error: error.message });
      }
    }

    return {
      verified: false,
      error: 'Signature not found in local cache',
    };
  }

  /**
   * Wire a BlockStore for retry sweeps
   * @param {Object} blockStore - BlockStore instance with getFailedAnchors()
   */
  setBlockStore(blockStore) {
    this._blockStore = blockStore;
  }

  /**
   * Start Fibonacci retry timer for failed anchors.
   * Sweeps every 21s (F(8)).
   * @private
   */
  _startRetryTimer() {
    if (this._retryInterval) return;
    this._retryInterval = setInterval(
      () => this._retryFailedAnchors(),
      SolanaAnchoringManager.RETRY_SWEEP_MS
    );
    // Don't prevent process exit (tests, graceful shutdown)
    if (this._retryInterval.unref) this._retryInterval.unref();
    log.debug('Retry timer started', { intervalMs: SolanaAnchoringManager.RETRY_SWEEP_MS });
  }

  /**
   * Stop retry timer.
   * @private
   */
  _stopRetryTimer() {
    if (this._retryInterval) {
      clearInterval(this._retryInterval);
      this._retryInterval = null;
    }
  }

  /**
   * Sweep failed anchors and retry with Fibonacci backoff.
   * @private
   */
  async _retryFailedAnchors() {
    if (!this._blockStore) return;

    let failed;
    try {
      failed = await this._blockStore.getFailedAnchors(5);
    } catch (err) {
      log.debug('Failed to fetch failed anchors for retry', { error: err.message });
      return;
    }

    if (!failed || failed.length === 0) return;

    for (const anchor of failed) {
      // Get retryCount from in-memory pendingAnchors or default to 0
      const pending = this._pendingAnchors.get(anchor.hash);
      const retryCount = pending?.retryCount || 0;

      // Cap retries
      if (retryCount >= SolanaAnchoringManager.MAX_RETRY_COUNT) {
        log.warn('Anchor retry cap reached', { slot: anchor.slot, retryCount });
        continue;
      }

      // Fibonacci backoff: skip if retried too recently
      const delayIdx = Math.min(retryCount, SolanaAnchoringManager.FIBONACCI_DELAYS.length - 1);
      const requiredDelay = SolanaAnchoringManager.FIBONACCI_DELAYS[delayIdx];
      const lastFailed = pending?.failedAt || 0;
      if (Date.now() - lastFailed < requiredDelay) continue;

      log.info('Retrying failed anchor', { slot: anchor.slot, retryCount, attempt: retryCount + 1 });
      this._stats.anchorsRetried++;

      await this.anchorBlock(
        { slot: anchor.slot, hash: anchor.hash, merkleRoot: anchor.merkleRoot },
        retryCount
      );
    }
  }

  /**
   * Cleanup anchorer (called on node stop)
   */
  cleanup() {
    this._stopRetryTimer();
    this._anchorer = null;
  }

  /** @returns {Object} Stats */
  get stats() {
    return { ...this._stats };
  }

  /** @returns {boolean} Whether enabled */
  get enabled() {
    return this._enabled;
  }
}

export default SolanaAnchoringManager;
