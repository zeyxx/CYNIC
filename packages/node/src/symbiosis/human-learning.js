/**
 * Human Learning - C5.5 (HUMAN × LEARN)
 *
 * Learns human patterns over time - preferences, work rhythms, decision styles.
 * Part of the 7×7 Fractal Matrix symbiosis layer.
 *
 * "Le chien apprend les habitudes de son maître" - κυνικός
 *
 * Learns:
 * - Time preferences (when human is most productive)
 * - Communication style (verbose vs concise responses)
 * - Risk tolerance (conservative vs adventurous)
 * - Domain expertise (areas of knowledge)
 * - Decision patterns (fast vs deliberate)
 *
 * @module @cynic/node/symbiosis/human-learning
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

/**
 * Learning categories
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

/**
 * φ-aligned learning constants
 */
const LEARNING_CONFIG = {
  // Minimum observations before forming belief
  minObservations: 5,

  // Learning rate (φ⁻² for gradual adaptation)
  learningRate: PHI_INV_2,

  // Decay rate for old observations (per day)
  decayRate: 0.95,

  // Maximum beliefs per category
  maxBeliefsPerCategory: 21, // Fib(8)

  // Confidence threshold for acting on belief
  confidenceThreshold: PHI_INV_2, // 38.2%

  // Strong confidence threshold
  strongConfidence: PHI_INV, // 61.8%
};

/**
 * Human Learning Service
 */
export class HumanLearning extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Object} [options.persistence] - Persistence layer
   * @param {string} [options.userId] - User identifier
   */
  constructor(options = {}) {
    super();

    this._persistence = options.persistence || null;
    this._userId = options.userId || 'default';

    // Beliefs organized by category
    this._beliefs = new Map();
    for (const category of Object.values(LearningCategory)) {
      this._beliefs.set(category, new Map());
    }

    // Observation history (for pattern detection)
    this._observations = [];
    this._maxObservations = 500;

    // Statistics
    this._stats = {
      observationsRecorded: 0,
      beliefsFormed: 0,
      beliefsUpdated: 0,
      predictionsCorrect: 0,
      predictionsMade: 0,
      lastObservation: null,
    };
  }

  /**
   * Record an observation about human behavior
   *
   * @param {string} category - LearningCategory
   * @param {string} key - Specific behavior (e.g., 'prefers_morning')
   * @param {any} value - Observed value
   * @param {Object} [context] - Additional context
   * @returns {Object} Recording result
   */
  recordObservation(category, key, value, context = {}) {
    const timestamp = Date.now();

    const observation = {
      category,
      key,
      value,
      context,
      timestamp,
      sessionId: context.sessionId || null,
    };

    // Add to history
    this._observations.push(observation);
    this._stats.observationsRecorded++;
    this._stats.lastObservation = timestamp;

    // Trim history
    while (this._observations.length > this._maxObservations) {
      this._observations.shift();
    }

    // Update belief
    const belief = this._updateBelief(category, key, value, context);

    this.emit('observation', { observation, belief });

    return {
      recorded: true,
      belief: belief ? {
        key: belief.key,
        confidence: belief.confidence,
        observations: belief.observations,
      } : null,
    };
  }

  /**
   * Update or create belief based on observation
   * @private
   */
  _updateBelief(category, key, value, context) {
    const categoryBeliefs = this._beliefs.get(category);
    if (!categoryBeliefs) return null;

    let belief = categoryBeliefs.get(key);

    if (!belief) {
      // Create new belief
      belief = {
        key,
        category,
        values: [],
        observations: 0,
        confidence: 0,
        dominantValue: null,
        createdAt: Date.now(),
        updatedAt: Date.now(),
      };
      categoryBeliefs.set(key, belief);
      this._stats.beliefsFormed++;
    }

    // Add value to history
    belief.values.push({
      value,
      timestamp: Date.now(),
      context,
    });

    // Trim old values
    while (belief.values.length > LEARNING_CONFIG.maxBeliefsPerCategory) {
      belief.values.shift();
    }

    belief.observations++;
    belief.updatedAt = Date.now();

    // Calculate dominant value and confidence
    this._calculateBeliefStrength(belief);

    this._stats.beliefsUpdated++;

    return belief;
  }

  /**
   * Calculate belief strength and dominant value
   * @private
   */
  _calculateBeliefStrength(belief) {
    if (belief.values.length < LEARNING_CONFIG.minObservations) {
      belief.confidence = 0;
      belief.dominantValue = null;
      return;
    }

    // Apply time decay to values
    const now = Date.now();
    const weightedValues = new Map();

    for (const entry of belief.values) {
      const ageDays = (now - entry.timestamp) / (24 * 60 * 60 * 1000);
      const weight = Math.pow(LEARNING_CONFIG.decayRate, ageDays);

      const valueKey = JSON.stringify(entry.value);
      const current = weightedValues.get(valueKey) || { value: entry.value, weight: 0 };
      current.weight += weight;
      weightedValues.set(valueKey, current);
    }

    // Find dominant value
    let maxWeight = 0;
    let totalWeight = 0;

    for (const [, data] of weightedValues) {
      totalWeight += data.weight;
      if (data.weight > maxWeight) {
        maxWeight = data.weight;
        belief.dominantValue = data.value;
      }
    }

    // Calculate confidence (proportion of dominant value)
    belief.confidence = totalWeight > 0
      ? Math.min(PHI_INV, maxWeight / totalWeight)
      : 0;
  }

  /**
   * Get belief about human behavior
   *
   * @param {string} category - LearningCategory
   * @param {string} key - Specific behavior
   * @returns {Object|null} Belief or null
   */
  getBelief(category, key) {
    const categoryBeliefs = this._beliefs.get(category);
    if (!categoryBeliefs) return null;

    const belief = categoryBeliefs.get(key);
    if (!belief) return null;

    return {
      key: belief.key,
      category: belief.category,
      dominantValue: belief.dominantValue,
      confidence: belief.confidence,
      observations: belief.observations,
      isStrong: belief.confidence >= LEARNING_CONFIG.strongConfidence,
      isActionable: belief.confidence >= LEARNING_CONFIG.confidenceThreshold,
    };
  }

  /**
   * Get all beliefs for a category
   *
   * @param {string} category - LearningCategory
   * @returns {Object[]} Array of beliefs
   */
  getCategoryBeliefs(category) {
    const categoryBeliefs = this._beliefs.get(category);
    if (!categoryBeliefs) return [];

    return Array.from(categoryBeliefs.values())
      .filter(b => b.confidence > 0)
      .map(b => ({
        key: b.key,
        dominantValue: b.dominantValue,
        confidence: b.confidence,
        observations: b.observations,
        isStrong: b.confidence >= LEARNING_CONFIG.strongConfidence,
      }))
      .sort((a, b) => b.confidence - a.confidence);
  }

  /**
   * Predict human preference
   *
   * @param {string} category - LearningCategory
   * @param {string} key - What to predict
   * @param {any} defaultValue - Default if no belief
   * @returns {Object} Prediction
   */
  predict(category, key, defaultValue = null) {
    const belief = this.getBelief(category, key);
    this._stats.predictionsMade++;

    if (!belief || !belief.isActionable) {
      return {
        value: defaultValue,
        confidence: 0,
        source: 'default',
        belief: null,
      };
    }

    return {
      value: belief.dominantValue,
      confidence: belief.confidence,
      source: belief.isStrong ? 'strong_belief' : 'belief',
      belief,
    };
  }

  /**
   * Record prediction outcome for learning
   *
   * @param {string} category - Category of prediction
   * @param {string} key - Prediction key
   * @param {boolean} wasCorrect - Was prediction correct?
   */
  recordPredictionOutcome(category, key, wasCorrect) {
    if (wasCorrect) {
      this._stats.predictionsCorrect++;
    }

    // Could adjust learning rate based on accuracy
    this.emit('prediction_outcome', { category, key, wasCorrect });
  }

  /**
   * Get time preference profile
   */
  getTimeProfile() {
    const timeBeliefs = this.getCategoryBeliefs(LearningCategory.TIME_PREFERENCE);

    return {
      peakProductivityHour: this.predict(LearningCategory.TIME_PREFERENCE, 'peak_hour', 10).value,
      lowEnergyHour: this.predict(LearningCategory.TIME_PREFERENCE, 'low_hour', 14).value,
      preferredSessionLength: this.predict(LearningCategory.TIME_PREFERENCE, 'session_length', 90).value,
      breakFrequency: this.predict(LearningCategory.TIME_PREFERENCE, 'break_frequency', 60).value,
      beliefs: timeBeliefs,
    };
  }

  /**
   * Get communication style profile
   */
  getCommunicationProfile() {
    return {
      verbosity: this.predict(LearningCategory.COMMUNICATION_STYLE, 'verbosity', 'balanced').value,
      prefersTechnical: this.predict(LearningCategory.COMMUNICATION_STYLE, 'technical', true).value,
      likesEmoji: this.predict(LearningCategory.COMMUNICATION_STYLE, 'emoji', false).value,
      prefersExamples: this.predict(LearningCategory.COMMUNICATION_STYLE, 'examples', true).value,
      beliefs: this.getCategoryBeliefs(LearningCategory.COMMUNICATION_STYLE),
    };
  }

  /**
   * Get expertise profile
   */
  getExpertiseProfile() {
    const expertiseBeliefs = this.getCategoryBeliefs(LearningCategory.DOMAIN_EXPERTISE);

    // Extract domains with high confidence
    const strongDomains = expertiseBeliefs
      .filter(b => b.isStrong && b.dominantValue === true)
      .map(b => b.key);

    const growingDomains = expertiseBeliefs
      .filter(b => !b.isStrong && b.dominantValue === true)
      .map(b => b.key);

    return {
      strongDomains,
      growingDomains,
      totalDomains: expertiseBeliefs.length,
      beliefs: expertiseBeliefs,
    };
  }

  /**
   * Get full human profile
   */
  getProfile() {
    return {
      userId: this._userId,
      time: this.getTimeProfile(),
      communication: this.getCommunicationProfile(),
      expertise: this.getExpertiseProfile(),
      riskTolerance: this.predict(LearningCategory.RISK_TOLERANCE, 'level', 'moderate').value,
      decisionStyle: this.predict(LearningCategory.DECISION_PATTERN, 'style', 'deliberate').value,
      stats: this.getStats(),
    };
  }

  /**
   * Get learning statistics
   */
  getStats() {
    let totalBeliefs = 0;
    let strongBeliefs = 0;

    for (const [, categoryBeliefs] of this._beliefs) {
      for (const [, belief] of categoryBeliefs) {
        if (belief.confidence > 0) {
          totalBeliefs++;
          if (belief.confidence >= LEARNING_CONFIG.strongConfidence) {
            strongBeliefs++;
          }
        }
      }
    }

    return {
      ...this._stats,
      totalBeliefs,
      strongBeliefs,
      predictionAccuracy: this._stats.predictionsMade > 0
        ? this._stats.predictionsCorrect / this._stats.predictionsMade
        : null,
    };
  }

  /**
   * Export beliefs for persistence
   */
  exportBeliefs() {
    const exported = {};

    for (const [category, beliefs] of this._beliefs) {
      exported[category] = {};
      for (const [key, belief] of beliefs) {
        exported[category][key] = {
          dominantValue: belief.dominantValue,
          confidence: belief.confidence,
          observations: belief.observations,
          updatedAt: belief.updatedAt,
        };
      }
    }

    return exported;
  }

  /**
   * Import beliefs from persistence
   */
  importBeliefs(data) {
    for (const [category, beliefs] of Object.entries(data)) {
      const categoryBeliefs = this._beliefs.get(category);
      if (!categoryBeliefs) continue;

      for (const [key, belief] of Object.entries(beliefs)) {
        categoryBeliefs.set(key, {
          key,
          category,
          values: [], // Will rebuild from new observations
          observations: belief.observations || 0,
          confidence: belief.confidence || 0,
          dominantValue: belief.dominantValue,
          createdAt: belief.updatedAt || Date.now(),
          updatedAt: belief.updatedAt || Date.now(),
        });
      }
    }
  }

  /**
   * Clear all learning
   */
  clear() {
    for (const [, categoryBeliefs] of this._beliefs) {
      categoryBeliefs.clear();
    }
    this._observations = [];
    this._stats = {
      observationsRecorded: 0,
      beliefsFormed: 0,
      beliefsUpdated: 0,
      predictionsCorrect: 0,
      predictionsMade: 0,
      lastObservation: null,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create HumanLearning singleton
 */
export function getHumanLearning(options = {}) {
  if (!_instance) {
    _instance = new HumanLearning(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetHumanLearning() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  HumanLearning,
  LearningCategory,
  getHumanLearning,
  resetHumanLearning,
};
