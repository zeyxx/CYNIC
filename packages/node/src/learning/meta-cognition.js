/**
 * MetaCognition - Self-monitoring and strategy switching
 *
 * Implements meta-cognitive layer for CYNIC:
 * - Self-monitoring of performance
 * - Stuck state detection
 * - Automatic strategy switching
 * - Performance trend analysis
 *
 * P3.4: "Thinking about thinking" - κυνικός
 *
 * @module @cynic/node/learning/meta-cognition
 */

'use strict';

import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';

const log = createLogger('MetaCognition');

/**
 * Cognitive states
 */
export const CognitiveState = {
  EXPLORING: 'exploring',       // Gathering information
  EXECUTING: 'executing',       // Making progress
  STUCK: 'stuck',               // No progress detected
  THRASHING: 'thrashing',       // Repeating same actions
  RECOVERING: 'recovering',     // Attempting recovery
  FLOW: 'flow',                 // Optimal performance
};

/**
 * Strategy types
 */
export const StrategyType = {
  DEPTH_FIRST: 'depth_first',   // Go deep on current approach
  BREADTH_FIRST: 'breadth_first', // Explore alternatives
  BACKTRACK: 'backtrack',       // Return to previous state
  SIMPLIFY: 'simplify',         // Reduce complexity
  ESCALATE: 'escalate',         // Request help/clarification
  RESET: 'reset',               // Start fresh
};

/**
 * Default configuration
 */
export const META_CONFIG = {
  // Stuck detection
  stuckThreshold: 5,            // Same action count before stuck
  progressWindow: 10,           // Actions to track for progress
  minProgressRate: PHI_INV_2,   // 38.2% progress rate minimum

  // Thrashing detection
  thrashingWindow: 8,           // Actions to check for repetition
  thrashingThreshold: 0.5,      // 50% repetition = thrashing

  // Strategy switching
  strategyTimeout: 60000,       // Max time on one strategy (1 min)
  maxStrategySwitches: 5,       // Before escalation

  // Performance tracking
  performanceWindow: 20,        // Actions for performance calculation
  flowThreshold: PHI_INV,       // 61.8% success rate for flow state
};

/**
 * Action record for tracking
 */
export class ActionRecord {
  constructor(options = {}) {
    this.id = options.id || `action_${Date.now()}`;
    this.type = options.type || 'unknown';
    this.tool = options.tool || null;
    this.success = options.success;
    this.timestamp = options.timestamp || Date.now();
    this.duration = options.duration || 0;
    this.signature = options.signature || this._computeSignature();
  }

  _computeSignature() {
    return `${this.type}:${this.tool || 'none'}`;
  }
}

/**
 * Strategy execution record
 */
export class StrategyRecord {
  constructor(strategy, reason) {
    this.strategy = strategy;
    this.reason = reason;
    this.startTime = Date.now();
    this.endTime = null;
    this.actionsCount = 0;
    this.successCount = 0;
    this.wasEffective = null;
  }

  end(effective) {
    this.endTime = Date.now();
    this.wasEffective = effective;
  }

  get duration() {
    return (this.endTime || Date.now()) - this.startTime;
  }

  get successRate() {
    return this.actionsCount > 0 ? this.successCount / this.actionsCount : 0;
  }
}

/**
 * MetaCognition - Self-monitoring layer
 */
export class MetaCognition {
  /**
   * @param {Object} options
   * @param {Object} [options.config] - Override config
   * @param {Function} [options.onStateChange] - State change callback
   * @param {Function} [options.onStrategySwitch] - Strategy switch callback
   */
  constructor(options = {}) {
    this.config = { ...META_CONFIG, ...options.config };
    this.onStateChange = options.onStateChange || null;
    this.onStrategySwitch = options.onStrategySwitch || null;

    // State tracking
    this.state = CognitiveState.EXPLORING;
    this.previousState = null;
    this.stateHistory = [];

    // Action tracking
    this.actions = [];
    this.lastProgressTime = Date.now();

    // Strategy tracking
    this.currentStrategy = StrategyType.DEPTH_FIRST;
    this.strategyRecord = new StrategyRecord(this.currentStrategy, 'initial');
    this.strategyHistory = [];
    this.strategySwitches = 0;

    // Performance metrics
    this.metrics = {
      successRate: 0,
      progressRate: 0,
      avgActionDuration: 0,
      stuckCount: 0,
      thrashingCount: 0,
      recoveriesSuccess: 0,
      recoveriesTotal: 0,
    };

    // Statistics
    this.stats = {
      totalActions: 0,
      successfulActions: 0,
      stateChanges: 0,
      strategySwitches: 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Action Recording
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record an action
   *
   * @param {Object} data - Action data
   * @returns {Object} Analysis result
   */
  recordAction(data) {
    const action = new ActionRecord(data);
    this.actions.push(action);
    this.stats.totalActions++;

    if (action.success) {
      this.stats.successfulActions++;
      this.strategyRecord.successCount++;
    }
    this.strategyRecord.actionsCount++;

    // Trim action history
    if (this.actions.length > this.config.performanceWindow * 2) {
      this.actions = this.actions.slice(-this.config.performanceWindow);
    }

    // Analyze state
    const analysis = this._analyzeState();

    // Update metrics
    this._updateMetrics();

    return analysis;
  }

  /**
   * Record progress (explicit progress signal)
   *
   * @param {string} [description] - What progress was made
   */
  recordProgress(description = 'Progress made') {
    this.lastProgressTime = Date.now();

    if (this.state === CognitiveState.STUCK || this.state === CognitiveState.RECOVERING) {
      this._transitionState(CognitiveState.EXECUTING, description);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State Analysis
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Analyze current state
   * @private
   */
  _analyzeState() {
    const result = {
      state: this.state,
      stuck: false,
      thrashing: false,
      recommendation: null,
    };

    // Check for stuck state
    if (this._isStuck()) {
      result.stuck = true;
      this.metrics.stuckCount++;

      if (this.state !== CognitiveState.STUCK) {
        this._transitionState(CognitiveState.STUCK, 'No progress detected');
        result.recommendation = this._recommendRecovery('stuck');
      }
    }

    // Check for thrashing
    if (this._isThrashing()) {
      result.thrashing = true;
      this.metrics.thrashingCount++;

      if (this.state !== CognitiveState.THRASHING) {
        this._transitionState(CognitiveState.THRASHING, 'Repetitive actions detected');
        result.recommendation = this._recommendRecovery('thrashing');
      }
    }

    // Check for flow state
    if (this._isInFlow() && this.state !== CognitiveState.FLOW) {
      this._transitionState(CognitiveState.FLOW, 'High success rate');
    }

    // Check strategy timeout
    if (this._shouldSwitchStrategy()) {
      result.recommendation = this._switchStrategy('timeout');
    }

    return result;
  }

  /**
   * Check if stuck
   * @private
   */
  _isStuck() {
    // Time since last progress
    const timeSinceProgress = Date.now() - this.lastProgressTime;
    if (timeSinceProgress > this.config.strategyTimeout) {
      return true;
    }

    // Check for repeated failures
    const recentActions = this.actions.slice(-this.config.stuckThreshold);
    if (recentActions.length >= this.config.stuckThreshold) {
      const failureRate = recentActions.filter(a => !a.success).length / recentActions.length;
      if (failureRate >= (1 - this.config.minProgressRate)) {
        return true;
      }
    }

    return false;
  }

  /**
   * Check if thrashing (repeating same actions)
   * @private
   */
  _isThrashing() {
    const recentActions = this.actions.slice(-this.config.thrashingWindow);
    if (recentActions.length < this.config.thrashingWindow) {
      return false;
    }

    // Count unique action signatures
    const signatures = recentActions.map(a => a.signature);
    const uniqueSignatures = new Set(signatures);

    // If less than 50% unique, we're thrashing
    const uniqueRatio = uniqueSignatures.size / signatures.length;
    return uniqueRatio < (1 - this.config.thrashingThreshold);
  }

  /**
   * Check if in flow state
   * @private
   */
  _isInFlow() {
    const recentActions = this.actions.slice(-this.config.performanceWindow);
    if (recentActions.length < 5) return false;

    const successRate = recentActions.filter(a => a.success).length / recentActions.length;
    return successRate >= this.config.flowThreshold;
  }

  /**
   * Check if strategy should switch
   * @private
   */
  _shouldSwitchStrategy() {
    // Check timeout
    if (this.strategyRecord.duration > this.config.strategyTimeout) {
      return true;
    }

    // Check if not effective
    if (this.strategyRecord.actionsCount >= 10 && this.strategyRecord.successRate < PHI_INV_2) {
      return true;
    }

    return false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State Transitions
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Transition to new state
   * @private
   */
  _transitionState(newState, reason) {
    this.previousState = this.state;
    this.state = newState;
    this.stats.stateChanges++;

    this.stateHistory.push({
      from: this.previousState,
      to: newState,
      reason,
      timestamp: Date.now(),
    });

    // Trim history
    if (this.stateHistory.length > 50) {
      this.stateHistory = this.stateHistory.slice(-25);
    }

    log.info('State transition', { from: this.previousState, to: newState, reason });

    if (this.onStateChange) {
      this.onStateChange(newState, this.previousState, reason);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Strategy Switching
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Recommend recovery strategy
   * @private
   */
  _recommendRecovery(issue) {
    let strategy;
    let reason;

    switch (issue) {
      case 'stuck':
        // If we've been using depth-first, try breadth-first
        if (this.currentStrategy === StrategyType.DEPTH_FIRST) {
          strategy = StrategyType.BREADTH_FIRST;
          reason = 'Stuck on current approach, explore alternatives';
        } else if (this.currentStrategy === StrategyType.BREADTH_FIRST) {
          strategy = StrategyType.BACKTRACK;
          reason = 'No alternatives found, backtrack to previous state';
        } else {
          strategy = StrategyType.SIMPLIFY;
          reason = 'Complex situation, simplify the problem';
        }
        break;

      case 'thrashing':
        // Thrashing usually means we need to step back
        strategy = StrategyType.BACKTRACK;
        reason = 'Repeating same actions, need to backtrack';
        break;

      case 'timeout':
        // Too long on one strategy
        if (this.strategySwitches >= this.config.maxStrategySwitches) {
          strategy = StrategyType.ESCALATE;
          reason = 'Multiple strategies failed, escalate for help';
        } else {
          strategy = StrategyType.SIMPLIFY;
          reason = 'Strategy timeout, try simpler approach';
        }
        break;

      default:
        strategy = StrategyType.BREADTH_FIRST;
        reason = 'Default recovery: explore alternatives';
    }

    return { strategy, reason };
  }

  /**
   * Switch to new strategy
   * @private
   */
  _switchStrategy(trigger) {
    const recommendation = this._recommendRecovery(trigger);

    // End current strategy record
    this.strategyRecord.end(this.strategyRecord.successRate >= PHI_INV_2);
    this.strategyHistory.push(this.strategyRecord);

    // Start new strategy
    this.currentStrategy = recommendation.strategy;
    this.strategyRecord = new StrategyRecord(recommendation.strategy, recommendation.reason);
    this.strategySwitches++;
    this.stats.strategySwitches++;

    // Transition to recovering state
    if (this.state === CognitiveState.STUCK || this.state === CognitiveState.THRASHING) {
      this._transitionState(CognitiveState.RECOVERING, `Switching to ${recommendation.strategy}`);
    }

    log.info('Strategy switch', {
      from: this.strategyHistory[this.strategyHistory.length - 1]?.strategy,
      to: recommendation.strategy,
      reason: recommendation.reason,
    });

    if (this.onStrategySwitch) {
      this.onStrategySwitch(recommendation.strategy, recommendation.reason);
    }

    return recommendation;
  }

  /**
   * Manually switch strategy
   *
   * @param {string} strategy - Strategy to switch to
   * @param {string} [reason] - Reason for switch
   */
  switchStrategy(strategy, reason = 'Manual switch') {
    // Validate strategy
    if (!Object.values(StrategyType).includes(strategy)) {
      throw new Error(`Invalid strategy: ${strategy}`);
    }

    // End current
    this.strategyRecord.end(this.strategyRecord.successRate >= PHI_INV_2);
    this.strategyHistory.push(this.strategyRecord);

    // Start new
    this.currentStrategy = strategy;
    this.strategyRecord = new StrategyRecord(strategy, reason);
    this.strategySwitches++;
    this.stats.strategySwitches++;

    if (this.onStrategySwitch) {
      this.onStrategySwitch(strategy, reason);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Metrics
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Update performance metrics
   * @private
   */
  _updateMetrics() {
    const recent = this.actions.slice(-this.config.performanceWindow);
    if (recent.length === 0) return;

    // Success rate
    this.metrics.successRate = recent.filter(a => a.success).length / recent.length;

    // Progress rate (unique actions / total actions)
    const signatures = new Set(recent.map(a => a.signature));
    this.metrics.progressRate = signatures.size / recent.length;

    // Average duration
    const durations = recent.filter(a => a.duration > 0).map(a => a.duration);
    this.metrics.avgActionDuration = durations.length > 0
      ? durations.reduce((a, b) => a + b, 0) / durations.length
      : 0;
  }

  /**
   * Get current state summary
   *
   * @returns {Object}
   */
  getState() {
    return {
      state: this.state,
      strategy: this.currentStrategy,
      metrics: { ...this.metrics },
      isStuck: this.state === CognitiveState.STUCK,
      isThrashing: this.state === CognitiveState.THRASHING,
      isInFlow: this.state === CognitiveState.FLOW,
      strategySwitches: this.strategySwitches,
      timeSinceProgress: Date.now() - this.lastProgressTime,
    };
  }

  /**
   * Get statistics
   *
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      currentState: this.state,
      currentStrategy: this.currentStrategy,
      metrics: this.metrics,
      recentStateHistory: this.stateHistory.slice(-10),
      strategyEffectiveness: this._calculateStrategyEffectiveness(),
    };
  }

  /**
   * Calculate strategy effectiveness
   * @private
   */
  _calculateStrategyEffectiveness() {
    const effectiveness = {};

    for (const record of this.strategyHistory) {
      if (!effectiveness[record.strategy]) {
        effectiveness[record.strategy] = { uses: 0, effective: 0 };
      }
      effectiveness[record.strategy].uses++;
      if (record.wasEffective) {
        effectiveness[record.strategy].effective++;
      }
    }

    // Calculate rates
    for (const strategy of Object.keys(effectiveness)) {
      const data = effectiveness[strategy];
      data.effectivenessRate = data.uses > 0 ? data.effective / data.uses : 0;
    }

    return effectiveness;
  }

  /**
   * Reset state (for new session)
   */
  reset() {
    this.state = CognitiveState.EXPLORING;
    this.previousState = null;
    this.actions = [];
    this.lastProgressTime = Date.now();
    this.currentStrategy = StrategyType.DEPTH_FIRST;
    this.strategyRecord = new StrategyRecord(this.currentStrategy, 'reset');
    this.strategySwitches = 0;

    // Keep strategy history for learning
    // Keep stats for analysis
  }

  /**
   * Export state for persistence
   *
   * @returns {Object}
   */
  exportState() {
    return {
      state: this.state,
      currentStrategy: this.currentStrategy,
      metrics: this.metrics,
      stats: this.stats,
      strategyHistory: this.strategyHistory.slice(-20).map(r => ({
        strategy: r.strategy,
        reason: r.reason,
        duration: r.duration,
        successRate: r.successRate,
        wasEffective: r.wasEffective,
      })),
    };
  }

  /**
   * Import state from persistence
   *
   * @param {Object} state
   */
  importState(state) {
    if (state.metrics) {
      this.metrics = { ...this.metrics, ...state.metrics };
    }
    if (state.stats) {
      this.stats = { ...this.stats, ...state.stats };
    }
    // Don't import current state - start fresh
  }
}

/**
 * Create MetaCognition instance
 *
 * @param {Object} options
 * @returns {MetaCognition}
 */
export function createMetaCognition(options = {}) {
  return new MetaCognition(options);
}

// Singleton
let _instance = null;

/**
 * Get or create global MetaCognition
 *
 * @param {Object} [options]
 * @returns {MetaCognition}
 */
export function getMetaCognition(options = {}) {
  if (!_instance) {
    _instance = createMetaCognition(options);
  }
  return _instance;
}

export default MetaCognition;
