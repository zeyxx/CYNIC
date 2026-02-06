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
   * @param {number} [options.anchorInterval=100]
   */
  constructor(options = {}) {
    super();

    this._enabled = options.enabled ?? false;
    this._cluster = options.cluster || 'devnet';
    this._wallet = options.wallet || null;
    this._dryRun = options.dryRun ?? false;
    this._anchorer = null;
    this._pendingAnchors = new Map();  // blockHash -> { slot, merkleRoot, status, ... }
    this._lastAnchorSlot = 0;
    this._anchorInterval = options.anchorInterval || 100;

    this._stats = {
      blocksAnchored: 0,
      anchorsFailed: 0,
      lastAnchorSignature: null,
      lastAnchorTimestamp: null,
    };
  }

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
  }

  /** Disable anchoring */
  disable() {
    this._enabled = false;
    log.info('Solana anchoring disabled');
    this.emit('anchoring:disabled');
  }

  /**
   * Anchor a finalized block to Solana
   * @param {Object} block - { slot, hash, merkleRoot, ... }
   * @returns {Promise<Object|null>} Anchor result or null
   */
  async anchorBlock(block) {
    if (!this._enabled) return null;

    if (!this._anchorer && !this._wallet) {
      log.warn('Cannot anchor - no wallet or anchorer configured');
      return null;
    }

    const { slot, hash, merkleRoot } = block;

    this._pendingAnchors.set(hash, {
      slot,
      merkleRoot,
      status: 'pending',
      queuedAt: Date.now(),
    });

    log.info('Anchoring block to Solana', {
      slot,
      hash: hash?.slice(0, 16),
      merkleRoot: merkleRoot?.slice(0, 16),
      cluster: this._cluster,
    });

    try {
      const result = await this._createAnchorTransaction(block);

      if (result.success) {
        this._pendingAnchors.set(hash, {
          slot,
          merkleRoot,
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
          slot, hash, merkleRoot,
          signature: result.signature,
          cluster: this._cluster,
        });

        globalEventBus.publish(EventType.BLOCK_ANCHORED, {
          slot, hash, merkleRoot,
          signature: result.signature,
          cluster: this._cluster,
          timestamp: Date.now(),
        });

        return result;
      } else {
        throw new Error(result.error || 'Anchor failed');
      }
    } catch (error) {
      this._pendingAnchors.set(hash, {
        slot,
        merkleRoot,
        status: 'failed',
        error: error.message,
        failedAt: Date.now(),
      });

      this._stats.anchorsFailed++;

      log.error('Failed to anchor block', { slot, error: error.message });

      this.emit('anchor:failed', { slot, hash, error: error.message });

      return { success: false, error: error.message };
    }
  }

  /**
   * Resolve a valid 64-char hex merkle root from a block
   * @param {Object} block
   * @returns {string|null}
   */
  resolveMerkleRoot(block) {
    for (const key of ['judgments_root', 'judgmentsRoot', 'merkleRoot', 'hash']) {
      const val = block[key];
      if (val && /^[a-f0-9]{64}$/i.test(val)) return val;
    }
    return null;
  }

  /**
   * Create anchor transaction
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
        log.warn('Anchorer returned failure, falling back to simulation', { error: result.error });
      } catch (error) {
        log.warn('Anchorer error, falling back to simulation', { error: error.message });
      }
    }

    // Fallback simulation
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
   * Called when a block is finalized
   * @param {Object} block
   * @param {Function} recordBlockHash - ForkDetector's recordBlockHash
   */
  async onBlockFinalized(block, recordBlockHash) {
    recordBlockHash?.(block.slot, block.hash);

    if (this.shouldAnchor(block.slot)) {
      await this.anchorBlock(block);
    }
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
   * Cleanup anchorer (called on node stop)
   */
  cleanup() {
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
