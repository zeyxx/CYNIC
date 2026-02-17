/**
 * HumanLearner Config — C5.5 (HUMAN × LEARN)
 *
 * Domain-specific configuration for the Human Learner.
 * Template logic lives in create-learner.js.
 *
 * Learns human patterns over time - preferences, work rhythms, decision styles.
 * Wraps HumanLearning (the core learning system) in φ-factory pattern.
 *
 * "Le chien apprend les habitudes de son maître" - κυνικός
 *
 * @module @cynic/node/cycle/configs/human-learner.config
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

/**
 * Learning categories (from HumanLearning)
 */
export const LearningCategory = {
  TIME_PREFERENCE: 'time_preference',
  COMMUNICATION_STYLE: 'communication_style',
  RISK_TOLERANCE: 'risk_tolerance',
  DOMAIN_EXPERTISE: 'domain_expertise',
  DECISION_PATTERN: 'decision_pattern',
  TOOL_PREFERENCE: 'tool_preference',
  FEEDBACK_PATTERN: 'feedback_pattern',
};

export const humanLearnerConfig = {
  name: 'HumanLearner',
  cell: 'C5.5',
  dimension: 'HUMAN',
  eventPrefix: 'human',
  maxHistory: 89, // Fib(11)
  extraStatFields: ['beliefsFormed', 'predictionsMade', 'predictionsCorrect'],

  init(learner) {
    // Import HumanLearning instance (singleton)
    const { getHumanLearning } = require('../../symbiosis/human-learning.js');
    learner._humanLearning = getHumanLearning();
    learner._beliefsFormed = 0;
    learner._predictionsMade = 0;
    learner._predictionsCorrect = 0;
  },

  /**
   * Learn from an outcome.
   *
   * @param {Object} outcome - Result of an action or observation
   * @param {string} outcome.category - LearningCategory
   * @param {string} outcome.key - Specific behavior
   * @param {any} outcome.value - Observed value
   * @param {Object} [outcome.context] - Additional context
   * @param {Object} [context] - Extra context
   * @param {Object} learner - The learner instance
   * @returns {Object} Learning result
   */
  learn(outcome, context, learner) {
    const category = outcome.category || inferCategory(outcome);
    const key = outcome.key || inferKey(outcome);
    const value = outcome.value !== undefined ? outcome.value : true;

    // Record observation via HumanLearning
    const result = learner._humanLearning.recordObservation(
      category,
      key,
      value,
      { ...outcome.context, ...context }
    );

    if (result.belief && result.belief.observations === 1) {
      learner._beliefsFormed++;
    }

    return {
      recorded: true,
      category,
      key,
      value,
      belief: result.belief,
      timestamp: Date.now(),
    };
  },

  /**
   * Predict human preference.
   *
   * @param {Object} query - What to predict
   * @param {string} query.category - LearningCategory
   * @param {string} query.key - Behavior key
   * @param {any} [query.defaultValue] - Default if no belief
   * @param {Object} learner - The learner instance
   * @returns {Object} Prediction
   */
  predict(query, learner) {
    learner._predictionsMade++;

    const prediction = learner._humanLearning.predict(
      query.category,
      query.key,
      query.defaultValue || null
    );

    return {
      predicted: true,
      category: query.category,
      key: query.key,
      value: prediction.value,
      confidence: prediction.confidence,
      source: prediction.source,
      belief: prediction.belief,
    };
  },

  /**
   * Record prediction outcome (for accuracy tracking).
   *
   * @param {Object} outcome - Actual outcome
   * @param {string} outcome.category - Category predicted
   * @param {string} outcome.key - Key predicted
   * @param {boolean} outcome.correct - Was prediction correct?
   * @param {Object} learner - The learner instance
   */
  recordPredictionOutcome(outcome, learner) {
    if (outcome.correct) {
      learner._predictionsCorrect++;
    }

    learner._humanLearning.recordPredictionOutcome(
      outcome.category,
      outcome.key,
      outcome.correct
    );
  },

  /**
   * Get learning stats.
   */
  getStats(learner) {
    const coreStats = learner._humanLearning.getStats();

    return {
      observations: coreStats.observationsRecorded,
      beliefs: coreStats.totalBeliefs,
      strongBeliefs: coreStats.strongBeliefs,
      predictionsMade: learner._predictionsMade,
      predictionsCorrect: learner._predictionsCorrect,
      predictionAccuracy: learner._predictionsMade > 0
        ? learner._predictionsCorrect / learner._predictionsMade
        : null,
      lastObservation: coreStats.lastObservation,
    };
  },

  updateExtraStats(stats, result) {
    // beliefsFormed, predictionsMade, predictionsCorrect tracked in init
  },

  getHealth(learner) {
    const stats = learner._humanLearning.getStats();

    return {
      status: stats.totalBeliefs >= 10 ? 'learning'
        : stats.totalBeliefs >= 3 ? 'forming'
          : 'nascent',
      score: Math.min(PHI_INV, stats.totalBeliefs * 0.05), // 5% per belief, cap at φ⁻¹
      beliefs: stats.totalBeliefs,
      strongBeliefs: stats.strongBeliefs,
      observations: stats.observationsRecorded,
    };
  },
};

// =============================================================================
// Inference helpers (when outcome doesn't specify category/key explicitly)
// =============================================================================

function inferCategory(outcome) {
  // Try to infer category from outcome structure
  if (outcome.type === 'tool_usage') return LearningCategory.TOOL_PREFERENCE;
  if (outcome.type === 'time_pattern') return LearningCategory.TIME_PREFERENCE;
  if (outcome.type === 'communication') return LearningCategory.COMMUNICATION_STYLE;
  if (outcome.type === 'decision') return LearningCategory.DECISION_PATTERN;
  if (outcome.type === 'feedback') return LearningCategory.FEEDBACK_PATTERN;
  if (outcome.type === 'domain') return LearningCategory.DOMAIN_EXPERTISE;

  // Default
  return LearningCategory.DECISION_PATTERN;
}

function inferKey(outcome) {
  // Try to infer key from outcome
  if (outcome.action) return outcome.action;
  if (outcome.tool) return outcome.tool;
  if (outcome.domain) return outcome.domain;
  if (outcome.type) return outcome.type;

  // Fallback
  return 'unknown';
}
