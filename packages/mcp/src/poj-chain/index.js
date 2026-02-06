/**
 * PoJ Chain Manager - Thin Orchestrator
 *
 * Manages Proof of Judgment chain with automatic block batching.
 * Delegates to extracted components for each concern:
 *   - crypto-utils: SHA-256 and Merkle tree
 *   - anchor-manager: Solana anchoring
 *   - p2p-consensus: Distributed finality
 *   - block-validator: Block verification
 *   - chain-exporter: Export/import
 *
 * "The chain remembers, the dog forgets" - κυνικός
 *
 * @module @cynic/mcp/poj-chain
 */

'use strict';

import { createLogger, globalEventBus } from '@cynic/core';
import { sha256, merkleRoot } from './crypto-utils.js';
import { AnchorManager, AnchorStatus, BlockchainEvent } from './anchor-manager.js';
import { P2PConsensus } from './p2p-consensus.js';
import { BlockValidator } from './block-validator.js';
import { ChainExporter } from './chain-exporter.js';

const log = createLogger('PoJChainManager');

// Default configuration
const DEFAULT_BATCH_SIZE = 10;     // Create block after N judgments
const DEFAULT_BATCH_TIMEOUT = 60;  // Create block after T seconds

/**
 * PoJ Chain Manager - Thin Orchestrator
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

    // Legacy single-operator key (for backwards compatibility)
    this._legacyOperatorKey = options.operatorKey || null;

    // SRP: Extracted components
    this._anchorManager = new AnchorManager({
      anchorQueue: options.anchorQueue || null,
      autoAnchor: options.autoAnchor ?? true,
    });

    this._p2pConsensus = new P2PConsensus({
      nodeUrl: options.p2pNodeUrl || null,
      enabled: options.p2pEnabled ?? false,
      onFinalized: (block) => this._onBlockFinalized(block),
    });

    this._blockValidator = new BlockValidator({
      operatorRegistry: options.operatorRegistry || null,
      requireSignatures: options.requireSignatures || false,
      verifyReceivedBlocks: options.verifyReceivedBlocks ?? true,
    });

    this._chainExporter = new ChainExporter();

    // Event callbacks for SSE integration
    this._onBlockCreated = options.onBlockCreated || null;

    // Pending judgments waiting to be batched
    this._pendingJudgments = [];

    // Current chain head (loaded from DB on init)
    this._head = null;

    // Timer for automatic block creation
    this._batchTimer = null;

    // Core stats (component stats merged via getter)
    this._coreStats = {
      blocksCreated: 0,
      judgmentsProcessed: 0,
      lastBlockTime: null,
    };

    this._initialized = false;
  }

  /**
   * Check if multi-operator mode is enabled
   */
  get isMultiOperator() {
    return this._blockValidator.isMultiOperator;
  }

  /**
   * Unified stats getter (backwards compatible)
   * Merges stats from all components
   */
  get _stats() {
    return {
      ...this._coreStats,
      ...this._blockValidator.stats,
      ...this._anchorManager.stats,
      blocksFinalized: this._p2pConsensus.stats.blocksFinalized,
      finalityTimeouts: this._p2pConsensus.stats.finalityTimeouts,
    };
  }

  /**
   * Get current operator info
   */
  getOperatorInfo() {
    if (this._blockValidator._operatorRegistry) {
      return this._blockValidator._operatorRegistry.getSelf();
    }
    return this._legacyOperatorKey
      ? { publicKey: this._legacyOperatorKey.slice(0, 16), name: 'legacy' }
      : null;
  }

  /**
   * Initialize chain manager
   */
  async initialize() {
    if (this._initialized) return;

    const hasPoJCapability = this.persistence?.capabilities?.pojChain;

    if (hasPoJCapability) {
      this._head = await this.persistence.getPoJHead();
      if (this._head) {
        const backend = this.persistence.pojBlocks ? 'PostgreSQL' : 'fallback';
        log.info('PoJ chain resumed', { slot: this._head.slot, backend });
      } else {
        await this._createGenesisBlock();
      }
    } else {
      log.warn('PoJ chain: no persistence available');
    }

    // Initialize P2P consensus if enabled
    this._p2pConsensus.initialize();

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
      log.info('PoJ chain genesis block created');
    }
  }

  /**
   * Add a judgment to the pending batch
   * @param {Object} judgment - Judgment to add
   */
  async addJudgment(judgment) {
    if (!this.persistence?.capabilities?.pojChain) return;
    if (!this._initialized) await this.initialize();

    this._pendingJudgments.push({
      judgment_id: judgment.judgment_id || judgment.judgmentId || `jdg_${Date.now().toString(36)}`,
      q_score: judgment.q_score || judgment.qScore,
      verdict: judgment.verdict,
      timestamp: judgment.created_at || Date.now(),
    });

    this._coreStats.judgmentsProcessed++;

    if (this._pendingJudgments.length >= this.batchSize) {
      await this._createBlock();
    } else if (!this._batchTimer) {
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

    if (this._batchTimer) {
      clearTimeout(this._batchTimer);
      this._batchTimer = null;
    }

    const judgments = [...this._pendingJudgments];
    this._pendingJudgments = [];

    const judgmentHashes = judgments.map(j => sha256(j));
    const judgmentsRoot = merkleRoot(judgmentHashes);

    let block = {
      slot: (this._head?.slot || 0) + 1,
      prev_hash: this._head?.hash || this._head?.block_hash || sha256('CYNIC_GENESIS_φ'),
      judgments_root: judgmentsRoot,
      judgments: judgments,
      timestamp: Date.now(),
    };

    // Sign block if multi-operator mode
    if (this._blockValidator._operatorRegistry) {
      block = this._blockValidator._operatorRegistry.signBlock(block);
    } else {
      block.operator = this._legacyOperatorKey
        ? this._legacyOperatorKey.slice(0, 16)
        : 'default';
    }

    block.hash = sha256(block);

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
        this._coreStats.blocksCreated++;
        this._coreStats.lastBlockTime = new Date();

        // Handle anchoring (consensus-aware if P2P enabled)
        if (this._p2pConsensus.isEnabled) {
          this._p2pConsensus.propose(block).catch(err => {
            log.warn('P2P propose failed', { error: err.message });
          });
          this._anchorManager.markPending(block, { awaitingFinality: true });

          if (this._anchorManager.isEnabled) {
            this._p2pConsensus.waitForFinality(block).then(result => {
              if (result.finalized || result.fallback) {
                this._anchorManager.anchorBlock(block);
              }
            }).catch(err => {
              log.warn('Finality wait error', { error: err.message });
            });
          }
        } else {
          // Local mode: anchor immediately
          if (this._anchorManager.isEnabled) {
            this._anchorManager.anchorBlock(block);
          } else {
            this._anchorManager.markPending(block);
          }
        }

        log.info('PoJ block created', { slot: block.slot, judgments: judgments.length });

        // SSE callback
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
            log.warn('Block callback error', { error: e.message });
          }
        }

        // Emit blockchain event
        try {
          globalEventBus.publish(BlockchainEvent.BLOCK_CREATED, {
            slot: block.slot,
            hash: block.hash,
            prevHash: block.prev_hash,
            judgmentsRoot: block.judgments_root,
            judgmentCount: judgments.length,
            judgmentIds: judgments.map(j => j.judgment_id),
            operator: block.operator,
            timestamp: block.timestamp,
            anchorStatus: this._anchorManager.isEnabled ? AnchorStatus.QUEUED : AnchorStatus.PENDING,
          }, { source: 'PoJChainManager' });
        } catch (eventErr) {
          log.warn('Block event emission failed', { error: eventErr.message });
        }

        return stored;
      }
    } catch (err) {
      log.error('Error creating PoJ block', { error: err.message });
      this._pendingJudgments.unshift(...judgments);
    }

    return null;
  }

  /**
   * Handle block finalized callback from P2P consensus
   * @private
   */
  _onBlockFinalized(block) {
    if (this._anchorManager.isEnabled) {
      this._anchorManager.anchorBlock(block);
    }
  }

  /**
   * Receive a block from another operator
   * @param {Object} block - Block to receive
   */
  async receiveBlock(block) {
    if (!this.persistence?.capabilities?.pojChain) {
      return { accepted: false, error: 'PoJ chain not available' };
    }
    if (!this._initialized) await this.initialize();

    const result = await this._blockValidator.receiveBlock(block, this._head, this.persistence);
    if (result.accepted && result.newHead) {
      this._head = result.newHead;
    }
    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Delegated methods
  // ═══════════════════════════════════════════════════════════════════════════

  setP2PNode(url) { this._p2pConsensus.setNodeUrl(url); }
  setAnchorQueue(q) { this._anchorManager.setAnchorQueue(q); }
  getAnchorStatus(hash) { return this._anchorManager.getStatus(hash); }
  getPendingAnchors() { return this._anchorManager.getPending(); }
  get isAnchoringEnabled() { return this._anchorManager.isEnabled; }

  async verifyIntegrity() { return this._chainExporter.verifyIntegrity(this.persistence); }
  async exportChain(opts) { return this._chainExporter.exportChain(this.persistence, opts); }
  async importChain(data, opts) {
    const result = await this._chainExporter.importChain(this.persistence, data, opts);
    if (result.imported > 0) {
      this._head = await this.persistence.getPoJHead();
    }
    return result;
  }

  getHead() { return this._head; }
  getPendingCount() { return this._pendingJudgments.length; }

  /**
   * Get chain status
   */
  getStatus() {
    const anchorCounts = this._anchorManager.getCounts();
    const p2pStatus = this._p2pConsensus.getStatus();
    const validatorStats = this._blockValidator.stats;
    const anchorStats = this._anchorManager.stats;

    return {
      initialized: this._initialized,
      headSlot: this._head?.slot || 0,
      headHash: this._head?.hash || this._head?.block_hash || null,
      pendingJudgments: this._pendingJudgments.length,
      batchSize: this.batchSize,
      batchTimeout: this.batchTimeout,
      stats: {
        ...this._stats,
        ...validatorStats,
        ...anchorStats,
        blocksFinalized: p2pStatus.stats.blocksFinalized,
        finalityTimeouts: p2pStatus.stats.finalityTimeouts,
      },
      multiOperator: this.isMultiOperator,
      anchoringEnabled: this._anchorManager.isEnabled,
      anchoredBlocks: anchorCounts.anchored,
      pendingAnchors: anchorCounts.pending,
      p2pEnabled: p2pStatus.enabled,
      p2pNodeUrl: p2pStatus.nodeUrl,
      pendingFinality: p2pStatus.pendingFinality,
      operator: this.getOperatorInfo() || { publicKey: 'default', name: 'single' },
      operatorCount: this._blockValidator._operatorRegistry?.getAllOperators().length || 1,
      hasQuorum: this._blockValidator._operatorRegistry?.hasQuorum() ?? true,
    };
  }

  /**
   * Close and cleanup
   */
  async close() {
    if (this._pendingJudgments.length > 0) {
      await this._createBlock();
    }

    if (this._batchTimer) {
      clearTimeout(this._batchTimer);
      this._batchTimer = null;
    }

    this._p2pConsensus.close();
  }
}

// Re-exports
export { AnchorStatus, BlockchainEvent } from './anchor-manager.js';
export { sha256, merkleRoot } from './crypto-utils.js';
export { OperatorRegistry } from '../operator-registry.js';

export default PoJChainManager;
