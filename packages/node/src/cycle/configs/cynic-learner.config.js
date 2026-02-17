/**
 * CynicLearner Config — C6.5 (CYNIC × LEARN)
 *
 * Domain-specific configuration for the CYNIC Learner.
 * Template logic lives in create-learner.js.
 *
 * Learns from C6 self-observation cycle outcomes:
 * - Budget governance effectiveness (Ollama shifts, cost savings)
 * - Memory optimization impact (GC results, compression ratios)
 * - Learning velocity changes (SONA prioritization outcomes)
 * - Dog rotation effects (consensus quality after rotation)
 * - Threshold calibration (decision accuracy vs outcomes)
 *
 * Closes the C6 feedback loop: outcomes → learning → better self-governance.
 *
 * @module @cynic/node/cycle/configs/cynic-learner.config
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

export const CynicLearningCategory = {
  BUDGET_GOVERNANCE: 'budget_governance',
  MEMORY_OPTIMIZATION: 'memory_optimization',
  LEARNING_VELOCITY: 'learning_velocity',
  DOG_ROTATION: 'dog_rotation',
  THRESHOLD_CALIBRATION: 'threshold_calibration',
  DECISION_ACCURACY: 'decision_accuracy',
};

export const cynicLearnerConfig = {
  name: 'CynicLearner',
  cell: 'C6.5',
  dimension: 'CYNIC',
  eventPrefix: 'cynic',
  categories: CynicLearningCategory,
  minObservations: 5,
  learningRate: PHI_INV_2, // 38.2%
  maxHistory: 500,

  initModels() {
    return {
      budget: {
        avgSavingsFromShift: 0,
        shiftSuccessRate: 0,
        totalShifts: 0,
        successfulShifts: 0,
        samples: 0,
      },
      memory: {
        avgCompressionRatio: 0,
        avgMemoryFreed: 0,
        gcSuccessRate: 0,
        totalOptimizations: 0,
        successfulOptimizations: 0,
        samples: 0,
      },
      learning: {
        avgVelocityChange: 0,
        sonaBoostEffectiveness: 0,
        totalBoosts: 0,
        effectiveBoosts: 0,
        samples: 0,
      },
      dogs: {
        avgConsensusAfterRotation: 0,
        rotationSuccessRate: 0,
        totalRotations: 0,
        successfulRotations: 0,
        samples: 0,
      },
      thresholds: {
        avgCalibrationDelta: 0,
        overAdjustments: 0,
        underAdjustments: 0,
        optimalAdjustments: 0,
        samples: 0,
      },
      decisions: {
        accuracy: 0,
        totalPredictions: 0,
        correctPredictions: 0,
        byDecisionType: {},
      },
    };
  },

  learn(category, data, models, histories, { learningRate, maxHistory }) {
    switch (category) {
      case CynicLearningCategory.BUDGET_GOVERNANCE:
        learnBudgetGovernance(data, models, histories, learningRate);
        break;
      case CynicLearningCategory.MEMORY_OPTIMIZATION:
        learnMemoryOptimization(data, models, histories, learningRate);
        break;
      case CynicLearningCategory.LEARNING_VELOCITY:
        learnLearningVelocity(data, models, histories, learningRate);
        break;
      case CynicLearningCategory.DOG_ROTATION:
        learnDogRotation(data, models, histories, learningRate);
        break;
      case CynicLearningCategory.THRESHOLD_CALIBRATION:
        learnThresholdCalibration(data, models, histories, learningRate);
        break;
      case CynicLearningCategory.DECISION_ACCURACY:
        learnDecisionAccuracy(data, models, histories);
        break;
    }
  },

  predictions: {
    predictBudgetSavings(models, stats) {
      const m = models.budget;
      if (m.samples < 5) {
        return { prediction: null, confidence: 0, reason: 'Insufficient budget data' };
      }

      const prediction = m.avgSavingsFromShift;
      const confidence = Math.min(
        PHI_INV,
        (m.samples / (m.samples + 10)) * m.shiftSuccessRate
      );

      stats.predictionsAttempted++;

      return {
        prediction: Math.round(prediction * 100) / 100,
        confidence,
        successRate: m.shiftSuccessRate,
        samples: m.samples,
      };
    },

    predictMemoryFreed(models, stats, optimizationType) {
      const m = models.memory;
      if (m.samples < 5) {
        return { prediction: null, confidence: 0, reason: 'Insufficient memory data' };
      }

      const prediction = m.avgMemoryFreed;
      const confidence = Math.min(
        PHI_INV,
        (m.samples / (m.samples + 10)) * m.gcSuccessRate
      );

      stats.predictionsAttempted++;

      return {
        prediction: Math.round(prediction * 100) / 100,
        confidence,
        compressionRatio: m.avgCompressionRatio,
        successRate: m.gcSuccessRate,
        samples: m.samples,
      };
    },

    shouldRotateDogs(models, stats, currentConsensus) {
      const m = models.dogs;
      if (m.samples < 3) {
        return {
          shouldRotate: currentConsensus < PHI_INV_2,
          confidence: PHI_INV_3,
          reason: 'Using φ⁻² default threshold',
        };
      }

      const shouldRotate = currentConsensus < m.avgConsensusAfterRotation;
      const confidence = Math.min(
        PHI_INV,
        (m.samples / (m.samples + 8)) * m.rotationSuccessRate
      );

      stats.predictionsAttempted++;

      return {
        shouldRotate,
        confidence,
        avgConsensusAfterRotation: m.avgConsensusAfterRotation,
        currentConsensus,
        samples: m.samples,
      };
    },
  },

  healthCheck(stats, models) {
    const total = stats.totalOutcomes;
    const accuracy = models.decisions.accuracy;

    // Health degrades if decisions are inaccurate
    const overallSuccessRate =
      (models.budget.shiftSuccessRate +
        models.memory.gcSuccessRate +
        models.learning.sonaBoostEffectiveness +
        models.dogs.rotationSuccessRate) /
      4;

    return {
      status: total >= 5 ? 'learning' : 'warming_up',
      score: Math.min(PHI_INV, accuracy || overallSuccessRate || 0.3),
      totalOutcomes: total,
      decisionAccuracy: accuracy,
      budgetSavings: models.budget.avgSavingsFromShift,
      memoryOptimizations: models.memory.totalOptimizations,
      learningBoosts: models.learning.totalBoosts,
      dogRotations: models.dogs.totalRotations,
    };
  },
};

// =============================================================================
// Learning functions
// =============================================================================

function learnBudgetGovernance(data, models, histories, learningRate) {
  const cat = CynicLearningCategory.BUDGET_GOVERNANCE;
  const savings = data.savings ?? data.costSaved ?? 0;
  const wasSuccessful = data.success ?? data.wasEffective ?? (savings > 0);

  histories[cat].push({ savings, wasSuccessful, timestamp: Date.now() });

  const m = models.budget;
  m.samples++;
  m.totalShifts++;
  if (wasSuccessful) m.successfulShifts++;

  m.avgSavingsFromShift =
    m.avgSavingsFromShift * (1 - learningRate) + savings * learningRate;
  m.shiftSuccessRate =
    m.totalShifts > 0 ? m.successfulShifts / m.totalShifts : 0;
}

function learnMemoryOptimization(data, models, histories, learningRate) {
  const cat = CynicLearningCategory.MEMORY_OPTIMIZATION;
  const freed = data.memoryFreed ?? data.freed ?? 0;
  const compressionRatio = data.compressionRatio ?? data.ratio ?? 1;
  const wasSuccessful = data.success ?? data.wasEffective ?? (freed > 0);

  histories[cat].push({
    freed,
    compressionRatio,
    wasSuccessful,
    timestamp: Date.now(),
  });

  const m = models.memory;
  m.samples++;
  m.totalOptimizations++;
  if (wasSuccessful) m.successfulOptimizations++;

  m.avgMemoryFreed = m.avgMemoryFreed * (1 - learningRate) + freed * learningRate;
  m.avgCompressionRatio =
    m.avgCompressionRatio * (1 - learningRate) + compressionRatio * learningRate;
  m.gcSuccessRate =
    m.totalOptimizations > 0
      ? m.successfulOptimizations / m.totalOptimizations
      : 0;
}

function learnLearningVelocity(data, models, histories, learningRate) {
  const cat = CynicLearningCategory.LEARNING_VELOCITY;
  const velocityChange = data.velocityChange ?? data.delta ?? 0;
  const wasEffective = data.wasEffective ?? data.success ?? (velocityChange > 0);

  histories[cat].push({ velocityChange, wasEffective, timestamp: Date.now() });

  const m = models.learning;
  m.samples++;
  m.totalBoosts++;
  if (wasEffective) m.effectiveBoosts++;

  m.avgVelocityChange =
    m.avgVelocityChange * (1 - learningRate) + velocityChange * learningRate;
  m.sonaBoostEffectiveness =
    m.totalBoosts > 0 ? m.effectiveBoosts / m.totalBoosts : 0;
}

function learnDogRotation(data, models, histories, learningRate) {
  const cat = CynicLearningCategory.DOG_ROTATION;
  const consensusAfter = data.consensusAfter ?? data.consensus ?? 0;
  const wasSuccessful =
    data.success ?? data.wasEffective ?? consensusAfter > PHI_INV_2;

  histories[cat].push({
    consensusAfter,
    wasSuccessful,
    timestamp: Date.now(),
  });

  const m = models.dogs;
  m.samples++;
  m.totalRotations++;
  if (wasSuccessful) m.successfulRotations++;

  m.avgConsensusAfterRotation =
    m.avgConsensusAfterRotation * (1 - learningRate) +
    consensusAfter * learningRate;
  m.rotationSuccessRate =
    m.totalRotations > 0 ? m.successfulRotations / m.totalRotations : 0;
}

function learnThresholdCalibration(data, models, histories, learningRate) {
  const cat = CynicLearningCategory.THRESHOLD_CALIBRATION;
  const delta = data.calibrationDelta ?? data.delta ?? 0;
  const adjustment = data.adjustment || 'optimal';

  histories[cat].push({ delta, adjustment, timestamp: Date.now() });

  const m = models.thresholds;
  m.samples++;

  if (adjustment === 'over') m.overAdjustments++;
  else if (adjustment === 'under') m.underAdjustments++;
  else m.optimalAdjustments++;

  m.avgCalibrationDelta =
    m.avgCalibrationDelta * (1 - learningRate) + Math.abs(delta) * learningRate;
}

function learnDecisionAccuracy(data, models, histories) {
  const cat = CynicLearningCategory.DECISION_ACCURACY;
  const decisionType = data.decisionType || data.type || 'unknown';
  const wasGood = data.wasGood ?? data.outcome === 'positive';

  histories[cat].push({ decisionType, wasGood, timestamp: Date.now() });

  const m = models.decisions;
  m.totalPredictions++;
  if (wasGood) m.correctPredictions++;
  m.accuracy =
    m.totalPredictions > 0 ? m.correctPredictions / m.totalPredictions : 0;

  // Track per decision type
  if (!m.byDecisionType[decisionType]) {
    m.byDecisionType[decisionType] = { total: 0, correct: 0 };
  }
  m.byDecisionType[decisionType].total++;
  if (wasGood) m.byDecisionType[decisionType].correct++;
}
