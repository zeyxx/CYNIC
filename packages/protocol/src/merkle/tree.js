/**
 * Merkle Knowledge Tree
 *
 * Merkle tree for pattern and learning storage
 * Partitioned by axiom (PHI, VERIFY, CULTURE, BURN)
 *
 * @module @cynic/protocol/merkle/tree
 */

'use strict';

import { AXIOMS } from '@cynic/core';
import { sha256, sha256Prefixed, hashObject } from '../crypto/hash.js';

/**
 * Merkle node
 */
class MerkleNode {
  /**
   * @param {string} hash - Node hash
   * @param {MerkleNode|null} left - Left child
   * @param {MerkleNode|null} right - Right child
   * @param {*} data - Leaf data (only for leaves)
   */
  constructor(hash, left = null, right = null, data = null) {
    this.hash = hash;
    this.left = left;
    this.right = right;
    this.data = data;
  }

  isLeaf() {
    return this.left === null && this.right === null;
  }
}

/**
 * Merkle Tree implementation
 */
export class MerkleTree {
  /**
   * Create merkle tree from data
   * @param {*[]} items - Array of items
   * @param {Function} [hashFn] - Hash function for items
   */
  constructor(items = [], hashFn = hashObject) {
    this.hashFn = hashFn;
    this.leaves = [];
    this.root = null;

    if (items.length > 0) {
      this.build(items);
    }
  }

  /**
   * Build tree from items
   * @param {*[]} items - Array of items
   */
  build(items) {
    // Create leaf nodes
    this.leaves = items.map((item) => {
      const hash = this.hashFn(item);
      return new MerkleNode(hash, null, null, item);
    });

    if (this.leaves.length === 0) {
      this.root = new MerkleNode(sha256('empty'));
      return;
    }

    // Build tree bottom-up
    this.root = this._buildLevel(this.leaves);
  }

  /**
   * Build tree level recursively
   * @private
   * @param {MerkleNode[]} nodes - Current level nodes
   * @returns {MerkleNode} Parent node
   */
  _buildLevel(nodes) {
    if (nodes.length === 1) {
      return nodes[0];
    }

    const parents = [];

    for (let i = 0; i < nodes.length; i += 2) {
      const left = nodes[i];
      const right = nodes[i + 1] || left; // Duplicate if odd

      const hash = sha256(`${left.hash}:${right.hash}`);
      parents.push(new MerkleNode(hash, left, right));
    }

    return this._buildLevel(parents);
  }

  /**
   * Get root hash
   * @returns {string} Root hash
   */
  getRoot() {
    return this.root ? this.root.hash : sha256('empty');
  }

  /**
   * Get root hash with prefix
   * @returns {string} Prefixed root hash
   */
  getRootPrefixed() {
    return sha256Prefixed(this.getRoot());
  }

  /**
   * Get proof of inclusion for item
   * @param {*} item - Item to prove
   * @returns {string[]|null} Proof path or null if not found
   */
  getProof(item) {
    const targetHash = this.hashFn(item);
    const leafIndex = this.leaves.findIndex((l) => l.hash === targetHash);

    if (leafIndex === -1) {
      return null;
    }

    return this._buildProof(leafIndex, this.leaves.length);
  }

  /**
   * Build proof for leaf at index
   * @private
   * @param {number} index - Leaf index
   * @param {number} levelSize - Current level size
   * @returns {string[]} Proof path
   */
  _buildProof(index, levelSize) {
    if (levelSize === 1) {
      return [];
    }

    const proof = [];
    let currentNodes = [...this.leaves];
    let currentIndex = index;

    while (currentNodes.length > 1) {
      const siblingIndex = currentIndex % 2 === 0 ? currentIndex + 1 : currentIndex - 1;
      const sibling = currentNodes[siblingIndex] || currentNodes[currentIndex];

      proof.push({
        hash: sibling.hash,
        position: currentIndex % 2 === 0 ? 'right' : 'left',
      });

      // Build next level
      const nextLevel = [];
      for (let i = 0; i < currentNodes.length; i += 2) {
        const left = currentNodes[i];
        const right = currentNodes[i + 1] || left;
        const hash = sha256(`${left.hash}:${right.hash}`);
        nextLevel.push(new MerkleNode(hash, left, right));
      }

      currentNodes = nextLevel;
      currentIndex = Math.floor(currentIndex / 2);
    }

    return proof;
  }

  /**
   * Verify proof of inclusion
   * @param {*} item - Item to verify
   * @param {Object[]} proof - Proof path
   * @param {string} root - Expected root hash
   * @returns {boolean} True if valid
   */
  static verifyProof(item, proof, root, hashFn = hashObject) {
    let currentHash = hashFn(item);

    for (const step of proof) {
      if (step.position === 'right') {
        currentHash = sha256(`${currentHash}:${step.hash}`);
      } else {
        currentHash = sha256(`${step.hash}:${currentHash}`);
      }
    }

    return currentHash === root;
  }

  /**
   * Add item to tree (rebuilds)
   * @param {*} item - Item to add
   */
  add(item) {
    const items = this.leaves.map((l) => l.data);
    items.push(item);
    this.build(items);
  }

  /**
   * Get all items
   * @returns {*[]} All items
   */
  getItems() {
    return this.leaves.map((l) => l.data);
  }

  /**
   * Get tree size
   * @returns {number} Number of leaves
   */
  size() {
    return this.leaves.length;
  }
}

/**
 * Knowledge Tree - Merkle tree partitioned by axiom
 */
export class KnowledgeTree {
  constructor() {
    // One tree per axiom
    this.trees = {
      PHI: { patterns: new MerkleTree(), learnings: new MerkleTree() },
      VERIFY: { patterns: new MerkleTree(), learnings: new MerkleTree() },
      CULTURE: { patterns: new MerkleTree(), learnings: new MerkleTree() },
      BURN: { patterns: new MerkleTree(), learnings: new MerkleTree() },
    };
  }

  /**
   * Add pattern to tree
   * @param {Object} pattern - Pattern to add
   * @param {string} pattern.axiom - Axiom (PHI, VERIFY, CULTURE, BURN)
   */
  addPattern(pattern) {
    const axiom = pattern.axiom || 'VERIFY';
    if (!this.trees[axiom]) {
      throw new Error(`Invalid axiom: ${axiom}`);
    }
    this.trees[axiom].patterns.add(pattern);
  }

  /**
   * Add learning to tree
   * @param {Object} learning - Learning to add
   * @param {string} learning.axiom - Axiom
   */
  addLearning(learning) {
    const axiom = learning.axiom || 'VERIFY';
    if (!this.trees[axiom]) {
      throw new Error(`Invalid axiom: ${axiom}`);
    }
    this.trees[axiom].learnings.add(learning);
  }

  /**
   * Get patterns by axiom
   * @param {string} axiom - Axiom name
   * @returns {Object[]} Patterns
   */
  getPatterns(axiom) {
    if (!this.trees[axiom]) {
      throw new Error(`Invalid axiom: ${axiom}`);
    }
    return this.trees[axiom].patterns.getItems();
  }

  /**
   * Get learnings by axiom
   * @param {string} axiom - Axiom name
   * @returns {Object[]} Learnings
   */
  getLearnings(axiom) {
    if (!this.trees[axiom]) {
      throw new Error(`Invalid axiom: ${axiom}`);
    }
    return this.trees[axiom].learnings.getItems();
  }

  /**
   * Get all patterns
   * @returns {Object[]} All patterns
   */
  getAllPatterns() {
    return Object.values(this.trees).flatMap((t) => t.patterns.getItems());
  }

  /**
   * Get all learnings
   * @returns {Object[]} All learnings
   */
  getAllLearnings() {
    return Object.values(this.trees).flatMap((t) => t.learnings.getItems());
  }

  /**
   * Get root hash for axiom
   * @param {string} axiom - Axiom name
   * @returns {Object} Root hashes for patterns and learnings
   */
  getAxiomRoot(axiom) {
    if (!this.trees[axiom]) {
      throw new Error(`Invalid axiom: ${axiom}`);
    }
    return {
      patterns: this.trees[axiom].patterns.getRoot(),
      learnings: this.trees[axiom].learnings.getRoot(),
    };
  }

  /**
   * Get global root (combines all axiom roots)
   * @returns {string} Global root hash
   */
  getGlobalRoot() {
    const axiomRoots = Object.entries(this.trees).map(([axiom, tree]) => {
      return sha256(`${axiom}:${tree.patterns.getRoot()}:${tree.learnings.getRoot()}`);
    });

    return sha256(axiomRoots.join(':'));
  }

  /**
   * Get proof for pattern
   * @param {Object} pattern - Pattern to prove
   * @returns {Object|null} Proof or null
   */
  getPatternProof(pattern) {
    const axiom = pattern.axiom || 'VERIFY';
    if (!this.trees[axiom]) return null;

    const proof = this.trees[axiom].patterns.getProof(pattern);
    if (!proof) return null;

    return {
      axiom,
      type: 'pattern',
      proof,
      axiomRoot: this.trees[axiom].patterns.getRoot(),
      globalRoot: this.getGlobalRoot(),
    };
  }

  /**
   * Get statistics
   * @returns {Object} Tree statistics
   */
  getStats() {
    const stats = {};
    for (const [axiom, tree] of Object.entries(this.trees)) {
      stats[axiom] = {
        patterns: tree.patterns.size(),
        learnings: tree.learnings.size(),
      };
    }
    return stats;
  }

  /**
   * Export tree state
   * @returns {Object} Serializable state
   */
  export() {
    const state = {};
    for (const [axiom, tree] of Object.entries(this.trees)) {
      state[axiom] = {
        patterns: tree.patterns.getItems(),
        learnings: tree.learnings.getItems(),
      };
    }
    return {
      globalRoot: this.getGlobalRoot(),
      axioms: state,
    };
  }

  /**
   * Import tree state
   * @param {Object} state - State to import
   * @returns {KnowledgeTree} New instance
   */
  static import(state) {
    const tree = new KnowledgeTree();
    for (const [axiom, data] of Object.entries(state.axioms || {})) {
      if (tree.trees[axiom]) {
        tree.trees[axiom].patterns = new MerkleTree(data.patterns || []);
        tree.trees[axiom].learnings = new MerkleTree(data.learnings || []);
      }
    }
    return tree;
  }
}

export default {
  MerkleTree,
  KnowledgeTree,
};
