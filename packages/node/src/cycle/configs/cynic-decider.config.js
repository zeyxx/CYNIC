/**
 * CynicDecider Config — C6.3 (CYNIC x DECIDE)
 *
 * Domain-specific configuration for the CYNIC Decider.
 * Template logic lives in create-decider.js.
 *
 * Self-governance: CYNIC decides how to respond to its own emergent patterns.
 * Bridges C6.7 (EMERGE) → C6.3 (DECIDE) → C6.4 (ACT)
 *
 * Decides:
 * - Budget governance (shift to Ollama when > φ⁻¹)
 * - Memory governance (compress/GC when pressure builds)
 * - Context governance (semantic compression when large)
 * - Learning governance (prioritize SONA when maturity low)
 * - Event governance (investigate wiring when orphans detected)
 *
 * @module @cynic/node/cycle/configs/cynic-decider.config
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

export const CynicDecisionType = {
  SHIFT_TO_OLLAMA: 'shift_to_ollama',
  COMPRESS_CONTEXT: 'compress_context',
  TRIGGER_GC: 'trigger_gc',
  SEMANTIC_COMPRESS: 'semantic_compress',
  PRIORITIZE_SONA: 'prioritize_sona',
  INVESTIGATE_WIRING: 'investigate_wiring',
  ADJUST_THRESHOLDS: 'adjust_thresholds',
  ACKNOWLEDGE: 'acknowledge',
};

/** Decision urgency levels */
const Urgency = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};

const THRESHOLDS = {
  budgetCritical: PHI_INV,        // 61.8% — shift to Ollama
  budgetWarning: PHI_INV_2,       // 38.2% — start monitoring
  memoryHeapCritical: 0.8,        // 80% heap usage
  memoryHeapWarning: PHI_INV,     // 61.8% heap usage
  contextSizeWarning: 10,         // 10 MB
  learningMaturityLow: PHI_INV_2 * 100, // 38.2% maturity
  orphanThreshold: 3,             // 3+ orphan events
  cooldownMs: 5 * 60000,          // 5 min between same decision
};

const MIN_JUDGMENTS = 3; // Wait for enough data before deciding

export const cynicDeciderConfig = {
  name: 'CynicDecider',
  cell: 'C6.3',
  dimension: 'CYNIC',
  eventPrefix: 'cynic',
  decisionTypes: CynicDecisionType,
  maxHistory: 89, // Fib(11)
  extraStatFields: ['goalViolations', 'budgetShifts', 'memoryCompactions'],
  calibrationGroupBy: 'decisionType',
  calibrationClamp: 0.1,

  init(decider) {
    decider._judgmentCount = 0;
    decider._lastBudgetShift = 0;
    decider._goals = {
      maxBudgetSpent: PHI_INV,         // 61.8% of budget
      maxMemoryLoad: PHI_INV,          // 61.8% heap usage
      minLearningMaturity: PHI_INV_2 * 100, // 38.2% maturity
      maxOrphanEvents: 3,              // 3 orphan events
    };
  },

  /**
   * Decide based on CYNIC self-state judgment.
   *
   * @param {Object} judgment - From cynic:judgment event
   * @param {Object} [context] - Additional context with state snapshot
   * @param {Object} decider - The decider instance
   * @returns {Object} Decision result
   */
  decide(judgment, context, decider) {
    const score = judgment.score || 0;
    const verdict = judgment.verdict || 'BARK';
    const state = context.state || {};

    decider._judgmentCount++;

    // Wait for sufficient data
    if (decider._judgmentCount < MIN_JUDGMENTS) {
      return decider.recordDecision({
        decision: CynicDecisionType.ACKNOWLEDGE,
        reason: `Collecting CYNIC state data — ${decider._judgmentCount}/${MIN_JUDGMENTS} judgments.`,
        severity: 'low',
        confidence: PHI_INV_3,
        score,
        verdict,
      });
    }

    const actions = [];
    const confidence = calculateConfidence(judgment, state, decider);

    // ========================================================================
    // Budget Governance (φ-bounded)
    // ========================================================================
    const budgetSpent = state.budget?.spent || 0;
    const budgetLimit = state.budget?.limit || 10;
    const budgetRatio = budgetSpent / budgetLimit;

    if (budgetRatio > THRESHOLDS.budgetCritical) {
      decider._stats.goalViolations++;
      decider._stats.budgetShifts++;
      decider._lastBudgetShift = Date.now();

      return decider.recordDecision({
        decision: CynicDecisionType.SHIFT_TO_OLLAMA,
        reason: `Budget at ${(budgetRatio * 100).toFixed(1)}% (φ⁻¹ threshold exceeded)`,
        severity: 'high',
        confidence,
        score,
        verdict,
        budgetRatio,
        goal: decider._goals.maxBudgetSpent,
      });
    }

    // ========================================================================
    // Memory Governance
    // ========================================================================
    const heapUsed = state.memory?.heapUsed || 0;
    const heapTotal = state.memory?.heapTotal || 1;
    const heapRatio = heapUsed / heapTotal;

    if (heapRatio > THRESHOLDS.memoryHeapCritical) {
      decider._stats.goalViolations++;
      decider._stats.memoryCompactions++;

      // Critical: both compress + GC
      return decider.recordDecision({
        decision: CynicDecisionType.COMPRESS_CONTEXT,
        reason: `Heap at ${(heapRatio * 100).toFixed(1)}% — emergency compaction needed`,
        severity: 'critical',
        confidence,
        score,
        verdict,
        heapRatio,
        actions: [CynicDecisionType.COMPRESS_CONTEXT, CynicDecisionType.TRIGGER_GC],
      });
    }

    if (heapRatio > THRESHOLDS.memoryHeapWarning) {
      decider._stats.memoryCompactions++;

      return decider.recordDecision({
        decision: CynicDecisionType.TRIGGER_GC,
        reason: `Heap at ${(heapRatio * 100).toFixed(1)}% (φ⁻¹ threshold) — trigger GC`,
        severity: 'medium',
        confidence,
        score,
        verdict,
        heapRatio,
      });
    }

    // ========================================================================
    // Context Governance
    // ========================================================================
    const contextSizeMB = state.context?.sizeMB || 0;

    if (contextSizeMB > THRESHOLDS.contextSizeWarning) {
      return decider.recordDecision({
        decision: CynicDecisionType.SEMANTIC_COMPRESS,
        reason: `Context size ${contextSizeMB.toFixed(1)}MB exceeds ${THRESHOLDS.contextSizeWarning}MB threshold`,
        severity: 'medium',
        confidence,
        score,
        verdict,
        contextSizeMB,
      });
    }

    // ========================================================================
    // Learning Governance
    // ========================================================================
    const learningMaturity = state.learning?.maturityPercent || 0;

    if (learningMaturity < THRESHOLDS.learningMaturityLow) {
      return decider.recordDecision({
        decision: CynicDecisionType.PRIORITIZE_SONA,
        reason: `Learning maturity ${learningMaturity.toFixed(1)}% below φ⁻² threshold (${THRESHOLDS.learningMaturityLow}%)`,
        severity: 'medium',
        confidence,
        score,
        verdict,
        learningMaturity,
        goal: decider._goals.minLearningMaturity,
      });
    }

    // ========================================================================
    // Event Governance
    // ========================================================================
    const orphanCount = state.events?.orphanCount || 0;

    if (orphanCount > THRESHOLDS.orphanThreshold) {
      decider._stats.goalViolations++;

      return decider.recordDecision({
        decision: CynicDecisionType.INVESTIGATE_WIRING,
        reason: `${orphanCount} orphan events detected (threshold: ${THRESHOLDS.orphanThreshold})`,
        severity: 'high',
        confidence,
        score,
        verdict,
        orphanCount,
        goal: decider._goals.maxOrphanEvents,
      });
    }

    // ========================================================================
    // Pattern-based Decisions (from old CynicDecider)
    // ========================================================================
    const patternType = judgment.patternType || context.patternType;

    if (patternType) {
      return handlePatternDecision(patternType, judgment, context, decider, confidence);
    }

    // ========================================================================
    // Default: Acknowledge healthy state
    // ========================================================================
    return decider.recordDecision({
      decision: CynicDecisionType.ACKNOWLEDGE,
      reason: `CYNIC state healthy (score: ${score.toFixed(1)}, budget: ${(budgetRatio * 100).toFixed(1)}%, heap: ${(heapRatio * 100).toFixed(1)}%)`,
      severity: 'low',
      confidence,
      score,
      verdict,
    });
  },

  updateExtraStats(stats, result) {
    // budgetShifts and memoryCompactions tracked in decide()
  },

  getHealth(decider) {
    const total = decider._stats.decisionsTotal;
    const investigateRate = total > 0
      ? (decider._stats.byType[CynicDecisionType.INVESTIGATE_WIRING] || 0) / total
      : 0;

    return {
      status: decider._stats.goalViolations > 3 ? 'stressed'
        : investigateRate > PHI_INV_3 ? 'high_investigation'
          : 'healthy',
      score: Math.min(PHI_INV, 0.5 - (decider._stats.goalViolations || 0) * 0.02),
      totalDecisions: total,
      investigateRate,
      budgetShifts: decider._stats.budgetShifts || 0,
      memoryCompactions: decider._stats.memoryCompactions || 0,
      goalViolations: decider._stats.goalViolations || 0,
    };
  },
};

// =============================================================================
// Helpers
// =============================================================================

function calculateConfidence(judgment, state, decider) {
  let confidence = PHI_INV;

  // Scale with judgment quality
  const score = judgment.score || 0;
  if (score < 30) confidence *= 0.5;
  else if (score < 50) confidence *= 0.7;
  else if (score < 70) confidence *= 0.85;

  // Scale with data freshness
  const dataAge = Date.now() - (state.timestamp || Date.now());
  if (dataAge > 60000) confidence *= 0.8; // >1min old

  // Apply calibration
  confidence = decider.applyCalibration('default', confidence);

  return Math.min(PHI_INV, Math.round(confidence * 1000) / 1000);
}

/**
 * Handle pattern-based decisions (compatibility with old CynicDecider).
 * Routes emergent patterns from C6.7 to decisions.
 */
function handlePatternDecision(patternType, judgment, context, decider, confidence) {
  const pattern = context.pattern || {};
  const significance = pattern.significance || 'medium';
  const data = pattern.data || {};

  switch (patternType) {
    case 'dog_dominance_shift':
      return handleDogDominance(data, significance, confidence, decider, judgment);

    case 'dog_silence':
      return handleDogSilence(data, significance, confidence, decider, judgment);

    case 'consensus_quality_change':
      return handleConsensusQuality(data, significance, confidence, decider, judgment);

    case 'memory_pressure':
      return handleMemoryPressure(data, significance, confidence, decider, judgment);

    case 'learning_velocity_change':
      return handleLearningVelocity(data, significance, confidence, decider, judgment);

    default:
      return decider.recordDecision({
        decision: CynicDecisionType.ACKNOWLEDGE,
        reason: `Pattern ${patternType} noted`,
        severity: 'low',
        confidence,
        score: judgment.score || 0,
        verdict: judgment.verdict || 'BARK',
      });
  }
}

function handleDogDominance(data, significance, confidence, decider, judgment) {
  const ratio = data?.ratio || 0;

  if (ratio > PHI_INV) {
    decider._stats.goalViolations++;
    return decider.recordDecision({
      decision: CynicDecisionType.ADJUST_THRESHOLDS,
      reason: `Dog dominance ${(ratio * 100).toFixed(1)}% exceeds φ⁻¹ — rotate dogs`,
      severity: significance === 'critical' ? 'high' : 'medium',
      confidence,
      score: judgment.score || 0,
      verdict: judgment.verdict || 'BARK',
      dominantDog: data?.dominantDog,
      ratio,
    });
  }

  return decider.recordDecision({
    decision: CynicDecisionType.ACKNOWLEDGE,
    reason: `Dog dominance ${(ratio * 100).toFixed(1)}% within bounds`,
    severity: 'low',
    confidence,
    score: judgment.score || 0,
    verdict: judgment.verdict || 'BARK',
  });
}

function handleDogSilence(data, significance, confidence, decider, judgment) {
  return decider.recordDecision({
    decision: CynicDecisionType.ADJUST_THRESHOLDS,
    reason: `Dog silence detected (${data?.silentDog}) — rotate dogs`,
    severity: significance === 'critical' ? 'high' : 'medium',
    confidence,
    score: judgment.score || 0,
    verdict: judgment.verdict || 'BARK',
    silentDog: data?.silentDog,
  });
}

function handleConsensusQuality(data, significance, confidence, decider, judgment) {
  const approvalRate = data?.approvalRate || data?.currentRate || 0;

  if (approvalRate < PHI_INV_2) {
    decider._stats.goalViolations++;
    return decider.recordDecision({
      decision: CynicDecisionType.ADJUST_THRESHOLDS,
      reason: `Consensus approval ${(approvalRate * 100).toFixed(1)}% below φ⁻² threshold`,
      severity: 'high',
      confidence,
      score: judgment.score || 0,
      verdict: judgment.verdict || 'BARK',
      approvalRate,
    });
  }

  return decider.recordDecision({
    decision: CynicDecisionType.ACKNOWLEDGE,
    reason: `Consensus quality noted (${(approvalRate * 100).toFixed(1)}%)`,
    severity: 'low',
    confidence,
    score: judgment.score || 0,
    verdict: judgment.verdict || 'BARK',
  });
}

function handleMemoryPressure(data, significance, confidence, decider, judgment) {
  const load = data?.memoryLoad || data?.load || 0;

  if (load > PHI_INV) {
    decider._stats.goalViolations++;
    decider._stats.memoryCompactions++;
    return decider.recordDecision({
      decision: CynicDecisionType.COMPRESS_CONTEXT,
      reason: `Memory pressure ${(load * 100).toFixed(1)}% exceeds φ⁻¹`,
      severity: 'high',
      confidence,
      score: judgment.score || 0,
      verdict: judgment.verdict || 'BARK',
      load,
    });
  }

  return null; // Below threshold
}

function handleLearningVelocity(data, significance, confidence, decider, judgment) {
  const velocity = data?.velocity || data?.currentVelocity || 0;

  if (velocity < PHI_INV_3) {
    return decider.recordDecision({
      decision: CynicDecisionType.PRIORITIZE_SONA,
      reason: `Learning velocity ${velocity.toFixed(3)} below φ⁻³ threshold — boost SONA`,
      severity: 'medium',
      confidence,
      score: judgment.score || 0,
      verdict: judgment.verdict || 'BARK',
      velocity,
    });
  }

  return null;
}
