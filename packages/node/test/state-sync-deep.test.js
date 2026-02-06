/**
 * Deep tests for StateSyncManager
 *
 * Tests state synchronization logic, peer tracking, fork resolution,
 * and validator updates.
 *
 * Target: ~23 tests covering all methods and edge cases
 *
 * @module @cynic/node/test/state-sync-deep.test
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { StateSyncManager } from '../src/network/state-sync-manager.js';

describe('StateSyncManager', () => {
  describe('constructor', () => {
    it('should initialize with correct default state', () => {
      const manager = new StateSyncManager();

      assert.deepEqual(manager.syncState, {
        lastSyncSlot: 0,
        syncing: false,
        behindBy: 0,
        syncInProgress: false,
        lastSyncAttempt: 0,
      });
    });

    it('should initialize with empty peerSlots Map', () => {
      const manager = new StateSyncManager();

      assert.equal(manager.peerSlots.size, 0);
      assert.ok(manager.peerSlots instanceof Map);
    });
  });

  describe('wire()', () => {
    it('should set all dependencies when provided', () => {
      const manager = new StateSyncManager();

      const mockGetLastFinalizedSlot = () => 100;
      const mockGetCurrentSlot = () => 105;
      const mockSendTo = async () => {};
      const mockPublicKey = 'test-public-key';
      const mockGetBlocks = async () => [];
      const mockStoreBlock = async () => null;

      manager.wire({
        getLastFinalizedSlot: mockGetLastFinalizedSlot,
        getCurrentSlot: mockGetCurrentSlot,
        sendTo: mockSendTo,
        publicKey: mockPublicKey,
        getBlocks: mockGetBlocks,
        storeBlock: mockStoreBlock,
      });

      // Verify dependencies are set by checking side effects
      assert.equal(manager._getLastFinalizedSlot(), 100);
      assert.equal(manager._getCurrentSlot(), 105);
      assert.equal(manager._publicKey, 'test-public-key');
    });

    it('should accept partial dependencies', () => {
      const manager = new StateSyncManager();

      const mockGetLastFinalizedSlot = () => 50;

      manager.wire({
        getLastFinalizedSlot: mockGetLastFinalizedSlot,
      });

      // Should set the provided dependency
      assert.equal(manager._getLastFinalizedSlot(), 50);

      // Should keep defaults for others
      assert.equal(manager._getCurrentSlot(), 0);
    });
  });

  describe('updatePeer()', () => {
    let manager;

    beforeEach(() => {
      manager = new StateSyncManager();
    });

    it('should store peer info correctly', () => {
      const peerId = 'peer-1';
      const heartbeat = {
        finalizedSlot: 100,
        finalizedHash: 'hash-100',
        slot: 105,
        state: 'ACTIVE',
        eScore: 75,
      };

      manager.updatePeer(peerId, heartbeat);

      const peerInfo = manager.peerSlots.get(peerId);
      assert.equal(peerInfo.finalizedSlot, 100);
      assert.equal(peerInfo.finalizedHash, 'hash-100');
      assert.equal(peerInfo.slot, 105);
      assert.equal(peerInfo.state, 'ACTIVE');
      assert.equal(peerInfo.eScore, 75);
      assert.ok(peerInfo.lastSeen > 0);
    });

    it('should update existing peer info', () => {
      const peerId = 'peer-1';

      manager.updatePeer(peerId, {
        finalizedSlot: 100,
        finalizedHash: 'hash-100',
        slot: 105,
        state: 'ACTIVE',
        eScore: 70,
      });

      const firstLastSeen = manager.peerSlots.get(peerId).lastSeen;

      // Wait a bit and update
      manager.updatePeer(peerId, {
        finalizedSlot: 110,
        finalizedHash: 'hash-110',
        slot: 115,
        state: 'ACTIVE',
        eScore: 80,
      });

      const peerInfo = manager.peerSlots.get(peerId);
      assert.equal(peerInfo.finalizedSlot, 110);
      assert.equal(peerInfo.finalizedHash, 'hash-110');
      assert.ok(peerInfo.lastSeen >= firstLastSeen);
    });

    it('should set defaults for missing fields', () => {
      const peerId = 'peer-2';
      const heartbeat = {}; // Empty heartbeat

      manager.updatePeer(peerId, heartbeat);

      const peerInfo = manager.peerSlots.get(peerId);
      assert.equal(peerInfo.finalizedSlot, 0);
      assert.equal(peerInfo.finalizedHash, null);
      assert.equal(peerInfo.slot, 0);
      assert.equal(peerInfo.state, 'UNKNOWN');
      assert.equal(peerInfo.eScore, 50);
    });
  });

  describe('checkStateSync() - not behind', () => {
    let manager;

    beforeEach(() => {
      manager = new StateSyncManager();
      manager.wire({
        getLastFinalizedSlot: () => 100,
      });
    });

    it('should return needsSync false when no peers', () => {
      const result = manager.checkStateSync();

      assert.equal(result.needsSync, false);
      assert.equal(result.bestPeer, null);
      assert.equal(result.behindBy, 0);
    });

    it('should return needsSync false when peers are behind or equal', () => {
      manager.updatePeer('peer-1', { finalizedSlot: 90 });
      manager.updatePeer('peer-2', { finalizedSlot: 100 });

      const result = manager.checkStateSync();

      assert.equal(result.needsSync, false);
      assert.equal(result.bestPeer, null);
      assert.equal(result.behindBy, 0);
    });
  });

  describe('checkStateSync() - behind', () => {
    let manager;
    let sentMessages;

    beforeEach(() => {
      manager = new StateSyncManager();
      sentMessages = [];

      manager.wire({
        getLastFinalizedSlot: () => 100,
        sendTo: async (peerId, msg) => {
          sentMessages.push({ peerId, msg });
        },
        publicKey: 'test-key',
      });
    });

    it('should detect when behind by >10 slots and trigger sync', async () => {
      manager.updatePeer('peer-1', { finalizedSlot: 120 });

      const events = [];
      manager.on('sync:needed', (data) => events.push(data));

      const result = manager.checkStateSync();

      // Wait for async _requestStateSync
      await new Promise((resolve) => setTimeout(resolve, 10));

      assert.equal(result.needsSync, true);
      assert.equal(result.bestPeer, 'peer-1');
      assert.equal(result.behindBy, 20);

      assert.equal(events.length, 1);
      assert.equal(events[0].behindBy, 20);
      assert.equal(events[0].bestPeer, 'peer-1');

      assert.equal(sentMessages.length, 1);
      assert.equal(sentMessages[0].msg.type, 'STATE_REQUEST');
      assert.equal(sentMessages[0].msg.fromSlot, 100);
    });

    it('should choose peer with highest finalized slot', () => {
      manager.updatePeer('peer-1', { finalizedSlot: 115 });
      manager.updatePeer('peer-2', { finalizedSlot: 125 });
      manager.updatePeer('peer-3', { finalizedSlot: 120 });

      const result = manager.checkStateSync();

      assert.equal(result.bestPeer, 'peer-2');
      assert.equal(result.behindBy, 25);
    });

    it('should ignore stale peers (lastSeen > 60s)', () => {
      const now = Date.now();

      manager.updatePeer('peer-1', { finalizedSlot: 130 });
      manager.updatePeer('peer-2', { finalizedSlot: 125 });

      // Manually set peer-1 as stale
      const peer1Info = manager.peerSlots.get('peer-1');
      peer1Info.lastSeen = now - 61000; // 61 seconds ago

      const result = manager.checkStateSync();

      assert.equal(result.bestPeer, 'peer-2');
      assert.equal(result.behindBy, 25);
    });
  });

  describe('checkStateSync() - sync complete', () => {
    let manager;

    beforeEach(() => {
      manager = new StateSyncManager();
      manager.wire({
        getLastFinalizedSlot: () => 100,
      });

      // Simulate sync in progress
      manager._syncState.syncInProgress = true;
    });

    it('should detect when sync is complete', () => {
      manager.updatePeer('peer-1', { finalizedSlot: 105 }); // Only 5 slots ahead

      const events = [];
      manager.on('sync:complete', (data) => events.push(data));

      const result = manager.checkStateSync();

      assert.equal(result.needsSync, false);
      assert.equal(result.justCompleted, true);
      assert.equal(result.behindBy, 5);

      assert.equal(events.length, 1);
      assert.equal(events[0].slot, 100);

      // syncInProgress should be cleared
      assert.equal(manager.syncState.syncInProgress, false);
    });

    it('should not emit sync:complete if sync was not in progress', () => {
      manager._syncState.syncInProgress = false;
      manager.updatePeer('peer-1', { finalizedSlot: 105 });

      const events = [];
      manager.on('sync:complete', (data) => events.push(data));

      const result = manager.checkStateSync();

      assert.equal(result.needsSync, false);
      assert.equal(result.justCompleted, undefined);
      assert.equal(events.length, 0);
    });
  });

  describe('handleStateRequest()', () => {
    let manager;
    let sentMessages;

    beforeEach(() => {
      manager = new StateSyncManager();
      sentMessages = [];

      manager.wire({
        getLastFinalizedSlot: () => 100,
        getCurrentSlot: () => 105,
        sendTo: async (peerId, msg) => {
          sentMessages.push({ peerId, msg });
        },
        getBlocks: async (from, to) => {
          return [
            { slot: from, hash: `hash-${from}` },
            { slot: to, hash: `hash-${to}` },
          ];
        },
      });
    });

    it('should respond with blocks from fromSlot to finalizedSlot', async () => {
      const message = { fromSlot: 80 };
      const peerId = 'peer-1';

      await manager.handleStateRequest(message, peerId);

      assert.equal(sentMessages.length, 1);
      assert.equal(sentMessages[0].peerId, 'peer-1');
      assert.equal(sentMessages[0].msg.type, 'STATE_RESPONSE');
      assert.equal(sentMessages[0].msg.fromSlot, 80);
      assert.equal(sentMessages[0].msg.currentSlot, 105);
      assert.equal(sentMessages[0].msg.finalizedSlot, 100);
      assert.equal(sentMessages[0].msg.blocks.length, 2);
      assert.equal(sentMessages[0].msg.blocks[0].slot, 80);
    });

    it('should call getBlocks with correct range', async () => {
      const calls = [];
      manager.wire({
        getBlocks: async (from, to) => {
          calls.push({ from, to });
          return [];
        },
      });

      const message = { fromSlot: 50 };
      await manager.handleStateRequest(message, 'peer-1');

      assert.equal(calls.length, 1);
      assert.equal(calls[0].from, 50);
      assert.equal(calls[0].to, 100);
    });
  });

  describe('handleStateResponse()', () => {
    let manager;
    let storedBlocks;
    let sentMessages;

    beforeEach(() => {
      manager = new StateSyncManager();
      storedBlocks = [];
      sentMessages = [];

      manager.wire({
        getLastFinalizedSlot: () => 100,
        storeBlock: async (block) => {
          storedBlocks.push(block);
          return block;
        },
        sendTo: async (peerId, msg) => {
          sentMessages.push({ peerId, msg });
        },
        publicKey: 'test-key',
      });
    });

    it('should store blocks when peer is ahead', async () => {
      const message = {
        finalizedSlot: 110,
        blocks: [
          { slot: 101, hash: 'hash-101' },
          { slot: 102, hash: 'hash-102' },
        ],
      };
      const peerId = 'peer-1';

      const events = [];
      manager.on('sync:blocks_received', (data) => events.push(data));

      await manager.handleStateResponse(message, peerId);

      assert.equal(storedBlocks.length, 2);
      assert.equal(storedBlocks[0].slot, 101);
      assert.equal(storedBlocks[1].slot, 102);

      assert.equal(events.length, 1);
      assert.equal(events[0].fromPeer, 'peer-1');
      assert.equal(events[0].blocksReceived, 2);
      assert.equal(events[0].blocksProcessed, 2);
    });

    it('should emit blocks_received event', async () => {
      const message = {
        finalizedSlot: 105,
        blocks: [{ slot: 101, hash: 'hash-101' }],
      };

      const events = [];
      manager.on('sync:blocks_received', (data) => events.push(data));

      await manager.handleStateResponse(message, 'peer-1');

      assert.equal(events.length, 1);
      assert.equal(events[0].blocksReceived, 1);
      assert.equal(events[0].blocksProcessed, 1);
    });

    it('should send BLOCK_REQUEST when peer ahead but no blocks', async () => {
      const message = {
        finalizedSlot: 120,
        blocks: [], // No blocks
      };
      const peerId = 'peer-1';

      await manager.handleStateResponse(message, peerId);

      assert.equal(sentMessages.length, 1);
      assert.equal(sentMessages[0].msg.type, 'BLOCK_REQUEST');
      assert.equal(sentMessages[0].msg.fromSlot, 101);
      assert.equal(sentMessages[0].msg.toSlot, 120);
    });

    it('should clear syncInProgress flag', async () => {
      manager._syncState.syncInProgress = true;

      const message = {
        finalizedSlot: 100,
        blocks: [],
      };

      await manager.handleStateResponse(message, 'peer-1');

      assert.equal(manager.syncState.syncInProgress, false);
    });
  });

  describe('handleBlockRequest()', () => {
    let manager;
    let sentMessages;

    beforeEach(() => {
      manager = new StateSyncManager();
      sentMessages = [];

      manager.wire({
        getLastFinalizedSlot: () => 100,
        sendTo: async (peerId, msg) => {
          sentMessages.push({ peerId, msg });
        },
        getBlocks: async (from, to) => {
          return [
            { slot: from, hash: `hash-${from}` },
            { slot: from + 1, hash: `hash-${from + 1}` },
          ];
        },
      });
    });

    it('should send blocks back to requester', async () => {
      const message = { fromSlot: 90, toSlot: 95 };
      const peerId = 'peer-1';

      await manager.handleBlockRequest(message, peerId);

      assert.equal(sentMessages.length, 1);
      assert.equal(sentMessages[0].peerId, 'peer-1');
      assert.equal(sentMessages[0].msg.type, 'STATE_RESPONSE');
      assert.equal(sentMessages[0].msg.fromSlot, 90);
      assert.equal(sentMessages[0].msg.finalizedSlot, 100);
      assert.equal(sentMessages[0].msg.blocks.length, 2);
    });

    it('should emit blocks_sent event', async () => {
      const message = { fromSlot: 85, toSlot: 95 };

      const events = [];
      manager.on('sync:blocks_sent', (data) => events.push(data));

      await manager.handleBlockRequest(message, 'peer-1');

      assert.equal(events.length, 1);
      assert.equal(events[0].toPeer, 'peer-1');
      assert.equal(events[0].fromSlot, 85);
      assert.equal(events[0].toSlot, 95);
      assert.equal(events[0].blocksSent, 2);
    });
  });

  describe('handleForkResolutionRequest()', () => {
    let manager;
    let sentMessages;

    beforeEach(() => {
      manager = new StateSyncManager();
      sentMessages = [];

      manager.wire({
        getLastFinalizedSlot: () => 100,
        sendTo: async (peerId, msg) => {
          sentMessages.push({ peerId, msg });
        },
        getBlocks: async (from, to) => {
          return [{ slot: from, hash: `hash-${from}` }];
        },
      });
    });

    it('should provide blocks when on target branch', async () => {
      const message = { forkSlot: 80, targetHash: 'hash-80' };
      const getSlotHash = (slot) => (slot === 80 ? 'hash-80' : 'hash-other');

      await manager.handleForkResolutionRequest(message, 'peer-1', getSlotHash);

      assert.equal(sentMessages.length, 1);
      assert.equal(sentMessages[0].msg.type, 'FORK_RESOLUTION_RESPONSE');
      assert.equal(sentMessages[0].msg.success, true);
      assert.equal(sentMessages[0].msg.forkSlot, 80);
      assert.ok(Array.isArray(sentMessages[0].msg.blocks));
    });

    it('should send failure when not on target branch', async () => {
      const message = { forkSlot: 80, targetHash: 'hash-80' };
      const getSlotHash = (slot) => 'hash-different'; // Different hash

      await manager.handleForkResolutionRequest(message, 'peer-1', getSlotHash);

      assert.equal(sentMessages.length, 1);
      assert.equal(sentMessages[0].msg.type, 'FORK_RESOLUTION_RESPONSE');
      assert.equal(sentMessages[0].msg.success, false);
      assert.equal(sentMessages[0].msg.reason, 'BRANCH_NOT_AVAILABLE');
    });
  });

  describe('handleForkResolutionResponse()', () => {
    let manager;
    let storedBlocks;
    let markForkResolvedCalled;

    beforeEach(() => {
      manager = new StateSyncManager();
      storedBlocks = [];
      markForkResolvedCalled = false;

      manager.wire({
        storeBlock: async (block) => {
          storedBlocks.push(block);
          return block;
        },
      });
    });

    it('should validate chain integrity and store blocks', async () => {
      const message = {
        forkSlot: 80,
        success: true,
        blocks: [
          { slot: 80, hash: 'hash-80', prev_hash: 'hash-79' },
          { slot: 81, hash: 'hash-81', prev_hash: 'hash-80' },
          { slot: 82, hash: 'hash-82', prev_hash: 'hash-81' },
        ],
      };

      const markForkResolved = () => {
        markForkResolvedCalled = true;
      };

      const events = [];
      manager.on('fork:reorg_complete', (data) => events.push(data));

      await manager.handleForkResolutionResponse(message, 'peer-1', markForkResolved);

      assert.equal(storedBlocks.length, 3);
      assert.equal(markForkResolvedCalled, true);
      assert.equal(events.length, 1);
      assert.equal(events[0].forkSlot, 80);
      assert.equal(events[0].blocksApplied, 3);
    });

    it('should reject chain with broken prev_hash links', async () => {
      const message = {
        forkSlot: 80,
        success: true,
        blocks: [
          { slot: 80, hash: 'hash-80', prev_hash: 'hash-79' },
          { slot: 81, hash: 'hash-81', prev_hash: 'hash-WRONG' }, // Broken link
          { slot: 82, hash: 'hash-82', prev_hash: 'hash-81' },
        ],
      };

      const events = [];
      manager.on('fork:resolution_failed', (data) => events.push(data));
      manager.on('fork:reorg_complete', (data) => events.push(data));

      await manager.handleForkResolutionResponse(message, 'peer-1', () => {});

      assert.equal(storedBlocks.length, 0); // No blocks stored
      assert.equal(events.length, 1);
      assert.equal(events[0].reason, 'INVALID_CHAIN');
    });

    it('should emit resolution_failed when success is false', async () => {
      const message = {
        forkSlot: 80,
        success: false,
        reason: 'BRANCH_NOT_AVAILABLE',
      };

      const events = [];
      manager.on('fork:resolution_failed', (data) => events.push(data));

      await manager.handleForkResolutionResponse(message, 'peer-1', () => {});

      assert.equal(events.length, 1);
      assert.equal(events[0].forkSlot, 80);
      assert.equal(events[0].reason, 'BRANCH_NOT_AVAILABLE');
    });
  });

  describe('handleValidatorUpdate()', () => {
    let manager;
    let registeredValidators;
    let removedValidators;

    beforeEach(() => {
      manager = new StateSyncManager();
      registeredValidators = [];
      removedValidators = [];
    });

    it('should register validator on ADD action', async () => {
      const message = {
        action: 'ADD',
        validator: { publicKey: 'val-1', stake: 1000 },
      };

      const registerValidator = (val) => {
        registeredValidators.push(val);
      };

      await manager.handleValidatorUpdate(message, registerValidator, null);

      assert.equal(registeredValidators.length, 1);
      assert.equal(registeredValidators[0].publicKey, 'val-1');
      assert.equal(registeredValidators[0].stake, 1000);
    });

    it('should remove validator on REMOVE action', async () => {
      const message = {
        action: 'REMOVE',
        validator: { publicKey: 'val-2' },
      };

      const removeValidator = (pubKey) => {
        removedValidators.push(pubKey);
      };

      await manager.handleValidatorUpdate(message, null, removeValidator);

      assert.equal(removedValidators.length, 1);
      assert.equal(removedValidators[0], 'val-2');
    });

    it('should handle missing callbacks gracefully', async () => {
      const message = {
        action: 'ADD',
        validator: { publicKey: 'val-3' },
      };

      // Should not throw
      await manager.handleValidatorUpdate(message, null, null);

      assert.equal(registeredValidators.length, 0);
    });
  });
});
