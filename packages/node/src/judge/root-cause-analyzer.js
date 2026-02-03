/**
 * Root Cause Analyzer - Fractal Oracle Resolution
 *
 * "Qui vérifie le vérificateur?" - κυνικός
 *
 * Applies the SAME pattern used at all CYNIC scales to the test vs code problem:
 *
 * Scale 1: Dimension judges dimension (THE_UNNAMEABLE)
 * Scale 2: Judgment judges judgment (SelfSkeptic)
 * Scale 5: Routing judges routing (Q-Learning)
 * Scale N: Oracle judges oracle (THIS MODULE)
 *
 * When test fails, we have 4 hypotheses:
 * - H1: Code is buggy (fix code)
 * - H2: Test is wrong (fix test specification)
 * - H3: Spec is incomplete (discover new dimension)
 * - H4: Environment is flaky (retry/ignore)
 *
 * Solution: Multiple independent oracles vote, φ-bounded confidence,
 * residual measures disagreement, learn from outcomes.
 *
 * @module @cynic/node/judge/root-cause-analyzer
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

/**
 * Oracle types that can provide ground truth signals
 */
export const OracleType = {
  TEST_RESULT: 'test_result',       // Automated tests pass/fail
  BUILD_STATUS: 'build_status',     // Build success/failure
  COMMIT_HISTORY: 'commit_history', // Recent changes pattern
  PR_FEEDBACK: 'pr_feedback',       // PR merged/rejected
  LLM_CONSENSUS: 'llm_consensus',   // Multiple LLMs agree
  HUMAN_REVIEW: 'human_review',     // Explicit human feedback
  STATIC_ANALYSIS: 'static_analysis', // Linter/type checker
  RUNTIME_BEHAVIOR: 'runtime_behavior', // Actual execution
};

/**
 * Root cause hypotheses
 */
export const Hypothesis = {
  CODE_BUG: 'code_bug',           // The implementation is wrong
  TEST_BUG: 'test_bug',           // The test specification is wrong
  SPEC_GAP: 'spec_gap',           // Missing dimension/requirement
  FLAKY: 'flaky',                 // Environment/timing issue
  UNKNOWN: 'unknown',             // Can't determine with confidence
};

/**
 * Decision thresholds (φ-aligned)
 */
export const DECISION_THRESHOLDS = {
  /** Confidence needed to assert code bug */
  CODE_BUG_THRESHOLD: PHI_INV,        // 61.8%

  /** Confidence needed to assert test bug */
  TEST_BUG_THRESHOLD: PHI_INV,        // 61.8%

  /** Confidence to trigger spec gap investigation */
  SPEC_GAP_THRESHOLD: PHI_INV_2,      // 38.2%

  /** Flakiness detection threshold */
  FLAKY_THRESHOLD: PHI_INV_2,         // 38.2%

  /** Below this, escalate to human */
  ESCALATION_THRESHOLD: PHI_INV_3,    // 23.6%

  /** Max learning adjustment per feedback */
  MAX_LEARNING_RATE: PHI_INV_2,       // 38.2%

  /** Min oracles needed for decision */
  MIN_ORACLES: 2,

  /** Ideal oracles for strong decision */
  IDEAL_ORACLES: 5,
};

/**
 * Default oracle trust scores (prior confidence in each oracle type)
 */
export const DEFAULT_ORACLE_TRUST = {
  [OracleType.TEST_RESULT]: 0.7,
  [OracleType.BUILD_STATUS]: 0.8,
  [OracleType.COMMIT_HISTORY]: 0.5,
  [OracleType.PR_FEEDBACK]: 0.75,
  [OracleType.LLM_CONSENSUS]: 0.6,
  [OracleType.HUMAN_REVIEW]: 0.85,
  [OracleType.STATIC_ANALYSIS]: 0.7,
  [OracleType.RUNTIME_BEHAVIOR]: 0.9,
};

/**
 * Evidence for root cause analysis
 * @typedef {Object} OracleEvidence
 * @property {string} type - Oracle type
 * @property {string} verdict - What the oracle says (pass/fail/unknown)
 * @property {number} confidence - Oracle's confidence [0, 1]
 * @property {Object} [data] - Additional data from oracle
 * @property {number} timestamp - When evidence was collected
 */

/**
 * Root cause decision
 * @typedef {Object} RootCauseDecision
 * @property {string} hypothesis - Most likely root cause
 * @property {Object} probabilities - P(hypothesis) for each
 * @property {number} confidence - Overall decision confidence
 * @property {string[]} reasoning - Why this decision
 * @property {string} action - Recommended action
 * @property {boolean} escalate - Should escalate to human?
 */

/**
 * Root Cause Analyzer
 *
 * Determines whether a mismatch is due to code bug, test bug, spec gap, or flakiness.
 *
 * @example
 * ```javascript
 * const analyzer = createRootCauseAnalyzer();
 *
 * // Collect evidence from multiple oracles
 * analyzer.addEvidence({
 *   type: OracleType.TEST_RESULT,
 *   verdict: 'fail',
 *   confidence: 0.9,
 *   data: { testName: 'should work', failCount: 3 }
 * });
 *
 * analyzer.addEvidence({
 *   type: OracleType.LLM_CONSENSUS,
 *   verdict: 'pass',
 *   confidence: 0.7,
 *   data: { agreement: 0.8, llmCount: 3 }
 * });
 *
 * // Analyze root cause
 * const decision = analyzer.analyze();
 * // { hypothesis: 'test_bug', confidence: 0.58, ... }
 *
 * // After finding actual cause, provide feedback
 * analyzer.feedback(decision.id, 'test_bug', true);
 * ```
 */
export class RootCauseAnalyzer extends EventEmitter {
  /**
   * @param {Object} options - Configuration
   * @param {Object} [options.oracleTrust] - Override default oracle trust scores
   * @param {number} [options.maxHistory] - Max decisions to remember
   */
  constructor(options = {}) {
    super();

    // Oracle trust scores (learned over time)
    this.oracleTrust = { ...DEFAULT_ORACLE_TRUST, ...options.oracleTrust };

    // Current evidence buffer
    this.evidence = [];

    // Decision history for learning
    this.history = [];
    this.maxHistory = options.maxHistory || 1000;

    // Learning statistics
    this.stats = {
      totalDecisions: 0,
      correctDecisions: 0,
      byHypothesis: {
        [Hypothesis.CODE_BUG]: { total: 0, correct: 0 },
        [Hypothesis.TEST_BUG]: { total: 0, correct: 0 },
        [Hypothesis.SPEC_GAP]: { total: 0, correct: 0 },
        [Hypothesis.FLAKY]: { total: 0, correct: 0 },
        [Hypothesis.UNKNOWN]: { total: 0, correct: 0 },
      },
      byOracle: {},
    };

    // Initialize oracle stats
    for (const type of Object.values(OracleType)) {
      this.stats.byOracle[type] = { total: 0, correct: 0, trust: this.oracleTrust[type] };
    }
  }

  /**
   * Add evidence from an oracle
   *
   * @param {OracleEvidence} evidence - Evidence to add
   */
  addEvidence(evidence) {
    const normalizedEvidence = {
      type: evidence.type,
      verdict: evidence.verdict,
      confidence: Math.min(PHI_INV, evidence.confidence), // Cap at φ⁻¹
      data: evidence.data || {},
      timestamp: evidence.timestamp || Date.now(),
    };

    this.evidence.push(normalizedEvidence);
    this.emit('evidence', normalizedEvidence);
  }

  /**
   * Clear current evidence buffer
   */
  clearEvidence() {
    this.evidence = [];
  }

  /**
   * Analyze root cause from collected evidence
   *
   * @param {Object} [context] - Additional context
   * @returns {RootCauseDecision} Decision with reasoning
   */
  analyze(context = {}) {
    if (this.evidence.length < DECISION_THRESHOLDS.MIN_ORACLES) {
      return this._createDecision(Hypothesis.UNKNOWN, {
        reason: 'INSUFFICIENT_EVIDENCE',
        required: DECISION_THRESHOLDS.MIN_ORACLES,
        present: this.evidence.length,
      });
    }

    // Step 1: Apply skepticism to each oracle (φ-bounded trust)
    const adjustedEvidence = this._applySkepticism(this.evidence);

    // Step 2: Calculate probability for each hypothesis
    const probabilities = this._calculateProbabilities(adjustedEvidence, context);

    // Step 3: Calculate residual (unexplained disagreement)
    const residual = this._calculateResidual(adjustedEvidence);

    // Step 4: Make decision
    const decision = this._decide(probabilities, residual, adjustedEvidence);

    // Step 5: Generate reasoning
    decision.reasoning = this._generateReasoning(adjustedEvidence, probabilities, residual);

    // Record for learning
    this._recordDecision(decision);

    this.emit('decision', decision);
    return decision;
  }

  /**
   * Provide feedback on a decision (for learning)
   *
   * @param {string} decisionId - Decision ID
   * @param {string} actualCause - What the actual root cause was
   * @param {boolean} wasCorrect - Was our decision correct?
   */
  feedback(decisionId, actualCause, wasCorrect) {
    const decision = this.history.find(d => d.id === decisionId);
    if (!decision) return;

    decision.feedback = {
      actualCause,
      wasCorrect,
      timestamp: Date.now(),
    };

    // Update statistics
    this.stats.totalDecisions++;
    if (wasCorrect) this.stats.correctDecisions++;

    this.stats.byHypothesis[decision.hypothesis].total++;
    if (wasCorrect) this.stats.byHypothesis[decision.hypothesis].correct++;

    // Learn: adjust oracle trust based on outcome
    this._learn(decision, actualCause, wasCorrect);

    this.emit('feedback', { decisionId, actualCause, wasCorrect });
  }

  /**
   * Create a decision object (for insufficient evidence cases)
   * @private
   */
  _createDecision(hypothesis, details = {}) {
    const id = `rca_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

    return {
      id,
      hypothesis,
      probabilities: {
        [Hypothesis.CODE_BUG]: 0,
        [Hypothesis.TEST_BUG]: 0,
        [Hypothesis.SPEC_GAP]: 0,
        [Hypothesis.FLAKY]: 0,
      },
      confidence: 0,
      residual: 0,
      evidenceCount: this.evidence.length,
      escalate: true,
      action: 'GATHER_MORE_EVIDENCE',
      reasoning: [`Insufficient evidence: ${details.reason || 'unknown'}`],
      timestamp: Date.now(),
      ...details,
    };
  }

  /**
   * Apply skepticism to oracle evidence (trust × φ⁻¹)
   * @private
   */
  _applySkepticism(evidence) {
    return evidence.map(e => {
      const trust = this.oracleTrust[e.type] || 0.5;

      // Adjusted confidence = oracle confidence × trust × φ⁻¹
      // This ensures no oracle can have > 61.8% effective confidence
      const adjustedConfidence = Math.min(PHI_INV, e.confidence * trust * PHI_INV);

      return {
        ...e,
        originalConfidence: e.confidence,
        trust,
        adjustedConfidence,
      };
    });
  }

  /**
   * Calculate probability for each hypothesis
   * @private
   */
  _calculateProbabilities(adjustedEvidence, context) {
    const probs = {
      [Hypothesis.CODE_BUG]: 0,
      [Hypothesis.TEST_BUG]: 0,
      [Hypothesis.SPEC_GAP]: 0,
      [Hypothesis.FLAKY]: 0,
    };

    let totalWeight = 0;

    for (const e of adjustedEvidence) {
      const weight = e.adjustedConfidence;
      totalWeight += weight;

      // Interpret verdict for each hypothesis
      if (e.verdict === 'fail') {
        // Test fails → likely code bug OR test bug
        // Check patterns to differentiate
        if (this._isLikelyTestBug(e)) {
          probs[Hypothesis.TEST_BUG] += weight * 0.7;
          probs[Hypothesis.CODE_BUG] += weight * 0.2;
        } else {
          probs[Hypothesis.CODE_BUG] += weight * 0.6;
          probs[Hypothesis.TEST_BUG] += weight * 0.3;
        }
        probs[Hypothesis.SPEC_GAP] += weight * 0.1;

      } else if (e.verdict === 'pass') {
        // Oracle says pass but we're analyzing a mismatch
        // This oracle disagrees with the failure → suggests test bug
        probs[Hypothesis.TEST_BUG] += weight * 0.5;
        probs[Hypothesis.SPEC_GAP] += weight * 0.3;
        probs[Hypothesis.CODE_BUG] += weight * 0.1;
        probs[Hypothesis.FLAKY] += weight * 0.1;

      } else if (e.verdict === 'flaky' || e.verdict === 'intermittent') {
        probs[Hypothesis.FLAKY] += weight * 0.8;
        probs[Hypothesis.CODE_BUG] += weight * 0.1;
        probs[Hypothesis.TEST_BUG] += weight * 0.1;
      }
    }

    // Normalize to [0, 1]
    if (totalWeight > 0) {
      for (const h of Object.keys(probs)) {
        probs[h] = Math.min(PHI_INV, probs[h] / totalWeight);
      }
    }

    // Context adjustments
    if (context.recentCodeChange) {
      probs[Hypothesis.CODE_BUG] *= 1.3;
    }
    if (context.recentTestChange) {
      probs[Hypothesis.TEST_BUG] *= 1.3;
    }
    if (context.newFeature) {
      probs[Hypothesis.SPEC_GAP] *= 1.5;
    }
    if (context.flakeHistory) {
      probs[Hypothesis.FLAKY] *= 1.5;
    }

    // Re-normalize after context
    const sum = Object.values(probs).reduce((a, b) => a + b, 0);
    if (sum > 0) {
      for (const h of Object.keys(probs)) {
        probs[h] = Math.min(PHI_INV, probs[h] / sum);
      }
    }

    return probs;
  }

  /**
   * Check if evidence suggests test bug (vs code bug)
   * @private
   */
  _isLikelyTestBug(evidence) {
    const data = evidence.data || {};

    // Patterns suggesting test bug:
    // - Test was recently changed
    // - Test uses mocked/stubbed values
    // - Test has specific assertion on implementation detail
    // - LLM consensus disagrees with test

    if (data.testRecentlyChanged) return true;
    if (data.usesMocks && data.mockMismatch) return true;
    if (evidence.type === OracleType.LLM_CONSENSUS && evidence.verdict === 'pass') return true;

    return false;
  }

  /**
   * Calculate residual (unexplained disagreement between oracles)
   * @private
   */
  _calculateResidual(adjustedEvidence) {
    if (adjustedEvidence.length < 2) return 0;

    // Calculate agreement/disagreement
    const verdicts = adjustedEvidence.map(e => e.verdict);
    const uniqueVerdicts = [...new Set(verdicts)];

    if (uniqueVerdicts.length === 1) {
      // All agree → low residual
      return 0;
    }

    // Count weighted agreement
    const verdictWeights = {};
    let totalWeight = 0;

    for (const e of adjustedEvidence) {
      verdictWeights[e.verdict] = (verdictWeights[e.verdict] || 0) + e.adjustedConfidence;
      totalWeight += e.adjustedConfidence;
    }

    // Residual = 1 - max_agreement_ratio
    const maxWeight = Math.max(...Object.values(verdictWeights));
    const residual = 1 - (maxWeight / totalWeight);

    return residual;
  }

  /**
   * Make final decision based on probabilities and residual
   * @private
   */
  _decide(probabilities, residual, evidence) {
    const id = `rca_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

    // Find highest probability hypothesis
    let maxProb = 0;
    let bestHypothesis = Hypothesis.UNKNOWN;

    for (const [hypothesis, prob] of Object.entries(probabilities)) {
      if (prob > maxProb) {
        maxProb = prob;
        bestHypothesis = hypothesis;
      }
    }

    // Adjust confidence based on residual
    // High residual = oracles disagree = less confidence
    const residualPenalty = residual * PHI_INV_2;
    const confidence = Math.max(0, Math.min(PHI_INV, maxProb - residualPenalty));

    // Determine if we should escalate
    const escalate = confidence < DECISION_THRESHOLDS.ESCALATION_THRESHOLD ||
                     residual > PHI_INV;

    // Check if confidence meets threshold for assertion
    let hypothesis = bestHypothesis;
    if (bestHypothesis === Hypothesis.CODE_BUG &&
        confidence < DECISION_THRESHOLDS.CODE_BUG_THRESHOLD) {
      // Not confident enough to assert code bug
      if (residual > DECISION_THRESHOLDS.SPEC_GAP_THRESHOLD) {
        hypothesis = Hypothesis.SPEC_GAP;
      } else {
        hypothesis = Hypothesis.UNKNOWN;
      }
    }
    if (bestHypothesis === Hypothesis.TEST_BUG &&
        confidence < DECISION_THRESHOLDS.TEST_BUG_THRESHOLD) {
      hypothesis = Hypothesis.UNKNOWN;
    }

    // Determine action
    const action = this._determineAction(hypothesis, confidence, escalate);

    return {
      id,
      hypothesis,
      probabilities,
      confidence,
      residual,
      evidenceCount: evidence.length,
      escalate,
      action,
      timestamp: Date.now(),
    };
  }

  /**
   * Determine recommended action
   * @private
   */
  _determineAction(hypothesis, confidence, escalate) {
    if (escalate) {
      return 'ESCALATE_TO_HUMAN';
    }

    switch (hypothesis) {
      case Hypothesis.CODE_BUG:
        return 'FIX_CODE_AND_LEARN';
      case Hypothesis.TEST_BUG:
        return 'FLAG_TEST_FOR_REVIEW';
      case Hypothesis.SPEC_GAP:
        return 'TRIGGER_DIMENSION_DISCOVERY';
      case Hypothesis.FLAKY:
        return 'RETRY_AND_TRACK_FLAKINESS';
      default:
        return 'GATHER_MORE_EVIDENCE';
    }
  }

  /**
   * Generate reasoning for the decision
   * @private
   */
  _generateReasoning(evidence, probabilities, residual) {
    const reasoning = [];

    // Evidence summary
    const verdictCounts = {};
    for (const e of evidence) {
      verdictCounts[e.verdict] = (verdictCounts[e.verdict] || 0) + 1;
    }
    reasoning.push(`Evidence: ${evidence.length} oracles (${JSON.stringify(verdictCounts)})`);

    // Probability leaders
    const sorted = Object.entries(probabilities).sort((a, b) => b[1] - a[1]);
    reasoning.push(`Top hypothesis: ${sorted[0][0]} (${Math.round(sorted[0][1] * 100)}%)`);
    if (sorted.length > 1 && sorted[1][1] > 0.2) {
      reasoning.push(`Alternative: ${sorted[1][0]} (${Math.round(sorted[1][1] * 100)}%)`);
    }

    // Residual interpretation
    if (residual > PHI_INV) {
      reasoning.push(`High residual (${Math.round(residual * 100)}%): oracles strongly disagree`);
    } else if (residual > PHI_INV_2) {
      reasoning.push(`Moderate residual (${Math.round(residual * 100)}%): some oracle disagreement`);
    } else {
      reasoning.push(`Low residual (${Math.round(residual * 100)}%): oracles mostly agree`);
    }

    // Specific evidence notes
    const llmEvidence = evidence.find(e => e.type === OracleType.LLM_CONSENSUS);
    const testEvidence = evidence.find(e => e.type === OracleType.TEST_RESULT);

    if (llmEvidence && testEvidence && llmEvidence.verdict !== testEvidence.verdict) {
      reasoning.push('LLM consensus disagrees with test result - test spec may be wrong');
    }

    return reasoning;
  }

  /**
   * Record decision for learning
   * @private
   */
  _recordDecision(decision) {
    this.history.push({
      ...decision,
      evidence: [...this.evidence],
    });

    // Keep bounded
    if (this.history.length > this.maxHistory) {
      this.history.shift();
    }
  }

  /**
   * Learn from feedback (adjust oracle trust)
   * @private
   */
  _learn(decision, actualCause, wasCorrect) {
    // For each oracle that contributed to this decision,
    // adjust trust based on whether it pointed to the right answer

    for (const e of decision.evidence || []) {
      const oracleStats = this.stats.byOracle[e.type];
      if (!oracleStats) continue;

      oracleStats.total++;

      // Did this oracle point toward the actual cause?
      const oracleWasHelpful = this._oracleWasHelpful(e, actualCause);

      if (oracleWasHelpful) {
        oracleStats.correct++;
        // Increase trust (bounded by φ⁻² max adjustment)
        const adjustment = DECISION_THRESHOLDS.MAX_LEARNING_RATE * 0.1;
        this.oracleTrust[e.type] = Math.min(0.95, this.oracleTrust[e.type] + adjustment);
      } else {
        // Decrease trust (bounded)
        const adjustment = DECISION_THRESHOLDS.MAX_LEARNING_RATE * 0.1;
        this.oracleTrust[e.type] = Math.max(0.1, this.oracleTrust[e.type] - adjustment);
      }

      oracleStats.trust = this.oracleTrust[e.type];
    }

    this.emit('learned', {
      decisionId: decision.id,
      actualCause,
      wasCorrect,
      updatedTrust: { ...this.oracleTrust },
    });
  }

  /**
   * Check if oracle evidence was helpful for finding actual cause
   * @private
   */
  _oracleWasHelpful(evidence, actualCause) {
    // Oracle was helpful if its verdict aligned with actual cause

    if (actualCause === Hypothesis.CODE_BUG) {
      return evidence.verdict === 'fail';
    }
    if (actualCause === Hypothesis.TEST_BUG) {
      return evidence.verdict === 'pass' || evidence.type === OracleType.LLM_CONSENSUS;
    }
    if (actualCause === Hypothesis.FLAKY) {
      return evidence.verdict === 'flaky' || evidence.verdict === 'intermittent';
    }

    return false;
  }

  /**
   * Get current oracle trust scores
   * @returns {Object}
   */
  getOracleTrust() {
    return { ...this.oracleTrust };
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    const accuracy = this.stats.totalDecisions > 0
      ? this.stats.correctDecisions / this.stats.totalDecisions
      : 0;

    return {
      totalDecisions: this.stats.totalDecisions,
      correctDecisions: this.stats.correctDecisions,
      accuracy,
      byHypothesis: { ...this.stats.byHypothesis },
      byOracle: { ...this.stats.byOracle },
      oracleTrust: { ...this.oracleTrust },
    };
  }

  /**
   * Export state for persistence
   * @returns {Object}
   */
  export() {
    return {
      oracleTrust: { ...this.oracleTrust },
      stats: JSON.parse(JSON.stringify(this.stats)),
      history: this.history.slice(-100), // Last 100 decisions
      exportedAt: Date.now(),
    };
  }

  /**
   * Import state from persistence
   * @param {Object} data
   */
  import(data) {
    if (data.oracleTrust) {
      this.oracleTrust = { ...DEFAULT_ORACLE_TRUST, ...data.oracleTrust };
    }
    if (data.stats) {
      this.stats = data.stats;
    }
    if (data.history) {
      this.history = data.history;
    }
  }

  /**
   * Reset learning
   */
  reset() {
    this.oracleTrust = { ...DEFAULT_ORACLE_TRUST };
    this.evidence = [];
    this.history = [];
    this.stats = {
      totalDecisions: 0,
      correctDecisions: 0,
      byHypothesis: {
        [Hypothesis.CODE_BUG]: { total: 0, correct: 0 },
        [Hypothesis.TEST_BUG]: { total: 0, correct: 0 },
        [Hypothesis.SPEC_GAP]: { total: 0, correct: 0 },
        [Hypothesis.FLAKY]: { total: 0, correct: 0 },
        [Hypothesis.UNKNOWN]: { total: 0, correct: 0 },
      },
      byOracle: {},
    };
    for (const type of Object.values(OracleType)) {
      this.stats.byOracle[type] = { total: 0, correct: 0, trust: this.oracleTrust[type] };
    }
  }
}

/**
 * Create a RootCauseAnalyzer instance
 * @param {Object} [options] - Configuration
 * @returns {RootCauseAnalyzer}
 */
export function createRootCauseAnalyzer(options = {}) {
  return new RootCauseAnalyzer(options);
}

// Singleton instance
let _instance = null;

/**
 * Get singleton RootCauseAnalyzer
 * @param {Object} [options] - Configuration (only used on first call)
 * @returns {RootCauseAnalyzer}
 */
export function getRootCauseAnalyzer(options = {}) {
  if (!_instance) {
    _instance = createRootCauseAnalyzer(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function _resetRootCauseAnalyzerForTesting() {
  _instance = null;
}

export default {
  RootCauseAnalyzer,
  createRootCauseAnalyzer,
  getRootCauseAnalyzer,
  OracleType,
  Hypothesis,
  DECISION_THRESHOLDS,
  DEFAULT_ORACLE_TRUST,
};
