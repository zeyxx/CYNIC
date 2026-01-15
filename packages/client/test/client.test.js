/**
 * CYNIC Client Tests
 *
 * "φ distrusts φ" - κυνικός
 */

/* global global */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert/strict';
import { CYNICClient, createHolDexClient } from '../src/index.js';

/**
 * Create a mock fetch function
 */
function createMockFetch(responses = {}) {
  const calls = [];

  const mockFn = async (url, options) => {
    calls.push({ url, options });

    // Find matching response
    const path = new URL(url).pathname;
    const method = options?.method || 'GET';
    const key = `${method} ${path}`;

    const response = responses[key] || responses[path] || responses.default;

    if (!response) {
      return {
        ok: false,
        status: 404,
        json: async () => ({ error: 'Not Found' }),
      };
    }

    if (response.error) {
      throw response.error;
    }

    return {
      ok: response.ok !== false,
      status: response.status || 200,
      json: async () => response.data,
    };
  };

  mockFn.calls = calls;
  return mockFn;
}

describe('CYNICClient', () => {
  let originalFetch;

  beforeEach(() => {
    originalFetch = global.fetch;
  });

  afterEach(() => {
    global.fetch = originalFetch;
  });

  describe('constructor', () => {
    it('uses default options', () => {
      const client = new CYNICClient();
      assert.equal(client.endpoint, 'http://localhost:3000');
      assert.equal(client.apiKey, null);
      assert.equal(client.timeout, 30000);
      assert.equal(client.retries, 3);
      assert.equal(client.verbose, false);
    });

    it('accepts custom options', () => {
      const client = new CYNICClient({
        endpoint: 'https://api.cynic.dev',
        apiKey: 'test-key',
        timeout: 5000,
        retries: 5,
        verbose: true,
      });
      assert.equal(client.endpoint, 'https://api.cynic.dev');
      assert.equal(client.apiKey, 'test-key');
      assert.equal(client.timeout, 5000);
      assert.equal(client.retries, 5);
      assert.equal(client.verbose, true);
    });

    it('removes trailing slash from endpoint', () => {
      const client = new CYNICClient({ endpoint: 'http://localhost:3000/' });
      assert.equal(client.endpoint, 'http://localhost:3000');
    });

    it('initializes stats', () => {
      const client = new CYNICClient();
      assert.deepEqual(client.stats, {
        requests: 0,
        successes: 0,
        failures: 0,
        lastRequest: null,
      });
    });
  });

  describe('health()', () => {
    it('returns health status', async () => {
      const mockFetch = createMockFetch({
        'GET /health': {
          data: { status: 'healthy', uptime: 1000 },
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient();
      const result = await client.health();

      assert.deepEqual(result, { status: 'healthy', uptime: 1000 });
      assert.equal(mockFetch.calls.length, 1);
      assert.ok(mockFetch.calls[0].url.endsWith('/health'));
    });
  });

  describe('info()', () => {
    it('returns node info', async () => {
      const mockFetch = createMockFetch({
        'GET /info': {
          data: { name: 'CYNIC', version: '0.1.0' },
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient();
      const result = await client.info();

      assert.deepEqual(result, { name: 'CYNIC', version: '0.1.0' });
    });
  });

  describe('consensusStatus()', () => {
    it('returns consensus status', async () => {
      const mockFetch = createMockFetch({
        'GET /consensus/status': {
          data: { height: 100, validators: 3 },
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient();
      const result = await client.consensusStatus();

      assert.deepEqual(result, { height: 100, validators: 3 });
    });
  });

  describe('judge()', () => {
    it('submits judgment request', async () => {
      const mockFetch = createMockFetch({
        'POST /judge': {
          data: { id: 'jdg_123', score: 75, verdict: 'WAG' },
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient();
      const result = await client.judge(
        { type: 'test', content: 'test content' },
        { source: 'unit-test' }
      );

      assert.equal(result.id, 'jdg_123');
      assert.equal(result.verdict, 'WAG');

      // Check request body
      const body = JSON.parse(mockFetch.calls[0].options.body);
      assert.deepEqual(body.item, { type: 'test', content: 'test content' });
      assert.deepEqual(body.context, { source: 'unit-test' });
    });
  });

  describe('submitKScore()', () => {
    it('submits K-Score with valid components', async () => {
      const mockFetch = createMockFetch({
        'POST /judge/kscore': {
          data: {
            requestId: 'ks_123',
            kScore: 82.5,
            tier: 'PLATINUM',
          },
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient();
      const result = await client.submitKScore('So11...mint', {
        D: 0.85,
        O: 0.72,
        L: 0.91,
      });

      assert.equal(result.kScore, 82.5);
      assert.equal(result.tier, 'PLATINUM');

      // Check request body
      const body = JSON.parse(mockFetch.calls[0].options.body);
      assert.equal(body.mint, 'So11...mint');
      assert.equal(body.components.D, 0.85);
    });

    it('throws on missing mint', async () => {
      const client = new CYNICClient();

      await assert.rejects(
        () => client.submitKScore(null, { D: 0.5, O: 0.5, L: 0.5 }),
        /Missing required parameter: mint/
      );
    });

    it('throws on missing components', async () => {
      const client = new CYNICClient();

      await assert.rejects(
        () => client.submitKScore('mint123', null),
        /Invalid components/
      );
    });

    it('throws on invalid component types', async () => {
      const client = new CYNICClient();

      await assert.rejects(
        () => client.submitKScore('mint123', { D: 'invalid', O: 0.5, L: 0.5 }),
        /Invalid components/
      );
    });

    it('throws on out-of-range components', async () => {
      const client = new CYNICClient();

      await assert.rejects(
        () => client.submitKScore('mint123', { D: 1.5, O: 0.5, L: 0.5 }),
        /Component values must be in range/
      );

      await assert.rejects(
        () => client.submitKScore('mint123', { D: -0.1, O: 0.5, L: 0.5 }),
        /Component values must be in range/
      );
    });

    it('truncates components to 6 decimal places', async () => {
      const mockFetch = createMockFetch({
        'POST /judge/kscore': { data: { kScore: 50 } },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient();
      await client.submitKScore('mint', {
        D: 0.123456789,
        O: 0.987654321,
        L: 0.555555555,
      });

      const body = JSON.parse(mockFetch.calls[0].options.body);
      assert.equal(body.components.D, 0.123457);
      assert.equal(body.components.O, 0.987654);
      assert.equal(body.components.L, 0.555556);
    });
  });

  describe('getMerkleProof()', () => {
    it('returns Merkle proof', async () => {
      const mockFetch = createMockFetch({
        'GET /merkle/proof/abc123': {
          data: { hash: 'abc123', proof: ['hash1', 'hash2'], root: 'root123' },
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient();
      const result = await client.getMerkleProof('abc123');

      assert.equal(result.hash, 'abc123');
      assert.ok(Array.isArray(result.proof));
    });
  });

  describe('getStats()', () => {
    it('returns client statistics', async () => {
      const mockFetch = createMockFetch({
        'GET /health': { data: { status: 'healthy' } },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient({ endpoint: 'http://test:3000' });
      await client.health();

      const stats = client.getStats();
      assert.equal(stats.requests, 1);
      assert.equal(stats.successes, 1);
      assert.equal(stats.failures, 0);
      assert.equal(stats.successRate, 1);
      assert.equal(stats.endpoint, 'http://test:3000');
    });

    it('calculates success rate correctly', async () => {
      const client = new CYNICClient();
      client.stats.requests = 10;
      client.stats.successes = 7;
      client.stats.failures = 3;

      const stats = client.getStats();
      assert.equal(stats.successRate, 0.7);
    });

    it('handles zero requests', () => {
      const client = new CYNICClient();
      const stats = client.getStats();
      assert.equal(stats.successRate, 0);
    });
  });

  describe('error handling', () => {
    it('throws on HTTP error', async () => {
      const mockFetch = createMockFetch({
        'GET /health': {
          ok: false,
          status: 500,
          data: { error: 'Internal Server Error' },
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient({ retries: 1 });

      await assert.rejects(
        () => client.health(),
        /Internal Server Error/
      );
    });

    it('does not retry on 400 errors', async () => {
      const mockFetch = createMockFetch({
        'GET /health': {
          ok: false,
          status: 400,
          data: {}, // Empty data forces fallback to "HTTP 400"
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient({ retries: 3 });

      await assert.rejects(() => client.health(), /HTTP 400/);
      // Should only have 1 call (no retries)
      assert.equal(mockFetch.calls.length, 1);
    });

    it('does not retry on 401 errors', async () => {
      const mockFetch = createMockFetch({
        'GET /health': {
          ok: false,
          status: 401,
          data: {}, // Empty data forces fallback to "HTTP 401"
        },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient({ retries: 3 });

      await assert.rejects(() => client.health(), /HTTP 401/);
      assert.equal(mockFetch.calls.length, 1);
    });

    it('retries on network errors', async () => {
      let callCount = 0;
      global.fetch = async () => {
        callCount++;
        if (callCount < 3) {
          throw new Error('Network error');
        }
        return {
          ok: true,
          status: 200,
          json: async () => ({ status: 'healthy' }),
        };
      };

      const client = new CYNICClient({ retries: 3 });
      const result = await client.health();

      assert.equal(result.status, 'healthy');
      assert.equal(callCount, 3);
    });

    it('tracks failures in stats', async () => {
      const mockFetch = createMockFetch({
        'GET /health': { error: new Error('Network error') },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient({ retries: 1 });

      try {
        await client.health();
      } catch (e) {
        // Expected
      }

      assert.equal(client.stats.requests, 1);
      assert.equal(client.stats.failures, 1);
      assert.equal(client.stats.successes, 0);
    });
  });

  describe('API key', () => {
    it('includes API key in headers when provided', async () => {
      const mockFetch = createMockFetch({
        'GET /health': { data: { status: 'healthy' } },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient({ apiKey: 'secret-key' });
      await client.health();

      assert.equal(mockFetch.calls[0].options.headers['X-API-Key'], 'secret-key');
    });

    it('does not include API key when not provided', async () => {
      const mockFetch = createMockFetch({
        'GET /health': { data: { status: 'healthy' } },
      });
      global.fetch = mockFetch;

      const client = new CYNICClient();
      await client.health();

      assert.equal(mockFetch.calls[0].options.headers['X-API-Key'], undefined);
    });
  });

  describe('static createPool()', () => {
    it('creates multiple clients', () => {
      const clients = CYNICClient.createPool([
        'http://node1:3000',
        'http://node2:3000',
        'http://node3:3000',
      ]);

      assert.equal(clients.length, 3);
      assert.equal(clients[0].endpoint, 'http://node1:3000');
      assert.equal(clients[1].endpoint, 'http://node2:3000');
      assert.equal(clients[2].endpoint, 'http://node3:3000');
    });

    it('passes options to all clients', () => {
      const clients = CYNICClient.createPool(
        ['http://node1:3000', 'http://node2:3000'],
        { apiKey: 'shared-key', timeout: 5000 }
      );

      assert.equal(clients[0].apiKey, 'shared-key');
      assert.equal(clients[0].timeout, 5000);
      assert.equal(clients[1].apiKey, 'shared-key');
      assert.equal(clients[1].timeout, 5000);
    });
  });

  describe('static submitKScoreRedundant()', () => {
    it('returns first successful result', async () => {
      const mockFetch = createMockFetch({
        'POST /judge/kscore': { data: { kScore: 80 } },
      });
      global.fetch = mockFetch;

      const clients = CYNICClient.createPool([
        'http://node1:3000',
        'http://node2:3000',
      ]);

      const result = await CYNICClient.submitKScoreRedundant(
        clients,
        'mint123',
        { D: 0.5, O: 0.5, L: 0.5 }
      );

      assert.equal(result.kScore, 80);
    });

    it('throws when all nodes fail', async () => {
      const mockFetch = createMockFetch({
        'POST /judge/kscore': { error: new Error('Connection refused') },
      });
      global.fetch = mockFetch;

      const clients = CYNICClient.createPool([
        'http://node1:3000',
        'http://node2:3000',
      ], { retries: 1 });

      await assert.rejects(
        () => CYNICClient.submitKScoreRedundant(clients, 'mint123', { D: 0.5, O: 0.5, L: 0.5 }),
        /All 2 nodes failed/
      );
    });
  });
});

describe('createHolDexClient()', () => {
  it('creates client with defaults', () => {
    const client = createHolDexClient();

    assert.ok(client instanceof CYNICClient);
    assert.equal(client.timeout, 30000);
    assert.equal(client.retries, 3);
  });

  it('accepts custom endpoint', () => {
    const client = createHolDexClient({ endpoint: 'http://custom:4000' });
    assert.equal(client.endpoint, 'http://custom:4000');
  });

  it('accepts custom API key', () => {
    const client = createHolDexClient({ apiKey: 'holdex-key' });
    assert.equal(client.apiKey, 'holdex-key');
  });
});

describe('waitForFinality()', () => {
  let originalFetch;

  beforeEach(() => {
    originalFetch = global.fetch;
  });

  afterEach(() => {
    global.fetch = originalFetch;
  });

  it('returns when consensus height advances', async () => {
    let callCount = 0;
    global.fetch = async () => {
      callCount++;
      return {
        ok: true,
        status: 200,
        json: async () => ({ height: callCount > 1 ? 5 : 0 }),
      };
    };

    const client = new CYNICClient();
    const result = await client.waitForFinality('req_123', {
      timeout: 5000,
      pollInterval: 50,
    });

    assert.equal(result.status, 'finalized');
    assert.equal(result.requestId, 'req_123');
    assert.equal(result.height, 5);
  });

  it('throws on timeout', async () => {
    global.fetch = async () => ({
      ok: true,
      status: 200,
      json: async () => ({ height: 0 }),
    });

    const client = new CYNICClient();

    await assert.rejects(
      () => client.waitForFinality('req_123', { timeout: 100, pollInterval: 50 }),
      /Timeout waiting for finality/
    );
  });
});
