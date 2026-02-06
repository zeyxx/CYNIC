/**
 * ForkDetector Tests
 *
 * PHASE 2: DECENTRALIZE
 *
 * Tests chain fork detection, heaviest branch calculation,
 * resolution flow, and cleanup.
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';
import { ForkDetector } from '../src/network/fork-detector.js';

function createMockFn(impl) {
  const calls = [];
  const fn = (...args) => { calls.push(args); return impl?.(...args); };
  fn.mock = { calls, get callCount() { return calls.length; } };
  return fn;
}

async function waitFor(fn, timeout = 1000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try { fn(); return; } catch { await new Promise(r => setTimeout(r, 10)); }
  }
  fn();
}

describe('ForkDetector', () => {
  let detector;

  beforeEach(() => {
    detector = new ForkDetector();
  });

  describe('constructor', () => {
    it('starts with no forks detected', () => {
      assert.strictEqual(detector.stats.forksDetected, 0);
      assert.strictEqual(detector.stats.forksResolved, 0);
    });

    it('starts with clean fork state', () => {
      const status = detector.getForkStatus();

      assert.strictEqual(status.detected, false);
      assert.strictEqual(status.forkSlot, null);
      assert.strictEqual(status.resolutionInProgress, false);
      assert.strictEqual(status.branches.length, 0);
    });
  });

  describe('wire()', () => {
    it('wires all dependencies', () => {
      const deps = {
        getLastFinalizedSlot: () => 100,
        sendTo: createMockFn(),
        getPeerSlots: () => new Map(),
        publicKey: 'node-key-123',
      };

      detector.wire(deps);

      assert.strictEqual(detector._getLastFinalizedSlot(), 100);
      assert.strictEqual(detector._sendTo, deps.sendTo);
      assert.strictEqual(detector._publicKey, 'node-key-123');
    });

    it('ignores undefined values', () => {
      const original = detector._getLastFinalizedSlot;
      detector.wire({});
      assert.strictEqual(detector._getLastFinalizedSlot, original);
    });
  });

  describe('checkForForks()', () => {
    it('does not detect fork with single hash per slot', () => {
      const events = [];
      detector.on('fork:detected', (e) => events.push(e));

      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaa' }], 50);

      assert.strictEqual(events.length, 0);
      assert.strictEqual(detector.stats.forksDetected, 0);
    });

    it('detects fork when two peers report different hashes for same slot', () => {
      const events = [];
      detector.on('fork:detected', (e) => events.push(e));

      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaa' }], 50);
      detector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbb' }], 60);

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].slot, 100);
      assert.strictEqual(events[0].branches, 2);
      assert.strictEqual(detector.stats.forksDetected, 1);
    });

    it('does not double-detect on same fork', () => {
      const events = [];
      detector.on('fork:detected', (e) => events.push(e));

      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaa' }], 50);
      detector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbb' }], 60);
      // Third peer with yet another hash — fork already detected
      detector.checkForForks('peer-c', [{ slot: 100, hash: 'hash-ccc' }], 70);

      assert.strictEqual(events.length, 1); // Only first detection fires
    });

    it('accumulates E-Score for same hash from different peers', () => {
      detector.checkForForks('peer-a1', [{ slot: 50, hash: 'hash-aaa' }], 40);
      detector.checkForForks('peer-a2', [{ slot: 50, hash: 'hash-aaa' }], 30);

      const slotForks = detector._forkState.forkHashes.get(50);
      const hashInfo = slotForks.get('hash-aaa');

      assert.strictEqual(hashInfo.peers.size, 2);
      assert.strictEqual(hashInfo.totalEScore, 70);
    });

    it('does not double-count same peer', () => {
      detector.checkForForks('peer-a', [{ slot: 50, hash: 'hash-aaa' }], 40);
      detector.checkForForks('peer-a', [{ slot: 50, hash: 'hash-aaa' }], 40);

      const slotForks = detector._forkState.forkHashes.get(50);
      const hashInfo = slotForks.get('hash-aaa');

      assert.strictEqual(hashInfo.peers.size, 1);
      assert.strictEqual(hashInfo.totalEScore, 40); // Not 80
    });

    it('skips hashes with null/undefined', () => {
      const events = [];
      detector.on('fork:detected', (e) => events.push(e));

      detector.checkForForks('peer-a', [{ slot: 100, hash: null }], 50);
      detector.checkForForks('peer-b', [{ slot: 100, hash: undefined }], 60);

      assert.strictEqual(events.length, 0);
    });

    it('handles multiple slots in single call', () => {
      detector.checkForForks('peer-a', [
        { slot: 100, hash: 'hash-100-a' },
        { slot: 101, hash: 'hash-101-a' },
      ], 50);

      assert.strictEqual(detector._forkState.forkHashes.has(100), true);
      assert.strictEqual(detector._forkState.forkHashes.has(101), true);
    });
  });

  describe('heaviest branch calculation', () => {
    it('identifies heaviest branch by total E-Score', () => {
      const events = [];
      detector.on('fork:detected', (e) => events.push(e));

      // Branch A: 50 + 40 = 90 E-Score
      detector.checkForForks('peer-a1', [{ slot: 100, hash: 'hash-aaaa' }], 50);
      detector.checkForForks('peer-a2', [{ slot: 100, hash: 'hash-aaaa' }], 40);

      // Branch B: 60 E-Score
      detector.checkForForks('peer-b1', [{ slot: 100, hash: 'hash-bbbb' }], 60);

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].heaviestBranch, 'hash-aaaa'.slice(0, 16));
    });

    it('recommends STAY when we are on heaviest branch', () => {
      const events = [];
      detector.on('fork:detected', (e) => events.push(e));

      // Record our hash first
      detector.recordBlockHash(100, 'hash-aaaa');

      // Branch A is heavier (our branch)
      detector.checkForForks('peer-a1', [{ slot: 100, hash: 'hash-aaaa' }], 80);
      detector.checkForForks('peer-b1', [{ slot: 100, hash: 'hash-bbbb' }], 30);

      assert.strictEqual(events[0].onHeaviestBranch, true);
      assert.strictEqual(events[0].recommendation, 'STAY');
    });

    it('recommends REORG_NEEDED when we are NOT on heaviest branch', () => {
      const events = [];
      detector.on('fork:detected', (e) => events.push(e));

      // Record our hash as the weaker branch
      detector.recordBlockHash(100, 'hash-bbbb');

      // Branch A is heavier (not ours)
      detector.checkForForks('peer-a1', [{ slot: 100, hash: 'hash-aaaa' }], 80);
      detector.checkForForks('peer-b1', [{ slot: 100, hash: 'hash-bbbb' }], 30);

      assert.strictEqual(events[0].onHeaviestBranch, false);
      assert.strictEqual(events[0].recommendation, 'REORG_NEEDED');
    });
  });

  describe('fork resolution', () => {
    it('sends FORK_RESOLUTION_REQUEST to best peer', async () => {
      const sendTo = createMockFn();
      const peerSlots = new Map([
        ['peer-a', { eScore: 80 }],
        ['peer-b', { eScore: 60 }],
      ]);

      detector.wire({
        sendTo,
        getPeerSlots: () => peerSlots,
        publicKey: 'our-node-key-1234567890abcdef',
      });

      // Record our hash as weaker branch
      detector.recordBlockHash(100, 'hash-bbbb');

      // Trigger fork where we're NOT on heaviest
      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa' }], 80);
      detector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb' }], 30);

      // Wait for async _resolveFork
      await waitFor(() => {
        assert.ok(sendTo.mock.callCount > 0);
      });

      const [peerId, message] = sendTo.mock.calls[0];
      assert.strictEqual(peerId, 'peer-a'); // Highest E-Score peer on target branch
      assert.strictEqual(message.type, 'FORK_RESOLUTION_REQUEST');
      assert.strictEqual(message.forkSlot, 100);
      assert.strictEqual(message.targetHash, 'hash-aaaa');
    });

    it('does not start resolution when already in progress', async () => {
      const sendTo = createMockFn();
      const peerSlots = new Map([['peer-a', { eScore: 80 }]]);

      detector.wire({
        sendTo,
        getPeerSlots: () => peerSlots,
        publicKey: 'our-key-1234567890abcdef1234567890',
      });

      // Manually set resolution in progress
      detector._forkState.resolutionInProgress = true;

      detector.recordBlockHash(100, 'hash-bbbb');
      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa' }], 80);
      detector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb' }], 30);

      // Should NOT send resolution request
      assert.strictEqual(sendTo.mock.callCount, 0);
    });

    it('handles no peers on target branch', () => {
      const sendTo = createMockFn();

      detector.wire({
        sendTo,
        getPeerSlots: () => new Map(), // No peer info
        publicKey: 'our-key-1234567890abcdef1234567890',
      });

      detector.recordBlockHash(100, 'hash-bbbb');
      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa' }], 80);
      detector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb' }], 30);

      // Resolution should not send (no peer info found)
      // resolutionInProgress should be reset
      assert.strictEqual(detector._forkState.resolutionInProgress, false);
    });

    it('emits fork:resolution_started on successful send', async () => {
      const sendTo = createMockFn();
      const peerSlots = new Map([['peer-a', { eScore: 80 }]]);
      const events = [];

      detector.wire({
        sendTo,
        getPeerSlots: () => peerSlots,
        publicKey: 'our-key-1234567890abcdef1234567890',
      });
      detector.on('fork:resolution_started', (e) => events.push(e));

      detector.recordBlockHash(100, 'hash-bbbb');
      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa' }], 80);
      detector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb' }], 30);

      await waitFor(() => {
        assert.ok(events.length > 0);
      });

      assert.strictEqual(events[0].forkSlot, 100);
    });
  });

  describe('markForkResolved()', () => {
    it('clears fork state and increments resolved count', () => {
      // Create a fork
      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaa' }], 50);
      detector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbb' }], 60);

      assert.strictEqual(detector.stats.forksDetected, 1);

      const events = [];
      detector.on('fork:resolved', (e) => events.push(e));

      detector.markForkResolved();

      assert.strictEqual(detector.stats.forksResolved, 1);
      assert.strictEqual(detector._forkState.detected, false);
      assert.strictEqual(detector._forkState.forkSlot, null);
      assert.strictEqual(detector._forkState.resolutionInProgress, false);
      assert.strictEqual(events[0].forkSlot, 100);
    });

    it('is safe to call when no fork is detected', () => {
      // Should not throw and should not increment resolved
      detector.markForkResolved();

      assert.strictEqual(detector.stats.forksResolved, 0);
    });
  });

  describe('recordBlockHash / getRecentBlockHashes', () => {
    it('records block hash for a slot', () => {
      detector.recordBlockHash(100, 'hash-100');

      assert.strictEqual(detector._slotHashes.get(100)?.hash, 'hash-100');
      assert.ok(detector._slotHashes.get(100)?.confirmedAt !== undefined);
    });

    it('returns recent block hashes', () => {
      detector.wire({ getLastFinalizedSlot: () => 102 });

      detector.recordBlockHash(100, 'hash-100');
      detector.recordBlockHash(101, 'hash-101');
      detector.recordBlockHash(102, 'hash-102');

      const recent = detector.getRecentBlockHashes(3);

      assert.strictEqual(recent.length, 3);
      assert.deepStrictEqual(recent[0], { slot: 102, hash: 'hash-102' });
      assert.deepStrictEqual(recent[1], { slot: 101, hash: 'hash-101' });
      assert.deepStrictEqual(recent[2], { slot: 100, hash: 'hash-100' });
    });

    it('returns empty when no hashes recorded', () => {
      detector.wire({ getLastFinalizedSlot: () => 100 });

      const recent = detector.getRecentBlockHashes(5);
      assert.strictEqual(recent.length, 0);
    });

    it('skips slots without recorded hashes', () => {
      detector.wire({ getLastFinalizedSlot: () => 102 });

      detector.recordBlockHash(100, 'hash-100');
      // slot 101 not recorded
      detector.recordBlockHash(102, 'hash-102');

      const recent = detector.getRecentBlockHashes(3);

      assert.strictEqual(recent.length, 2);
    });
  });

  describe('_cleanupForkData()', () => {
    it('removes fork data older than 100 slots', () => {
      detector.wire({ getLastFinalizedSlot: () => 150 });

      // Add old data
      for (let i = 0; i < 200; i++) {
        detector._slotHashes.set(i, { hash: `hash-${i}` });
        detector._forkState.forkHashes.set(i, new Map());
      }

      detector._cleanupForkData();

      // Slots < 50 (150 - 100) should be removed
      assert.strictEqual(detector._slotHashes.has(49), false);
      assert.strictEqual(detector._slotHashes.has(50), true);
      assert.strictEqual(detector._forkState.forkHashes.has(49), false);
      assert.strictEqual(detector._forkState.forkHashes.has(50), true);
    });

    it('is called automatically during checkForForks', () => {
      detector.wire({ getLastFinalizedSlot: () => 200 });

      // Add old slot hash
      detector._slotHashes.set(50, { hash: 'old' });

      // checkForForks triggers cleanup
      detector.checkForForks('peer', [{ slot: 200, hash: 'new' }], 50);

      assert.strictEqual(detector._slotHashes.has(50), false);
    });
  });

  describe('getForkStatus()', () => {
    it('returns complete status with fork details', () => {
      detector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa1111222233334444555566667777' }], 50);
      detector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb1111222233334444555566667777' }], 60);

      const status = detector.getForkStatus();

      assert.strictEqual(status.detected, true);
      assert.strictEqual(status.forkSlot, 100);
      assert.strictEqual(status.branches.length, 2);
      assert.strictEqual(status.stats.forksDetected, 1);

      // Each branch should have hash, peers, totalEScore
      for (const branch of status.branches) {
        assert.ok(branch.hash !== undefined);
        assert.ok(branch.peers !== undefined);
        assert.ok(branch.totalEScore !== undefined);
      }
    });

    it('returns empty branches when no fork', () => {
      const status = detector.getForkStatus();

      assert.strictEqual(status.detected, false);
      assert.strictEqual(status.branches.length, 0);
    });
  });

  describe('stats', () => {
    it('returns a copy of stats', () => {
      const s1 = detector.stats;
      const s2 = detector.stats;

      assert.deepStrictEqual(s1, s2);
      assert.notStrictEqual(s1, s2);
    });
  });
});
