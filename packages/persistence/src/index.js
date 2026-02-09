/**
 * @cynic/persistence - Persistence Layer
 *
 * PostgreSQL + Redis + Merkle DAG for hybrid persistence.
 * Transitioning from centralized to decentralized storage.
 *
 * "Memory makes wisdom possible" - CYNIC
 *
 * @module @cynic/persistence
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// LEGACY CLIENTS (PostgreSQL + Redis)
// ═══════════════════════════════════════════════════════════════════════════

export { PostgresClient, getPool } from './postgres/client.js';
export { RedisClient, getRedis } from './redis/client.js';

// Migration
export { migrate } from './postgres/migrate.js';

// Repositories
export * from './postgres/repositories/index.js';

// Services
export * from './services/index.js';

// Repository Factory (DI pattern)
export { RepositoryFactory, createMockFactory } from './factory.js';

// Fallback Factory (auto-detect PostgreSQL/SQLite)
export { FallbackRepositoryFactory, BackendType, createFallbackFactory } from './fallback-factory.js';

// File-Backed Repository (last resort, no DB needed)
export { FileBackedRepo, createFileBackedRepo } from './file-repo.js';

// Repository Interfaces (for extension)
export { BaseRepository } from './interfaces/IRepository.js';

// Session Store
export { SessionStore } from './redis/session-store.js';

// v1.1: Batch Operations
export { BatchQueue, createTableBatchQueue, DEFAULT_BATCH_CONFIG } from './batch-queue.js';

// ═══════════════════════════════════════════════════════════════════════════
// DECENTRALIZED STORAGE (Merkle DAG)
// ═══════════════════════════════════════════════════════════════════════════

// CID Generation
export {
  createCID,
  createRawCID,
  parseCID,
  isValidCID,
  shardCID,
  compareCIDs,
  CODECS,
  HASH_FUNCTIONS,
} from './dag/cid.js';

// DAG Node Structure
export {
  DAGNode,
  DAGLink,
  NodeType,
  createJudgmentNode,
  createBlockNode,
  createEntityNode,
  createEdgeNode,
  createPatternNode,
  createIndexNode,
  createRootNode,
} from './dag/node.js';

// Block Store
export { BlockStore } from './dag/store.js';

// HAMT Index
export { HAMTIndex, HAMTEntry, HAMTBucket, HAMT_CONFIG } from './dag/hamt.js';

// Main DAG Operations
export { MerkleDAG } from './dag/dag.js';

// ═══════════════════════════════════════════════════════════════════════════
// PROOF OF JUDGMENT CHAIN (PoJ)
// ═══════════════════════════════════════════════════════════════════════════

// Block structures
export {
  PoJBlockHeader,
  PoJBlock,
  Attestation,
  JudgmentRef,
  computeMerkleRoot,
  createGenesisBlock,
  createBlock,
  POJ_CONSTANTS,
} from './poj/block.js';

// Chain manager
export { PoJChain } from './poj/chain.js';

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH OVERLAY (Relationship Graph)
// ═══════════════════════════════════════════════════════════════════════════

// Node and Edge Types
export {
  GraphNodeType,
  GraphEdgeType,
  NodeSchemas,
  EdgeSpecs,
  GraphNode,
  GraphEdge,
  createTokenNode,
  createWalletNode,
  createProjectNode,
  createRepoNode,
  createUserNode,
  createContractNode,
  createCynicNode,
  GRAPH_PHI,
} from './graph/types.js';

// Graph Store
export { GraphStore } from './graph/store.js';

// Traversal Algorithms
export { GraphTraversal, TraversalResult } from './graph/traversal.js';

// Main Graph API
export { GraphOverlay, GraphQuery } from './graph/graph.js';

// ═══════════════════════════════════════════════════════════════════════════
// VECTOR SEARCH (Semantic Search Infrastructure)
// ═══════════════════════════════════════════════════════════════════════════

// HNSW Index (fast approximate nearest neighbor)
export {
  HNSWIndex,
  HNSWNode,
  HNSW_CONFIG,
  createHNSWIndex,
} from './vector/hnsw.js';

// Vector Store (embeddings + search)
export {
  VectorStore,
  VECTOR_STORE_CONFIG,
  createVectorStore,
  getVectorStore,
} from './vector/store.js';

// Semantic Pattern Matching (V3)
export {
  SemanticPatternMatcher,
  SemanticPattern,
  PatternCluster,
  SEMANTIC_PATTERN_CONFIG,
  createSemanticPatternMatcher,
  getSemanticPatternMatcher,
} from './vector/semantic-patterns.js';

// ═══════════════════════════════════════════════════════════════════════════
// LOCAL PRIVACY STORES (SQLite - never syncs by default)
// "Your data, your device, your choice" - κυνικός
// ═══════════════════════════════════════════════════════════════════════════

// X/Twitter local store (tweets, users, trends)
export { LocalXStore } from './sqlite/LocalXStore.js';

// Unified privacy store (E-Score, Learning, Psychology, Patterns, Sessions)
export { LocalPrivacyStore, SyncStatus } from './sqlite/LocalPrivacyStore.js';

// ═══════════════════════════════════════════════════════════════════════════
// VERSION
// ═══════════════════════════════════════════════════════════════════════════

export const VERSION = '0.4.0';
