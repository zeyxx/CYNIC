/**
 * Merkle DAG Operations
 *
 * High-level API for content-addressable storage operations.
 * Provides put, get, resolve, and traversal functionality.
 *
 * "Truth is immutable, references are paths" - κυνικός
 *
 * @module @cynic/persistence/dag/dag
 */

'use strict';

import { EventEmitter } from 'events';
import { BlockStore } from './store.js';
import { HAMTIndex } from './hamt.js';
import {
  DAGNode,
  DAGLink,
  NodeType,
  createJudgmentNode,
  createBlockNode,
  createEntityNode,
  createEdgeNode,
  createPatternNode,
  createRootNode,
} from './node.js';
import { isValidCID, compareCIDs } from './cid.js';

// φ constant for timing and batching
const PHI = 1.618033988749895;
const BATCH_SIZE = 13; // Fibonacci number

/**
 * Merkle DAG - Content-addressable storage with indices
 */
export class MerkleDAG extends EventEmitter {
  /**
   * @param {Object} [config] - Configuration options
   */
  constructor(config = {}) {
    super();

    this.config = {
      basePath: config.basePath || './data',
      nodeId: config.nodeId || `node_${Date.now().toString(36)}`,
      ...config,
    };

    // Initialize stores
    this.store = new BlockStore({
      basePath: `${this.config.basePath}/blocks`,
    });

    // Indices for different data types
    this._indices = {};
    this._rootCid = null;
    this._initialized = false;
  }

  /**
   * Initialize the DAG
   * @param {string} [rootCid] - Optional existing root CID
   */
  async init(rootCid = null) {
    if (this._initialized) return;

    await this.store.init();

    // Load or create root
    if (rootCid) {
      await this._loadRoot(rootCid);
    } else {
      await this._createRoot();
    }

    this._initialized = true;
    this.emit('initialized', { rootCid: this._rootCid });
  }

  /**
   * Load existing root
   * @private
   */
  async _loadRoot(rootCid) {
    const rootNode = await this.store.getNode(rootCid);
    if (!rootNode || rootNode.type !== NodeType.ROOT) {
      throw new Error('Invalid root node');
    }

    this._rootCid = rootCid;

    // Load indices from root links
    for (const link of rootNode.links) {
      if (link.name.endsWith('_index')) {
        const indexName = link.name.replace('_index', '');
        this._indices[indexName] = new HAMTIndex(this.store, link.cid);
        await this._indices[indexName].init();
      }
    }
  }

  /**
   * Create new root
   * @private
   */
  async _createRoot() {
    // Initialize empty indices
    const indexTypes = ['judgments', 'entities', 'patterns', 'sessions'];

    for (const indexType of indexTypes) {
      this._indices[indexType] = new HAMTIndex(this.store);
      await this._indices[indexType].init();
    }

    // Create and save root node
    await this._saveRoot();
  }

  /**
   * Save root node
   * @private
   */
  async _saveRoot() {
    const links = {};

    // Add index links
    for (const [name, index] of Object.entries(this._indices)) {
      if (index.rootCid) {
        links[`${name}_index`] = index.rootCid;
      }
    }

    const root = createRootNode(
      {
        version: 1,
        nodeId: this.config.nodeId,
      },
      links
    );

    this._rootCid = await this.store.putNode(root);
    return this._rootCid;
  }

  /**
   * Get the current root CID
   * @returns {string} Root CID
   */
  get rootCid() {
    return this._rootCid;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUT OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Put a raw DAG node
   * @param {DAGNode} node - Node to store
   * @returns {Promise<string>} Node CID
   */
  async put(node) {
    const cid = await this.store.putNode(node);
    this.emit('put', { cid, type: node.type });
    return cid;
  }

  /**
   * Put a judgment
   * @param {Object} judgment - Judgment data
   * @returns {Promise<string>} Judgment CID
   */
  async putJudgment(judgment) {
    const node = createJudgmentNode(judgment);
    const cid = await this.put(node);

    // Index by judgment ID
    await this._indices.judgments.set(node.data.id, cid, {
      qScore: node.data.qScore,
      verdict: node.data.verdict,
      timestamp: node.metadata.created,
    });

    await this._saveRoot();
    return cid;
  }

  /**
   * Put an entity
   * @param {Object} entity - Entity data
   * @returns {Promise<string>} Entity CID
   */
  async putEntity(entity) {
    const node = createEntityNode(entity);
    const cid = await this.put(node);

    // Index by entity type and identifier
    const key = `${entity.entityType}:${entity.identifier}`;
    await this._indices.entities.set(key, cid, {
      name: entity.name,
      entityType: entity.entityType,
    });

    await this._saveRoot();
    return cid;
  }

  /**
   * Put an edge (relationship)
   * @param {Object} edge - Edge data
   * @param {string} sourceCid - Source entity CID
   * @param {string} targetCid - Target entity CID
   * @returns {Promise<string>} Edge CID
   */
  async putEdge(edge, sourceCid, targetCid) {
    const node = createEdgeNode(edge, sourceCid, targetCid);
    const cid = await this.put(node);

    // No index for edges (traversed from entities)
    return cid;
  }

  /**
   * Put a pattern
   * @param {Object} pattern - Pattern data
   * @returns {Promise<string>} Pattern CID
   */
  async putPattern(pattern) {
    const node = createPatternNode(pattern);
    const cid = await this.put(node);

    // Index by pattern category and name
    const key = `${pattern.category}:${pattern.name}`;
    await this._indices.patterns.set(key, cid, {
      frequency: pattern.frequency,
    });

    await this._saveRoot();
    return cid;
  }

  /**
   * Put multiple items in batch
   * @param {DAGNode[]} nodes - Nodes to store
   * @returns {Promise<string[]>} Array of CIDs
   */
  async putBatch(nodes) {
    const cids = [];

    for (let i = 0; i < nodes.length; i += BATCH_SIZE) {
      const batch = nodes.slice(i, i + BATCH_SIZE);
      const batchCids = await Promise.all(
        batch.map(node => this.store.putNode(node))
      );
      cids.push(...batchCids);
    }

    this.emit('putBatch', { count: cids.length });
    return cids;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // GET OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get a node by CID
   * @param {string} cid - Content identifier
   * @returns {Promise<DAGNode|null>} Node or null
   */
  async get(cid) {
    if (!isValidCID(cid)) return null;
    return this.store.getNode(cid);
  }

  /**
   * Get a judgment by ID
   * @param {string} judgmentId - Judgment ID
   * @returns {Promise<DAGNode|null>} Judgment node or null
   */
  async getJudgment(judgmentId) {
    const cid = await this._indices.judgments.get(judgmentId);
    if (!cid) return null;
    return this.get(cid);
  }

  /**
   * Get an entity by type and identifier
   * @param {string} entityType - Entity type
   * @param {string} identifier - Entity identifier
   * @returns {Promise<DAGNode|null>} Entity node or null
   */
  async getEntity(entityType, identifier) {
    const key = `${entityType}:${identifier}`;
    const cid = await this._indices.entities.get(key);
    if (!cid) return null;
    return this.get(cid);
  }

  /**
   * Get a pattern by category and name
   * @param {string} category - Pattern category
   * @param {string} name - Pattern name
   * @returns {Promise<DAGNode|null>} Pattern node or null
   */
  async getPattern(category, name) {
    const key = `${category}:${name}`;
    const cid = await this._indices.patterns.get(key);
    if (!cid) return null;
    return this.get(cid);
  }

  /**
   * Get multiple nodes by CIDs
   * @param {string[]} cids - Content identifiers
   * @returns {Promise<Map<string, DAGNode>>} Map of CID to node
   */
  async getMany(cids) {
    const results = new Map();

    await Promise.all(
      cids.map(async cid => {
        const node = await this.get(cid);
        if (node) results.set(cid, node);
      })
    );

    return results;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // RESOLVE OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Resolve a path from a node
   * @param {string} cid - Starting CID
   * @param {string} path - Path like "links/0/cid" or "data/qScore"
   * @returns {Promise<any>} Resolved value
   */
  async resolve(cid, path) {
    const node = await this.get(cid);
    if (!node) return null;

    const parts = path.split('/').filter(p => p);
    let current = node;

    for (const part of parts) {
      if (current === null || current === undefined) return null;

      // Handle array index
      if (Array.isArray(current)) {
        const index = parseInt(part, 10);
        if (isNaN(index)) return null;
        current = current[index];
      }
      // Handle object property
      else if (typeof current === 'object') {
        current = current[part];
      }
      // Handle DAGNode properties
      else if (current instanceof DAGNode) {
        if (part === 'data') current = current.data;
        else if (part === 'links') current = current.links;
        else if (part === 'metadata') current = current.metadata;
        else if (part === 'type') current = current.type;
        else if (part === 'cid') current = current.cid;
        else return null;
      }
      else {
        return null;
      }

      // If we hit a CID (link), resolve it
      if (typeof current === 'object' && current && current['/']) {
        const linkedCid = current['/'];
        current = await this.get(linkedCid);
      }
    }

    return current;
  }

  /**
   * Get the tree structure from a node
   * @param {string} cid - Root CID
   * @param {number} [maxDepth=3] - Maximum depth to traverse
   * @returns {Promise<Object>} Tree structure
   */
  async tree(cid, maxDepth = 3) {
    return this._buildTree(cid, 0, maxDepth, new Set());
  }

  /**
   * Build tree recursively
   * @private
   */
  async _buildTree(cid, depth, maxDepth, visited) {
    if (depth > maxDepth || visited.has(cid)) {
      return { cid, truncated: true };
    }

    visited.add(cid);
    const node = await this.get(cid);
    if (!node) return { cid, error: 'not found' };

    const result = {
      cid,
      type: node.type,
      data: node.data,
    };

    if (node.links.length > 0) {
      result.links = await Promise.all(
        node.links.map(async link => ({
          name: link.name,
          size: link.size,
          child: await this._buildTree(link.cid, depth + 1, maxDepth, visited),
        }))
      );
    }

    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRAVERSAL OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Traverse all linked nodes from a starting point
   * @param {string} cid - Starting CID
   * @param {Object} [options] - Traversal options
   * @yields {DAGNode} Visited nodes
   */
  async *traverse(cid, options = {}) {
    const {
      maxDepth = Infinity,
      filter = null,
      order = 'bfs', // 'bfs' or 'dfs'
    } = options;

    const visited = new Set();
    const queue = [{ cid, depth: 0 }];

    while (queue.length > 0) {
      const { cid: currentCid, depth } = order === 'bfs'
        ? queue.shift()
        : queue.pop();

      if (visited.has(currentCid) || depth > maxDepth) continue;
      visited.add(currentCid);

      const node = await this.get(currentCid);
      if (!node) continue;

      // Apply filter
      if (filter && !filter(node)) continue;

      yield node;

      // Add linked nodes to queue
      for (const link of node.links) {
        if (!visited.has(link.cid)) {
          queue.push({ cid: link.cid, depth: depth + 1 });
        }
      }
    }
  }

  /**
   * Find all nodes of a specific type
   * @param {string} type - Node type
   * @returns {AsyncGenerator<DAGNode>} Matching nodes
   */
  async *findByType(type) {
    for await (const { cid } of this.store.iterate()) {
      const node = await this.get(cid);
      if (node && node.type === type) {
        yield node;
      }
    }
  }

  /**
   * Diff two DAG roots
   * @param {string} cid1 - First root CID
   * @param {string} cid2 - Second root CID
   * @returns {Promise<Object>} Diff result
   */
  async diff(cid1, cid2) {
    const visited1 = new Set();
    const visited2 = new Set();

    // Collect all CIDs reachable from each root
    for await (const node of this.traverse(cid1)) {
      visited1.add(node.cid);
    }

    for await (const node of this.traverse(cid2)) {
      visited2.add(node.cid);
    }

    const added = [...visited2].filter(cid => !visited1.has(cid));
    const removed = [...visited1].filter(cid => !visited2.has(cid));
    const common = [...visited1].filter(cid => visited2.has(cid));

    return {
      added,
      removed,
      common: common.length,
      totalIn1: visited1.size,
      totalIn2: visited2.size,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // UTILITY OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get statistics
   * @returns {Promise<Object>} DAG statistics
   */
  async stats() {
    const storeStats = await this.store.stats();

    const indexStats = {};
    for (const [name, index] of Object.entries(this._indices)) {
      indexStats[name] = await index.stats();
    }

    return {
      rootCid: this._rootCid,
      nodeId: this.config.nodeId,
      store: storeStats,
      indices: indexStats,
    };
  }

  /**
   * Verify integrity
   * @returns {Promise<Object>} Verification results
   */
  async verify() {
    return this.store.verify();
  }

  /**
   * Export DAG to file
   * @param {string} outputPath - Output file path
   * @returns {Promise<Object>} Export stats
   */
  async export(outputPath) {
    return this.store.export([this._rootCid], outputPath);
  }

  /**
   * Import DAG from file
   * @param {string} inputPath - Input file path
   * @returns {Promise<Object>} Import stats
   */
  async import(inputPath) {
    return this.store.import(inputPath);
  }
}

export {
  DAGNode,
  DAGLink,
  NodeType,
  BlockStore,
  HAMTIndex,
};

export default MerkleDAG;
