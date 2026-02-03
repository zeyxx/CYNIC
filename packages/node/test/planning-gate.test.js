/**
 * PlanningGate Tests
 *
 * Tests for the CYNIC PlanningGate - Meta-Cognition Layer.
 *
 * "Un système qui pense avant d'agir" - κυνικός
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  PlanningGate,
  PlanningResult,
  PlanningTrigger,
  PlanningDecision,
  PLANNING_THRESHOLDS,
  createPlanningGate,
  getPlanningGate,
  _resetPlanningGateForTesting,
} from '../src/orchestration/planning-gate.js';

import { DecisionEvent } from '../src/orchestration/decision-event.js';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// =============================================================================
// TEST HELPERS
// =============================================================================

function createMockBrain(overrides = {}) {
  return {
    think: mock.fn(async () => ({
      synthesis: {
        consultations: [
          {
            response: {
              alternatives: [
                { id: 'alt1', label: 'Alternative 1' },
                { id: 'alt2', label: 'Alternative 2' },
              ],
            },
          },
        ],
      },
      decision: { action: 'proceed', reason: 'Looks good' },
      confidence: 0.55,
    })),
    ...overrides,
  };
}

function createMockChaosGenerator(overrides = {}) {
  return {
    shouldForcePlanning: mock.fn(() => ({ force: false })),
    ...overrides,
  };
}

function createTestEvent(overrides = {}) {
  return new DecisionEvent({
    eventType: 'user_prompt',
    source: 'test',
    content: 'Test content',
    context: {},
    userContext: {
      userId: 'test-user',
      trustLevel: 'BUILDER',
    },
    ...overrides,
  });
}

// =============================================================================
// PLANNING RESULT TESTS
// =============================================================================

describe('PlanningResult', () => {
  describe('construction', () => {
    it('should create with defaults', () => {
      const result = new PlanningResult();

      assert.equal(result.needed, false);
      assert.equal(result.decision, PlanningDecision.CONTINUE);
      assert.deepEqual(result.triggers, []);
      assert.deepEqual(result.alternatives, []);
      assert.equal(result.plan, null);
      assert.ok(result.timestamp > 0);
    });

    it('should create with provided data', () => {
      const result = new PlanningResult({
        needed: true,
        decision: PlanningDecision.PAUSE,
        triggers: [PlanningTrigger.COMPLEXITY_UNCERTAIN],
        alternatives: [{ id: 'alt1' }],
        confidence: 0.5,
      });

      assert.equal(result.needed, true);
      assert.equal(result.decision, PlanningDecision.PAUSE);
      assert.deepEqual(result.triggers, [PlanningTrigger.COMPLEXITY_UNCERTAIN]);
      assert.equal(result.alternatives.length, 1);
      assert.equal(result.confidence, 0.5);
    });

    it('should cap confidence at φ⁻¹', () => {
      const result = new PlanningResult({ confidence: 0.9 });

      assert.ok(result.confidence <= PHI_INV);
    });
  });

  describe('toJSON', () => {
    it('should serialize correctly', () => {
      const result = new PlanningResult({
        needed: true,
        decision: PlanningDecision.CONSULT,
        triggers: [PlanningTrigger.BORDERLINE_CONFIDENCE],
        alternatives: [{ id: 'a1' }, { id: 'a2' }],
      });

      const json = result.toJSON();

      assert.equal(json.needed, true);
      assert.equal(json.decision, 'consult');
      assert.deepEqual(json.triggers, ['borderline_confidence']);
      assert.equal(json.alternativeCount, 2);
      assert.ok(json.timestamp > 0);
    });
  });
});

// =============================================================================
// PLANNING GATE TESTS
// =============================================================================

describe('PlanningGate', () => {
  let planningGate;

  beforeEach(() => {
    _resetPlanningGateForTesting();
    planningGate = new PlanningGate();
  });

  describe('construction', () => {
    it('should create with defaults', () => {
      const gate = new PlanningGate();

      assert.ok(gate.thresholds);
      assert.equal(gate.thresholds.COMPLEXITY_MIN, 0.33);
      assert.equal(gate.thresholds.CONFIDENCE_LOW, PHI_INV_2);
      assert.equal(gate.thresholds.CONFIDENCE_HIGH, PHI_INV);
      assert.equal(gate.chaosGenerator, null);
      assert.equal(gate.brain, null);
    });

    it('should accept custom thresholds', () => {
      const gate = new PlanningGate({
        thresholds: { COMPLEXITY_MIN: 0.5 },
      });

      assert.equal(gate.thresholds.COMPLEXITY_MIN, 0.5);
      // Other thresholds should be preserved
      assert.equal(gate.thresholds.CONFIDENCE_LOW, PHI_INV_2);
    });

    it('should accept chaos generator', () => {
      const chaos = createMockChaosGenerator();
      const gate = new PlanningGate({ chaosGenerator: chaos });

      assert.strictEqual(gate.chaosGenerator, chaos);
    });
  });

  // ===========================================================================
  // TRIGGER DETECTION TESTS
  // ===========================================================================

  describe('shouldPlan - trigger detection', () => {
    it('should not trigger on simple low-risk events', () => {
      const event = createTestEvent();
      event.setRouting({
        risk: 'low',
        intervention: 'silent',
      });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,  // High confidence in complexity
        confidence: 0.7,  // Above φ⁻¹
        entropy: 0.1,     // Low entropy
        consensusRatio: 0.95,  // High consensus
      });

      // Should CONTINUE without planning (only chaos test would fire)
      assert.equal(result.decision, PlanningDecision.CONTINUE);
    });

    it('should trigger on complexity uncertain (< 33%)', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.2,  // Below 33% threshold
      });

      assert.equal(result.needed, true);
      assert.ok(result.triggers.includes(PlanningTrigger.COMPLEXITY_UNCERTAIN));
    });

    it('should trigger on borderline confidence (38.2% - 61.8%)', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.5,  // In borderline range
      });

      assert.equal(result.needed, true);
      assert.ok(result.triggers.includes(PlanningTrigger.BORDERLINE_CONFIDENCE));
    });

    it('should NOT trigger on confidence below borderline', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.2,  // Below borderline range
      });

      // BORDERLINE_CONFIDENCE should NOT be triggered
      assert.ok(!result.triggers.includes(PlanningTrigger.BORDERLINE_CONFIDENCE));
    });

    it('should trigger on high risk + low trust', () => {
      const event = createTestEvent({
        userContext: { trustLevel: 'GUEST' },
      });
      event.setRouting({ risk: 'critical' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
      });

      assert.equal(result.needed, true);
      assert.ok(result.triggers.includes(PlanningTrigger.HIGH_RISK_LOW_TRUST));
    });

    it('should NOT trigger on high risk + high trust', () => {
      const event = createTestEvent({
        userContext: { trustLevel: 'ARCHITECT' },  // High trust
      });
      event.setRouting({ risk: 'critical' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
      });

      // HIGH_RISK_LOW_TRUST should NOT be triggered
      assert.ok(!result.triggers.includes(PlanningTrigger.HIGH_RISK_LOW_TRUST));
    });

    it('should trigger on high entropy (> φ⁻¹ + 15%)', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
        entropy: 0.8,  // Above ~0.768 threshold
      });

      assert.equal(result.needed, true);
      assert.ok(result.triggers.includes(PlanningTrigger.HIGH_ENTROPY));
    });

    it('should trigger on agent disagreement (< 70% consensus)', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
        consensusRatio: 0.5,  // Below 70% threshold
      });

      assert.equal(result.needed, true);
      assert.ok(result.triggers.includes(PlanningTrigger.AGENT_DISAGREEMENT));
    });

    it('should trigger on explicit request', () => {
      const event = createTestEvent({
        context: { requestPlanning: true },
      });
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
      });

      assert.equal(result.needed, true);
      assert.ok(result.triggers.includes(PlanningTrigger.EXPLICIT_REQUEST));
    });

    it('should trigger on chaos injection', () => {
      const chaos = createMockChaosGenerator({
        shouldForcePlanning: mock.fn(() => ({ force: true })),
      });
      const gate = new PlanningGate({ chaosGenerator: chaos });

      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = gate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
      });

      assert.equal(result.needed, true);
      assert.ok(result.triggers.includes(PlanningTrigger.CHAOS_TEST));
    });
  });

  // ===========================================================================
  // DECISION MAPPING TESTS
  // ===========================================================================

  describe('shouldPlan - decision mapping', () => {
    it('should CONTINUE when no triggers', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
        entropy: 0.1,
        consensusRatio: 0.95,
      });

      assert.equal(result.decision, PlanningDecision.CONTINUE);
    });

    it('should PAUSE on critical triggers (HIGH_RISK_LOW_TRUST)', () => {
      const event = createTestEvent({
        userContext: { trustLevel: 'GUEST' },
      });
      event.setRouting({ risk: 'critical' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
      });

      assert.equal(result.decision, PlanningDecision.PAUSE);
    });

    it('should PAUSE on explicit request', () => {
      const event = createTestEvent({
        context: { requestPlanning: true },
      });
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.9,
        confidence: 0.7,
      });

      assert.equal(result.decision, PlanningDecision.PAUSE);
    });

    it('should PAUSE on multiple triggers', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.2,  // Trigger 1: COMPLEXITY_UNCERTAIN
        confidence: 0.5,  // Trigger 2: BORDERLINE_CONFIDENCE
      });

      assert.equal(result.triggers.length, 2);
      assert.equal(result.decision, PlanningDecision.PAUSE);
    });

    it('should CONSULT on single non-critical trigger', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      const result = planningGate.shouldPlan(event, {
        complexity: 0.2,  // Only COMPLEXITY_UNCERTAIN
        confidence: 0.7,
        entropy: 0.1,
        consensusRatio: 0.95,
      });

      assert.equal(result.triggers.length, 1);
      assert.equal(result.decision, PlanningDecision.CONSULT);
    });
  });

  // ===========================================================================
  // PLAN GENERATION TESTS
  // ===========================================================================

  describe('generatePlan', () => {
    it('should return unchanged result if not needed', async () => {
      const event = createTestEvent();
      const planningResult = new PlanningResult({ needed: false });

      const result = await planningGate.generatePlan(event, planningResult);

      assert.equal(result.needed, false);
      assert.equal(result.plan, null);
    });

    it('should generate basic alternatives without brain', async () => {
      const event = createTestEvent();
      const planningResult = new PlanningResult({
        needed: true,
        triggers: [PlanningTrigger.COMPLEXITY_UNCERTAIN],
      });

      const result = await planningGate.generatePlan(event, planningResult);

      assert.ok(result.plan);
      assert.ok(result.alternatives.length >= 2);  // At least proceed + cancel
      assert.ok(result.alternatives.some(a => a.id === 'proceed'));
      assert.ok(result.alternatives.some(a => a.id === 'cancel'));
    });

    it('should include trigger-specific alternatives', async () => {
      const event = createTestEvent();
      const planningResult = new PlanningResult({
        needed: true,
        triggers: [PlanningTrigger.COMPLEXITY_UNCERTAIN],
      });

      const result = await planningGate.generatePlan(event, planningResult);

      // Should include "simplify" for complexity uncertainty
      assert.ok(result.alternatives.some(a => a.id === 'simplify'));
    });

    it('should use brain for alternatives when available', async () => {
      const brain = createMockBrain();
      const gate = new PlanningGate({ brain });

      const event = createTestEvent();
      const planningResult = new PlanningResult({
        needed: true,
        triggers: [PlanningTrigger.BORDERLINE_CONFIDENCE],
      });

      const result = await gate.generatePlan(event, planningResult);

      assert.ok(brain.think.mock.calls.length > 0);
      // Should have alternatives from brain synthesis
      assert.ok(result.alternatives.length >= 1);
    });

    it('should handle brain errors gracefully', async () => {
      const brain = createMockBrain({
        think: mock.fn(async () => { throw new Error('Brain error'); }),
      });
      const gate = new PlanningGate({ brain });

      const event = createTestEvent();
      const planningResult = new PlanningResult({
        needed: true,
        triggers: [PlanningTrigger.BORDERLINE_CONFIDENCE],
      });

      const result = await gate.generatePlan(event, planningResult);

      // Should not throw, should have plan
      assert.ok(result.plan);
      assert.ok(result.plan.reasoning.some(r => r.includes('error')));
    });
  });

  // ===========================================================================
  // STATISTICS TESTS
  // ===========================================================================

  describe('statistics', () => {
    it('should track checks', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      planningGate.shouldPlan(event, { complexity: 0.9 });
      planningGate.shouldPlan(event, { complexity: 0.9 });

      assert.equal(planningGate.stats.checked, 2);
    });

    it('should track planning triggered', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      planningGate.shouldPlan(event, { complexity: 0.2 });  // Triggers
      planningGate.shouldPlan(event, { complexity: 0.9, confidence: 0.7 });  // No trigger

      assert.equal(planningGate.stats.planningTriggered, 1);
    });

    it('should track by trigger type', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      planningGate.shouldPlan(event, { complexity: 0.2 });

      assert.ok(planningGate.stats.byTrigger[PlanningTrigger.COMPLEXITY_UNCERTAIN] >= 1);
    });

    it('should track by decision', () => {
      const event = createTestEvent({
        context: { requestPlanning: true },
      });
      event.setRouting({ risk: 'low' });

      planningGate.shouldPlan(event, {});

      assert.ok(planningGate.stats.byDecision[PlanningDecision.PAUSE] >= 1);
    });

    it('should calculate planning rate', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      planningGate.shouldPlan(event, { complexity: 0.2 });  // Triggers
      planningGate.shouldPlan(event, { complexity: 0.9, confidence: 0.7 });  // No trigger

      const stats = planningGate.getStats();
      assert.equal(stats.planningRate, '50.0%');
    });

    it('should reset stats', () => {
      const event = createTestEvent();
      event.setRouting({ risk: 'low' });

      planningGate.shouldPlan(event, { complexity: 0.2 });
      planningGate.resetStats();

      assert.equal(planningGate.stats.checked, 0);
      assert.equal(planningGate.stats.planningTriggered, 0);
    });
  });

  // ===========================================================================
  // SINGLETON TESTS
  // ===========================================================================

  describe('singleton', () => {
    beforeEach(() => {
      _resetPlanningGateForTesting();
    });

    it('should return same instance', () => {
      const gate1 = getPlanningGate();
      const gate2 = getPlanningGate();

      assert.strictEqual(gate1, gate2);
    });

    it('should reset for testing', () => {
      const gate1 = getPlanningGate();
      _resetPlanningGateForTesting();
      const gate2 = getPlanningGate();

      assert.notStrictEqual(gate1, gate2);
    });
  });

  // ===========================================================================
  // FACTORY TESTS
  // ===========================================================================

  describe('createPlanningGate', () => {
    it('should create new instance', () => {
      const gate1 = createPlanningGate();
      const gate2 = createPlanningGate();

      assert.notStrictEqual(gate1, gate2);
    });

    it('should accept options', () => {
      const chaos = createMockChaosGenerator();
      const gate = createPlanningGate({ chaosGenerator: chaos });

      assert.strictEqual(gate.chaosGenerator, chaos);
    });
  });
});
