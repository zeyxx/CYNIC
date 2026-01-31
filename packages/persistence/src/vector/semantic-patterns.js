/**
 * Semantic Pattern Matcher - V3 Integration
 *
 * Bridges VectorStore with CYNIC's pattern detection system.
 * Enables finding patterns by meaning, not just keywords.
 *
 * "Similar problems have similar patterns" - κυνικός
 *
 * @module @cynic/persistence/vector/semantic-patterns
 */

'use strict';

import { createVectorStore, VectorStore } from './store.js';

// φ constants for similarity thresholds
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

/**
 * Configuration for semantic pattern matching
 */
export const SEMANTIC_PATTERN_CONFIG = {
  // Similarity thresholds
  clusterThreshold: PHI_INV,      // 61.8% - patterns this similar form clusters
  matchThreshold: PHI_INV_2,      // 38.2% - minimum to be considered a match

  // Index settings
  dimensions: 384,                 // Default embedding dimensions
  embedder: 'mock',               // 'mock' | 'openai' | 'ollama'

  // Clustering
  minClusterSize: 2,              // Minimum patterns for a cluster
  maxClusters: 21,                // Fibonacci - max clusters to track
};

/**
 * Pattern representation for vector storage
 */
export class SemanticPattern {
  /**
   * @param {string} id - Unique pattern ID
   * @param {string} description - Natural language description
   * @param {Object} metadata - Pattern metadata
   */
  constructor(id, description, metadata = {}) {
    this.id = id;
    this.description = description;
    this.metadata = {
      ...metadata,
      createdAt: metadata.createdAt || Date.now(),
      occurrences: metadata.occurrences || 1,
      confidence: metadata.confidence || PHI_INV,
    };
  }

  /**
   * Convert pattern to storable format
   */
  toStorable() {
    return {
      id: this.id,
      text: this.description,
      metadata: this.metadata,
    };
  }

  /**
   * Create from search result
   */
  static fromSearchResult(result) {
    return new SemanticPattern(
      result.id,
      result.text,
      result.metadata
    );
  }
}

/**
 * Pattern cluster - group of semantically similar patterns
 */
export class PatternCluster {
  /**
   * @param {string} id - Cluster ID
   * @param {SemanticPattern} centroid - Representative pattern
   * @param {SemanticPattern[]} members - Member patterns
   */
  constructor(id, centroid, members = []) {
    this.id = id;
    this.centroid = centroid;
    this.members = members;
    this.createdAt = Date.now();
  }

  /**
   * Get cluster size
   */
  get size() {
    return this.members.length;
  }

  /**
   * Get cluster theme (centroid description)
   */
  get theme() {
    return this.centroid.description;
  }

  /**
   * Add member to cluster
   */
  addMember(pattern) {
    if (!this.members.find(m => m.id === pattern.id)) {
      this.members.push(pattern);
    }
  }

  /**
   * Export cluster info
   */
  toJSON() {
    return {
      id: this.id,
      theme: this.theme,
      size: this.size,
      centroid: this.centroid.id,
      members: this.members.map(m => m.id),
      createdAt: this.createdAt,
    };
  }
}

/**
 * Semantic Pattern Matcher
 *
 * Main service for semantic pattern operations.
 *
 * @example
 * const matcher = createSemanticPatternMatcher();
 *
 * // Add patterns
 * await matcher.addPattern('p1', 'User prefers tabs over spaces');
 * await matcher.addPattern('p2', 'User likes indentation with tabs');
 *
 * // Find similar
 * const similar = await matcher.findSimilar('indentation preferences');
 * // Returns patterns about tabs/spaces
 *
 * // Cluster patterns
 * const clusters = await matcher.clusterPatterns();
 */
export class SemanticPatternMatcher {
  /**
   * @param {Object} options
   * @param {VectorStore} [options.store] - Existing VectorStore instance
   * @param {Object} [options.config] - Override SEMANTIC_PATTERN_CONFIG
   */
  constructor(options = {}) {
    this.config = { ...SEMANTIC_PATTERN_CONFIG, ...options.config };

    // Use provided store or create new one
    this._store = options.store || createVectorStore({
      embedder: this.config.embedder,
      dimensions: this.config.dimensions,
    });

    // Pattern index: id -> SemanticPattern
    this._patterns = new Map();

    // Cluster cache
    this._clusters = new Map();
    this._clustersDirty = true;

    // Statistics
    this._stats = {
      added: 0,
      searches: 0,
      clusterings: 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Pattern Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Add a pattern to the semantic index
   *
   * @param {string} id - Pattern ID
   * @param {string} description - Natural language description
   * @param {Object} [metadata] - Additional metadata
   * @returns {Promise<SemanticPattern>}
   */
  async addPattern(id, description, metadata = {}) {
    const pattern = new SemanticPattern(id, description, metadata);

    // Store in vector store
    await this._store.store(id, description, {
      ...metadata,
      _patternId: id,
    });

    // Track locally
    this._patterns.set(id, pattern);
    this._clustersDirty = true;
    this._stats.added++;

    return pattern;
  }

  /**
   * Add multiple patterns in batch
   *
   * @param {Array<{id: string, description: string, metadata?: Object}>} patterns
   * @returns {Promise<number>} Count added
   */
  async addPatterns(patterns) {
    let count = 0;
    for (const p of patterns) {
      await this.addPattern(p.id, p.description, p.metadata || {});
      count++;
    }
    return count;
  }

  /**
   * Remove a pattern
   *
   * @param {string} id - Pattern ID
   * @returns {boolean}
   */
  removePattern(id) {
    const removed = this._store.delete(id);
    this._patterns.delete(id);
    if (removed) {
      this._clustersDirty = true;
    }
    return removed;
  }

  /**
   * Get pattern by ID
   *
   * @param {string} id - Pattern ID
   * @returns {SemanticPattern|null}
   */
  getPattern(id) {
    return this._patterns.get(id) || null;
  }

  /**
   * Check if pattern exists
   *
   * @param {string} id - Pattern ID
   * @returns {boolean}
   */
  hasPattern(id) {
    return this._patterns.has(id);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Semantic Search
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Find patterns similar to a query
   *
   * @param {string} query - Natural language query
   * @param {number} [k=10] - Number of results
   * @param {Object} [options] - Search options
   * @returns {Promise<Array<{pattern: SemanticPattern, score: number}>>}
   */
  async findSimilar(query, k = 10, options = {}) {
    this._stats.searches++;

    const minScore = options.minScore ?? this.config.matchThreshold;
    const results = await this._store.search(query, k, {
      minScore,
      filter: options.filter,
    });

    return results.map(r => ({
      pattern: SemanticPattern.fromSearchResult(r),
      score: r.score,
    }));
  }

  /**
   * Find patterns similar to an existing pattern
   *
   * @param {string} patternId - Pattern to find similar to
   * @param {number} [k=10] - Number of results
   * @returns {Promise<Array<{pattern: SemanticPattern, score: number}>>}
   */
  async findSimilarToPattern(patternId, k = 10) {
    const pattern = this._patterns.get(patternId);
    if (!pattern) {
      throw new Error(`Pattern not found: ${patternId}`);
    }

    const results = await this._store.searchSimilar(patternId, k);

    return results.map(r => ({
      pattern: SemanticPattern.fromSearchResult(r),
      score: r.score,
    }));
  }

  /**
   * Check if a description matches any existing pattern
   *
   * @param {string} description - Description to check
   * @param {number} [threshold] - Similarity threshold
   * @returns {Promise<SemanticPattern|null>} Matching pattern or null
   */
  async matchExisting(description, threshold = this.config.clusterThreshold) {
    const results = await this.findSimilar(description, 1, {
      minScore: threshold,
    });

    if (results.length > 0) {
      return results[0].pattern;
    }
    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Clustering
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Cluster patterns by semantic similarity
   *
   * Uses simple greedy clustering:
   * 1. Sort patterns by occurrence count
   * 2. For each pattern, find if it fits in existing cluster
   * 3. If not, create new cluster with pattern as centroid
   *
   * @param {Object} [options] - Clustering options
   * @returns {Promise<PatternCluster[]>}
   */
  async clusterPatterns(options = {}) {
    const {
      threshold = this.config.clusterThreshold,
      minSize = this.config.minClusterSize,
      maxClusters = this.config.maxClusters,
    } = options;

    this._stats.clusterings++;

    // Return cached if not dirty
    if (!this._clustersDirty && this._clusters.size > 0) {
      return [...this._clusters.values()];
    }

    // Clear clusters
    this._clusters.clear();

    // Sort patterns by occurrences (most frequent first = better centroids)
    const sortedPatterns = [...this._patterns.values()].sort(
      (a, b) => (b.metadata.occurrences || 0) - (a.metadata.occurrences || 0)
    );

    // Greedy clustering
    let clusterId = 0;
    for (const pattern of sortedPatterns) {
      // Try to find matching cluster
      let assigned = false;

      for (const cluster of this._clusters.values()) {
        // Check similarity to centroid
        const similar = await this.findSimilar(pattern.description, 1, {
          minScore: threshold,
          filter: (m) => m._patternId === cluster.centroid.id,
        });

        if (similar.length > 0) {
          cluster.addMember(pattern);
          assigned = true;
          break;
        }
      }

      // Create new cluster if not assigned
      if (!assigned && this._clusters.size < maxClusters) {
        const cluster = new PatternCluster(
          `cluster-${++clusterId}`,
          pattern,
          [pattern]
        );
        this._clusters.set(cluster.id, cluster);
      }
    }

    // Filter out small clusters
    for (const [id, cluster] of this._clusters) {
      if (cluster.size < minSize) {
        this._clusters.delete(id);
      }
    }

    this._clustersDirty = false;
    return [...this._clusters.values()];
  }

  /**
   * Get clusters (cached)
   *
   * @returns {PatternCluster[]}
   */
  getClusters() {
    return [...this._clusters.values()];
  }

  /**
   * Get cluster by ID
   *
   * @param {string} id - Cluster ID
   * @returns {PatternCluster|null}
   */
  getCluster(id) {
    return this._clusters.get(id) || null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Integration Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Extract patterns from judgment data
   *
   * @param {Object} judgment - Judgment object
   * @returns {Promise<SemanticPattern[]>} Extracted patterns
   */
  async extractFromJudgment(judgment) {
    const patterns = [];

    // Extract from verdict reasoning
    if (judgment.reasoning) {
      const pattern = await this.addPattern(
        `judgment-${judgment.id}-reasoning`,
        judgment.reasoning,
        {
          source: 'judgment',
          judgmentId: judgment.id,
          confidence: judgment.score || PHI_INV,
        }
      );
      patterns.push(pattern);
    }

    // Extract from domain notes
    if (judgment.dimensions) {
      for (const [dim, data] of Object.entries(judgment.dimensions)) {
        if (data.notes) {
          const pattern = await this.addPattern(
            `judgment-${judgment.id}-${dim}`,
            data.notes,
            {
              source: 'judgment',
              judgmentId: judgment.id,
              dimension: dim,
              confidence: data.score || PHI_INV,
            }
          );
          patterns.push(pattern);
        }
      }
    }

    return patterns;
  }

  /**
   * Recommend patterns for a new context
   *
   * @param {string} context - Current context description
   * @param {number} [k=5] - Number of recommendations
   * @returns {Promise<Array<{pattern: SemanticPattern, relevance: number}>>}
   */
  async recommendPatterns(context, k = 5) {
    const results = await this.findSimilar(context, k);

    // Return sorted by relevance (score * confidence)
    return results
      .map(r => ({
        pattern: r.pattern,
        relevance: r.score * (r.pattern.metadata.confidence || PHI_INV),
      }))
      .sort((a, b) => b.relevance - a.relevance);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Statistics & Persistence
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get matcher statistics
   */
  stats() {
    return {
      ...this._stats,
      patterns: this._patterns.size,
      clusters: this._clusters.size,
      store: this._store.stats(),
    };
  }

  /**
   * Get all pattern IDs
   */
  patternIds() {
    return [...this._patterns.keys()];
  }

  /**
   * Clear all data
   */
  clear() {
    this._store.clear();
    this._patterns.clear();
    this._clusters.clear();
    this._clustersDirty = true;
    this._stats = {
      added: 0,
      searches: 0,
      clusterings: 0,
    };
  }

  /**
   * Export state for persistence
   */
  toJSON() {
    return {
      config: this.config,
      patterns: [...this._patterns.values()].map(p => p.toStorable()),
      clusters: [...this._clusters.values()].map(c => c.toJSON()),
      stats: this._stats,
      store: this._store.toJSON(),
    };
  }

  /**
   * Import state from persistence
   */
  static fromJSON(json) {
    const matcher = new SemanticPatternMatcher({
      config: json.config,
      store: VectorStore.fromJSON(json.store),
    });

    // Restore patterns
    for (const p of json.patterns) {
      matcher._patterns.set(p.id, new SemanticPattern(p.id, p.text, p.metadata));
    }

    // Restore stats
    matcher._stats = json.stats;
    matcher._clustersDirty = true;

    return matcher;
  }
}

/**
 * Create a new SemanticPatternMatcher
 *
 * @param {Object} [options] - Matcher options
 * @returns {SemanticPatternMatcher}
 */
export function createSemanticPatternMatcher(options = {}) {
  return new SemanticPatternMatcher(options);
}

// Singleton instance
let _instance = null;

/**
 * Get or create singleton SemanticPatternMatcher
 *
 * @param {Object} [options] - Matcher options (only used on first call)
 * @returns {SemanticPatternMatcher}
 */
export function getSemanticPatternMatcher(options = {}) {
  if (!_instance) {
    _instance = createSemanticPatternMatcher(options);
  }
  return _instance;
}

export default SemanticPatternMatcher;
