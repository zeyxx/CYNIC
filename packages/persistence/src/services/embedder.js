/**
 * Embedder Service - Vector Embedding Generation
 *
 * Pluggable embedding service for Total Memory system.
 * Supports: OpenAI, Local models (future), or mock for testing.
 *
 * "Memory needs meaning" - CYNIC
 *
 * @module @cynic/persistence/services/embedder
 */

'use strict';

// φ constants for embedding normalization
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;

/**
 * Embedding dimensions (OpenAI ada-002 compatible)
 */
export const EMBEDDING_DIMENSIONS = 1536;

/**
 * Embedder types
 */
export const EmbedderType = {
  OPENAI: 'openai',
  LOCAL: 'local',
  MOCK: 'mock',
};

/**
 * Base Embedder class
 */
export class Embedder {
  constructor(options = {}) {
    this.dimensions = options.dimensions || EMBEDDING_DIMENSIONS;
    this.type = options.type || EmbedderType.MOCK;
    this.cache = new Map();
    this.cacheMaxSize = options.cacheMaxSize || 1000;
  }

  /**
   * Generate embedding for text
   * @param {string} text - Text to embed
   * @returns {Promise<number[]>} Embedding vector
   */
  async embed(text) {
    throw new Error('embed() must be implemented by subclass');
  }

  /**
   * Generate embeddings for multiple texts
   * @param {string[]} texts - Texts to embed
   * @returns {Promise<number[][]>} Array of embedding vectors
   */
  async embedBatch(texts) {
    return Promise.all(texts.map(t => this.embed(t)));
  }

  /**
   * Calculate cosine similarity between two embeddings
   * @param {number[]} a - First embedding
   * @param {number[]} b - Second embedding
   * @returns {number} Similarity score (0-1)
   */
  cosineSimilarity(a, b) {
    if (a.length !== b.length) {
      throw new Error('Embedding dimensions must match');
    }

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const magnitude = Math.sqrt(normA) * Math.sqrt(normB);
    if (magnitude === 0) return 0;

    return dotProduct / magnitude;
  }

  /**
   * Get cached embedding or generate new one
   * @param {string} text - Text to embed
   * @returns {Promise<number[]>} Embedding vector
   */
  async embedWithCache(text) {
    const cacheKey = this._hashText(text);

    if (this.cache.has(cacheKey)) {
      return this.cache.get(cacheKey);
    }

    const embedding = await this.embed(text);

    // LRU-style cache management
    if (this.cache.size >= this.cacheMaxSize) {
      const firstKey = this.cache.keys().next().value;
      this.cache.delete(firstKey);
    }

    this.cache.set(cacheKey, embedding);
    return embedding;
  }

  /**
   * Simple hash for cache key
   * @private
   */
  _hashText(text) {
    let hash = 0;
    for (let i = 0; i < text.length; i++) {
      const char = text.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32bit integer
    }
    return hash.toString(36);
  }
}

/**
 * Mock Embedder for testing and offline use
 * Generates deterministic pseudo-embeddings based on text content
 */
export class MockEmbedder extends Embedder {
  constructor(options = {}) {
    super({ ...options, type: EmbedderType.MOCK });
  }

  /**
   * Generate deterministic mock embedding
   * Uses text characteristics to create a consistent vector
   */
  async embed(text) {
    const normalized = text.toLowerCase().trim();
    const embedding = new Array(this.dimensions).fill(0);

    // Use character frequencies and positions for determinism
    for (let i = 0; i < normalized.length; i++) {
      const char = normalized.charCodeAt(i);
      const idx = (char * (i + 1)) % this.dimensions;
      embedding[idx] += Math.sin(char * PHI_INV) * 0.1;
    }

    // Add word-level features
    const words = normalized.split(/\s+/);
    for (let w = 0; w < words.length; w++) {
      const word = words[w];
      const wordHash = this._wordHash(word);
      const idx = wordHash % this.dimensions;
      embedding[idx] += Math.cos(wordHash * PHI_INV) * (1 / (w + 1));
    }

    // Normalize to unit vector
    const norm = Math.sqrt(embedding.reduce((sum, v) => sum + v * v, 0));
    if (norm > 0) {
      for (let i = 0; i < embedding.length; i++) {
        embedding[i] /= norm;
      }
    }

    return embedding;
  }

  /**
   * Simple word hash
   * @private
   */
  _wordHash(word) {
    let hash = 0;
    for (let i = 0; i < word.length; i++) {
      hash = ((hash << 5) - hash) + word.charCodeAt(i);
      hash = hash & hash;
    }
    return Math.abs(hash);
  }
}

/**
 * OpenAI Embedder using text-embedding-ada-002
 */
export class OpenAIEmbedder extends Embedder {
  constructor(options = {}) {
    super({ ...options, type: EmbedderType.OPENAI });
    this.apiKey = options.apiKey || process.env.OPENAI_API_KEY;
    this.model = options.model || 'text-embedding-ada-002';
    this.baseUrl = options.baseUrl || 'https://api.openai.com/v1';

    if (!this.apiKey) {
      throw new Error('OpenAI API key required (pass apiKey or set OPENAI_API_KEY)');
    }
  }

  /**
   * Generate embedding via OpenAI API
   */
  async embed(text) {
    const response = await fetch(`${this.baseUrl}/embeddings`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
      },
      body: JSON.stringify({
        model: this.model,
        input: text,
      }),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({}));
      throw new Error(`OpenAI API error: ${error.error?.message || response.statusText}`);
    }

    const data = await response.json();
    return data.data[0].embedding;
  }

  /**
   * Batch embed multiple texts (more efficient API usage)
   */
  async embedBatch(texts) {
    if (texts.length === 0) return [];
    if (texts.length === 1) return [await this.embed(texts[0])];

    // OpenAI supports batching up to 2048 inputs
    const batchSize = Math.min(texts.length, 2048);
    const batches = [];

    for (let i = 0; i < texts.length; i += batchSize) {
      batches.push(texts.slice(i, i + batchSize));
    }

    const results = [];
    for (const batch of batches) {
      const response = await fetch(`${this.baseUrl}/embeddings`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.apiKey}`,
        },
        body: JSON.stringify({
          model: this.model,
          input: batch,
        }),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(`OpenAI API error: ${error.error?.message || response.statusText}`);
      }

      const data = await response.json();
      // Sort by index to maintain order
      const sorted = data.data.sort((a, b) => a.index - b.index);
      results.push(...sorted.map(d => d.embedding));
    }

    return results;
  }
}

/**
 * Create embedder based on environment and options
 *
 * Default: MockEmbedder (free, local, deterministic)
 * Set CYNIC_EMBEDDER=openai to use OpenAI (requires OPENAI_API_KEY)
 *
 * @param {Object} options - Embedder options
 * @returns {Embedder} Embedder instance
 */
export function createEmbedder(options = {}) {
  // Default to mock - opt-in to OpenAI via CYNIC_EMBEDDER=openai
  const envType = process.env.CYNIC_EMBEDDER?.toLowerCase();
  const type = options.type || (envType === 'openai' ? EmbedderType.OPENAI : EmbedderType.MOCK);

  switch (type) {
    case EmbedderType.OPENAI:
      return new OpenAIEmbedder(options);
    case EmbedderType.LOCAL:
      // Future: Support for local models (e.g., sentence-transformers)
      console.warn('Local embedder not yet implemented, falling back to mock');
      return new MockEmbedder(options);
    case EmbedderType.MOCK:
    default:
      return new MockEmbedder(options);
  }
}

/**
 * Singleton embedder instance
 */
let _embedder = null;

/**
 * Get or create singleton embedder
 * @param {Object} options - Embedder options
 * @returns {Embedder} Embedder instance
 */
export function getEmbedder(options = {}) {
  if (!_embedder) {
    _embedder = createEmbedder(options);
  }
  return _embedder;
}

/**
 * Reset singleton (for testing)
 */
export function resetEmbedder() {
  _embedder = null;
}
