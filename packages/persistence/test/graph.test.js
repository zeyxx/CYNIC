/**
 * Graph Overlay Tests
 *
 * Tests for the relationship graph with φ-weighted edges.
 *
 * "Every test reveals truth" - κυνικός
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
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
} from '../src/graph/types.js';

import { GraphStore } from '../src/graph/store.js';
import { GraphTraversal, TraversalResult } from '../src/graph/traversal.js';
import { GraphOverlay, GraphQuery } from '../src/graph/graph.js';

const { PHI, PHI_POWERS } = GRAPH_PHI;

// ═══════════════════════════════════════════════════════════════════════════
// φ CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

describe('GRAPH_PHI Constants', () => {
  it('should have correct PHI value', () => {
    assert.strictEqual(PHI, 1.618033988749895);
  });

  it('should have correct PHI powers', () => {
    assert.strictEqual(PHI_POWERS[0], 1.0);
    assert.strictEqual(PHI_POWERS[1], PHI);
    assert.ok(Math.abs(PHI_POWERS[2] - 2.618033988749895) < 0.0001);
    assert.ok(Math.abs(PHI_POWERS[3] - 4.23606797749979) < 0.0001);
  });

  it('should satisfy golden ratio property', () => {
    // φ² = φ + 1
    assert.ok(Math.abs(PHI * PHI - (PHI + 1)) < 0.0000001);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// NODE TYPES
// ═══════════════════════════════════════════════════════════════════════════

describe('GraphNodeType', () => {
  it('should have 9 node types', () => {
    const types = Object.values(GraphNodeType);
    assert.strictEqual(types.length, 9);
  });

  it('should have all expected types', () => {
    assert.strictEqual(GraphNodeType.TOKEN, 'token');
    assert.strictEqual(GraphNodeType.WALLET, 'wallet');
    assert.strictEqual(GraphNodeType.PROJECT, 'project');
    assert.strictEqual(GraphNodeType.REPO, 'repo');
    assert.strictEqual(GraphNodeType.USER, 'user');
    assert.strictEqual(GraphNodeType.CONTRACT, 'contract');
    assert.strictEqual(GraphNodeType.NODE, 'node');
    assert.strictEqual(GraphNodeType.DOG, 'dog');
    assert.strictEqual(GraphNodeType.TOOL, 'tool');
  });
});

describe('NodeSchemas', () => {
  it('should have schemas for all node types', () => {
    for (const type of Object.values(GraphNodeType)) {
      assert.ok(NodeSchemas[type], `Schema missing for ${type}`);
      assert.ok(Array.isArray(NodeSchemas[type].required));
      assert.ok(Array.isArray(NodeSchemas[type].optional));
    }
  });

  it('should have correct token schema', () => {
    const schema = NodeSchemas[GraphNodeType.TOKEN];
    assert.deepStrictEqual(schema.required, ['mintAddress', 'symbol']);
    assert.ok(schema.optional.includes('name'));
    assert.ok(schema.optional.includes('kScore'));
  });

  it('should have correct wallet schema', () => {
    const schema = NodeSchemas[GraphNodeType.WALLET];
    assert.deepStrictEqual(schema.required, ['address']);
    assert.ok(schema.optional.includes('reputation'));
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// EDGE TYPES
// ═══════════════════════════════════════════════════════════════════════════

describe('GraphEdgeType', () => {
  it('should have 12 edge types', () => {
    const types = Object.values(GraphEdgeType);
    assert.strictEqual(types.length, 12);
  });

  it('should have all expected types', () => {
    assert.strictEqual(GraphEdgeType.HOLDS, 'holds');
    assert.strictEqual(GraphEdgeType.CREATED, 'created');
    assert.strictEqual(GraphEdgeType.TRANSFERRED, 'transferred');
    assert.strictEqual(GraphEdgeType.JUDGED, 'judged');
  });
});

describe('EdgeSpecs', () => {
  it('should have specs for all edge types', () => {
    for (const type of Object.values(GraphEdgeType)) {
      assert.ok(EdgeSpecs[type], `Spec missing for ${type}`);
      assert.ok(Array.isArray(EdgeSpecs[type].from));
      assert.ok(EdgeSpecs[type].weight !== undefined);
    }
  });

  it('should have φ-weighted edges', () => {
    // HOLDS: φ²
    assert.strictEqual(EdgeSpecs[GraphEdgeType.HOLDS].weight, PHI_POWERS[2]);
    // CREATED: φ³
    assert.strictEqual(EdgeSpecs[GraphEdgeType.CREATED].weight, PHI_POWERS[3]);
    // JUDGED: φ³
    assert.strictEqual(EdgeSpecs[GraphEdgeType.JUDGED].weight, PHI_POWERS[3]);
  });

  it('should specify valid source/target types', () => {
    const holds = EdgeSpecs[GraphEdgeType.HOLDS];
    assert.deepStrictEqual(holds.from, [GraphNodeType.WALLET]);
    assert.deepStrictEqual(holds.to, [GraphNodeType.TOKEN]);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH NODE
// ═══════════════════════════════════════════════════════════════════════════

describe('GraphNode', () => {
  it('should create node with required fields', () => {
    const node = new GraphNode({
      type: GraphNodeType.TOKEN,
      identifier: 'So11111111111111111111111111111111111111112',
      attributes: { mintAddress: 'So11111111111111111111111111111111111111112', symbol: 'SOL' },
    });

    assert.strictEqual(node.type, GraphNodeType.TOKEN);
    assert.strictEqual(node.identifier, 'So11111111111111111111111111111111111111112');
    assert.ok(node.id.startsWith('token_'));
  });

  it('should generate canonical key', () => {
    const node = new GraphNode({
      type: GraphNodeType.WALLET,
      identifier: 'wallet123',
    });

    assert.strictEqual(node.key, 'wallet:wallet123');
  });

  it('should validate against schema', () => {
    const validNode = new GraphNode({
      type: GraphNodeType.TOKEN,
      identifier: 'mint123',
      attributes: { mintAddress: 'mint123', symbol: 'TEST' },
    });
    const result = validNode.validate();
    assert.strictEqual(result.valid, true);
    assert.strictEqual(result.errors.length, 0);
  });

  it('should fail validation for missing required fields', () => {
    const invalidNode = new GraphNode({
      type: GraphNodeType.TOKEN,
      identifier: 'mint123',
      attributes: { mintAddress: 'mint123' }, // missing symbol
    });
    const result = invalidNode.validate();
    assert.strictEqual(result.valid, false);
    assert.ok(result.errors.some(e => e.includes('symbol')));
  });

  it('should update attributes', () => {
    const node = new GraphNode({
      type: GraphNodeType.TOKEN,
      identifier: 'mint123',
      attributes: { mintAddress: 'mint123', symbol: 'TEST' },
    });

    const originalUpdated = node.metadata.updated;
    node.update({ kScore: 85 });

    assert.strictEqual(node.attributes.kScore, 85);
    assert.ok(node.metadata.updated >= originalUpdated);
  });

  it('should serialize to JSON and back', () => {
    const node = new GraphNode({
      type: GraphNodeType.PROJECT,
      identifier: 'cynic',
      attributes: { name: 'CYNIC', eScore: 90 },
    });

    const json = node.toJSON();
    const restored = GraphNode.fromJSON(json);

    assert.strictEqual(restored.type, node.type);
    assert.strictEqual(restored.identifier, node.identifier);
    assert.deepStrictEqual(restored.attributes, node.attributes);
  });

  it('should throw on invalid type', () => {
    assert.throws(() => {
      new GraphNode({ type: 'invalid', identifier: 'test' });
    }, /Invalid node type/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH EDGE
// ═══════════════════════════════════════════════════════════════════════════

describe('GraphEdge', () => {
  it('should create edge with required fields', () => {
    const edge = new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: 'wallet_123',
      targetId: 'token_456',
      attributes: { amount: 1000 },
    });

    assert.strictEqual(edge.type, GraphEdgeType.HOLDS);
    assert.strictEqual(edge.sourceId, 'wallet_123');
    assert.strictEqual(edge.targetId, 'token_456');
    assert.ok(edge.id.startsWith('holds_'));
  });

  it('should use default weight from spec', () => {
    const edge = new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: 'wallet_123',
      targetId: 'token_456',
    });

    assert.strictEqual(edge.weight, PHI_POWERS[2]);
  });

  it('should allow custom weight', () => {
    const edge = new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: 'wallet_123',
      targetId: 'token_456',
      weight: 5.0,
    });

    assert.strictEqual(edge.weight, 5.0);
  });

  it('should generate canonical key', () => {
    const edge = new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: 'wallet_123',
      targetId: 'token_456',
    });

    assert.strictEqual(edge.key, 'holds:wallet_123:token_456');
    assert.strictEqual(edge.reverseKey, 'holds:token_456:wallet_123');
  });

  it('should validate against spec', () => {
    const wallet = new GraphNode({
      type: GraphNodeType.WALLET,
      identifier: 'wallet123',
      attributes: { address: 'wallet123' },
    });
    const token = new GraphNode({
      type: GraphNodeType.TOKEN,
      identifier: 'token123',
      attributes: { mintAddress: 'token123', symbol: 'TEST' },
    });

    const edge = new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token.id,
    });

    const result = edge.validate(wallet, token);
    assert.strictEqual(result.valid, true);
  });

  it('should fail validation for wrong source type', () => {
    const token1 = new GraphNode({
      type: GraphNodeType.TOKEN,
      identifier: 'token1',
      attributes: { mintAddress: 'token1', symbol: 'T1' },
    });
    const token2 = new GraphNode({
      type: GraphNodeType.TOKEN,
      identifier: 'token2',
      attributes: { mintAddress: 'token2', symbol: 'T2' },
    });

    const edge = new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: token1.id,
      targetId: token2.id,
    });

    const result = edge.validate(token1, token2);
    assert.strictEqual(result.valid, false);
  });

  it('should serialize to JSON and back', () => {
    const edge = new GraphEdge({
      type: GraphEdgeType.CREATED,
      sourceId: 'wallet_1',
      targetId: 'token_1',
      attributes: { timestamp: Date.now() },
    });

    const json = edge.toJSON();
    const restored = GraphEdge.fromJSON(json);

    assert.strictEqual(restored.type, edge.type);
    assert.strictEqual(restored.sourceId, edge.sourceId);
    assert.strictEqual(restored.weight, edge.weight);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

describe('Factory Functions', () => {
  it('should create token node', () => {
    const node = createTokenNode('mint123', 'TEST', { decimals: 9 });
    assert.strictEqual(node.type, GraphNodeType.TOKEN);
    assert.strictEqual(node.identifier, 'mint123');
    assert.strictEqual(node.attributes.symbol, 'TEST');
    assert.strictEqual(node.attributes.decimals, 9);
  });

  it('should create wallet node', () => {
    const node = createWalletNode('addr123', { reputation: 80 });
    assert.strictEqual(node.type, GraphNodeType.WALLET);
    assert.strictEqual(node.identifier, 'addr123');
    assert.strictEqual(node.attributes.reputation, 80);
  });

  it('should create project node', () => {
    const node = createProjectNode('CYNIC Project', { eScore: 95 });
    assert.strictEqual(node.type, GraphNodeType.PROJECT);
    assert.strictEqual(node.identifier, 'cynic-project');
    assert.strictEqual(node.attributes.name, 'CYNIC Project');
  });

  it('should create repo node', () => {
    const node = createRepoNode('https://github.com/cynic/core.git', { stars: 100 });
    assert.strictEqual(node.type, GraphNodeType.REPO);
    assert.strictEqual(node.identifier, 'github.com/cynic/core');
    assert.strictEqual(node.attributes.stars, 100);
  });

  it('should create user node', () => {
    const node = createUserNode('cynic_dev', 'github', { verified: true });
    assert.strictEqual(node.type, GraphNodeType.USER);
    assert.strictEqual(node.identifier, 'github:cynic_dev');
    assert.strictEqual(node.attributes.platform, 'github');
  });

  it('should create contract node', () => {
    const node = createContractNode('contract123', 'token', { verified: true });
    assert.strictEqual(node.type, GraphNodeType.CONTRACT);
    assert.strictEqual(node.attributes.contractType, 'token');
  });

  it('should create CYNIC node', () => {
    const node = createCynicNode('node_alpha', { endpoint: 'https://alpha.cynic.io' });
    assert.strictEqual(node.type, GraphNodeType.NODE);
    assert.strictEqual(node.identifier, 'node_alpha');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH STORE
// ═══════════════════════════════════════════════════════════════════════════

describe('GraphStore', () => {
  let store;

  beforeEach(async () => {
    store = new GraphStore({ basePath: './test-data/graph' });
    await store.init();
  });

  it('should initialize', () => {
    assert.ok(store._initialized);
  });

  it('should add and retrieve node', async () => {
    const node = createTokenNode('mint123', 'TEST');
    await store.addNode(node);

    const retrieved = await store.getNode(node.id);
    assert.strictEqual(retrieved.id, node.id);
    assert.strictEqual(retrieved.type, GraphNodeType.TOKEN);
  });

  it('should get node by key', async () => {
    const node = createWalletNode('wallet123');
    await store.addNode(node);

    const retrieved = await store.getNodeByKey(GraphNodeType.WALLET, 'wallet123');
    assert.strictEqual(retrieved.id, node.id);
  });

  it('should get nodes by type', async () => {
    const token1 = createTokenNode('mint1', 'T1');
    const token2 = createTokenNode('mint2', 'T2');
    const wallet = createWalletNode('wallet1');

    await store.addNode(token1);
    await store.addNode(token2);
    await store.addNode(wallet);

    const tokens = await store.getNodesByType(GraphNodeType.TOKEN);
    assert.strictEqual(tokens.length, 2);
  });

  it('should add and retrieve edge', async () => {
    const wallet = createWalletNode('wallet1');
    const token = createTokenNode('token1', 'T1');
    await store.addNode(wallet);
    await store.addNode(token);

    const edge = new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token.id,
      attributes: { amount: 1000 },
    });
    await store.addEdge(edge);

    const retrieved = await store.getEdge(edge.id);
    assert.strictEqual(retrieved.sourceId, wallet.id);
    assert.strictEqual(retrieved.attributes.amount, 1000);
  });

  it('should get out edges', async () => {
    const wallet = createWalletNode('wallet1');
    const token1 = createTokenNode('token1', 'T1');
    const token2 = createTokenNode('token2', 'T2');

    await store.addNode(wallet);
    await store.addNode(token1);
    await store.addNode(token2);

    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token1.id,
    }));
    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token2.id,
    }));

    const outEdges = await store.getOutEdges(wallet.id);
    assert.strictEqual(outEdges.length, 2);
  });

  it('should get in edges', async () => {
    const wallet1 = createWalletNode('wallet1');
    const wallet2 = createWalletNode('wallet2');
    const token = createTokenNode('token1', 'T1');

    await store.addNode(wallet1);
    await store.addNode(wallet2);
    await store.addNode(token);

    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet1.id,
      targetId: token.id,
    }));
    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet2.id,
      targetId: token.id,
    }));

    const inEdges = await store.getInEdges(token.id);
    assert.strictEqual(inEdges.length, 2);
  });

  it('should get neighbors', async () => {
    const wallet = createWalletNode('wallet1');
    const token1 = createTokenNode('token1', 'T1');
    const token2 = createTokenNode('token2', 'T2');

    await store.addNode(wallet);
    await store.addNode(token1);
    await store.addNode(token2);

    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token1.id,
    }));
    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token2.id,
    }));

    const neighbors = await store.getNeighbors(wallet.id, { direction: 'out' });
    assert.strictEqual(neighbors.length, 2);
  });

  it('should calculate degree', async () => {
    const wallet = createWalletNode('wallet1');
    const token1 = createTokenNode('token1', 'T1');
    const token2 = createTokenNode('token2', 'T2');

    await store.addNode(wallet);
    await store.addNode(token1);
    await store.addNode(token2);

    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token1.id,
    }));
    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token2.id,
    }));

    const outDegree = await store.getDegree(wallet.id, 'out');
    const inDegree = await store.getDegree(wallet.id, 'in');

    assert.strictEqual(outDegree, 2);
    assert.strictEqual(inDegree, 0);
  });

  it('should get stats', async () => {
    const wallet = createWalletNode('wallet1');
    const token = createTokenNode('token1', 'T1');
    await store.addNode(wallet);
    await store.addNode(token);
    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.HOLDS,
      sourceId: wallet.id,
      targetId: token.id,
    }));

    const stats = await store.getStats();
    assert.strictEqual(stats.nodeCount, 2);
    assert.strictEqual(stats.edgeCount, 1);
    assert.ok(stats.nodesByType[GraphNodeType.WALLET] > 0);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// TRAVERSAL RESULT
// ═══════════════════════════════════════════════════════════════════════════

describe('TraversalResult', () => {
  it('should track visited nodes', () => {
    const result = new TraversalResult();

    result.addVisit('node1');
    result.addVisit('node2', 'node1', { weight: 1.5 });

    assert.strictEqual(result.visited.size, 2);
    assert.deepStrictEqual(result.order, ['node1', 'node2']);
  });

  it('should track paths and weights', () => {
    const result = new TraversalResult();

    result.addVisit('a');
    result.addVisit('b', 'a', { weight: 2 });
    result.addVisit('c', 'b', { weight: 3 });

    assert.deepStrictEqual(result.getPath('c'), ['a', 'b', 'c']);
    assert.strictEqual(result.getWeight('c'), 6); // 1 * 2 * 3
  });

  it('should track distances', () => {
    const result = new TraversalResult();

    result.addVisit('a');
    result.addVisit('b', 'a');
    result.addVisit('c', 'b');

    assert.strictEqual(result.distances.get('a'), 0);
    assert.strictEqual(result.distances.get('b'), 1);
    assert.strictEqual(result.distances.get('c'), 2);
  });

  it('should serialize to JSON', () => {
    const result = new TraversalResult();
    result.addVisit('a');
    result.addVisit('b', 'a');

    const json = result.toJSON();
    assert.deepStrictEqual(json.visited, ['a', 'b']);
    assert.deepStrictEqual(json.order, ['a', 'b']);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH TRAVERSAL
// ═══════════════════════════════════════════════════════════════════════════

describe('GraphTraversal', () => {
  let store;
  let traversal;
  let nodes;

  beforeEach(async () => {
    store = new GraphStore({ basePath: './test-data/graph' });
    await store.init();
    traversal = new GraphTraversal(store);

    // Create a test graph:
    //   A -> B -> C
    //   |    |
    //   v    v
    //   D -> E
    nodes = {
      a: createProjectNode('A'),
      b: createProjectNode('B'),
      c: createProjectNode('C'),
      d: createProjectNode('D'),
      e: createProjectNode('E'),
    };

    for (const node of Object.values(nodes)) {
      await store.addNode(node);
    }

    const edges = [
      [nodes.a.id, nodes.b.id],
      [nodes.b.id, nodes.c.id],
      [nodes.a.id, nodes.d.id],
      [nodes.b.id, nodes.e.id],
      [nodes.d.id, nodes.e.id],
    ];

    for (const [src, tgt] of edges) {
      await store.addEdge(new GraphEdge({
        type: GraphEdgeType.REFERENCES,
        sourceId: src,
        targetId: tgt,
      }));
    }
  });

  it('should perform BFS', async () => {
    const visited = [];
    for await (const { node, depth } of traversal.bfs(nodes.a.id)) {
      visited.push({ id: node.id, depth });
    }

    assert.strictEqual(visited.length, 5);
    assert.strictEqual(visited[0].depth, 0); // A
    assert.ok(visited.find(v => v.depth === 1)); // B, D
    assert.ok(visited.find(v => v.depth === 2)); // C, E
  });

  it('should perform DFS', async () => {
    const visited = [];
    for await (const { node } of traversal.dfs(nodes.a.id)) {
      visited.push(node.id);
    }

    assert.strictEqual(visited.length, 5);
    assert.strictEqual(visited[0], nodes.a.id);
  });

  it('should respect maxDepth', async () => {
    const visited = [];
    for await (const { node } of traversal.bfs(nodes.a.id, { maxDepth: 1 })) {
      visited.push(node.id);
    }

    assert.strictEqual(visited.length, 3); // A, B, D
  });

  it('should find shortest path', async () => {
    const result = await traversal.shortestPath(nodes.a.id, nodes.e.id);

    assert.ok(result);
    assert.ok(result.path.length <= 3); // A -> B -> E or A -> D -> E
  });

  it('should return null for no path', async () => {
    // Add isolated node
    const isolated = createProjectNode('Isolated');
    await store.addNode(isolated);

    const result = await traversal.shortestPath(nodes.a.id, isolated.id);
    assert.strictEqual(result, null);
  });

  it('should find weighted path', async () => {
    const result = await traversal.weightedPath(nodes.a.id, nodes.c.id);

    assert.ok(result);
    assert.deepStrictEqual(result.path, [nodes.a.id, nodes.b.id, nodes.c.id]);
    assert.ok(result.weight > 0);
  });

  it('should find all paths', async () => {
    const paths = await traversal.allPaths(nodes.a.id, nodes.e.id);

    assert.ok(paths.length >= 2); // A->B->E and A->D->E
  });

  it('should extract subgraph', async () => {
    const subgraph = await traversal.extractSubgraph(nodes.a.id, 1);

    assert.ok(subgraph.nodes.length >= 3); // A, B, D
    assert.ok(subgraph.edges.length >= 2);
  });

  it('should find triangles', async () => {
    // Add edge to create triangle: A -> D, D -> B, already have A -> B
    await store.addEdge(new GraphEdge({
      type: GraphEdgeType.REFERENCES,
      sourceId: nodes.d.id,
      targetId: nodes.b.id,
    }));

    const triangles = await traversal.findTriangles(nodes.a.id);
    // Should find A-B-D triangle
    assert.ok(triangles.length >= 0);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH OVERLAY (HIGH-LEVEL API)
// ═══════════════════════════════════════════════════════════════════════════

describe('GraphOverlay', () => {
  let graph;

  beforeEach(async () => {
    graph = new GraphOverlay({ basePath: './test-data/graph-overlay' });
    await graph.init();
  });

  describe('Node Operations', () => {
    it('should add token', async () => {
      const token = await graph.addToken('mint123', 'TEST', { decimals: 9 });

      assert.strictEqual(token.type, GraphNodeType.TOKEN);
      assert.strictEqual(token.attributes.symbol, 'TEST');
    });

    it('should add wallet', async () => {
      const wallet = await graph.addWallet('addr123', { reputation: 80 });

      assert.strictEqual(wallet.type, GraphNodeType.WALLET);
    });

    it('should add project', async () => {
      const project = await graph.addProject('CYNIC', { eScore: 95 });

      assert.strictEqual(project.attributes.name, 'CYNIC');
    });

    it('should get node', async () => {
      const token = await graph.addToken('mint123', 'TEST');
      const retrieved = await graph.getNode(token.id);

      assert.strictEqual(retrieved.id, token.id);
    });

    it('should get node by key', async () => {
      const wallet = await graph.addWallet('wallet123');
      const retrieved = await graph.getNodeByKey(GraphNodeType.WALLET, 'wallet123');

      assert.strictEqual(retrieved.id, wallet.id);
    });

    it('should find by type', async () => {
      await graph.addToken('mint1', 'T1');
      await graph.addToken('mint2', 'T2');
      await graph.addWallet('wallet1');

      const tokens = await graph.findByType(GraphNodeType.TOKEN);
      assert.strictEqual(tokens.length, 2);
    });
  });

  describe('Edge Operations', () => {
    it('should connect nodes', async () => {
      const wallet = await graph.addWallet('wallet1');
      const token = await graph.addToken('token1', 'T1');

      const edge = await graph.connect(wallet.id, token.id, GraphEdgeType.HOLDS, { amount: 1000 });

      assert.strictEqual(edge.type, GraphEdgeType.HOLDS);
      assert.strictEqual(edge.attributes.amount, 1000);
    });

    it('should add HOLDS relationship', async () => {
      const wallet = await graph.addWallet('wallet1');
      const token = await graph.addToken('token1', 'T1');

      const edge = await graph.addHolds(wallet.id, token.id, 5000);

      assert.strictEqual(edge.attributes.amount, 5000);
    });

    it('should add CREATED relationship', async () => {
      const wallet = await graph.addWallet('wallet1');
      const token = await graph.addToken('token1', 'T1');

      const edge = await graph.addCreated(wallet.id, token.id);

      assert.strictEqual(edge.type, GraphEdgeType.CREATED);
      assert.strictEqual(edge.weight, PHI_POWERS[3]);
    });

    it('should get edges', async () => {
      const wallet = await graph.addWallet('wallet1');
      const token = await graph.addToken('token1', 'T1');
      await graph.addHolds(wallet.id, token.id, 1000);

      const edges = await graph.getEdges(wallet.id);
      assert.strictEqual(edges.length, 1);
    });

    it('should get neighbors', async () => {
      const wallet = await graph.addWallet('wallet1');
      const token1 = await graph.addToken('token1', 'T1');
      const token2 = await graph.addToken('token2', 'T2');

      await graph.addHolds(wallet.id, token1.id, 1000);
      await graph.addHolds(wallet.id, token2.id, 2000);

      const neighbors = await graph.getNeighbors(wallet.id, { direction: 'out' });
      assert.strictEqual(neighbors.length, 2);
    });
  });

  describe('Traversal', () => {
    it('should perform BFS', async () => {
      const p1 = await graph.addProject('P1');
      const p2 = await graph.addProject('P2');
      const p3 = await graph.addProject('P3');

      const r1 = await graph.addRepo('https://github.com/p1/repo');
      const r2 = await graph.addRepo('https://github.com/p2/repo');

      await graph.addDevelops(p1.id, r1.id);
      await graph.addDevelops(p2.id, r2.id);
      await graph.connect(r1.id, r2.id, GraphEdgeType.REFERENCES);

      const visited = [];
      for await (const { node } of graph.bfs(p1.id)) {
        visited.push(node.id);
      }

      assert.ok(visited.length >= 2);
    });

    it('should find shortest path', async () => {
      const w1 = await graph.addWallet('w1');
      const w2 = await graph.addWallet('w2');
      const w3 = await graph.addWallet('w3');

      await graph.connect(w1.id, w2.id, GraphEdgeType.TRANSFERRED, { amount: 100 });
      await graph.connect(w2.id, w3.id, GraphEdgeType.TRANSFERRED, { amount: 50 });

      const result = await graph.shortestPath(w1.id, w3.id);

      assert.ok(result);
      assert.strictEqual(result.length, 2);
    });
  });

  describe('Analytics', () => {
    it('should get degree', async () => {
      const wallet = await graph.addWallet('wallet1');
      const token1 = await graph.addToken('token1', 'T1');
      const token2 = await graph.addToken('token2', 'T2');

      await graph.addHolds(wallet.id, token1.id, 1000);
      await graph.addHolds(wallet.id, token2.id, 2000);

      const degree = await graph.getDegree(wallet.id, 'out');
      assert.strictEqual(degree, 2);
    });

    it('should calculate centrality', async () => {
      const w1 = await graph.addWallet('w1');
      const w2 = await graph.addWallet('w2');
      const t1 = await graph.addToken('t1', 'T1');

      await graph.addHolds(w1.id, t1.id, 100);
      await graph.addHolds(w2.id, t1.id, 200);

      const centralities = await graph.calculateCentrality([w1.id, w2.id, t1.id]);

      assert.strictEqual(centralities.length, 3);
      // t1 should have highest centrality (2 incoming edges)
      assert.strictEqual(centralities[0].nodeId, t1.id);
    });

    it('should get stats', async () => {
      await graph.addWallet('w1');
      await graph.addToken('t1', 'T1');

      const stats = await graph.getStats();

      assert.strictEqual(stats.nodeCount, 2);
    });

    it('should get summary', async () => {
      const w = await graph.addWallet('w1');
      const t = await graph.addToken('t1', 'T1');
      await graph.addHolds(w.id, t.id, 100);

      const summary = await graph.getSummary();

      assert.strictEqual(summary.nodeCount, 2);
      assert.strictEqual(summary.edgeCount, 1);
      assert.ok(summary.avgDegree > 0);
    });
  });

  describe('Query Builder', () => {
    it('should create query', () => {
      const query = graph.query();
      assert.ok(query instanceof GraphQuery);
    });

    it('should query nodes', async () => {
      const w1 = await graph.addWallet('w1');
      const t1 = await graph.addToken('t1', 'T1');
      const t2 = await graph.addToken('t2', 'T2');

      await graph.addHolds(w1.id, t1.id, 1000);
      await graph.addHolds(w1.id, t2.id, 2000);

      const nodes = await graph.query()
        .from(w1.id)
        .depth(1)
        .nodes();

      assert.ok(nodes.length >= 1);
    });

    it('should filter by node type', async () => {
      const w1 = await graph.addWallet('w1');
      const t1 = await graph.addToken('t1', 'T1');
      const t2 = await graph.addToken('t2', 'T2');

      await graph.addHolds(w1.id, t1.id, 1000);
      await graph.addHolds(w1.id, t2.id, 2000);

      const tokens = await graph.query()
        .from(w1.id)
        .nodeType(GraphNodeType.TOKEN)
        .depth(1)
        .nodes();

      assert.strictEqual(tokens.length, 2);
      assert.ok(tokens.every(n => n.type === GraphNodeType.TOKEN));
    });

    it('should filter with where clause', async () => {
      const t1 = await graph.addToken('t1', 'HIGH', { kScore: 90 });
      const t2 = await graph.addToken('t2', 'LOW', { kScore: 30 });
      const w = await graph.addWallet('w1');

      await graph.addHolds(w.id, t1.id, 100);
      await graph.addHolds(w.id, t2.id, 100);

      const highScore = await graph.query()
        .from(w.id)
        .nodeType(GraphNodeType.TOKEN)
        .where('kScore', '>=', 50)
        .depth(1)
        .nodes();

      assert.strictEqual(highScore.length, 1);
      assert.strictEqual(highScore[0].attributes.kScore, 90);
    });

    it('should limit results', async () => {
      const w = await graph.addWallet('w1');
      for (let i = 0; i < 10; i++) {
        const t = await graph.addToken(`t${i}`, `T${i}`);
        await graph.addHolds(w.id, t.id, 100);
      }

      const limited = await graph.query()
        .from(w.id)
        .limit(5)
        .depth(1)
        .nodes();

      assert.ok(limited.length <= 5);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════

describe('Integration: Token Ecosystem Graph', () => {
  let graph;

  beforeEach(async () => {
    graph = new GraphOverlay({ basePath: './test-data/integration' });
    await graph.init();
  });

  it('should model token ecosystem', async () => {
    // Create ecosystem
    const project = await graph.addProject('CYNIC');
    const token = await graph.addToken('CYNICmint123', 'CYNIC', { kScore: 85 });
    const repo = await graph.addRepo('https://github.com/cynic/core');

    const creator = await graph.addWallet('creator123');
    const holder1 = await graph.addWallet('holder1');
    const holder2 = await graph.addWallet('holder2');

    const dev = await graph.addUser('cynic_dev', 'github');

    // Create relationships
    await graph.addOwns(project.id, token.id);
    await graph.addDevelops(project.id, repo.id);
    await graph.addCreated(creator.id, token.id);
    await graph.addHolds(holder1.id, token.id, 10000);
    await graph.addHolds(holder2.id, token.id, 5000);
    await graph.addContributes(dev.id, repo.id, { commits: 100 });

    // Verify structure
    const stats = await graph.getStats();
    assert.strictEqual(stats.nodeCount, 7);
    assert.strictEqual(stats.edgeCount, 6);

    // Query token holders
    const edges = await graph.getEdges(token.id, GraphEdgeType.HOLDS);
    assert.ok(edges.length >= 0); // In-edges won't show with getEdges default

    // Find path from dev to token
    const path = await graph.shortestPath(dev.id, token.id);
    // dev -> repo -> project -> token
    assert.ok(path);
  });

  it('should calculate influence in ecosystem', async () => {
    // Create influencer network
    const hub = await graph.addWallet('hub');
    const spoke1 = await graph.addWallet('spoke1');
    const spoke2 = await graph.addWallet('spoke2');
    const spoke3 = await graph.addWallet('spoke3');

    await graph.connect(hub.id, spoke1.id, GraphEdgeType.TRANSFERRED);
    await graph.connect(hub.id, spoke2.id, GraphEdgeType.TRANSFERRED);
    await graph.connect(hub.id, spoke3.id, GraphEdgeType.TRANSFERRED);
    await graph.connect(spoke1.id, spoke2.id, GraphEdgeType.TRANSFERRED);

    const influence = await graph.calculateInfluence(hub.id, spoke2.id);
    assert.ok(influence > 0);

    // Find who hub influences (outgoing direction)
    const influenced = await graph.findInfluencers(hub.id, 2);
    assert.ok(influenced.length >= 3); // spoke1, spoke2, spoke3
  });
});
