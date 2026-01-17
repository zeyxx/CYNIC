/**
 * PoJ Chain Manager
 *
 * Manages Proof of Judgment chain with automatic block batching.
 * Creates blocks when N judgments accumulated or T seconds passed.
 *
 * "The chain remembers, the dog forgets" - κυνικός
 *
 * @module @cynic/mcp/poj-chain-manager
 */

'use strict';

import crypto from 'crypto';

// Default configuration
const DEFAULT_BATCH_SIZE = 10;     // Create block after N judgments
const DEFAULT_BATCH_TIMEOUT = 60;  // Create block after T seconds (even if < N judgments)
const DEFAULT_OPERATOR_KEY = 'default_operator_' + crypto.randomBytes(16).toString('hex');

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
 */
export class PoJChainManager {
  /**
   * @param {Object} persistence - PersistenceManager instance
   * @param {Object} [options] - Configuration options
   */
  constructor(persistence, options = {}) {
    this.persistence = persistence;
    this.batchSize = options.batchSize || DEFAULT_BATCH_SIZE;
    this.batchTimeout = options.batchTimeout || DEFAULT_BATCH_TIMEOUT;
    this.operatorKey = options.operatorKey || DEFAULT_OPERATOR_KEY;

    // Pending judgments waiting to be batched
    this._pendingJudgments = [];

    // Current chain head (loaded from DB on init)
    this._head = null;

    // Timer for automatic block creation
    this._batchTimer = null;

    // Stats
    this._stats = {
      blocksCreated: 0,
      judgmentsProcessed: 0,
      lastBlockTime: null,
    };

    this._initialized = false;
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
    const block = {
      slot: (this._head?.slot || 0) + 1,
      prev_hash: this._head?.hash || this._head?.block_hash || sha256('CYNIC_GENESIS_φ'),
      judgments_root: judgmentsRoot,
      judgments: judgments,
      timestamp: Date.now(),
      operator: this.operatorKey.slice(0, 16),
    };
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

        console.error(`🔗 PoJ Block #${block.slot} created: ${judgments.length} judgments`);
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
   * Get chain status
   */
  getStatus() {
    return {
      initialized: this._initialized,
      headSlot: this._head?.slot || 0,
      headHash: this._head?.hash || this._head?.block_hash || null,
      pendingJudgments: this._pendingJudgments.length,
      batchSize: this.batchSize,
      batchTimeout: this.batchTimeout,
      stats: this._stats,
    };
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

export default PoJChainManager;
