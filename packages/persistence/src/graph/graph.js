/**
 * Graph Overlay
 *
 * High-level API for the relationship graph with queries and patterns.
 *
 * "Relationships define truth" - κυνικός
 *
 * @module @cynic/persistence/graph/graph
 */

'use strict';

import { EventEmitter } from 'events';
import { GraphStore } from './store.js';
import { GraphTraversal } from './traversal.js';
import {
  GraphNode,
  GraphEdge,
  GraphNodeType,
  GraphEdgeType,
  EdgeSpecs,
  createTokenNode,
  createWalletNode,
  createProjectNode,
  createRepoNode,
  createUserNode,
  createContractNode,
  createCynicNode,
  createDogNode,
  createToolNode,
  GRAPH_PHI,
} from './types.js';

const { PHI } = GRAPH_PHI;

/**
 * Query builder for graph queries
 */
class GraphQuery {
  constructor(graph) {
    this.graph = graph;
    this._nodeFilters = [];
    this._edgeFilters = [];
    this._startNodes = [];
    this._traversalOptions = {};
    this._limit = 100;
    this._sortBy = null;
  }

  /**
   * Start from specific nodes
   */
  from(...nodeIds) {
    this._startNodes.push(...nodeIds);
    return this;
  }

  /**
   * Filter by node type
   */
  nodeType(type) {
    this._nodeFilters.push(node => node.type === type);
    return this;
  }

  /**
   * Filter by edge type
   */
  edgeType(type) {
    this._edgeFilters.push(edge => edge.type === type);
    this._traversalOptions.edgeType = type;
    return this;
  }

  /**
   * Filter by node attribute
   */
  where(field, operator, value) {
    this._nodeFilters.push(node => {
      const nodeValue = node.attributes[field] ?? node[field];
      switch (operator) {
        case '=':
        case '==':
          return nodeValue === value;
        case '!=':
          return nodeValue !== value;
        case '>':
          return nodeValue > value;
        case '>=':
          return nodeValue >= value;
        case '<':
          return nodeValue < value;
        case '<=':
          return nodeValue <= value;
        case 'contains':
          return String(nodeValue).includes(value);
        case 'startsWith':
          return String(nodeValue).startsWith(value);
        case 'in':
          return Array.isArray(value) && value.includes(nodeValue);
        default:
          return true;
      }
    });
    return this;
  }

  /**
   * Set traversal depth
   */
  depth(maxDepth) {
    this._traversalOptions.maxDepth = maxDepth;
    return this;
  }

  /**
   * Set traversal direction
   */
  direction(dir) {
    this._traversalOptions.direction = dir;
    return this;
  }

  /**
   * Limit results
   */
  limit(n) {
    this._limit = n;
    return this;
  }

  /**
   * Sort results
   */
  sortBy(field, order = 'asc') {
    this._sortBy = { field, order };
    return this;
  }

  /**
   * Execute query and return nodes
   */
  async nodes() {
    const results = [];

    for (const startId of this._startNodes) {
      for await (const { node } of this.graph.traversal.bfs(startId, this._traversalOptions)) {
        // Apply filters
        let passes = true;
        for (const filter of this._nodeFilters) {
          if (!filter(node)) {
            passes = false;
            break;
          }
        }

        if (passes) {
          results.push(node);
          if (results.length >= this._limit) break;
        }
      }
      if (results.length >= this._limit) break;
    }

    // Sort if requested
    if (this._sortBy) {
      const { field, order } = this._sortBy;
      results.sort((a, b) => {
        const aVal = a.attributes[field] ?? a[field] ?? 0;
        const bVal = b.attributes[field] ?? b[field] ?? 0;
        return order === 'asc' ? aVal - bVal : bVal - aVal;
      });
    }

    return results;
  }

  /**
   * Execute query and return edges
   */
  async edges() {
    const results = [];
    const seenEdges = new Set();

    for (const startId of this._startNodes) {
      for await (const { node } of this.graph.traversal.bfs(startId, this._traversalOptions)) {
        const edges = await this.graph.store.getEdges(node.id, this._traversalOptions.edgeType);

        for (const edge of edges) {
          if (seenEdges.has(edge.id)) continue;
          seenEdges.add(edge.id);

          // Apply filters
          let passes = true;
          for (const filter of this._edgeFilters) {
            if (!filter(edge)) {
              passes = false;
              break;
            }
          }

          if (passes) {
            results.push(edge);
            if (results.length >= this._limit) break;
          }
        }
        if (results.length >= this._limit) break;
      }
      if (results.length >= this._limit) break;
    }

    return results;
  }

  /**
   * Count matching nodes
   */
  async count() {
    const nodes = await this.nodes();
    return nodes.length;
  }
}

/**
 * Graph Overlay - relationship graph for the ecosystem
 */
export class GraphOverlay extends EventEmitter {
  /**
   * @param {Object} config - Configuration
   */
  constructor(config = {}) {
    super();

    this.config = {
      basePath: config.basePath || './data/graph',
      ...config,
    };

    this.store = new GraphStore(this.config);
    this.traversal = new GraphTraversal(this.store);

    this._initialized = false;
  }

  /**
   * Initialize the graph
   */
  async init() {
    if (this._initialized) return;

    await this.store.init();
    this._initialized = true;
    this.emit('initialized');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // NODE OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Add a node
   */
  async addNode(node) {
    await this.init();
    return this.store.addNode(node);
  }

  /**
   * Get a node
   */
  async getNode(nodeId) {
    return this.store.getNode(nodeId);
  }

  /**
   * Get node by type and identifier
   */
  async getNodeByKey(type, identifier) {
    return this.store.getNodeByKey(type, identifier);
  }

  /**
   * Add a token
   */
  async addToken(mintAddress, symbol, attributes = {}) {
    const node = createTokenNode(mintAddress, symbol, attributes);
    await this.addNode(node);
    return node;
  }

  /**
   * Add a wallet
   */
  async addWallet(address, attributes = {}) {
    const node = createWalletNode(address, attributes);
    await this.addNode(node);
    return node;
  }

  /**
   * Add a project
   */
  async addProject(name, attributes = {}) {
    const node = createProjectNode(name, attributes);
    await this.addNode(node);
    return node;
  }

  /**
   * Add a repo
   */
  async addRepo(url, attributes = {}) {
    const node = createRepoNode(url, attributes);
    await this.addNode(node);
    return node;
  }

  /**
   * Add a user
   */
  async addUser(handle, platform, attributes = {}) {
    const node = createUserNode(handle, platform, attributes);
    await this.addNode(node);
    return node;
  }

  /**
   * Add a CYNIC dog (agent)
   */
  async addDog(name, attributes = {}) {
    const node = createDogNode(name, attributes);
    await this.addNode(node);
    return node;
  }

  /**
   * Add a tool
   */
  async addTool(name, attributes = {}) {
    const node = createToolNode(name, attributes);
    await this.addNode(node);
    return node;
  }

  /**
   * Ensure a node exists (get or create)
   * @param {string} type - Node type (dog, tool, etc.)
   * @param {string} identifier - Node identifier
   * @param {Object} attributes - Attributes for creation
   * @returns {Promise<GraphNode>} The existing or newly created node
   */
  async ensureNode(type, identifier, attributes = {}) {
    // Try to get existing node
    const existing = await this.getNodeByKey(type, identifier);
    if (existing) return existing;

    // Create new node based on type
    switch (type) {
      case GraphNodeType.DOG:
        return this.addDog(identifier, attributes);
      case GraphNodeType.TOOL:
        return this.addTool(identifier, attributes);
      case GraphNodeType.TOKEN:
        return this.addToken(identifier, attributes.symbol || identifier, attributes);
      case GraphNodeType.WALLET:
        return this.addWallet(identifier, attributes);
      case GraphNodeType.PROJECT:
        return this.addProject(identifier, attributes);
      case GraphNodeType.REPO:
        return this.addRepo(identifier, attributes);
      case GraphNodeType.USER:
        return this.addUser(identifier, attributes.platform || 'unknown', attributes);
      default:
        // Generic node creation
        const node = new GraphNode({ type, identifier, attributes });
        await this.addNode(node);
        return node;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EDGE OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Add an edge
   */
  async addEdge(edge) {
    await this.init();
    return this.store.addEdge(edge);
  }

  /**
   * Create and add an edge
   */
  async connect(sourceId, targetId, type, attributes = {}) {
    const edge = new GraphEdge({
      type,
      sourceId,
      targetId,
      attributes,
    });
    await this.addEdge(edge);
    return edge;
  }

  /**
   * Add HOLDS relationship
   */
  async addHolds(walletId, tokenId, amount, attributes = {}) {
    return this.connect(walletId, tokenId, GraphEdgeType.HOLDS, { amount, ...attributes });
  }

  /**
   * Add CREATED relationship
   */
  async addCreated(walletId, tokenId, attributes = {}) {
    return this.connect(walletId, tokenId, GraphEdgeType.CREATED, attributes);
  }

  /**
   * Add OWNS relationship
   */
  async addOwns(projectId, tokenId, attributes = {}) {
    return this.connect(projectId, tokenId, GraphEdgeType.OWNS, attributes);
  }

  /**
   * Add DEVELOPS relationship
   */
  async addDevelops(projectId, repoId, attributes = {}) {
    return this.connect(projectId, repoId, GraphEdgeType.DEVELOPS, attributes);
  }

  /**
   * Add CONTRIBUTES relationship
   */
  async addContributes(userId, repoId, attributes = {}) {
    return this.connect(userId, repoId, GraphEdgeType.CONTRIBUTES, attributes);
  }

  /**
   * Add JUDGED relationship (from CYNIC node)
   */
  async addJudged(cynicNodeId, entityId, judgmentData) {
    return this.connect(cynicNodeId, entityId, GraphEdgeType.JUDGED, judgmentData);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // QUERY
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create a query builder
   */
  query() {
    return new GraphQuery(this);
  }

  /**
   * Find nodes by type
   */
  async findByType(type) {
    return this.store.getNodesByType(type);
  }

  /**
   * Get neighbors
   */
  async getNeighbors(nodeId, options = {}) {
    return this.store.getNeighbors(nodeId, options);
  }

  /**
   * Get edges for a node
   */
  async getEdges(nodeId, edgeType = null) {
    return this.store.getEdges(nodeId, edgeType);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PATH FINDING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Find shortest path
   */
  async shortestPath(startId, endId, options = {}) {
    return this.traversal.shortestPath(startId, endId, options);
  }

  /**
   * Find weighted path
   */
  async weightedPath(startId, endId, options = {}) {
    return this.traversal.weightedPath(startId, endId, options);
  }

  /**
   * Find all paths
   */
  async allPaths(startId, endId, options = {}) {
    return this.traversal.allPaths(startId, endId, options);
  }

  /**
   * Calculate influence
   */
  async calculateInfluence(sourceId, targetId, options = {}) {
    return this.traversal.calculateInfluence(sourceId, targetId, options);
  }

  /**
   * Find influencers
   */
  async findInfluencers(nodeId, hops = 3) {
    return this.traversal.findInfluencers(nodeId, hops);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SUBGRAPH & PATTERNS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Extract subgraph
   */
  async extractSubgraph(centerId, radius = 2) {
    return this.traversal.extractSubgraph(centerId, radius);
  }

  /**
   * Find connected components
   */
  async findConnectedComponents() {
    return this.traversal.findConnectedComponents();
  }

  /**
   * Find cycles
   */
  async findCycles(nodeId, options = {}) {
    return this.traversal.findCycles(nodeId, options);
  }

  /**
   * Find triangles
   */
  async findTriangles(nodeId) {
    return this.traversal.findTriangles(nodeId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRAVERSAL
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * BFS traversal
   */
  async *bfs(startId, options = {}) {
    yield* this.traversal.bfs(startId, options);
  }

  /**
   * DFS traversal
   */
  async *dfs(startId, options = {}) {
    yield* this.traversal.dfs(startId, options);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // ANALYTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get node degree
   */
  async getDegree(nodeId, direction = 'both') {
    return this.store.getDegree(nodeId, direction);
  }

  /**
   * Calculate degree centrality for multiple nodes
   */
  async calculateCentrality(nodeIds) {
    const centralities = [];

    for (const nodeId of nodeIds) {
      const inDegree = await this.store.getDegree(nodeId, 'in');
      const outDegree = await this.store.getDegree(nodeId, 'out');

      centralities.push({
        nodeId,
        inDegree,
        outDegree,
        totalDegree: inDegree + outDegree,
        centrality: (inDegree + outDegree) / (nodeIds.length - 1), // Normalized
      });
    }

    return centralities.sort((a, b) => b.centrality - a.centrality);
  }

  /**
   * Calculate φ-weighted page rank (simplified)
   * @param {number} [iterations=20] - Number of iterations
   * @param {number} [dampingFactor=0.618] - Damping factor (φ⁻¹)
   */
  async pageRank(iterations = 20, dampingFactor = 1 / PHI) {
    const nodeIds = [...this.store._nodeCache.keys()];
    const n = nodeIds.length;
    if (n === 0) return new Map();

    // Initialize ranks
    const ranks = new Map();
    const initialRank = 1 / n;
    for (const id of nodeIds) {
      ranks.set(id, initialRank);
    }

    // Iterate
    for (let i = 0; i < iterations; i++) {
      const newRanks = new Map();

      for (const nodeId of nodeIds) {
        let sum = 0;

        // Get incoming edges
        const inEdges = await this.store.getInEdges(nodeId);
        for (const edge of inEdges) {
          const sourceRank = ranks.get(edge.sourceId) || 0;
          const sourceDegree = await this.store.getDegree(edge.sourceId, 'out');
          if (sourceDegree > 0) {
            // Weight by edge weight (φ-weighted)
            sum += (sourceRank / sourceDegree) * edge.weight;
          }
        }

        newRanks.set(nodeId, (1 - dampingFactor) / n + dampingFactor * sum);
      }

      // Update ranks
      for (const [id, rank] of newRanks) {
        ranks.set(id, rank);
      }
    }

    return ranks;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get graph statistics
   */
  async getStats() {
    return this.store.getStats();
  }

  /**
   * Get summary
   */
  async getSummary() {
    const stats = await this.getStats();
    const components = await this.findConnectedComponents();

    return {
      ...stats,
      connectedComponents: components.length,
      largestComponent: Math.max(...components.map(c => c.size), 0),
      avgDegree: stats.nodeCount > 0 ? (stats.edgeCount * 2) / stats.nodeCount : 0,
    };
  }
}

// Re-export types and classes
export {
  GraphQuery,
  GraphNode,
  GraphEdge,
  GraphNodeType,
  GraphEdgeType,
  EdgeSpecs,
  GraphStore,
  GraphTraversal,
  GRAPH_PHI,
};

export default GraphOverlay;
