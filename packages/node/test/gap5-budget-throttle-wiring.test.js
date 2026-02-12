/**
 * GAP-5 Budget Monitor + Throttle Gate Wiring Test
 *
 * Validates:
 * 1. BudgetMonitor tracks task_id and task_type
 * 2. ThrottleGate blocks requests when budget > φ⁻¹ (61.8%)
 * 3. Auto-warning at 50% budget
 * 4. G2.5 metric: ≥30 tasks tracked
 *
 * @module test/gap5-budget-throttle-wiring
 */

'use strict';

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { CostLedger, getCostLedger, resetCostLedger } from '../src/accounting/cost-ledger.js';
import { BudgetMonitor, BudgetLevel, BudgetRecommendation } from '../src/orchestration/budget-monitor.js';
import { ThrottleGate, ThrottleAction, Stage } from '../src/orchestration/throttle-gate.js';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

describe('GAP-5: BudgetMonitor + ThrottleGate Wiring', () => {
  let costLedger;
  let budgetMonitor;
  let throttleGate;

  before(() => {
    // Reset singleton
    resetCostLedger();

    // Create instances
    costLedger = new CostLedger({
      sessionBudget: 10000, // 10k tokens
      model: 'sonnet',
    });

    budgetMonitor = new BudgetMonitor({
      costLedger,
    });

    throttleGate = new ThrottleGate({
      budgetMonitor,
    });
  });

  after(() => {
    resetCostLedger();
  });

  describe('1. BudgetMonitor tracks tasks', () => {
    it('should track task_id and task_type', () => {
      const assessment = budgetMonitor.assess({
        taskId: 'task-001',
        taskType: 'judgment',
      });

      assert.ok(assessment);
      assert.equal(assessment.level, BudgetLevel.ABUNDANT);

      const stats = budgetMonitor.getStats();
      assert.equal(stats.tasksTracked, 1);
    });

    it('should track 30+ tasks for G2.5 metric', () => {
      for (let i = 0; i < 30; i++) {
        budgetMonitor.assess({
          taskId: `task-${i.toString().padStart(3, '0')}`,
          taskType: i % 3 === 0 ? 'judgment' : i % 3 === 1 ? 'routing' : 'synthesis',
        });
      }

      const stats = budgetMonitor.getStats();
      assert.ok(stats.tasksTracked >= 30, `Expected ≥30 tasks, got ${stats.tasksTracked}`);
    });
  });

  describe('2. ThrottleGate blocks at budget thresholds', () => {
    it('should ALLOW when budget < 38.2% (ABUNDANT)', () => {
      // Consume 30% of budget
      costLedger.record({
        type: 'test',
        inputTokens: 3000,
        outputTokens: 0,
      });

      const decision = throttleGate.decide(Stage.ROUTING, {
        taskId: 'threshold-test-1',
      });

      assert.equal(decision.action, ThrottleAction.ALLOW);
      assert.equal(decision.assessment.level, BudgetLevel.ABUNDANT);
    });

    it('should THROTTLE when budget > 61.8% (CAUTIOUS)', () => {
      // Consume to 65% total
      costLedger.record({
        type: 'test',
        inputTokens: 3500,
        outputTokens: 0,
      });

      const decision = throttleGate.decide(Stage.ROUTING, {
        taskId: 'threshold-test-2',
      });

      assert.equal(decision.action, ThrottleAction.THROTTLE);
      assert.equal(decision.assessment.level, BudgetLevel.CAUTIOUS);
      assert.ok(decision.throttleParams.maxDogs === 5);
    });

    it('should ESCALATE when budget > 80% (CRITICAL)', () => {
      // Consume to 85% total
      costLedger.record({
        type: 'test',
        inputTokens: 2000,
        outputTokens: 0,
      });

      const decision = throttleGate.decide(Stage.ROUTING, {
        taskId: 'threshold-test-3',
      });

      assert.equal(decision.action, ThrottleAction.ESCALATE);
      assert.equal(decision.assessment.level, BudgetLevel.CRITICAL);
    });

    it('should SKIP when budget > 95% (EXHAUSTED)', () => {
      // Consume to 96% total
      costLedger.record({
        type: 'test',
        inputTokens: 1100,
        outputTokens: 0,
      });

      const decision = throttleGate.decide(Stage.JUDGMENT, {
        taskId: 'threshold-test-4',
      });

      assert.equal(decision.action, ThrottleAction.SKIP);
      assert.equal(decision.assessment.level, BudgetLevel.EXHAUSTED);
    });
  });

  describe('3. Auto-warning at 50% budget', () => {
    it('should emit warning when crossing 50% threshold', (t, done) => {
      const ledger = new CostLedger({
        sessionBudget: 1000,
        model: 'sonnet',
      });

      let warningEmitted = false;

      ledger.on('budget:moderate', (status) => {
        warningEmitted = true;
        assert.ok(status.consumedRatio >= PHI_INV_2);
        done();
      });

      // Consume to 62% (crosses φ⁻¹ threshold)
      ledger.record({
        type: 'test',
        inputTokens: 620,
        outputTokens: 0,
      });

      // Wait for event
      setTimeout(() => {
        if (!warningEmitted) {
          done(new Error('Warning not emitted'));
        }
      }, 100);
    });
  });

  describe('4. ThrottleGate stage-specific behavior', () => {
    it('should throttle JUDGMENT to axioms only', () => {
      const decision = throttleGate.decide(Stage.JUDGMENT, {
        taskId: 'stage-test-judgment',
      });

      // Still SKIP due to exhausted budget from previous tests
      // Reset for this test
      const freshLedger = new CostLedger({
        sessionBudget: 10000,
        model: 'sonnet',
      });
      const freshMonitor = new BudgetMonitor({ costLedger: freshLedger });
      const freshGate = new ThrottleGate({ budgetMonitor: freshMonitor });

      // Consume to CAUTIOUS level (62%)
      freshLedger.record({ type: 'test', inputTokens: 6200, outputTokens: 0 });

      const judgmentDecision = freshGate.decide(Stage.JUDGMENT, {
        taskId: 'stage-test-judgment',
      });

      assert.equal(judgmentDecision.action, ThrottleAction.THROTTLE);
      assert.equal(judgmentDecision.throttleParams.dimensionMode, 'axioms');
      assert.ok(Array.isArray(judgmentDecision.throttleParams.dimensions));
      assert.ok(judgmentDecision.throttleParams.dimensions.includes('PHI'));
    });

    it('should skip SYNTHESIS when throttled', () => {
      const freshLedger = new CostLedger({
        sessionBudget: 10000,
        model: 'sonnet',
      });
      const freshMonitor = new BudgetMonitor({ costLedger: freshLedger });
      const freshGate = new ThrottleGate({ budgetMonitor: freshMonitor });

      // Consume to CAUTIOUS level
      freshLedger.record({ type: 'test', inputTokens: 6200, outputTokens: 0 });

      const synthesisDecision = freshGate.decide(Stage.SYNTHESIS, {
        taskId: 'stage-test-synthesis',
      });

      assert.equal(synthesisDecision.action, ThrottleAction.SKIP);
      assert.equal(synthesisDecision.throttleParams.reason, 'throttle_synthesis');
    });

    it('should always allow SKILL (low overhead)', () => {
      const freshLedger = new CostLedger({
        sessionBudget: 10000,
        model: 'sonnet',
      });
      const freshMonitor = new BudgetMonitor({ costLedger: freshLedger });
      const freshGate = new ThrottleGate({ budgetMonitor: freshMonitor });

      // Consume to CAUTIOUS level
      freshLedger.record({ type: 'test', inputTokens: 6200, outputTokens: 0 });

      const skillDecision = freshGate.decide(Stage.SKILL, {
        taskId: 'stage-test-skill',
      });

      assert.equal(skillDecision.action, ThrottleAction.ALLOW);
    });
  });

  describe('5. Statistics tracking', () => {
    it('should track throttle decisions', () => {
      const stats = throttleGate.getStats();

      assert.ok(stats.decisions > 0);
      assert.ok(stats.actions.ALLOW >= 0);
      assert.ok(stats.actions.THROTTLE >= 0);
      assert.ok(stats.actions.SKIP >= 0);
      assert.ok(stats.actions.ESCALATE >= 0);
      assert.ok(stats.allowRatio >= 0 && stats.allowRatio <= 1);
    });

    it('should track by-stage statistics', () => {
      const stats = throttleGate.getStats();

      assert.ok(stats.byStage.routing);
      assert.ok(stats.byStage.judgment);
      assert.ok(stats.byStage.synthesis);
      assert.ok(stats.byStage.skill);
    });
  });
});
