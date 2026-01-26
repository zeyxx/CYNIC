/**
 * State Manager
 *
 * Manages all node state: chain, knowledge, peers, operator
 *
 * @module @cynic/node/state/manager
 */

'use strict';

import { PoJChain, KnowledgeTree } from '@cynic/protocol';
import { MemoryStorage, FileStorage } from './storage.js';
import { createLogger } from '@cynic/core';

const log = createLogger('StateManager');

/**
 * State Manager - Orchestrates all node state
 */
export class StateManager {
  /**
   * @param {Object} options - Manager options
   * @param {string} [options.dataDir] - Data directory (null for memory only)
   * @param {Object} options.operator - Operator instance
   */
  constructor(options = {}) {
    this.dataDir = options.dataDir;
    this.operator = options.operator;

    // Initialize storage
    this.storage = this.dataDir
      ? new FileStorage(this.dataDir)
      : new MemoryStorage();

    // State components (lazy initialized)
    this._chain = null;
    this._knowledge = null;
    this._peers = new Map();
    this._judgments = [];

    // Block persistence (individual blocks stored separately)
    this._blockCache = new Map();

    this.initialized = false;
  }

  /**
   * Initialize state manager
   * Loads existing state or creates fresh state
   */
  async initialize() {
    if (this.initialized) return;

    // Try to load existing state
    const savedState = await this.storage.get('state');

    if (savedState) {
      await this._loadState(savedState);
    } else {
      await this._initFreshState();
    }

    this.initialized = true;
  }

  /**
   * Load state from storage
   * @private
   */
  async _loadState(savedState) {
    // Restore chain
    if (savedState.chain) {
      this._chain = PoJChain.import(savedState.chain, this.operator.privateKey);
    } else {
      await this._initChain();
    }

    // Restore knowledge tree
    if (savedState.knowledge) {
      this._knowledge = KnowledgeTree.import(savedState.knowledge);
    } else {
      this._knowledge = new KnowledgeTree();
    }

    // Restore peers
    if (savedState.peers) {
      this._peers = new Map(Object.entries(savedState.peers));
    }

    // Restore judgments (recent only)
    if (savedState.judgments) {
      this._judgments = savedState.judgments;
    }
  }

  /**
   * Initialize fresh state
   * @private
   */
  async _initFreshState() {
    await this._initChain();
    this._knowledge = new KnowledgeTree();
    this._peers = new Map();
    this._judgments = [];
  }

  /**
   * Initialize chain
   * @private
   */
  async _initChain() {
    this._chain = new PoJChain({
      operatorPublicKey: this.operator.publicKey,
      operatorPrivateKey: this.operator.privateKey,
    });
  }

  /**
   * Get chain
   * @returns {PoJChain} Chain instance
   */
  get chain() {
    if (!this._chain) {
      throw new Error('State not initialized');
    }
    return this._chain;
  }

  /**
   * Get knowledge tree
   * @returns {KnowledgeTree} Knowledge tree instance
   */
  get knowledge() {
    if (!this._knowledge) {
      throw new Error('State not initialized');
    }
    return this._knowledge;
  }

  /**
   * Add judgment to state
   * @param {Object} judgment - Judgment to add
   */
  addJudgment(judgment) {
    this._judgments.push(judgment);

    // Keep bounded (last 1000)
    if (this._judgments.length > 1000) {
      this._judgments.shift();
    }
  }

  /**
   * Get recent judgments
   * @param {number} [count=100] - Number to return
   * @returns {Object[]} Recent judgments
   */
  getRecentJudgments(count = 100) {
    return this._judgments.slice(-count);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Block Persistence (Gap #3)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Persist a finalized block
   * @param {Object} block - Block to persist
   * @returns {Promise<boolean>} Success
   */
  async persistBlock(block) {
    if (!block?.hash && !block?.slot) return false;

    const blockKey = `block_${block.slot}`;
    const blockData = {
      slot: block.slot,
      hash: block.hash || this._hashBlock(block),
      previousHash: block.previousHash || block.prevHash,
      timestamp: block.timestamp,
      proposer: block.proposer,
      signature: block.signature,
      judgments: block.judgments?.map(j => ({
        id: j.id,
        global_score: j.global_score,
        verdict: j.verdict,
        timestamp: j.timestamp,
      })) || [],
      judgmentCount: block.judgments?.length || 0,
      status: block.status || 'FINALIZED',
      persistedAt: Date.now(),
    };

    try {
      await this.storage.set(blockKey, blockData);
      this._blockCache.set(block.slot, blockData);

      // Update block index
      const index = await this.storage.get('block_index') || { slots: [], hashes: {} };
      if (!index.slots.includes(block.slot)) {
        index.slots.push(block.slot);
        index.slots.sort((a, b) => a - b);
      }
      index.hashes[blockData.hash] = block.slot;
      index.lastSlot = Math.max(index.lastSlot || 0, block.slot);
      await this.storage.set('block_index', index);

      return true;
    } catch (err) {
      log.warn('Failed to persist block', { error: err.message });
      return false;
    }
  }

  /**
   * Get block by slot
   * @param {number} slot - Slot number
   * @returns {Promise<Object|null>} Block or null
   */
  async getBlockBySlot(slot) {
    // Check cache first
    if (this._blockCache.has(slot)) {
      return this._blockCache.get(slot);
    }

    // Check in-memory chain
    const chainBlock = this._chain?.getBlockBySlot?.(slot);
    if (chainBlock) return chainBlock;

    // Load from storage
    try {
      const block = await this.storage.get(`block_${slot}`);
      if (block) {
        this._blockCache.set(slot, block);
      }
      return block || null;
    } catch {
      return null;
    }
  }

  /**
   * Get block by hash
   * @param {string} hash - Block hash
   * @returns {Promise<Object|null>} Block or null
   */
  async getBlockByHash(hash) {
    // Check in-memory chain first
    const chainBlock = this._chain?.getBlock?.(hash);
    if (chainBlock) return chainBlock;

    // Get slot from index
    try {
      const index = await this.storage.get('block_index');
      if (index?.hashes?.[hash]) {
        return this.getBlockBySlot(index.hashes[hash]);
      }
    } catch {
      // Fall through
    }

    return null;
  }

  /**
   * Get recent blocks from persistence
   * @param {number} [count=20] - Number of blocks
   * @returns {Promise<Object[]>} Recent blocks
   */
  async getRecentBlocks(count = 20) {
    // Get from in-memory chain first
    const chainBlocks = this._chain?.getRecentBlocks?.(count) || [];
    if (chainBlocks.length >= count) {
      return chainBlocks;
    }

    // Supplement from storage
    try {
      const index = await this.storage.get('block_index');
      if (!index?.slots?.length) return chainBlocks;

      const blocks = [];
      const slots = index.slots.slice(-count).reverse();

      for (const slot of slots) {
        const block = await this.getBlockBySlot(slot);
        if (block) blocks.push(block);
        if (blocks.length >= count) break;
      }

      return blocks;
    } catch {
      return chainBlocks;
    }
  }

  /**
   * Get block count from persistence
   * @returns {Promise<number>} Block count
   */
  async getBlockCount() {
    const chainHeight = this._chain?.getHeight?.() || 0;

    try {
      const index = await this.storage.get('block_index');
      return Math.max(chainHeight, index?.slots?.length || 0);
    } catch {
      return chainHeight;
    }
  }

  /**
   * Simple block hash for storage key
   * @private
   */
  _hashBlock(block) {
    const data = `${block.slot}:${block.timestamp}:${block.proposer || 'unknown'}`;
    let hash = 0;
    for (let i = 0; i < data.length; i++) {
      hash = ((hash << 5) - hash) + data.charCodeAt(i);
      hash |= 0;
    }
    return 'blk_' + Math.abs(hash).toString(16);
  }

  /**
   * Add peer
   * @param {Object} peer - Peer info
   */
  addPeer(peer) {
    this._peers.set(peer.id, peer);
  }

  /**
   * Remove peer
   * @param {string} peerId - Peer ID
   */
  removePeer(peerId) {
    this._peers.delete(peerId);
  }

  /**
   * Get peer
   * @param {string} peerId - Peer ID
   * @returns {Object|null} Peer info
   */
  getPeer(peerId) {
    return this._peers.get(peerId) || null;
  }

  /**
   * Get all peers
   * @returns {Object[]} All peers
   */
  getAllPeers() {
    return Array.from(this._peers.values());
  }

  /**
   * Save state to storage
   */
  async save() {
    const state = {
      chain: this._chain.export(),
      knowledge: this._knowledge.export(),
      peers: Object.fromEntries(this._peers),
      judgments: this._judgments.slice(-100), // Save last 100
      savedAt: Date.now(),
    };

    await this.storage.set('state', state);

    // Also save operator state
    if (this.operator) {
      await this.storage.set('operator', this.operator.export());
    }
  }

  /**
   * Get state summary
   * @returns {Object} State summary
   */
  getSummary() {
    return {
      chainHeight: this._chain?.getHeight() || 0,
      knowledgeStats: this._knowledge?.getStats() || {},
      peerCount: this._peers.size,
      judgmentCount: this._judgments.length,
      initialized: this.initialized,
    };
  }

  /**
   * Clear all state (use with caution!)
   */
  async clear() {
    await this.storage.clear();
    this._chain = null;
    this._knowledge = null;
    this._peers.clear();
    this._judgments = [];
    this.initialized = false;
  }
}

export default { StateManager };
