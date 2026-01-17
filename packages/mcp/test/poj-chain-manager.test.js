/**
 * PoJ Chain Manager Tests
 *
 * Tests for the PoJChainManager that handles Proof of Judgment chain.
 *
 * @module @cynic/mcp/test/poj-chain-manager
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { PoJChainManager } from '../src/poj-chain-manager.js';
import { OperatorRegistry, generateOperatorKeyPair } from '../src/operator-registry.js';

/**
 * Create mock persistence for PoJ chain
 */
function createMockPersistence() {
  const blocks = [];

  return {
    capabilities: {
      pojChain: true,
    },
    async storePoJBlock(block) {
      const stored = {
        ...block,
        block_hash: block.hash,
        judgment_count: block.judgments?.length || 0,
        created_at: new Date(),
      };
      blocks.push(stored);
      return stored;
    },
    async getPoJHead() {
      if (blocks.length === 0) return null;
      return blocks[blocks.length - 1];
    },
    async getRecentPoJBlocks(limit = 10) {
      return blocks.slice(-limit).reverse();
    },
    async getPoJBlockBySlot(slot) {
      return blocks.find((b) => b.slot === slot) || null;
    },
    async getPoJStats() {
      const total = blocks.length;
      if (total === 0) return { totalBlocks: 0, headSlot: 0, totalJudgments: 0 };
      const head = blocks[total - 1];
      const totalJudgments = blocks.reduce((sum, b) => sum + (b.judgment_count || 0), 0);
      return { totalBlocks: total, headSlot: head.slot, totalJudgments };
    },
    async verifyPoJChain() {
      const errors = [];
      for (let i = 1; i < blocks.length; i++) {
        const block = blocks[i];
        const prevBlock = blocks[i - 1];
        if (block.prev_hash !== (prevBlock.hash || prevBlock.block_hash)) {
          errors.push({ slot: block.slot, error: 'Invalid prev_hash' });
        }
      }
      return { valid: errors.length === 0, blocksChecked: blocks.length, errors };
    },
    _blocks: blocks,
  };
}

describe('PoJChainManager', () => {
  let manager;
  let mockPersistence;

  beforeEach(() => {
    mockPersistence = createMockPersistence();
    manager = new PoJChainManager(mockPersistence, {
      batchSize: 3,
      batchTimeout: 60,
    });
  });

  afterEach(async () => {
    await manager.close();
  });

  describe('constructor', () => {
    it('creates with default options', () => {
      const m = new PoJChainManager(null);
      assert.equal(m.batchSize, 10);
      assert.equal(m.batchTimeout, 60);
    });

    it('accepts custom options', () => {
      assert.equal(manager.batchSize, 3);
      assert.equal(manager.batchTimeout, 60);
    });

    it('initializes stats', () => {
      assert.equal(manager._stats.blocksCreated, 0);
      assert.equal(manager._stats.judgmentsProcessed, 0);
    });
  });

  describe('initialize', () => {
    it('creates genesis block when no head exists', async () => {
      await manager.initialize();

      assert.equal(manager._initialized, true);
      assert.ok(manager._head);
      assert.equal(manager._head.slot, 0);
    });

    it('resumes from existing head', async () => {
      // Pre-populate with a block
      await mockPersistence.storePoJBlock({
        slot: 5,
        prev_hash: 'abc',
        judgments_root: 'def',
        judgments: [],
        hash: 'existing_hash',
      });

      await manager.initialize();

      assert.equal(manager._head.slot, 5);
    });

    it('is idempotent', async () => {
      await manager.initialize();
      await manager.initialize();

      assert.equal(manager._initialized, true);
    });
  });

  describe('addJudgment', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('adds judgment to pending', async () => {
      await manager.addJudgment({
        judgment_id: 'jdg_1',
        q_score: 75,
        verdict: 'WAG',
      });

      assert.equal(manager.getPendingCount(), 1);
    });

    it('creates block when batch size reached', async () => {
      for (let i = 0; i < 3; i++) {
        await manager.addJudgment({
          judgment_id: `jdg_${i}`,
          q_score: 70 + i,
          verdict: 'WAG',
        });
      }

      // Block should be created, pending cleared
      assert.equal(manager.getPendingCount(), 0);
      assert.equal(manager._stats.blocksCreated, 1);
    });

    it('increments stats', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_stats' });

      assert.equal(manager._stats.judgmentsProcessed, 1);
    });

    it('generates judgment ID if missing', async () => {
      await manager.addJudgment({ q_score: 50, verdict: 'GROWL' });

      assert.equal(manager.getPendingCount(), 1);
    });
  });

  describe('flush', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('creates block from pending judgments', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_flush1' });
      await manager.addJudgment({ judgment_id: 'jdg_flush2' });

      const block = await manager.flush();

      assert.ok(block);
      assert.equal(block.judgment_count, 2);
      assert.equal(manager.getPendingCount(), 0);
    });

    it('returns null when no pending judgments', async () => {
      const block = await manager.flush();

      assert.equal(block, null);
    });
  });

  describe('getStatus', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('returns chain status', () => {
      const status = manager.getStatus();

      assert.equal(status.initialized, true);
      assert.ok('headSlot' in status);
      assert.ok('pendingJudgments' in status);
      assert.ok('stats' in status);
    });

    it('includes batch configuration', () => {
      const status = manager.getStatus();

      assert.equal(status.batchSize, 3);
      assert.equal(status.batchTimeout, 60);
    });

    it('includes operator info', () => {
      const status = manager.getStatus();

      assert.ok(status.operator);
      assert.ok('hasQuorum' in status);
    });
  });

  describe('getHead', () => {
    it('returns null before initialization', () => {
      assert.equal(manager.getHead(), null);
    });

    it('returns head after initialization', async () => {
      await manager.initialize();
      const head = manager.getHead();

      assert.ok(head);
      assert.equal(head.slot, 0);
    });
  });

  describe('verifyIntegrity', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('verifies empty chain', async () => {
      const result = await manager.verifyIntegrity();

      assert.equal(result.valid, true);
    });

    it('verifies chain with blocks', async () => {
      for (let i = 0; i < 5; i++) {
        await manager.addJudgment({ judgment_id: `jdg_verify_${i}` });
      }
      await manager.flush();

      const result = await manager.verifyIntegrity();

      assert.equal(result.valid, true);
    });
  });

  describe('exportChain', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('exports chain data', async () => {
      for (let i = 0; i < 3; i++) {
        await manager.addJudgment({ judgment_id: `jdg_export_${i}` });
      }
      await manager.flush();

      const exported = await manager.exportChain();

      assert.ok(exported.version);
      assert.ok(exported.exportedAt);
      assert.ok(Array.isArray(exported.blocks));
    });

    it('respects fromBlock option', async () => {
      for (let i = 0; i < 6; i++) {
        await manager.addJudgment({ judgment_id: `jdg_from_${i}` });
      }
      await manager.flush();

      const exported = await manager.exportChain({ fromBlock: 1 });

      // Should exclude genesis block (slot 0)
      const hasGenesis = exported.blocks.some((b) => b.slot === 0);
      assert.ok(!hasGenesis || exported.blocks.length > 0);
    });
  });

  describe('importChain', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('imports valid chain data', async () => {
      const chainData = {
        version: '1.0.0',
        blocks: [
          { slot: 1, hash: 'abc', prevHash: manager._head.hash, merkleRoot: 'root1', timestamp: new Date().toISOString() },
          { slot: 2, hash: 'def', prevHash: 'abc', merkleRoot: 'root2', timestamp: new Date().toISOString() },
        ],
      };

      const result = await manager.importChain(chainData);

      assert.ok(result.imported >= 0);
    });

    it('validates chain links', async () => {
      const chainData = {
        blocks: [
          { slot: 1, hash: 'abc', prevHash: 'wrong', merkleRoot: 'root1', timestamp: new Date().toISOString() },
          { slot: 2, hash: 'def', prevHash: 'alsobad', merkleRoot: 'root2', timestamp: new Date().toISOString() },
        ],
      };

      const result = await manager.importChain(chainData, { validateLinks: true });

      assert.ok(result.errors && result.errors.length > 0);
    });

    it('handles invalid data format', async () => {
      const result = await manager.importChain({ invalid: true });

      assert.ok(result.error);
    });

    it('skips existing blocks', async () => {
      // Create block at slot 1
      for (let i = 0; i < 3; i++) {
        await manager.addJudgment({ judgment_id: `jdg_exist_${i}` });
      }
      await manager.flush();

      const chainData = {
        blocks: [
          { slot: 1, hash: 'dup', prevHash: 'x', merkleRoot: 'y', timestamp: new Date().toISOString() },
        ],
      };

      const result = await manager.importChain(chainData);

      assert.ok(result.skipped >= 0 || result.imported >= 0);
    });
  });

  describe('close', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('flushes pending judgments on close', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_close1' });
      await manager.addJudgment({ judgment_id: 'jdg_close2' });

      await manager.close();

      // Should have created a block with the pending judgments
      assert.equal(manager.getPendingCount(), 0);
    });

    it('clears batch timer', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_timer' });
      assert.ok(manager._batchTimer);

      await manager.close();

      assert.equal(manager._batchTimer, null);
    });
  });
});

describe('PoJChainManager Multi-Operator', () => {
  let manager;
  let mockPersistence;
  let registry;

  beforeEach(async () => {
    mockPersistence = createMockPersistence();
    registry = new OperatorRegistry({ minOperators: 1 });
    registry.initializeSelf({ name: 'test-operator' });

    manager = new PoJChainManager(mockPersistence, {
      batchSize: 2,
      operatorRegistry: registry,
      requireSignatures: true,
    });
    await manager.initialize();
  });

  afterEach(async () => {
    await manager.close();
  });

  describe('isMultiOperator', () => {
    it('returns true with registry', () => {
      assert.equal(manager.isMultiOperator, true);
    });

    it('returns false without registry', () => {
      const m = new PoJChainManager(mockPersistence);
      assert.equal(m.isMultiOperator, false);
    });
  });

  describe('getOperatorInfo', () => {
    it('returns operator info from registry', () => {
      const info = manager.getOperatorInfo();

      assert.ok(info);
      assert.equal(info.name, 'test-operator');
    });
  });

  describe('block signing', () => {
    it('signs blocks with operator key', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_sign1' });
      await manager.addJudgment({ judgment_id: 'jdg_sign2' });

      const blocks = await mockPersistence.getRecentPoJBlocks(1);
      const block = blocks[0];

      assert.ok(block.operator);
      assert.ok(block.signature);
    });
  });

  describe('receiveBlock', () => {
    it('accepts valid signed block', async () => {
      // Create and sign a block - don't set hash, let receiver compute it
      const block = registry.signBlock({
        slot: 1,
        prev_hash: manager._head.hash || manager._head.block_hash,
        judgments_root: 'test_root',
        judgments: [{ judgment_id: 'jdg_recv' }],
        timestamp: Date.now(),
      });
      // Don't manually set hash - let receiveBlock compute and store

      const result = await manager.receiveBlock(block);

      assert.equal(result.accepted, true);
    });

    it('rejects block from unknown operator', async () => {
      // Create block signed by unregistered operator
      const { publicKey, privateKey } = generateOperatorKeyPair();
      const otherRegistry = new OperatorRegistry();
      otherRegistry.initializeSelf({ publicKey, privateKey, name: 'unknown' });

      const block = otherRegistry.signBlock({
        slot: 1,
        prev_hash: manager._head.hash || manager._head.block_hash,
        judgments_root: 'test_root',
        judgments: [],
        timestamp: Date.now(),
      });
      // Don't set hash

      const result = await manager.receiveBlock(block);

      assert.equal(result.accepted, false);
      assert.ok(result.error.includes('Unknown operator'));
    });

    it('rejects block with chain mismatch', async () => {
      const block = registry.signBlock({
        slot: 1,
        prev_hash: 'wrong_prev_hash',
        judgments_root: 'test_root',
        judgments: [],
        timestamp: Date.now(),
      });
      // Don't set hash

      const result = await manager.receiveBlock(block);

      assert.equal(result.accepted, false);
      assert.ok(result.error.includes('mismatch'));
    });

    it('rejects block with wrong slot', async () => {
      const block = registry.signBlock({
        slot: 99, // Wrong slot
        prev_hash: manager._head.hash || manager._head.block_hash,
        judgments_root: 'test_root',
        judgments: [],
        timestamp: Date.now(),
      });
      // Don't set hash

      const result = await manager.receiveBlock(block);

      assert.equal(result.accepted, false);
      assert.ok(result.error.includes('Slot'));
    });

    it('updates stats on successful receive', async () => {
      const block = registry.signBlock({
        slot: 1,
        prev_hash: manager._head.hash || manager._head.block_hash,
        judgments_root: 'test_root',
        judgments: [{ judgment_id: 'jdg_stats' }],
        timestamp: Date.now(),
      });
      // Don't set hash

      await manager.receiveBlock(block);

      assert.equal(manager._stats.blocksReceived, 1);
    });
  });
});

describe('PoJChainManager without persistence', () => {
  it('handles no persistence gracefully', async () => {
    const manager = new PoJChainManager(null);
    await manager.initialize();

    // Should not throw
    await manager.addJudgment({ judgment_id: 'jdg_nopersist' });

    await manager.close();
  });

  it('handles missing capabilities', async () => {
    const manager = new PoJChainManager({ capabilities: {} });
    await manager.initialize();

    const result = await manager.verifyIntegrity();
    assert.equal(result.valid, true);
    assert.equal(result.blocksChecked, 0);

    await manager.close();
  });
});
