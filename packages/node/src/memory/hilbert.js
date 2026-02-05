/**
 * CYNIC Hilbert Curve Memory Indexing
 *
 * Space-filling curves for locality-preserving memory organization.
 * "La mémoire a une géométrie" - κυνικός
 *
 * Properties:
 * - Points close in N-D space → close on 1D curve
 * - Excellent cache locality
 * - Efficient range queries
 * - φ-aligned order selection
 *
 * @module @cynic/node/memory/hilbert
 */

'use strict';

import { PHI_INV, PHI_INV_2, createLogger } from '@cynic/core';

const log = createLogger('HilbertMemory');

// ═══════════════════════════════════════════════════════════════════════════
// HILBERT CURVE CORE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Hilbert curve order (resolution)
 * φ-aligned: order 8 gives 2^8 = 256 cells per dimension
 * Total cells in 2D: 256² = 65536 (≈ Fib(24))
 */
export const HILBERT_CONFIG = {
  DEFAULT_ORDER: 8,        // 2^8 = 256 resolution
  MAX_ORDER: 16,           // 2^16 = 65536 max resolution
  DIMENSIONS: 2,           // Start with 2D (can extend to N-D)
};

/**
 * Convert (x, y) coordinates to Hilbert curve index (d)
 *
 * Uses iterative algorithm for efficiency.
 *
 * @param {number} x - X coordinate [0, 2^order - 1]
 * @param {number} y - Y coordinate [0, 2^order - 1]
 * @param {number} [order=8] - Hilbert curve order (resolution)
 * @returns {number} Hilbert index d
 */
export function xy2d(x, y, order = HILBERT_CONFIG.DEFAULT_ORDER) {
  const n = 1 << order; // 2^order
  let rx, ry, s, d = 0;

  // Clamp inputs
  x = Math.max(0, Math.min(n - 1, Math.floor(x)));
  y = Math.max(0, Math.min(n - 1, Math.floor(y)));

  for (s = n >> 1; s > 0; s >>= 1) {
    rx = (x & s) > 0 ? 1 : 0;
    ry = (y & s) > 0 ? 1 : 0;
    d += s * s * ((3 * rx) ^ ry);

    // Rotate quadrant
    [x, y] = rotate(n, x, y, rx, ry);
  }

  return d;
}

/**
 * Convert Hilbert index (d) to (x, y) coordinates
 *
 * @param {number} d - Hilbert index
 * @param {number} [order=8] - Hilbert curve order
 * @returns {{x: number, y: number}} Coordinates
 */
export function d2xy(d, order = HILBERT_CONFIG.DEFAULT_ORDER) {
  const n = 1 << order;
  let rx, ry, s, t = d;
  let x = 0, y = 0;

  for (s = 1; s < n; s <<= 1) {
    rx = 1 & (t >> 1);
    ry = 1 & (t ^ rx);

    // Rotate quadrant
    [x, y] = rotate(s, x, y, rx, ry);

    x += s * rx;
    y += s * ry;
    t >>= 2;
  }

  return { x, y };
}

/**
 * Rotate/flip quadrant appropriately
 * @private
 */
function rotate(n, x, y, rx, ry) {
  if (ry === 0) {
    if (rx === 1) {
      x = n - 1 - x;
      y = n - 1 - y;
    }
    // Swap x and y
    [x, y] = [y, x];
  }
  return [x, y];
}

// ═══════════════════════════════════════════════════════════════════════════
// N-DIMENSIONAL HILBERT (for embeddings)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Convert N-dimensional point to Hilbert index
 *
 * Uses recursive bisection approach for N dimensions.
 *
 * @param {number[]} coords - N-dimensional coordinates [0, 1]
 * @param {number} [bits=8] - Bits per dimension
 * @returns {bigint} Hilbert index (bigint for large indices)
 */
export function coordsToHilbert(coords, bits = 8) {
  const n = coords.length;
  if (n === 0) return 0n;

  // Quantize coordinates to integer grid
  const max = (1 << bits) - 1;
  const quantized = coords.map(c =>
    Math.max(0, Math.min(max, Math.floor(c * (max + 1))))
  );

  // For 2D, use optimized algorithm
  if (n === 2) {
    return BigInt(xy2d(quantized[0], quantized[1], bits));
  }

  // For N-D, use generalized algorithm
  return nDimHilbert(quantized, bits);
}

/**
 * Generalized N-dimensional Hilbert curve
 * Uses Gray code interleaving
 * @private
 */
function nDimHilbert(coords, bits) {
  const n = coords.length;
  let index = 0n;

  for (let i = bits - 1; i >= 0; i--) {
    // Extract bit i from each coordinate
    let bits_at_level = 0;
    for (let j = 0; j < n; j++) {
      if ((coords[j] >> i) & 1) {
        bits_at_level |= (1 << (n - 1 - j));
      }
    }

    // Convert to Gray code
    const gray = bits_at_level ^ (bits_at_level >> 1);

    // Add to index
    index = (index << BigInt(n)) | BigInt(gray);
  }

  return index;
}

/**
 * Convert Hilbert index back to N-dimensional coordinates
 *
 * @param {bigint} index - Hilbert index
 * @param {number} dims - Number of dimensions
 * @param {number} [bits=8] - Bits per dimension
 * @returns {number[]} Normalized coordinates [0, 1]
 */
export function hilbertToCoords(index, dims, bits = 8) {
  if (dims === 2) {
    const { x, y } = d2xy(Number(index), bits);
    const max = (1 << bits) - 1;
    return [x / max, y / max];
  }

  // For N-D, use inverse of nDimHilbert
  const coords = new Array(dims).fill(0);
  const max = (1 << bits) - 1;

  for (let i = 0; i < bits; i++) {
    const gray = Number((index >> BigInt(i * dims)) & BigInt((1 << dims) - 1));

    // Inverse Gray code
    let bits_at_level = gray;
    for (let j = 1; j < dims; j++) {
      bits_at_level ^= (bits_at_level >> j);
    }

    // Distribute bits to coordinates
    for (let j = 0; j < dims; j++) {
      if ((bits_at_level >> (dims - 1 - j)) & 1) {
        coords[j] |= (1 << i);
      }
    }
  }

  return coords.map(c => c / max);
}

// ═══════════════════════════════════════════════════════════════════════════
// MEMORY INDEXING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * HilbertMemoryIndex - Spatial index for memory organization
 *
 * Maps memory embeddings to Hilbert indices for locality-preserving storage.
 */
export class HilbertMemoryIndex {
  /**
   * @param {Object} options - Configuration
   * @param {number} [options.dimensions=2] - Embedding dimensions to use
   * @param {number} [options.bits=8] - Bits per dimension (resolution)
   * @param {number} [options.bucketSize=16] - Memories per bucket
   */
  constructor(options = {}) {
    this.dimensions = options.dimensions || 2;
    this.bits = options.bits || HILBERT_CONFIG.DEFAULT_ORDER;
    this.bucketSize = options.bucketSize || 16;

    // Index structure: Map<hilbertIndex, Memory[]>
    this.buckets = new Map();

    // Reverse index: Map<memoryId, hilbertIndex>
    this.memoryToIndex = new Map();

    // Stats
    this.stats = {
      totalMemories: 0,
      totalBuckets: 0,
      avgBucketSize: 0,
      maxBucketSize: 0,
    };
  }

  /**
   * Add a memory to the index
   *
   * @param {Object} memory - Memory object with embedding
   * @param {string} memory.id - Memory ID
   * @param {number[]} memory.embedding - N-dimensional embedding
   * @returns {bigint} Hilbert index assigned
   */
  add(memory) {
    if (!memory.embedding || memory.embedding.length < this.dimensions) {
      throw new Error(`Memory requires embedding with ${this.dimensions}+ dimensions`);
    }

    // Use first N dimensions (could use PCA reduction for high-D)
    const coords = memory.embedding.slice(0, this.dimensions);

    // Normalize to [0, 1] if needed
    const normalized = this._normalizeCoords(coords);

    // Get Hilbert index
    const index = coordsToHilbert(normalized, this.bits);

    // Add to bucket
    if (!this.buckets.has(index)) {
      this.buckets.set(index, []);
      this.stats.totalBuckets++;
    }

    this.buckets.get(index).push(memory);
    this.memoryToIndex.set(memory.id, index);

    // Update stats
    this.stats.totalMemories++;
    this._updateStats();

    return index;
  }

  /**
   * Remove a memory from the index
   *
   * @param {string} memoryId - Memory ID to remove
   * @returns {boolean} True if removed
   */
  remove(memoryId) {
    const index = this.memoryToIndex.get(memoryId);
    if (index === undefined) return false;

    const bucket = this.buckets.get(index);
    if (bucket) {
      const idx = bucket.findIndex(m => m.id === memoryId);
      if (idx !== -1) {
        bucket.splice(idx, 1);
        this.stats.totalMemories--;

        // Clean up empty bucket
        if (bucket.length === 0) {
          this.buckets.delete(index);
          this.stats.totalBuckets--;
        }
      }
    }

    this.memoryToIndex.delete(memoryId);
    this._updateStats();

    return true;
  }

  /**
   * Find memories near a given point
   *
   * Uses Hilbert locality: nearby indices = nearby points
   *
   * @param {number[]} coords - Query coordinates
   * @param {number} [k=10] - Number of neighbors to return
   * @param {number} [radius=1] - Search radius (in Hilbert index space)
   * @returns {Object[]} Nearest memories
   */
  findNear(coords, k = 10, radius = 1) {
    const normalized = this._normalizeCoords(coords.slice(0, this.dimensions));
    const centerIndex = coordsToHilbert(normalized, this.bits);

    const candidates = [];

    // Search center and nearby buckets
    const searchRadius = BigInt(radius * (1 << this.bits));

    for (const [index, bucket] of this.buckets) {
      const distance = index > centerIndex
        ? index - centerIndex
        : centerIndex - index;

      if (distance <= searchRadius) {
        candidates.push(...bucket.map(m => ({
          ...m,
          hilbertDistance: Number(distance),
        })));
      }
    }

    // Sort by Hilbert distance (locality-preserving approximation)
    candidates.sort((a, b) => a.hilbertDistance - b.hilbertDistance);

    return candidates.slice(0, k);
  }

  /**
   * Find memories within a Hilbert range
   *
   * @param {bigint} startIndex - Start of range
   * @param {bigint} endIndex - End of range
   * @returns {Object[]} Memories in range
   */
  rangeQuery(startIndex, endIndex) {
    const results = [];

    for (const [index, bucket] of this.buckets) {
      if (index >= startIndex && index <= endIndex) {
        results.push(...bucket);
      }
    }

    return results;
  }

  /**
   * Get memories grouped by spatial region
   *
   * Useful for visualizing memory organization
   *
   * @param {number} [gridSize=4] - Grid resolution
   * @returns {Map<string, Object[]>} Grid cells with memories
   */
  getSpatialGrid(gridSize = 4) {
    const grid = new Map();
    const cellSize = (1 << this.bits) / gridSize;

    for (const [index, bucket] of this.buckets) {
      const { x, y } = d2xy(Number(index), this.bits);
      const cellX = Math.floor(x / cellSize);
      const cellY = Math.floor(y / cellSize);
      const key = `${cellX},${cellY}`;

      if (!grid.has(key)) {
        grid.set(key, []);
      }
      grid.get(key).push(...bucket);
    }

    return grid;
  }

  /**
   * Get the Hilbert path through all memories
   *
   * Returns memories in Hilbert order (spatially coherent traversal)
   *
   * @returns {Object[]} Memories in Hilbert order
   */
  getHilbertPath() {
    const indices = Array.from(this.buckets.keys()).sort((a, b) =>
      a < b ? -1 : a > b ? 1 : 0
    );

    const path = [];
    for (const index of indices) {
      path.push(...this.buckets.get(index));
    }

    return path;
  }

  /**
   * Normalize coordinates to [0, 1]
   * @private
   */
  _normalizeCoords(coords) {
    // Assume embeddings are already normalized or use tanh squashing
    return coords.map(c => {
      if (c >= 0 && c <= 1) return c;
      // Tanh squashing for unbounded values
      return (Math.tanh(c) + 1) / 2;
    });
  }

  /**
   * Update statistics
   * @private
   */
  _updateStats() {
    if (this.stats.totalBuckets === 0) {
      this.stats.avgBucketSize = 0;
      this.stats.maxBucketSize = 0;
      return;
    }

    let maxSize = 0;
    for (const bucket of this.buckets.values()) {
      maxSize = Math.max(maxSize, bucket.length);
    }

    this.stats.avgBucketSize = this.stats.totalMemories / this.stats.totalBuckets;
    this.stats.maxBucketSize = maxSize;
  }

  /**
   * Get index statistics
   */
  getStats() {
    return {
      ...this.stats,
      dimensions: this.dimensions,
      bits: this.bits,
      resolution: 1 << this.bits,
      maxIndex: BigInt(1) << BigInt(this.bits * this.dimensions),
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// LOCALITY-SENSITIVE HASHING (LSH) with Hilbert
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Hilbert-based Locality-Sensitive Hash
 *
 * Combines Hilbert indexing with LSH for approximate nearest neighbor search.
 *
 * @param {number[]} embedding - Input embedding
 * @param {number} [numHashes=4] - Number of hash functions
 * @param {number} [bits=8] - Bits per dimension
 * @returns {string[]} LSH signatures
 */
export function hilbertLSH(embedding, numHashes = 4, bits = 8) {
  const signatures = [];
  const dims = embedding.length;

  for (let h = 0; h < numHashes; h++) {
    // Random projection (deterministic based on hash index)
    const projected = [];
    for (let i = 0; i < 2; i++) { // Project to 2D for Hilbert
      let sum = 0;
      for (let j = 0; j < dims; j++) {
        // Pseudo-random weight based on h, i, j
        const weight = Math.sin((h * 1000 + i * 100 + j) * PHI_INV);
        sum += embedding[j] * weight;
      }
      // Normalize to [0, 1]
      projected.push((Math.tanh(sum) + 1) / 2);
    }

    // Get Hilbert index
    const index = coordsToHilbert(projected, bits);
    signatures.push(index.toString(36)); // Base36 for compact representation
  }

  return signatures;
}

/**
 * Estimate similarity from Hilbert LSH signatures
 *
 * @param {string[]} sig1 - First signature set
 * @param {string[]} sig2 - Second signature set
 * @returns {number} Estimated similarity [0, 1]
 */
export function hilbertLSHSimilarity(sig1, sig2) {
  if (sig1.length !== sig2.length) {
    throw new Error('Signature lengths must match');
  }

  let matches = 0;
  for (let i = 0; i < sig1.length; i++) {
    if (sig1[i] === sig2[i]) matches++;
  }

  return matches / sig1.length;
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

export default {
  // Core Hilbert functions
  xy2d,
  d2xy,
  coordsToHilbert,
  hilbertToCoords,

  // Memory indexing
  HilbertMemoryIndex,

  // LSH
  hilbertLSH,
  hilbertLSHSimilarity,

  // Config
  HILBERT_CONFIG,
};
