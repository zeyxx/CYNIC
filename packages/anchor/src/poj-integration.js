/**
 * @cynic/anchor - PoJ Chain Integration
 *
 * Connects the PoJ chain to Solana anchoring.
 * Listens for finalized blocks and anchors their merkle roots.
 *
 * "Onchain is truth" - κυνικός
 *
 * @module @cynic/anchor/poj-integration
 */

'use strict';

import { EventEmitter } from 'events';
import { SolanaAnchorer } from './anchorer.js';
import { CynicWallet } from './wallet.js';
import { DEFAULT_CONFIG } from './constants.js';

/**
 * PoJ Anchor Integration
 *
 * Automatically anchors finalized PoJ blocks to Solana.
 */
export class PoJAnchorIntegration extends EventEmitter {
  /**
   * @param {Object} config - Configuration
   * @param {Object} config.pojChain - PoJ chain instance
   * @param {Object} [config.wallet] - Wallet for signing
   * @param {string} [config.cluster] - Solana cluster
   * @param {boolean} [config.autoAnchor] - Auto-anchor on finalization (default: true)
   * @param {number} [config.batchBlocks] - Anchor every N blocks (default: 1)
   */
  constructor(config = {}) {
    super();

    this.pojChain = config.pojChain;
    this.autoAnchor = config.autoAnchor !== false;
    this.batchBlocks = config.batchBlocks || 1;
    this.cluster = config.cluster || DEFAULT_CONFIG.cluster;

    // Create wallet
    if (config.wallet) {
      this.wallet = config.wallet instanceof CynicWallet
        ? config.wallet
        : new CynicWallet(config.wallet);
    } else if (config.walletPath) {
      this.wallet = CynicWallet.fromFile(config.walletPath);
    } else {
      this.wallet = null; // Simulation mode
    }

    // Create anchorer
    this.anchorer = new SolanaAnchorer({
      cluster: this.cluster,
      wallet: this.wallet,
      useAnchorProgram: true,
      onAnchor: (record) => this._onAnchorComplete(record),
      onError: (record, error) => this._onAnchorError(record, error),
    });

    // Track pending blocks
    this._pendingBlocks = [];
    this._lastAnchoredSlot = -1;

    // Stats
    this.stats = {
      blocksProcessed: 0,
      blocksAnchored: 0,
      rootsAnchored: 0,
      lastAnchorTime: null,
      lastAnchoredSlot: -1,
      errors: 0,
    };

    this._initialized = false;
  }

  /**
   * Initialize the integration
   */
  async init() {
    if (this._initialized) return;

    if (!this.pojChain) {
      throw new Error('PoJ chain is required');
    }

    // Listen for block finalization
    this.pojChain.on('block:finalized', (event) => this._onBlockFinalized(event));

    // Load last anchored slot from chain if available
    const chainStats = await this.pojChain.getStats?.() || {};
    this._lastAnchoredSlot = chainStats.lastAnchoredSlot || -1;

    this._initialized = true;
    this.emit('initialized');

    console.log(`🐕 PoJ Anchor Integration initialized`);
    console.log(`   Cluster: ${this.cluster}`);
    console.log(`   Auto-anchor: ${this.autoAnchor}`);
    console.log(`   Wallet: ${this.wallet ? 'configured' : 'simulation mode'}`);
  }

  /**
   * Handle block finalization
   * @param {Object} event - Finalization event
   * @private
   */
  async _onBlockFinalized(event) {
    const { block, slot } = event;

    this.stats.blocksProcessed++;

    if (!this.autoAnchor) {
      this.emit('block:skipped', { block, slot, reason: 'auto-anchor disabled' });
      return;
    }

    // Add to pending batch
    this._pendingBlocks.push(block);

    // Check if we should anchor
    if (this._pendingBlocks.length >= this.batchBlocks) {
      await this._anchorPendingBlocks();
    }
  }

  /**
   * Anchor pending blocks
   * @private
   */
  async _anchorPendingBlocks() {
    if (this._pendingBlocks.length === 0) return;

    const blocks = this._pendingBlocks;
    this._pendingBlocks = [];

    // Get merkle root from last block (includes all judgments)
    const lastBlock = blocks[blocks.length - 1];
    const merkleRoot = lastBlock.header?.judgmentsRoot || lastBlock.judgmentsRoot;

    if (!merkleRoot) {
      console.warn('🐕 Block has no judgments root, skipping anchor');
      return;
    }

    // Collect all judgment IDs from blocks
    const itemIds = [];
    for (const block of blocks) {
      const judgments = block.body?.judgments || block.judgments || [];
      for (const j of judgments) {
        const id = j.id || j.data?.id;
        if (id) itemIds.push(id);
      }
    }

    // Set block height for anchorer
    this.anchorer.setBlockHeight(lastBlock.slot);

    // Convert merkle root to hex string if needed
    const rootHex = Buffer.isBuffer(merkleRoot)
      ? merkleRoot.toString('hex')
      : merkleRoot;

    console.log(`🐕 Anchoring PoJ block ${lastBlock.slot} with ${itemIds.length} judgments`);
    console.log(`   Root: ${rootHex.slice(0, 16)}...`);

    try {
      const result = await this.anchorer.anchor(rootHex, itemIds);

      if (result.success) {
        this.stats.blocksAnchored += blocks.length;
        this.stats.rootsAnchored++;
        this.stats.lastAnchorTime = Date.now();
        this.stats.lastAnchoredSlot = lastBlock.slot;
        this._lastAnchoredSlot = lastBlock.slot;

        this.emit('blocks:anchored', {
          blocks,
          slot: lastBlock.slot,
          merkleRoot: rootHex,
          signature: result.signature,
          itemCount: itemIds.length,
        });

        console.log(`✅ Anchored! Signature: ${result.signature?.slice(0, 20)}...`);
      } else {
        this.stats.errors++;
        this.emit('anchor:failed', {
          blocks,
          error: result.error,
        });

        // Return blocks to pending for retry
        this._pendingBlocks.unshift(...blocks);
      }
    } catch (error) {
      this.stats.errors++;
      this.emit('anchor:error', { blocks, error });
      console.error('🐕 Anchor error:', error.message);

      // Return blocks to pending
      this._pendingBlocks.unshift(...blocks);
    }
  }

  /**
   * Force anchor current pending blocks
   * @returns {Promise<Object>} Anchor result
   */
  async flush() {
    return this._anchorPendingBlocks();
  }

  /**
   * Manually anchor a specific block
   * @param {Object} block - Block to anchor
   * @returns {Promise<Object>} Anchor result
   */
  async anchorBlock(block) {
    const merkleRoot = block.header?.judgmentsRoot || block.judgmentsRoot;
    if (!merkleRoot) {
      throw new Error('Block has no judgments root');
    }

    const itemIds = [];
    const judgments = block.body?.judgments || block.judgments || [];
    for (const j of judgments) {
      const id = j.id || j.data?.id;
      if (id) itemIds.push(id);
    }

    this.anchorer.setBlockHeight(block.slot);

    const rootHex = Buffer.isBuffer(merkleRoot)
      ? merkleRoot.toString('hex')
      : merkleRoot;

    return this.anchorer.anchor(rootHex, itemIds);
  }

  /**
   * Verify a block's anchor on Solana
   * @param {Object} block - Block to verify
   * @returns {Promise<Object>} Verification result
   */
  async verifyBlockAnchor(block) {
    const merkleRoot = block.header?.judgmentsRoot || block.judgmentsRoot;
    if (!merkleRoot) {
      return { verified: false, error: 'Block has no judgments root' };
    }

    const rootHex = Buffer.isBuffer(merkleRoot)
      ? merkleRoot.toString('hex')
      : merkleRoot;

    return this.anchorer.verifyAnchor(rootHex);
  }

  /**
   * Handle anchor completion
   * @param {Object} record - Anchor record
   * @private
   */
  _onAnchorComplete(record) {
    this.emit('anchor:complete', record);
  }

  /**
   * Handle anchor error
   * @param {Object} record - Anchor record
   * @param {Error} error - Error
   * @private
   */
  _onAnchorError(record, error) {
    this.emit('anchor:error', { record, error });
  }

  /**
   * Get integration stats
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      anchorerStats: this.anchorer.getStats(),
      pendingBlocks: this._pendingBlocks.length,
      cluster: this.cluster,
      hasWallet: !!this.wallet,
      autoAnchor: this.autoAnchor,
    };
  }

  /**
   * Stop the integration
   */
  stop() {
    if (this.pojChain) {
      this.pojChain.removeAllListeners('block:finalized');
    }
    this._initialized = false;
    this.emit('stopped');
  }
}

/**
 * Create PoJ anchor integration
 * @param {Object} config - Configuration
 * @returns {PoJAnchorIntegration}
 */
export function createPoJAnchorIntegration(config = {}) {
  return new PoJAnchorIntegration(config);
}

/**
 * Quick setup helper - connects PoJ chain to Solana
 * @param {Object} pojChain - PoJ chain instance
 * @param {Object} [options] - Options
 * @returns {Promise<PoJAnchorIntegration>}
 */
export async function connectPoJToSolana(pojChain, options = {}) {
  const integration = new PoJAnchorIntegration({
    pojChain,
    ...options,
  });

  await integration.init();
  return integration;
}
