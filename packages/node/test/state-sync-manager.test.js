/**
 * StateSyncManager Tests
 *
 * PHASE 2: DECENTRALIZE
 *
 * Tests state synchronization between nodes:
 * peer tracking, sync detection, message handling,
 * fork resolution, and chain integrity validation.
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';
import { StateSyncManager } from '../src/network/state-sync-manager.js';

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

describe('StateSyncManager', () => {
  let sync;

  beforeEach(() => {
    sync = new StateSyncManager();
  });

  describe('constructor', () => {
    it('starts with clean sync state', () => {
      assert.strictEqual(sync.syncState.lastSyncSlot, 0);
      assert.strictEqual(sync.syncState.syncing, false);
      assert.strictEqual(sync.syncState.behindBy, 0);
      assert.strictEqual(sync.syncState.syncInProgress, false);
    });

    it('starts with empty peer slots', () => {
      assert.strictEqual(sync.peerSlots.size, 0);
    });
  });

  describe('wire()', () => {
    it('wires all dependencies', () => {
      const mockGetBlocks = createMockFn(async () => []);
      const mockStoreBlock = createMockFn(async () => ({}));
      const mockSendTo = createMockFn();

      sync.wire({
        getLastFinalizedSlot: () => 50,
        getCurrentSlot: () => 55,
        sendTo: mockSendTo,
        publicKey: 'node-key',
        getBlocks: mockGetBlocks,
        storeBlock: mockStoreBlock,
      });

      assert.strictEqual(sync._getLastFinalizedSlot(), 50);
      assert.strictEqual(sync._getCurrentSlot(), 55);
      assert.strictEqual(sync._sendTo, mockSendTo);
      assert.strictEqual(sync._publicKey, 'node-key');
      assert.strictEqual(sync._getBlocks, mockGetBlocks);
      assert.strictEqual(sync._storeBlock, mockStoreBlock);
    });
  });

  describe('updatePeer()', () => {
    it('stores peer info from heartbeat', () => {
      sync.updatePeer('peer-1', {
        finalizedSlot: 100,
        finalizedHash: 'hash-100',
        slot: 105,
        state: 'PARTICIPATING',
        eScore: 75,
      });

      const peer = sync.peerSlots.get('peer-1');
      assert.strictEqual(peer.finalizedSlot, 100);
      assert.strictEqual(peer.finalizedHash, 'hash-100');
      assert.strictEqual(peer.slot, 105);
      assert.strictEqual(peer.state, 'PARTICIPATING');
      assert.strictEqual(peer.eScore, 75);
      assert.ok(peer.lastSeen !== undefined);
    });

    it('updates existing peer', () => {
      sync.updatePeer('peer-1', { finalizedSlot: 50, eScore: 60 });
      sync.updatePeer('peer-1', { finalizedSlot: 100, eScore: 70 });

      const peer = sync.peerSlots.get('peer-1');
      assert.strictEqual(peer.finalizedSlot, 100);
      assert.strictEqual(peer.eScore, 70);
    });

    it('defaults missing fields', () => {
      sync.updatePeer('peer-1', {});

      const peer = sync.peerSlots.get('peer-1');
      assert.strictEqual(peer.finalizedSlot, 0);
      assert.strictEqual(peer.finalizedHash, null);
      assert.strictEqual(peer.slot, 0);
      assert.strictEqual(peer.state, 'UNKNOWN');
      assert.strictEqual(peer.eScore, 50);
    });
  });

  describe('checkStateSync()', () => {
    it('returns needsSync=false when not behind', () => {
      sync.wire({ getLastFinalizedSlot: () => 100 });
      sync.updatePeer('peer-1', { finalizedSlot: 105 }); // Only 5 behind (< 10)

      const result = sync.checkStateSync();

      assert.strictEqual(result.needsSync, false);
      assert.strictEqual(result.behindBy, 5);
    });

    it('returns needsSync=true when >10 slots behind', () => {
      const sendTo = createMockFn();
      sync.wire({
        getLastFinalizedSlot: () => 0,
        sendTo,
        publicKey: 'our-key-1234567890abcdef1234567890',
      });
      sync.updatePeer('peer-1', { finalizedSlot: 100 });

      const events = [];
      sync.on('sync:needed', (e) => events.push(e));

      const result = sync.checkStateSync();

      assert.strictEqual(result.needsSync, true);
      assert.strictEqual(result.behindBy, 100);
      assert.strictEqual(result.bestPeer, 'peer-1');
      assert.strictEqual(events.length, 1);
    });

    it('picks peer with highest finalized slot', () => {
      const sendTo = createMockFn();
      sync.wire({
        getLastFinalizedSlot: () => 0,
        sendTo,
        publicKey: 'our-key-1234567890abcdef1234567890',
      });

      sync.updatePeer('peer-a', { finalizedSlot: 50 });
      sync.updatePeer('peer-b', { finalizedSlot: 200 });
      sync.updatePeer('peer-c', { finalizedSlot: 150 });

      const result = sync.checkStateSync();

      assert.strictEqual(result.bestPeer, 'peer-b');
      assert.strictEqual(result.behindBy, 200);
    });

    it('ignores stale peers (>60s old)', () => {
      sync.wire({ getLastFinalizedSlot: () => 0 });

      sync.updatePeer('peer-1', { finalizedSlot: 200 });

      // Make peer stale
      const peer = sync.peerSlots.get('peer-1');
      peer.lastSeen = Date.now() - 70000; // 70 seconds ago

      const result = sync.checkStateSync();

      assert.strictEqual(result.needsSync, false);
      assert.strictEqual(result.behindBy, 0);
    });

    it('sends STATE_REQUEST to best peer', async () => {
      const sendTo = createMockFn();
      sync.wire({
        getLastFinalizedSlot: () => 0,
        sendTo,
        publicKey: 'our-key-1234567890abcdef1234567890',
      });
      sync.updatePeer('peer-1', { finalizedSlot: 100 });

      sync.checkStateSync();

      // Wait for async request
      await waitFor(() => {
        assert.ok(sendTo.mock.calls.length > 0);
      });

      const [peerId, message] = sendTo.mock.calls[0];
      assert.strictEqual(peerId, 'peer-1');
      assert.strictEqual(message.type, 'STATE_REQUEST');
      assert.strictEqual(message.fromSlot, 0);
    });

    it('detects sync completion when caught up', () => {
      sync.wire({ getLastFinalizedSlot: () => 100 });
      sync.updatePeer('peer-1', { finalizedSlot: 105 });

      // Simulate sync was in progress
      sync._syncState.syncInProgress = true;

      const events = [];
      sync.on('sync:complete', (e) => events.push(e));

      const result = sync.checkStateSync();

      assert.strictEqual(result.needsSync, false);
      assert.strictEqual(result.justCompleted, true);
      assert.strictEqual(events.length, 1);
    });

    it('throttles sync requests (5s cooldown)', async () => {
      const sendTo = createMockFn();
      sync.wire({
        getLastFinalizedSlot: () => 0,
        sendTo,
        publicKey: 'our-key-1234567890abcdef1234567890',
      });
      sync.updatePeer('peer-1', { finalizedSlot: 100 });

      // First call
      sync.checkStateSync();

      // Reset syncInProgress so second call triggers
      sync._syncState.syncInProgress = false;

      // Second call (within 5s) should be throttled
      sync.checkStateSync();

      await waitFor(() => {
        // Only 1 STATE_REQUEST sent (throttled)
        assert.strictEqual(sendTo.mock.calls.length, 1);
      });
    });
  });

  describe('handleStateRequest()', () => {
    it('responds with blocks from store', async () => {
      const sendTo = createMockFn();
      const mockBlocks = [
        { slot: 10, hash: 'hash-10' },
        { slot: 11, hash: 'hash-11' },
      ];

      sync.wire({
        getLastFinalizedSlot: () => 20,
        getCurrentSlot: () => 25,
        sendTo,
        getBlocks: createMockFn(async () => mockBlocks),
      });

      await sync.handleStateRequest({ fromSlot: 10 }, 'peer-1');

      assert.ok(sendTo.mock.calls.length > 0);
      const [peerId, msg] = sendTo.mock.calls[0];
      assert.strictEqual(peerId, 'peer-1');
      assert.strictEqual(msg.type, 'STATE_RESPONSE');
      assert.strictEqual(msg.fromSlot, 10);
      assert.strictEqual(msg.finalizedSlot, 20);
      assert.strictEqual(msg.currentSlot, 25);
      assert.deepStrictEqual(msg.blocks, mockBlocks);
    });
  });

  describe('handleStateResponse()', () => {
    it('stores received blocks', async () => {
      const storeBlock = createMockFn(async () => ({}));
      sync.wire({
        getLastFinalizedSlot: () => 5,
        storeBlock,
      });

      const events = [];
      sync.on('sync:blocks_received', (e) => events.push(e));

      await sync.handleStateResponse({
        finalizedSlot: 10,
        blocks: [
          { slot: 6, hash: 'h6' },
          { slot: 7, hash: 'h7' },
          { slot: 8, hash: 'h8' },
        ],
      }, 'peer-1');

      assert.strictEqual(storeBlock.mock.calls.length, 3);
      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].blocksReceived, 3);
      assert.strictEqual(events[0].blocksProcessed, 3);
    });

    it('skips blocks at or below our slot', async () => {
      const storeBlock = createMockFn(async () => ({}));
      sync.wire({
        getLastFinalizedSlot: () => 5,
        storeBlock,
      });

      await sync.handleStateResponse({
        finalizedSlot: 10,
        blocks: [
          { slot: 3, hash: 'old' },
          { slot: 5, hash: 'current' },
          { slot: 6, hash: 'new' },
        ],
      }, 'peer-1');

      // Only slot 6 should be stored (> ourSlot of 5)
      assert.strictEqual(storeBlock.mock.calls.length, 1);
      assert.deepStrictEqual(storeBlock.mock.calls[0][0], { slot: 6, hash: 'new' });
    });

    it('sends BLOCK_REQUEST when peer is ahead but no blocks', async () => {
      const sendTo = createMockFn();
      sync.wire({
        getLastFinalizedSlot: () => 5,
        sendTo,
        publicKey: 'our-key-1234567890abcdef1234567890',
      });

      await sync.handleStateResponse({
        finalizedSlot: 100,
        blocks: null, // No blocks in response
      }, 'peer-1');

      assert.ok(sendTo.mock.calls.length > 0);
      const [peerId, msg] = sendTo.mock.calls[0];
      assert.strictEqual(peerId, 'peer-1');
      assert.strictEqual(msg.type, 'BLOCK_REQUEST');
      assert.strictEqual(msg.fromSlot, 6);
      assert.strictEqual(msg.toSlot, 100);
    });

    it('does nothing when we are caught up', async () => {
      const storeBlock = createMockFn();
      const sendTo = createMockFn();
      sync.wire({
        getLastFinalizedSlot: () => 100,
        storeBlock,
        sendTo,
      });

      await sync.handleStateResponse({
        finalizedSlot: 90, // They're behind us
        blocks: [],
      }, 'peer-1');

      assert.strictEqual(storeBlock.mock.calls.length, 0);
      assert.strictEqual(sendTo.mock.calls.length, 0);
    });

    it('clears syncInProgress on response', async () => {
      sync.wire({ getLastFinalizedSlot: () => 100 });
      sync._syncState.syncInProgress = true;

      await sync.handleStateResponse({ finalizedSlot: 90 }, 'peer-1');

      assert.strictEqual(sync._syncState.syncInProgress, false);
    });

    it('handles storeBlock failures gracefully', async () => {
      const storeBlock = createMockFn(async () => { throw new Error('DB down'); });
      sync.wire({
        getLastFinalizedSlot: () => 5,
        storeBlock,
      });

      // Should not throw
      await sync.handleStateResponse({
        finalizedSlot: 10,
        blocks: [{ slot: 6, hash: 'h6' }, { slot: 7, hash: 'h7' }],
      }, 'peer-1');
    });
  });

  describe('handleBlockRequest()', () => {
    it('responds with blocks for requested range', async () => {
      const sendTo = createMockFn();
      const mockBlocks = [{ slot: 10 }, { slot: 11 }];

      sync.wire({
        getLastFinalizedSlot: () => 20,
        sendTo,
        getBlocks: createMockFn(async () => mockBlocks),
      });

      const events = [];
      sync.on('sync:blocks_sent', (e) => events.push(e));

      await sync.handleBlockRequest({ fromSlot: 10, toSlot: 15 }, 'peer-1');

      assert.ok(sendTo.mock.calls.length > 0);
      const [peerId, msg] = sendTo.mock.calls[0];
      assert.strictEqual(peerId, 'peer-1');
      assert.strictEqual(msg.type, 'STATE_RESPONSE');
      assert.strictEqual(msg.fromSlot, 10);
      assert.strictEqual(msg.finalizedSlot, 20);
      assert.deepStrictEqual(msg.blocks, mockBlocks);

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].fromSlot, 10);
      assert.strictEqual(events[0].toSlot, 15);
      assert.strictEqual(events[0].blocksSent, 2);
    });
  });

  describe('handleForkResolutionRequest()', () => {
    it('responds with blocks when we have target branch', async () => {
      const sendTo = createMockFn();
      const mockBlocks = [{ slot: 100 }, { slot: 101 }];

      sync.wire({
        getLastFinalizedSlot: () => 110,
        sendTo,
        getBlocks: createMockFn(async () => mockBlocks),
      });

      const events = [];
      sync.on('fork:resolution_provided', (e) => events.push(e));

      const getSlotHash = (slot) => slot === 100 ? 'target-hash' : null;

      await sync.handleForkResolutionRequest(
        { forkSlot: 100, targetHash: 'target-hash' },
        'peer-1',
        getSlotHash,
      );

      assert.ok(sendTo.mock.calls.length > 0);
      const [peerId, msg] = sendTo.mock.calls[0];
      assert.strictEqual(peerId, 'peer-1');
      assert.strictEqual(msg.type, 'FORK_RESOLUTION_RESPONSE');
      assert.strictEqual(msg.forkSlot, 100);
      assert.strictEqual(msg.success, true);
      assert.deepStrictEqual(msg.blocks, mockBlocks);

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].blocksProvided, 2);
    });

    it('responds with failure when we do NOT have target branch', async () => {
      const sendTo = createMockFn();
      sync.wire({ sendTo });

      const getSlotHash = () => 'different-hash';

      await sync.handleForkResolutionRequest(
        { forkSlot: 100, targetHash: 'target-hash' },
        'peer-1',
        getSlotHash,
      );

      assert.ok(sendTo.mock.calls.length > 0);
      const [peerId, msg] = sendTo.mock.calls[0];
      assert.strictEqual(peerId, 'peer-1');
      assert.strictEqual(msg.type, 'FORK_RESOLUTION_RESPONSE');
      assert.strictEqual(msg.forkSlot, 100);
      assert.strictEqual(msg.success, false);
      assert.strictEqual(msg.reason, 'BRANCH_NOT_AVAILABLE');
    });
  });

  describe('handleForkResolutionResponse()', () => {
    it('stores validated blocks and marks fork resolved', async () => {
      const storeBlock = createMockFn(async () => ({}));
      const markForkResolved = createMockFn();
      sync.wire({ storeBlock });

      const events = [];
      sync.on('fork:reorg_complete', (e) => events.push(e));

      await sync.handleForkResolutionResponse(
        {
          forkSlot: 100,
          success: true,
          blocks: [
            { slot: 100, hash: 'hash-100', prev_hash: 'hash-99' },
            { slot: 101, hash: 'hash-101', prev_hash: 'hash-100' },
          ],
        },
        'peer-1',
        markForkResolved,
      );

      assert.strictEqual(storeBlock.mock.calls.length, 2);
      assert.ok(markForkResolved.mock.calls.length > 0);
      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].blocksApplied, 2);
    });

    it('handles failed resolution (success=false)', async () => {
      const events = [];
      sync.on('fork:resolution_failed', (e) => events.push(e));

      await sync.handleForkResolutionResponse(
        { forkSlot: 100, success: false, reason: 'BRANCH_NOT_AVAILABLE' },
        'peer-1',
        createMockFn(),
      );

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].reason, 'BRANCH_NOT_AVAILABLE');
    });

    it('rejects blocks with broken chain integrity', async () => {
      const storeBlock = createMockFn(async () => ({}));
      const markForkResolved = createMockFn();
      sync.wire({ storeBlock });

      const events = [];
      sync.on('fork:resolution_failed', (e) => events.push(e));

      await sync.handleForkResolutionResponse(
        {
          forkSlot: 100,
          success: true,
          blocks: [
            { slot: 100, hash: 'hash-100', prev_hash: 'hash-99' },
            { slot: 101, hash: 'hash-101', prev_hash: 'WRONG-HASH' }, // Broken chain!
          ],
        },
        'peer-1',
        markForkResolved,
      );

      assert.strictEqual(storeBlock.mock.calls.length, 0);
      assert.strictEqual(markForkResolved.mock.calls.length, 0);
      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].reason, 'INVALID_CHAIN');
    });

    it('accepts single block (no chain to validate)', async () => {
      const storeBlock = createMockFn(async () => ({}));
      const markForkResolved = createMockFn();
      sync.wire({ storeBlock });

      await sync.handleForkResolutionResponse(
        {
          forkSlot: 100,
          success: true,
          blocks: [{ slot: 100, hash: 'hash-100', prev_hash: 'hash-99' }],
        },
        'peer-1',
        markForkResolved,
      );

      assert.strictEqual(storeBlock.mock.calls.length, 1);
      assert.ok(markForkResolved.mock.calls.length > 0);
    });

    it('handles empty blocks array', async () => {
      const markForkResolved = createMockFn();

      await sync.handleForkResolutionResponse(
        { forkSlot: 100, success: true, blocks: [] },
        'peer-1',
        markForkResolved,
      );

      assert.ok(markForkResolved.mock.calls.length > 0);
    });

    it('handles null blocks', async () => {
      const markForkResolved = createMockFn();

      await sync.handleForkResolutionResponse(
        { forkSlot: 100, success: true, blocks: null },
        'peer-1',
        markForkResolved,
      );

      assert.ok(markForkResolved.mock.calls.length > 0);
    });
  });

  describe('handleValidatorUpdate()', () => {
    it('calls registerValidator on ADD action', async () => {
      const registerValidator = createMockFn();
      const removeValidator = createMockFn();

      await sync.handleValidatorUpdate(
        { validator: { publicKey: 'val-1', eScore: 50 }, action: 'ADD' },
        registerValidator,
        removeValidator,
      );

      assert.deepStrictEqual(registerValidator.mock.calls[0][0], { publicKey: 'val-1', eScore: 50 });
      assert.strictEqual(removeValidator.mock.calls.length, 0);
    });

    it('calls removeValidator on REMOVE action', async () => {
      const registerValidator = createMockFn();
      const removeValidator = createMockFn();

      await sync.handleValidatorUpdate(
        { validator: { publicKey: 'val-1' }, action: 'REMOVE' },
        registerValidator,
        removeValidator,
      );

      assert.strictEqual(registerValidator.mock.calls.length, 0);
      assert.deepStrictEqual(removeValidator.mock.calls[0][0], 'val-1');
    });

    it('ignores unknown actions', async () => {
      const registerValidator = createMockFn();
      const removeValidator = createMockFn();

      await sync.handleValidatorUpdate(
        { validator: { publicKey: 'val-1' }, action: 'UNKNOWN' },
        registerValidator,
        removeValidator,
      );

      assert.strictEqual(registerValidator.mock.calls.length, 0);
      assert.strictEqual(removeValidator.mock.calls.length, 0);
    });
  });
});
