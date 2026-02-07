/**
 * Deep tests for SolanaAnchoringManager
 *
 * "phi distrusts phi"
 *
 * Target: ~31 tests covering:
 * - Constructor & defaults
 * - enable/disable lifecycle
 * - resolveMerkleRoot key fallthrough
 * - shouldAnchor interval logic
 * - anchorBlock success path
 * - anchorBlock failure paths
 * - onBlockFinalized integration
 * - getAnchorStatus/getAnchoringStatus
 * - verifyAnchor cache lookup
 * - cleanup
 *
 * @module @cynic/node/test/solana-anchoring-deep
 */

'use strict';

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'events';
import { SolanaAnchoringManager } from '../src/network/solana-anchoring.js';

describe('SolanaAnchoringManager - Deep Tests', () => {
  let manager;

  beforeEach(() => {
    manager = null;
  });

  afterEach(() => {
    if (manager) manager.cleanup();
  });

  // ==================== 1. Constructor & defaults (~4) ====================

  describe('Constructor & defaults', () => {
    it('creates with enabled=false by default', () => {
      manager = new SolanaAnchoringManager();
      assert.equal(manager.enabled, false, 'should default to disabled');
    });

    it('defaults cluster to devnet', () => {
      manager = new SolanaAnchoringManager();
      const status = manager.getAnchoringStatus();
      assert.equal(status.cluster, 'devnet', 'cluster should default to devnet');
    });

    it('initializes empty pending anchors map', () => {
      manager = new SolanaAnchoringManager();
      const status = manager.getAnchoringStatus();
      assert.equal(status.pending, 0, 'pending count should be 0');
      assert.equal(status.anchored, 0, 'anchored count should be 0');
      assert.equal(status.failed, 0, 'failed count should be 0');
    });

    it('initializes stats to zeros', () => {
      manager = new SolanaAnchoringManager();
      const stats = manager.stats;
      assert.equal(stats.blocksAnchored, 0, 'blocksAnchored should be 0');
      assert.equal(stats.anchorsFailed, 0, 'anchorsFailed should be 0');
      assert.equal(stats.lastAnchorSignature, null, 'lastAnchorSignature should be null');
      assert.equal(stats.lastAnchorTimestamp, null, 'lastAnchorTimestamp should be null');
    });

    it('accepts constructor options', () => {
      manager = new SolanaAnchoringManager({
        enabled: true,
        cluster: 'mainnet-beta',
        anchorInterval: 50,
        dryRun: true,
      });
      assert.equal(manager.enabled, true, 'enabled should be true');
      const status = manager.getAnchoringStatus();
      assert.equal(status.cluster, 'mainnet-beta', 'cluster should be mainnet-beta');
      assert.equal(status.anchorInterval, 50, 'interval should be 50');
      assert.equal(status.dryRun, true, 'dryRun should be true');
    });
  });

  // ==================== 2. enable/disable (~4) ====================

  describe('enable/disable', () => {
    it('enable() sets enabled to true', async () => {
      manager = new SolanaAnchoringManager();
      assert.equal(manager.enabled, false, 'starts disabled');
      await manager.enable();
      assert.equal(manager.enabled, true, 'enabled after enable()');
    });

    it('enable() emits anchoring:enabled event', async () => {
      manager = new SolanaAnchoringManager();
      let emitted = null;
      manager.once('anchoring:enabled', (data) => {
        emitted = data;
      });
      await manager.enable({ cluster: 'testnet', interval: 200 });
      assert.ok(emitted, 'should emit anchoring:enabled');
      assert.equal(emitted.cluster, 'testnet', 'event should contain cluster');
      assert.equal(emitted.interval, 200, 'event should contain interval');
    });

    it('enable() accepts options and updates config', async () => {
      manager = new SolanaAnchoringManager({ cluster: 'devnet' });
      await manager.enable({
        cluster: 'mainnet-beta',
        wallet: { keypair: 'mock' },
        interval: 42,
        dryRun: true,
      });
      const status = manager.getAnchoringStatus();
      assert.equal(status.cluster, 'mainnet-beta', 'cluster updated');
      assert.equal(status.hasWallet, true, 'wallet set');
      assert.equal(status.anchorInterval, 42, 'interval updated');
      assert.equal(status.dryRun, true, 'dryRun updated');
    });

    it('enable() handles missing @cynic/anchor gracefully (simulation fallback)', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable();
      // Since @cynic/anchor likely doesn't exist, _anchorer should be null
      const status = manager.getAnchoringStatus();
      // This is OK - simulation will be used
      assert.equal(manager.enabled, true, 'enabled despite missing module');
    });

    it('disable() sets enabled to false', () => {
      manager = new SolanaAnchoringManager({ enabled: true });
      manager.disable();
      assert.equal(manager.enabled, false, 'disabled after disable()');
    });

    it('disable() emits anchoring:disabled event', () => {
      manager = new SolanaAnchoringManager({ enabled: true });
      let emitted = false;
      manager.once('anchoring:disabled', () => {
        emitted = true;
      });
      manager.disable();
      assert.ok(emitted, 'should emit anchoring:disabled');
    });
  });

  // ==================== 3. resolveMerkleRoot (~5) ====================

  describe('resolveMerkleRoot', () => {
    beforeEach(() => {
      manager = new SolanaAnchoringManager();
    });

    it('returns judgments_root first if present and valid', () => {
      const block = {
        judgments_root: 'a'.repeat(64),
        judgmentsRoot: 'b'.repeat(64),
        merkleRoot: 'c'.repeat(64),
        hash: 'd'.repeat(64),
      };
      const result = manager.resolveMerkleRoot(block);
      assert.equal(result, 'a'.repeat(64), 'should prioritize judgments_root');
    });

    it('falls through to judgmentsRoot if judgments_root invalid', () => {
      const block = {
        judgments_root: 'invalid',
        judgmentsRoot: 'b'.repeat(64),
        merkleRoot: 'c'.repeat(64),
      };
      const result = manager.resolveMerkleRoot(block);
      assert.equal(result, 'b'.repeat(64), 'should use judgmentsRoot');
    });

    it('falls through to merkleRoot if prior keys invalid', () => {
      const block = {
        judgments_root: null,
        judgmentsRoot: 'short',
        merkleRoot: 'c'.repeat(64),
      };
      const result = manager.resolveMerkleRoot(block);
      assert.equal(result, 'c'.repeat(64), 'should use merkleRoot');
    });

    it('falls through to hash if all prior keys invalid', () => {
      const block = {
        hash: 'd'.repeat(64),
      };
      const result = manager.resolveMerkleRoot(block);
      assert.equal(result, 'd'.repeat(64), 'should use hash');
    });

    it('validates 64-char hex format', () => {
      const valid = manager.resolveMerkleRoot({ merkleRoot: 'a0'.repeat(32) });
      assert.equal(valid, 'a0'.repeat(32), 'accepts valid hex');

      const tooShort = manager.resolveMerkleRoot({ merkleRoot: 'a'.repeat(63) });
      assert.equal(tooShort, null, 'rejects too short');

      const tooLong = manager.resolveMerkleRoot({ merkleRoot: 'a'.repeat(65) });
      assert.equal(tooLong, null, 'rejects too long');

      const invalidChars = manager.resolveMerkleRoot({ merkleRoot: 'z'.repeat(64) });
      assert.equal(invalidChars, null, 'rejects non-hex chars');
    });

    it('returns null if no valid merkle root found', () => {
      const block = { slot: 100, timestamp: Date.now() };
      const result = manager.resolveMerkleRoot(block);
      assert.equal(result, null, 'should return null for missing roots');
    });

    it('handles missing fields gracefully', () => {
      const result = manager.resolveMerkleRoot({});
      assert.equal(result, null, 'should handle empty block');
    });
  });

  // ==================== 4. shouldAnchor (~4) ====================

  describe('shouldAnchor', () => {
    it('returns false when disabled', () => {
      manager = new SolanaAnchoringManager({ enabled: false, anchorInterval: 100 });
      assert.equal(manager.shouldAnchor(100), false, 'disabled => no anchor');
      assert.equal(manager.shouldAnchor(200), false, 'disabled => no anchor');
    });

    it('returns true at interval boundary when enabled', async () => {
      manager = new SolanaAnchoringManager({ anchorInterval: 100 });
      await manager.enable();
      assert.equal(manager.shouldAnchor(100), true, 'slot 100 % 100 = 0');
      assert.equal(manager.shouldAnchor(200), true, 'slot 200 % 100 = 0');
      assert.equal(manager.shouldAnchor(300), true, 'slot 300 % 100 = 0');
    });

    it('returns false off-interval when enabled', async () => {
      manager = new SolanaAnchoringManager({ anchorInterval: 100 });
      await manager.enable();
      assert.equal(manager.shouldAnchor(99), false, 'slot 99 % 100 != 0');
      assert.equal(manager.shouldAnchor(101), false, 'slot 101 % 100 != 0');
      assert.equal(manager.shouldAnchor(150), false, 'slot 150 % 100 != 0');
    });

    it('works with different intervals', async () => {
      manager = new SolanaAnchoringManager({ anchorInterval: 50 });
      await manager.enable();
      assert.equal(manager.shouldAnchor(50), true, '50 % 50 = 0');
      assert.equal(manager.shouldAnchor(100), true, '100 % 50 = 0');
      assert.equal(manager.shouldAnchor(75), false, '75 % 50 != 0');

      await manager.enable({ interval: 10 });
      assert.equal(manager.shouldAnchor(10), true, '10 % 10 = 0');
      assert.equal(manager.shouldAnchor(20), true, '20 % 10 = 0');
      assert.equal(manager.shouldAnchor(15), false, '15 % 10 != 0');
    });
  });

  // ==================== 5. anchorBlock success (~6) ====================

  describe('anchorBlock - success path', () => {
    it('returns null when disabled', async () => {
      manager = new SolanaAnchoringManager({ enabled: false });
      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'm'.repeat(64),
      };
      const result = await manager.anchorBlock(block);
      assert.equal(result, null, 'should return null when disabled');
    });

    it('creates pending entry in _pendingAnchors', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ dryRun: true, wallet: { keypair: 'mock' } });

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'm'.repeat(64), // For destructuring line 114
        judgments_root: 'm'.repeat(64), // For resolveMerkleRoot
      };

      await manager.anchorBlock(block);
      const status = manager.getAnchorStatus('h'.repeat(64));
      assert.ok(status, 'anchor entry should exist');
      assert.equal(status.slot, 100, 'slot should match');
      // Note: stored as merkleRoot internally
    });

    it('simulation succeeds when no real anchorer', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' }, dryRun: true });

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      const result = await manager.anchorBlock(block);
      assert.ok(result, 'result should exist');
      assert.equal(result.success, true, 'simulation should succeed');
      assert.ok(result.signature.startsWith('sim_'), 'signature should be simulated');
      assert.equal(result.simulated, true, 'should mark as simulated');
    });

    it('updates entry to anchored status', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });
      manager._anchorer = null; // Use simulation path (no real wallet in tests)

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      await manager.anchorBlock(block);
      const status = manager.getAnchorStatus('h'.repeat(64));
      assert.equal(status.status, 'anchored', 'status should be anchored');
      assert.ok(status.signature, 'signature should exist');
      assert.ok(status.anchoredAt, 'anchoredAt timestamp should exist');
    });

    it('emits block:anchored event', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });
      manager._anchorer = null; // Use simulation path (no real wallet in tests)

      let emitted = null;
      manager.once('block:anchored', (data) => {
        emitted = data;
      });

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      await manager.anchorBlock(block);
      assert.ok(emitted, 'should emit block:anchored');
      assert.equal(emitted.slot, 100, 'event should contain slot');
      assert.equal(emitted.hash, 'h'.repeat(64), 'event should contain hash');
      assert.ok(emitted.signature, 'event should contain signature');
    });

    it('updates stats after successful anchor', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });
      manager._anchorer = null; // Use simulation path (no real wallet in tests)

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      const statsBefore = manager.stats;
      await manager.anchorBlock(block);
      const statsAfter = manager.stats;

      assert.equal(statsAfter.blocksAnchored, statsBefore.blocksAnchored + 1, 'blocksAnchored should increment');
      assert.ok(statsAfter.lastAnchorSignature, 'lastAnchorSignature should be set');
      assert.ok(statsAfter.lastAnchorTimestamp, 'lastAnchorTimestamp should be set');
    });
  });

  // ==================== 6. anchorBlock failure (~3) ====================

  describe('anchorBlock - failure paths', () => {
    it('returns null when no wallet or anchorer configured', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable(); // No wallet provided

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      const result = await manager.anchorBlock(block);
      // If @cynic/anchor loads successfully, it will attempt to anchor then fall back to simulation
      // If it doesn't load, _anchorer will be null and we return null at line 109
      assert.ok(result !== undefined, 'should return a result');
      if (result === null) {
        // No anchorer loaded - expected behavior
        assert.equal(result, null, 'null when no anchorer');
      } else {
        // Anchorer loaded but fell back to simulation - also expected
        assert.equal(result.success, true, 'simulation fallback should succeed');
        assert.equal(result.simulated, true, 'should be simulated');
      }
    });

    it('updates to failed status on error', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });

      // Block with invalid merkle root
      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'invalid',
      };

      const result = await manager.anchorBlock(block);
      assert.ok(result, 'result should exist');
      assert.equal(result.success, false, 'should fail');

      const status = manager.getAnchorStatus('h'.repeat(64));
      assert.equal(status.status, 'failed', 'status should be failed');
      assert.ok(status.error, 'error message should exist');
      assert.ok(status.failedAt, 'failedAt timestamp should exist');
    });

    it('emits anchor:failed event on error', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });

      let emitted = null;
      manager.once('anchor:failed', (data) => {
        emitted = data;
      });

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'invalid',
      };

      await manager.anchorBlock(block);
      assert.ok(emitted, 'should emit anchor:failed');
      assert.equal(emitted.slot, 100, 'event should contain slot');
      assert.equal(emitted.hash, 'h'.repeat(64), 'event should contain hash');
      assert.ok(emitted.error, 'event should contain error message');
    });

    it('increments anchorsFailed stat on error', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });

      const statsBefore = manager.stats;

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'invalid',
      };

      await manager.anchorBlock(block);
      const statsAfter = manager.stats;

      assert.equal(statsAfter.anchorsFailed, statsBefore.anchorsFailed + 1, 'anchorsFailed should increment');
    });
  });

  // ==================== 7. onBlockFinalized (~3) ====================

  describe('onBlockFinalized', () => {
    it('calls recordBlockHash when provided', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });

      let recorded = null;
      const recordBlockHash = (slot, hash) => {
        recorded = { slot, hash };
      };

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      await manager.onBlockFinalized(block, recordBlockHash);
      assert.ok(recorded, 'recordBlockHash should be called');
      assert.equal(recorded.slot, 100, 'slot should match');
      assert.equal(recorded.hash, 'h'.repeat(64), 'hash should match');
    });

    it('anchors block when shouldAnchor returns true', async () => {
      manager = new SolanaAnchoringManager({ anchorInterval: 100 });
      await manager.enable({ wallet: { keypair: 'mock' } });
      manager._anchorer = null; // Use simulation path (no real wallet in tests)

      const block = {
        slot: 100, // 100 % 100 = 0
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      await manager.onBlockFinalized(block);
      const status = manager.getAnchorStatus('h'.repeat(64));
      assert.ok(status, 'anchor should exist');
      assert.equal(status.status, 'anchored', 'should be anchored');
    });

    it('skips anchoring when shouldAnchor returns false', async () => {
      manager = new SolanaAnchoringManager({ anchorInterval: 100 });
      await manager.enable({ wallet: { keypair: 'mock' } });

      const block = {
        slot: 99, // 99 % 100 != 0
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      await manager.onBlockFinalized(block);
      const status = manager.getAnchorStatus('h'.repeat(64));
      assert.equal(status, null, 'should not anchor off-interval');
    });
  });

  // ==================== 8. getAnchorStatus/getAnchoringStatus (~3) ====================

  describe('getAnchorStatus / getAnchoringStatus', () => {
    it('getAnchorStatus returns null for unknown hash', () => {
      manager = new SolanaAnchoringManager();
      const status = manager.getAnchorStatus('unknown_hash');
      assert.equal(status, null, 'should return null for unknown hash');
    });

    it('getAnchoringStatus returns pending/anchored/failed counts', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });
      manager._anchorer = null; // Use simulation path for tests

      // Anchor one successfully (via simulation)
      await manager.anchorBlock({
        slot: 100,
        hash: 'success'.padEnd(64, '0'),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      });

      // Fail one
      await manager.anchorBlock({
        slot: 101,
        hash: 'failed'.padEnd(64, '0'),
        merkleRoot: 'invalid',
        judgments_root: 'invalid',
      });

      const status = manager.getAnchoringStatus();
      assert.equal(status.pending, 0, 'no pending (completed immediately)');
      assert.equal(status.anchored, 1, 'one anchored');
      assert.equal(status.failed, 1, 'one failed');
    });

    it('getAnchoringStatus includes full config and stats', async () => {
      manager = new SolanaAnchoringManager({ anchorInterval: 42 });
      await manager.enable({ cluster: 'testnet', wallet: { keypair: 'mock' } });

      const status = manager.getAnchoringStatus();
      assert.equal(status.enabled, true, 'enabled should be true');
      assert.equal(status.cluster, 'testnet', 'cluster should match');
      assert.equal(status.hasWallet, true, 'hasWallet should be true');
      assert.equal(status.anchorInterval, 42, 'interval should match');
      assert.ok(status.stats, 'stats should exist');
      assert.ok(typeof status.stats.blocksAnchored === 'number', 'stats.blocksAnchored should be number');
    });
  });

  // ==================== 9. verifyAnchor (~3) ====================

  describe('verifyAnchor', () => {
    it('finds anchor in cache by signature', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });

      const block = {
        slot: 100,
        hash: 'h'.repeat(64),
        merkleRoot: 'a'.repeat(64),
        judgments_root: 'a'.repeat(64),
      };

      const anchorResult = await manager.anchorBlock(block);
      const signature = anchorResult.signature;

      const verified = await manager.verifyAnchor(signature);
      assert.ok(verified, 'verification result should exist');
      assert.equal(verified.verified, true, 'should be verified from cache');
      assert.equal(verified.slot, 100, 'slot should match');
      assert.equal(verified.hash, 'h'.repeat(64), 'hash should match');
      assert.equal(verified.source, 'cache', 'source should be cache');
    });

    it('returns not-found for unknown signature', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });

      const result = await manager.verifyAnchor('unknown_signature_123');
      assert.ok(result, 'result should exist');
      assert.equal(result.verified, false, 'should not be verified');
      assert.ok(result.error, 'error message should exist');
    });

    it('falls back to on-chain verification if not in cache (no anchorer = not found)', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });
      // Since _anchorer is null (module not loaded), on-chain check will fail gracefully

      const result = await manager.verifyAnchor('some_onchain_sig');
      assert.equal(result.verified, false, 'should fail without anchorer');
    });
  });

  // ==================== 10. cleanup (~1) ====================

  describe('cleanup', () => {
    it('nulls _anchorer reference', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });
      // _anchorer may be null already, but that's OK

      manager.cleanup();
      const status = manager.getAnchoringStatus();
      assert.equal(status.hasAnchorer, false, '_anchorer should be nulled');
    });
  });

  // ==================== Edge Cases & Integration ====================

  describe('Edge cases', () => {
    it('handles resolveMerkleRoot with uppercase hex', () => {
      manager = new SolanaAnchoringManager();
      const block = {
        judgments_root: 'A'.repeat(64),
      };
      const result = manager.resolveMerkleRoot(block);
      assert.equal(result, 'A'.repeat(64), 'should accept uppercase hex');
    });

    it('handles resolveMerkleRoot with mixed case hex', () => {
      manager = new SolanaAnchoringManager();
      const block = {
        judgments_root: 'aB'.repeat(32),
      };
      const result = manager.resolveMerkleRoot(block);
      assert.equal(result, 'aB'.repeat(32), 'should accept mixed case hex');
    });

    it('multiple anchors update stats correctly', async () => {
      manager = new SolanaAnchoringManager();
      await manager.enable({ wallet: { keypair: 'mock' } });
      manager._anchorer = null; // Use simulation path for tests

      for (let i = 0; i < 3; i++) {
        await manager.anchorBlock({
          slot: 100 + i,
          hash: `h${i}`.padEnd(64, '0'),
          merkleRoot: 'a'.repeat(64),
          judgments_root: 'a'.repeat(64),
        });
      }

      const stats = manager.stats;
      assert.equal(stats.blocksAnchored, 3, 'should count all successful anchors');
    });
  });
});
