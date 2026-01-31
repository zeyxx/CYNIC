/**
 * Claude Flow MCP Tools Tests
 *
 * Tests for routing, optimization, and embedding tools
 *
 * @module @cynic/mcp/test/tools-claude-flow
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';
import {
  createComplexityTool,
  createBoosterTool,
  createOptimizerTool,
  createRouterTool,
  createHyperbolicTool,
  createSONATool,
} from '../src/tools/domains/claude-flow.js';

// ═══════════════════════════════════════════════════════════════════════════
// MOCK SERVICES
// ═══════════════════════════════════════════════════════════════════════════

function createMockClassifier() {
  return {
    classify: ({ content }) => {
      const isSimple = content.length < 50 && !content.includes('analyze');
      const isComplex = content.includes('architect') || content.includes('refactor');
      return {
        tier: isComplex ? 'FULL' : isSimple ? 'LOCAL' : 'LIGHT',
        complexity: isComplex ? 0.8 : isSimple ? 0.1 : 0.4,
        confidence: 0.75,
        signals: { length: content.length },
      };
    },
  };
}

function createMockBooster() {
  return {
    canHandle: (text) => {
      if (text.includes('var to const')) {
        return { intent: 'var-to-const', confidence: 0.9 };
      }
      return null;
    },
    transform: ({ code, intent }) => ({
      code: intent === 'var-to-const' ? code.replace(/\bvar\b/g, 'const') : code,
      intent,
      changes: 1,
      elapsed: 0.5,
    }),
    getAvailableIntents: () => ['var-to-const', 'add-types', 'remove-console'],
  };
}

function createMockOptimizer() {
  return {
    optimize: ({ content, strategies }) => {
      const optimized = content
        .replace(/\s+/g, ' ')
        .replace(/please|kindly/gi, '')
        .trim();
      return {
        optimized,
        originalTokens: content.split(/\s+/).length,
        optimizedTokens: optimized.split(/\s+/).length,
        savedTokens: content.length - optimized.length,
        compressionRatio: optimized.length / content.length,
        appliedStrategies: strategies || ['whitespace'],
      };
    },
    getFromCache: () => null,
    addToCache: () => {},
  };
}

function createMockRouter() {
  return {
    route: async ({ content, options }) => ({
      routing: {
        tier: content.length < 50 ? 'LOCAL' : 'LIGHT',
        confidence: 0.7,
        cost: content.length < 50 ? 0 : 1,
        latency: content.length < 50 ? 1 : 500,
      },
    }),
  };
}

function createMockHyperbolicSpace() {
  const nodes = new Map();
  nodes.set('root', { parent: null });
  nodes.set('child1', { parent: 'root' });
  nodes.set('child2', { parent: 'root' });

  return {
    nodeCount: 3,
    dim: 8,
    curvature: -1,
    distance: (id1, id2) => {
      if (id1 === id2) return 0;
      return 1.5;
    },
    kNearest: (id, k) => ['root', 'child1', 'child2'].filter(n => n !== id).slice(0, k),
    getAncestors: (id) => {
      const ancestors = [];
      let current = nodes.get(id);
      while (current?.parent) {
        ancestors.push(current.parent);
        current = nodes.get(current.parent);
      }
      return ancestors;
    },
    getDescendants: (id) => {
      return Array.from(nodes.entries())
        .filter(([key, val]) => val.parent === id)
        .map(([key]) => key);
    },
    centroid: (ids) => ({ x: 0, y: 0 }),
    add: (id, vector, parent) => {
      nodes.set(id, { parent: parent || null });
    },
  };
}

function createMockSONA() {
  const patterns = new Map();
  const correlations = {};

  return {
    getTrackedCount: () => patterns.size,
    getCorrelationCount: () => Object.keys(correlations).length,
    getCorrelations: () => correlations,
    getSuggestions: () => [{ dimension: 'phi', adjustment: 0.1 }],
    observe: ({ patternId, dimensionScores }) => {
      patterns.set(patternId, dimensionScores);
    },
    processFeedback: ({ patternId, success, impact }) => {
      if (patterns.has(patternId)) {
        correlations[patternId] = { success, impact };
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPLEXITY TOOL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('brain_complexity Tool', () => {
  let tool;
  let classifier;

  beforeEach(() => {
    classifier = createMockClassifier();
    tool = createComplexityTool(classifier);
  });

  it('should have correct name', () => {
    assert.strictEqual(tool.name, 'brain_complexity');
  });

  it('should classify simple requests as LOCAL', async () => {
    const result = await tool.handler({ content: 'list files' });
    assert.strictEqual(result.tier, 'LOCAL');
    assert.ok(result.complexity < 0.3);
  });

  it('should classify complex requests as FULL', async () => {
    const result = await tool.handler({ content: 'architect the entire system' });
    assert.strictEqual(result.tier, 'FULL');
    assert.ok(result.complexity > 0.5);
  });

  it('should cap confidence at φ⁻¹', async () => {
    const result = await tool.handler({ content: 'test' });
    // Mock returns 0.75, tool should cap at 0.618
    // Note: Using 0.619 to account for floating point precision
    assert.ok(result.confidence <= 0.619, `Expected <= 0.618 but got ${result.confidence}`);
  });

  it('should handle missing classifier', async () => {
    const noClassifierTool = createComplexityTool(null);
    const result = await noClassifierTool.handler({ content: 'test' });
    assert.ok(result.error);
    assert.strictEqual(result.tier, 'FULL'); // Fallback
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// BOOSTER TOOL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('brain_boost Tool', () => {
  let tool;
  let booster;

  beforeEach(() => {
    booster = createMockBooster();
    tool = createBoosterTool(booster);
  });

  it('should have correct name', () => {
    assert.strictEqual(tool.name, 'brain_boost');
  });

  it('should transform code with explicit intent', async () => {
    const result = await tool.handler({
      code: 'var x = 5;',
      intent: 'var-to-const',
    });
    assert.strictEqual(result.code, 'const x = 5;');
    assert.strictEqual(result.cost, 0);
  });

  it('should auto-detect intent from natural language', async () => {
    const result = await tool.handler({
      code: 'var x = 5;',
      detect: 'convert var to const',
    });
    assert.strictEqual(result.code, 'const x = 5;');
  });

  it('should return error for unknown intent detection', async () => {
    const result = await tool.handler({
      code: 'var x = 5;',
      detect: 'do something unknown',
    });
    assert.ok(result.error);
    assert.ok(result.availableIntents);
  });

  it('should require either intent or detect', async () => {
    const result = await tool.handler({
      code: 'var x = 5;',
    });
    assert.ok(result.error);
  });

  it('should handle missing booster', async () => {
    const noBoosterTool = createBoosterTool(null);
    const result = await noBoosterTool.handler({
      code: 'var x = 5;',
      intent: 'var-to-const',
    });
    assert.ok(result.error);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// OPTIMIZER TOOL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('brain_optimize Tool', () => {
  let tool;
  let optimizer;

  beforeEach(() => {
    optimizer = createMockOptimizer();
    tool = createOptimizerTool(optimizer);
  });

  it('should have correct name', () => {
    assert.strictEqual(tool.name, 'brain_optimize');
  });

  it('should compress content', async () => {
    const result = await tool.handler({
      content: 'please   kindly   help   me',
    });
    assert.ok(result.optimized.length < 'please   kindly   help   me'.length);
    assert.ok(result.savedTokens > 0);
  });

  it('should report compression ratio', async () => {
    const result = await tool.handler({
      content: 'hello    world',
    });
    assert.ok(result.compressionRatio);
    assert.ok(result.compressionRatio <= 1);
  });

  it('should handle missing optimizer', async () => {
    const noOptimizerTool = createOptimizerTool(null);
    const result = await noOptimizerTool.handler({
      content: 'test',
    });
    assert.ok(result.error);
    assert.strictEqual(result.optimized, 'test');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// ROUTER TOOL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('brain_route Tool', () => {
  let tool;
  let router;

  beforeEach(() => {
    router = createMockRouter();
    tool = createRouterTool(router);
  });

  it('should have correct name', () => {
    assert.strictEqual(tool.name, 'brain_route');
  });

  it('should route simple requests to LOCAL', async () => {
    const result = await tool.handler({ content: 'hi' });
    assert.strictEqual(result.routing.tier, 'LOCAL');
    assert.strictEqual(result.routing.cost, 0);
  });

  it('should route complex requests to LIGHT or FULL', async () => {
    const result = await tool.handler({
      content: 'This is a much longer request that requires more processing',
    });
    assert.ok(['LIGHT', 'FULL'].includes(result.routing.tier));
  });

  it('should cap confidence at φ⁻¹', async () => {
    const result = await tool.handler({ content: 'test' });
    // Mock returns 0.7, tool should cap at 0.618
    // Note: Using 0.619 to account for floating point precision
    assert.ok(result.routing.confidence <= 0.619, `Expected <= 0.618 but got ${result.routing.confidence}`);
  });

  it('should handle missing router', async () => {
    const noRouterTool = createRouterTool(null);
    const result = await noRouterTool.handler({ content: 'test' });
    assert.ok(result.error);
    assert.strictEqual(result.routing.tier, 'FULL');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// HYPERBOLIC TOOL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('brain_hyperbolic Tool', () => {
  let tool;
  let space;

  beforeEach(() => {
    space = createMockHyperbolicSpace();
    tool = createHyperbolicTool(space);
  });

  it('should have correct name', () => {
    assert.strictEqual(tool.name, 'brain_hyperbolic');
  });

  it('should compute distance', async () => {
    const result = await tool.handler({
      operation: 'distance',
      id: 'root',
      id2: 'child1',
    });
    assert.ok(result.distance >= 0);
  });

  it('should find nearest neighbors', async () => {
    const result = await tool.handler({
      operation: 'nearest',
      id: 'root',
      k: 2,
    });
    assert.ok(Array.isArray(result.neighbors));
    assert.ok(result.neighbors.length <= 2);
  });

  it('should get ancestors', async () => {
    const result = await tool.handler({
      operation: 'ancestors',
      id: 'child1',
    });
    assert.ok(Array.isArray(result.ancestors));
    assert.ok(result.ancestors.includes('root'));
  });

  it('should add nodes', async () => {
    const result = await tool.handler({
      operation: 'add',
      id: 'new-node',
      parentId: 'root',
    });
    assert.strictEqual(result.added, 'new-node');
    assert.strictEqual(result.parent, 'root');
  });

  it('should return stats', async () => {
    const result = await tool.handler({
      operation: 'stats',
    });
    assert.strictEqual(result.nodeCount, 3);
    assert.strictEqual(result.dim, 8);
    assert.strictEqual(result.curvature, -1);
  });

  it('should handle missing space', async () => {
    const noSpaceTool = createHyperbolicTool(null);
    const result = await noSpaceTool.handler({ operation: 'stats' });
    assert.ok(result.error);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// SONA TOOL TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('brain_sona Tool', () => {
  let tool;
  let sona;

  beforeEach(() => {
    sona = createMockSONA();
    tool = createSONATool(sona);
  });

  it('should have correct name', () => {
    assert.strictEqual(tool.name, 'brain_sona');
  });

  it('should return status', async () => {
    const result = await tool.handler({ action: 'status' });
    assert.strictEqual(result.status, 'active');
    assert.ok(result.adaptationRate > 0);
  });

  it('should observe patterns', async () => {
    const result = await tool.handler({
      action: 'observe',
      patternId: 'p-123',
      dimensionScores: { phi: 0.8, verify: 0.7 },
    });
    assert.strictEqual(result.observed, 'p-123');
  });

  it('should process feedback', async () => {
    // First observe
    await tool.handler({
      action: 'observe',
      patternId: 'p-456',
      dimensionScores: { phi: 0.6 },
    });

    // Then feedback
    const result = await tool.handler({
      action: 'feedback',
      patternId: 'p-456',
      success: true,
      impact: 0.9,
    });
    assert.strictEqual(result.processed, 'p-456');
    assert.strictEqual(result.success, true);
  });

  it('should return correlations', async () => {
    const result = await tool.handler({ action: 'correlations' });
    assert.ok(result.correlations !== undefined);
  });

  it('should return suggestions', async () => {
    const result = await tool.handler({ action: 'suggestions' });
    assert.ok(Array.isArray(result.suggestions));
  });

  it('should handle missing sona', async () => {
    const noSonaTool = createSONATool(null);
    const result = await noSonaTool.handler({ action: 'status' });
    assert.ok(result.error);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════

describe('Claude Flow Tools Integration', () => {
  it('should create all tools with mock services', async () => {
    const tools = [
      createComplexityTool(createMockClassifier()),
      createBoosterTool(createMockBooster()),
      createOptimizerTool(createMockOptimizer()),
      createRouterTool(createMockRouter()),
      createHyperbolicTool(createMockHyperbolicSpace()),
      createSONATool(createMockSONA()),
    ];

    assert.strictEqual(tools.length, 6);
    assert.ok(tools.every(t => t.name && t.handler));
  });

  it('should have brain_ prefix for all tools', async () => {
    const tools = [
      createComplexityTool(createMockClassifier()),
      createBoosterTool(createMockBooster()),
      createOptimizerTool(createMockOptimizer()),
      createRouterTool(createMockRouter()),
      createHyperbolicTool(createMockHyperbolicSpace()),
      createSONATool(createMockSONA()),
    ];

    assert.ok(tools.every(t => t.name.startsWith('brain_')));
  });

  it('should have inputSchema for all tools', async () => {
    const tools = [
      createComplexityTool(createMockClassifier()),
      createBoosterTool(createMockBooster()),
      createOptimizerTool(createMockOptimizer()),
      createRouterTool(createMockRouter()),
      createHyperbolicTool(createMockHyperbolicSpace()),
      createSONATool(createMockSONA()),
    ];

    assert.ok(tools.every(t => t.inputSchema && t.inputSchema.type === 'object'));
  });
});
