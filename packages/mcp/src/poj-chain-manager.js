/**
 * PoJ Chain Manager
 *
 * Manages Proof of Judgment chain with automatic block batching.
 * Creates blocks when N judgments accumulated or T seconds passed.
 *
 * Supports multi-operator mode with:
 * - Operator registry for authorized block producers
 * - Block signing with operator keys
 * - Signature verification for received blocks
 *
 * Supports Solana anchoring:
 * - Anchors block merkle roots to Solana for "onchain is truth"
 * - Batches anchors for cost efficiency
 * - Tracks anchor status per block
 *
 * "The chain remembers, the dog forgets" - κυνικός
 * "Many dogs, one pack" - multi-operator consensus
 * "Onchain is truth" - anchoring to Solana
 *
 * @module @cynic/mcp/poj-chain-manager
 */

'use strict';

import crypto from 'crypto';
import { OperatorRegistry } from './operator-registry.js';

/**
 * Anchor status for blocks
 */
export const AnchorStatus = {
  /** Block not yet anchored */
  PENDING: 'PENDING',
  /** Block queued for anchoring */
  QUEUED: 'QUEUED',
  /** Block anchored to Solana */
  ANCHORED: 'ANCHORED',
  /** Anchoring failed */
  FAILED: 'FAILED',
};

// Default configuration
const DEFAULT_BATCH_SIZE = 10;     // Create block after N judgments
const DEFAULT_BATCH_TIMEOUT = 60;  // Create block after T seconds (even if < N judgments)

/**
 * Simple SHA-256 hash
 */
function sha256(data) {
  return crypto.createHash('sha256')
    .update(typeof data === 'string' ? data : JSON.stringify(data))
    .digest('hex');
}

/**
 * Calculate merkle root from array of hashes
 */
function merkleRoot(hashes) {
  if (hashes.length === 0) return sha256('empty');
  if (hashes.length === 1) return hashes[0];

  const pairs = [];
  for (let i = 0; i < hashes.length; i += 2) {
    const left = hashes[i];
    const right = hashes[i + 1] || left; // Duplicate last if odd
    pairs.push(sha256(left + right));
  }
  return merkleRoot(pairs);
}

/**
 * PoJ Chain Manager
 *
 * Coordinates judgment batching and block creation.
 * Supports single-operator (legacy) and multi-operator modes.
 */
export class PoJChainManager {
  /**
   * @param {Object} persistence - PersistenceManager instance
   * @param {Object} [options] - Configuration options
   * @param {number} [options.batchSize=10] - Judgments per block
   * @param {number} [options.batchTimeout=60] - Seconds before auto-creating block
   * @param {string} [options.operatorKey] - Legacy single operator key
   * @param {OperatorRegistry} [options.operatorRegistry] - Multi-operator registry
   * @param {boolean} [options.requireSignatures=false] - Require signed blocks
   * @param {boolean} [options.verifyReceivedBlocks=true] - Verify blocks from others
   * @param {Object} [options.anchorQueue] - AnchorQueue instance for Solana anchoring
   * @param {boolean} [options.autoAnchor=true] - Auto-anchor blocks to Solana
   * @param {Function} [options.onBlockCreated] - Callback when new block is created
   */
  constructor(persistence, options = {}) {
    this.persistence = persistence;
    this.batchSize = options.batchSize || DEFAULT_BATCH_SIZE;
    this.batchTimeout = options.batchTimeout || DEFAULT_BATCH_TIMEOUT;

    // Multi-operator support
    this.operatorRegistry = options.operatorRegistry || null;
    this.requireSignatures = options.requireSignatures || false;
    this.verifyReceivedBlocks = options.verifyReceivedBlocks ?? true;

    // Solana anchoring support
    this._anchorQueue = options.anchorQueue || null;
    this._autoAnchor = options.autoAnchor ?? true;

    // Event callbacks for SSE integration
    this._onBlockCreated = options.onBlockCreated || null;

    // Legacy single-operator key (for backwards compatibility)
    this._legacyOperatorKey = options.operatorKey || null;

    // Pending judgments waiting to be batched
    this._pendingJudgments = [];

    // Current chain head (loaded from DB on init)
    this._head = null;

    // Timer for automatic block creation
    this._batchTimer = null;

    // Track anchor status per block
    this._anchorStatus = new Map();

    // Stats
    this._stats = {
      blocksCreated: 0,
      blocksReceived: 0,
      blocksRejected: 0,
      judgmentsProcessed: 0,
      lastBlockTime: null,
      signatureVerifications: 0,
      signatureFailures: 0,
      blocksAnchored: 0,
      anchorsFailed: 0,
    };

    this._initialized = false;
  }

  /**
   * Check if multi-operator mode is enabled
   * @returns {boolean} True if multi-operator mode
   */
  get isMultiOperator() {
    return this.operatorRegistry !== null;
  }

  /**
   * Get current operator info
   * @returns {Object|null} Operator info or null
   */
  getOperatorInfo() {
    if (this.operatorRegistry) {
      return this.operatorRegistry.getSelf();
    }
    return this._legacyOperatorKey
      ? { publicKey: this._legacyOperatorKey.slice(0, 16), name: 'legacy' }
      : null;
  }

  /**
   * Initialize chain manager (load head from DB or fallback)
   * "φ distrusts φ" - the chain must exist for CYNIC to verify itself
   */
  async initialize() {
    if (this._initialized) return;

    // Check if PoJ chain is available (PostgreSQL or fallback)
    const hasPoJCapability = this.persistence?.capabilities?.pojChain;

    if (hasPoJCapability) {
      this._head = await this.persistence.getPoJHead();
      if (this._head) {
        const backend = this.persistence.pojBlocks ? 'PostgreSQL' : 'fallback';
        console.error(`   PoJ Chain: resumed at slot ${this._head.slot} (${this._head.judgment_count || 0} judgments) [${backend}]`);
      } else {
        // Create genesis block
        await this._createGenesisBlock();
      }
    } else {
      console.error('   PoJ Chain: no persistence available (CYNIC cannot verify itself!)');
    }

    this._initialized = true;
  }

  /**
   * Create genesis block
   * @private
   */
  async _createGenesisBlock() {
    const genesis = {
      slot: 0,
      prev_hash: sha256('CYNIC_GENESIS_φ'),
      judgments_root: sha256('genesis'),
      judgments: [],
      timestamp: Date.now(),
      hash: null,
    };
    genesis.hash = sha256(genesis);

    const stored = await this.persistence.storePoJBlock(genesis);
    if (stored) {
      this._head = stored;
      console.error('   PoJ Chain: genesis block created');
    }
  }

  /**
   * Add a judgment to the pending batch
   * @param {Object} judgment - Judgment to add (must have judgment_id)
   */
  async addJudgment(judgment) {
    // Check if PoJ chain is available (PostgreSQL or fallback)
    if (!this.persistence?.capabilities?.pojChain) {
      return; // PoJ chain not available (no persistence)
    }

    // Auto-initialize if needed (e.g., after reset)
    if (!this._initialized) {
      await this.initialize();
    }

    this._pendingJudgments.push({
      judgment_id: judgment.judgment_id || judgment.judgmentId || `jdg_${Date.now().toString(36)}`,
      q_score: judgment.q_score || judgment.qScore,
      verdict: judgment.verdict,
      timestamp: judgment.created_at || Date.now(),
    });

    this._stats.judgmentsProcessed++;

    // Check if we should create a block
    if (this._pendingJudgments.length >= this.batchSize) {
      await this._createBlock();
    } else if (!this._batchTimer) {
      // Start timeout timer
      this._batchTimer = setTimeout(async () => {
        if (this._pendingJudgments.length > 0) {
          await this._createBlock();
        }
        this._batchTimer = null;
      }, this.batchTimeout * 1000);
    }
  }

  /**
   * Force create a block from pending judgments
   * @returns {Object|null} Created block or null
   */
  async flush() {
    if (this._pendingJudgments.length > 0) {
      return await this._createBlock();
    }
    return null;
  }

  /**
   * Create a new block from pending judgments
   * @private
   */
  async _createBlock() {
    if (this._pendingJudgments.length === 0) return null;

    // Clear timer if running
    if (this._batchTimer) {
      clearTimeout(this._batchTimer);
      this._batchTimer = null;
    }

    // Take pending judgments
    const judgments = [...this._pendingJudgments];
    this._pendingJudgments = [];

    // Calculate hashes
    const judgmentHashes = judgments.map(j => sha256(j));
    const judgmentsRoot = merkleRoot(judgmentHashes);

    // Create block
    let block = {
      slot: (this._head?.slot || 0) + 1,
      prev_hash: this._head?.hash || this._head?.block_hash || sha256('CYNIC_GENESIS_φ'),
      judgments_root: judgmentsRoot,
      judgments: judgments,
      timestamp: Date.now(),
    };

    // Sign block if multi-operator mode
    if (this.operatorRegistry) {
      block = this.operatorRegistry.signBlock(block);
    } else {
      // Legacy single-operator mode
      block.operator = this._legacyOperatorKey
        ? this._legacyOperatorKey.slice(0, 16)
        : 'default';
    }

    block.hash = sha256(block);

    // Store block
    try {
      const stored = await this.persistence.storePoJBlock(block);
      if (stored) {
        this._head = {
          slot: block.slot,
          hash: block.hash,
          block_hash: block.hash,
          prev_hash: block.prev_hash,
          judgment_count: judgments.length,
        };
        this._stats.blocksCreated++;
        this._stats.lastBlockTime = new Date();

        // Queue for Solana anchoring if enabled
        if (this._anchorQueue && this._autoAnchor) {
          this._anchorBlock(block);
        } else {
          // Mark as PENDING anchor status
          this._anchorStatus.set(block.hash, {
            status: AnchorStatus.PENDING,
            slot: block.slot,
          });
        }

        console.error(`🔗 PoJ Block #${block.slot} created: ${judgments.length} judgments`);

        // Emit event for SSE broadcast
        if (this._onBlockCreated) {
          try {
            this._onBlockCreated({
              blockNumber: block.slot,
              hash: block.hash,
              prevHash: block.prev_hash,
              judgmentCount: judgments.length,
              timestamp: block.timestamp,
            });
          } catch (e) {
            // Non-blocking - don't fail block creation for callback errors
            console.error('Block callback error:', e.message);
          }
        }

        return stored;
      }
    } catch (err) {
      console.error('Error creating PoJ block:', err.message);
      // Put judgments back in pending
      this._pendingJudgments.unshift(...judgments);
    }

    return null;
  }

  /**
   * Anchor a block to Solana
   * @param {Object} block - Block to anchor
   * @private
   */
  async _anchorBlock(block) {
    const blockId = `poj_block_${block.slot}`;

    // Mark as queued
    this._anchorStatus.set(block.hash, {
      status: AnchorStatus.QUEUED,
      slot: block.slot,
      queuedAt: Date.now(),
    });

    try {
      // Queue the block's merkle root (judgments_root) for anchoring
      // The anchor queue will batch and anchor to Solana
      this._anchorQueue.enqueue(blockId, {
        type: 'poj_block',
        slot: block.slot,
        hash: block.hash,
        merkleRoot: block.judgments_root,
        judgmentCount: block.judgments?.length || 0,
        timestamp: block.timestamp,
      });

      console.error(`⚓ PoJ Block #${block.slot} queued for Solana anchoring`);
    } catch (err) {
      console.error(`Error queuing block for anchor: ${err.message}`);
      this._anchorStatus.set(block.hash, {
        status: AnchorStatus.FAILED,
        slot: block.slot,
        error: err.message,
      });
      this._stats.anchorsFailed++;
    }
  }

  /**
   * Set anchor queue for Solana anchoring
   * @param {Object} anchorQueue - AnchorQueue instance
   */
  setAnchorQueue(anchorQueue) {
    this._anchorQueue = anchorQueue;

    // Set up callback for anchor completion
    if (anchorQueue && anchorQueue.onAnchorComplete === undefined) {
      anchorQueue.onAnchorComplete = (batch, result) => {
        this._onAnchorComplete(batch, result);
      };
    }
  }

  /**
   * Handle anchor completion callback
   * @param {Object} batch - Anchored batch
   * @param {Object} result - Anchor result
   * @private
   */
  _onAnchorComplete(batch, result) {
    if (!result.success) return;

    // Update anchor status for all blocks in this batch
    for (const item of batch.items) {
      if (item.id.startsWith('poj_block_')) {
        const slot = parseInt(item.id.replace('poj_block_', ''), 10);

        // Find block by slot
        for (const [hash, status] of this._anchorStatus.entries()) {
          if (status.slot === slot) {
            this._anchorStatus.set(hash, {
              status: AnchorStatus.ANCHORED,
              slot,
              signature: result.signature,
              anchoredAt: result.timestamp,
            });
            this._stats.blocksAnchored++;
            console.error(`⚓ PoJ Block #${slot} anchored to Solana: ${result.signature.slice(0, 16)}...`);
            break;
          }
        }
      }
    }
  }

  /**
   * Get anchor status for a block
   * @param {string} blockHash - Block hash
   * @returns {Object|null} Anchor status
   */
  getAnchorStatus(blockHash) {
    return this._anchorStatus.get(blockHash) || null;
  }

  /**
   * Get all pending anchor statuses
   * @returns {Object[]} Pending anchors
   */
  getPendingAnchors() {
    const pending = [];
    for (const [hash, status] of this._anchorStatus.entries()) {
      if (status.status === AnchorStatus.PENDING || status.status === AnchorStatus.QUEUED) {
        pending.push({ hash, ...status });
      }
    }
    return pending;
  }

  /**
   * Check if anchoring is enabled
   * @returns {boolean}
   */
  get isAnchoringEnabled() {
    return this._anchorQueue !== null;
  }

  /**
   * Receive a block from another operator
   * @param {Object} block - Block to receive
   * @returns {{accepted: boolean, error?: string}} Result
   */
  async receiveBlock(block) {
    // Check if PoJ chain is available
    if (!this.persistence?.capabilities?.pojChain) {
      return { accepted: false, error: 'PoJ chain not available' };
    }

    // Auto-initialize if needed
    if (!this._initialized) {
      await this.initialize();
    }

    // Verify signature if multi-operator mode and verification enabled
    if (this.verifyReceivedBlocks && this.operatorRegistry) {
      const verification = this.operatorRegistry.verifyBlock(block);
      if (!verification.valid) {
        this._stats.blocksRejected++;
        this._stats.signatureFailures++;
        return { accepted: false, error: verification.error };
      }
      this._stats.signatureVerifications++;
    } else if (this.requireSignatures && !block.signature) {
      this._stats.blocksRejected++;
      return { accepted: false, error: 'Block signature required but not present' };
    }

    // Validate chain integrity
    const expectedPrevHash = this._head?.hash || this._head?.block_hash || sha256('CYNIC_GENESIS_φ');
    if (block.prev_hash !== expectedPrevHash) {
      // Could be a fork - for now reject
      this._stats.blocksRejected++;
      return { accepted: false, error: `Chain mismatch: expected prev_hash ${expectedPrevHash.slice(0, 16)}...` };
    }

    const expectedSlot = (this._head?.slot || 0) + 1;
    if (block.slot !== expectedSlot) {
      this._stats.blocksRejected++;
      return { accepted: false, error: `Slot mismatch: expected ${expectedSlot}, got ${block.slot}` };
    }

    // Verify block hash
    const computedHash = sha256({
      slot: block.slot,
      prev_hash: block.prev_hash,
      judgments_root: block.judgments_root,
      judgments: block.judgments,
      timestamp: block.timestamp,
      operator: block.operator,
      operator_name: block.operator_name,
      signature: block.signature,
    });

    if (block.hash && block.hash !== computedHash) {
      this._stats.blocksRejected++;
      return { accepted: false, error: 'Invalid block hash' };
    }

    // Store block
    try {
      const stored = await this.persistence.storePoJBlock(block);
      if (stored) {
        this._head = {
          slot: block.slot,
          hash: block.hash || computedHash,
          block_hash: block.hash || computedHash,
          prev_hash: block.prev_hash,
          judgment_count: block.judgments?.length || 0,
        };
        this._stats.blocksReceived++;

        console.error(`🔗 PoJ Block #${block.slot} received from ${(block.operator_name || block.operator || 'unknown').slice(0, 16)}`);
        return { accepted: true };
      }
    } catch (err) {
      this._stats.blocksRejected++;
      return { accepted: false, error: err.message };
    }

    return { accepted: false, error: 'Unknown error storing block' };
  }

  /**
   * Get chain status
   */
  getStatus() {
    // Count anchor statuses
    let anchoredCount = 0;
    let pendingAnchorCount = 0;
    for (const status of this._anchorStatus.values()) {
      if (status.status === AnchorStatus.ANCHORED) anchoredCount++;
      else if (status.status === AnchorStatus.PENDING || status.status === AnchorStatus.QUEUED) {
        pendingAnchorCount++;
      }
    }

    const status = {
      initialized: this._initialized,
      headSlot: this._head?.slot || 0,
      headHash: this._head?.hash || this._head?.block_hash || null,
      pendingJudgments: this._pendingJudgments.length,
      batchSize: this.batchSize,
      batchTimeout: this.batchTimeout,
      stats: this._stats,
      multiOperator: this.isMultiOperator,
      // Anchoring status
      anchoringEnabled: this.isAnchoringEnabled,
      anchoredBlocks: anchoredCount,
      pendingAnchors: pendingAnchorCount,
    };

    // Add operator info
    if (this.operatorRegistry) {
      const self = this.operatorRegistry.getSelf();
      status.operator = self ? {
        publicKey: self.publicKey.slice(0, 16) + '...',
        name: self.name,
      } : null;
      status.operatorCount = this.operatorRegistry.getAllOperators().length;
      status.hasQuorum = this.operatorRegistry.hasQuorum();
    } else {
      status.operator = this._legacyOperatorKey
        ? { publicKey: this._legacyOperatorKey.slice(0, 16), name: 'legacy' }
        : { publicKey: 'default', name: 'single' };
      status.operatorCount = 1;
      status.hasQuorum = true;
    }

    return status;
  }

  /**
   * Get current head block
   */
  getHead() {
    return this._head;
  }

  /**
   * Get pending judgments count
   */
  getPendingCount() {
    return this._pendingJudgments.length;
  }

  /**
   * Verify chain integrity (PostgreSQL or fallback)
   */
  async verifyIntegrity() {
    if (!this.persistence?.capabilities?.pojChain) {
      return { valid: true, blocksChecked: 0, errors: [] };
    }
    return await this.persistence.verifyPoJChain();
  }

  /**
   * Export chain data for backup (PostgreSQL or fallback)
   * @param {Object} [options] - Export options
   * @returns {Promise<Object>} Exportable chain data
   */
  async exportChain(options = {}) {
    const { fromBlock = 0, limit = 1000 } = options;

    if (!this.persistence?.capabilities?.pojChain) {
      return { error: 'Persistence not available', blocks: [] };
    }

    // Use PersistenceManager methods which handle fallback internally
    const blocks = await this.persistence.getRecentPoJBlocks(limit);
    const stats = await this.persistence.getPoJStats();

    // Filter blocks by fromBlock slot
    const filteredBlocks = blocks.filter(b => b.slot >= fromBlock);

    return {
      version: '1.0.0',
      exportedAt: new Date().toISOString(),
      chainStats: stats,
      blocks: filteredBlocks.map(b => ({
        slot: b.slot,
        hash: b.hash || b.block_hash,
        prevHash: b.prev_hash,
        merkleRoot: b.merkle_root || b.judgments_root,
        judgmentCount: b.judgment_count,
        judgmentIds: b.judgment_ids,
        timestamp: b.timestamp instanceof Date ? b.timestamp.toISOString() : b.timestamp,
      })),
      totalBlocks: filteredBlocks.length,
    };
  }

  /**
   * Import chain data from backup (PostgreSQL or fallback)
   * @param {Object} chainData - Exported chain data
   * @param {Object} [options] - Import options
   * @returns {Promise<Object>} Import result
   */
  async importChain(chainData, options = {}) {
    const { validateLinks = true, skipExisting = true } = options;

    if (!this.persistence?.capabilities?.pojChain) {
      return { error: 'Persistence not available', imported: 0 };
    }

    if (!chainData?.blocks || !Array.isArray(chainData.blocks)) {
      return { error: 'Invalid chain data format', imported: 0 };
    }

    const results = {
      imported: 0,
      skipped: 0,
      errors: [],
    };

    // Sort blocks by slot
    const sortedBlocks = [...chainData.blocks].sort((a, b) => a.slot - b.slot);

    // Validate chain links if requested
    if (validateLinks && sortedBlocks.length > 1) {
      for (let i = 1; i < sortedBlocks.length; i++) {
        const block = sortedBlocks[i];
        const prevBlock = sortedBlocks[i - 1];
        if (block.prevHash !== prevBlock.hash) {
          results.errors.push({
            slot: block.slot,
            error: `Invalid prev_hash: expected ${prevBlock.hash}, got ${block.prevHash}`,
          });
        }
      }

      if (results.errors.length > 0) {
        return {
          error: 'Chain validation failed',
          ...results,
        };
      }
    }

    // Import blocks using PersistenceManager methods
    for (const block of sortedBlocks) {
      try {
        // Check if exists
        if (skipExisting) {
          const existing = await this.persistence.getPoJBlockBySlot(block.slot);
          if (existing) {
            results.skipped++;
            continue;
          }
        }

        // Store block using PersistenceManager method
        await this.persistence.storePoJBlock({
          slot: block.slot,
          hash: block.hash,
          block_hash: block.hash,
          prev_hash: block.prevHash,
          judgments_root: block.merkleRoot,
          merkle_root: block.merkleRoot,
          judgments: block.judgmentIds?.map(id => ({ judgment_id: id })) || [],
          timestamp: new Date(block.timestamp).getTime(),
        });

        results.imported++;
      } catch (err) {
        results.errors.push({
          slot: block.slot,
          error: err.message,
        });
      }
    }

    // Update head if we imported blocks
    if (results.imported > 0) {
      this._head = await this.persistence.getPoJHead();
    }

    return results;
  }

  /**
   * Close and cleanup
   */
  async close() {
    // Flush any pending judgments
    if (this._pendingJudgments.length > 0) {
      await this._createBlock();
    }

    // Clear timer
    if (this._batchTimer) {
      clearTimeout(this._batchTimer);
      this._batchTimer = null;
    }
  }
}

export { OperatorRegistry };
export default PoJChainManager;
