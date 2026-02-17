/**
 * CynicActor Test - C6.4 (CYNIC × ACT)
 *
 * Test self-optimization actions generated via factory pattern.
 *
 * @module @cynic/node/test/cynic-actor.test
 */

'use strict';

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { getCynicActor, resetCynicActor, CynicActionType } from '../src/cynic/cynic-actor.js';

describe('CynicActor (factory-generated)', () => {
  let actor;

  beforeEach(() => {
    resetCynicActor();
    actor = getCynicActor();
  });

  afterEach(() => {
    resetCynicActor();
  });

  it('should be singleton', () => {
    const actor2 = getCynicActor();
    assert.strictEqual(actor, actor2);
  });

  it('should initialize with correct stats', () => {
    const stats = actor.getStats();
    assert.strictEqual(stats.actionsTotal, 0);
    assert.strictEqual(stats.delivered, 0);
    assert.ok(stats.byType);
  });

  it('should handle COMPACT_MEMORY decision', () => {
    const decision = {
      type: 'compact_memory',
      context: {
        reason: 'memory_pressure_exceeds_goal',
        load: 0.75,
      },
      confidence: 0.58,
    };

    const result = actor.act(decision, { qScore: 42 });

    assert.ok(result);
    assert.strictEqual(result.type, CynicActionType.OPTIMIZE_MEMORY);
    assert.strictEqual(result.status, 'delivered');
    assert.ok(result.message.includes('*sniff*'));
    assert.ok(result.message.includes('memory'));
    assert.strictEqual(result.cell, 'C6.4');
    assert.strictEqual(result.dimension, 'CYNIC');
  });

  it('should handle PAUSE_LEARNING decision', () => {
    const decision = {
      type: 'pause_learning',
      context: {
        reason: 'learning_stagnation',
        velocity: 0.15,
      },
      urgency: 'medium',
      confidence: 0.52,
    };

    const result = actor.act(decision);

    assert.ok(result);
    assert.strictEqual(result.type, CynicActionType.ADJUST_LEARNING);
    assert.strictEqual(result.urgency, 'medium');
    assert.ok(result.message.includes('learning'));
  });

  it('should handle ROTATE_DOGS decision', () => {
    const decision = {
      type: 'rotate_dogs',
      context: {
        reason: 'dog_dominance_exceeds_goal',
        dominantDog: 'JUDGE',
        ratio: 0.72,
      },
      confidence: 0.61,
    };

    const result = actor.act(decision);

    assert.ok(result);
    assert.strictEqual(result.type, CynicActionType.ROTATE_DOGS);
    assert.ok(result.message.includes('dog'));
  });

  it('should handle ESCALATE_PATTERN decision', () => {
    const decision = {
      type: 'escalate_pattern',
      context: {
        reason: 'guardian_escalation_pattern',
        escalationCount: 5,
      },
      urgency: 'high',
      confidence: 0.58,
    };

    const result = actor.act(decision);

    assert.ok(result);
    assert.strictEqual(result.type, CynicActionType.ESCALATE_ALERT);
    assert.strictEqual(result.urgency, 'high');
    assert.ok(result.message.includes('*GROWL*'));
  });

  it('should handle ADJUST_THRESHOLDS decision', () => {
    const decision = {
      type: 'adjust_thresholds',
      context: {
        reason: 'consensus_below_goal',
        approvalRate: 0.35,
      },
      confidence: 0.48,
    };

    const result = actor.act(decision);

    assert.ok(result);
    assert.strictEqual(result.type, CynicActionType.ADJUST_THRESHOLDS);
  });

  it('should handle ACKNOWLEDGE decision', () => {
    const decision = {
      type: 'acknowledge',
      context: {
        reason: 'consensus_quality_change_noted',
        approvalRate: 0.45,
      },
      confidence: 0.42,
    };

    const result = actor.act(decision);

    assert.ok(result);
    assert.strictEqual(result.type, CynicActionType.ACKNOWLEDGE);
    assert.ok(result.message.includes('*yawn*'));
  });

  it('should respect cooldowns', () => {
    const decision = {
      type: 'compact_memory',
      context: { reason: 'memory_pressure' },
      confidence: 0.55,
    };

    // First action succeeds
    const result1 = actor.act(decision);
    assert.ok(result1);

    // Second action immediately after is on cooldown
    const result2 = actor.act(decision);
    assert.strictEqual(result2, null);
  });

  it('should update stats correctly', () => {
    const decisions = [
      { type: 'compact_memory', context: { reason: 'test1' }, confidence: 0.5 },
      { type: 'pause_learning', context: { reason: 'test2' }, confidence: 0.5 },
      { type: 'escalate_pattern', context: { reason: 'test3' }, urgency: 'high', confidence: 0.5 },
    ];

    decisions.forEach(d => actor.act(d));

    const stats = actor.getStats();
    assert.strictEqual(stats.actionsTotal, 3);
    assert.strictEqual(stats.delivered, 3);
    assert.strictEqual(stats.byType[CynicActionType.OPTIMIZE_MEMORY], 1);
    assert.strictEqual(stats.byType[CynicActionType.ADJUST_LEARNING], 1);
    assert.strictEqual(stats.byType[CynicActionType.ESCALATE_ALERT], 1);
    assert.strictEqual(stats.alertsEscalated, 1);
    assert.strictEqual(stats.optimizationsExecuted, 1);
  });

  it('should emit action events', (t, done) => {
    const decision = {
      type: 'escalate_pattern',
      context: { reason: 'test_event' },
      confidence: 0.55,
    };

    actor.once('action', (action) => {
      assert.ok(action);
      assert.strictEqual(action.type, CynicActionType.ESCALATE_ALERT);
      done();
    });

    actor.act(decision);
  });

  it('should track optimization log via postAct', () => {
    const decision = {
      type: 'compact_memory',
      context: { reason: 'memory_optimization_test' },
      confidence: 0.58,
    };

    actor.act(decision);

    // Check optimization log (private field, accessed via internal state)
    assert.ok(actor._optimizationLog);
    assert.strictEqual(actor._optimizationLog.length, 1);
    assert.strictEqual(actor._optimizationLog[0].type, CynicActionType.OPTIMIZE_MEMORY);
    assert.strictEqual(actor._optimizationLog[0].reason, 'memory_optimization_test');
  });

  it('should return health status', () => {
    // Trigger a few alerts to affect health
    for (let i = 0; i < 3; i++) {
      actor.act({
        type: 'escalate_pattern',
        context: { reason: `alert_${i}` },
        confidence: 0.5,
      });
      // Wait a tiny bit to bypass cooldown (would need to mock time in real scenario)
      actor._lastAction.clear(); // Hack for test
    }

    const health = actor.getHealth();
    assert.ok(health);
    assert.ok(['healthy', 'high_alert_rate'].includes(health.status));
    assert.ok(typeof health.score === 'number');
    assert.ok(health.score <= 0.618); // φ⁻¹ bound
    assert.strictEqual(health.actionsTotal, 3);
  });

  it('should map reason hints to LLM routing action', () => {
    const decision = {
      type: 'adjust_thresholds',
      context: {
        reason: 'budget_optimization_via_ollama',
      },
      confidence: 0.48,
    };

    const result = actor.act(decision);

    assert.ok(result);
    // Should detect 'ollama' in reason and map to SHIFT_LLM_ROUTING
    assert.strictEqual(result.type, CynicActionType.SHIFT_LLM_ROUTING);
    assert.ok(result.message.includes('LLM routing'));
  });

  it('should include judgment context in result', () => {
    const decision = {
      type: 'compact_memory',
      context: { reason: 'memory_test' },
      confidence: 0.55,
    };

    const context = {
      judgmentId: 'judgment_123',
      qScore: 42,
    };

    const result = actor.act(decision, context);

    assert.strictEqual(result.judgmentId, 'judgment_123');
    assert.ok(result.message.includes('Q:42'));
  });

  it('should handle null action type (no-op)', () => {
    const decision = {
      type: 'unknown_decision_type',
      context: { reason: 'test' },
      confidence: 0.5,
    };

    // mapDecisionToAction returns null/undefined for unknown types -> defaults to ACKNOWLEDGE
    const result = actor.act(decision);

    // ACKNOWLEDGE has no cooldown, so should work
    assert.ok(result);
    assert.strictEqual(result.type, CynicActionType.ACKNOWLEDGE);
  });

  it('should record action response (acted_on / dismissed)', () => {
    const decision = {
      type: 'escalate_pattern',
      context: { reason: 'test_response' },
      confidence: 0.55,
    };

    actor.act(decision);

    // Simulate user response
    actor.recordResponse(CynicActionType.ESCALATE_ALERT, 'acted_on');

    const history = actor.getHistory();
    assert.ok(history.length > 0);
    assert.strictEqual(history[history.length - 1].status, 'acted_on');
  });

  it('should limit history to maxHistory', () => {
    // maxHistory = 144 (Fib(12))
    // Create more than 144 actions (mock by bypassing cooldowns)
    for (let i = 0; i < 150; i++) {
      actor._lastAction.clear(); // Hack to bypass cooldowns
      actor.act({
        type: 'acknowledge',
        context: { reason: `action_${i}` },
        confidence: 0.5,
      });
    }

    const history = actor.getHistory(200);
    assert.ok(history.length <= 144);
  });
});
