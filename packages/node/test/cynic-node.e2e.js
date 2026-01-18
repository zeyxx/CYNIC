/**
 * CYNICNode Tests
 *
 * Tests for the main CYNIC node implementation.
 *
 * @module @cynic/node/test/cynic-node
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { CYNICNode, NodeStatus } from '../src/node.js';

describe('NodeStatus', () => {
  it('has all status values', () => {
    assert.equal(NodeStatus.STOPPED, 'STOPPED');
    assert.equal(NodeStatus.STARTING, 'STARTING');
    assert.equal(NodeStatus.RUNNING, 'RUNNING');
    assert.equal(NodeStatus.SYNCING, 'SYNCING');
    assert.equal(NodeStatus.STOPPING, 'STOPPING');
  });
});

describe('CYNICNode', () => {
  let node;

  beforeEach(() => {
    node = new CYNICNode({
      name: 'test-node',
      sendFn: async () => {}, // Mock send function
    });
  });

  afterEach(async () => {
    if (node && node.status !== NodeStatus.STOPPED) {
      await node.stop();
    }
  });

  describe('constructor', () => {
    it('creates node with default options', () => {
      const n = new CYNICNode();
      assert.ok(n.operator);
      assert.ok(n.state);
      assert.ok(n._judge);
      assert.ok(n.residualDetector);
      assert.ok(n.gossip);
      assert.equal(n.status, NodeStatus.STOPPED);
    });

    it('accepts name option', () => {
      assert.equal(node.operator.identity.name, 'test-node');
    });

    it('initializes with stopped status', () => {
      assert.equal(node.status, NodeStatus.STOPPED);
      assert.equal(node.startedAt, null);
    });

    it('initializes pending observations', () => {
      assert.ok(node._pendingObservations instanceof Map);
      assert.equal(node._pendingObservations.size, 0);
    });

    it('accepts custom limits', () => {
      const n = new CYNICNode({
        maxPendingItems: 500,
        maxObservationsPerItem: 25,
      });
      assert.equal(n._maxPendingItems, 500);
      assert.equal(n._maxObservationsPerItem, 25);
    });
  });

  describe('start', () => {
    it('starts node successfully', async () => {
      const result = await node.start();

      assert.ok(result.success);
      assert.ok(result.nodeId);
      assert.equal(result.name, 'test-node');
      assert.equal(node.status, NodeStatus.RUNNING);
      assert.ok(node.startedAt);
    });

    it('throws if node not stopped', async () => {
      await node.start();

      await assert.rejects(async () => {
        await node.start();
      }, /Cannot start node/);
    });

    it('starts timers', async () => {
      await node.start();

      assert.ok(node._epochTimer);
      assert.ok(node._cycleTimer);
    });
  });

  describe('stop', () => {
    it('stops running node', async () => {
      await node.start();
      await node.stop();

      assert.equal(node.status, NodeStatus.STOPPED);
    });

    it('clears timers on stop', async () => {
      await node.start();
      assert.ok(node._epochTimer);

      await node.stop();

      assert.equal(node._epochTimer, null);
      assert.equal(node._cycleTimer, null);
    });

    it('does nothing if already stopped', async () => {
      // Should not throw
      await node.stop();
      assert.equal(node.status, NodeStatus.STOPPED);
    });
  });

  describe('getInfo', () => {
    it('returns node info when stopped', () => {
      const info = node.getInfo();

      assert.ok(info.id);
      assert.equal(info.name, 'test-node');
      assert.equal(info.status, NodeStatus.STOPPED);
      assert.equal(info.uptime, 0);
    });

    it('returns node info when running', async () => {
      await node.start();
      // Wait a tiny bit for uptime
      await new Promise((r) => setTimeout(r, 10));

      const info = node.getInfo();

      assert.equal(info.status, NodeStatus.RUNNING);
      assert.ok(info.uptime > 0);
      assert.ok(info.operator);
      assert.ok(info.state);
      assert.ok(info.judge);
      assert.ok(info.gossip);
    });
  });

  describe('judge', () => {
    beforeEach(async () => {
      await node.start();
    });

    it('judges an item', async () => {
      const item = { content: 'test content', source: 'unit test' };
      const result = await node.judge(item);

      assert.ok(result);
      assert.ok(result.success);
      assert.ok(result.judgment);
      assert.ok(result.judgment.verdict);
      assert.ok(typeof result.judgment.qScore === 'number');
    });

    it('accepts context', async () => {
      const item = { content: 'test' };
      const result = await node.judge(item, { requestId: 'test123' });

      assert.ok(result);
    });

    it('updates operator stats', async () => {
      const beforeJudgments = node.operator.getPublicInfo().stats.judgmentsMade || 0;

      await node.judge({ content: 'test' });

      const afterJudgments = node.operator.getPublicInfo().stats.judgmentsMade || 0;
      assert.ok(afterJudgments > beforeJudgments);
    });
  });

  describe('chain and knowledge accessors', () => {
    beforeEach(async () => {
      await node.start();
    });

    it('provides chain accessor', () => {
      assert.ok(node.chain);
    });

    it('provides knowledge accessor', () => {
      assert.ok(node.knowledge);
    });
  });

  describe('addPeer', () => {
    beforeEach(async () => {
      await node.start();
    });

    it('adds peer to gossip and state', () => {
      const peerInfo = {
        id: 'peer123',
        address: 'localhost:8000',
        publicKey: 'pk_test',
      };

      // Should not throw
      node.addPeer(peerInfo);
    });
  });

  describe('_handleMessage', () => {
    beforeEach(async () => {
      await node.start();
    });

    it('handles JUDGMENT message', async () => {
      const message = {
        type: 'JUDGMENT',
        payload: {
          item_hash: 'hash123',
          score: 75,
          verdict: 'WAG',
        },
      };

      await node._handleMessage(message);

      assert.ok(node._pendingObservations.has('hash123'));
    });

    it('handles PATTERN message', async () => {
      const message = {
        type: 'PATTERN',
        payload: {
          id: 'pat123',
          content: { itemHash: 'hash' },
          axiom: 'VERIFY',
          strength: 0.5,
        },
      };

      // Should not throw
      await node._handleMessage(message);
    });
  });

  describe('_handleJudgment (pending observations)', () => {
    beforeEach(async () => {
      await node.start();
    });

    it('adds observation to pending', async () => {
      const judgment = {
        item_hash: 'item1',
        score: 80,
        verdict: 'WAG',
      };

      await node._handleJudgment(judgment);

      assert.ok(node._pendingObservations.has('item1'));
      const entry = node._pendingObservations.get('item1');
      assert.equal(entry.observations.length, 1);
    });

    it('accumulates observations for same item', async () => {
      await node._handleJudgment({ item_hash: 'item2', score: 70 });
      await node._handleJudgment({ item_hash: 'item2', score: 75 });
      await node._handleJudgment({ item_hash: 'item2', score: 80 });

      const entry = node._pendingObservations.get('item2');
      assert.equal(entry.observations.length, 3);
    });

    it('enforces per-item observation limit', async () => {
      node._maxObservationsPerItem = 3;

      for (let i = 0; i < 5; i++) {
        await node._handleJudgment({ item_hash: 'limited', score: 50 + i });
      }

      const entry = node._pendingObservations.get('limited');
      assert.equal(entry.observations.length, 3);
    });

    it('evicts LRU when at capacity', async () => {
      node._maxPendingItems = 3;

      // Add items
      await node._handleJudgment({ item_hash: 'oldest', score: 50 });
      await new Promise((r) => setTimeout(r, 5));
      await node._handleJudgment({ item_hash: 'middle', score: 60 });
      await new Promise((r) => setTimeout(r, 5));
      await node._handleJudgment({ item_hash: 'newest', score: 70 });

      // Should still have 3
      assert.equal(node._pendingObservations.size, 3);

      // Add one more, should evict oldest
      await node._handleJudgment({ item_hash: 'extra', score: 80 });

      assert.equal(node._pendingObservations.size, 3);
      assert.ok(!node._pendingObservations.has('oldest'));
      assert.ok(node._pendingObservations.has('extra'));
    });
  });

  describe('export/restore', () => {
    it('exports node state', async () => {
      await node.start();
      await node.judge({ content: 'test' });

      const exported = await node.export();

      assert.ok(exported.operator);
      // stateDir may be undefined if not configured
    });

    it('restores node from saved state', async () => {
      await node.start();
      const exported = await node.export();
      await node.stop();

      const restoredNode = await CYNICNode.restore(exported, {
        sendFn: async () => {},
      });

      assert.ok(restoredNode);
      assert.equal(restoredNode.status, NodeStatus.STOPPED);
    });
  });

  describe('_checkPatternEmergence', () => {
    beforeEach(async () => {
      await node.start();
    });

    it('checks pending observations for patterns', () => {
      // Add some observations
      node._pendingObservations.set('test_item', {
        observations: [
          { score: 70, source: 'a' },
          { score: 75, source: 'b' },
          { score: 72, source: 'c' },
        ],
        lastUpdated: Date.now(),
      });

      // Should not throw
      node._checkPatternEmergence();
    });
  });
});

describe('CYNICNode lifecycle', () => {
  it('full lifecycle: create -> start -> judge -> stop', async () => {
    const node = new CYNICNode({
      name: 'lifecycle-test',
      sendFn: async () => {},
    });

    // Create
    assert.equal(node.status, NodeStatus.STOPPED);

    // Start
    await node.start();
    assert.equal(node.status, NodeStatus.RUNNING);

    // Judge
    const result = await node.judge({ content: 'test item' });
    assert.ok(result.success);
    assert.ok(result.judgment.verdict);

    // Stop
    await node.stop();
    assert.equal(node.status, NodeStatus.STOPPED);
  });
});
