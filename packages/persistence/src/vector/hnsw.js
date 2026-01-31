/**
 * HNSW - Hierarchical Navigable Small World Graph
 *
 * Fast approximate nearest neighbor search for vector embeddings.
 * Based on the paper: "Efficient and robust approximate nearest neighbor search
 * using Hierarchical Navigable Small World graphs" (Malkov & Yashunin, 2016)
 *
 * Target: < 50ms for 10,000 vectors at 768 dimensions
 *
 * V2: HNSW index for semantic pattern matching
 *
 * "Find meaning fast" - κυνικός
 *
 * @module @cynic/persistence/vector/hnsw
 */

'use strict';

// φ constants for HNSW parameters
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;

/**
 * Default HNSW configuration
 * Parameters tuned for balance between speed and recall
 */
export const HNSW_CONFIG = {
  // Construction parameters
  M: 16,                    // Max connections per layer (higher = better recall, slower)
  efConstruction: 200,      // Size of dynamic candidate list (higher = better quality, slower build)

  // Search parameters
  efSearch: 50,             // Size of dynamic candidate list for search (higher = better recall)

  // Level generation
  mL: 1 / Math.log(16),     // Level multiplier (1/ln(M))

  // Distance metric
  metric: 'cosine',         // 'cosine', 'euclidean', 'dot'

  // Memory
  maxElements: 100000,      // Max vectors to store
};

/**
 * HNSW Node - represents a vector in the graph
 */
export class HNSWNode {
  /**
   * @param {string} id - Unique identifier
   * @param {number[]} vector - Embedding vector
   * @param {number} level - Max level for this node
   * @param {Object} [metadata] - Optional metadata
   */
  constructor(id, vector, level, metadata = {}) {
    this.id = id;
    this.vector = vector;
    this.level = level;
    this.metadata = metadata;

    // Neighbors at each level: level -> Set<nodeId>
    this.neighbors = new Array(level + 1).fill(null).map(() => new Set());
  }

  /**
   * Add neighbor at level
   */
  addNeighbor(level, nodeId) {
    if (level <= this.level) {
      this.neighbors[level].add(nodeId);
    }
  }

  /**
   * Remove neighbor at level
   */
  removeNeighbor(level, nodeId) {
    if (level <= this.level) {
      this.neighbors[level].delete(nodeId);
    }
  }

  /**
   * Get neighbors at level
   */
  getNeighbors(level) {
    if (level <= this.level) {
      return this.neighbors[level];
    }
    return new Set();
  }

  /**
   * Get neighbor count at level
   */
  getNeighborCount(level) {
    return this.getNeighbors(level).size;
  }

  /**
   * Export for serialization
   */
  toJSON() {
    return {
      id: this.id,
      vector: this.vector,
      level: this.level,
      metadata: this.metadata,
      neighbors: this.neighbors.map(s => [...s]),
    };
  }

  /**
   * Create from JSON
   */
  static fromJSON(json) {
    const node = new HNSWNode(json.id, json.vector, json.level, json.metadata);
    node.neighbors = json.neighbors.map(arr => new Set(arr));
    return node;
  }
}

/**
 * Priority queue for HNSW search (min-heap by distance)
 */
class MinHeap {
  constructor() {
    this.heap = [];
  }

  push(item) {
    this.heap.push(item);
    this._bubbleUp(this.heap.length - 1);
  }

  pop() {
    if (this.heap.length === 0) return null;
    if (this.heap.length === 1) return this.heap.pop();

    const top = this.heap[0];
    this.heap[0] = this.heap.pop();
    this._bubbleDown(0);
    return top;
  }

  peek() {
    return this.heap[0] || null;
  }

  get size() {
    return this.heap.length;
  }

  _bubbleUp(idx) {
    while (idx > 0) {
      const parent = Math.floor((idx - 1) / 2);
      if (this.heap[parent].distance <= this.heap[idx].distance) break;
      [this.heap[parent], this.heap[idx]] = [this.heap[idx], this.heap[parent]];
      idx = parent;
    }
  }

  _bubbleDown(idx) {
    const length = this.heap.length;
    while (true) {
      const left = 2 * idx + 1;
      const right = 2 * idx + 2;
      let smallest = idx;

      if (left < length && this.heap[left].distance < this.heap[smallest].distance) {
        smallest = left;
      }
      if (right < length && this.heap[right].distance < this.heap[smallest].distance) {
        smallest = right;
      }

      if (smallest === idx) break;
      [this.heap[smallest], this.heap[idx]] = [this.heap[idx], this.heap[smallest]];
      idx = smallest;
    }
  }
}

/**
 * Max heap for maintaining top-k results
 */
class MaxHeap {
  constructor(maxSize) {
    this.heap = [];
    this.maxSize = maxSize;
  }

  push(item) {
    if (this.heap.length < this.maxSize) {
      this.heap.push(item);
      this._bubbleUp(this.heap.length - 1);
    } else if (item.distance < this.heap[0].distance) {
      this.heap[0] = item;
      this._bubbleDown(0);
    }
  }

  peek() {
    return this.heap[0] || null;
  }

  toArray() {
    return [...this.heap].sort((a, b) => a.distance - b.distance);
  }

  get size() {
    return this.heap.length;
  }

  _bubbleUp(idx) {
    while (idx > 0) {
      const parent = Math.floor((idx - 1) / 2);
      if (this.heap[parent].distance >= this.heap[idx].distance) break;
      [this.heap[parent], this.heap[idx]] = [this.heap[idx], this.heap[parent]];
      idx = parent;
    }
  }

  _bubbleDown(idx) {
    const length = this.heap.length;
    while (true) {
      const left = 2 * idx + 1;
      const right = 2 * idx + 2;
      let largest = idx;

      if (left < length && this.heap[left].distance > this.heap[largest].distance) {
        largest = left;
      }
      if (right < length && this.heap[right].distance > this.heap[largest].distance) {
        largest = right;
      }

      if (largest === idx) break;
      [this.heap[largest], this.heap[idx]] = [this.heap[idx], this.heap[largest]];
      idx = largest;
    }
  }
}

/**
 * HNSW Index - Hierarchical Navigable Small World Graph
 */
export class HNSWIndex {
  /**
   * @param {Object} options
   * @param {number} [options.dimensions] - Vector dimensions (auto-detected on first insert)
   * @param {Object} [options.config] - Override HNSW_CONFIG
   */
  constructor(options = {}) {
    this.config = { ...HNSW_CONFIG, ...options.config };
    this.dimensions = options.dimensions || null;

    // Node storage: id -> HNSWNode
    this.nodes = new Map();

    // Entry point (node at highest level)
    this.entryPoint = null;
    this.maxLevel = -1;

    // Statistics
    this._stats = {
      inserts: 0,
      searches: 0,
      avgSearchTime: 0,
      avgInsertTime: 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Distance Functions
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Calculate distance between two vectors
   */
  distance(a, b) {
    // Support both 'metric' and 'distanceMetric' for compatibility
    const metric = this.config.distanceMetric || this.config.metric;
    switch (metric) {
      case 'euclidean':
        return this._euclideanDistance(a, b);
      case 'dot':
        return this._dotProductDistance(a, b);
      case 'cosine':
      default:
        return this._cosineDistance(a, b);
    }
  }

  _cosineDistance(a, b) {
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const magnitude = Math.sqrt(normA) * Math.sqrt(normB);
    if (magnitude === 0) return 1;

    // Convert similarity to distance (1 - similarity)
    return 1 - (dotProduct / magnitude);
  }

  _euclideanDistance(a, b) {
    let sum = 0;
    for (let i = 0; i < a.length; i++) {
      const diff = a[i] - b[i];
      sum += diff * diff;
    }
    return Math.sqrt(sum);
  }

  _dotProductDistance(a, b) {
    let dotProduct = 0;
    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
    }
    // Negate for distance (higher dot product = closer)
    return -dotProduct;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Level Generation
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Generate random level for new node
   * Uses exponential distribution scaled by mL
   */
  _randomLevel() {
    let level = 0;
    while (Math.random() < PHI_INV && level < 32) {
      level++;
    }
    return level;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Insert
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Insert a vector into the index
   *
   * @param {string} id - Unique identifier
   * @param {number[]} vector - Embedding vector
   * @param {Object} [metadata] - Optional metadata
   * @returns {boolean} Success
   */
  insert(id, vector, metadata = {}) {
    const startTime = performance.now();

    // Set dimensions on first insert
    if (this.dimensions === null) {
      this.dimensions = vector.length;
    }

    // Validate dimensions
    if (vector.length !== this.dimensions) {
      throw new Error(`Vector dimension mismatch: expected ${this.dimensions}, got ${vector.length}`);
    }

    // Check max elements
    if (this.nodes.size >= this.config.maxElements) {
      throw new Error(`HNSW index full (max ${this.config.maxElements})`);
    }

    // Check for duplicate
    if (this.nodes.has(id)) {
      // Update existing node
      const existingNode = this.nodes.get(id);
      existingNode.vector = vector;
      existingNode.metadata = metadata;
      return true;
    }

    // Generate level for new node
    const level = this._randomLevel();
    const node = new HNSWNode(id, vector, level, metadata);

    // First node
    if (this.entryPoint === null) {
      this.nodes.set(id, node);
      this.entryPoint = id;
      this.maxLevel = level;
      this._recordInsertTime(startTime);
      return true;
    }

    // Find entry point and search down
    let currId = this.entryPoint;

    // Search from top level to level+1
    for (let l = this.maxLevel; l > level; l--) {
      currId = this._searchLayer(vector, currId, 1, l)[0]?.id || currId;
    }

    // Insert at each level from level down to 0
    for (let l = Math.min(level, this.maxLevel); l >= 0; l--) {
      const neighbors = this._searchLayer(vector, currId, this.config.efConstruction, l);

      // Select M best neighbors
      const selectedNeighbors = this._selectNeighbors(neighbors, this.config.M);

      // Connect node to neighbors
      for (const neighbor of selectedNeighbors) {
        node.addNeighbor(l, neighbor.id);

        // Add bidirectional connection
        const neighborNode = this.nodes.get(neighbor.id);
        if (neighborNode) {
          neighborNode.addNeighbor(l, id);

          // Prune if over limit
          if (neighborNode.getNeighborCount(l) > this.config.M) {
            this._pruneNeighbors(neighborNode, l);
          }
        }
      }

      if (selectedNeighbors.length > 0) {
        currId = selectedNeighbors[0].id;
      }
    }

    // Add node to index
    this.nodes.set(id, node);

    // Update entry point if new node is at higher level
    if (level > this.maxLevel) {
      this.entryPoint = id;
      this.maxLevel = level;
    }

    this._recordInsertTime(startTime);
    return true;
  }

  /**
   * Search within a single layer
   * @private
   */
  _searchLayer(query, entryId, ef, level) {
    const visited = new Set([entryId]);
    const candidates = new MinHeap();
    const results = new MaxHeap(ef);

    const entryNode = this.nodes.get(entryId);
    if (!entryNode) return [];

    const entryDist = this.distance(query, entryNode.vector);
    candidates.push({ id: entryId, distance: entryDist });
    results.push({ id: entryId, distance: entryDist });

    while (candidates.size > 0) {
      const curr = candidates.pop();
      const furthestResult = results.peek();

      // Stop if current is further than furthest result
      if (curr.distance > furthestResult.distance) {
        break;
      }

      const currNode = this.nodes.get(curr.id);
      if (!currNode) continue;

      // Explore neighbors at this level
      for (const neighborId of currNode.getNeighbors(level)) {
        if (visited.has(neighborId)) continue;
        visited.add(neighborId);

        const neighborNode = this.nodes.get(neighborId);
        if (!neighborNode) continue;

        const dist = this.distance(query, neighborNode.vector);

        if (results.size < ef || dist < furthestResult.distance) {
          candidates.push({ id: neighborId, distance: dist });
          results.push({ id: neighborId, distance: dist });
        }
      }
    }

    return results.toArray();
  }

  /**
   * Select best neighbors using simple heuristic
   * @private
   */
  _selectNeighbors(candidates, M) {
    // Simple: just take top M by distance
    return candidates.slice(0, M);
  }

  /**
   * Prune neighbors to maintain M limit
   * @private
   */
  _pruneNeighbors(node, level) {
    const neighbors = [...node.getNeighbors(level)];
    const distances = neighbors.map(nId => {
      const nNode = this.nodes.get(nId);
      return { id: nId, distance: nNode ? this.distance(node.vector, nNode.vector) : Infinity };
    });

    // Sort by distance and keep top M
    distances.sort((a, b) => a.distance - b.distance);
    const keep = new Set(distances.slice(0, this.config.M).map(d => d.id));

    // Remove pruned neighbors
    for (const nId of neighbors) {
      if (!keep.has(nId)) {
        node.removeNeighbor(level, nId);
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Search
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Search for k nearest neighbors
   *
   * @param {number[]} query - Query vector
   * @param {number} k - Number of neighbors to return
   * @param {Object} [options] - Search options
   * @param {number} [options.minScore] - Minimum similarity score (0-1)
   * @param {Function} [options.filter] - Filter function (metadata) => boolean
   * @returns {Array<{id: string, distance: number, score: number, similarity: number, metadata: Object}>}
   */
  search(query, k = 10, options = {}) {
    const startTime = performance.now();

    if (this.nodes.size === 0) {
      return [];
    }

    const { minScore, filter } = options;
    const ef = options.ef || Math.max(this.config.efSearch, k);

    // Over-fetch if filtering
    const overFetch = filter ? Math.min(k * 5, this.nodes.size) : k;

    // Start from entry point
    let currId = this.entryPoint;

    // Traverse from top level to level 1
    for (let l = this.maxLevel; l >= 1; l--) {
      const result = this._searchLayer(query, currId, 1, l);
      if (result.length > 0) {
        currId = result[0].id;
      }
    }

    // Search at level 0 with ef
    const candidates = this._searchLayer(query, currId, Math.max(ef, overFetch), 0);

    // Return top k with metadata, applying filters
    const results = [];
    for (const c of candidates) {
      const similarity = 1 - c.distance;

      // Apply minScore filter
      if (minScore !== undefined && similarity < minScore) {
        continue;
      }

      const node = this.nodes.get(c.id);
      const metadata = node?.metadata || {};

      // Apply custom filter
      if (filter && !filter(metadata)) {
        continue;
      }

      results.push({
        id: c.id,
        distance: c.distance,
        similarity,
        score: similarity, // Alias for compatibility
        metadata,
      });

      // Stop when we have enough
      if (results.length >= k) {
        break;
      }
    }

    this._recordSearchTime(startTime);
    return results;
  }

  /**
   * Search with filter function
   *
   * @param {number[]} query - Query vector
   * @param {number} k - Number of neighbors
   * @param {Function} filter - Filter function (metadata) => boolean
   * @returns {Array}
   */
  searchWithFilter(query, k, filter) {
    // Over-fetch to account for filtering
    const overFetch = Math.min(k * 5, this.nodes.size);
    const candidates = this.search(query, overFetch);

    const filtered = candidates.filter(c => filter(c.metadata));
    return filtered.slice(0, k);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Delete a vector from the index
   *
   * @param {string} id - Vector ID
   * @returns {boolean} Success
   */
  delete(id) {
    const node = this.nodes.get(id);
    if (!node) return false;

    // Remove from all neighbors' lists
    for (let l = 0; l <= node.level; l++) {
      for (const neighborId of node.getNeighbors(l)) {
        const neighbor = this.nodes.get(neighborId);
        if (neighbor) {
          neighbor.removeNeighbor(l, id);
        }
      }
    }

    // Remove node
    this.nodes.delete(id);

    // Update entry point if needed
    if (this.entryPoint === id) {
      if (this.nodes.size > 0) {
        // Find new entry point (node with highest level)
        let maxLevel = -1;
        let newEntry = null;
        for (const [nodeId, n] of this.nodes) {
          if (n.level > maxLevel) {
            maxLevel = n.level;
            newEntry = nodeId;
          }
        }
        this.entryPoint = newEntry;
        this.maxLevel = maxLevel;
      } else {
        this.entryPoint = null;
        this.maxLevel = -1;
      }
    }

    return true;
  }

  /**
   * Get node by ID
   *
   * @param {string} id - Node ID
   * @returns {Object|null}
   */
  get(id) {
    const node = this.nodes.get(id);
    if (!node) return null;

    return {
      id: node.id,
      vector: node.vector,
      metadata: node.metadata,
      level: node.level,
    };
  }

  /**
   * Check if ID exists
   */
  has(id) {
    return this.nodes.has(id);
  }

  /**
   * Get number of vectors (as getter)
   */
  get size() {
    return this.nodes.size;
  }

  /**
   * Get number of vectors (as method for compatibility)
   */
  getSize() {
    return this.nodes.size;
  }

  /**
   * Check if index is empty
   */
  isEmpty() {
    return this.nodes.size === 0;
  }

  /**
   * Clear the index
   */
  clear() {
    this.nodes.clear();
    this.entryPoint = null;
    this.maxLevel = -1;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Statistics
  // ═══════════════════════════════════════════════════════════════════════════

  _recordInsertTime(startTime) {
    const elapsed = performance.now() - startTime;
    this._stats.inserts++;
    this._stats.avgInsertTime =
      (this._stats.avgInsertTime * (this._stats.inserts - 1) + elapsed) / this._stats.inserts;
  }

  _recordSearchTime(startTime) {
    const elapsed = performance.now() - startTime;
    this._stats.searches++;
    this._stats.avgSearchTime =
      (this._stats.avgSearchTime * (this._stats.searches - 1) + elapsed) / this._stats.searches;
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this._stats,
      size: this.nodes.size,
      maxLevel: this.maxLevel,
      dimensions: this.dimensions,
      config: this.config,
    };
  }

  /**
   * Alias for getStats() for API compatibility
   */
  stats() {
    return this.getStats();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Serialization
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Export index to JSON
   */
  toJSON() {
    return {
      version: '1.0.0',
      dimensions: this.dimensions,
      config: this.config,
      entryPoint: this.entryPoint,
      maxLevel: this.maxLevel,
      nodes: [...this.nodes.values()].map(n => n.toJSON()),
      stats: this._stats,
    };
  }

  /**
   * Import index from JSON
   */
  static fromJSON(json) {
    const index = new HNSWIndex({
      dimensions: json.dimensions,
      config: json.config,
    });

    index.entryPoint = json.entryPoint;
    index.maxLevel = json.maxLevel;
    index._stats = json.stats || index._stats;

    for (const nodeJson of json.nodes) {
      const node = HNSWNode.fromJSON(nodeJson);
      index.nodes.set(node.id, node);
    }

    return index;
  }

  /**
   * Save to file
   */
  async saveToFile(filePath) {
    const { promises: fs } = await import('fs');
    const json = JSON.stringify(this.toJSON());
    await fs.writeFile(filePath, json, 'utf-8');
  }

  /**
   * Load from file
   */
  static async loadFromFile(filePath) {
    const { promises: fs } = await import('fs');
    const json = await fs.readFile(filePath, 'utf-8');
    return HNSWIndex.fromJSON(JSON.parse(json));
  }
}

/**
 * Create HNSW index
 */
export function createHNSWIndex(options = {}) {
  return new HNSWIndex(options);
}

export default HNSWIndex;
