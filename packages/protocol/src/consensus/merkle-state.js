/**
 * Consensus Merkle State Tree
 *
 * Merkle tree for efficient O(log n) state diff detection during gossip sync.
 * Inspired by Claude Flow's gossip coordinator which uses "Merkle tree
 * comparison for efficient difference detection."
 *
 * Instead of sending full state (O(n)), peers exchange Merkle roots first.
 * If roots differ, only the diff is requested - achieving O(log n) efficiency.
 *
 * "Compare roots first, sync only what differs" - κυνικός
 *
 * @module @cynic/protocol/consensus/merkle-state
 */

'use strict';

import { MerkleTree } from '../merkle/tree.js';
import { sha256, hashObject } from '../crypto/hash.js';

/**
 * φ-aligned constants for Merkle operations
 */
const PHI_INV = 0.618; // φ⁻¹
const FIBONACCI = {
  F8: 21,   // Batch size for proof generation
  F10: 55,  // Default chunk size
  F11: 89,  // Max blocks per diff request
};

/**
 * Consensus State Merkle Tree
 *
 * Wraps finalized blocks in a Merkle tree for efficient sync.
 */
export class ConsensusStateTree {
  /**
   * @param {Object[]} [blocks=[]] - Initial finalized blocks
   */
  constructor(blocks = []) {
    this.blocks = new Map(); // slot -> block
    this.tree = null;
    this.cachedRoot = null;
    this.dirty = true;

    // Load initial blocks
    for (const block of blocks) {
      this.addBlock(block);
    }

    if (blocks.length > 0) {
      this._rebuild();
    }
  }

  /**
   * Add a finalized block to the state
   * @param {Object} block - Finalized block
   */
  addBlock(block) {
    const slot = block.slot ?? block.height ?? this.blocks.size;
    this.blocks.set(slot, block);
    this.dirty = true;
  }

  /**
   * Get block by slot
   * @param {number} slot - Slot number
   * @returns {Object|null} Block or null
   */
  getBlock(slot) {
    return this.blocks.get(slot) || null;
  }

  /**
   * Get all blocks since a slot
   * @param {number} sinceSlot - Starting slot (exclusive)
   * @param {number} [maxBlocks=89] - Max blocks to return (F11)
   * @returns {Object[]} Blocks after sinceSlot
   */
  getBlocksSince(sinceSlot, maxBlocks = FIBONACCI.F11) {
    const result = [];
    const slots = Array.from(this.blocks.keys()).sort((a, b) => a - b);

    for (const slot of slots) {
      if (slot > sinceSlot && result.length < maxBlocks) {
        result.push(this.blocks.get(slot));
      }
    }

    return result;
  }

  /**
   * Compute Merkle root of current state
   * @returns {string} Merkle root hash
   */
  getRoot() {
    if (this.dirty || !this.cachedRoot) {
      this._rebuild();
    }
    return this.cachedRoot;
  }

  /**
   * Get root info for broadcasting
   * @returns {Object} Root info with metadata
   */
  getRootInfo() {
    return {
      root: this.getRoot(),
      slot: this.getLatestSlot(),
      blockCount: this.blocks.size,
    };
  }

  /**
   * Get latest finalized slot
   * @returns {number} Latest slot or 0
   */
  getLatestSlot() {
    const slots = Array.from(this.blocks.keys());
    return slots.length > 0 ? Math.max(...slots) : 0;
  }

  /**
   * Rebuild Merkle tree from blocks
   * @private
   */
  _rebuild() {
    // Sort blocks by slot for deterministic tree
    const slots = Array.from(this.blocks.keys()).sort((a, b) => a - b);
    const sortedBlocks = slots.map((slot) => this.blocks.get(slot));

    // Build tree from block hashes
    const blockHashes = sortedBlocks.map((block) =>
      this._hashBlock(block)
    );

    if (blockHashes.length === 0) {
      this.cachedRoot = sha256('empty:consensus:state');
    } else {
      this.tree = new MerkleTree(blockHashes, (hash) => hash); // Items are already hashes
      this.cachedRoot = this.tree.getRoot();
    }

    this.dirty = false;
  }

  /**
   * Hash a block for tree inclusion
   * @private
   * @param {Object} block - Block to hash
   * @returns {string} Block hash
   */
  _hashBlock(block) {
    // Use block's existing hash if available
    if (block.hash) {
      return block.hash;
    }

    // Otherwise compute deterministic hash
    return hashObject({
      slot: block.slot ?? block.height,
      proposer: block.proposer,
      timestamp: block.timestamp,
      judgments: block.judgments?.map((j) => j.judgment_id || j.id) || [],
    });
  }

  /**
   * Compare roots to determine if sync needed
   * @param {string} otherRoot - Peer's Merkle root
   * @returns {boolean} True if roots differ (sync needed)
   */
  needsSync(otherRoot) {
    return this.getRoot() !== otherRoot;
  }

  /**
   * Get diff between our state and peer's claimed state
   *
   * Returns blocks that the peer is likely missing, based on
   * their reported sinceSlot.
   *
   * @param {number} sinceSlot - Peer's latest slot
   * @returns {Object} Diff info
   */
  getDiff(sinceSlot) {
    const blocks = this.getBlocksSince(sinceSlot);

    return {
      blocks,
      newRoot: this.getRoot(),
      latestSlot: this.getLatestSlot(),
      diffSize: blocks.length,
    };
  }

  /**
   * Get proof that a block is included in the state
   * @param {Object} block - Block to prove
   * @returns {Object|null} Proof or null
   */
  getProof(block) {
    if (this.dirty) {
      this._rebuild();
    }

    if (!this.tree) {
      return null;
    }

    const blockHash = this._hashBlock(block);
    const proof = this.tree.getProof(blockHash);

    if (!proof) {
      return null;
    }

    return {
      blockHash,
      proof,
      root: this.cachedRoot,
      slot: block.slot ?? block.height,
    };
  }

  /**
   * Verify proof of block inclusion
   * @param {Object} block - Block to verify
   * @param {Object[]} proof - Proof path
   * @param {string} root - Expected root
   * @returns {boolean} True if valid
   */
  static verifyProof(block, proof, root) {
    const blockHash = hashObject({
      slot: block.slot ?? block.height,
      proposer: block.proposer,
      timestamp: block.timestamp,
      judgments: block.judgments?.map((j) => j.judgment_id || j.id) || [],
    });

    return MerkleTree.verifyProof(blockHash, proof, root, (hash) => hash);
  }

  /**
   * Import blocks from peer diff response
   * @param {Object[]} blocks - Blocks to import
   * @returns {Object} Import result
   */
  importDiff(blocks) {
    let imported = 0;
    let skipped = 0;

    for (const block of blocks) {
      const slot = block.slot ?? block.height;

      // Skip if we already have this slot
      if (this.blocks.has(slot)) {
        skipped++;
        continue;
      }

      this.addBlock(block);
      imported++;
    }

    return {
      imported,
      skipped,
      newRoot: this.getRoot(),
      latestSlot: this.getLatestSlot(),
    };
  }

  /**
   * Export state for full sync (fallback when Merkle diff not possible)
   * @param {number} [sinceSlot=0] - Starting slot
   * @param {number} [maxBlocks=55] - Max blocks (F10)
   * @returns {Object} Exportable state
   */
  export(sinceSlot = 0, maxBlocks = FIBONACCI.F10) {
    return {
      blocks: this.getBlocksSince(sinceSlot, maxBlocks),
      root: this.getRoot(),
      latestSlot: this.getLatestSlot(),
      blockCount: this.blocks.size,
    };
  }

  /**
   * Get statistics
   * @returns {Object} Tree stats
   */
  getStats() {
    return {
      blockCount: this.blocks.size,
      latestSlot: this.getLatestSlot(),
      root: this.getRoot(),
      treeDepth: this.blocks.size > 0 ? Math.ceil(Math.log2(this.blocks.size)) : 0,
    };
  }
}

/**
 * Create state tree from consensus engine's exported state
 * @param {Object} state - Exported consensus state
 * @returns {ConsensusStateTree} State tree
 */
export function createStateTree(state) {
  return new ConsensusStateTree(state.blocks || []);
}

export default {
  ConsensusStateTree,
  createStateTree,
};
