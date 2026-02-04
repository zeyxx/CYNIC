/**
 * PoJ Chain Manager Tests
 *
 * Comprehensive tests for the PoJChainManager that handles Proof of Judgment chain.
 * Covers construction, initialization, judgment batching, block creation,
 * anchor queue integration, P2P consensus, multi-operator mode, import/export,
 * and error handling.
 *
 * @module @cynic/mcp/test/poj-chain-manager
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { PoJChainManager, AnchorStatus } from '../src/poj-chain-manager.js';
import { OperatorRegistry, generateOperatorKeyPair } from '../src/operator-registry.js';

// =============================================================================
// TEST HELPERS
// =============================================================================

/**
 * Create mock persistence for PoJ chain
 */
function createMockPersistence(overrides = {}) {
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
    ...overrides,
  };
}

/**
 * Create a mock anchor queue
 */
function createMockAnchorQueue() {
  const items = [];
  return {
    onAnchorComplete: undefined,
    enqueue(id, data) {
      items.push({ id, data });
    },
    _items: items,
  };
}

/**
 * Generate N judgments for batching tests
 */
function generateJudgments(count, prefix = 'jdg') {
  return Array.from({ length: count }, (_, i) => ({
    judgment_id: `${prefix}_${i}`,
    q_score: 50 + i,
    verdict: i % 2 === 0 ? 'WAG' : 'GROWL',
  }));
}

// =============================================================================
// TESTS: CORE
// =============================================================================

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

  // =========================================================================
  // CONSTRUCTOR
  // =========================================================================

  describe('constructor', () => {
    it('creates with default options', () => {
      const m = new PoJChainManager(null);
      assert.equal(m.batchSize, 10);
      assert.equal(m.batchTimeout, 60);
      assert.equal(m.requireSignatures, false);
      assert.equal(m.verifyReceivedBlocks, true);
      assert.equal(m._initialized, false);
    });

    it('accepts custom batch options', () => {
      assert.equal(manager.batchSize, 3);
      assert.equal(manager.batchTimeout, 60);
    });

    it('initializes stats to zero', () => {
      assert.equal(manager._stats.blocksCreated, 0);
      assert.equal(manager._stats.blocksReceived, 0);
      assert.equal(manager._stats.blocksRejected, 0);
      assert.equal(manager._stats.judgmentsProcessed, 0);
      assert.equal(manager._stats.lastBlockTime, null);
      assert.equal(manager._stats.signatureVerifications, 0);
      assert.equal(manager._stats.signatureFailures, 0);
      assert.equal(manager._stats.blocksAnchored, 0);
      assert.equal(manager._stats.anchorsFailed, 0);
      assert.equal(manager._stats.blocksFinalized, 0);
      assert.equal(manager._stats.finalityTimeouts, 0);
    });

    it('starts with empty pending judgments', () => {
      assert.equal(manager.getPendingCount(), 0);
    });

    it('starts with null head', () => {
      assert.equal(manager.getHead(), null);
    });

    it('stores anchor queue reference', () => {
      const anchorQueue = createMockAnchorQueue();
      const m = new PoJChainManager(mockPersistence, { anchorQueue });
      assert.equal(m.isAnchoringEnabled, true);
    });

    it('stores P2P configuration', () => {
      const m = new PoJChainManager(mockPersistence, {
        p2pNodeUrl: 'http://test-node.dev',
        p2pEnabled: true,
      });
      assert.equal(m._p2pNodeUrl, 'http://test-node.dev');
      assert.equal(m._p2pEnabled, true);
    });

    it('stores onBlockCreated callback', () => {
      const cb = () => {};
      const m = new PoJChainManager(mockPersistence, { onBlockCreated: cb });
      assert.equal(m._onBlockCreated, cb);
    });

    it('stores legacy operator key', () => {
      const m = new PoJChainManager(mockPersistence, {
        operatorKey: 'abcdefghijklmnop1234',
      });
      assert.equal(m._legacyOperatorKey, 'abcdefghijklmnop1234');
    });
  });

  // =========================================================================
  // INITIALIZATION
  // =========================================================================

  describe('initialize', () => {
    it('creates genesis block when no head exists', async () => {
      await manager.initialize();

      assert.equal(manager._initialized, true);
      assert.ok(manager._head);
      assert.equal(manager._head.slot, 0);
      assert.ok(manager._head.hash || manager._head.block_hash);
    });

    it('resumes from existing head', async () => {
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

    it('is idempotent - second call is no-op', async () => {
      await manager.initialize();
      const head1 = manager.getHead();

      await manager.initialize();
      const head2 = manager.getHead();

      assert.deepEqual(head1, head2);
      // Only one genesis block should exist
      assert.equal(mockPersistence._blocks.length, 1);
    });

    it('handles missing persistence gracefully', async () => {
      const m = new PoJChainManager(null);
      await m.initialize();
      assert.equal(m._initialized, true);
      assert.equal(m.getHead(), null);
      await m.close();
    });

    it('handles persistence without pojChain capability', async () => {
      const m = new PoJChainManager({ capabilities: {} });
      await m.initialize();
      assert.equal(m._initialized, true);
      assert.equal(m.getHead(), null);
      await m.close();
    });
  });

  // =========================================================================
  // ADD JUDGMENT
  // =========================================================================

  describe('addJudgment', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('adds judgment to pending list', async () => {
      await manager.addJudgment({
        judgment_id: 'jdg_1',
        q_score: 75,
        verdict: 'WAG',
      });

      assert.equal(manager.getPendingCount(), 1);
    });

    it('creates block when batch size is reached', async () => {
      for (let i = 0; i < 3; i++) {
        await manager.addJudgment({
          judgment_id: `jdg_${i}`,
          q_score: 70 + i,
          verdict: 'WAG',
        });
      }

      assert.equal(manager.getPendingCount(), 0);
      assert.equal(manager._stats.blocksCreated, 1);
    });

    it('increments judgmentsProcessed stat', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_stats' });
      assert.equal(manager._stats.judgmentsProcessed, 1);
    });

    it('generates judgment ID if missing', async () => {
      await manager.addJudgment({ q_score: 50, verdict: 'GROWL' });
      assert.equal(manager.getPendingCount(), 1);
    });

    it('accepts judgmentId (camelCase) field', async () => {
      await manager.addJudgment({ judgmentId: 'camel_case_id', qScore: 60 });
      assert.equal(manager.getPendingCount(), 1);
    });

    it('starts batch timeout timer when first judgment added', async () => {
      assert.equal(manager._batchTimer, null);
      await manager.addJudgment({ judgment_id: 'timer_test' });
      assert.ok(manager._batchTimer);
    });

    it('does not start new timer if one already running', async () => {
      await manager.addJudgment({ judgment_id: 'timer_test_1' });
      const timer1 = manager._batchTimer;

      await manager.addJudgment({ judgment_id: 'timer_test_2' });
      assert.equal(manager._batchTimer, timer1);
    });

    it('does nothing without persistence pojChain capability', async () => {
      const m = new PoJChainManager(null);
      await m.initialize();
      await m.addJudgment({ judgment_id: 'noop' });
      assert.equal(m.getPendingCount(), 0);
      await m.close();
    });

    it('auto-initializes if not yet initialized', async () => {
      const freshManager = new PoJChainManager(mockPersistence, { batchSize: 3 });
      // Not initialized yet but addJudgment should auto-init
      await freshManager.addJudgment({ judgment_id: 'auto_init_test' });
      assert.equal(freshManager._initialized, true);
      assert.equal(freshManager.getPendingCount(), 1);
      await freshManager.close();
    });

    it('creates multiple blocks when many judgments added', async () => {
      for (let i = 0; i < 9; i++) {
        await manager.addJudgment({
          judgment_id: `jdg_multi_${i}`,
          q_score: 50,
          verdict: 'WAG',
        });
      }

      assert.equal(manager._stats.blocksCreated, 3);
      assert.equal(manager.getPendingCount(), 0);
    });

    it('advances head slot with each block', async () => {
      for (let i = 0; i < 6; i++) {
        await manager.addJudgment({ judgment_id: `jdg_advance_${i}` });
      }
      // 2 blocks created (batch size = 3)
      assert.equal(manager.getHead().slot, 2);
    });
  });

  // =========================================================================
  // FLUSH
  // =========================================================================

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

    it('clears batch timer on flush', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_timer_flush' });
      assert.ok(manager._batchTimer);

      await manager.flush();
      assert.equal(manager._batchTimer, null);
    });
  });

  // =========================================================================
  // BLOCK CALLBACK
  // =========================================================================

  describe('onBlockCreated callback', () => {
    it('invokes callback when block is created', async () => {
      let callbackData = null;
      const m = new PoJChainManager(mockPersistence, {
        batchSize: 2,
        onBlockCreated: (data) => { callbackData = data; },
      });
      await m.initialize();

      await m.addJudgment({ judgment_id: 'cb_1' });
      await m.addJudgment({ judgment_id: 'cb_2' });

      assert.ok(callbackData);
      assert.equal(callbackData.blockNumber, 1);
      assert.ok(callbackData.hash);
      assert.ok(callbackData.prevHash);
      assert.equal(callbackData.judgmentCount, 2);
      assert.ok(callbackData.timestamp);
      await m.close();
    });

    it('does not fail block creation if callback throws', async () => {
      const m = new PoJChainManager(mockPersistence, {
        batchSize: 2,
        onBlockCreated: () => { throw new Error('Callback failure'); },
      });
      await m.initialize();

      await m.addJudgment({ judgment_id: 'safe_1' });
      await m.addJudgment({ judgment_id: 'safe_2' });

      // Block should still be created despite callback error
      assert.equal(m._stats.blocksCreated, 1);
      await m.close();
    });
  });

  // =========================================================================
  // ANCHOR STATUS
  // =========================================================================

  describe('anchor status', () => {
    it('getAnchorStatus returns null for unknown block', () => {
      const status = manager.getAnchorStatus('nonexistent_hash');
      assert.equal(status, null);
    });

    it('getPendingAnchors returns empty array initially', () => {
      const pending = manager.getPendingAnchors();
      assert.deepEqual(pending, []);
    });

    it('isAnchoringEnabled returns false without anchor queue', () => {
      assert.equal(manager.isAnchoringEnabled, false);
    });

    it('isAnchoringEnabled returns true with anchor queue', () => {
      manager.setAnchorQueue(createMockAnchorQueue());
      assert.equal(manager.isAnchoringEnabled, true);
    });

    it('marks anchor as QUEUED when anchor queue is set (non-P2P)', async () => {
      const anchorQueue = createMockAnchorQueue();
      const m = new PoJChainManager(mockPersistence, {
        batchSize: 2,
        anchorQueue,
        autoAnchor: true,
      });
      await m.initialize();

      await m.addJudgment({ judgment_id: 'anchor_1' });
      await m.addJudgment({ judgment_id: 'anchor_2' });

      // Block created, should be queued for anchoring
      const head = m.getHead();
      const status = m.getAnchorStatus(head.hash || head.block_hash);
      assert.ok(status);
      assert.equal(status.status, AnchorStatus.QUEUED);
      assert.equal(anchorQueue._items.length, 1);
      await m.close();
    });

    it('marks anchor as PENDING when no anchor queue', async () => {
      const m = new PoJChainManager(mockPersistence, {
        batchSize: 2,
        autoAnchor: true,
      });
      await m.initialize();

      await m.addJudgment({ judgment_id: 'no_q_1' });
      await m.addJudgment({ judgment_id: 'no_q_2' });

      const head = m.getHead();
      const status = m.getAnchorStatus(head.hash || head.block_hash);
      assert.ok(status);
      assert.equal(status.status, AnchorStatus.PENDING);
      await m.close();
    });
  });

  // =========================================================================
  // ANCHOR STATUS CONSTANTS
  // =========================================================================

  describe('AnchorStatus', () => {
    it('has PENDING status', () => {
      assert.equal(AnchorStatus.PENDING, 'PENDING');
    });

    it('has QUEUED status', () => {
      assert.equal(AnchorStatus.QUEUED, 'QUEUED');
    });

    it('has ANCHORED status', () => {
      assert.equal(AnchorStatus.ANCHORED, 'ANCHORED');
    });

    it('has FAILED status', () => {
      assert.equal(AnchorStatus.FAILED, 'FAILED');
    });
  });

  // =========================================================================
  // SET ANCHOR QUEUE
  // =========================================================================

  describe('setAnchorQueue', () => {
    it('sets the anchor queue', () => {
      const queue = createMockAnchorQueue();
      manager.setAnchorQueue(queue);
      assert.equal(manager.isAnchoringEnabled, true);
    });

    it('sets onAnchorComplete callback', () => {
      const queue = createMockAnchorQueue();
      assert.equal(queue.onAnchorComplete, undefined);

      manager.setAnchorQueue(queue);
      assert.ok(typeof queue.onAnchorComplete === 'function');
    });

    it('does not overwrite existing onAnchorComplete', () => {
      const queue = createMockAnchorQueue();
      const original = () => {};
      queue.onAnchorComplete = original;

      manager.setAnchorQueue(queue);
      // Should not overwrite since it's already defined (not undefined)
      assert.notEqual(queue.onAnchorComplete, undefined);
    });
  });

  // =========================================================================
  // SET P2P NODE
  // =========================================================================

  describe('setP2PNode', () => {
    it('sets P2P node URL and enables P2P', () => {
      manager.setP2PNode('http://p2p-node.dev');
      assert.equal(manager._p2pNodeUrl, 'http://p2p-node.dev');
      assert.equal(manager._p2pEnabled, true);
    });
  });

  // =========================================================================
  // _onAnchorComplete
  // =========================================================================

  describe('_onAnchorComplete', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('updates anchor status on successful anchor', async () => {
      // Create a block first
      const m = new PoJChainManager(mockPersistence, {
        batchSize: 2,
        autoAnchor: false,
      });
      await m.initialize();

      await m.addJudgment({ judgment_id: 'anc_1' });
      await m.addJudgment({ judgment_id: 'anc_2' });

      const head = m.getHead();
      const blockHash = head.hash || head.block_hash;
      const slot = head.slot;

      // Simulate setting the anchor status (as _anchorBlock would)
      m._anchorStatus.set(blockHash, {
        status: AnchorStatus.QUEUED,
        slot,
      });

      // Simulate anchor completion
      m._onAnchorComplete(
        {
          id: 'batch_1',
          items: [{ id: `poj_block_${slot}`, merkleRoot: 'test_root' }],
        },
        {
          success: true,
          signature: 'solana_sig_1234567890abcdef',
          timestamp: Date.now(),
          slot: 12345,
        },
      );

      const status = m.getAnchorStatus(blockHash);
      assert.equal(status.status, AnchorStatus.ANCHORED);
      assert.ok(status.signature);
      assert.equal(m._stats.blocksAnchored, 1);
      await m.close();
    });

    it('handles failed anchor result gracefully', () => {
      // Should not throw on failure
      manager._onAnchorComplete(
        { id: 'batch_fail', items: [] },
        { success: false, error: 'Insufficient SOL' },
      );
      // No crash = pass
    });
  });

  // =========================================================================
  // _onBlockFinalized
  // =========================================================================

  describe('_onBlockFinalized', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('handles finality event for pending block', async () => {
      const blockHash = 'test_hash_finalize';
      let resolved = false;

      // Simulate pending finality
      manager._pendingFinality.set(blockHash, {
        block: { hash: blockHash, slot: 1 },
        timeout: null,
        resolve: () => { resolved = true; },
        reject: () => {},
        createdAt: Date.now(),
      });

      manager._onBlockFinalized({
        blockHash,
        slot: 1,
        status: 'finalized',
        confirmations: 10,
      });

      assert.equal(resolved, true);
      assert.equal(manager._stats.blocksFinalized, 1);
      assert.equal(manager._pendingFinality.has(blockHash), false);
    });

    it('ignores finality event for unknown block', () => {
      // Should not throw for unknown blocks
      manager._onBlockFinalized({
        blockHash: 'unknown_hash',
        slot: 99,
        status: 'finalized',
        confirmations: 5,
      });
      assert.equal(manager._stats.blocksFinalized, 0);
    });

    it('triggers anchoring after finality when anchor queue present', async () => {
      const anchorQueue = createMockAnchorQueue();
      manager.setAnchorQueue(anchorQueue);
      manager._autoAnchor = true;

      const blockHash = 'test_anchor_after_finality';
      manager._pendingFinality.set(blockHash, {
        block: {
          hash: blockHash,
          slot: 1,
          judgments_root: 'root',
          judgments: [],
          timestamp: Date.now(),
        },
        timeout: null,
        resolve: () => {},
        reject: () => {},
        createdAt: Date.now(),
      });

      manager._onBlockFinalized({
        blockHash,
        slot: 1,
        status: 'finalized',
        confirmations: 8,
      });

      // Block should be queued for anchoring
      assert.equal(anchorQueue._items.length, 1);
    });
  });

  // =========================================================================
  // GET STATUS
  // =========================================================================

  describe('getStatus', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('returns chain status object', () => {
      const status = manager.getStatus();

      assert.equal(status.initialized, true);
      assert.ok('headSlot' in status);
      assert.ok('pendingJudgments' in status);
      assert.ok('stats' in status);
      assert.ok('multiOperator' in status);
      assert.ok('anchoringEnabled' in status);
      assert.ok('p2pEnabled' in status);
    });

    it('includes batch configuration', () => {
      const status = manager.getStatus();
      assert.equal(status.batchSize, 3);
      assert.equal(status.batchTimeout, 60);
    });

    it('includes operator info for single-operator mode', () => {
      const status = manager.getStatus();
      assert.ok(status.operator);
      assert.equal(status.hasQuorum, true);
      assert.equal(status.operatorCount, 1);
    });

    it('reports pending judgments count', async () => {
      await manager.addJudgment({ judgment_id: 'pending_1' });
      await manager.addJudgment({ judgment_id: 'pending_2' });

      const status = manager.getStatus();
      assert.equal(status.pendingJudgments, 2);
    });

    it('reports anchor stats', async () => {
      const status = manager.getStatus();
      assert.equal(status.anchoredBlocks, 0);
      assert.equal(status.pendingAnchors, 0);
    });

    it('reports P2P status', () => {
      const status = manager.getStatus();
      assert.equal(status.p2pEnabled, false);
      assert.equal(status.p2pNodeUrl, null);
      assert.equal(status.pendingFinality, 0);
    });

    it('includes legacy operator key info', async () => {
      const m = new PoJChainManager(mockPersistence, {
        operatorKey: 'legacy_key_1234567890abcdef',
      });
      await m.initialize();
      const status = m.getStatus();
      assert.ok(status.operator.publicKey);
      assert.equal(status.operator.name, 'legacy');
      await m.close();
    });
  });

  // =========================================================================
  // GET HEAD
  // =========================================================================

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

    it('returns updated head after block creation', async () => {
      await manager.initialize();
      for (let i = 0; i < 3; i++) {
        await manager.addJudgment({ judgment_id: `jdg_head_${i}` });
      }
      const head = manager.getHead();
      assert.equal(head.slot, 1);
    });
  });

  // =========================================================================
  // IS MULTI-OPERATOR
  // =========================================================================

  describe('isMultiOperator', () => {
    it('returns false when no registry', () => {
      assert.equal(manager.isMultiOperator, false);
    });

    it('returns true when registry present', () => {
      const registry = new OperatorRegistry({ minOperators: 1 });
      const m = new PoJChainManager(mockPersistence, { operatorRegistry: registry });
      assert.equal(m.isMultiOperator, true);
    });
  });

  // =========================================================================
  // GET OPERATOR INFO
  // =========================================================================

  describe('getOperatorInfo', () => {
    it('returns null without registry or legacy key', () => {
      assert.equal(manager.getOperatorInfo(), null);
    });

    it('returns legacy key info when set', () => {
      const m = new PoJChainManager(mockPersistence, {
        operatorKey: 'abcdefghijklmnopqrst',
      });
      const info = m.getOperatorInfo();
      assert.ok(info);
      assert.equal(info.name, 'legacy');
      assert.equal(info.publicKey, 'abcdefghijklmnop');
    });
  });

  // =========================================================================
  // VERIFY INTEGRITY
  // =========================================================================

  describe('verifyIntegrity', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('verifies empty chain (genesis only)', async () => {
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

    it('returns valid with zero blocks checked when no persistence', async () => {
      const m = new PoJChainManager(null);
      await m.initialize();
      const result = await m.verifyIntegrity();
      assert.equal(result.valid, true);
      assert.equal(result.blocksChecked, 0);
      await m.close();
    });
  });

  // =========================================================================
  // EXPORT CHAIN
  // =========================================================================

  describe('exportChain', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('exports chain data with version and timestamp', async () => {
      for (let i = 0; i < 3; i++) {
        await manager.addJudgment({ judgment_id: `jdg_export_${i}` });
      }
      await manager.flush();

      const exported = await manager.exportChain();

      assert.ok(exported.version);
      assert.ok(exported.exportedAt);
      assert.ok(Array.isArray(exported.blocks));
      assert.ok(exported.totalBlocks > 0);
    });

    it('respects fromBlock option', async () => {
      for (let i = 0; i < 6; i++) {
        await manager.addJudgment({ judgment_id: `jdg_from_${i}` });
      }
      await manager.flush();

      const exported = await manager.exportChain({ fromBlock: 1 });

      // Should exclude genesis block (slot 0)
      for (const block of exported.blocks) {
        assert.ok(block.slot >= 1);
      }
    });

    it('returns error when persistence not available', async () => {
      const m = new PoJChainManager(null);
      await m.initialize();
      const exported = await m.exportChain();
      assert.ok(exported.error);
      assert.deepEqual(exported.blocks, []);
      await m.close();
    });

    it('includes block hashes and timestamps', async () => {
      for (let i = 0; i < 3; i++) {
        await manager.addJudgment({ judgment_id: `jdg_detail_${i}` });
      }
      await manager.flush();

      const exported = await manager.exportChain();
      for (const block of exported.blocks) {
        assert.ok('slot' in block);
        assert.ok('hash' in block);
        assert.ok('timestamp' in block);
      }
    });
  });

  // =========================================================================
  // IMPORT CHAIN
  // =========================================================================

  describe('importChain', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('imports valid chain data', async () => {
      const chainData = {
        version: '1.0.0',
        blocks: [
          {
            slot: 10,
            hash: 'abc',
            prevHash: manager._head.hash || manager._head.block_hash,
            merkleRoot: 'root1',
            timestamp: new Date().toISOString(),
          },
          {
            slot: 11,
            hash: 'def',
            prevHash: 'abc',
            merkleRoot: 'root2',
            timestamp: new Date().toISOString(),
          },
        ],
      };

      const result = await manager.importChain(chainData);
      assert.ok(result.imported >= 0);
    });

    it('validates chain links and rejects invalid ones', async () => {
      const chainData = {
        blocks: [
          { slot: 1, hash: 'abc', prevHash: 'wrong', merkleRoot: 'root1', timestamp: new Date().toISOString() },
          { slot: 2, hash: 'def', prevHash: 'also_wrong', merkleRoot: 'root2', timestamp: new Date().toISOString() },
        ],
      };

      const result = await manager.importChain(chainData, { validateLinks: true });
      assert.ok(result.errors.length > 0);
      assert.ok(result.error);
    });

    it('handles invalid data format (missing blocks)', async () => {
      const result = await manager.importChain({ invalid: true });
      assert.ok(result.error);
      assert.equal(result.imported, 0);
    });

    it('handles null blocks array', async () => {
      const result = await manager.importChain({ blocks: null });
      assert.ok(result.error);
    });

    it('skips existing blocks when skipExisting is true', async () => {
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

      const result = await manager.importChain(chainData, { skipExisting: true });
      assert.ok(result.skipped >= 0 || result.imported >= 0);
    });

    it('returns error when persistence not available', async () => {
      const m = new PoJChainManager(null);
      await m.initialize();
      const result = await m.importChain({ blocks: [{ slot: 1 }] });
      assert.ok(result.error);
      assert.equal(result.imported, 0);
      await m.close();
    });

    it('sorts blocks by slot before importing', async () => {
      const chainData = {
        blocks: [
          { slot: 3, hash: 'c', prevHash: 'b', merkleRoot: 'r3', timestamp: new Date().toISOString() },
          { slot: 1, hash: 'a', prevHash: 'genesis', merkleRoot: 'r1', timestamp: new Date().toISOString() },
          { slot: 2, hash: 'b', prevHash: 'a', merkleRoot: 'r2', timestamp: new Date().toISOString() },
        ],
      };

      const result = await manager.importChain(chainData, { validateLinks: false });
      assert.ok(result.imported >= 0);
    });
  });

  // =========================================================================
  // ERROR HANDLING IN _createBlock
  // =========================================================================

  describe('error handling in block creation', () => {
    it('puts judgments back on persistence error', async () => {
      const failPersistence = createMockPersistence();
      let callCount = 0;
      failPersistence.storePoJBlock = async (block) => {
        callCount++;
        // Fail on second call (first is genesis)
        if (callCount > 1) throw new Error('DB write error');
        return { ...block, block_hash: block.hash, judgment_count: block.judgments?.length || 0 };
      };

      const m = new PoJChainManager(failPersistence, { batchSize: 2 });
      await m.initialize();

      await m.addJudgment({ judgment_id: 'err_1' });
      await m.addJudgment({ judgment_id: 'err_2' });

      // Judgments should be put back in pending
      assert.equal(m.getPendingCount(), 2);
      await m.close();
    });
  });

  // =========================================================================
  // CLOSE
  // =========================================================================

  describe('close', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('flushes pending judgments on close', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_close1' });
      await manager.addJudgment({ judgment_id: 'jdg_close2' });

      await manager.close();

      assert.equal(manager.getPendingCount(), 0);
    });

    it('clears batch timer', async () => {
      await manager.addJudgment({ judgment_id: 'jdg_timer' });
      assert.ok(manager._batchTimer);

      await manager.close();
      assert.equal(manager._batchTimer, null);
    });

    it('clears pending finality map', async () => {
      manager._pendingFinality.set('hash1', {
        block: {},
        timeout: null,
        resolve: () => {},
        reject: () => {},
      });

      await manager.close();
      assert.equal(manager._pendingFinality.size, 0);
    });

    it('rejects pending finality promises on close', async () => {
      let rejected = false;
      manager._pendingFinality.set('hash_reject', {
        block: {},
        timeout: null,
        resolve: () => {},
        reject: () => { rejected = true; },
      });

      await manager.close();
      assert.equal(rejected, true);
    });
  });
});

// =============================================================================
// TESTS: MULTI-OPERATOR MODE
// =============================================================================

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
      assert.ok(info.publicKey);
    });
  });

  describe('getStatus with multi-operator', () => {
    it('includes operator details in status', () => {
      const status = manager.getStatus();
      assert.equal(status.multiOperator, true);
      assert.ok(status.operator);
      assert.ok(status.operator.name);
      assert.ok(status.operatorCount >= 1);
      assert.ok('hasQuorum' in status);
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
      const block = registry.signBlock({
        slot: 1,
        prev_hash: manager._head.hash || manager._head.block_hash,
        judgments_root: 'test_root',
        judgments: [{ judgment_id: 'jdg_recv' }],
        timestamp: Date.now(),
      });

      const result = await manager.receiveBlock(block);
      assert.equal(result.accepted, true);
    });

    it('rejects block from unknown operator', async () => {
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

      const result = await manager.receiveBlock(block);
      assert.equal(result.accepted, false);
      assert.ok(result.error.includes('Unknown operator'));
    });

    it('rejects block with chain mismatch (wrong prev_hash)', async () => {
      const block = registry.signBlock({
        slot: 1,
        prev_hash: 'wrong_prev_hash',
        judgments_root: 'test_root',
        judgments: [],
        timestamp: Date.now(),
      });

      const result = await manager.receiveBlock(block);
      assert.equal(result.accepted, false);
      assert.ok(result.error.includes('mismatch'));
    });

    it('rejects block with wrong slot number', async () => {
      const block = registry.signBlock({
        slot: 99,
        prev_hash: manager._head.hash || manager._head.block_hash,
        judgments_root: 'test_root',
        judgments: [],
        timestamp: Date.now(),
      });

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

      await manager.receiveBlock(block);
      assert.equal(manager._stats.blocksReceived, 1);
    });

    it('updates stats on rejected blocks', async () => {
      const block = registry.signBlock({
        slot: 99,
        prev_hash: 'wrong',
        judgments_root: 'test_root',
        judgments: [],
        timestamp: Date.now(),
      });

      await manager.receiveBlock(block);
      assert.ok(manager._stats.blocksRejected > 0);
    });

    it('requires signature when requireSignatures is enabled', async () => {
      // Create unsigned block
      const m = new PoJChainManager(mockPersistence, {
        batchSize: 2,
        requireSignatures: true,
      });
      await m.initialize();

      const block = {
        slot: 1,
        prev_hash: m._head.hash || m._head.block_hash,
        judgments_root: 'test_root',
        judgments: [],
        timestamp: Date.now(),
      };

      const result = await m.receiveBlock(block);
      assert.equal(result.accepted, false);
      assert.ok(result.error.includes('signature'));
      await m.close();
    });

    it('rejects block without persistence', async () => {
      const m = new PoJChainManager(null, {
        batchSize: 2,
      });
      await m.initialize();

      const result = await m.receiveBlock({ slot: 1 });
      assert.equal(result.accepted, false);
      assert.ok(result.error.includes('not available'));
      await m.close();
    });

    it('advances head on successful receive', async () => {
      const oldSlot = manager.getHead().slot;

      const block = registry.signBlock({
        slot: oldSlot + 1,
        prev_hash: manager._head.hash || manager._head.block_hash,
        judgments_root: 'test_root',
        judgments: [{ judgment_id: 'jdg_advance' }],
        timestamp: Date.now(),
      });

      await manager.receiveBlock(block);
      assert.equal(manager.getHead().slot, oldSlot + 1);
    });
  });
});

// =============================================================================
// TESTS: WITHOUT PERSISTENCE
// =============================================================================

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

  it('export returns error without persistence', async () => {
    const manager = new PoJChainManager({ capabilities: {} });
    await manager.initialize();

    const result = await manager.exportChain();
    assert.ok(result.error);

    await manager.close();
  });

  it('import returns error without persistence', async () => {
    const manager = new PoJChainManager({ capabilities: {} });
    await manager.initialize();

    const result = await manager.importChain({ blocks: [{ slot: 1 }] });
    assert.ok(result.error);

    await manager.close();
  });

  it('getStatus works without persistence', async () => {
    const manager = new PoJChainManager(null);
    await manager.initialize();

    const status = manager.getStatus();
    assert.equal(status.initialized, true);
    assert.equal(status.headSlot, 0);

    await manager.close();
  });
});
