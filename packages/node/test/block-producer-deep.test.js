/**
 * @cynic/node - BlockProducer Deep Tests
 *
 * Comprehensive tests for BlockProducer (PHASE 2 block production):
 * - Constructor & initialization (publicKey, stats, lastBlockHash, maxJudgmentsPerBlock, slotManager)
 * - wire() dependency injection (proposeBlock, getValidators)
 * - start/stop lifecycle (running flag, event handlers, cleanup)
 * - Slot management & _syncValidators (validator set conversion, self-inclusion, weight calculation, epoch resync)
 * - Judgment collection (globalEventBus listener, caps at 3x max, field extraction, pending count)
 * - Block production _createBlock (slot/proposer/hash, prev_hash chaining, merkle_root, judgment_ids, timestamp)
 * - _computeMerkleRoot (empty returns zeros, single judgment, multiple judgments deterministic)
 * - _onSlot flow (slotsTotal increment, leader check, judgment drain, proposeBlock call, block:produced event)
 * - Error handling (proposeBlock throws, returns falsy, not wired)
 * - Stats tracking (blocksProduced, emptyBlocks, judgmentsIncluded)
 *
 * Uses node:test (NOT vitest) for CI compatibility.
 *
 * "φ distrusts φ" - κυνικός
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import { BlockProducer } from '../src/network/block-producer.js';
import { globalEventBus, EventType } from '@cynic/core';

describe('BlockProducer (Deep)', () => {
  let producer;
  const TEST_PUBLIC_KEY = 'test_validator_pubkey_12345';

  beforeEach(() => {
    producer = new BlockProducer({
      publicKey: TEST_PUBLIC_KEY,
      slotDuration: 50, // 50ms for fast tests
    });
  });

  afterEach(() => {
    if (producer?.running) {
      producer.stop();
    }
  });

  // ═══════════════════════════════════════════════════════════════════════
  // CONSTRUCTOR & INITIALIZATION
  // ═══════════════════════════════════════════════════════════════════════

  describe('constructor', () => {
    it('should set publicKey from options', () => {
      const bp = new BlockProducer({ publicKey: 'test_key' });
      assert.strictEqual(bp._publicKey, 'test_key');
    });

    it('should initialize stats to zeros', () => {
      const stats = producer.stats;
      assert.strictEqual(stats.blocksProduced, 0);
      assert.strictEqual(stats.emptyBlocks, 0);
      assert.strictEqual(stats.judgmentsIncluded, 0);
      assert.strictEqual(stats.slotsAsLeader, 0);
      assert.strictEqual(stats.slotsTotal, 0);
    });

    it('should initialize lastBlockHash to genesis (64 zeros)', () => {
      assert.strictEqual(producer._lastBlockHash, '0'.repeat(64));
    });

    it('should set maxJudgmentsPerBlock from options (default 100)', () => {
      assert.strictEqual(producer._maxJudgmentsPerBlock, 100);

      const custom = new BlockProducer({ maxJudgmentsPerBlock: 50 });
      assert.strictEqual(custom._maxJudgmentsPerBlock, 50);
    });

    it('should create SlotManager', () => {
      assert.ok(producer.slotManager);
      assert.strictEqual(typeof producer.slotManager.start, 'function');
    });

    it('should start with running=false', () => {
      assert.strictEqual(producer.running, false);
    });

    it('should initialize empty pending judgments', () => {
      assert.strictEqual(producer.pendingCount, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // wire()
  // ═══════════════════════════════════════════════════════════════════════

  describe('wire', () => {
    it('should wire proposeBlock function', () => {
      const proposeBlock = () => ({ id: 1 });
      producer.wire({ proposeBlock });
      assert.strictEqual(producer._proposeBlock, proposeBlock);
    });

    it('should wire getValidators function', () => {
      const getValidators = () => [];
      producer.wire({ getValidators });
      assert.strictEqual(producer._getValidators, getValidators);
    });

    it('should allow partial wire (only proposeBlock)', () => {
      const proposeBlock = () => ({ id: 1 });
      producer.wire({ proposeBlock });
      assert.strictEqual(producer._proposeBlock, proposeBlock);
      assert.strictEqual(producer._getValidators, null);
    });

    it('should allow partial wire (only getValidators)', () => {
      const getValidators = () => [];
      producer.wire({ getValidators });
      assert.strictEqual(producer._getValidators, getValidators);
      assert.strictEqual(producer._proposeBlock, null);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // start/stop
  // ═══════════════════════════════════════════════════════════════════════

  describe('start/stop', () => {
    it('should set running flag on start', () => {
      producer.start();
      assert.strictEqual(producer.running, true);
    });

    it('should emit started event', (t, done) => {
      producer.once('started', () => {
        assert.ok(true);
        done();
      });
      producer.start();
    });

    it('should be idempotent (multiple starts safe)', () => {
      producer.start();
      producer.start();
      assert.strictEqual(producer.running, true);
    });

    it('should unset running flag on stop', () => {
      producer.start();
      producer.stop();
      assert.strictEqual(producer.running, false);
    });

    it('should emit stopped event', (t, done) => {
      producer.start();
      producer.once('stopped', () => {
        assert.ok(true);
        done();
      });
      producer.stop();
    });

    it('should be idempotent (multiple stops safe)', () => {
      producer.start();
      producer.stop();
      producer.stop();
      assert.strictEqual(producer.running, false);
    });

    it('should cleanup judgment handler on stop', () => {
      producer.start();
      const handler = producer._judgmentHandler;
      assert.ok(handler, 'Should have judgment handler after start');

      producer.stop();
      assert.strictEqual(producer._judgmentHandler, null, 'Should cleanup handler on stop');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // SLOT MANAGEMENT & _syncValidators
  // ═══════════════════════════════════════════════════════════════════════

  describe('_syncValidators', () => {
    it('should add self to validator set if not present', () => {
      producer.wire({ getValidators: () => [] });
      producer._syncValidators();

      const validators = producer.slotManager.validators;
      assert.ok(validators.length > 0);
      assert.ok(validators.some(v => v.id === TEST_PUBLIC_KEY));
    });

    it('should convert ValidatorManager format to SlotManager format', () => {
      const validators = [
        { publicKey: 'val1', eScore: 70, burned: 1000, uptime: 0.99 },
        { publicKey: 'val2', eScore: 50, burned: 500, uptime: 0.95 },
      ];

      producer.wire({ getValidators: () => validators });
      producer._syncValidators();

      const slotVals = producer.slotManager.validators;
      assert.ok(slotVals.length >= 2);
      assert.ok(slotVals.some(v => v.id === 'val1'));
      assert.ok(slotVals.some(v => v.id === 'val2'));
      assert.ok(slotVals.every(v => typeof v.weight === 'number'));
    });

    it('should handle empty validators array', () => {
      producer.wire({ getValidators: () => [] });
      producer._syncValidators();

      // Should still add self
      const validators = producer.slotManager.validators;
      assert.strictEqual(validators.length, 1);
      assert.strictEqual(validators[0].id, TEST_PUBLIC_KEY);
    });

    it('should calculate validator weight using calculateVoteWeight', () => {
      const validators = [
        { publicKey: 'val1', eScore: 80, burned: 2000, uptime: 1.0 },
      ];

      producer.wire({ getValidators: () => validators });
      producer._syncValidators();

      const slotVals = producer.slotManager.validators;
      const val1 = slotVals.find(v => v.id === 'val1');
      assert.ok(val1);
      assert.ok(val1.weight > 0);
      assert.ok(typeof val1.weight === 'number');
    });

    it('should use default values for missing burned/uptime', () => {
      const validators = [
        { publicKey: 'val1', eScore: 60 }, // No burned/uptime
      ];

      producer.wire({ getValidators: () => validators });
      producer._syncValidators();

      const slotVals = producer.slotManager.validators;
      const val1 = slotVals.find(v => v.id === 'val1');
      assert.ok(val1);
      assert.ok(val1.weight > 0);
    });

    it('should resync validators on epoch boundary', async () => {
      let validatorCount = 1;
      const getValidators = () => {
        return validatorCount === 1
          ? [{ publicKey: 'val1', eScore: 50, burned: 100, uptime: 1.0 }]
          : [
              { publicKey: 'val1', eScore: 50, burned: 100, uptime: 1.0 },
              { publicKey: 'val2', eScore: 60, burned: 200, uptime: 1.0 },
            ];
      };

      producer.wire({ getValidators, proposeBlock: () => ({ id: 1 }) });

      // Mock slotManager to always be leader and trigger epoch
      const originalIsLeader = producer.slotManager.isLeader.bind(producer.slotManager);
      producer.slotManager.isLeader = mock.fn(() => true);

      producer.start();

      // Wait for first slot
      await new Promise(resolve => setTimeout(resolve, 60));

      // Change validator count
      validatorCount = 2;

      // Trigger _onSlot with isNewEpoch=true
      producer._onSlot(1, true);

      const validators = producer.slotManager.validators;
      // Should now include val2 (plus self)
      assert.ok(validators.length >= 2);

      producer.slotManager.isLeader = originalIsLeader;
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // JUDGMENT COLLECTION
  // ═══════════════════════════════════════════════════════════════════════

  describe('judgment collection', () => {
    it('should collect judgments from globalEventBus', async () => {
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_test_001',
        payload: { qScore: 75, verdict: 'GOOD' },
      });

      await new Promise(resolve => setTimeout(resolve, 10));

      assert.ok(producer.pendingCount > 0);
    });

    it('should cap pending judgments at 3x maxJudgmentsPerBlock', () => {
      producer.start();

      // Try to add more than 3x max (300)
      for (let i = 0; i < 350; i++) {
        globalEventBus.emit(EventType.JUDGMENT_CREATED, {
          id: `jdg_${i}`,
          payload: { qScore: 50, verdict: 'BARK' },
        });
      }

      assert.ok(producer.pendingCount <= producer._maxJudgmentsPerBlock * 3);
    });

    it('should extract judgment_id from event.id', () => {
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_from_event_id',
        payload: { qScore: 60, verdict: 'WARN' },
      });

      const pending = producer._pendingJudgments;
      assert.ok(pending.some(j => j.judgment_id === 'jdg_from_event_id'));
    });

    it('should extract judgment_id from payload.id if event.id missing', () => {
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        payload: { id: 'jdg_from_payload_id', qScore: 55, verdict: 'BARK' },
      });

      const pending = producer._pendingJudgments;
      assert.ok(pending.some(j => j.judgment_id === 'jdg_from_payload_id'));
    });

    it('should fallback to generated judgment_id if both missing', () => {
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        payload: { qScore: 50, verdict: 'BARK' },
      });

      const pending = producer._pendingJudgments;
      assert.ok(pending.length > 0);
      assert.ok(pending[pending.length - 1].judgment_id.startsWith('jdg_'));
    });

    it('should extract q_score from payload', () => {
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_score_test',
        payload: { qScore: 88, verdict: 'EXCELLENT' },
      });

      const pending = producer._pendingJudgments;
      const judgment = pending.find(j => j.judgment_id === 'jdg_score_test');
      assert.strictEqual(judgment.q_score, 88);
    });

    it('should extract verdict from payload', () => {
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_verdict_test',
        payload: { qScore: 40, verdict: 'REJECT' },
      });

      const pending = producer._pendingJudgments;
      const judgment = pending.find(j => j.judgment_id === 'jdg_verdict_test');
      assert.strictEqual(judgment.verdict, 'REJECT');
    });

    it('should default q_score to 50 if missing', () => {
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_no_score',
        payload: { verdict: 'BARK' },
      });

      const pending = producer._pendingJudgments;
      const judgment = pending.find(j => j.judgment_id === 'jdg_no_score');
      assert.strictEqual(judgment.q_score, 50);
    });

    it('should default verdict to BARK if missing', () => {
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_no_verdict',
        payload: { qScore: 60 },
      });

      const pending = producer._pendingJudgments;
      const judgment = pending.find(j => j.judgment_id === 'jdg_no_verdict');
      assert.strictEqual(judgment.verdict, 'BARK');
    });

    it('should set timestamp on collected judgments', () => {
      producer.start();

      const before = Date.now();
      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_timestamp',
        payload: { qScore: 50, verdict: 'BARK' },
      });
      const after = Date.now();

      const pending = producer._pendingJudgments;
      const judgment = pending.find(j => j.judgment_id === 'jdg_timestamp');
      assert.ok(judgment.timestamp >= before);
      assert.ok(judgment.timestamp <= after);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // BLOCK PRODUCTION _createBlock
  // ═══════════════════════════════════════════════════════════════════════

  describe('_createBlock', () => {
    it('should include slot in block', () => {
      const block = producer._createBlock(42, []);
      assert.strictEqual(block.slot, 42);
    });

    it('should include proposer publicKey', () => {
      const block = producer._createBlock(1, []);
      assert.strictEqual(block.proposer, TEST_PUBLIC_KEY);
    });

    it('should generate hash', () => {
      const block = producer._createBlock(1, []);
      assert.ok(block.hash);
      assert.strictEqual(block.hash.length, 64); // SHA-256 hex
    });

    it('should set block_hash equal to hash', () => {
      const block = producer._createBlock(1, []);
      assert.strictEqual(block.block_hash, block.hash);
    });

    it('should chain prev_hash from lastBlockHash', () => {
      producer._lastBlockHash = 'abc123'.padEnd(64, '0');
      const block = producer._createBlock(1, []);
      assert.strictEqual(block.prev_hash, 'abc123'.padEnd(64, '0'));
    });

    it('should update lastBlockHash after producing block', () => {
      producer.wire({ proposeBlock: (b) => ({ id: 1, hash: b.hash }) });

      // Mock slotManager to always be leader
      producer.slotManager.isLeader = mock.fn(() => true);

      producer.start();

      const initialHash = producer._lastBlockHash;
      producer._onSlot(1, false);

      // lastBlockHash should have changed
      assert.notStrictEqual(producer._lastBlockHash, initialHash);
    });

    it('should compute merkle_root from judgments', () => {
      const judgments = [
        { judgment_id: 'jdg_001', q_score: 60, verdict: 'BARK' },
        { judgment_id: 'jdg_002', q_score: 70, verdict: 'GOOD' },
      ];
      const block = producer._createBlock(1, judgments);
      assert.ok(block.merkle_root);
      assert.strictEqual(block.merkle_root.length, 64);
    });

    it('should set judgments_root equal to merkle_root', () => {
      const judgments = [{ judgment_id: 'jdg_001', q_score: 50, verdict: 'BARK' }];
      const block = producer._createBlock(1, judgments);
      assert.strictEqual(block.judgments_root, block.merkle_root);
    });

    it('should include judgments array', () => {
      const judgments = [
        { judgment_id: 'jdg_001', q_score: 50, verdict: 'BARK' },
        { judgment_id: 'jdg_002', q_score: 60, verdict: 'WARN' },
      ];
      const block = producer._createBlock(1, judgments);
      assert.deepStrictEqual(block.judgments, judgments);
    });

    it('should set judgment_count', () => {
      const judgments = [
        { judgment_id: 'jdg_001', q_score: 50, verdict: 'BARK' },
        { judgment_id: 'jdg_002', q_score: 60, verdict: 'WARN' },
        { judgment_id: 'jdg_003', q_score: 70, verdict: 'GOOD' },
      ];
      const block = producer._createBlock(1, judgments);
      assert.strictEqual(block.judgment_count, 3);
    });

    it('should extract judgment_ids array', () => {
      const judgments = [
        { judgment_id: 'jdg_001', q_score: 50, verdict: 'BARK' },
        { judgment_id: 'jdg_002', q_score: 60, verdict: 'WARN' },
      ];
      const block = producer._createBlock(1, judgments);
      assert.deepStrictEqual(block.judgment_ids, ['jdg_001', 'jdg_002']);
    });

    it('should set timestamp', () => {
      const before = Date.now();
      const block = producer._createBlock(1, []);
      const after = Date.now();

      assert.ok(block.timestamp >= before);
      assert.ok(block.timestamp <= after);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _computeMerkleRoot
  // ═══════════════════════════════════════════════════════════════════════

  describe('_computeMerkleRoot', () => {
    it('should return 64 zeros for empty judgments', () => {
      const root = producer._computeMerkleRoot([]);
      assert.strictEqual(root, '0'.repeat(64));
    });

    it('should compute hash for single judgment', () => {
      const judgments = [{ judgment_id: 'jdg_single' }];
      const root = producer._computeMerkleRoot(judgments);
      assert.ok(root);
      assert.strictEqual(root.length, 64);
      assert.notStrictEqual(root, '0'.repeat(64));
    });

    it('should compute deterministic hash for same judgments', () => {
      const judgments = [
        { judgment_id: 'jdg_001' },
        { judgment_id: 'jdg_002' },
        { judgment_id: 'jdg_003' },
      ];
      const root1 = producer._computeMerkleRoot(judgments);
      const root2 = producer._computeMerkleRoot(judgments);
      assert.strictEqual(root1, root2);
    });

    it('should compute different hash for different judgments', () => {
      const judgments1 = [{ judgment_id: 'jdg_001' }];
      const judgments2 = [{ judgment_id: 'jdg_002' }];
      const root1 = producer._computeMerkleRoot(judgments1);
      const root2 = producer._computeMerkleRoot(judgments2);
      assert.notStrictEqual(root1, root2);
    });

    it('should compute different hash for different order', () => {
      const judgments1 = [{ judgment_id: 'jdg_001' }, { judgment_id: 'jdg_002' }];
      const judgments2 = [{ judgment_id: 'jdg_002' }, { judgment_id: 'jdg_001' }];
      const root1 = producer._computeMerkleRoot(judgments1);
      const root2 = producer._computeMerkleRoot(judgments2);
      assert.notStrictEqual(root1, root2);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _onSlot FLOW
  // ═══════════════════════════════════════════════════════════════════════

  describe('_onSlot flow', () => {
    it('should increment slotsTotal on every slot', () => {
      producer._onSlot(1, false);
      producer._onSlot(2, false);
      producer._onSlot(3, false);

      const stats = producer.stats;
      assert.strictEqual(stats.slotsTotal, 3);
    });

    it('should check if node is leader before producing', () => {
      // Mock slotManager.isLeader to return false
      producer.slotManager.isLeader = mock.fn(() => false);

      producer._onSlot(1, false);

      const stats = producer.stats;
      assert.strictEqual(stats.slotsAsLeader, 0);
      assert.strictEqual(stats.blocksProduced, 0);
    });

    it('should increment slotsAsLeader when leader', () => {
      producer.slotManager.isLeader = mock.fn(() => true);
      producer.wire({ proposeBlock: () => ({ id: 1 }) });

      producer._onSlot(1, false);

      const stats = producer.stats;
      assert.strictEqual(stats.slotsAsLeader, 1);
    });

    it('should drain pending judgments when leader', () => {
      producer.slotManager.isLeader = mock.fn(() => true);
      producer.wire({ proposeBlock: () => ({ id: 1 }) });

      // Add some pending judgments
      producer._pendingJudgments.push(
        { judgment_id: 'jdg_001', q_score: 50, verdict: 'BARK', timestamp: Date.now() },
        { judgment_id: 'jdg_002', q_score: 60, verdict: 'WARN', timestamp: Date.now() }
      );

      assert.strictEqual(producer.pendingCount, 2);

      producer._onSlot(1, false);

      assert.strictEqual(producer.pendingCount, 0);
    });

    it('should only drain up to maxJudgmentsPerBlock', () => {
      producer.slotManager.isLeader = mock.fn(() => true);
      producer.wire({ proposeBlock: () => ({ id: 1 }) });
      producer._maxJudgmentsPerBlock = 3;

      // Add 5 judgments
      for (let i = 0; i < 5; i++) {
        producer._pendingJudgments.push({
          judgment_id: `jdg_${i}`,
          q_score: 50,
          verdict: 'BARK',
          timestamp: Date.now(),
        });
      }

      producer._onSlot(1, false);

      // Should have 2 remaining (5 - 3)
      assert.strictEqual(producer.pendingCount, 2);
    });

    it('should call proposeBlock when leader', () => {
      const proposeBlockMock = mock.fn(() => ({ id: 1 }));
      producer.wire({ proposeBlock: proposeBlockMock });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer._onSlot(1, false);

      assert.strictEqual(proposeBlockMock.mock.calls.length, 1);
    });

    it('should pass block to proposeBlock', () => {
      let capturedBlock = null;
      const proposeBlock = (block) => {
        capturedBlock = block;
        return { id: 1 };
      };
      producer.wire({ proposeBlock });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer._onSlot(42, false);

      assert.ok(capturedBlock);
      assert.strictEqual(capturedBlock.slot, 42);
      assert.strictEqual(capturedBlock.proposer, TEST_PUBLIC_KEY);
    });

    it('should emit block:produced event', (t, done) => {
      producer.wire({ proposeBlock: () => ({ id: 1 }) });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer.once('block:produced', (event) => {
        assert.strictEqual(event.slot, 1);
        assert.ok(event.hash);
        assert.strictEqual(event.judgmentCount, 0);
        done();
      });

      producer._onSlot(1, false);
    });

    it('should resync validators on new epoch', () => {
      const getValidatorsMock = mock.fn(() => []);
      producer.wire({ getValidators: getValidatorsMock, proposeBlock: () => ({ id: 1 }) });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer._onSlot(1, true); // isNewEpoch=true

      assert.ok(getValidatorsMock.mock.calls.length > 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // ERROR HANDLING
  // ═══════════════════════════════════════════════════════════════════════

  describe('error handling', () => {
    it('should handle proposeBlock throwing error', () => {
      const proposeBlock = () => {
        throw new Error('Consensus failed');
      };
      producer.wire({ proposeBlock });
      producer.slotManager.isLeader = mock.fn(() => true);

      // Should not throw
      assert.doesNotThrow(() => {
        producer._onSlot(1, false);
      });

      // Should not increment blocksProduced
      const stats = producer.stats;
      assert.strictEqual(stats.blocksProduced, 0);
    });

    it('should handle proposeBlock returning falsy', () => {
      const proposeBlock = () => null;
      producer.wire({ proposeBlock });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer._onSlot(1, false);

      const stats = producer.stats;
      assert.strictEqual(stats.blocksProduced, 0);
    });

    it('should handle proposeBlock not wired', () => {
      producer.slotManager.isLeader = mock.fn(() => true);

      // Should not throw
      assert.doesNotThrow(() => {
        producer._onSlot(1, false);
      });

      const stats = producer.stats;
      assert.strictEqual(stats.blocksProduced, 0);
    });

    it('should handle getValidators not wired', () => {
      // Should not throw
      assert.doesNotThrow(() => {
        producer._syncValidators();
      });

      // Should still add self
      const validators = producer.slotManager.validators;
      assert.ok(validators.length > 0);
    });

    it('should handle getValidators returning undefined', () => {
      producer.wire({ getValidators: () => undefined });

      assert.doesNotThrow(() => {
        producer._syncValidators();
      });

      const validators = producer.slotManager.validators;
      assert.ok(validators.length > 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // STATS TRACKING
  // ═══════════════════════════════════════════════════════════════════════

  describe('stats tracking', () => {
    it('should increment blocksProduced on successful proposal', () => {
      producer.wire({ proposeBlock: () => ({ id: 1 }) });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer._onSlot(1, false);
      producer._onSlot(2, false);

      const stats = producer.stats;
      assert.strictEqual(stats.blocksProduced, 2);
    });

    it('should increment emptyBlocks when no judgments', () => {
      producer.wire({ proposeBlock: () => ({ id: 1 }) });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer._onSlot(1, false);

      const stats = producer.stats;
      assert.strictEqual(stats.emptyBlocks, 1);
    });

    it('should NOT increment emptyBlocks when judgments present', () => {
      producer.wire({ proposeBlock: () => ({ id: 1 }) });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer._pendingJudgments.push({
        judgment_id: 'jdg_001',
        q_score: 50,
        verdict: 'BARK',
        timestamp: Date.now(),
      });

      producer._onSlot(1, false);

      const stats = producer.stats;
      assert.strictEqual(stats.emptyBlocks, 0);
    });

    it('should increment judgmentsIncluded by count in block', () => {
      producer.wire({ proposeBlock: () => ({ id: 1 }) });
      producer.slotManager.isLeader = mock.fn(() => true);

      // Add 3 judgments
      for (let i = 0; i < 3; i++) {
        producer._pendingJudgments.push({
          judgment_id: `jdg_${i}`,
          q_score: 50,
          verdict: 'BARK',
          timestamp: Date.now(),
        });
      }

      producer._onSlot(1, false);

      const stats = producer.stats;
      assert.strictEqual(stats.judgmentsIncluded, 3);
    });

    it('should return copy of stats (not reference)', () => {
      const stats1 = producer.stats;
      const stats2 = producer.stats;

      assert.notStrictEqual(stats1, stats2);
      assert.deepStrictEqual(stats1, stats2);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // INTEGRATION
  // ═══════════════════════════════════════════════════════════════════════

  describe('integration', () => {
    it('should produce blocks end-to-end', async () => {
      const blocks = [];
      const proposeBlock = (block) => {
        blocks.push(block);
        return { id: blocks.length };
      };

      const validators = [
        { publicKey: TEST_PUBLIC_KEY, eScore: 70, burned: 1000, uptime: 1.0 },
      ];

      producer.wire({ proposeBlock, getValidators: () => validators });

      // Add some judgments
      producer._pendingJudgments.push(
        { judgment_id: 'jdg_001', q_score: 60, verdict: 'GOOD', timestamp: Date.now() },
        { judgment_id: 'jdg_002', q_score: 50, verdict: 'BARK', timestamp: Date.now() }
      );

      // Mock leader
      producer.slotManager.isLeader = mock.fn(() => true);

      producer.start();

      // Wait for slot tick
      await new Promise(resolve => setTimeout(resolve, 100));

      producer.stop();

      assert.ok(blocks.length > 0);
      assert.strictEqual(blocks[0].judgment_count, 2);
      assert.deepStrictEqual(blocks[0].judgment_ids, ['jdg_001', 'jdg_002']);
    });

    it('should maintain block chain (prev_hash links)', async () => {
      const blocks = [];
      const proposeBlock = (block) => {
        blocks.push(block);
        return { id: blocks.length };
      };

      producer.wire({ proposeBlock });
      producer.slotManager.isLeader = mock.fn(() => true);

      producer._onSlot(1, false);
      producer._onSlot(2, false);
      producer._onSlot(3, false);

      assert.strictEqual(blocks.length, 3);
      assert.strictEqual(blocks[0].prev_hash, '0'.repeat(64)); // Genesis
      assert.strictEqual(blocks[1].prev_hash, blocks[0].hash);
      assert.strictEqual(blocks[2].prev_hash, blocks[1].hash);
    });

    it('should collect judgments while running', async () => {
      producer.wire({ proposeBlock: () => ({ id: 1 }) });
      producer.start();

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_runtime_001',
        payload: { qScore: 70, verdict: 'GOOD' },
      });

      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: 'jdg_runtime_002',
        payload: { qScore: 55, verdict: 'BARK' },
      });

      await new Promise(resolve => setTimeout(resolve, 10));

      assert.strictEqual(producer.pendingCount, 2);
    });
  });
});
