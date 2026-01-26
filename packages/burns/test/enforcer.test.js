/**
 * @cynic/burns - BurnEnforcer Tests
 *
 * Tests for burn enforcement before critical operations.
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert';

import {
  BurnEnforcer,
  createBurnEnforcer,
  BurnRequiredError,
  DEFAULT_ENFORCER_CONFIG,
} from '../src/enforcer.js';

describe('BurnEnforcer', () => {
  let enforcer;

  beforeEach(() => {
    enforcer = createBurnEnforcer({
      enabled: true,
      gracePeriod: 0, // Disable grace period for testing
    });
  });

  describe('constructor', () => {
    it('creates enforcer with defaults', () => {
      const e = createBurnEnforcer();
      assert.strictEqual(e.enabled, false); // Disabled by default
      assert.strictEqual(e.minAmount, DEFAULT_ENFORCER_CONFIG.minAmount);
    });

    it('accepts custom configuration', () => {
      const e = createBurnEnforcer({
        enabled: true,
        minAmount: 1_000_000,
        validityPeriod: 3600000,
        gracePeriod: 0,
        protectedOperations: ['judge'],
      });

      assert.strictEqual(e.enabled, true);
      assert.strictEqual(e.minAmount, 1_000_000);
      assert.strictEqual(e.validityPeriod, 3600000);
      assert.deepStrictEqual(e.protectedOperations, ['judge']);
    });
  });

  describe('requireBurn', () => {
    it('passes when enforcement is disabled', () => {
      const e = createBurnEnforcer({ enabled: false });
      assert.doesNotThrow(() => e.requireBurn('user1', 'judge'));
    });

    it('passes for unprotected operations', () => {
      assert.doesNotThrow(() => enforcer.requireBurn('user1', 'unprotected_op'));
    });

    it('throws BurnRequiredError for protected operation without burn', () => {
      assert.throws(
        () => enforcer.requireBurn('user1', 'judge'),
        BurnRequiredError
      );
    });

    it('throws with correct error details', () => {
      try {
        enforcer.requireBurn('user1', 'judge');
        assert.fail('Should have thrown');
      } catch (err) {
        assert.strictEqual(err.name, 'BurnRequiredError');
        assert.strictEqual(err.code, 'BURN_REQUIRED');
        assert.strictEqual(err.details.userId, 'user1');
        assert.strictEqual(err.details.operation, 'judge');
      }
    });

    it('passes when user has valid burn', async () => {
      // Mock verifier that always succeeds
      const mockVerifier = {
        verify: async () => ({
          verified: true,
          txSignature: 'test_tx',
          amount: 1_000_000_000,
          burner: 'test_burner',
          slot: 12345,
        }),
      };

      const e = createBurnEnforcer({
        enabled: true,
        gracePeriod: 0,
        verifier: mockVerifier,
      });

      await e.registerBurn('user1', 'test_tx');

      assert.doesNotThrow(() => e.requireBurn('user1', 'judge'));
    });
  });

  describe('registerBurn', () => {
    it('rejects without userId', async () => {
      await assert.rejects(
        enforcer.registerBurn(null, 'tx123'),
        /userId is required/
      );
    });

    it('rejects without txSignature', async () => {
      await assert.rejects(
        enforcer.registerBurn('user1', null),
        /txSignature is required/
      );
    });

    it('returns failure when verification fails', async () => {
      const mockVerifier = {
        verify: async () => ({
          verified: false,
          error: 'Transaction not found',
        }),
      };

      const e = createBurnEnforcer({
        enabled: true,
        verifier: mockVerifier,
      });

      const result = await e.registerBurn('user1', 'invalid_tx');

      assert.strictEqual(result.registered, false);
      assert.ok(result.error);
    });

    it('registers burn on successful verification', async () => {
      const mockVerifier = {
        verify: async () => ({
          verified: true,
          txSignature: 'valid_tx',
          amount: 618_000_000,
          burner: 'burner_address',
          slot: 99999,
        }),
      };

      const e = createBurnEnforcer({
        enabled: true,
        verifier: mockVerifier,
      });

      const result = await e.registerBurn('user1', 'valid_tx');

      assert.strictEqual(result.registered, true);
      assert.strictEqual(result.burn.amount, 618_000_000);
      assert.ok(result.burn.expiresAt > Date.now());
    });
  });

  describe('hasValidBurn', () => {
    it('returns true when enforcement disabled', () => {
      const e = createBurnEnforcer({ enabled: false });
      assert.strictEqual(e.hasValidBurn('anyone'), true);
    });

    it('returns false when no burn registered', () => {
      assert.strictEqual(enforcer.hasValidBurn('user1'), false);
    });

    it('returns true after valid burn registered', async () => {
      const mockVerifier = {
        verify: async () => ({
          verified: true,
          amount: 1_000_000_000,
        }),
      };

      const e = createBurnEnforcer({
        enabled: true,
        verifier: mockVerifier,
      });

      await e.registerBurn('user1', 'tx');
      assert.strictEqual(e.hasValidBurn('user1'), true);
    });

    it('returns false after burn expires', async () => {
      const mockVerifier = {
        verify: async () => ({
          verified: true,
          amount: 1_000_000_000,
        }),
      };

      const e = createBurnEnforcer({
        enabled: true,
        validityPeriod: 1, // 1ms - expires immediately
        verifier: mockVerifier,
      });

      await e.registerBurn('user1', 'tx');

      // Wait for expiration
      await new Promise(r => setTimeout(r, 10));

      assert.strictEqual(e.hasValidBurn('user1'), false);
    });
  });

  describe('gracePeriod', () => {
    it('allows operation during grace period', () => {
      const e = createBurnEnforcer({
        enabled: true,
        gracePeriod: 60000, // 1 minute
      });

      // First check should pass (grace period starts)
      assert.doesNotThrow(() => e.requireBurn('newuser', 'judge'));
    });

    it('tracks grace period correctly', () => {
      const e = createBurnEnforcer({
        enabled: true,
        gracePeriod: 60000,
      });

      assert.strictEqual(e.isInGracePeriod('newuser'), true);

      // After being seen, still in grace period
      assert.strictEqual(e.isInGracePeriod('newuser'), true);
    });

    it('denies after grace period expires', async () => {
      const e = createBurnEnforcer({
        enabled: true,
        gracePeriod: 1, // 1ms
      });

      // Start grace period
      e.isInGracePeriod('user1');

      // Wait for expiration
      await new Promise(r => setTimeout(r, 10));

      assert.strictEqual(e.isInGracePeriod('user1'), false);
      assert.throws(() => e.requireBurn('user1', 'judge'), BurnRequiredError);
    });
  });

  describe('getBurnStatus', () => {
    it('returns disabled status when enforcement off', () => {
      const e = createBurnEnforcer({ enabled: false });
      const status = e.getBurnStatus('user1');

      assert.strictEqual(status.enforcementEnabled, false);
      assert.strictEqual(status.hasValidBurn, true);
    });

    it('returns valid burn status', async () => {
      const mockVerifier = {
        verify: async () => ({
          verified: true,
          txSignature: 'tx123',
          amount: 618_000_000,
        }),
      };

      const e = createBurnEnforcer({
        enabled: true,
        gracePeriod: 0,
        verifier: mockVerifier,
      });

      await e.registerBurn('user1', 'tx123');
      const status = e.getBurnStatus('user1');

      assert.strictEqual(status.enforcementEnabled, true);
      assert.strictEqual(status.hasValidBurn, true);
      assert.ok(status.burn.txSignature);
      assert.ok(status.burn.remainingMs > 0);
    });

    it('returns grace period status', () => {
      const e = createBurnEnforcer({
        enabled: true,
        gracePeriod: 60000,
      });

      const status = e.getBurnStatus('newuser');

      assert.strictEqual(status.enforcementEnabled, true);
      assert.strictEqual(status.hasValidBurn, false);
      assert.strictEqual(status.inGracePeriod, true);
      assert.ok(status.gracePeriodRemainingMs > 0);
    });

    it('returns required status when no burn', () => {
      const status = enforcer.getBurnStatus('user1');

      assert.strictEqual(status.enforcementEnabled, true);
      assert.strictEqual(status.hasValidBurn, false);
      assert.strictEqual(status.inGracePeriod, false);
      assert.ok(status.message.includes('Burn required'));
    });
  });

  describe('revokeBurn', () => {
    it('removes registered burn', async () => {
      const mockVerifier = {
        verify: async () => ({ verified: true, amount: 1_000_000_000 }),
      };

      const e = createBurnEnforcer({
        enabled: true,
        verifier: mockVerifier,
      });

      await e.registerBurn('user1', 'tx');
      assert.strictEqual(e.hasValidBurn('user1'), true);

      e.revokeBurn('user1');
      assert.strictEqual(e.hasValidBurn('user1'), false);
    });
  });

  describe('getStats', () => {
    it('tracks enforcement statistics', async () => {
      const mockVerifier = {
        verify: async () => ({ verified: true, amount: 1_000_000_000 }),
      };

      const e = createBurnEnforcer({
        enabled: true,
        gracePeriod: 0,
        verifier: mockVerifier,
      });

      // Register burn
      await e.registerBurn('user1', 'tx');

      // Perform checks
      e.requireBurn('user1', 'judge');
      e.requireBurn('user1', 'refine');

      try {
        e.requireBurn('user2', 'judge');
      } catch (_) {}

      const stats = e.getStats();

      assert.strictEqual(stats.enabled, true);
      assert.strictEqual(stats.burnsRegistered, 1);
      assert.strictEqual(stats.checksPerformed, 3);
      assert.strictEqual(stats.checksPassed, 2);
      assert.strictEqual(stats.checksFailed, 1);
      assert.strictEqual(stats.activeBurns, 1);
    });
  });
});
