/**
 * CYNIC Decider - C6.3 (CYNIC × DECIDE)
 *
 * Self-governance: CYNIC decides how to respond to its own emergent patterns.
 * Sits between C6.7 (EMERGE) and C6.4 (ACT) — patterns become decisions become actions.
 *
 * "Le chien choisit sa route" - κυνικός
 *
 * Decides:
 * - Dog rotation when dominance/silence detected
 * - Pattern escalation when significance warrants it
 * - Memory compaction when pressure builds
 * - Learning pause when drift detected
 * - Threshold adjustment when consensus quality shifts
 *
 * @module @cynic/node/cynic/cynic-decider
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('CynicDecider');

/** Self-governance decision types */
export const CynicDecisionType = {
  ROTATE_DOGS: 'rotate_dogs',
  ESCALATE_PATTERN: 'escalate_pattern',
  COMPACT_MEMORY: 'compact_memory',
  PAUSE_LEARNING: 'pause_learning',
  ADJUST_THRESHOLDS: 'adjust_thresholds',
  ACKNOWLEDGE: 'acknowledge',
};

/** Decision urgency */
const Urgency = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};

const THRESHOLDS = {
  escalateSignificance: PHI_INV,     // 61.8% — escalate at high significance
  rotateThreshold: PHI_INV_2,        // 38.2% — rotate when dominance > this
  compactThreshold: PHI_INV,         // 61.8% — compact when memory load > this
  pauseThreshold: PHI_INV_3,         // 23.6% — pause when learning velocity < this
  cooldownMs: 5 * 60000,             // 5 min between same decision type
  maxHistory: 89,                    // Fib(11)
  minOutcomesForCalibration: 5,      // Need 5+ outcomes before adjusting
};

/**
 * CynicDecider - C6.3 self-governance
 */
export class CynicDecider extends EventEmitter {
  constructor(options = {}) {
    super();

    this._history = [];         // decision log
    this._outcomes = [];        // outcome tracking for calibration
    this._lastDecisionTime = new Map(); // type → timestamp

    // Goals (explicit objectives CYNIC tries to maintain)
    this._goals = {
      minConsensusAgreement: PHI_INV_2,  // 38.2% — minimum approval rate
      maxDogDominance: PHI_INV,          // 61.8% — no single dog > this
      maxMemoryLoad: PHI_INV,            // 61.8% — trigger compaction above
      minHealthScore: PHI_INV_3,         // 23.6% — alert below
    };

    this._stats = {
      decisionsTotal: 0,
      byType: {},
      outcomesRecorded: 0,
      goalViolations: 0,
      lastDecision: null,
    };

    for (const type of Object.values(CynicDecisionType)) {
      this._stats.byType[type] = 0;
    }
  }

  /**
   * Decide how to respond to an emergent pattern from C6.7.
   *
   * @param {Object} pattern - From CynicEmergence.pattern_detected
   * @param {string} pattern.type - CynicPatternType
   * @param {string} pattern.significance - LOW/MEDIUM/HIGH/CRITICAL
   * @param {Object} pattern.data - Pattern-specific payload
   * @param {number} pattern.confidence - Detection confidence (0-1)
   * @returns {Object|null} Decision object or null if no action needed
   */
  decideOnPattern(pattern) {
    const { type, significance, data, confidence } = pattern;

    // Route by pattern type
    switch (type) {
      case 'dog_dominance_shift':
        return this._decideDogDominance(data, significance, confidence);
      case 'dog_silence':
        return this._decideDogSilence(data, significance, confidence);
      case 'consensus_quality_change':
        return this._decideConsensusQuality(data, significance, confidence);
      case 'memory_pressure':
        return this._decideMemoryPressure(data, significance, confidence);
      case 'learning_velocity_change':
        return this._decideLearningVelocity(data, significance, confidence);
      case 'guardian_escalation':
        return this._decideGuardianEscalation(data, significance, confidence);
      case 'collective_health_trend':
        return this._decideHealthTrend(data, significance, confidence);
      case 'pattern_acceleration':
        return this._decidePatternAcceleration(data, significance, confidence);
      default:
        log.debug('Unknown pattern type', { type });
        return null;
    }
  }

  /**
   * Decide based on consciousness state change (from C6.1).
   * Intermediate layer between perception and action.
   *
   * @param {Object} state - { awarenessLevel, healthState, trend }
   * @returns {Object|null} Decision
   */
  decideOnConsciousness(state) {
    const { healthState, trend } = state;

    // If already critical, CynicActor handles emergency directly
    if (healthState === 'critical') return null;

    // Negative trend while still nominal — preemptive decision
    if (trend < -0.1 && healthState === 'nominal') {
      return this._makeDecision(CynicDecisionType.ADJUST_THRESHOLDS, {
        reason: 'negative_consciousness_trend',
        trend,
        recommendation: 'tighten routing thresholds',
      }, Urgency.MEDIUM, Math.min(PHI_INV, Math.abs(trend)));
    }

    return null;
  }

  /**
   * Record the outcome of a previous decision for calibration.
   *
   * @param {Object} outcome
   * @param {string} outcome.decisionId - ID of the decision
   * @param {string} outcome.decisionType - Type of decision
   * @param {string} outcome.result - 'success', 'partial', or 'failure'
   * @param {string} [outcome.reason] - Why it succeeded/failed
   */
  recordOutcome(outcome) {
    const entry = {
      decisionType: outcome.decisionType || 'unknown',
      result: outcome.result || 'unknown',
      reason: outcome.reason || null,
      ts: Date.now(),
    };

    this._outcomes.push(entry);
    while (this._outcomes.length > THRESHOLDS.maxHistory) this._outcomes.shift();
    this._stats.outcomesRecorded++;

    this.emit('outcome_recorded', entry);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Pattern-specific decision logic
  // ═══════════════════════════════════════════════════════════════════════════

  _decideDogDominance(data, significance, confidence) {
    const ratio = data?.ratio || 0;

    // Goal check: dog dominance > threshold?
    if (ratio > this._goals.maxDogDominance) {
      this._stats.goalViolations++;
      return this._makeDecision(CynicDecisionType.ROTATE_DOGS, {
        reason: 'dog_dominance_exceeds_goal',
        dominantDog: data?.dominantDog,
        ratio,
        goal: this._goals.maxDogDominance,
      }, significance === 'critical' ? Urgency.HIGH : Urgency.MEDIUM, confidence);
    }

    return this._makeDecision(CynicDecisionType.ACKNOWLEDGE, {
      reason: 'dominance_shift_within_bounds',
      dominantDog: data?.dominantDog,
      ratio,
    }, Urgency.LOW, confidence);
  }

  _decideDogSilence(data, significance, confidence) {
    return this._makeDecision(CynicDecisionType.ROTATE_DOGS, {
      reason: 'dog_silence_detected',
      silentDog: data?.silentDog,
      eventsSinceLastSeen: data?.eventsSinceLastSeen,
    }, significance === 'critical' ? Urgency.HIGH : Urgency.MEDIUM, confidence);
  }

  _decideConsensusQuality(data, significance, confidence) {
    const approvalRate = data?.approvalRate || data?.currentRate || 0;

    if (approvalRate < this._goals.minConsensusAgreement) {
      this._stats.goalViolations++;
      return this._makeDecision(CynicDecisionType.ADJUST_THRESHOLDS, {
        reason: 'consensus_below_goal',
        approvalRate,
        goal: this._goals.minConsensusAgreement,
        trend: data?.trend || 'declining',
      }, Urgency.HIGH, confidence);
    }

    return this._makeDecision(CynicDecisionType.ACKNOWLEDGE, {
      reason: 'consensus_quality_change_noted',
      approvalRate,
    }, Urgency.LOW, confidence);
  }

  _decideMemoryPressure(data, significance, confidence) {
    const load = data?.memoryLoad || data?.load || 0;

    if (load > this._goals.maxMemoryLoad) {
      this._stats.goalViolations++;
      return this._makeDecision(CynicDecisionType.COMPACT_MEMORY, {
        reason: 'memory_pressure_exceeds_goal',
        load,
        goal: this._goals.maxMemoryLoad,
      }, Urgency.HIGH, confidence);
    }

    return null; // Below threshold, no action needed
  }

  _decideLearningVelocity(data, significance, confidence) {
    const velocity = data?.velocity || data?.currentVelocity || 0;

    if (velocity < this._goals.minHealthScore) {
      return this._makeDecision(CynicDecisionType.PAUSE_LEARNING, {
        reason: 'learning_stagnation',
        velocity,
        recommendation: 'pause learning loops, let system stabilize',
      }, Urgency.MEDIUM, confidence);
    }

    return null;
  }

  _decideGuardianEscalation(data, significance, confidence) {
    return this._makeDecision(CynicDecisionType.ESCALATE_PATTERN, {
      reason: 'guardian_escalation_pattern',
      escalationCount: data?.count || data?.escalationCount || 0,
      recentThreats: data?.recentThreats || [],
    }, significance === 'critical' ? Urgency.CRITICAL : Urgency.HIGH, confidence);
  }

  _decideHealthTrend(data, significance, confidence) {
    const trend = data?.trend || data?.direction || 'stable';
    const avgHealth = data?.avgHealth || data?.currentHealth || 0.5;

    if (avgHealth < this._goals.minHealthScore) {
      this._stats.goalViolations++;
      return this._makeDecision(CynicDecisionType.ESCALATE_PATTERN, {
        reason: 'health_below_minimum',
        avgHealth,
        trend,
        goal: this._goals.minHealthScore,
      }, Urgency.HIGH, confidence);
    }

    if (trend === 'declining') {
      return this._makeDecision(CynicDecisionType.ADJUST_THRESHOLDS, {
        reason: 'declining_health_trend',
        avgHealth,
      }, Urgency.MEDIUM, confidence);
    }

    return null;
  }

  _decidePatternAcceleration(data, significance, confidence) {
    return this._makeDecision(CynicDecisionType.ESCALATE_PATTERN, {
      reason: 'pattern_acceleration',
      patternType: data?.acceleratingPattern,
      rate: data?.rate,
    }, significance === 'critical' ? Urgency.HIGH : Urgency.MEDIUM, confidence);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Decision construction
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Construct and emit a decision, respecting cooldowns.
   */
  _makeDecision(type, context, urgency, confidence) {
    // ACKNOWLEDGE always passes (no cooldown)
    if (type !== CynicDecisionType.ACKNOWLEDGE) {
      const now = Date.now();
      const lastTime = this._lastDecisionTime.get(type) || 0;

      // Critical urgency bypasses cooldown
      if (urgency !== Urgency.CRITICAL && now - lastTime < THRESHOLDS.cooldownMs) {
        return null;
      }
      this._lastDecisionTime.set(type, now);
    }

    // Apply calibration adjustment
    const calibratedConfidence = this._applyCalibration(type, confidence || 0);

    const decision = {
      cell: 'C6.3',
      dimension: 'CYNIC',
      analysis: 'DECIDE',
      type,
      urgency,
      confidence: Math.min(PHI_INV, calibratedConfidence),
      context,
      timestamp: Date.now(),
    };

    // Record
    this._history.push(decision);
    while (this._history.length > THRESHOLDS.maxHistory) this._history.shift();

    this._stats.decisionsTotal++;
    this._stats.byType[type] = (this._stats.byType[type] || 0) + 1;
    this._stats.lastDecision = decision;

    // Emit
    this.emit('decision', decision);
    globalEventBus.emit('cynic:decision', decision);

    log.info('Self-governance decision', {
      type,
      urgency,
      reason: context.reason,
      confidence: calibratedConfidence.toFixed(3),
    });

    return decision;
  }

  /**
   * Apply outcome-based calibration.
   * If past decisions of this type succeeded more than confidence predicted, boost.
   */
  _applyCalibration(type, baseConfidence) {
    const relevant = this._outcomes.filter(o => o.decisionType === type);
    if (relevant.length < THRESHOLDS.minOutcomesForCalibration) return baseConfidence;

    const successRate = relevant.filter(o => o.result === 'success').length / relevant.length;
    const gap = successRate - baseConfidence;

    // Clamp adjustment to [-0.15, +0.15], scale by φ⁻¹
    const adjustment = Math.max(-0.15, Math.min(0.15, gap * PHI_INV));
    return Math.max(0, Math.min(PHI_INV, baseConfidence + adjustment));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Public API
  // ═══════════════════════════════════════════════════════════════════════════

  getStats() {
    return {
      ...this._stats,
      historySize: this._history.length,
      outcomesSize: this._outcomes.length,
      goals: { ...this._goals },
      calibration: this._getCalibrationSummary(),
    };
  }

  getHealth() {
    const recentDecisions = this._history.filter(d => Date.now() - d.timestamp < 3600000);
    return {
      status: this._stats.goalViolations > 3 ? 'stressed' : 'healthy',
      decisionsLastHour: recentDecisions.length,
      goalViolations: this._stats.goalViolations,
      outcomesRecorded: this._stats.outcomesRecorded,
      score: Math.min(PHI_INV, 0.5 - this._stats.goalViolations * 0.02),
    };
  }

  getRecentDecisions(limit = 10) {
    return this._history.slice(-limit);
  }

  _getCalibrationSummary() {
    const summary = {};
    for (const type of Object.values(CynicDecisionType)) {
      const relevant = this._outcomes.filter(o => o.decisionType === type);
      if (relevant.length === 0) continue;
      summary[type] = {
        outcomes: relevant.length,
        successRate: relevant.filter(o => o.result === 'success').length / relevant.length,
      };
    }
    return summary;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Singleton
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getCynicDecider(options = {}) {
  if (!_instance) _instance = new CynicDecider(options);
  return _instance;
}

export function resetCynicDecider() {
  if (_instance) _instance.removeAllListeners();
  _instance = null;
}

export default { CynicDecider, getCynicDecider, resetCynicDecider, CynicDecisionType };
