/**
 * MetaCognitionController — Active Learning Control
 *
 * Extends MetaCognition with active control capabilities:
 * - Automatic learning rate adjustment
 * - Exploration/exploitation balance
 * - Intervention when stuck/thrashing
 * - Performance-based strategy switching
 *
 * Monitors → Analyzes → Controls → Learns
 *
 * φ-bounded interventions: max 61.8% parameter change per cycle
 *
 * "Think about thinking. Then act on it." — κυνικός
 *
 * @module @cynic/node/learning/meta-cognition-controller
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { getMetaCognition, CognitiveState, StrategyType } from './meta-cognition.js';

const log = createLogger('MetaCognitionController');

// ═══════════════════════════════════════════════════════════════════════════
// CONTROL PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Learning parameter ranges (φ-bounded)
 */
export const ParameterRanges = {
  LEARNING_RATE: { min: 0.01, max: PHI_INV, default: 0.1 },
  EXPLORATION_RATE: { min: PHI_INV_2, max: PHI_INV, default: 0.5 },
  TEMPERATURE: { min: 0.1, max: 2.0, default: 1.0 },
  DISCOUNT_FACTOR: { min: 0.5, max: 0.99, default: 0.9 },
};

/**
 * Intervention types
 */
export const InterventionType = {
  LEARNING_RATE_INCREASE: 'learning_rate_increase',
  LEARNING_RATE_DECREASE: 'learning_rate_decrease',
  EXPLORATION_INCREASE: 'exploration_increase',
  EXPLORATION_DECREASE: 'exploration_decrease',
  TEMPERATURE_INCREASE: 'temperature_increase',
  TEMPERATURE_DECREASE: 'temperature_decrease',
  STRATEGY_SWITCH: 'strategy_switch',
  RESET_PARAMETERS: 'reset_parameters',
};

/**
 * Control targets (what can be controlled)
 */
export const ControlTarget = {
  Q_LEARNING: 'q_learning',
  THOMPSON_SAMPLING: 'thompson_sampling',
  SONA: 'sona',
  BEHAVIOR_MODIFIER: 'behavior_modifier',
  LEARNING_PIPELINE: 'learning_pipeline',
};

// ═══════════════════════════════════════════════════════════════════════════
// INTERVENTION RECORD
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Intervention — a control action taken by the controller
 */
export class Intervention {
  constructor(data = {}) {
    this.id = data.id || `intervention-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.timestamp = data.timestamp || Date.now();
    this.type = data.type || InterventionType.LEARNING_RATE_INCREASE;
    this.target = data.target || ControlTarget.Q_LEARNING;
    this.reason = data.reason || '';
    this.before = data.before || {};
    this.after = data.after || {};
    this.effectiveAt = data.effectiveAt || null; // When did this take effect?
    this.wasEffective = data.wasEffective || null; // Did it help?
  }

  toJSON() {
    return {
      id: this.id,
      timestamp: this.timestamp,
      type: this.type,
      target: this.target,
      reason: this.reason,
      before: this.before,
      after: this.after,
      wasEffective: this.wasEffective,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// META-COGNITION CONTROLLER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * MetaCognitionController — active learning control
 *
 * Monitors MetaCognition state and actively adjusts learning parameters
 * when performance degrades or opportunities for improvement are detected.
 */
export class MetaCognitionController extends EventEmitter {
  constructor(options = {}) {
    super();

    // Dependencies
    this.metaCognition = options.metaCognition || getMetaCognition();
    this.qLearning = options.qLearning || null;
    this.modelIntelligence = options.modelIntelligence || null;
    this.learningPipeline = options.learningPipeline || null;

    // Configuration
    this.config = {
      enableAutoIntervention: options.enableAutoIntervention !== false,
      interventionCooldown: options.interventionCooldown || 30000, // 30s min between interventions
      minPerformanceForControl: options.minPerformanceForControl || 0.2, // 20% success rate minimum
      maxParameterChange: options.maxParameterChange || PHI_INV, // φ⁻¹ max change per intervention
      evaluationWindow: options.evaluationWindow || 10, // Actions to evaluate intervention effectiveness
    };

    // State
    this.interventions = [];
    this.lastInterventionTime = 0;
    this.activeInterventions = new Map(); // interventionId → evaluation state

    // Current parameters (tracked)
    this.currentParameters = {
      learningRate: ParameterRanges.LEARNING_RATE.default,
      explorationRate: ParameterRanges.EXPLORATION_RATE.default,
      temperature: ParameterRanges.TEMPERATURE.default,
      discountFactor: ParameterRanges.DISCOUNT_FACTOR.default,
    };

    // Stats
    this.stats = {
      totalInterventions: 0,
      effectiveInterventions: 0,
      ineffectiveInterventions: 0,
      byType: {},
      byTarget: {},
    };

    log.info('MetaCognitionController created', {
      autoIntervention: this.config.enableAutoIntervention,
      cooldown: this.config.interventionCooldown,
    });
  }

  // ───────────────────────────────────────────────────────────────────────────
  // LIFECYCLE
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Start the controller (begins monitoring)
   */
  start() {
    // Hook into MetaCognition state changes
    this.metaCognition.onStateChange = (state, reason) => {
      this._onStateChange(state, reason);
    };

    this.metaCognition.onStrategySwitch = (strategy, reason) => {
      this._onStrategySwitch(strategy, reason);
    };

    this.emit('started');
    log.info('MetaCognitionController started');
  }

  /**
   * Stop the controller
   */
  stop() {
    this.metaCognition.onStateChange = null;
    this.metaCognition.onStrategySwitch = null;
    this.emit('stopped');
    log.info('MetaCognitionController stopped');
  }

  // ───────────────────────────────────────────────────────────────────────────
  // MONITORING
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Handle MetaCognition state change
   */
  _onStateChange(state, reason) {
    log.debug('Cognitive state changed', { state, reason });

    if (!this.config.enableAutoIntervention) return;

    // Check if intervention is needed
    switch (state) {
      case CognitiveState.STUCK:
        this._considerIntervention({
          reason: 'Stuck state detected',
          suggestions: [
            InterventionType.EXPLORATION_INCREASE,
            InterventionType.LEARNING_RATE_INCREASE,
            InterventionType.STRATEGY_SWITCH,
          ],
        });
        break;

      case CognitiveState.THRASHING:
        this._considerIntervention({
          reason: 'Thrashing detected',
          suggestions: [
            InterventionType.LEARNING_RATE_DECREASE,
            InterventionType.TEMPERATURE_DECREASE,
            InterventionType.RESET_PARAMETERS,
          ],
        });
        break;

      case CognitiveState.FLOW:
        // Reinforce current parameters (no change, but record success)
        log.debug('Flow state — parameters optimal');
        break;

      case CognitiveState.RECOVERING:
        // Give recovery strategy time before intervening
        log.debug('Recovering — monitoring...');
        break;
    }

    this.emit('state:change', { state, reason });
  }

  /**
   * Handle MetaCognition strategy switch
   */
  _onStrategySwitch(strategy, reason) {
    log.debug('Strategy switched', { strategy, reason });
    this.emit('strategy:switch', { strategy, reason });
  }

  // ───────────────────────────────────────────────────────────────────────────
  // INTERVENTION DECISION
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Consider whether to intervene
   */
  _considerIntervention(context) {
    // Cooldown check
    const now = Date.now();
    if (now - this.lastInterventionTime < this.config.interventionCooldown) {
      log.debug('Intervention skipped (cooldown)', {
        remaining: this.config.interventionCooldown - (now - this.lastInterventionTime),
      });
      return;
    }

    // Performance check (don't intervene if too little data)
    const metrics = this.metaCognition.metrics;
    if (this.metaCognition.stats.totalActions < 10) {
      log.debug('Intervention skipped (insufficient data)');
      return;
    }

    // Select intervention based on suggestions
    const interventionType = this._selectIntervention(context.suggestions, metrics);
    if (!interventionType) {
      log.debug('No suitable intervention found');
      return;
    }

    // Execute intervention
    this._executeIntervention(interventionType, context.reason);
  }

  /**
   * Select best intervention from suggestions
   */
  _selectIntervention(suggestions, metrics) {
    // Simple heuristic: pick first suggestion that makes sense
    // TODO: More sophisticated selection based on past effectiveness

    for (const type of suggestions) {
      const param = this._getParameterForIntervention(type);
      if (!param) continue;

      const current = this.currentParameters[param.name];
      const range = ParameterRanges[param.name.toUpperCase()];

      // Check if we can actually change this parameter
      if (type.includes('INCREASE') && current >= range.max) continue;
      if (type.includes('DECREASE') && current <= range.min) continue;

      return type;
    }

    return null;
  }

  /**
   * Get parameter info for intervention type
   */
  _getParameterForIntervention(type) {
    if (type.includes('LEARNING_RATE')) {
      return { name: 'learningRate', direction: type.includes('INCREASE') ? 1 : -1 };
    }
    if (type.includes('EXPLORATION')) {
      return { name: 'explorationRate', direction: type.includes('INCREASE') ? 1 : -1 };
    }
    if (type.includes('TEMPERATURE')) {
      return { name: 'temperature', direction: type.includes('INCREASE') ? 1 : -1 };
    }
    return null;
  }

  // ───────────────────────────────────────────────────────────────────────────
  // INTERVENTION EXECUTION
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Execute an intervention
   */
  _executeIntervention(type, reason) {
    const param = this._getParameterForIntervention(type);
    if (!param) {
      log.warn('Cannot execute intervention', { type });
      return;
    }

    const current = this.currentParameters[param.name];
    const range = ParameterRanges[param.name.toUpperCase()];

    // Calculate new value (φ-bounded change)
    const maxChange = current * this.config.maxParameterChange;
    const delta = param.direction * maxChange;
    const newValue = Math.max(range.min, Math.min(range.max, current + delta));

    // Create intervention record
    const intervention = new Intervention({
      type,
      target: ControlTarget.Q_LEARNING, // TODO: Determine target dynamically
      reason,
      before: { [param.name]: current },
      after: { [param.name]: newValue },
    });

    // Apply intervention
    this.currentParameters[param.name] = newValue;
    this.lastInterventionTime = Date.now();
    this.interventions.push(intervention);
    this.stats.totalInterventions++;
    this.stats.byType[type] = (this.stats.byType[type] || 0) + 1;

    // Track for evaluation
    this.activeInterventions.set(intervention.id, {
      intervention,
      startActionCount: this.metaCognition.stats.totalActions,
      startSuccessRate: this.metaCognition.metrics.successRate,
    });

    // Emit event
    this.emit('intervention', intervention);

    log.info('Intervention executed', {
      type,
      parameter: param.name,
      before: current.toFixed(3),
      after: newValue.toFixed(3),
      reason,
    });

    // TODO: Actually apply to Q-Learning, ModelIntelligence, etc.
    // For now, just track the parameters
  }

  /**
   * Evaluate effectiveness of active interventions
   */
  _evaluateInterventions() {
    const currentActionCount = this.metaCognition.stats.totalActions;
    const currentSuccessRate = this.metaCognition.metrics.successRate;

    for (const [id, state] of this.activeInterventions.entries()) {
      const actionsSince = currentActionCount - state.startActionCount;

      if (actionsSince < this.config.evaluationWindow) {
        continue; // Not enough data yet
      }

      const { intervention } = state;
      const successRateChange = currentSuccessRate - state.startSuccessRate;

      // Simple effectiveness: did success rate improve?
      intervention.wasEffective = successRateChange > 0;
      intervention.effectiveAt = Date.now();

      if (intervention.wasEffective) {
        this.stats.effectiveInterventions++;
      } else {
        this.stats.ineffectiveInterventions++;
      }

      this.activeInterventions.delete(id);

      log.info('Intervention evaluated', {
        id: intervention.id,
        effective: intervention.wasEffective,
        successRateChange: successRateChange.toFixed(3),
      });

      this.emit('intervention:evaluated', intervention);
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // PUBLIC API
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Manually trigger an intervention
   *
   * @param {string} type - InterventionType
   * @param {string} reason - Why this intervention
   */
  intervene(type, reason = 'Manual intervention') {
    this._executeIntervention(type, reason);
  }

  /**
   * Get current learning parameters
   */
  getParameters() {
    return { ...this.currentParameters };
  }

  /**
   * Set a parameter manually
   *
   * @param {string} name - Parameter name
   * @param {number} value - New value
   */
  setParameter(name, value) {
    const rangeName = name.toUpperCase();
    const range = ParameterRanges[rangeName];

    if (!range) {
      throw new Error(`Unknown parameter: ${name}`);
    }

    const clamped = Math.max(range.min, Math.min(range.max, value));
    const old = this.currentParameters[name];

    this.currentParameters[name] = clamped;

    log.info('Parameter manually set', {
      parameter: name,
      before: old,
      after: clamped,
    });

    this.emit('parameter:set', { name, before: old, after: clamped });
  }

  /**
   * Reset all parameters to defaults
   */
  resetParameters() {
    for (const [key, range] of Object.entries(ParameterRanges)) {
      this.currentParameters[key.toLowerCase()] = range.default;
    }

    log.info('Parameters reset to defaults');
    this.emit('parameters:reset', this.currentParameters);
  }

  /**
   * Get controller statistics
   */
  getStats() {
    return {
      ...this.stats,
      currentParameters: this.currentParameters,
      activeInterventions: this.activeInterventions.size,
      recentInterventions: this.interventions.slice(-10),
    };
  }

  /**
   * Reflect on learning progress (called by LearningPipeline)
   *
   * @param {Object} context - Reflection context
   * @returns {Promise<Object>} Reflection insights
   */
  async reflect(context) {
    // Evaluate active interventions
    this._evaluateInterventions();

    // Get MetaCognition state
    const state = this.metaCognition.state;
    const metrics = this.metaCognition.metrics;

    // Suggest learning rate based on performance
    let suggestedLearningRate = this.currentParameters.learningRate;

    if (metrics.successRate > PHI_INV) {
      // High success rate → can increase learning rate
      suggestedLearningRate = Math.min(
        ParameterRanges.LEARNING_RATE.max,
        this.currentParameters.learningRate * (1 + PHI_INV_2)
      );
    } else if (metrics.successRate < 0.3) {
      // Low success rate → decrease learning rate
      suggestedLearningRate = Math.max(
        ParameterRanges.LEARNING_RATE.min,
        this.currentParameters.learningRate * (1 - PHI_INV_2)
      );
    }

    const insights = [
      `State: ${state}`,
      `Success rate: ${(metrics.successRate * 100).toFixed(1)}%`,
      `Interventions: ${this.stats.totalInterventions} total, ${this.stats.effectiveInterventions} effective`,
    ];

    return {
      insights,
      suggestedLearningRate,
      currentState: state,
      metrics,
    };
  }

  /**
   * Health check
   */
  async health() {
    return {
      state: this.metaCognition.state,
      parameters: this.currentParameters,
      interventions: {
        total: this.stats.totalInterventions,
        effective: this.stats.effectiveInterventions,
        effectiveness: this.stats.totalInterventions > 0
          ? this.stats.effectiveInterventions / this.stats.totalInterventions
          : 0,
      },
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _singleton = null;

export function getMetaCognitionController(options) {
  if (!_singleton) {
    _singleton = new MetaCognitionController(options);
  }
  return _singleton;
}

export function _resetForTesting() {
  if (_singleton) {
    _singleton.stop();
  }
  _singleton = null;
}
