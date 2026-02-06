/**
 * Deep tests for ForkDetector
 *
 * Tests chain fork detection, resolution, and E-Score weighted branch selection.
 *
 * @module @cynic/node/test/fork-detector-deep
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { ForkDetector } from '../src/network/fork-detector.js';

describe('ForkDetector - Deep Tests', () => {
  describe('Constructor', () => {
    it('should initialize with empty fork state', () => {
      const detector = new ForkDetector();

      assert.equal(detector._forkState.detected, false);
      assert.equal(detector._forkState.forkSlot, null);
      assert.ok(detector._forkState.forkHashes instanceof Map);
      assert.equal(detector._forkState.forkHashes.size, 0);
      assert.equal(detector._forkState.ourBranch, null);
      assert.equal(detector._forkState.resolutionInProgress, false);
    });

    it('should initialize with empty slot hashes map', () => {
      const detector = new ForkDetector();

      assert.ok(detector._slotHashes instanceof Map);
      assert.equal(detector._slotHashes.size, 0);
    });

    it('should initialize stats with zeros', () => {
      const detector = new ForkDetector();

      assert.deepEqual(detector._stats, {
        forksDetected: 0,
        forksResolved: 0,
      });
    });
  });

  describe('wire()', () => {
    it('should set all dependencies when provided', () => {
      const detector = new ForkDetector();
      const mockGetSlot = () => 100;
      const mockSendTo = async () => {};
      const mockGetPeerSlots = () => new Map();
      const mockPublicKey = 'test-key-123';

      detector.wire({
        getLastFinalizedSlot: mockGetSlot,
        sendTo: mockSendTo,
        getPeerSlots: mockGetPeerSlots,
        publicKey: mockPublicKey,
      });

      assert.equal(detector._getLastFinalizedSlot, mockGetSlot);
      assert.equal(detector._sendTo, mockSendTo);
      assert.equal(detector._getPeerSlots, mockGetPeerSlots);
      assert.equal(detector._publicKey, mockPublicKey);
    });

    it('should handle partial wiring gracefully', () => {
      const detector = new ForkDetector();
      const mockSendTo = async () => {};

      detector.wire({ sendTo: mockSendTo });

      assert.equal(detector._sendTo, mockSendTo);
      assert.equal(typeof detector._getLastFinalizedSlot, 'function'); // default still there
      assert.equal(typeof detector._getPeerSlots, 'function');
    });
  });

  describe('recordBlockHash()', () => {
    it('should store block hash with timestamp', () => {
      const detector = new ForkDetector();
      const beforeTime = Date.now();

      detector.recordBlockHash(100, 'hash-abc123');

      const afterTime = Date.now();
      const stored = detector._slotHashes.get(100);

      assert.ok(stored);
      assert.equal(stored.hash, 'hash-abc123');
      assert.ok(stored.confirmedAt >= beforeTime && stored.confirmedAt <= afterTime);
    });

    it('should overwrite existing hash for same slot', () => {
      const detector = new ForkDetector();

      detector.recordBlockHash(100, 'hash-old');
      detector.recordBlockHash(100, 'hash-new');

      const stored = detector._slotHashes.get(100);
      assert.equal(stored.hash, 'hash-new');
    });
  });

  describe('getRecentBlockHashes()', () => {
    it('should return recent slots in descending order', () => {
      const detector = new ForkDetector();
      detector.wire({ getLastFinalizedSlot: () => 105 });

      detector.recordBlockHash(103, 'hash-103');
      detector.recordBlockHash(104, 'hash-104');
      detector.recordBlockHash(105, 'hash-105');

      const hashes = detector.getRecentBlockHashes(3);

      assert.equal(hashes.length, 3);
      assert.deepEqual(hashes[0], { slot: 105, hash: 'hash-105' });
      assert.deepEqual(hashes[1], { slot: 104, hash: 'hash-104' });
      assert.deepEqual(hashes[2], { slot: 103, hash: 'hash-103' });
    });

    it('should return empty array when no hashes recorded', () => {
      const detector = new ForkDetector();
      detector.wire({ getLastFinalizedSlot: () => 100 });

      const hashes = detector.getRecentBlockHashes(5);

      assert.deepEqual(hashes, []);
    });

    it('should respect count parameter and skip missing slots', () => {
      const detector = new ForkDetector();
      detector.wire({ getLastFinalizedSlot: () => 110 });

      detector.recordBlockHash(108, 'hash-108');
      detector.recordBlockHash(110, 'hash-110');
      // slot 109 missing

      const hashes = detector.getRecentBlockHashes(3);

      assert.equal(hashes.length, 2);
      assert.deepEqual(hashes, [
        { slot: 110, hash: 'hash-110' },
        { slot: 108, hash: 'hash-108' },
      ]);
    });
  });

  describe('checkForForks() - No Fork Scenarios', () => {
    it('should not detect fork when single hash per slot', () => {
      const detector = new ForkDetector();
      let forkDetected = false;
      detector.on('fork:detected', () => { forkDetected = true; });

      detector.checkForForks('peer1', [{ slot: 100, hash: 'hash-a' }], 50);
      detector.checkForForks('peer2', [{ slot: 100, hash: 'hash-a' }], 60);

      assert.equal(forkDetected, false);
      assert.equal(detector._forkState.detected, false);
    });

    it('should accumulate peer info for same hash', () => {
      const detector = new ForkDetector();

      detector.checkForForks('peer1', [{ slot: 100, hash: 'hash-a' }], 50);
      detector.checkForForks('peer2', [{ slot: 100, hash: 'hash-a' }], 60);

      const slotForks = detector._forkState.forkHashes.get(100);
      const hashInfo = slotForks.get('hash-a');

      assert.equal(hashInfo.peers.size, 2);
      assert.ok(hashInfo.peers.has('peer1'));
      assert.ok(hashInfo.peers.has('peer2'));
    });

    it('should track total E-Score for each hash', () => {
      const detector = new ForkDetector();

      detector.checkForForks('peer1', [{ slot: 100, hash: 'hash-a' }], 50);
      detector.checkForForks('peer2', [{ slot: 100, hash: 'hash-a' }], 60);
      detector.checkForForks('peer3', [{ slot: 100, hash: 'hash-a' }], 30);

      const slotForks = detector._forkState.forkHashes.get(100);
      const hashInfo = slotForks.get('hash-a');

      assert.equal(hashInfo.totalEScore, 140);
    });
  });

  describe('checkForForks() - Fork Detection', () => {
    it('should detect fork when two different hashes for same slot', () => {
      const detector = new ForkDetector();
      let forkEvent = null;
      detector.on('fork:detected', (event) => { forkEvent = event; });

      detector.checkForForks('peer1', [{ slot: 100, hash: 'hash-a' }], 50);
      detector.checkForForks('peer2', [{ slot: 100, hash: 'hash-b' }], 60);

      assert.equal(detector._forkState.detected, true);
      assert.equal(detector._forkState.forkSlot, 100);
      assert.ok(forkEvent);
      assert.equal(forkEvent.slot, 100);
      assert.equal(forkEvent.branches, 2);
    });

    it('should emit fork:detected event with correct details', () => {
      const detector = new ForkDetector();
      detector.recordBlockHash(100, 'hash-a'); // our hash
      let forkEvent = null;
      detector.on('fork:detected', (event) => { forkEvent = event; });

      detector.checkForForks('peer1', [{ slot: 100, hash: 'hash-a' }], 50);
      detector.checkForForks('peer2', [{ slot: 100, hash: 'hash-b' }], 60);

      assert.ok(forkEvent);
      assert.equal(forkEvent.slot, 100);
      assert.equal(forkEvent.branches, 2);
      assert.equal(forkEvent.forks.length, 2);
      assert.ok(forkEvent.ourBranch);
      assert.ok(forkEvent.heaviestBranch);
    });

    it('should identify heaviest branch by total E-Score', () => {
      const detector = new ForkDetector();
      let forkEvent = null;
      detector.on('fork:detected', (event) => { forkEvent = event; });

      // Branch A: total 110 (50 + 60)
      detector.checkForForks('peer1', [{ slot: 100, hash: 'hash-a' }], 50);
      detector.checkForForks('peer2', [{ slot: 100, hash: 'hash-a' }], 60);

      // Branch B: total 80
      detector.checkForForks('peer3', [{ slot: 100, hash: 'hash-b' }], 80);

      assert.ok(forkEvent);
      const heaviest = forkEvent.forks.find(f => f.totalEScore === 110);
      assert.ok(heaviest);
    });

    it('should recommend STAY when on heaviest branch, REORG_NEEDED when not', () => {
      const detector = new ForkDetector();
      detector.recordBlockHash(100, 'hash-a'); // our hash
      let forkEvent = null;
      detector.on('fork:detected', (event) => { forkEvent = event; });

      // We're on hash-a with total E-Score 50
      detector.checkForForks('peer1', [{ slot: 100, hash: 'hash-a' }], 50);
      // hash-b has higher E-Score 80 (heaviest)
      detector.checkForForks('peer2', [{ slot: 100, hash: 'hash-b' }], 80);

      assert.ok(forkEvent);
      assert.equal(forkEvent.recommendation, 'REORG_NEEDED');

      // Test STAY case
      const detector2 = new ForkDetector();
      detector2.recordBlockHash(200, 'hash-c');
      let forkEvent2 = null;
      detector2.on('fork:detected', (event) => { forkEvent2 = event; });

      detector2.checkForForks('peer1', [{ slot: 200, hash: 'hash-c' }], 100);
      detector2.checkForForks('peer2', [{ slot: 200, hash: 'hash-d' }], 50);

      assert.ok(forkEvent2);
      assert.equal(forkEvent2.recommendation, 'STAY');
    });
  });

  describe('_resolveFork()', () => {
    it('should find peer with highest E-Score on target branch', async () => {
      const detector = new ForkDetector();
      const sentMessages = [];
      const mockSendTo = async (peerId, msg) => sentMessages.push({ peerId, msg });
      const mockGetPeerSlots = () => new Map([
        ['peer1', { eScore: 60 }],
        ['peer2', { eScore: 80 }],
        ['peer3', { eScore: 50 }],
      ]);

      detector.wire({
        sendTo: mockSendTo,
        getPeerSlots: mockGetPeerSlots,
        publicKey: 'node-key-abc',
      });

      // Manually set up fork state
      detector._forkState.forkHashes.set(100, new Map([
        ['hash-target', {
          peers: new Set(['peer1', 'peer2', 'peer3']),
          totalEScore: 190,
        }],
      ]));

      await detector._resolveFork(100, 'hash-target');

      assert.equal(sentMessages.length, 1);
      assert.equal(sentMessages[0].peerId, 'peer2'); // highest E-Score
    });

    it('should send FORK_RESOLUTION_REQUEST to best peer', async () => {
      const detector = new ForkDetector();
      const sentMessages = [];
      const mockSendTo = async (peerId, msg) => sentMessages.push({ peerId, msg });
      const mockGetPeerSlots = () => new Map([
        ['peer1', { eScore: 70 }],
      ]);

      detector.wire({
        sendTo: mockSendTo,
        getPeerSlots: mockGetPeerSlots,
        publicKey: 'node-key-xyz',
      });

      detector._forkState.forkHashes.set(100, new Map([
        ['hash-xyz', {
          peers: new Set(['peer1']),
          totalEScore: 70,
        }],
      ]));

      await detector._resolveFork(100, 'hash-xyz');

      assert.equal(sentMessages.length, 1);
      const msg = sentMessages[0].msg;
      assert.equal(msg.type, 'FORK_RESOLUTION_REQUEST');
      assert.equal(msg.forkSlot, 100);
      assert.equal(msg.targetHash, 'hash-xyz');
      assert.ok(msg.timestamp);
    });

    it('should emit fork:resolution_started event', async () => {
      const detector = new ForkDetector();
      const mockSendTo = async () => {};
      const mockGetPeerSlots = () => new Map([
        ['peer1', { eScore: 70 }],
      ]);

      detector.wire({
        sendTo: mockSendTo,
        getPeerSlots: mockGetPeerSlots,
      });

      detector._forkState.forkHashes.set(100, new Map([
        ['hash-abc', {
          peers: new Set(['peer1']),
          totalEScore: 70,
        }],
      ]));

      let resolutionEvent = null;
      detector.on('fork:resolution_started', (event) => { resolutionEvent = event; });

      await detector._resolveFork(100, 'hash-abc');

      assert.ok(resolutionEvent);
      assert.equal(resolutionEvent.forkSlot, 100);
      assert.ok(resolutionEvent.targetBranch);
      assert.ok(resolutionEvent.resolvingWith);
    });
  });

  describe('markForkResolved()', () => {
    it('should clear fork state and emit fork:resolved', () => {
      const detector = new ForkDetector();
      detector._forkState.detected = true;
      detector._forkState.forkSlot = 100;
      detector._forkState.ourBranch = 'hash-a';
      detector._forkState.resolutionInProgress = true;

      let resolvedEvent = null;
      detector.on('fork:resolved', (event) => { resolvedEvent = event; });

      detector.markForkResolved();

      assert.equal(detector._forkState.detected, false);
      assert.equal(detector._forkState.forkSlot, null);
      assert.equal(detector._forkState.ourBranch, null);
      assert.equal(detector._forkState.resolutionInProgress, false);

      assert.ok(resolvedEvent);
      assert.equal(resolvedEvent.forkSlot, 100);
    });

    it('should increment forksResolved stat', () => {
      const detector = new ForkDetector();
      detector._forkState.detected = true;
      detector._forkState.forkSlot = 100;

      assert.equal(detector._stats.forksResolved, 0);

      detector.markForkResolved();

      assert.equal(detector._stats.forksResolved, 1);
    });
  });

  describe('_cleanupForkData()', () => {
    it('should remove fork data for slots older than 100 slots', () => {
      const detector = new ForkDetector();
      detector.wire({ getLastFinalizedSlot: () => 250 });

      // Add old data (slot < 150)
      detector._forkState.forkHashes.set(140, new Map());
      detector._forkState.forkHashes.set(145, new Map());
      // Add recent data (slot >= 150)
      detector._forkState.forkHashes.set(200, new Map());
      detector._forkState.forkHashes.set(250, new Map());

      detector._cleanupForkData();

      assert.equal(detector._forkState.forkHashes.has(140), false);
      assert.equal(detector._forkState.forkHashes.has(145), false);
      assert.equal(detector._forkState.forkHashes.has(200), true);
      assert.equal(detector._forkState.forkHashes.has(250), true);
    });

    it('should remove slot hashes older than 100 slots', () => {
      const detector = new ForkDetector();
      detector.wire({ getLastFinalizedSlot: () => 300 });

      detector.recordBlockHash(190, 'hash-old1');
      detector.recordBlockHash(195, 'hash-old2');
      detector.recordBlockHash(250, 'hash-new1');
      detector.recordBlockHash(300, 'hash-new2');

      detector._cleanupForkData();

      assert.equal(detector._slotHashes.has(190), false);
      assert.equal(detector._slotHashes.has(195), false);
      assert.equal(detector._slotHashes.has(250), true);
      assert.equal(detector._slotHashes.has(300), true);
    });
  });

  describe('getForkStatus()', () => {
    it('should return complete fork state when fork detected', () => {
      const detector = new ForkDetector();
      detector._forkState.detected = true;
      detector._forkState.forkSlot = 100;
      detector._forkState.ourBranch = 'hash-our-branch-abc123';
      detector._forkState.resolutionInProgress = true;
      detector._stats.forksDetected = 2;
      detector._stats.forksResolved = 1;

      detector._forkState.forkHashes.set(100, new Map([
        ['hash-a', { peers: new Set(['peer1', 'peer2']), totalEScore: 120 }],
        ['hash-b', { peers: new Set(['peer3']), totalEScore: 80 }],
      ]));

      const status = detector.getForkStatus();

      assert.equal(status.detected, true);
      assert.equal(status.forkSlot, 100);
      assert.ok(status.ourBranch.startsWith('hash-our-branc'));
      assert.equal(status.resolutionInProgress, true);
      assert.equal(status.branches.length, 2);
      assert.deepEqual(status.stats, { forksDetected: 2, forksResolved: 1 });
    });

    it('should return clean state when no fork detected', () => {
      const detector = new ForkDetector();

      const status = detector.getForkStatus();

      assert.equal(status.detected, false);
      assert.equal(status.forkSlot, null);
      assert.equal(status.ourBranch, undefined);
      assert.equal(status.resolutionInProgress, false);
      assert.deepEqual(status.branches, []);
      assert.deepEqual(status.stats, { forksDetected: 0, forksResolved: 0 });
    });
  });
});
