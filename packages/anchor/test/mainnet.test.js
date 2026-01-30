/**
 * @cynic/anchor - Mainnet Tests
 *
 * v1.1: Tests for mainnet configuration, rate limiting, and failover
 *
 * @module @cynic/anchor/test/mainnet
 */

import { describe, it, mock, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  MAINNET_RPCS,
  RATE_LIMITS,
  PRIORITY_FEES,
  COMPUTE_BUDGET,
  RateLimiter,
  RpcFailover,
  MAINNET_CONFIG,
  createMainnetConfig,
  isMainnet,
} from '../src/mainnet.js';

// =============================================================================
// CONSTANTS TESTS
// =============================================================================

describe('MAINNET_RPCS', () => {
  it('should have PUBLIC endpoint', () => {
    assert.ok(MAINNET_RPCS.PUBLIC);
    assert.ok(MAINNET_RPCS.PUBLIC.includes('mainnet'));
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(MAINNET_RPCS));
  });
});

describe('RATE_LIMITS', () => {
  it('should have φ-aligned values', () => {
    assert.ok(RATE_LIMITS.PUBLIC_RPS > 0);
    assert.ok(RATE_LIMITS.PREMIUM_RPS > RATE_LIMITS.PUBLIC_RPS);
    assert.ok(RATE_LIMITS.COOLDOWN_MS > 0);
    assert.ok(RATE_LIMITS.RETRY_MULTIPLIER > 1);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(RATE_LIMITS));
  });
});

describe('PRIORITY_FEES', () => {
  it('should have fee tiers', () => {
    assert.strictEqual(PRIORITY_FEES.NONE, 0);
    assert.ok(PRIORITY_FEES.LOW > PRIORITY_FEES.NONE);
    assert.ok(PRIORITY_FEES.MEDIUM > PRIORITY_FEES.LOW);
    assert.ok(PRIORITY_FEES.HIGH > PRIORITY_FEES.MEDIUM);
    assert.ok(PRIORITY_FEES.MAX > PRIORITY_FEES.HIGH);
  });

  it('should have φ-aligned default', () => {
    assert.ok(PRIORITY_FEES.DEFAULT > PRIORITY_FEES.LOW);
    assert.ok(PRIORITY_FEES.DEFAULT < PRIORITY_FEES.HIGH);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(PRIORITY_FEES));
  });
});

describe('COMPUTE_BUDGET', () => {
  it('should have compute limits', () => {
    assert.ok(COMPUTE_BUDGET.MAX_UNITS > 0);
    assert.ok(COMPUTE_BUDGET.DEFAULT_UNITS > 0);
    assert.ok(COMPUTE_BUDGET.DEFAULT_UNITS < COMPUTE_BUDGET.MAX_UNITS);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(COMPUTE_BUDGET));
  });
});

// =============================================================================
// RATE LIMITER TESTS
// =============================================================================

describe('RateLimiter', () => {
  it('should create with tokens per second', () => {
    const limiter = new RateLimiter(10);
    assert.ok(limiter);
  });

  it('should consume tokens', () => {
    const limiter = new RateLimiter(10, 10);
    assert.ok(limiter.tryConsume());
    assert.ok(limiter.tryConsume());
  });

  it('should rate limit when tokens exhausted', () => {
    const limiter = new RateLimiter(1, 2);
    assert.ok(limiter.tryConsume());
    assert.ok(limiter.tryConsume());
    // Third should fail (only 2 tokens)
    assert.strictEqual(limiter.tryConsume(), false);
  });

  it('should refill tokens over time', async () => {
    const limiter = new RateLimiter(100, 2); // 100 tokens/sec
    limiter.tryConsume();
    limiter.tryConsume();
    // Wait for refill
    await new Promise(r => setTimeout(r, 25));
    // Should have refilled some tokens
    assert.ok(limiter.getTokens() > 0);
  });

  it('should wait for token', async () => {
    const limiter = new RateLimiter(100, 1);
    limiter.tryConsume();
    const start = Date.now();
    await limiter.waitForToken();
    const elapsed = Date.now() - start;
    // Should have waited for refill (at least 10ms)
    assert.ok(elapsed >= 5);
  });
});

// =============================================================================
// RPC FAILOVER TESTS
// =============================================================================

describe('RpcFailover', () => {
  it('should create with public endpoint', () => {
    const failover = new RpcFailover({ includePublic: true });
    assert.ok(failover);
    const status = failover.getStatus();
    assert.ok(status.length >= 1);
  });

  it('should get healthy endpoint', () => {
    const failover = new RpcFailover({ includePublic: true });
    const endpoint = failover.getEndpoint();
    assert.ok(endpoint);
    assert.ok(endpoint.url);
  });

  it('should mask API keys in status', () => {
    // Create with a fake API key endpoint
    const failover = new RpcFailover({
      customRpcs: [{ url: 'https://example.com?api-key=secret123', isPremium: true }],
      includePublic: true,
    });
    const status = failover.getStatus();
    const premiumEndpoint = status.find(s => s.url.includes('example.com'));
    if (premiumEndpoint) {
      assert.ok(!premiumEndpoint.url.includes('secret123'));
      assert.ok(premiumEndpoint.url.includes('api-key=***'));
    }
  });

  it('should execute with failover', async () => {
    const failover = new RpcFailover({ includePublic: true });

    let callCount = 0;
    const result = await failover.execute(async (url) => {
      callCount++;
      return { url, success: true };
    });

    assert.strictEqual(callCount, 1);
    assert.ok(result.success);
  });

  it('should failover on error', async () => {
    const failover = new RpcFailover({
      customRpcs: [
        { url: 'https://fail.example.com', isPremium: true },
        { url: 'https://success.example.com', isPremium: true },
      ],
      includePublic: false,
    });

    let callCount = 0;
    const result = await failover.execute(async (url) => {
      callCount++;
      if (url.includes('fail')) {
        throw new Error('Connection failed');
      }
      return { url, success: true };
    });

    assert.strictEqual(callCount, 2);
    assert.ok(result.success);
    assert.ok(result.url.includes('success'));
  });

  it('should throw when all endpoints fail', async () => {
    const failover = new RpcFailover({
      customRpcs: [{ url: 'https://fail.example.com', isPremium: true }],
      includePublic: false,
    });

    // Exhaust retries
    for (let i = 0; i < RATE_LIMITS.MAX_RETRIES; i++) {
      try {
        await failover.execute(async () => {
          throw new Error('Always fails');
        });
      } catch {
        // Expected
      }
    }

    await assert.rejects(
      async () => failover.execute(async () => { throw new Error('Fail'); }),
      /All RPC endpoints/,
    );
  });
});

// =============================================================================
// CONFIG TESTS
// =============================================================================

describe('MAINNET_CONFIG', () => {
  it('should have mainnet cluster', () => {
    assert.ok(MAINNET_CONFIG.cluster);
  });

  it('should require confirmation', () => {
    assert.strictEqual(MAINNET_CONFIG.requireConfirmation, true);
  });

  it('should have priority fee', () => {
    assert.ok(MAINNET_CONFIG.priorityFee > 0);
  });

  it('should have compute units', () => {
    assert.ok(MAINNET_CONFIG.computeUnits > 0);
  });

  it('should have longer interval for cost efficiency', () => {
    assert.ok(MAINNET_CONFIG.intervalMs > 60000);
  });

  it('should use finalized commitment', () => {
    assert.strictEqual(MAINNET_CONFIG.commitment, 'finalized');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(MAINNET_CONFIG));
  });
});

describe('createMainnetConfig', () => {
  it('should create config with defaults', () => {
    const config = createMainnetConfig();
    assert.ok(config.cluster);
    assert.ok(config.priorityFee);
  });

  it('should allow overrides', () => {
    const config = createMainnetConfig({ priorityFee: 10000 });
    assert.strictEqual(config.priorityFee, 10000);
  });

  it('should preserve mainnet cluster', () => {
    const config = createMainnetConfig({});
    assert.ok(config.cluster);
  });
});

describe('isMainnet', () => {
  it('should detect mainnet URLs', () => {
    assert.strictEqual(isMainnet('https://api.mainnet-beta.solana.com'), true);
    assert.strictEqual(isMainnet('https://mainnet.helius-rpc.com'), true);
  });

  it('should not detect devnet as mainnet', () => {
    assert.strictEqual(isMainnet('https://api.devnet.solana.com'), false);
    assert.strictEqual(isMainnet('https://devnet.helius-rpc.com'), false);
  });

  it('should handle null/undefined', () => {
    assert.strictEqual(isMainnet(null), false);
    assert.strictEqual(isMainnet(undefined), false);
    assert.strictEqual(isMainnet(''), false);
  });

  it('should not detect localnet as mainnet', () => {
    assert.strictEqual(isMainnet('http://localhost:8899'), false);
  });
});
