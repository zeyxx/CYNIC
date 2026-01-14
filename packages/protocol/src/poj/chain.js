/**
 * Proof of Judgment - Chain Management
 *
 * SHA-256 chain for verifiable judgment history
 *
 * @module @cynic/protocol/poj/chain
 */

'use strict';

import {
  createGenesisBlock,
  createJudgmentBlock,
  createKnowledgeBlock,
  createGovernanceBlock,
  hashBlock,
  validateBlockChain,
  calculateSlot,
  BlockType,
} from './block.js';

/**
 * PoJ Chain - Manages sequence of judgment blocks
 */
export class PoJChain {
  /**
   * Create new chain
   * @param {Object} params - Chain parameters
   * @param {string} params.operatorPublicKey - Operator public key
   * @param {string} params.operatorPrivateKey - Operator private key
   * @param {number} [params.genesisTime] - Genesis timestamp
   */
  constructor({ operatorPublicKey, operatorPrivateKey, genesisTime }) {
    this.operatorPublicKey = operatorPublicKey;
    this.operatorPrivateKey = operatorPrivateKey;
    this.genesisTime = genesisTime || Date.now();

    // Initialize with genesis
    this.blocks = [];
    this.blocksByHash = new Map();
    this.head = null;

    this._initGenesis();
  }

  /**
   * Initialize genesis block
   * @private
   */
  _initGenesis() {
    const genesis = createGenesisBlock(
      this.operatorPublicKey,
      this.operatorPrivateKey,
      this.genesisTime
    );
    this._addBlock(genesis);
  }

  /**
   * Add block to chain (internal)
   * @private
   * @param {Object} block - Block to add
   */
  _addBlock(block) {
    const hash = hashBlock(block);
    this.blocks.push(block);
    this.blocksByHash.set(hash, block);
    this.head = block;
  }

  /**
   * Get current slot
   * @returns {number} Current slot number
   */
  getCurrentSlot() {
    return calculateSlot(Date.now(), this.genesisTime);
  }

  /**
   * Get next available slot (at least one more than head)
   * @returns {number} Next slot number
   */
  getNextSlot() {
    const currentSlot = this.getCurrentSlot();
    const minSlot = this.head ? this.head.slot + 1 : 0;
    return Math.max(currentSlot, minSlot);
  }

  /**
   * Get next valid timestamp (must be greater than head)
   * @returns {number} Next timestamp
   */
  getNextTimestamp() {
    const now = Date.now();
    const minTimestamp = this.head ? this.head.timestamp + 1 : now;
    return Math.max(now, minTimestamp);
  }

  /**
   * Get head block
   * @returns {Object} Head block
   */
  getHead() {
    return this.head;
  }

  /**
   * Get head block hash
   * @returns {string} Head block hash
   */
  getHeadHash() {
    return hashBlock(this.head);
  }

  /**
   * Get block by hash
   * @param {string} hash - Block hash
   * @returns {Object|null} Block or null
   */
  getBlock(hash) {
    return this.blocksByHash.get(hash) || null;
  }

  /**
   * Get block by slot
   * @param {number} slot - Slot number
   * @returns {Object|null} Block or null
   */
  getBlockBySlot(slot) {
    return this.blocks.find((b) => b.slot === slot) || null;
  }

  /**
   * Get chain height
   * @returns {number} Number of blocks
   */
  getHeight() {
    return this.blocks.length;
  }

  /**
   * Add judgment block to chain
   * @param {Object[]} judgments - Array of judgments
   * @param {string} [stateRoot] - Current state root
   * @returns {Object} Created block
   */
  addJudgmentBlock(judgments, stateRoot = null) {
    const slot = this.getNextSlot();
    const prevHash = this.getHeadHash();
    const timestamp = this.getNextTimestamp();

    const block = createJudgmentBlock({
      slot,
      prevHash,
      judgments,
      operatorPublicKey: this.operatorPublicKey,
      operatorPrivateKey: this.operatorPrivateKey,
      stateRoot,
      timestamp,
    });

    // Validate before adding
    const validation = validateBlockChain(block, this.head);
    if (!validation.valid) {
      throw new Error(`Invalid block: ${validation.errors.join(', ')}`);
    }

    this._addBlock(block);
    return block;
  }

  /**
   * Add knowledge block to chain
   * @param {Object[]} patterns - Array of patterns
   * @param {Object[]} learnings - Array of learnings
   * @returns {Object} Created block
   */
  addKnowledgeBlock(patterns, learnings) {
    const slot = this.getNextSlot();
    const prevHash = this.getHeadHash();
    const timestamp = this.getNextTimestamp();

    const block = createKnowledgeBlock({
      slot,
      prevHash,
      patterns,
      learnings,
      operatorPublicKey: this.operatorPublicKey,
      operatorPrivateKey: this.operatorPrivateKey,
      timestamp,
    });

    const validation = validateBlockChain(block, this.head);
    if (!validation.valid) {
      throw new Error(`Invalid block: ${validation.errors.join(', ')}`);
    }

    this._addBlock(block);
    return block;
  }

  /**
   * Add governance block to chain
   * @param {Object} proposal - Governance proposal
   * @param {Object[]} votes - Array of votes
   * @returns {Object} Created block
   */
  addGovernanceBlock(proposal, votes) {
    const slot = this.getNextSlot();
    const prevHash = this.getHeadHash();
    const timestamp = this.getNextTimestamp();

    const block = createGovernanceBlock({
      slot,
      prevHash,
      proposal,
      votes,
      operatorPublicKey: this.operatorPublicKey,
      operatorPrivateKey: this.operatorPrivateKey,
      timestamp,
    });

    const validation = validateBlockChain(block, this.head);
    if (!validation.valid) {
      throw new Error(`Invalid block: ${validation.errors.join(', ')}`);
    }

    this._addBlock(block);
    return block;
  }

  /**
   * Import external block (from gossip)
   * @param {Object} block - Block to import
   * @returns {{ success: boolean, errors?: string[] }} Import result
   */
  importBlock(block) {
    const validation = validateBlockChain(block, this.head);
    if (!validation.valid) {
      return { success: false, errors: validation.errors };
    }

    this._addBlock(block);
    return { success: true };
  }

  /**
   * Get blocks since slot
   * @param {number} slot - Starting slot
   * @returns {Object[]} Blocks since slot
   */
  getBlocksSince(slot) {
    return this.blocks.filter((b) => b.slot > slot);
  }

  /**
   * Get recent blocks
   * @param {number} [count=10] - Number of blocks
   * @returns {Object[]} Recent blocks
   */
  getRecentBlocks(count = 10) {
    return this.blocks.slice(-count);
  }

  /**
   * Verify chain integrity
   * @returns {{ valid: boolean, errors: string[] }} Verification result
   */
  verifyIntegrity() {
    const errors = [];

    for (let i = 1; i < this.blocks.length; i++) {
      const block = this.blocks[i];
      const prevBlock = this.blocks[i - 1];
      const validation = validateBlockChain(block, prevBlock);

      if (!validation.valid) {
        errors.push(`Block ${i} (slot ${block.slot}): ${validation.errors.join(', ')}`);
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * Export chain state for serialization
   * @returns {Object} Chain state
   */
  export() {
    return {
      genesisTime: this.genesisTime,
      operator: this.operatorPublicKey,
      blocks: this.blocks,
    };
  }

  /**
   * Import chain state
   * @param {Object} state - Chain state
   * @param {string} operatorPrivateKey - Operator private key
   * @returns {PoJChain} New chain instance
   */
  static import(state, operatorPrivateKey) {
    const chain = new PoJChain({
      operatorPublicKey: state.operator,
      operatorPrivateKey,
      genesisTime: state.genesisTime,
    });

    // Clear auto-generated genesis and import
    chain.blocks = [];
    chain.blocksByHash.clear();

    for (const block of state.blocks) {
      chain._addBlock(block);
    }

    return chain;
  }
}

export default { PoJChain };
