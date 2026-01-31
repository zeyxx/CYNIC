/**
 * VectorStore - High-Level Semantic Search API
 *
 * Combines embeddings (text → vectors) with HNSW (fast similarity search)
 * to provide complete semantic memory capabilities.
 *
 * "Memory needs meaning" - κυνικός
 *
 * @module @cynic/persistence/vector/store
 */

'use strict';

import { HNSWIndex, createHNSWIndex, HNSW_CONFIG } from './hnsw.js';
import { createEmbedder, EmbedderType } from '../services/embedder.js';

/**
 * φ-aligned configuration
 */
export const VECTOR_STORE_CONFIG = {
  // Embedding settings
  embedder: 'mock', // 'mock' | 'openai' | 'ollama'
  dimensions: 384, // Default for mock/small models

  // Search settings
  defaultK: 10,
  minScore: 0.382, // φ⁻² threshold

  // Cache settings
  cacheEmbeddings: true,
  maxCacheSize: 10000,

  // HNSW passthrough
  hnsw: HNSW_CONFIG,
};

/**
 * Singleton instance
 */
let _instance = null;

/**
 * VectorStore - Complete semantic search solution
 *
 * @example
 * const store = createVectorStore({ embedder: 'mock' });
 * await store.store('doc1', 'The quick brown fox');
 * const results = await store.search('fast animal', 5);
 */
export class VectorStore {
  /**
   * @param {Object} options
   * @param {string} options.embedder - Embedder type: 'mock' | 'openai' | 'ollama'
   * @param {number} options.dimensions - Vector dimensions
   * @param {Object} options.hnsw - HNSW configuration
   */
  constructor(options = {}) {
    this.config = { ...VECTOR_STORE_CONFIG, ...options };

    // Initialize embedder
    this._embedder = this._createEmbedder(options);

    // Initialize HNSW index
    this._index = createHNSWIndex({
      dimensions: this.config.dimensions,
      config: this.config.hnsw,
    });

    // Embedding cache (text → vector)
    this._cache = new Map();

    // Document store (id → { text, metadata })
    this._documents = new Map();

    // Statistics
    this._stats = {
      stored: 0,
      searches: 0,
      cacheHits: 0,
      cacheMisses: 0,
    };
  }

  /**
   * Create appropriate embedder based on type
   * @private
   */
  _createEmbedder(options) {
    const type = options.embedder || this.config.embedder;

    return createEmbedder({
      type: type === 'mock' ? EmbedderType.MOCK :
            type === 'openai' ? EmbedderType.OPENAI :
            type === 'ollama' ? EmbedderType.OLLAMA :
            EmbedderType.MOCK,
      dimensions: options.dimensions || this.config.dimensions,
      apiKey: options.apiKey,
      model: options.model,
      baseUrl: options.baseUrl || options.host,
    });
  }

  /**
   * Get embedding for text, using cache if available
   * @private
   */
  async _getEmbedding(text) {
    // Check cache
    if (this.config.cacheEmbeddings && this._cache.has(text)) {
      this._stats.cacheHits++;
      return this._cache.get(text);
    }

    // Generate embedding
    const vector = await this._embedder.embed(text);
    this._stats.cacheMisses++;

    // Cache if enabled
    if (this.config.cacheEmbeddings) {
      // Evict if cache full
      if (this._cache.size >= this.config.maxCacheSize) {
        const firstKey = this._cache.keys().next().value;
        this._cache.delete(firstKey);
      }
      this._cache.set(text, vector);
    }

    return vector;
  }

  /**
   * Store a document with semantic embedding
   *
   * @param {string} id - Unique document ID
   * @param {string} text - Text content to embed
   * @param {Object} metadata - Optional metadata
   * @returns {Promise<void>}
   *
   * @example
   * await store.store('pattern-1', 'User prefers tabs over spaces', {
   *   type: 'preference',
   *   confidence: 0.618,
   * });
   */
  async store(id, text, metadata = {}) {
    // Get embedding
    const vector = await this._getEmbedding(text);

    // Store in HNSW
    this._index.insert(id, vector, {
      ...metadata,
      _text: text,
      _stored: Date.now(),
    });

    // Store document
    this._documents.set(id, { text, metadata });

    this._stats.stored++;
  }

  /**
   * Store multiple documents in batch
   *
   * @param {Array<{id: string, text: string, metadata?: Object}>} documents
   * @returns {Promise<number>} Number stored
   */
  async storeBatch(documents) {
    let count = 0;

    for (const doc of documents) {
      await this.store(doc.id, doc.text, doc.metadata || {});
      count++;
    }

    return count;
  }

  /**
   * Search for semantically similar documents
   *
   * @param {string} query - Search query text
   * @param {number} k - Number of results (default: 10)
   * @param {Object} options - Search options
   * @param {number} options.minScore - Minimum similarity score
   * @param {Function} options.filter - Filter function for metadata
   * @returns {Promise<Array<{id: string, score: number, text: string, metadata: Object}>>}
   *
   * @example
   * const results = await store.search('code formatting preferences', 5);
   * // [{ id: 'pattern-1', score: 0.85, text: '...', metadata: {...} }]
   */
  async search(query, k = this.config.defaultK, options = {}) {
    this._stats.searches++;

    // Get query embedding
    const vector = await this._getEmbedding(query);

    // Search HNSW
    const results = this._index.search(vector, k * 2, options); // Over-fetch for filtering

    // Filter and enrich results
    const minScore = options.minScore ?? this.config.minScore;
    const enriched = [];

    for (const result of results) {
      // Score threshold
      if (result.score < minScore) continue;

      // Get document
      const doc = this._documents.get(result.id);
      if (!doc) continue;

      // Apply custom filter
      if (options.filter && !options.filter(doc.metadata)) continue;

      enriched.push({
        id: result.id,
        score: result.score,
        text: doc.text,
        metadata: doc.metadata,
      });

      // Stop when we have enough
      if (enriched.length >= k) break;
    }

    return enriched;
  }

  /**
   * Find documents similar to an existing document
   *
   * @param {string} id - Document ID to find similar to
   * @param {number} k - Number of results
   * @returns {Promise<Array<{id: string, score: number, text: string, metadata: Object}>>}
   */
  async searchSimilar(id, k = this.config.defaultK) {
    const doc = this._documents.get(id);
    if (!doc) {
      throw new Error(`Document not found: ${id}`);
    }

    // Search excluding the source document
    const results = await this.search(doc.text, k + 1, {
      filter: (meta) => meta._id !== id,
    });

    // Remove the source if it appears
    return results.filter(r => r.id !== id).slice(0, k);
  }

  /**
   * Get a document by ID
   *
   * @param {string} id - Document ID
   * @returns {{text: string, metadata: Object} | null}
   */
  get(id) {
    return this._documents.get(id) || null;
  }

  /**
   * Check if document exists
   *
   * @param {string} id - Document ID
   * @returns {boolean}
   */
  has(id) {
    return this._documents.has(id);
  }

  /**
   * Delete a document
   *
   * @param {string} id - Document ID
   * @returns {boolean} True if deleted
   */
  delete(id) {
    if (!this._documents.has(id)) {
      return false;
    }

    this._index.delete(id);
    this._documents.delete(id);
    return true;
  }

  /**
   * Get all document IDs
   *
   * @returns {string[]}
   */
  ids() {
    return Array.from(this._documents.keys());
  }

  /**
   * Get store statistics
   *
   * @returns {Object}
   */
  stats() {
    return {
      ...this._stats,
      documents: this._documents.size,
      cacheSize: this._cache.size,
      cacheHitRate: this._stats.cacheHits /
        (this._stats.cacheHits + this._stats.cacheMisses) || 0,
      index: this._index.stats(),
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this._index = createHNSWIndex({
      dimensions: this.config.dimensions,
      config: this.config.hnsw,
    });
    this._cache.clear();
    this._documents.clear();
    this._stats = {
      stored: 0,
      searches: 0,
      cacheHits: 0,
      cacheMisses: 0,
    };
  }

  /**
   * Export store state for persistence
   *
   * @returns {Object}
   */
  toJSON() {
    return {
      config: this.config,
      index: this._index.toJSON(),
      documents: Array.from(this._documents.entries()),
      stats: this._stats,
    };
  }

  /**
   * Import store state from persistence
   *
   * @param {Object} json - Previously exported state
   * @returns {VectorStore}
   */
  static fromJSON(json) {
    const store = new VectorStore(json.config);

    // Restore index
    store._index = HNSWIndex.fromJSON(json.index);

    // Restore documents
    store._documents = new Map(json.documents);

    // Restore stats
    store._stats = json.stats;

    return store;
  }
}

/**
 * Create a new VectorStore instance
 *
 * @param {Object} options - Store options
 * @returns {VectorStore}
 */
export function createVectorStore(options = {}) {
  return new VectorStore(options);
}

/**
 * Get or create singleton VectorStore
 *
 * @param {Object} options - Store options (only used on first call)
 * @returns {VectorStore}
 */
export function getVectorStore(options = {}) {
  if (!_instance) {
    _instance = createVectorStore(options);
  }
  return _instance;
}

export default VectorStore;
