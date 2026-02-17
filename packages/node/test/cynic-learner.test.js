/**
 * Test: CynicLearner (C6.5 LEARN)
 *
 * Verifies:
 * - Factory-generated class structure
 * - Six learning categories functional
 * - Q-Table updates after learning
 * - Predictions φ-bounded
 * - learning_events table persistence (integration)
 *
 * @module @cynic/node/test/cynic-learner.test
 */

'use strict';

import { describe, it, before, after, afterEach } from 'node:test';
import assert from 'node:assert';
import {
  getCynicLearner,
  resetCynicLearner,
  CynicLearningCategory,
} from '../src/cynic/cynic-learner.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

describe('CynicLearner (C6.5)', () => {
  let learner;

  before(() => {
    learner = getCynicLearner();
  });

  afterEach(() => {
    learner.clear();
  });

  after(() => {
    resetCynicLearner();
  });

  it('should be a singleton', () => {
    const second = getCynicLearner();
    assert.strictEqual(learner, second, 'Singleton violated');
  });

  it('should have C6.5 metadata', () => {
    const stats = learner.getStats();
    assert.ok(stats, 'Stats exist');

    // Models initialized
    assert.ok(stats.models.budget, 'Budget model exists');
    assert.ok(stats.models.memory, 'Memory model exists');
    assert.ok(stats.models.learning, 'Learning model exists');
    assert.ok(stats.models.dogs, 'Dogs model exists');
    assert.ok(stats.models.thresholds, 'Thresholds model exists');
    assert.ok(stats.models.decisions, 'Decisions model exists');
  });

  describe('Budget Governance Learning', () => {
    it('should learn from Ollama shift outcomes', () => {
      const outcome1 = {
        category: CynicLearningCategory.BUDGET_GOVERNANCE,
        data: { savings: 2.5, success: true },
      };

      const result = learner.recordOutcome(outcome1);

      assert.strictEqual(result.category, CynicLearningCategory.BUDGET_GOVERNANCE);
      assert.ok(result.modelsUpdated, 'Models updated');

      const stats = learner.getStats();
      assert.strictEqual(stats.models.budget.totalShifts, 1);
      assert.strictEqual(stats.models.budget.successfulShifts, 1);
      assert.strictEqual(stats.models.budget.shiftSuccessRate, 1.0);
      assert.ok(stats.models.budget.avgSavingsFromShift > 0, 'Savings learned');
    });

    it('should predict budget savings after sufficient data', () => {
      // Feed 5 successful shifts
      for (let i = 0; i < 5; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.BUDGET_GOVERNANCE,
          data: { savings: 2.0 + Math.random(), success: true },
        });
      }

      const prediction = learner.predictBudgetSavings();

      assert.ok(prediction.prediction !== null, 'Prediction exists');
      assert.ok(prediction.confidence > 0, 'Confidence > 0');
      assert.ok(prediction.confidence <= PHI_INV, 'Confidence φ-bounded');
      assert.strictEqual(prediction.samples, 5);
    });

    it('should return null prediction with insufficient data', () => {
      // Clear + feed only 2 outcomes
      learner.clear();
      for (let i = 0; i < 2; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.BUDGET_GOVERNANCE,
          data: { savings: 1.5, success: true },
        });
      }

      const prediction = learner.predictBudgetSavings();
      assert.strictEqual(prediction.prediction, null, 'No prediction with <5 samples');
      assert.strictEqual(prediction.confidence, 0);
    });
  });

  describe('Memory Optimization Learning', () => {
    it('should learn from memory optimization outcomes', () => {
      const outcome = {
        category: CynicLearningCategory.MEMORY_OPTIMIZATION,
        data: { memoryFreed: 50, compressionRatio: 0.6, success: true },
      };

      const result = learner.recordOutcome(outcome);

      assert.strictEqual(result.category, CynicLearningCategory.MEMORY_OPTIMIZATION);

      const stats = learner.getStats();
      assert.strictEqual(stats.models.memory.totalOptimizations, 1);
      assert.strictEqual(stats.models.memory.successfulOptimizations, 1);
      assert.ok(stats.models.memory.avgMemoryFreed > 0, 'Memory freed learned');
      assert.ok(stats.models.memory.avgCompressionRatio > 0, 'Compression ratio learned');
    });

    it('should predict memory freed after sufficient data', () => {
      for (let i = 0; i < 5; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.MEMORY_OPTIMIZATION,
          data: { memoryFreed: 40 + i * 5, compressionRatio: 0.6, success: true },
        });
      }

      const prediction = learner.predictMemoryFreed();

      assert.ok(prediction.prediction !== null, 'Prediction exists');
      assert.ok(prediction.confidence > 0, 'Confidence > 0');
      assert.ok(prediction.confidence <= PHI_INV, 'Confidence φ-bounded');
      assert.ok(prediction.compressionRatio > 0, 'Compression ratio included');
    });
  });

  describe('Learning Velocity Learning', () => {
    it('should learn from SONA prioritization outcomes', () => {
      const outcome = {
        category: CynicLearningCategory.LEARNING_VELOCITY,
        data: { velocityChange: 0.05, wasEffective: true },
      };

      const result = learner.recordOutcome(outcome);

      assert.strictEqual(result.category, CynicLearningCategory.LEARNING_VELOCITY);

      const stats = learner.getStats();
      assert.strictEqual(stats.models.learning.totalBoosts, 1);
      assert.strictEqual(stats.models.learning.effectiveBoosts, 1);
      assert.strictEqual(stats.models.learning.sonaBoostEffectiveness, 1.0);
    });

    it('should update boost effectiveness with mixed outcomes', () => {
      // 3 successful, 2 failed
      for (let i = 0; i < 3; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.LEARNING_VELOCITY,
          data: { velocityChange: 0.05, wasEffective: true },
        });
      }
      for (let i = 0; i < 2; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.LEARNING_VELOCITY,
          data: { velocityChange: -0.02, wasEffective: false },
        });
      }

      const stats = learner.getStats();
      assert.strictEqual(stats.models.learning.totalBoosts, 5);
      assert.strictEqual(stats.models.learning.effectiveBoosts, 3);
      assert.strictEqual(stats.models.learning.sonaBoostEffectiveness, 0.6);
    });
  });

  describe('Dog Rotation Learning', () => {
    it('should learn from dog rotation outcomes', () => {
      const outcome = {
        category: CynicLearningCategory.DOG_ROTATION,
        data: { consensusAfter: 0.68, success: true },
      };

      const result = learner.recordOutcome(outcome);

      assert.strictEqual(result.category, CynicLearningCategory.DOG_ROTATION);

      const stats = learner.getStats();
      assert.strictEqual(stats.models.dogs.totalRotations, 1);
      assert.strictEqual(stats.models.dogs.successfulRotations, 1);
      assert.ok(stats.models.dogs.avgConsensusAfterRotation > 0, 'Consensus learned');
    });

    it('should predict when to rotate dogs', () => {
      // Feed 5 successful rotations with high consensus
      for (let i = 0; i < 5; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.DOG_ROTATION,
          data: { consensusAfter: 0.65 + Math.random() * 0.1, success: true },
        });
      }

      const prediction = learner.shouldRotateDogs(0.5); // Current consensus low

      assert.strictEqual(prediction.shouldRotate, true, 'Should rotate at low consensus');
      assert.ok(prediction.confidence > 0, 'Confidence > 0');
      assert.ok(prediction.confidence <= PHI_INV, 'Confidence φ-bounded');
    });

    it('should not recommend rotation with high current consensus', () => {
      for (let i = 0; i < 5; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.DOG_ROTATION,
          data: { consensusAfter: 0.6, success: true },
        });
      }

      const prediction = learner.shouldRotateDogs(0.7); // Current consensus high

      assert.strictEqual(prediction.shouldRotate, false, 'No rotation needed');
    });
  });

  describe('Threshold Calibration Learning', () => {
    it('should learn threshold adjustments', () => {
      const outcome = {
        category: CynicLearningCategory.THRESHOLD_CALIBRATION,
        data: { calibrationDelta: 0.05, adjustment: 'optimal' },
      };

      const result = learner.recordOutcome(outcome);

      assert.strictEqual(result.category, CynicLearningCategory.THRESHOLD_CALIBRATION);

      const stats = learner.getStats();
      assert.strictEqual(stats.models.thresholds.optimalAdjustments, 1);
    });

    it('should track over/under adjustments separately', () => {
      learner.recordOutcome({
        category: CynicLearningCategory.THRESHOLD_CALIBRATION,
        data: { calibrationDelta: 0.1, adjustment: 'over' },
      });
      learner.recordOutcome({
        category: CynicLearningCategory.THRESHOLD_CALIBRATION,
        data: { calibrationDelta: -0.05, adjustment: 'under' },
      });
      learner.recordOutcome({
        category: CynicLearningCategory.THRESHOLD_CALIBRATION,
        data: { calibrationDelta: 0.02, adjustment: 'optimal' },
      });

      const stats = learner.getStats();
      assert.strictEqual(stats.models.thresholds.overAdjustments, 1);
      assert.strictEqual(stats.models.thresholds.underAdjustments, 1);
      assert.strictEqual(stats.models.thresholds.optimalAdjustments, 1);
    });
  });

  describe('Decision Accuracy Learning', () => {
    it('should learn decision accuracy', () => {
      const outcome = {
        category: CynicLearningCategory.DECISION_ACCURACY,
        data: { decisionType: 'shift_to_ollama', wasGood: true },
      };

      const result = learner.recordOutcome(outcome);

      assert.strictEqual(result.category, CynicLearningCategory.DECISION_ACCURACY);

      const stats = learner.getStats();
      assert.strictEqual(stats.models.decisions.totalPredictions, 1);
      assert.strictEqual(stats.models.decisions.correctPredictions, 1);
      assert.strictEqual(stats.models.decisions.accuracy, 1.0);
    });

    it('should track accuracy per decision type', () => {
      learner.recordOutcome({
        category: CynicLearningCategory.DECISION_ACCURACY,
        data: { decisionType: 'shift_to_ollama', wasGood: true },
      });
      learner.recordOutcome({
        category: CynicLearningCategory.DECISION_ACCURACY,
        data: { decisionType: 'shift_to_ollama', wasGood: false },
      });
      learner.recordOutcome({
        category: CynicLearningCategory.DECISION_ACCURACY,
        data: { decisionType: 'compress_context', wasGood: true },
      });

      const stats = learner.getStats();
      const byType = stats.models.decisions.byDecisionType;

      assert.strictEqual(byType.shift_to_ollama.total, 2);
      assert.strictEqual(byType.shift_to_ollama.correct, 1);
      assert.strictEqual(byType.compress_context.total, 1);
      assert.strictEqual(byType.compress_context.correct, 1);
    });

    it('should calculate overall accuracy across types', () => {
      // 7 correct, 3 incorrect = 70%
      for (let i = 0; i < 7; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.DECISION_ACCURACY,
          data: { decisionType: 'various', wasGood: true },
        });
      }
      for (let i = 0; i < 3; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.DECISION_ACCURACY,
          data: { decisionType: 'various', wasGood: false },
        });
      }

      const stats = learner.getStats();
      assert.strictEqual(stats.models.decisions.accuracy, 0.7);
    });
  });

  describe('Health Check', () => {
    it('should report warming_up status with <5 outcomes', () => {
      learner.recordOutcome({
        category: CynicLearningCategory.BUDGET_GOVERNANCE,
        data: { savings: 1.5, success: true },
      });

      const health = learner.getHealth();
      assert.strictEqual(health.status, 'warming_up');
    });

    it('should report learning status with ≥5 outcomes', () => {
      for (let i = 0; i < 5; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.BUDGET_GOVERNANCE,
          data: { savings: 2.0, success: true },
        });
      }

      const health = learner.getHealth();
      assert.strictEqual(health.status, 'learning');
      assert.ok(health.score > 0, 'Score > 0');
      assert.ok(health.score <= PHI_INV, 'Score φ-bounded');
    });

    it('should include aggregate metrics in health', () => {
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
      learner.recordOutcome({
        category: CynicLearningCategory.DOG_ROTATION,
        data: { consensusAfter: 0.65, success: true },
      });
      learner.recordOutcome({
        category: CynicLearningCategory.DECISION_ACCURACY,
        data: { decisionType: 'test', wasGood: true },
      });

      const health = learner.getHealth();
      assert.ok(health.budgetSavings !== undefined, 'Budget savings included');
      assert.ok(health.memoryOptimizations !== undefined, 'Memory optimizations included');
      assert.ok(health.learningBoosts !== undefined, 'Learning boosts included');
      assert.ok(health.dogRotations !== undefined, 'Dog rotations included');
    });
  });

  describe('History Management', () => {
    it('should maintain category-specific histories', () => {
      learner.recordOutcome({
        category: CynicLearningCategory.BUDGET_GOVERNANCE,
        data: { savings: 2.5, success: true },
      });
      learner.recordOutcome({
        category: CynicLearningCategory.MEMORY_OPTIMIZATION,
        data: { memoryFreed: 50, success: true },
      });

      const budgetHistory = learner.getHistory(CynicLearningCategory.BUDGET_GOVERNANCE);
      const memoryHistory = learner.getHistory(CynicLearningCategory.MEMORY_OPTIMIZATION);

      assert.strictEqual(budgetHistory.length, 1, 'Budget history has 1 entry');
      assert.strictEqual(memoryHistory.length, 1, 'Memory history has 1 entry');
      assert.ok(budgetHistory[0].savings !== undefined, 'Budget data preserved');
      assert.ok(memoryHistory[0].freed !== undefined, 'Memory data preserved');
    });

    it('should trim history at maxHistory limit', () => {
      // Factory sets maxHistory: 500
      // Feed 550 outcomes to trigger trim
      for (let i = 0; i < 550; i++) {
        learner.recordOutcome({
          category: CynicLearningCategory.BUDGET_GOVERNANCE,
          data: { savings: 2.0, success: true },
        });
      }

      const history = learner.getHistory(CynicLearningCategory.BUDGET_GOVERNANCE);
      assert.ok(history.length <= 500, 'History trimmed at maxHistory');
    });
  });

  describe('Event Emission', () => {
    it('should emit "learned" event on recordOutcome', (t, done) => {
      learner.once('learned', (result) => {
        assert.strictEqual(result.category, CynicLearningCategory.BUDGET_GOVERNANCE);
        assert.strictEqual(result.cell, 'C6.5');
        assert.strictEqual(result.dimension, 'CYNIC');
        assert.strictEqual(result.analysis, 'LEARN');
        assert.ok(result.modelsUpdated, 'Models updated flag');
        done();
      });

      learner.recordOutcome({
        category: CynicLearningCategory.BUDGET_GOVERNANCE,
        data: { savings: 2.5, success: true },
      });
    });
  });

  describe('Clear/Reset', () => {
    it('should clear all histories and models', () => {
      learner.recordOutcome({
        category: CynicLearningCategory.BUDGET_GOVERNANCE,
        data: { savings: 2.5, success: true },
      });
      learner.recordOutcome({
        category: CynicLearningCategory.MEMORY_OPTIMIZATION,
        data: { memoryFreed: 50, success: true },
      });

      learner.clear();

      const stats = learner.getStats();
      assert.strictEqual(stats.totalOutcomes, 0, 'Total outcomes cleared');
      assert.strictEqual(stats.models.budget.totalShifts, 0, 'Budget stats cleared');
      assert.strictEqual(stats.models.memory.totalOptimizations, 0, 'Memory stats cleared');

      const history = learner.getHistory(CynicLearningCategory.BUDGET_GOVERNANCE);
      assert.strictEqual(history.length, 0, 'History cleared');
    });
  });
});
