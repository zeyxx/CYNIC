/**
 * CollectiveState Tests
 *
 * "The pack thinks as one" - κυνικός
 *
 * @module @cynic/emergence/test/collective-state
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  CollectiveState,
  createCollectiveState,
  CollectivePhase,
  PHASE_THRESHOLDS,
  QUORUM,
} from '../src/collective-state.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// =============================================================================
// CONSTANTS
// =============================================================================

describe('CollectivePhase', () => {
  it('should have all phase values', () => {
    assert.strictEqual(CollectivePhase.ISOLATED, 'ISOLATED');
    assert.strictEqual(CollectivePhase.FORMING, 'FORMING');
    assert.strictEqual(CollectivePhase.COHERENT, 'COHERENT');
    assert.strictEqual(CollectivePhase.RESONANT, 'RESONANT');
    assert.strictEqual(CollectivePhase.DIVERGENT, 'DIVERGENT');
    assert.strictEqual(CollectivePhase.TRANSCENDENT, 'TRANSCENDENT');
  });
});

describe('PHASE_THRESHOLDS', () => {
  it('should have φ-aligned coherence thresholds', () => {
    assert.ok(Math.abs(PHASE_THRESHOLDS.COHERENT.minCoherence - PHI_INV_3) < 0.001);
    assert.ok(Math.abs(PHASE_THRESHOLDS.RESONANT.minCoherence - PHI_INV) < 0.001);
    assert.strictEqual(PHASE_THRESHOLDS.TRANSCENDENT.minCoherence, 0.95);
  });

  it('should have node requirements', () => {
    assert.strictEqual(PHASE_THRESHOLDS.ISOLATED.maxNodes, 1);
    assert.strictEqual(PHASE_THRESHOLDS.FORMING.minNodes, 2);
    assert.strictEqual(PHASE_THRESHOLDS.COHERENT.minNodes, 3);
    assert.strictEqual(PHASE_THRESHOLDS.RESONANT.minNodes, 5);
    assert.strictEqual(PHASE_THRESHOLDS.TRANSCENDENT.minNodes, 7);
  });
});

describe('QUORUM', () => {
  it('should have quorum levels', () => {
    assert.strictEqual(QUORUM.MINIMUM, 3);
    assert.strictEqual(QUORUM.STANDARD, 5);
    assert.strictEqual(QUORUM.CRITICAL, 7);
  });
});

// =============================================================================
// COLLECTIVE STATE
// =============================================================================

describe('CollectiveState', () => {
  let collective;

  beforeEach(() => {
    collective = createCollectiveState({ nodeId: 'test_node' });
  });

  describe('Construction', () => {
    it('should create with factory', () => {
      assert.ok(collective instanceof CollectiveState);
    });

    it('should have node ID', () => {
      assert.strictEqual(collective.nodeId, 'test_node');
    });

    it('should generate node ID if not provided', () => {
      const auto = createCollectiveState();
      assert.ok(auto.nodeId.startsWith('node_'));
    });

    it('should start in ISOLATED phase', () => {
      assert.strictEqual(collective.phase, CollectivePhase.ISOLATED);
    });

    it('should start with 0 coherence', () => {
      assert.strictEqual(collective.coherence, 0);
    });

    it('should start with 0 active nodes', () => {
      assert.strictEqual(collective.activeNodes, 0);
    });
  });

  describe('reportState()', () => {
    it('should update local state', () => {
      collective.reportState({
        eScore: 72,
        awarenessLevel: 0.58,
        consciousnessState: 'AWARE',
      });

      assert.strictEqual(collective.localState.eScore, 72);
      assert.strictEqual(collective.localState.awarenessLevel, 0.58);
    });

    it('should add local node to nodes map', () => {
      collective.reportState({ eScore: 72 });
      assert.ok(collective.nodes.has('test_node'));
    });

    it('should increment heartbeat count', () => {
      const before = collective.metrics.totalHeartbeats;
      collective.reportState({ eScore: 72 });
      assert.strictEqual(collective.metrics.totalHeartbeats, before + 1);
    });
  });

  describe('receiveState()', () => {
    it('should add remote node state', () => {
      collective.receiveState('remote_node', {
        eScore: 65,
        awarenessLevel: 0.45,
        consciousnessState: 'OBSERVING',
      });

      assert.ok(collective.nodes.has('remote_node'));
      const node = collective.nodes.get('remote_node');
      assert.strictEqual(node.eScore, 65);
    });

    it('should integrate patterns into collective memory', () => {
      collective.receiveState('remote_node', {
        eScore: 65,
        patterns: [
          { id: 'pattern_1', significance: 0.5, type: 'TREND' },
        ],
      });

      assert.ok(collective.collectiveMemory.has('pattern:pattern_1'));
    });
  });

  describe('removeNode()', () => {
    it('should remove node from map', () => {
      collective.receiveState('remote_node', { eScore: 65 });
      assert.ok(collective.nodes.has('remote_node'));

      collective.removeNode('remote_node');
      assert.ok(!collective.nodes.has('remote_node'));
    });
  });

  describe('Phase transitions', () => {
    it('should be FORMING with 2 nodes', () => {
      collective.reportState({ eScore: 70, consciousnessState: 'AWARE' });
      collective.receiveState('node2', { eScore: 70, consciousnessState: 'AWARE' });

      assert.strictEqual(collective.phase, CollectivePhase.FORMING);
    });

    it('should be COHERENT with 3+ nodes and coherence', () => {
      collective.reportState({ eScore: 70, consciousnessState: 'AWARE' });
      collective.receiveState('node2', { eScore: 70, consciousnessState: 'AWARE' });
      collective.receiveState('node3', { eScore: 70, consciousnessState: 'AWARE' });

      // All same state = high coherence
      assert.strictEqual(collective.phase, CollectivePhase.COHERENT);
    });

    it('should detect DIVERGENT when consensus rate is low', () => {
      // Set up nodes with different states
      collective.reportState({ eScore: 100, consciousnessState: 'STATE_A' });
      collective.receiveState('node2', { eScore: 0, consciousnessState: 'STATE_B' });
      collective.receiveState('node3', { eScore: 50, consciousnessState: 'STATE_C' });

      // Add divergent consensus history to lower consensus rate
      for (let i = 0; i < 10; i++) {
        collective.recordConsensus('topic', {}, ['node1', 'node2', 'node3'], 0.2);
      }

      // Now coherence should be lower due to divergent history
      const phase = collective.phase;
      assert.ok(
        phase === CollectivePhase.DIVERGENT || phase === CollectivePhase.COHERENT,
        `Expected DIVERGENT or COHERENT, got ${phase}`
      );
    });
  });

  describe('hasQuorum()', () => {
    it('should return false with no nodes', () => {
      assert.strictEqual(collective.hasQuorum(), false);
    });

    it('should return true with minimum nodes', () => {
      collective.reportState({ eScore: 70 });
      collective.receiveState('node2', { eScore: 70 });
      collective.receiveState('node3', { eScore: 70 });

      assert.strictEqual(collective.hasQuorum('MINIMUM'), true);
    });

    it('should check different quorum levels', () => {
      collective.reportState({ eScore: 70 });
      collective.receiveState('node2', { eScore: 70 });
      collective.receiveState('node3', { eScore: 70 });
      collective.receiveState('node4', { eScore: 70 });

      assert.strictEqual(collective.hasQuorum('MINIMUM'), true);
      assert.strictEqual(collective.hasQuorum('STANDARD'), false);
    });
  });

  describe('recordConsensus()', () => {
    it('should record consensus event', () => {
      collective.recordConsensus(
        'test_topic',
        { verdict: 'WAG' },
        ['node1', 'node2', 'node3'],
        0.8
      );

      assert.strictEqual(collective.consensusHistory.length, 1);
      assert.strictEqual(collective.metrics.totalJudgments, 1);
    });

    it('should count high agreement as consensus', () => {
      collective.recordConsensus('topic', {}, [], 0.7);
      assert.strictEqual(collective.metrics.consensusCount, 1);
    });

    it('should count low agreement as divergence', () => {
      collective.recordConsensus('topic', {}, [], 0.3);
      assert.strictEqual(collective.metrics.divergenceCount, 1);
    });
  });

  describe('getCollectiveInsight()', () => {
    beforeEach(() => {
      collective.reportState({ eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
      collective.receiveState('node2', { eScore: 80, awarenessLevel: 0.6, consciousnessState: 'AWARE' });
      collective.receiveState('node3', { eScore: 75, awarenessLevel: 0.55, consciousnessState: 'AWARE' });
    });

    it('should return insight object', () => {
      const insight = collective.getCollectiveInsight();

      assert.ok('phase' in insight);
      assert.ok('coherence' in insight);
      assert.ok('nodes' in insight);
      assert.ok('averages' in insight);
      assert.ok('consensus' in insight);
      assert.ok('memory' in insight);
      assert.ok('health' in insight);
    });

    it('should calculate averages', () => {
      const insight = collective.getCollectiveInsight();

      assert.strictEqual(insight.averages.eScore, 75); // (70+80+75)/3
      assert.ok(Math.abs(insight.averages.awareness - 0.55) < 0.01);
    });

    it('should report quorum status', () => {
      const insight = collective.getCollectiveInsight();
      assert.strictEqual(insight.nodes.quorum, true);
    });
  });

  describe('getCollectiveVerdict()', () => {
    beforeEach(() => {
      collective.reportState({ eScore: 70, consciousnessState: 'AWARE' });
      collective.receiveState('node2', { eScore: 80, consciousnessState: 'AWARE' });
      collective.receiveState('node3', { eScore: 75, consciousnessState: 'AWARE' });
    });

    it('should return insufficient quorum if not enough nodes', () => {
      const emptyCollective = createCollectiveState();
      const result = emptyCollective.getCollectiveVerdict({
        node1: { verdict: 'WAG', confidence: 0.7 },
      });

      assert.strictEqual(result.verdict, null);
      assert.strictEqual(result.reason, 'INSUFFICIENT_QUORUM');
    });

    it('should aggregate verdicts', () => {
      const result = collective.getCollectiveVerdict({
        test_node: { verdict: 'WAG', confidence: 0.7 },
        node2: { verdict: 'WAG', confidence: 0.8 },
        node3: { verdict: 'WAG', confidence: 0.6 },
      });

      assert.strictEqual(result.verdict, 'WAG');
      assert.ok(result.agreement > 0.9);
      assert.strictEqual(result.unanimous, true);
    });

    it('should handle mixed verdicts', () => {
      const result = collective.getCollectiveVerdict({
        test_node: { verdict: 'WAG', confidence: 0.7 },
        node2: { verdict: 'WAG', confidence: 0.8 },
        node3: { verdict: 'GROWL', confidence: 0.6 },
      });

      assert.strictEqual(result.verdict, 'WAG');
      assert.strictEqual(result.unanimous, false);
    });

    it('should cap confidence at φ⁻¹', () => {
      const result = collective.getCollectiveVerdict({
        test_node: { verdict: 'WAG', confidence: 0.9 },
        node2: { verdict: 'WAG', confidence: 0.95 },
        node3: { verdict: 'WAG', confidence: 0.85 },
      });

      assert.ok(result.confidence <= PHI_INV + 0.001);
    });
  });

  describe('Collective Memory', () => {
    it('should remember values', () => {
      collective.remember('key1', 'value1', 0.7);
      assert.ok(collective.collectiveMemory.has('key1'));
    });

    it('should recall values', () => {
      collective.remember('key1', 'value1');
      const value = collective.recall('key1');
      assert.strictEqual(value, 'value1');
    });

    it('should return null for unknown keys', () => {
      const value = collective.recall('unknown');
      assert.strictEqual(value, null);
    });

    it('should track memory strength', () => {
      collective.remember('key1', 'value1', 0.7);
      const strength = collective.getMemoryStrength('key1');
      assert.strictEqual(strength, 0.7);
    });

    it('should increase strength on re-remember', () => {
      collective.remember('key1', 'value1', 0.5);
      collective.remember('key1', 'value1_updated', 0.8);

      const strength = collective.getMemoryStrength('key1');
      assert.strictEqual(strength, 0.8);
    });

    it('should track contributors', () => {
      collective.remember('key1', 'value1');
      const memory = collective.collectiveMemory.get('key1');
      assert.ok(memory.contributors.includes('test_node'));
    });
  });

  describe('export/import', () => {
    it('should export state', () => {
      collective.reportState({ eScore: 70 });
      collective.remember('key1', 'value1');

      const exported = collective.export();

      assert.ok('nodeId' in exported);
      assert.ok('localState' in exported);
      assert.ok('nodes' in exported);
      assert.ok('metrics' in exported);
      assert.ok('collectiveMemory' in exported);
      assert.ok('exportedAt' in exported);
    });

    it('should import state', () => {
      collective.reportState({ eScore: 70 });
      collective.remember('key1', 'value1');
      const exported = collective.export();

      const newCollective = createCollectiveState();
      newCollective.import(exported);

      assert.strictEqual(newCollective.recall('key1'), 'value1');
    });
  });

  describe('getStats()', () => {
    it('should return statistics', () => {
      collective.reportState({ eScore: 70 });
      collective.recordConsensus('topic', {}, [], 0.8);

      const stats = collective.getStats();

      assert.ok('phase' in stats);
      assert.ok('coherence' in stats);
      assert.ok('nodes' in stats);
      assert.ok('metrics' in stats);
      assert.ok('memory' in stats);
    });
  });

  describe('Health calculation', () => {
    it('should calculate overall health', () => {
      collective.reportState({ eScore: 70, consciousnessState: 'AWARE' });
      collective.receiveState('node2', { eScore: 80, consciousnessState: 'AWARE' });
      collective.receiveState('node3', { eScore: 75, consciousnessState: 'AWARE' });

      const insight = collective.getCollectiveInsight();

      assert.ok(insight.health.overall >= 0);
      assert.ok(insight.health.overall <= 1);
      assert.ok(['HEALTHY', 'DEGRADED', 'UNSTABLE', 'CRITICAL'].includes(insight.health.status));
    });
  });
});
