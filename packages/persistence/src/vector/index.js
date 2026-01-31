/**
 * Vector Module - Semantic Search Infrastructure
 *
 * Combines embeddings with HNSW for fast semantic similarity search.
 *
 * V1-V3: Complete vector search implementation
 *
 * "Memory needs meaning" - κυνικός
 *
 * @module @cynic/persistence/vector
 */

'use strict';

export {
  HNSWIndex,
  HNSWNode,
  HNSW_CONFIG,
  createHNSWIndex,
} from './hnsw.js';

export {
  VectorStore,
  VECTOR_STORE_CONFIG,
  createVectorStore,
  getVectorStore,
} from './store.js';

// V3: Semantic Pattern Matching
export {
  SemanticPatternMatcher,
  SemanticPattern,
  PatternCluster,
  SEMANTIC_PATTERN_CONFIG,
  createSemanticPatternMatcher,
  getSemanticPatternMatcher,
} from './semantic-patterns.js';
