/**
 * CYNIC Vector Search / Embeddings
 *
 * "φ distrusts φ" - Semantic search with skepticism
 *
 * Provides semantic search capabilities:
 * 1. TF-IDF local embeddings (no external API needed)
 * 2. External embedding support (OpenAI, Cohere, etc.)
 * 3. Cosine similarity search
 * 4. Vector storage and retrieval
 *
 * @module @cynic/core/vector
 * @philosophy Understanding meaning, not just matching strings
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '../axioms/constants.js';

// =============================================================================
// CONSTANTS
// =============================================================================

export const VECTOR_CONSTANTS = {
  /** Default embedding dimension for TF-IDF (Fib(8) = 21 × 16 = 336) */
  DEFAULT_DIMENSION: 336,

  /** Max vocabulary size (Fib(13) = 233 × 10 = 2330) */
  MAX_VOCABULARY: 2330,

  /** Minimum word frequency to include */
  MIN_WORD_FREQUENCY: 2,

  /** Similarity threshold for relevance */
  SIMILARITY_THRESHOLD: PHI_INV_2, // 38.2%

  /** Max results per search */
  MAX_RESULTS: 21, // Fib(8)
};

// =============================================================================
// TF-IDF EMBEDDINGS (Local, no external API needed)
// =============================================================================

/**
 * Simple tokenizer
 */
function tokenize(text) {
  return text
    .toLowerCase()
    .replace(/[^\w\s]/g, ' ')
    .split(/\s+/)
    .filter(word => word.length >= 2 && word.length <= 30);
}

/**
 * Build vocabulary from documents
 */
function buildVocabulary(documents, maxSize = VECTOR_CONSTANTS.MAX_VOCABULARY) {
  const wordCounts = new Map();

  for (const doc of documents) {
    const tokens = tokenize(doc);
    const seen = new Set();

    for (const token of tokens) {
      if (!seen.has(token)) {
        wordCounts.set(token, (wordCounts.get(token) || 0) + 1);
        seen.add(token);
      }
    }
  }

  // Sort by frequency and take top N
  const sorted = [...wordCounts.entries()]
    .filter(([_, count]) => count >= VECTOR_CONSTANTS.MIN_WORD_FREQUENCY)
    .sort((a, b) => b[1] - a[1])
    .slice(0, maxSize);

  // Create word → index mapping
  const vocabulary = new Map();
  sorted.forEach(([word, _], index) => {
    vocabulary.set(word, index);
  });

  return vocabulary;
}

/**
 * Calculate TF-IDF vector for a document
 */
function calculateTfIdf(document, vocabulary, documentFrequencies, totalDocs) {
  const tokens = tokenize(document);
  const termFrequencies = new Map();

  // Count term frequencies
  for (const token of tokens) {
    if (vocabulary.has(token)) {
      termFrequencies.set(token, (termFrequencies.get(token) || 0) + 1);
    }
  }

  // Calculate TF-IDF vector
  const vector = new Float32Array(vocabulary.size);
  const maxTf = Math.max(...termFrequencies.values(), 1);

  for (const [word, index] of vocabulary) {
    const tf = (termFrequencies.get(word) || 0) / maxTf; // Normalized TF
    const df = documentFrequencies.get(word) || 1;
    const idf = Math.log(totalDocs / df);
    vector[index] = tf * idf;
  }

  // Normalize to unit vector
  const magnitude = Math.sqrt(vector.reduce((sum, val) => sum + val * val, 0));
  if (magnitude > 0) {
    for (let i = 0; i < vector.length; i++) {
      vector[i] /= magnitude;
    }
  }

  return Array.from(vector);
}

/**
 * TF-IDF Embedder
 *
 * Local embedding generation using TF-IDF.
 * No external API required.
 */
export class TfIdfEmbedder {
  constructor() {
    /** @type {Map<string, number>} Word → index */
    this.vocabulary = new Map();

    /** @type {Map<string, number>} Word → document frequency */
    this.documentFrequencies = new Map();

    /** Total documents indexed */
    this.totalDocs = 0;

    /** Whether embedder is trained */
    this.trained = false;
  }

  /**
   * Train embedder on a corpus
   * @param {string[]} documents - Array of document texts
   */
  train(documents) {
    this.vocabulary = buildVocabulary(documents);
    this.totalDocs = documents.length;

    // Calculate document frequencies
    this.documentFrequencies = new Map();
    for (const doc of documents) {
      const tokens = new Set(tokenize(doc));
      for (const token of tokens) {
        if (this.vocabulary.has(token)) {
          this.documentFrequencies.set(token, (this.documentFrequencies.get(token) || 0) + 1);
        }
      }
    }

    this.trained = true;
    return this;
  }

  /**
   * Add document to corpus (incremental)
   * @param {string} document
   */
  addDocument(document) {
    const tokens = new Set(tokenize(document));

    for (const token of tokens) {
      if (this.vocabulary.has(token)) {
        this.documentFrequencies.set(token, (this.documentFrequencies.get(token) || 0) + 1);
      }
    }

    this.totalDocs++;
  }

  /**
   * Generate embedding for text
   * @param {string} text
   * @returns {number[]} Embedding vector
   */
  embed(text) {
    if (!this.trained) {
      throw new Error('Embedder must be trained first');
    }

    return calculateTfIdf(
      text,
      this.vocabulary,
      this.documentFrequencies,
      Math.max(this.totalDocs, 1)
    );
  }

  /**
   * Get embedding dimension
   */
  getDimension() {
    return this.vocabulary.size;
  }

  /**
   * Export state for persistence
   */
  export() {
    return {
      vocabulary: Array.from(this.vocabulary.entries()),
      documentFrequencies: Array.from(this.documentFrequencies.entries()),
      totalDocs: this.totalDocs,
    };
  }

  /**
   * Import state from persistence
   */
  import(state) {
    this.vocabulary = new Map(state.vocabulary);
    this.documentFrequencies = new Map(state.documentFrequencies);
    this.totalDocs = state.totalDocs;
    this.trained = true;
    return this;
  }
}

// =============================================================================
// EXTERNAL EMBEDDING SUPPORT
// =============================================================================

/**
 * External Embedder (for OpenAI, Cohere, etc.)
 *
 * Uses external API to generate embeddings.
 */
export class ExternalEmbedder {
  /**
   * @param {Object} options
   * @param {string} options.provider - 'openai', 'cohere', 'voyage', etc.
   * @param {string} options.apiKey - API key
   * @param {string} [options.model] - Model name
   * @param {number} [options.dimension] - Embedding dimension
   */
  constructor(options = {}) {
    this.provider = options.provider || 'openai';
    this.apiKey = options.apiKey;
    this.model = options.model || this._getDefaultModel();
    this.dimension = options.dimension || this._getDefaultDimension();
    this.baseUrl = options.baseUrl || this._getDefaultBaseUrl();
  }

  _getDefaultModel() {
    switch (this.provider) {
      case 'openai': return 'text-embedding-3-small';
      case 'cohere': return 'embed-english-v3.0';
      case 'voyage': return 'voyage-2';
      default: return 'embed';
    }
  }

  _getDefaultDimension() {
    switch (this.provider) {
      case 'openai': return 1536;
      case 'cohere': return 1024;
      case 'voyage': return 1024;
      default: return 768;
    }
  }

  _getDefaultBaseUrl() {
    switch (this.provider) {
      case 'openai': return 'https://api.openai.com/v1';
      case 'cohere': return 'https://api.cohere.ai/v1';
      case 'voyage': return 'https://api.voyageai.com/v1';
      default: return null;
    }
  }

  /**
   * Generate embedding for text
   * @param {string} text
   * @returns {Promise<number[]>} Embedding vector
   */
  async embed(text) {
    if (!this.apiKey) {
      throw new Error(`API key required for ${this.provider}`);
    }

    switch (this.provider) {
      case 'openai':
        return this._embedOpenAI(text);
      case 'cohere':
        return this._embedCohere(text);
      default:
        throw new Error(`Unsupported provider: ${this.provider}`);
    }
  }

  /**
   * Batch embed multiple texts
   * @param {string[]} texts
   * @returns {Promise<number[][]>} Array of embedding vectors
   */
  async embedBatch(texts) {
    if (!this.apiKey) {
      throw new Error(`API key required for ${this.provider}`);
    }

    switch (this.provider) {
      case 'openai':
        return this._embedBatchOpenAI(texts);
      case 'cohere':
        return this._embedBatchCohere(texts);
      default:
        throw new Error(`Unsupported provider: ${this.provider}`);
    }
  }

  async _embedOpenAI(text) {
    const response = await fetch(`${this.baseUrl}/embeddings`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: this.model,
        input: text,
      }),
    });

    if (!response.ok) {
      throw new Error(`OpenAI API error: ${response.statusText}`);
    }

    const data = await response.json();
    return data.data[0].embedding;
  }

  async _embedBatchOpenAI(texts) {
    const response = await fetch(`${this.baseUrl}/embeddings`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: this.model,
        input: texts,
      }),
    });

    if (!response.ok) {
      throw new Error(`OpenAI API error: ${response.statusText}`);
    }

    const data = await response.json();
    return data.data.map(d => d.embedding);
  }

  async _embedCohere(text) {
    const response = await fetch(`${this.baseUrl}/embed`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: this.model,
        texts: [text],
        input_type: 'search_document',
      }),
    });

    if (!response.ok) {
      throw new Error(`Cohere API error: ${response.statusText}`);
    }

    const data = await response.json();
    return data.embeddings[0];
  }

  async _embedBatchCohere(texts) {
    const response = await fetch(`${this.baseUrl}/embed`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: this.model,
        texts,
        input_type: 'search_document',
      }),
    });

    if (!response.ok) {
      throw new Error(`Cohere API error: ${response.statusText}`);
    }

    const data = await response.json();
    return data.embeddings;
  }

  getDimension() {
    return this.dimension;
  }
}

// =============================================================================
// SIMILARITY FUNCTIONS
// =============================================================================

/**
 * Calculate cosine similarity between two vectors
 * @param {number[]} a
 * @param {number[]} b
 * @returns {number} Similarity score (0-1)
 */
export function cosineSimilarity(a, b) {
  if (a.length !== b.length) {
    throw new Error('Vectors must have same dimension');
  }

  let dotProduct = 0;
  let normA = 0;
  let normB = 0;

  for (let i = 0; i < a.length; i++) {
    dotProduct += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }

  normA = Math.sqrt(normA);
  normB = Math.sqrt(normB);

  if (normA === 0 || normB === 0) {
    return 0;
  }

  return dotProduct / (normA * normB);
}

/**
 * Calculate Euclidean distance between two vectors
 * @param {number[]} a
 * @param {number[]} b
 * @returns {number} Distance
 */
export function euclideanDistance(a, b) {
  if (a.length !== b.length) {
    throw new Error('Vectors must have same dimension');
  }

  let sum = 0;
  for (let i = 0; i < a.length; i++) {
    const diff = a[i] - b[i];
    sum += diff * diff;
  }

  return Math.sqrt(sum);
}

// =============================================================================
// VECTOR INDEX
// =============================================================================

/**
 * In-memory vector index for fast similarity search
 */
export class VectorIndex {
  constructor(options = {}) {
    /** @type {Array<{ id: string, vector: number[], metadata: Object }>} */
    this.vectors = [];

    /** Similarity function */
    this.similarityFn = options.similarity || cosineSimilarity;

    /** ID → index mapping */
    this.idIndex = new Map();
  }

  /**
   * Add vector to index
   * @param {string} id - Unique identifier
   * @param {number[]} vector - Embedding vector
   * @param {Object} [metadata] - Additional metadata
   */
  add(id, vector, metadata = {}) {
    // Check if exists
    if (this.idIndex.has(id)) {
      // Update existing
      const index = this.idIndex.get(id);
      this.vectors[index] = { id, vector, metadata };
    } else {
      // Add new
      this.idIndex.set(id, this.vectors.length);
      this.vectors.push({ id, vector, metadata });
    }
  }

  /**
   * Remove vector from index
   * @param {string} id
   */
  remove(id) {
    if (!this.idIndex.has(id)) return false;

    const index = this.idIndex.get(id);
    this.vectors.splice(index, 1);

    // Rebuild index
    this.idIndex.clear();
    this.vectors.forEach((v, i) => {
      this.idIndex.set(v.id, i);
    });

    return true;
  }

  /**
   * Search for similar vectors
   * @param {number[]} queryVector
   * @param {Object} [options]
   * @param {number} [options.limit] - Max results
   * @param {number} [options.threshold] - Min similarity
   * @param {Object} [options.filter] - Metadata filter
   * @returns {Array<{ id: string, similarity: number, metadata: Object }>}
   */
  search(queryVector, options = {}) {
    const {
      limit = VECTOR_CONSTANTS.MAX_RESULTS,
      threshold = VECTOR_CONSTANTS.SIMILARITY_THRESHOLD,
      filter = null,
    } = options;

    const results = [];

    for (const item of this.vectors) {
      // Apply filter
      if (filter && !this._matchesFilter(item.metadata, filter)) {
        continue;
      }

      const similarity = this.similarityFn(queryVector, item.vector);

      if (similarity >= threshold) {
        results.push({
          id: item.id,
          similarity,
          metadata: item.metadata,
        });
      }
    }

    // Sort by similarity descending
    results.sort((a, b) => b.similarity - a.similarity);

    // Apply limit
    return results.slice(0, limit);
  }

  /**
   * Check if metadata matches filter
   * @private
   */
  _matchesFilter(metadata, filter) {
    for (const [key, value] of Object.entries(filter)) {
      if (metadata[key] !== value) {
        return false;
      }
    }
    return true;
  }

  /**
   * Get index size
   */
  size() {
    return this.vectors.length;
  }

  /**
   * Clear index
   */
  clear() {
    this.vectors = [];
    this.idIndex.clear();
  }

  /**
   * Export index for persistence
   */
  export() {
    return {
      vectors: this.vectors,
    };
  }

  /**
   * Import index from persistence
   */
  import(state) {
    this.vectors = state.vectors || [];
    this.idIndex.clear();
    this.vectors.forEach((v, i) => {
      this.idIndex.set(v.id, i);
    });
    return this;
  }
}

// =============================================================================
// SEMANTIC SEARCH ENGINE
// =============================================================================

/**
 * Semantic Search Engine
 *
 * Complete semantic search with embedding and indexing.
 */
export class SemanticSearch {
  /**
   * @param {Object} options
   * @param {Object} [options.embedder] - Embedder instance (TfIdfEmbedder or ExternalEmbedder)
   * @param {Object} [options.index] - VectorIndex instance
   */
  constructor(options = {}) {
    this.embedder = options.embedder || new TfIdfEmbedder();
    this.index = options.index || new VectorIndex();
    this.documents = new Map(); // id → document text
  }

  /**
   * Initialize with documents (trains TF-IDF if using local embedder)
   * @param {Array<{ id: string, text: string, metadata?: Object }>} documents
   */
  async initialize(documents) {
    // Extract texts for training
    const texts = documents.map(d => d.text);

    // Train if TF-IDF embedder
    if (this.embedder instanceof TfIdfEmbedder) {
      this.embedder.train(texts);
    }

    // Index all documents
    for (const doc of documents) {
      await this.addDocument(doc.id, doc.text, doc.metadata);
    }
  }

  /**
   * Add a document
   * @param {string} id
   * @param {string} text
   * @param {Object} [metadata]
   */
  async addDocument(id, text, metadata = {}) {
    // Generate embedding
    const vector = await this._embed(text);

    // Store document
    this.documents.set(id, text);

    // Add to index
    this.index.add(id, vector, { ...metadata, textLength: text.length });

    // Update embedder if TF-IDF
    if (this.embedder instanceof TfIdfEmbedder && this.embedder.trained) {
      this.embedder.addDocument(text);
    }
  }

  /**
   * Search for similar documents
   * @param {string} query
   * @param {Object} [options]
   * @returns {Promise<Array<{ id: string, similarity: number, text: string, metadata: Object }>>}
   */
  async search(query, options = {}) {
    // Generate query embedding
    const queryVector = await this._embed(query);

    // Search index
    const results = this.index.search(queryVector, options);

    // Attach document text
    return results.map(r => ({
      ...r,
      text: this.documents.get(r.id) || null,
      confidence: Math.min(r.similarity, PHI_INV), // Cap at φ⁻¹
    }));
  }

  /**
   * Remove a document
   * @param {string} id
   */
  removeDocument(id) {
    this.documents.delete(id);
    this.index.remove(id);
  }

  /**
   * Generate embedding
   * @private
   */
  async _embed(text) {
    if (this.embedder.embed.constructor.name === 'AsyncFunction') {
      return this.embedder.embed(text);
    }
    return this.embedder.embed(text);
  }

  /**
   * Get stats
   */
  getStats() {
    return {
      documents: this.documents.size,
      indexSize: this.index.size(),
      embedderType: this.embedder.constructor.name,
      dimension: this.embedder.getDimension(),
    };
  }

  /**
   * Export for persistence
   */
  export() {
    return {
      embedder: this.embedder.export ? this.embedder.export() : null,
      index: this.index.export(),
      documents: Array.from(this.documents.entries()),
    };
  }

  /**
   * Import from persistence
   */
  import(state) {
    if (state.embedder && this.embedder.import) {
      this.embedder.import(state.embedder);
    }
    this.index.import(state.index);
    this.documents = new Map(state.documents);
    return this;
  }
}

// =============================================================================
// EXPORTS
// =============================================================================

export default {
  // Constants
  VECTOR_CONSTANTS,

  // Embedders
  TfIdfEmbedder,
  ExternalEmbedder,

  // Similarity
  cosineSimilarity,
  euclideanDistance,

  // Index
  VectorIndex,

  // Search Engine
  SemanticSearch,
};
