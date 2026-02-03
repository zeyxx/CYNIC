/**
 * ChaosGenerator - Chaos Engineering for CYNIC
 *
 * "Un système qui survit au hasard survit à tout"
 *
 * Injects controlled randomness to test system robustness:
 * - Random planning triggers (φ⁻³ probability)
 * - Adversarial scenario injection (φ⁻⁴ probability)
 * - Alternative validation
 * - Learning from chaos results
 *
 * Philosophy: A system that survives chaos survives everything.
 * The best way to find weaknesses is to randomly stress-test.
 *
 * @module @cynic/node/chaos/chaos-generator
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

const log = createLogger('ChaosGenerator');

// φ⁻⁴ ≈ 0.146
const PHI_INV_4 = PHI_INV * PHI_INV_3;

/**
 * Chaos configuration
 */
export const CHAOS_CONFIG = {
  /** Probability of forcing planning (φ⁻³ ≈ 23.6%) */
  PLANNING_PROBABILITY: PHI_INV_3,
  /** Probability of injecting failure (φ⁻⁴ ≈ 14.6%) */
  FAILURE_PROBABILITY: PHI_INV_4,
  /** Probability of adversarial scenario (φ⁻⁴ ≈ 14.6%) */
  ADVERSARIAL_PROBABILITY: PHI_INV_4,
  /** Max chaos events per minute (rate limiting) */
  MAX_EVENTS_PER_MINUTE: 5,
  /** Cooldown after chaos event (ms) */
  COOLDOWN_MS: 10000,
};

/**
 * Chaos event types
 * @enum {string}
 */
export const ChaosEventType = {
  FORCE_PLANNING: 'force_planning',
  INJECT_FAILURE: 'inject_failure',
  ADVERSARIAL_SCENARIO: 'adversarial_scenario',
  DELAY_INJECTION: 'delay_injection',
  CONFIDENCE_PERTURBATION: 'confidence_perturbation',
};

/**
 * Adversarial scenarios for testing
 */
const ADVERSARIAL_SCENARIOS = [
  {
    id: 'ambiguous_request',
    description: 'Request that could be interpreted multiple ways',
    modification: (content) => `${content} (this could mean several things)`,
  },
  {
    id: 'conflicting_context',
    description: 'Add conflicting information to context',
    modification: (content) => `${content} [Note: previous instructions may conflict]`,
  },
  {
    id: 'edge_case',
    description: 'Push towards edge case handling',
    modification: (content) => `Edge case: ${content}`,
  },
  {
    id: 'time_pressure',
    description: 'Add artificial urgency',
    modification: (content) => `URGENT: ${content}`,
  },
];

/**
 * Chaos result - what happened when chaos was injected
 */
export class ChaosResult {
  constructor(data = {}) {
    this.id = data.id || `chaos-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    this.type = data.type || ChaosEventType.FORCE_PLANNING;
    this.injected = data.injected || false;
    this.reason = data.reason || null;
    this.scenario = data.scenario || null;
    this.originalConfidence = data.originalConfidence || 0;
    this.perturbedConfidence = data.perturbedConfidence || null;
    this.timestamp = Date.now();
  }

  toJSON() {
    return {
      id: this.id,
      type: this.type,
      injected: this.injected,
      reason: this.reason,
      scenario: this.scenario?.id || null,
      timestamp: this.timestamp,
    };
  }
}

/**
 * ChaosGenerator - Controlled chaos for robustness testing
 *
 * Randomly injects chaos events to test system resilience.
 * All chaos is controlled and logged for learning.
 */
export class ChaosGenerator extends EventEmitter {
  /**
   * Create a ChaosGenerator
   *
   * @param {Object} options
   * @param {boolean} [options.enabled=false] - Enable chaos (default disabled for safety)
   * @param {Object} [options.config] - Custom config overrides
   * @param {Object} [options.learningService] - Learning service for recording results
   */
  constructor(options = {}) {
    super();

    this.enabled = options.enabled ?? false;
    this.config = { ...CHAOS_CONFIG, ...options.config };
    this.learningService = options.learningService || null;

    // Rate limiting
    this._recentEvents = [];
    this._lastEventTime = 0;

    // Statistics
    this.stats = {
      checks: 0,
      injected: 0,
      byType: {},
      survived: 0,
      failed: 0,
    };

    // Results history (for learning)
    this._results = [];
    this._maxResults = 100;

    log.debug('ChaosGenerator created', { enabled: this.enabled });
  }

  /**
   * Check if planning should be forced (chaos injection)
   *
   * @param {Object} task - Task context
   * @param {string} task.content - Task content
   * @param {number} [task.complexity] - Task complexity (0-1)
   * @param {number} [task.confidence] - Current confidence (0-1)
   * @returns {ChaosResult}
   */
  shouldForcePlanning(task) {
    this.stats.checks++;

    if (!this.enabled) {
      return new ChaosResult({ type: ChaosEventType.FORCE_PLANNING, injected: false });
    }

    // Rate limiting
    if (!this._canInjectChaos()) {
      return new ChaosResult({
        type: ChaosEventType.FORCE_PLANNING,
        injected: false,
        reason: 'rate_limited',
      });
    }

    // Random check with φ⁻³ probability
    const roll = Math.random();
    const force = roll < this.config.PLANNING_PROBABILITY;

    const result = new ChaosResult({
      type: ChaosEventType.FORCE_PLANNING,
      injected: force,
      reason: force ? 'chaos_test' : 'probability_miss',
      originalConfidence: task.confidence || 0,
    });

    if (force) {
      this._recordChaosEvent(result);
      log.info('Chaos: Forcing planning', { taskContent: task.content?.substring(0, 50) });
      this.emit('chaos:inject', result);
    }

    return result;
  }

  /**
   * Inject an adversarial scenario
   *
   * @param {Object} context - Current context
   * @param {string} context.content - Original content
   * @returns {ChaosResult}
   */
  injectAdversarialScenario(context) {
    this.stats.checks++;

    if (!this.enabled) {
      return new ChaosResult({ type: ChaosEventType.ADVERSARIAL_SCENARIO, injected: false });
    }

    if (!this._canInjectChaos()) {
      return new ChaosResult({
        type: ChaosEventType.ADVERSARIAL_SCENARIO,
        injected: false,
        reason: 'rate_limited',
      });
    }

    const roll = Math.random();
    const inject = roll < this.config.ADVERSARIAL_PROBABILITY;

    if (!inject) {
      return new ChaosResult({
        type: ChaosEventType.ADVERSARIAL_SCENARIO,
        injected: false,
        reason: 'probability_miss',
      });
    }

    // Select random scenario
    const scenario = ADVERSARIAL_SCENARIOS[
      Math.floor(Math.random() * ADVERSARIAL_SCENARIOS.length)
    ];

    const result = new ChaosResult({
      type: ChaosEventType.ADVERSARIAL_SCENARIO,
      injected: true,
      reason: 'chaos_test',
      scenario,
    });

    this._recordChaosEvent(result);
    log.info('Chaos: Injecting adversarial scenario', { scenario: scenario.id });
    this.emit('chaos:inject', result);

    return result;
  }

  /**
   * Perturb confidence value
   *
   * @param {number} confidence - Original confidence (0-1)
   * @returns {ChaosResult}
   */
  perturbConfidence(confidence) {
    this.stats.checks++;

    if (!this.enabled) {
      return new ChaosResult({
        type: ChaosEventType.CONFIDENCE_PERTURBATION,
        injected: false,
        originalConfidence: confidence,
        perturbedConfidence: confidence,
      });
    }

    const roll = Math.random();
    const perturb = roll < this.config.ADVERSARIAL_PROBABILITY;

    if (!perturb) {
      return new ChaosResult({
        type: ChaosEventType.CONFIDENCE_PERTURBATION,
        injected: false,
        reason: 'probability_miss',
        originalConfidence: confidence,
        perturbedConfidence: confidence,
      });
    }

    // Perturb by ±20% (but cap at φ⁻¹)
    const perturbation = (Math.random() - 0.5) * 0.4;
    const perturbed = Math.max(0, Math.min(PHI_INV, confidence + perturbation));

    const result = new ChaosResult({
      type: ChaosEventType.CONFIDENCE_PERTURBATION,
      injected: true,
      reason: 'chaos_test',
      originalConfidence: confidence,
      perturbedConfidence: perturbed,
    });

    this._recordChaosEvent(result);
    log.debug('Chaos: Perturbed confidence', { from: confidence, to: perturbed });

    return result;
  }

  /**
   * Validate that system handled alternatives correctly
   *
   * @param {Object} primary - Primary decision
   * @param {Object[]} alternatives - Generated alternatives
   * @returns {Object} Validation result
   */
  validateAlternatives(primary, alternatives) {
    const validation = {
      valid: true,
      issues: [],
      coverage: 0,
    };

    // Check that alternatives exist
    if (!alternatives || alternatives.length === 0) {
      validation.valid = false;
      validation.issues.push('No alternatives generated');
      return validation;
    }

    // Check that alternatives are different from primary
    for (const alt of alternatives) {
      if (alt.id === primary?.id || alt.label === primary?.label) {
        validation.issues.push('Alternative matches primary');
      }
    }

    // Check diversity (rough heuristic)
    const uniqueLabels = new Set(alternatives.map(a => a.label || a.id));
    validation.coverage = uniqueLabels.size / alternatives.length;

    if (validation.coverage < 0.8) {
      validation.issues.push('Low alternative diversity');
    }

    validation.valid = validation.issues.length === 0;

    return validation;
  }

  /**
   * Record the outcome of a chaos event
   *
   * @param {string} chaosId - Chaos event ID
   * @param {Object} outcome - What happened
   * @param {boolean} outcome.survived - Did system handle it gracefully?
   * @param {string} [outcome.error] - Error if failed
   */
  recordChaosResult(chaosId, outcome) {
    const result = this._results.find(r => r.id === chaosId);
    if (!result) {
      log.warn('Chaos result not found', { chaosId });
      return;
    }

    result.outcome = outcome;
    result.outcomeTimestamp = Date.now();

    if (outcome.survived) {
      this.stats.survived++;
    } else {
      this.stats.failed++;
      log.warn('Chaos event caused failure', { chaosId, error: outcome.error });
    }

    // Record in learning service if available
    if (this.learningService) {
      this.learningService.recordFeedback?.({
        source: 'chaos',
        chaosId,
        type: result.type,
        outcome,
      });
    }

    this.emit('chaos:result', { chaosId, outcome });
  }

  /**
   * Check if chaos can be injected (rate limiting)
   * @private
   */
  _canInjectChaos() {
    const now = Date.now();

    // Cooldown check
    if (now - this._lastEventTime < this.config.COOLDOWN_MS) {
      return false;
    }

    // Rate limit check
    const oneMinuteAgo = now - 60000;
    this._recentEvents = this._recentEvents.filter(t => t > oneMinuteAgo);

    if (this._recentEvents.length >= this.config.MAX_EVENTS_PER_MINUTE) {
      return false;
    }

    return true;
  }

  /**
   * Record a chaos event
   * @private
   */
  _recordChaosEvent(result) {
    const now = Date.now();
    this._lastEventTime = now;
    this._recentEvents.push(now);

    this.stats.injected++;
    this.stats.byType[result.type] = (this.stats.byType[result.type] || 0) + 1;

    this._results.push(result);
    if (this._results.length > this._maxResults) {
      this._results.shift();
    }
  }

  /**
   * Enable chaos
   */
  enable() {
    this.enabled = true;
    log.info('Chaos enabled');
  }

  /**
   * Disable chaos
   */
  disable() {
    this.enabled = false;
    log.info('Chaos disabled');
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this.stats,
      enabled: this.enabled,
      survivalRate: this.stats.survived + this.stats.failed > 0
        ? (this.stats.survived / (this.stats.survived + this.stats.failed) * 100).toFixed(1) + '%'
        : 'N/A',
    };
  }

  /**
   * Get recent results
   */
  getRecentResults(limit = 10) {
    return this._results.slice(-limit);
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      checks: 0,
      injected: 0,
      byType: {},
      survived: 0,
      failed: 0,
    };
    this._results = [];
  }
}

/**
 * Create a ChaosGenerator instance
 *
 * @param {Object} options
 * @returns {ChaosGenerator}
 */
export function createChaosGenerator(options = {}) {
  return new ChaosGenerator(options);
}

// Singleton (disabled by default for safety)
let _globalChaosGenerator = null;

/**
 * Get the global ChaosGenerator instance
 *
 * @param {Object} [options]
 * @returns {ChaosGenerator}
 */
export function getChaosGenerator(options) {
  if (!_globalChaosGenerator) {
    _globalChaosGenerator = new ChaosGenerator(options);
  }
  return _globalChaosGenerator;
}

/**
 * Reset global chaos generator (for testing)
 */
export function _resetChaosGeneratorForTesting() {
  _globalChaosGenerator = null;
}

export default ChaosGenerator;
