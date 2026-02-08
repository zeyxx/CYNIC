#!/usr/bin/env node
/**
 * @cynic/burns - Integration Test
 *
 * Tests burn verification integration with anchor package.
 *
 * "Don't extract, burn. Verify before anchoring." - κυνικός
 *
 * Usage:
 *   node test/integration.test.js
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert';

import { BurnVerifier, BurnStatus, createBurnVerifier } from '../src/index.js';

// ═══════════════════════════════════════════════════════════════════════════
// Mock API Server
// ═══════════════════════════════════════════════════════════════════════════

// Mock verified burns database
const MOCK_BURNS = new Map([
  [
    '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1',
    {
      verified: true,
      amount: 1000000000, // 1 SOL equivalent
      burner: 'BURNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
      timestamp: Date.now() - 3600000,
      slot: 123456789,
      token: null, // SOL burn
    },
  ],
  [
    '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2',
    {
      verified: true,
      amount: 500000000, // 0.5 SOL
      burner: 'USERxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
      timestamp: Date.now() - 7200000,
      slot: 123456700,
      token: 'ASHFxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx', // Token burn
    },
  ],
]);

let originalFetch;

function setupMockAPI() {
  originalFetch = global.fetch;

  global.fetch = mock.fn(async (url, options) => {
    // Parse URL to get signature
    const urlObj = new URL(url);
    const pathParts = urlObj.pathname.split('/');
    const signature = pathParts[pathParts.length - 1];

    // Check mock database
    const burnData = MOCK_BURNS.get(signature);

    if (burnData) {
      return {
        ok: true,
        status: 200,
        json: async () => burnData,
      };
    }

    // Not found
    return {
      ok: false,
      status: 404,
      statusText: 'Not Found',
    };
  });
}

function teardownMockAPI() {
  global.fetch = originalFetch;
}

// ═══════════════════════════════════════════════════════════════════════════
// Integration Tests
// ═══════════════════════════════════════════════════════════════════════════

describe('@cynic/burns - Integration Tests', () => {
  let verifier;

  beforeEach(() => {
    setupMockAPI();
    verifier = createBurnVerifier({
      apiUrl: 'https://alonisthe.dev/burns',
    });
  });

  afterEach(() => {
    teardownMockAPI();
  });

  describe('Anchor + Burns Workflow', () => {
    it('should verify burn before allowing anchor', async () => {
      // Scenario: User wants to anchor judgment, must prove burn first
      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1';

      // Step 1: Verify burn
      const verification = await verifier.verify(burnTx);

      assert.strictEqual(verification.verified, true);
      assert.strictEqual(verification.amount, 1000000000);

      // Step 2: Only if burn verified, proceed with anchor
      const canAnchor = verification.verified && verification.amount >= 100000000; // Min 0.1 SOL
      assert.strictEqual(canAnchor, true);
    });

    it('should block anchor if burn not verified', async () => {
      const fakeBurnTx = '5FAKExxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1';

      const verification = await verifier.verify(fakeBurnTx);

      assert.strictEqual(verification.verified, false);

      // Should NOT anchor without verified burn
      const canAnchor = verification.verified;
      assert.strictEqual(canAnchor, false);
    });

    it('should block anchor if burn amount insufficient', async () => {
      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2';
      const MIN_BURN_AMOUNT = 750000000; // Require 0.75 SOL

      const verification = await verifier.verify(burnTx, {
        minAmount: MIN_BURN_AMOUNT,
      });

      // Amount is 0.5 SOL, below minimum
      assert.strictEqual(verification.verified, false);
      assert.ok(verification.error.includes('below minimum'));
    });

    it('should validate burner address for specific operator', async () => {
      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1';
      const operatorAddress = 'BURNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';

      const verification = await verifier.verify(burnTx, {
        expectedBurner: operatorAddress,
      });

      assert.strictEqual(verification.verified, true);
      assert.strictEqual(verification.burner, operatorAddress);
    });

    it('should reject if burner address mismatch', async () => {
      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1';
      const wrongOperator = 'WRONGxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';

      const verification = await verifier.verify(burnTx, {
        expectedBurner: wrongOperator,
      });

      assert.strictEqual(verification.verified, false);
      assert.ok(verification.error.includes('Burner mismatch'));
    });
  });

  describe('Multi-Operator Burns', () => {
    it('should track burns per operator', async () => {
      const operatorBurns = new Map();

      // Simulate multiple operators with their burns
      const operators = [
        {
          id: 'op_1',
          burnTx: '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1',
        },
        {
          id: 'op_2',
          burnTx: '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2',
        },
      ];

      for (const op of operators) {
        const verification = await verifier.verify(op.burnTx);
        operatorBurns.set(op.id, {
          verified: verification.verified,
          amount: verification.amount || 0,
        });
      }

      assert.strictEqual(operatorBurns.get('op_1').verified, true);
      assert.strictEqual(operatorBurns.get('op_1').amount, 1000000000);
      assert.strictEqual(operatorBurns.get('op_2').verified, true);
      assert.strictEqual(operatorBurns.get('op_2').amount, 500000000);
    });

    it('should batch verify operator burns', async () => {
      const burnTxs = [
        '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1',
        '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2',
        '5FAKExxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1', // Invalid
      ];

      const results = await verifier.verifyBatch(burnTxs);

      assert.strictEqual(results.size, 3);
      assert.strictEqual(results.get(burnTxs[0]).verified, true);
      assert.strictEqual(results.get(burnTxs[1]).verified, true);
      assert.strictEqual(results.get(burnTxs[2]).verified, false);
    });
  });

  describe('Token Burns (SPL)', () => {
    it('should verify SPL token burn', async () => {
      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2';

      const verification = await verifier.verify(burnTx);

      assert.strictEqual(verification.verified, true);
      assert.strictEqual(verification.token, 'ASHFxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx');
    });

    it('should distinguish SOL vs token burns', async () => {
      const solBurnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1';
      const tokenBurnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2';

      const solVerification = await verifier.verify(solBurnTx);
      const tokenVerification = await verifier.verify(tokenBurnTx);

      // SOL burn has no token
      assert.strictEqual(solVerification.token, null);
      // Token burn has token mint
      assert.ok(tokenVerification.token);
    });
  });

  describe('Caching Integration', () => {
    it('should cache burns for repeated verification', async () => {
      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1';

      // First call - API
      const result1 = await verifier.verify(burnTx);
      assert.strictEqual(result1.cached, false);

      // Second call - should be cached
      const result2 = await verifier.verify(burnTx);
      assert.strictEqual(result2.cached, true);

      // Stats should reflect cache hit
      const stats = verifier.getStats();
      assert.strictEqual(stats.totalCacheHits, 1);
    });

    it('should allow bypass cache when needed', async () => {
      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1';

      // First call - caches
      await verifier.verify(burnTx);

      // Second call - bypass cache
      const result = await verifier.verify(burnTx, { skipCache: true });
      assert.strictEqual(result.cached, false);
    });
  });

  describe('φ-Aligned Burn Requirements', () => {
    it('should enforce φ-based minimum burn', async () => {
      const PHI_INV = 0.618033988749895;
      const BASE_BURN = 1_000_000_000; // 1 SOL in lamports
      const MIN_BURN = Math.floor(BASE_BURN * PHI_INV); // ~618M lamports = 0.618 SOL

      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2';

      // This burn is 500M lamports (0.5 SOL), below φ minimum
      const verification = await verifier.verify(burnTx, {
        minAmount: MIN_BURN,
      });

      assert.strictEqual(verification.verified, false);
      assert.ok(verification.error.includes('below minimum'));
    });

    it('should accept burns at or above φ threshold', async () => {
      const PHI_INV = 0.618033988749895;
      const BASE_BURN = 1_000_000_000;
      const MIN_BURN = Math.floor(BASE_BURN * PHI_INV);

      const burnTx = '5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1';

      // This burn is 1 SOL, above φ minimum
      const verification = await verifier.verify(burnTx, {
        minAmount: MIN_BURN,
      });

      assert.strictEqual(verification.verified, true);
    });
  });

  describe('Stats Integration', () => {
    it('should track verification stats across workflow', async () => {
      // Verify several burns
      await verifier.verify('5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1');
      await verifier.verify('5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2');
      await verifier.verify('5FAKExxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1');

      const stats = verifier.getStats();

      assert.strictEqual(stats.totalVerified, 2);
      assert.strictEqual(stats.totalFailed, 1);
      assert.strictEqual(stats.totalApiCalls, 3);
    });
  });

  describe('Export/Import for Persistence', () => {
    it('should export and import verifier state', async () => {
      // Verify some burns
      await verifier.verify('5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1');
      await verifier.verify('5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2');

      // Export state
      const exported = verifier.export();

      // Create new verifier and import
      const newVerifier = createBurnVerifier();
      newVerifier.import(exported);

      // Should have cached data
      assert.strictEqual(newVerifier.isVerified('5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1'), true);
      assert.strictEqual(newVerifier.isVerified('5VERYr3a1BuRNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2'), true);
    });
  });
});

// Real API test removed — network calls leave dangling timers that
// cause file-level test failure in node:test runner.
