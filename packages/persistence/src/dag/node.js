/**
 * DAG Node Structure
 *
 * Defines the structure for Merkle DAG nodes with CBOR encoding.
 * Each node contains data, links to other nodes, and metadata.
 *
 * "Every node is a truth, every link a relationship" - κυνικός
 *
 * @module @cynic/persistence/dag/node
 */

'use strict';

import { encode, decode } from 'cbor-x';
import { createCID } from './cid.js';

// Node types in the DAG
export const NodeType = {
  JUDGMENT: 'judgment',
  BLOCK: 'block',
  ENTITY: 'entity',
  EDGE: 'edge',
  PATTERN: 'pattern',
  SESSION: 'session',
  FEEDBACK: 'feedback',
  INDEX: 'index',
  ROOT: 'root',
};

/**
 * DAG Link - reference to another node
 */
export class DAGLink {
  /**
   * @param {string} cid - CID of linked node
   * @param {string} [name] - Optional name for the link
   * @param {number} [size] - Size of linked content in bytes
   */
  constructor(cid, name = '', size = 0) {
    this.cid = cid;
    this.name = name;
    this.size = size;
  }

  toJSON() {
    return {
      '/': this.cid,
      name: this.name || undefined,
      size: this.size || undefined,
    };
  }

  static fromJSON(json) {
    return new DAGLink(json['/'], json.name, json.size);
  }
}

/**
 * DAG Node - content-addressable node in the Merkle DAG
 */
export class DAGNode {
  /**
   * @param {Object} options - Node options
   * @param {string} options.type - Node type
   * @param {Object} options.data - Node data payload
   * @param {DAGLink[]} [options.links] - Links to other nodes
   * @param {Object} [options.metadata] - Additional metadata
   */
  constructor({ type, data, links = [], metadata = {} }) {
    this.type = type;
    this.data = data;
    this.links = links;
    this.metadata = {
      created: Date.now(),
      ...metadata,
    };

    // CID is computed lazily
    this._cid = null;
    this._encoded = null;
  }

  /**
   * Get the CID for this node
   * @returns {string} CID string
   */
  get cid() {
    if (!this._cid) {
      this._cid = createCID(this.encode());
    }
    return this._cid;
  }

  /**
   * Encode the node to CBOR bytes
   * @returns {Buffer} CBOR encoded bytes
   */
  encode() {
    if (!this._encoded) {
      const obj = {
        type: this.type,
        data: this.data,
        links: this.links.map(l => l.toJSON()),
        metadata: this.metadata,
      };
      this._encoded = Buffer.from(encode(obj));
    }
    return this._encoded;
  }

  /**
   * Get the size of the encoded node
   * @returns {number} Size in bytes
   */
  get size() {
    return this.encode().length;
  }

  /**
   * Convert to plain object
   * @returns {Object} Plain object representation
   */
  toJSON() {
    return {
      cid: this.cid,
      type: this.type,
      data: this.data,
      links: this.links.map(l => l.toJSON()),
      metadata: this.metadata,
    };
  }

  /**
   * Create a DAGLink pointing to this node
   * @param {string} [name] - Optional link name
   * @returns {DAGLink} Link to this node
   */
  toLink(name = '') {
    return new DAGLink(this.cid, name, this.size);
  }

  /**
   * Add a link to another node
   * @param {DAGLink|DAGNode} target - Target node or link
   * @param {string} [name] - Link name
   * @returns {DAGNode} This node (for chaining)
   */
  addLink(target, name = '') {
    // Invalidate cached values
    this._cid = null;
    this._encoded = null;

    if (target instanceof DAGLink) {
      this.links.push(target);
    } else if (target instanceof DAGNode) {
      this.links.push(target.toLink(name));
    } else if (typeof target === 'string') {
      this.links.push(new DAGLink(target, name));
    } else {
      throw new Error('Invalid link target');
    }

    return this;
  }

  /**
   * Get links by name
   * @param {string} name - Link name to find
   * @returns {DAGLink[]} Matching links
   */
  getLinks(name) {
    return this.links.filter(l => l.name === name);
  }

  /**
   * Check if node has a link with given CID
   * @param {string} cid - CID to check
   * @returns {boolean} True if link exists
   */
  hasLink(cid) {
    return this.links.some(l => l.cid === cid);
  }

  /**
   * Decode CBOR bytes into a DAGNode
   * @param {Buffer} bytes - CBOR encoded bytes
   * @returns {DAGNode} Decoded node
   */
  static decode(bytes) {
    const obj = decode(bytes);

    const node = new DAGNode({
      type: obj.type,
      data: obj.data,
      links: (obj.links || []).map(l => DAGLink.fromJSON(l)),
      metadata: obj.metadata || {},
    });

    // Store the original encoded form
    node._encoded = Buffer.from(bytes);
    return node;
  }

  /**
   * Create a node from raw CID and bytes (for loading from store)
   * @param {string} cid - Known CID
   * @param {Buffer} bytes - CBOR bytes
   * @returns {DAGNode} Node with verified CID
   */
  static fromBytes(cid, bytes) {
    const node = DAGNode.decode(bytes);

    // Verify CID matches
    if (node.cid !== cid) {
      throw new Error(`CID mismatch: expected ${cid}, got ${node.cid}`);
    }

    node._cid = cid;
    return node;
  }
}

/**
 * Create a judgment node
 * @param {Object} judgment - Judgment data
 * @returns {DAGNode} Judgment node
 */
export function createJudgmentNode(judgment) {
  return new DAGNode({
    type: NodeType.JUDGMENT,
    data: {
      id: judgment.id || `jdg_${Date.now().toString(36)}`,
      itemType: judgment.itemType,
      qScore: judgment.qScore,
      verdict: judgment.verdict,
      confidence: judgment.confidence,
      dimensions: judgment.dimensions,
      axiomScores: judgment.axiomScores,
    },
    metadata: {
      sessionId: judgment.sessionId,
      userId: judgment.userId,
    },
  });
}

/**
 * Create a block node (PoJ Chain)
 * @param {Object} block - Block data
 * @param {DAGLink[]} judgmentLinks - Links to judgment nodes
 * @param {DAGLink} [prevBlock] - Link to previous block
 * @returns {DAGNode} Block node
 */
export function createBlockNode(block, judgmentLinks = [], prevBlock = null) {
  const links = [...judgmentLinks];
  if (prevBlock) {
    links.unshift(new DAGLink(prevBlock.cid || prevBlock, 'prev', prevBlock.size || 0));
  }

  return new DAGNode({
    type: NodeType.BLOCK,
    data: {
      slot: block.slot,
      timestamp: block.timestamp || Date.now(),
      proposer: block.proposer,
      judgmentCount: judgmentLinks.length,
    },
    links,
    metadata: {
      finalized: block.finalized || false,
    },
  });
}

/**
 * Create an entity node (token, wallet, project, etc.)
 * @param {Object} entity - Entity data
 * @returns {DAGNode} Entity node
 */
export function createEntityNode(entity) {
  return new DAGNode({
    type: NodeType.ENTITY,
    data: {
      entityType: entity.entityType,
      identifier: entity.identifier,
      name: entity.name,
      attributes: entity.attributes || {},
    },
    metadata: {
      source: entity.source,
    },
  });
}

/**
 * Create an edge node (relationship between entities)
 * @param {Object} edge - Edge data
 * @param {DAGLink} source - Source entity link
 * @param {DAGLink} target - Target entity link
 * @returns {DAGNode} Edge node
 */
export function createEdgeNode(edge, source, target) {
  return new DAGNode({
    type: NodeType.EDGE,
    data: {
      edgeType: edge.edgeType,
      weight: edge.weight || 1.0,
      attributes: edge.attributes || {},
    },
    links: [
      new DAGLink(source.cid || source, 'source', source.size || 0),
      new DAGLink(target.cid || target, 'target', target.size || 0),
    ],
    metadata: {
      created: edge.created || Date.now(),
    },
  });
}

/**
 * Create a pattern node
 * @param {Object} pattern - Pattern data
 * @returns {DAGNode} Pattern node
 */
export function createPatternNode(pattern) {
  return new DAGNode({
    type: NodeType.PATTERN,
    data: {
      category: pattern.category,
      name: pattern.name,
      frequency: pattern.frequency,
      confidence: pattern.confidence,
      details: pattern.details || {},
    },
  });
}

/**
 * Create an index node (HAMT bucket)
 * @param {Object} index - Index data
 * @param {DAGLink[]} entries - Links to entries or child buckets
 * @returns {DAGNode} Index node
 */
export function createIndexNode(index, entries = []) {
  return new DAGNode({
    type: NodeType.INDEX,
    data: {
      depth: index.depth || 0,
      prefix: index.prefix || '',
      count: entries.length,
    },
    links: entries,
  });
}

/**
 * Create a root node (entry point)
 * @param {Object} root - Root data
 * @param {Object} links - Named links to indices and chain heads
 * @returns {DAGNode} Root node
 */
export function createRootNode(root, links = {}) {
  const dagLinks = Object.entries(links).map(
    ([name, link]) => new DAGLink(link.cid || link, name, link.size || 0)
  );

  return new DAGNode({
    type: NodeType.ROOT,
    data: {
      version: root.version || 1,
      created: root.created || Date.now(),
      updated: Date.now(),
    },
    links: dagLinks,
    metadata: {
      nodeId: root.nodeId,
    },
  });
}

export default {
  NodeType,
  DAGLink,
  DAGNode,
  createJudgmentNode,
  createBlockNode,
  createEntityNode,
  createEdgeNode,
  createPatternNode,
  createIndexNode,
  createRootNode,
};
