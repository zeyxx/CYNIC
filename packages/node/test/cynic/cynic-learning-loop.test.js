/**
 * Test: C6 Learning Loop Integration
 *
 * Verifies the full C6.5 LEARN integration:
 * - CynicDecider decision → CynicActor action → CynicLearner outcome
 * - Learning updates models
 * - Predictions improve over time
 *
 * @module @cynic/node/test/cynic/cynic-learning-loop.test
 */

'use strict';

import { describe, it, before, after, afterEach } from 'node:test';
import assert from 'node:assert';
import {
  getCynicDecider,
  resetCynicDecider,
  CynicDecisionType,
} from '../../src/cynic/cynic-decider.js';
import {
  getCynicActor,
  resetCynicActor,
  CynicActionType,
} from '../../src/cynic/cynic-actor.js';
import {
  getCynicLearner,
  resetCynicLearner,
  CynicLearningCategory,
} from '../../src/cynic/cynic-learner.js';
import { PHI_INV } from '@cynic/core';

describe('C6 Learning Loop (DECIDE → ACT → LEARN)', () => {
  let decider;
  let actor;
  let learner;

  before(() => {
    decider = getCynicDecider();
    actor = getCynicActor();
    learner = getCynicLearner();
  });

  afterEach(() => {
    // Note: CynicDecider and CynicActor don't have clear() — use history limits internally
    learner.clear();
  });

  after(() => {
    resetCynicDecider();
    resetCynicActor();
    resetCynicLearner();
  });

  it('should complete budget governance learning loop', () => {
    // Warm up decider (MIN_JUDGMENTS = 3)
    for (let i = 0; i < 3; i++) {
      decider.decide({ score: 50, verdict: 'WAG' }, {
        state: {
          budget: { spent: 3, limit: 10 },
          memory: { heapUsed: 400, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    // DECIDE: Budget critical → shift to Ollama
    const decision = decider.decide(
      { score: 45, verdict: 'GROWL' },
      {
        state: {
          budget: { spent: 7, limit: 10 }, // 70% > φ⁻¹
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      }
    );

    assert.strictEqual(decision.decision, CynicDecisionType.SHIFT_TO_OLLAMA);

    // ACT: Execute LLM routing shift (pass full decision object)
    const actionResult = actor.act(decision, {});

    assert.strictEqual(actionResult.type, CynicActionType.SHIFT_LLM_ROUTING);
    assert.strictEqual(actionResult.status, 'delivered');

    // LEARN: Record outcome (successful shift saved $2.50)
    const learningResult = learner.recordOutcome({
      category: CynicLearningCategory.BUDGET_GOVERNANCE,
      data: { savings: 2.5, success: true },
    });

    assert.strictEqual(learningResult.category, CynicLearningCategory.BUDGET_GOVERNANCE);
    assert.ok(learningResult.modelsUpdated, 'Models updated');

    const stats = learner.getStats();
    assert.strictEqual(stats.models.budget.totalShifts, 1);
    assert.strictEqual(stats.models.budget.successfulShifts, 1);
    assert.ok(stats.models.budget.avgSavingsFromShift > 0, 'Savings learned');
  });

  it('should complete memory optimization learning loop', () => {
    // Warm up decider
    for (let i = 0; i < 3; i++) {
      decider.decide({ score: 50, verdict: 'WAG' }, {
        state: {
          budget: { spent: 3, limit: 10 },
          memory: { heapUsed: 400, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    // DECIDE: Heap critical → compress + GC
    const decision = decider.decide(
      { score: 35, verdict: 'GROWL' },
      {
        state: {
          budget: { spent: 3, limit: 10 },
          memory: { heapUsed: 850, heapTotal: 1000 }, // 85% > 80% critical
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      }
    );

    assert.strictEqual(decision.decision, CynicDecisionType.COMPRESS_CONTEXT);

    // ACT: Execute memory optimization (pass full decision object)
    const actionResult = actor.act(decision, {});

    assert.strictEqual(actionResult.type, CynicActionType.OPTIMIZE_MEMORY);
    assert.strictEqual(actionResult.status, 'delivered');

    // LEARN: Record outcome (freed 50MB, 60% compression)
    const learningResult = learner.recordOutcome({
      category: CynicLearningCategory.MEMORY_OPTIMIZATION,
      data: { memoryFreed: 50, compressionRatio: 0.6, success: true },
    });

    assert.strictEqual(learningResult.category, CynicLearningCategory.MEMORY_OPTIMIZATION);

    const stats = learner.getStats();
    assert.strictEqual(stats.models.memory.totalOptimizations, 1);
    assert.strictEqual(stats.models.memory.successfulOptimizations, 1);
    assert.ok(stats.models.memory.avgMemoryFreed > 0, 'Memory freed learned');
  });

  it('should complete learning velocity learning loop', () => {
    // Warm up decider
    for (let i = 0; i < 3; i++) {
      decider.decide({ score: 50, verdict: 'WAG' }, {
        state: {
          budget: { spent: 3, limit: 10 },
          memory: { heapUsed: 400, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    // DECIDE: Learning maturity low → prioritize SONA
    const decision = decider.decide(
      { score: 42, verdict: 'WAG' },
      {
        state: {
          budget: { spent: 3, limit: 10 },
          memory: { heapUsed: 400, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 25 }, // 25% < φ⁻² (38.2%)
          events: { orphanCount: 0 },
        },
      }
    );

    assert.strictEqual(decision.decision, CynicDecisionType.PRIORITIZE_SONA);

    // ACT: Adjust learning priority (pass full decision object)
    const actionResult = actor.act(decision, {});

    assert.strictEqual(actionResult.type, CynicActionType.ADJUST_LEARNING);
    assert.strictEqual(actionResult.status, 'delivered');

    // LEARN: Record outcome (velocity increased by 5%)
    const learningResult = learner.recordOutcome({
      category: CynicLearningCategory.LEARNING_VELOCITY,
      data: { velocityChange: 0.05, wasEffective: true },
    });

    assert.strictEqual(learningResult.category, CynicLearningCategory.LEARNING_VELOCITY);

    const stats = learner.getStats();
    assert.strictEqual(stats.models.learning.totalBoosts, 1);
    assert.strictEqual(stats.models.learning.effectiveBoosts, 1);
  });

  it('should improve predictions after multiple learning cycles', () => {
    // Run 10 budget governance cycles
    for (let i = 0; i < 10; i++) {
      const decision = decider.decide(
        { score: 45, verdict: 'GROWL' },
        {
          state: {
            budget: { spent: 7 + Math.random(), limit: 10 },
            memory: { heapUsed: 100, heapTotal: 1000 },
            context: { sizeMB: 1 },
            learning: { maturityPercent: 50 },
            events: { orphanCount: 0 },
          },
        }
      );

      actor.act(decision, {});

      learner.recordOutcome({
        category: CynicLearningCategory.BUDGET_GOVERNANCE,
        data: { savings: 2.0 + Math.random(), success: true },
      });
    }

    // Prediction should now be confident
    const prediction = learner.predictBudgetSavings();

    assert.ok(prediction.prediction !== null, 'Prediction exists after 10 cycles');
    assert.ok(prediction.confidence > 0.3, 'Confidence improved');
    assert.ok(prediction.confidence <= PHI_INV, 'Confidence φ-bounded');
    assert.strictEqual(prediction.samples, 10, '10 samples learned');
  });

  it('should track decision accuracy over multiple cycles', () => {
    // Run mixed outcome cycles
    const outcomes = [
      { type: 'shift_to_ollama', wasGood: true },
      { type: 'shift_to_ollama', wasGood: true },
      { type: 'compress_context', wasGood: true },
      { type: 'compress_context', wasGood: false },
      { type: 'prioritize_sona', wasGood: true },
    ];

    for (const outcome of outcomes) {
      learner.recordOutcome({
        category: CynicLearningCategory.DECISION_ACCURACY,
        data: outcome,
      });
    }

    const stats = learner.getStats();
    const accuracy = stats.models.decisions.accuracy;

    // 4 good out of 5 = 80%
    assert.strictEqual(accuracy, 0.8);

    // Per-type tracking
    const byType = stats.models.decisions.byDecisionType;
    assert.strictEqual(byType.shift_to_ollama.total, 2);
    assert.strictEqual(byType.shift_to_ollama.correct, 2);
    assert.strictEqual(byType.compress_context.total, 2);
    assert.strictEqual(byType.compress_context.correct, 1);
  });

  it('should maintain separate histories per learning category', () => {
    learner.recordOutcome({
      category: CynicLearningCategory.BUDGET_GOVERNANCE,
      data: { savings: 2.5, success: true },
    });
    learner.recordOutcome({
      category: CynicLearningCategory.MEMORY_OPTIMIZATION,
      data: { memoryFreed: 50, success: true },
    });
    learner.recordOutcome({
      category: CynicLearningCategory.LEARNING_VELOCITY,
      data: { velocityChange: 0.05, wasEffective: true },
    });

    const budgetHistory = learner.getHistory(CynicLearningCategory.BUDGET_GOVERNANCE);
    const memoryHistory = learner.getHistory(CynicLearningCategory.MEMORY_OPTIMIZATION);
    const learningHistory = learner.getHistory(CynicLearningCategory.LEARNING_VELOCITY);

    assert.strictEqual(budgetHistory.length, 1, 'Budget history isolated');
    assert.strictEqual(memoryHistory.length, 1, 'Memory history isolated');
    assert.strictEqual(learningHistory.length, 1, 'Learning history isolated');

    assert.ok(budgetHistory[0].savings !== undefined, 'Budget data preserved');
    assert.ok(memoryHistory[0].freed !== undefined, 'Memory data preserved');
    assert.ok(learningHistory[0].velocityChange !== undefined, 'Learning data preserved');
  });
});
