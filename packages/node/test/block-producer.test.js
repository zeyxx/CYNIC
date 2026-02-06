/**
 * BlockProducer Tests
 *
 * PHASE 2: DECENTRALIZE
 *
 * Tests slot-based block production from JUDGMENT_CREATED events.
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert';
import { BlockProducer } from '../src/network/block-producer.js';
import { globalEventBus, EventType } from '@cynic/core';

const mockPublicKey = 'test-producer-key-0123456789abcdef';

describe('BlockProducer', () => {
  let producer;

  beforeEach(() => {
    producer = null;
  });

  afterEach(() => {
    if (producer) {
      producer.stop();
    }
    globalEventBus.removeAllListeners(EventType.JUDGMENT_CREATED);
  });

  describe('constructor', () => {
    it('creates with default options', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      assert.strictEqual(producer.running, false);
      assert.strictEqual(producer.pendingCount, 0);
      assert.strictEqual(producer.stats.blocksProduced, 0);
      assert.strictEqual(producer.stats.emptyBlocks, 0);
      assert.strictEqual(producer.stats.judgmentsIncluded, 0);
      assert.strictEqual(producer.stats.slotsAsLeader, 0);
      assert.strictEqual(producer.stats.slotsTotal, 0);
    });

    it('has a SlotManager instance', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      assert.ok(producer.slotManager !== undefined);
      assert.strictEqual(typeof producer.slotManager.getCurrentSlot, 'function');
    });

    it('accepts custom maxJudgmentsPerBlock', () => {
      producer = new BlockProducer({
        publicKey: mockPublicKey,
        maxJudgmentsPerBlock: 50,
      });

      assert.strictEqual(producer._maxJudgmentsPerBlock, 50);
    });
  });

  describe('wire()', () => {
    it('wires proposeBlock callback', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      const mockPropose = mock.fn();

      producer.wire({ proposeBlock: mockPropose });

      assert.strictEqual(producer._proposeBlock, mockPropose);
    });

    it('wires getValidators callback', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      const mockGetValidators = mock.fn(() => []);

      producer.wire({ getValidators: mockGetValidators });

      assert.strictEqual(producer._getValidators, mockGetValidators);
    });

    it('ignores null values', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      producer.wire({ proposeBlock: null, getValidators: null });

      assert.strictEqual(producer._proposeBlock, null);
      assert.strictEqual(producer._getValidators, null);
    });
  });

  describe('start/stop lifecycle', () => {
    it('starts and sets running to true', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const events = [];
      producer.on('started', () => events.push('started'));

      producer.start();

      assert.strictEqual(producer.running, true);
      assert.ok(events.includes('started'));
    });

    it('stops and sets running to false', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const events = [];
      producer.on('stopped', () => events.push('stopped'));

      producer.start();
      producer.stop();

      assert.strictEqual(producer.running, false);
      assert.ok(events.includes('stopped'));
    });

    it('start is idempotent', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      let count = 0;
      producer.on('started', () => count++);

      producer.start();
      producer.start(); // Second call should be noop

      assert.strictEqual(count, 1);
    });

    it('stop is idempotent', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      let count = 0;
      producer.on('stopped', () => count++);

      producer.start();
      producer.stop();
      producer.stop(); // Second call should be noop

      assert.strictEqual(count, 1);
    });
  });

  describe('judgment collection', () => {
    it('collects JUDGMENT_CREATED events when running', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg-1',
        payload: { qScore: 75, verdict: 'WAG' },
      });

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg-2',
        payload: { qScore: 30, verdict: 'GROWL' },
      });

      assert.strictEqual(producer.pendingCount, 2);
    });

    it('does not collect events when stopped', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      // Don't start - emit events
      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg-1',
        payload: { qScore: 50, verdict: 'BARK' },
      });

      assert.strictEqual(producer.pendingCount, 0);
    });

    it('stops collecting events after stop()', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg-1',
        payload: { qScore: 50 },
      });

      assert.strictEqual(producer.pendingCount, 1);

      producer.stop();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg-2',
        payload: { qScore: 50 },
      });

      // Should still be 1, not 2
      assert.strictEqual(producer.pendingCount, 1);
    });

    it('caps pending judgments at 3x maxJudgmentsPerBlock', () => {
      producer = new BlockProducer({
        publicKey: mockPublicKey,
        maxJudgmentsPerBlock: 2,
      });
      producer.start();

      // Emit 10 judgments, cap is 2 * 3 = 6
      for (let i = 0; i < 10; i++) {
        globalEventBus.emit(EventType.JUDGMENT_CREATED, {
          id: `jdg-${i}`,
          payload: { qScore: 50 },
        });
      }

      assert.strictEqual(producer.pendingCount, 6);
    });

    it('extracts judgment_id from event.id or payload.id', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'from-event-id',
        payload: { qScore: 50 },
      });

      assert.strictEqual(producer._pendingJudgments[0].judgment_id, 'from-event-id');
    });
  });

  describe('block production (_onSlot)', () => {
    it('increments slotsTotal on each slot', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.start();

      // Directly call _onSlot (bypasses SlotManager timing)
      // Mock isLeader to return false
      mock.method(producer._slotManager, 'isLeader', () => false);

      producer._onSlot(1, false);
      producer._onSlot(2, false);
      producer._onSlot(3, false);

      assert.strictEqual(producer.stats.slotsTotal, 3);
      assert.strictEqual(producer.stats.slotsAsLeader, 0);
    });

    it('produces block when we are leader', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      const mockPropose = mock.fn(() => ({ id: 'record-1' }));
      producer.wire({ proposeBlock: mockPropose });
      producer.start();

      mock.method(producer._slotManager, 'isLeader', () => true);

      producer._onSlot(10, false);

      assert.strictEqual(producer.stats.slotsAsLeader, 1);
      assert.strictEqual(producer.stats.blocksProduced, 1);
      assert.strictEqual(producer.stats.emptyBlocks, 1); // No pending judgments
      assert.strictEqual(mockPropose.mock.callCount(), 1);
    });

    it('does not produce block when not leader', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      const mockPropose = mock.fn();
      producer.wire({ proposeBlock: mockPropose });
      producer.start();

      mock.method(producer._slotManager, 'isLeader', () => false);

      producer._onSlot(10, false);

      assert.strictEqual(producer.stats.slotsAsLeader, 0);
      assert.strictEqual(producer.stats.blocksProduced, 0);
      assert.strictEqual(mockPropose.mock.callCount(), 0);
    });

    it('includes pending judgments in block', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      const blocks = [];
      const mockPropose = mock.fn((block) => {
        blocks.push(block);
        return { id: 'record-1' };
      });
      producer.wire({ proposeBlock: mockPropose });
      producer.start();

      // Add judgments
      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg-1',
        payload: { qScore: 80, verdict: 'WAG' },
      });
      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg-2',
        payload: { qScore: 40, verdict: 'GROWL' },
      });

      mock.method(producer._slotManager, 'isLeader', () => true);
      producer._onSlot(10, false);

      assert.strictEqual(blocks.length, 1);
      assert.strictEqual(blocks[0].judgment_count, 2);
      assert.ok(blocks[0].judgment_ids.includes('jdg-1'));
      assert.ok(blocks[0].judgment_ids.includes('jdg-2'));
      assert.strictEqual(producer.stats.judgmentsIncluded, 2);
      assert.strictEqual(producer.stats.emptyBlocks, 0);

      // Pending should be drained
      assert.strictEqual(producer.pendingCount, 0);
    });

    it('drains at most maxJudgmentsPerBlock', () => {
      producer = new BlockProducer({
        publicKey: mockPublicKey,
        maxJudgmentsPerBlock: 2,
      });
      const mockPropose = mock.fn(() => ({ id: 'r' }));
      producer.wire({ proposeBlock: mockPropose });
      producer.start();

      // Add 5 judgments
      for (let i = 0; i < 5; i++) {
        globalEventBus.emit(EventType.JUDGMENT_CREATED, {
          id: `jdg-${i}`,
          payload: { qScore: 50 },
        });
      }

      mock.method(producer._slotManager, 'isLeader', () => true);
      producer._onSlot(10, false);

      // First block takes 2, 3 remain
      assert.strictEqual(producer.stats.judgmentsIncluded, 2);
      assert.strictEqual(producer.pendingCount, 3);
    });

    it('emits block:produced event', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.wire({ proposeBlock: mock.fn(() => ({ id: 'r' })) });
      producer.start();

      const events = [];
      producer.on('block:produced', (e) => events.push(e));

      mock.method(producer._slotManager, 'isLeader', () => true);
      producer._onSlot(42, false);

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].slot, 42);
      assert.ok(events[0].hash !== undefined);
      assert.strictEqual(events[0].judgmentCount, 0);
    });

    it('updates lastBlockHash after successful production', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      const blocks = [];
      producer.wire({
        proposeBlock: mock.fn((block) => {
          blocks.push(block);
          return { id: 'r' };
        }),
      });
      producer.start();

      const initialHash = producer._lastBlockHash;
      assert.strictEqual(initialHash, '0'.repeat(64));

      mock.method(producer._slotManager, 'isLeader', () => true);
      producer._onSlot(10, false);

      assert.notStrictEqual(producer._lastBlockHash, initialHash);
      assert.strictEqual(producer._lastBlockHash, blocks[0].hash);
    });

    it('chains blocks via prev_hash', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      const blocks = [];
      producer.wire({
        proposeBlock: mock.fn((block) => {
          blocks.push(block);
          return { id: 'r' };
        }),
      });
      producer.start();

      mock.method(producer._slotManager, 'isLeader', () => true);

      producer._onSlot(10, false);
      producer._onSlot(11, false);

      assert.strictEqual(blocks.length, 2);
      assert.strictEqual(blocks[1].prev_hash, blocks[0].hash);
    });

    it('handles proposeBlock failure gracefully', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.wire({
        proposeBlock: mock.fn(() => {
          throw new Error('Consensus offline');
        }),
      });
      producer.start();

      mock.method(producer._slotManager, 'isLeader', () => true);

      // Should not throw
      assert.doesNotThrow(() => producer._onSlot(10, false));
      assert.strictEqual(producer.stats.blocksProduced, 0);
    });

    it('handles null proposeBlock return gracefully', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.wire({ proposeBlock: mock.fn(() => null) });
      producer.start();

      mock.method(producer._slotManager, 'isLeader', () => true);
      producer._onSlot(10, false);

      // No record returned = block not counted
      assert.strictEqual(producer.stats.blocksProduced, 0);
    });

    it('syncs validators on new epoch', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      const mockGetValidators = mock.fn(() => [
        { publicKey: 'val-1', eScore: 60, burned: 0, uptime: 1.0 },
        { publicKey: 'val-2', eScore: 80, burned: 99, uptime: 0.9 },
      ]);
      producer.wire({ getValidators: mockGetValidators });
      producer.start();

      mock.method(producer._slotManager, 'isLeader', () => false);

      // isNewEpoch = true should trigger _syncValidators
      producer._onSlot(432, true);

      // Called once on start() and once on epoch boundary
      assert.strictEqual(mockGetValidators.mock.callCount(), 2);
    });
  });

  describe('_syncValidators (format conversion)', () => {
    it('converts ValidatorManager format to SlotManager {id, weight} format', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.wire({
        getValidators: () => [
          { publicKey: 'val-1', eScore: 60, burned: 0, uptime: 1.0 },
          { publicKey: 'val-2', eScore: 80, burned: 99, uptime: 0.9 },
        ],
      });

      const spy = mock.method(producer._slotManager, 'setValidators', () => {});
      producer._syncValidators();

      assert.strictEqual(spy.mock.callCount(), 1);
      const validators = spy.mock.calls[0].arguments[0];

      // val-1: 60 * sqrt(1) * 1.0 = 60
      assert.deepStrictEqual(validators[0], { id: 'val-1', weight: 60 });
      // val-2: 80 * sqrt(100) * 0.9 = 80 * 10 * 0.9 = 720
      assert.deepStrictEqual(validators[1], { id: 'val-2', weight: 720 });
    });

    it('adds self to validator set if not present', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.wire({
        getValidators: () => [
          { publicKey: 'other-validator', eScore: 50, burned: 0, uptime: 1.0 },
        ],
      });

      const spy = mock.method(producer._slotManager, 'setValidators', () => {});
      producer._syncValidators();

      const validators = spy.mock.calls[0].arguments[0];
      assert.strictEqual(validators.length, 2);
      assert.ok(validators.find(v => v.id === mockPublicKey) !== undefined);
      assert.strictEqual(validators.find(v => v.id === mockPublicKey).weight, 50);
    });

    it('does not duplicate self if already in validator set', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.wire({
        getValidators: () => [
          { publicKey: mockPublicKey, eScore: 75, burned: 0, uptime: 1.0 },
        ],
      });

      const spy = mock.method(producer._slotManager, 'setValidators', () => {});
      producer._syncValidators();

      const validators = spy.mock.calls[0].arguments[0];
      assert.strictEqual(validators.length, 1);
      assert.strictEqual(validators[0].id, mockPublicKey);
    });

    it('handles empty validator set (adds self only)', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.wire({ getValidators: () => [] });

      const spy = mock.method(producer._slotManager, 'setValidators', () => {});
      producer._syncValidators();

      const validators = spy.mock.calls[0].arguments[0];
      assert.strictEqual(validators.length, 1);
      assert.strictEqual(validators[0].id, mockPublicKey);
    });

    it('handles null getValidators gracefully', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      // No getValidators wired

      const spy = mock.method(producer._slotManager, 'setValidators', () => {});
      producer._syncValidators();

      const validators = spy.mock.calls[0].arguments[0];
      assert.strictEqual(validators.length, 1); // Just self
    });

    it('defaults missing burned/uptime fields', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });
      producer.wire({
        getValidators: () => [
          { publicKey: 'val-1', eScore: 50 }, // No burned/uptime
        ],
      });

      const spy = mock.method(producer._slotManager, 'setValidators', () => {});
      producer._syncValidators();

      const validators = spy.mock.calls[0].arguments[0];
      // weight = 50 * sqrt(0+1) * 1.0 = 50
      assert.strictEqual(validators[0].weight, 50);
    });
  });

  describe('_createBlock', () => {
    it('creates block with correct structure', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const judgments = [
        { judgment_id: 'jdg-1', q_score: 80, verdict: 'WAG', timestamp: Date.now() },
      ];

      const block = producer._createBlock(42, judgments);

      assert.strictEqual(block.slot, 42);
      assert.strictEqual(block.proposer, mockPublicKey);
      assert.ok(block.hash !== undefined);
      assert.strictEqual(block.hash.length, 64); // SHA-256 hex
      assert.strictEqual(block.block_hash, block.hash);
      assert.strictEqual(block.prev_hash, '0'.repeat(64));
      assert.ok(block.merkle_root !== undefined);
      assert.strictEqual(block.judgments_root, block.merkle_root);
      assert.strictEqual(block.judgments.length, 1);
      assert.strictEqual(block.judgment_count, 1);
      assert.ok(block.judgment_ids.includes('jdg-1'));
      assert.ok(block.timestamp !== undefined);
    });

    it('creates different hashes for different slots', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const block1 = producer._createBlock(1, []);
      const block2 = producer._createBlock(2, []);

      assert.notStrictEqual(block1.hash, block2.hash);
    });
  });

  describe('_computeMerkleRoot', () => {
    it('returns zero hash for empty judgments', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const root = producer._computeMerkleRoot([]);
      assert.strictEqual(root, '0'.repeat(64));
    });

    it('returns SHA-256 hash of judgment IDs', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const root = producer._computeMerkleRoot([
        { judgment_id: 'jdg-1' },
        { judgment_id: 'jdg-2' },
      ]);

      assert.strictEqual(root.length, 64);
      assert.notStrictEqual(root, '0'.repeat(64));
    });

    it('returns different roots for different judgment sets', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const root1 = producer._computeMerkleRoot([{ judgment_id: 'a' }]);
      const root2 = producer._computeMerkleRoot([{ judgment_id: 'b' }]);

      assert.notStrictEqual(root1, root2);
    });

    it('is deterministic', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const judgments = [{ judgment_id: 'x' }, { judgment_id: 'y' }];
      const root1 = producer._computeMerkleRoot(judgments);
      const root2 = producer._computeMerkleRoot(judgments);

      assert.strictEqual(root1, root2);
    });
  });

  describe('stats', () => {
    it('returns a copy of stats (not reference)', () => {
      producer = new BlockProducer({ publicKey: mockPublicKey });

      const stats1 = producer.stats;
      const stats2 = producer.stats;

      assert.deepStrictEqual(stats1, stats2);
      assert.notStrictEqual(stats1, stats2); // Different objects
    });
  });
});
