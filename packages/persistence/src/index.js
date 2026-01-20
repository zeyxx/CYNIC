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

// Session Store
export { SessionStore } from './redis/session-store.js';

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
// VERSION
// ═══════════════════════════════════════════════════════════════════════════

export const VERSION = '0.4.0';
