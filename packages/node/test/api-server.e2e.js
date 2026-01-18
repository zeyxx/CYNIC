/**
 * API Server Tests
 *
 * Tests for the CYNIC HTTP API server.
 *
 * @module @cynic/node/test/api-server
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { APIServer } from '../src/api/server.js';

/**
 * Create mock node for testing
 */
function createMockNode(overrides = {}) {
  return {
    status: 'RUNNING',
    startedAt: Date.now() - 10000,
    getInfo: () => ({
      id: 'node_test123',
      name: 'test-node',
      status: 'RUNNING',
      uptime: 10000,
      operator: { id: 'op_test', name: 'Test Operator' },
      state: { chain: { height: 5 } },
      judge: { totalJudgments: 10 },
      residual: {},
      gossip: { peers: 3 },
    }),
    gossip: {
      getStats: () => ({ peers: 3, messagesReceived: 100 }),
    },
    state: {
      chain: { height: 5, latestHash: 'abc123' },
      knowledge: {
        getProof: (hash) => hash === 'known_hash' ? { siblings: ['a', 'b'] } : null,
        root: 'merkle_root',
      },
      getSummary: () => ({
        chain: { height: 5, latestHash: 'abc123' },
        pendingJudgments: 2,
      }),
    },
    judge: async (item, context) => ({
      global_score: 75,
      qScore: 72,
      verdict: 'WAG',
      qVerdict: { verdict: 'WAG' },
      confidence: 0.6,
      axiomScores: { PHI: 0.8, VERIFY: 0.7, CULTURE: 0.6, BURN: 0.75 },
      weaknesses: [],
      dimensions: {},
      ...overrides.judgeResult,
    }),
    ...overrides,
  };
}

// Use random ports to avoid conflicts between tests when running with other test files
// Start from a high port range to avoid common conflicts
let nextPort = 29000 + Math.floor(Math.random() * 1000);
function getTestPort() {
  return nextPort++;
}

/**
 * Make HTTP request to server
 */
async function request(server, method, path, body = null, headers = {}) {
  const url = `http://localhost:${server.port}${path}`;
  const options = {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...headers,
    },
  };

  if (body) {
    options.body = JSON.stringify(body);
  }

  try {
    const response = await fetch(url, options);
    const text = await response.text();
    let data;
    try {
      data = JSON.parse(text);
    } catch {
      data = text;
    }
    return { status: response.status, data };
  } catch (err) {
    console.error(`Request failed: ${method} ${url}`, err.message);
    throw err;
  }
}

describe('APIServer', () => {
  let server;

  afterEach(async () => {
    if (server) {
      try {
        if (server.server) {
          await server.stop();
        }
      } catch {
        // Ignore cleanup errors
      }
      server = null;
    }
    // Give time for sockets to close
    await new Promise(r => setTimeout(r, 50));
  });

  describe('constructor', () => {
    it('creates with default options', () => {
      server = new APIServer({ node: createMockNode() });
      assert.ok(server.app);
      assert.equal(server.server, null);
    });

    it('accepts custom port', () => {
      server = new APIServer({ node: createMockNode(), port: 4000 });
      assert.equal(server.port, 4000);
    });

    it('accepts API key', () => {
      server = new APIServer({ node: createMockNode(), apiKey: 'secret' });
      assert.equal(server.apiKey, 'secret');
    });
  });

  describe('start/stop', () => {
    it('starts and stops server', async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() }); // Port 0 = random available port
      const result = await server.start();

      assert.ok(result.port > 0);
      assert.ok(result.url.includes('localhost'));
      assert.ok(server.server);

      await server.stop();
      assert.equal(server.server, null);
    });
  });

  describe('GET /', () => {
    beforeEach(async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();
    });

    it('returns API info', async () => {
      const { status, data } = await request(server, 'GET', '/');

      assert.equal(status, 200);
      assert.equal(data.name, 'CYNIC API');
      assert.ok(data.endpoints);
      assert.ok(data.greek);
    });
  });

  describe('GET /health', () => {
    it('returns healthy when node is running', async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();

      const { status, data } = await request(server, 'GET', '/health');

      assert.equal(status, 200);
      assert.equal(data.status, 'healthy');
      assert.equal(data.nodeStatus, 'RUNNING');
      assert.ok(data.phi);
    });

    it('returns unhealthy when node not running', async () => {
      const stoppedNode = createMockNode({
        getInfo: () => ({ status: 'STOPPED' }),
      });
      server = new APIServer({ node: stoppedNode, port: getTestPort() });
      await server.start();

      const { status, data } = await request(server, 'GET', '/health');

      assert.equal(status, 503);
      assert.equal(data.status, 'unhealthy');
    });

    it('handles missing node', async () => {
      server = new APIServer({ node: null, port: getTestPort() });
      await server.start();

      const { status, data } = await request(server, 'GET', '/health');

      assert.equal(status, 503);
      assert.equal(data.nodeStatus, 'NO_NODE');
    });
  });

  describe('GET /info', () => {
    beforeEach(async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();
    });

    it('returns node info', async () => {
      const { status, data } = await request(server, 'GET', '/info');

      assert.equal(status, 200);
      assert.equal(data.name, 'test-node');
      assert.equal(data.status, 'RUNNING');
    });

    it('returns 503 when no node', async () => {
      await server.stop();
      server = new APIServer({ node: null, port: getTestPort() });
      await server.start();

      const { status, data } = await request(server, 'GET', '/info');

      assert.equal(status, 503);
      assert.ok(data.error);
    });
  });

  describe('GET /consensus/status', () => {
    beforeEach(async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();
    });

    it('returns consensus status', async () => {
      const { status, data } = await request(server, 'GET', '/consensus/status');

      assert.equal(status, 200);
      assert.equal(data.height, 5);
      assert.equal(data.validators, 3);
      assert.ok(data.consensusThreshold);
    });
  });

  describe('POST /judge', () => {
    beforeEach(async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();
    });

    it('judges valid item', async () => {
      const { status, data } = await request(server, 'POST', '/judge', {
        type: 'test',
        item: { content: 'test content', source: 'unit test' },
      });

      assert.equal(status, 201);
      assert.ok(data.requestId);
      assert.equal(data.status, 'finalized');
      assert.ok(data.judgment);
      assert.ok(data.judgment.verdict);
      assert.ok(data.judgment.qScore);
    });

    it('rejects missing item', async () => {
      const { status, data } = await request(server, 'POST', '/judge', {
        type: 'test',
      });

      assert.equal(status, 400);
      assert.ok(data.error.includes('Bad Request'));
    });

    it('rejects null item', async () => {
      const { status, data } = await request(server, 'POST', '/judge', {
        type: 'test',
        item: null,
      });

      assert.equal(status, 400);
      assert.ok(data.message.includes('non-null'));
    });

    it('rejects empty item', async () => {
      const { status, data } = await request(server, 'POST', '/judge', {
        type: 'test',
        item: {},
      });

      assert.equal(status, 400);
      assert.ok(data.message.includes('empty'));
    });

    it('rejects array item', async () => {
      const { status, data } = await request(server, 'POST', '/judge', {
        type: 'test',
        item: ['a', 'b'],
      });

      assert.equal(status, 400);
    });

    it('includes Final score when kScore provided', async () => {
      const nodeWithKScore = createMockNode({
        judgeResult: {
          kScore: 80,
          finalScore: 72,
          finalVerdict: { verdict: 'WAG' },
          limiting: 'Q',
        },
      });
      await server.stop();
      server = new APIServer({ node: nodeWithKScore, port: getTestPort() });
      await server.start();

      const { status, data } = await request(server, 'POST', '/judge', {
        item: { content: 'test' },
        context: { kScore: 80 },
      });

      assert.equal(status, 201);
      assert.equal(data.judgment.kScore, 80);
      assert.equal(data.judgment.finalScore, 72);
    });
  });

  describe('POST /judge/kscore', () => {
    beforeEach(async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();
    });

    it('calculates K-Score', async () => {
      const { status, data } = await request(server, 'POST', '/judge/kscore', {
        mint: 'token123',
        components: { D: 0.8, O: 0.7, L: 0.6 },
      });

      assert.equal(status, 201);
      assert.ok(data.requestId);
      assert.equal(data.mint, 'token123');
      assert.ok(data.kScore > 0);
      assert.ok(data.components);
    });

    it('rejects missing mint', async () => {
      const { status, data } = await request(server, 'POST', '/judge/kscore', {
        components: { D: 0.8, O: 0.7, L: 0.6 },
      });

      assert.equal(status, 400);
      assert.ok(data.message.includes('mint'));
    });

    it('rejects invalid components', async () => {
      const { status, data } = await request(server, 'POST', '/judge/kscore', {
        mint: 'token123',
        components: { D: 'not a number', O: 0.7, L: 0.6 },
      });

      assert.equal(status, 400);
      assert.ok(data.message.includes('components'));
    });

    it('rejects missing components', async () => {
      const { status, data } = await request(server, 'POST', '/judge/kscore', {
        mint: 'token123',
      });

      assert.equal(status, 400);
    });
  });

  describe('GET /merkle/proof/:hash', () => {
    beforeEach(async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();
    });

    it('returns proof for known hash', async () => {
      const { status, data } = await request(server, 'GET', '/merkle/proof/known_hash');

      assert.equal(status, 200);
      assert.equal(data.hash, 'known_hash');
      assert.ok(data.proof);
      assert.equal(data.root, 'merkle_root');
    });

    it('returns 404 for unknown hash', async () => {
      const { status, data } = await request(server, 'GET', '/merkle/proof/unknown_hash');

      assert.equal(status, 404);
      assert.ok(data.error.includes('Not Found'));
    });
  });

  describe('404 handler', () => {
    beforeEach(async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();
    });

    it('returns 404 for unknown routes', async () => {
      const { status, data } = await request(server, 'GET', '/nonexistent');

      assert.equal(status, 404);
      assert.ok(data.error.includes('Not Found'));
      assert.ok(data.available);
    });
  });

  describe('CORS', () => {
    beforeEach(async () => {
      server = new APIServer({ node: createMockNode(), port: getTestPort() });
      await server.start();
    });

    it('handles OPTIONS request', async () => {
      const url = `http://localhost:${server.port}/`;
      const response = await fetch(url, { method: 'OPTIONS' });

      assert.equal(response.status, 204);
      assert.ok(response.headers.get('Access-Control-Allow-Origin'));
    });
  });

  describe('API key auth', () => {
    beforeEach(async () => {
      server = new APIServer({
        node: createMockNode(),
        port: getTestPort(),
        apiKey: 'test-secret-key',
      });
      await server.start();
    });

    it('allows health without API key', async () => {
      const { status } = await request(server, 'GET', '/health');
      assert.equal(status, 200);
    });

    it('allows root without API key', async () => {
      const { status } = await request(server, 'GET', '/');
      assert.equal(status, 200);
    });

    it('rejects protected routes without API key', async () => {
      const { status, data } = await request(server, 'GET', '/info');

      assert.equal(status, 401);
      assert.ok(data.error.includes('Unauthorized'));
    });

    it('rejects invalid API key', async () => {
      const { status, data } = await request(server, 'GET', '/info', null, {
        'X-API-Key': 'wrong-key',
      });

      assert.equal(status, 401);
    });

    it('accepts valid API key', async () => {
      const { status, data } = await request(server, 'GET', '/info', null, {
        'X-API-Key': 'test-secret-key',
      });

      assert.equal(status, 200);
      assert.equal(data.name, 'test-node');
    });
  });

  describe('error handling', () => {
    it('handles judge errors', async () => {
      const errorNode = createMockNode({
        judge: async () => {
          throw new Error('Judge failed');
        },
      });
      server = new APIServer({ node: errorNode, port: getTestPort() });
      await server.start();

      const { status, data } = await request(server, 'POST', '/judge', {
        item: { content: 'test' },
      });

      assert.equal(status, 500);
      assert.ok(data.error.includes('Internal Server Error'));
    });
  });
});
