/**
 * Merkle DAG Module
 *
 * Content-addressable storage for CYNIC's decentralized architecture.
 *
 * "Every truth has an address, every relationship a link" - κυνικός
 *
 * @module @cynic/persistence/dag
 */

'use strict';

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
} from './cid.js';

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
} from './node.js';

// Block Store
export { BlockStore } from './store.js';

// HAMT Index
export { HAMTIndex, HAMTEntry, HAMTBucket, HAMT_CONFIG } from './hamt.js';

// Main DAG Operations
export { MerkleDAG } from './dag.js';

// Default export
export { default } from './dag.js';
