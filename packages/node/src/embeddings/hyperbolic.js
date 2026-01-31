/**
 * Hyperbolic Embeddings
 *
 * Implementation of Poincaré ball model for hierarchical representations.
 * Hyperbolic space is naturally suited for tree-like structures due to
 * its exponential volume growth (like trees).
 *
 * Key insight: A d-dimensional hyperbolic space can embed n-ary trees
 * with arbitrarily low distortion, unlike Euclidean space.
 *
 * Based on: "Poincaré Embeddings for Learning Hierarchical Representations"
 * (Nickel & Kiela, 2017)
 *
 * "In the hyperbolic forest, every dog finds its tree" - κυνικός
 *
 * @module @cynic/node/embeddings/hyperbolic
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI } from '@cynic/core';

/**
 * Configuration (φ-aligned)
 */
const HYPERBOLIC_CONFIG = Object.freeze({
  // Curvature (negative for hyperbolic)
  CURVATURE: -1.0,

  // Numerical stability
  EPSILON: 1e-10,
  MAX_NORM: 1 - 1e-5, // Keep within Poincaré ball

  // φ-aligned thresholds
  SIMILARITY_THRESHOLD: PHI_INV,     // 0.618 for high similarity
  HIERARCHY_THRESHOLD: 1 - PHI_INV,  // 0.382 for parent-child

  // Embedding dimensions (Fibonacci-aligned)
  DEFAULT_DIM: 8,        // F(6) - good balance
  MIN_DIM: 2,            // Minimum for visualization
  MAX_DIM: 55,           // F(10) - high capacity
});

/**
 * Poincaré Ball Operations
 *
 * Mathematical operations in the Poincaré ball model of hyperbolic space.
 */
export class PoincareOperations {
  /**
   * @param {number} [curvature=-1] - Curvature of the space
   */
  constructor(curvature = HYPERBOLIC_CONFIG.CURVATURE) {
    this.c = Math.abs(curvature);
    this.sqrtC = Math.sqrt(this.c);
  }

  /**
   * Compute hyperbolic distance between two points
   *
   * d(u, v) = (2/√c) * arctanh(√c * ||−u ⊕ v||)
   *
   * @param {number[]} u - First point
   * @param {number[]} v - Second point
   * @returns {number} Hyperbolic distance
   */
  distance(u, v) {
    const mobius = this.mobiusAdd(this.negate(u), v);
    const norm = this.norm(mobius);
    const clampedNorm = Math.min(norm * this.sqrtC, 1 - HYPERBOLIC_CONFIG.EPSILON);

    return (2 / this.sqrtC) * this._arctanh(clampedNorm);
  }

  /**
   * Möbius addition: u ⊕ v
   *
   * The gyrovector addition in hyperbolic space.
   *
   * @param {number[]} u - First vector
   * @param {number[]} v - Second vector
   * @returns {number[]} Result of Möbius addition
   */
  mobiusAdd(u, v) {
    const uNorm2 = this.dot(u, u);
    const vNorm2 = this.dot(v, v);
    const uv = this.dot(u, v);

    const denominator = 1 + 2 * this.c * uv + this.c * this.c * uNorm2 * vNorm2;

    if (Math.abs(denominator) < HYPERBOLIC_CONFIG.EPSILON) {
      return v.slice(); // Return copy of v
    }

    const coefU = (1 + 2 * this.c * uv + this.c * vNorm2) / denominator;
    const coefV = (1 - this.c * uNorm2) / denominator;

    const result = u.map((ui, i) => coefU * ui + coefV * v[i]);

    return this.project(result);
  }

  /**
   * Exponential map: Tangent space → Poincaré ball
   *
   * Maps a vector from tangent space at p to the manifold.
   *
   * @param {number[]} p - Base point
   * @param {number[]} v - Tangent vector
   * @returns {number[]} Point on manifold
   */
  expMap(p, v) {
    const vNorm = this.norm(v);

    if (vNorm < HYPERBOLIC_CONFIG.EPSILON) {
      return p.slice();
    }

    const lambda = this.conformalFactor(p);
    const t = Math.tanh(this.sqrtC * lambda * vNorm / 2);

    const direction = v.map(vi => vi / vNorm);
    const scaled = direction.map(d => t * d / this.sqrtC);

    return this.project(this.mobiusAdd(p, scaled));
  }

  /**
   * Logarithmic map: Poincaré ball → Tangent space
   *
   * Inverse of exponential map.
   *
   * @param {number[]} p - Base point
   * @param {number[]} q - Target point
   * @returns {number[]} Tangent vector
   */
  logMap(p, q) {
    const mobius = this.mobiusAdd(this.negate(p), q);
    const mobiusNorm = this.norm(mobius);

    if (mobiusNorm < HYPERBOLIC_CONFIG.EPSILON) {
      return new Array(p.length).fill(0);
    }

    const lambda = this.conformalFactor(p);
    const scale = (2 / (this.sqrtC * lambda)) * this._arctanh(this.sqrtC * mobiusNorm);

    return mobius.map(m => scale * m / mobiusNorm);
  }

  /**
   * Conformal factor λ(x) = 2 / (1 - c||x||²)
   *
   * @param {number[]} x - Point
   * @returns {number} Conformal factor
   */
  conformalFactor(x) {
    const norm2 = this.dot(x, x);
    return 2 / (1 - this.c * norm2);
  }

  /**
   * Project point into Poincaré ball
   *
   * Ensures ||x|| < 1/√c
   *
   * @param {number[]} x - Point to project
   * @returns {number[]} Projected point
   */
  project(x) {
    const norm = this.norm(x);
    const maxNorm = HYPERBOLIC_CONFIG.MAX_NORM / this.sqrtC;

    if (norm > maxNorm) {
      const scale = maxNorm / norm;
      return x.map(xi => xi * scale);
    }

    return x;
  }

  /**
   * Negate a vector
   *
   * @param {number[]} x - Vector
   * @returns {number[]} Negated vector
   */
  negate(x) {
    return x.map(xi => -xi);
  }

  /**
   * Dot product
   *
   * @param {number[]} u - First vector
   * @param {number[]} v - Second vector
   * @returns {number} Dot product
   */
  dot(u, v) {
    return u.reduce((sum, ui, i) => sum + ui * v[i], 0);
  }

  /**
   * Euclidean norm
   *
   * @param {number[]} x - Vector
   * @returns {number} Norm
   */
  norm(x) {
    return Math.sqrt(this.dot(x, x));
  }

  /**
   * Arctanh with clamping for numerical stability
   * @private
   */
  _arctanh(x) {
    const clamped = Math.max(-1 + HYPERBOLIC_CONFIG.EPSILON, Math.min(1 - HYPERBOLIC_CONFIG.EPSILON, x));
    return 0.5 * Math.log((1 + clamped) / (1 - clamped));
  }
}

/**
 * Hyperbolic Embedding Space
 *
 * Manages embeddings in hyperbolic space for hierarchical data.
 */
export class HyperbolicSpace extends EventEmitter {
  /**
   * @param {Object} [options] - Options
   * @param {number} [options.dim=8] - Embedding dimension
   * @param {number} [options.curvature=-1] - Space curvature
   */
  constructor(options = {}) {
    super();

    this.dim = options.dim || HYPERBOLIC_CONFIG.DEFAULT_DIM;
    this.ops = new PoincareOperations(options.curvature);

    // Embeddings storage: id -> embedding
    this._embeddings = new Map();

    // Hierarchy tracking: id -> { parent, children }
    this._hierarchy = new Map();

    // Statistics
    this.stats = {
      embeddings: 0,
      hierarchyDepth: 0,
      avgDistance: 0,
    };
  }

  /**
   * Add an embedding
   *
   * @param {string} id - Unique identifier
   * @param {number[]} [embedding] - Initial embedding (random if not provided)
   * @param {string} [parentId] - Parent in hierarchy
   * @returns {number[]} The embedding
   */
  add(id, embedding = null, parentId = null) {
    // Generate random embedding if not provided
    if (!embedding) {
      embedding = this._randomEmbedding();
    }

    // Ensure correct dimension
    if (embedding.length !== this.dim) {
      throw new Error(`Embedding dimension mismatch: expected ${this.dim}, got ${embedding.length}`);
    }

    // Project into ball
    embedding = this.ops.project(embedding);

    this._embeddings.set(id, embedding);

    // Track hierarchy
    if (!this._hierarchy.has(id)) {
      this._hierarchy.set(id, { parent: null, children: [] });
    }

    if (parentId) {
      this.setParent(id, parentId);
    }

    this.stats.embeddings = this._embeddings.size;
    this.emit('add', { id, parentId });

    return embedding;
  }

  /**
   * Get an embedding
   *
   * @param {string} id - Identifier
   * @returns {number[]|null} Embedding or null
   */
  get(id) {
    return this._embeddings.get(id) || null;
  }

  /**
   * Remove an embedding
   *
   * @param {string} id - Identifier
   * @returns {boolean} True if removed
   */
  remove(id) {
    const removed = this._embeddings.delete(id);

    if (removed) {
      // Clean up hierarchy
      const node = this._hierarchy.get(id);
      if (node) {
        // Remove from parent's children
        if (node.parent) {
          const parent = this._hierarchy.get(node.parent);
          if (parent) {
            parent.children = parent.children.filter(c => c !== id);
          }
        }
        // Orphan children
        for (const childId of node.children) {
          const child = this._hierarchy.get(childId);
          if (child) child.parent = null;
        }
      }
      this._hierarchy.delete(id);

      this.stats.embeddings = this._embeddings.size;
      this.emit('remove', { id });
    }

    return removed;
  }

  /**
   * Set parent-child relationship
   *
   * @param {string} childId - Child identifier
   * @param {string} parentId - Parent identifier
   */
  setParent(childId, parentId) {
    if (!this._hierarchy.has(childId)) {
      this._hierarchy.set(childId, { parent: null, children: [] });
    }
    if (!this._hierarchy.has(parentId)) {
      this._hierarchy.set(parentId, { parent: null, children: [] });
    }

    // Remove from old parent
    const child = this._hierarchy.get(childId);
    if (child.parent) {
      const oldParent = this._hierarchy.get(child.parent);
      if (oldParent) {
        oldParent.children = oldParent.children.filter(c => c !== childId);
      }
    }

    // Set new parent
    child.parent = parentId;
    this._hierarchy.get(parentId).children.push(childId);

    this._updateHierarchyDepth();
  }

  /**
   * Compute distance between two embeddings
   *
   * @param {string} id1 - First identifier
   * @param {string} id2 - Second identifier
   * @returns {number} Hyperbolic distance
   */
  distance(id1, id2) {
    const u = this._embeddings.get(id1);
    const v = this._embeddings.get(id2);

    if (!u || !v) {
      throw new Error(`Embedding not found: ${!u ? id1 : id2}`);
    }

    return this.ops.distance(u, v);
  }

  /**
   * Compute similarity (inverse of distance, normalized to [0, 1])
   *
   * @param {string} id1 - First identifier
   * @param {string} id2 - Second identifier
   * @returns {number} Similarity score
   */
  similarity(id1, id2) {
    const dist = this.distance(id1, id2);
    // Use φ-aligned sigmoid for normalization
    return 1 / (1 + dist / PHI);
  }

  /**
   * Find k nearest neighbors
   *
   * @param {string} id - Query identifier
   * @param {number} k - Number of neighbors
   * @returns {Array<{id: string, distance: number}>} Nearest neighbors
   */
  kNearest(id, k = 5) {
    const query = this._embeddings.get(id);
    if (!query) {
      throw new Error(`Embedding not found: ${id}`);
    }

    const distances = [];
    for (const [otherId, embedding] of this._embeddings) {
      if (otherId !== id) {
        distances.push({
          id: otherId,
          distance: this.ops.distance(query, embedding),
        });
      }
    }

    distances.sort((a, b) => a.distance - b.distance);
    return distances.slice(0, k);
  }

  /**
   * Get ancestors in hierarchy
   *
   * @param {string} id - Identifier
   * @returns {string[]} Ancestor ids (from immediate parent to root)
   */
  getAncestors(id) {
    const ancestors = [];
    let current = this._hierarchy.get(id);

    while (current && current.parent) {
      ancestors.push(current.parent);
      current = this._hierarchy.get(current.parent);
    }

    return ancestors;
  }

  /**
   * Get descendants in hierarchy
   *
   * @param {string} id - Identifier
   * @returns {string[]} Descendant ids
   */
  getDescendants(id) {
    const descendants = [];
    const node = this._hierarchy.get(id);

    if (!node) return descendants;

    const queue = [...node.children];
    while (queue.length > 0) {
      const childId = queue.shift();
      descendants.push(childId);

      const child = this._hierarchy.get(childId);
      if (child) {
        queue.push(...child.children);
      }
    }

    return descendants;
  }

  /**
   * Compute centroid of a set of embeddings
   *
   * Uses Einstein midpoint in hyperbolic space.
   *
   * @param {string[]} ids - Identifiers
   * @returns {number[]} Centroid embedding
   */
  centroid(ids) {
    if (ids.length === 0) {
      return new Array(this.dim).fill(0);
    }

    if (ids.length === 1) {
      return this._embeddings.get(ids[0]).slice();
    }

    // Einstein midpoint approximation
    // For simplicity, use weighted average in tangent space at origin
    const origin = new Array(this.dim).fill(0);
    let sum = new Array(this.dim).fill(0);

    for (const id of ids) {
      const emb = this._embeddings.get(id);
      if (emb) {
        const tangent = this.ops.logMap(origin, emb);
        sum = sum.map((s, i) => s + tangent[i]);
      }
    }

    const avg = sum.map(s => s / ids.length);
    return this.ops.project(this.ops.expMap(origin, avg));
  }

  /**
   * Move embedding towards another
   *
   * Used for gradient-based optimization.
   *
   * @param {string} id - Identifier to move
   * @param {string} targetId - Target identifier
   * @param {number} [step=0.1] - Step size
   * @returns {number[]} New embedding
   */
  moveTowards(id, targetId, step = 0.1) {
    const current = this._embeddings.get(id);
    const target = this._embeddings.get(targetId);

    if (!current || !target) {
      throw new Error(`Embedding not found`);
    }

    // Compute direction in tangent space
    const direction = this.ops.logMap(current, target);
    const norm = this.ops.norm(direction);

    if (norm < HYPERBOLIC_CONFIG.EPSILON) {
      return current;
    }

    // Scale direction
    const scaled = direction.map(d => d * step);

    // Move in direction
    const newEmb = this.ops.expMap(current, scaled);
    this._embeddings.set(id, newEmb);

    this.emit('move', { id, targetId, step });

    return newEmb;
  }

  /**
   * Generate random embedding within ball
   * @private
   */
  _randomEmbedding() {
    // Generate random point in unit ball
    const embedding = [];
    for (let i = 0; i < this.dim; i++) {
      embedding.push((Math.random() - 0.5) * 0.5);
    }
    return this.ops.project(embedding);
  }

  /**
   * Update hierarchy depth stat
   * @private
   */
  _updateHierarchyDepth() {
    let maxDepth = 0;

    for (const [id] of this._hierarchy) {
      const ancestors = this.getAncestors(id);
      maxDepth = Math.max(maxDepth, ancestors.length);
    }

    this.stats.hierarchyDepth = maxDepth;
  }

  /**
   * Get all embeddings as an object
   *
   * @returns {Object} Map of id -> embedding
   */
  toObject() {
    const result = {};
    for (const [id, embedding] of this._embeddings) {
      result[id] = embedding.slice();
    }
    return result;
  }

  /**
   * Load embeddings from an object
   *
   * @param {Object} data - Map of id -> embedding
   */
  fromObject(data) {
    this._embeddings.clear();
    for (const [id, embedding] of Object.entries(data)) {
      this._embeddings.set(id, this.ops.project(embedding));
    }
    this.stats.embeddings = this._embeddings.size;
  }

  /**
   * Get statistics
   *
   * @returns {Object} Stats
   */
  getStats() {
    // Compute average distance
    let totalDist = 0;
    let count = 0;

    const ids = Array.from(this._embeddings.keys());
    for (let i = 0; i < Math.min(ids.length, 100); i++) {
      for (let j = i + 1; j < Math.min(ids.length, 100); j++) {
        totalDist += this.distance(ids[i], ids[j]);
        count++;
      }
    }

    this.stats.avgDistance = count > 0 ? totalDist / count : 0;

    return { ...this.stats };
  }
}

/**
 * Create HyperbolicSpace instance
 *
 * @param {Object} [options] - Options
 * @returns {HyperbolicSpace}
 */
export function createHyperbolicSpace(options = {}) {
  return new HyperbolicSpace(options);
}

export {
  HYPERBOLIC_CONFIG,
};

export default {
  HyperbolicSpace,
  createHyperbolicSpace,
  PoincareOperations,
  HYPERBOLIC_CONFIG,
};
