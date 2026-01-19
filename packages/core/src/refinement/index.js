/**
 * CYNIC Self-Refinement System
 *
 * "φ distrusts φ" - CYNIC critiques even itself
 *
 * Self-refinement loop:
 * 1. Judge → Initial Q-Score
 * 2. Critique → Identify weaknesses, biases, errors
 * 3. Refine → Re-judge with critique context
 * 4. Compare → Track improvement, store learning
 *
 * @module @cynic/core/refinement
 * @philosophy Self-doubt leads to self-improvement
 */

'use strict';

import { PHI_INV, THRESHOLDS } from '../axioms/constants.js';
import { analyzeWeaknesses, getVerdict, calculateQScoreFromAxioms } from '../qscore/index.js';

// =============================================================================
// CRITIQUE GENERATION
// =============================================================================

/**
 * Critique types that can be identified
 */
const CRITIQUE_TYPES = {
  WEAK_AXIOM: 'weak_axiom',          // One axiom significantly lower than others
  IMBALANCE: 'imbalance',            // Large gap between strongest/weakest
  BIAS_DETECTED: 'bias_detected',     // Potential bias in scoring
  MISSING_CONTEXT: 'missing_context', // Important context may be missing
  OVERCONFIDENCE: 'overconfidence',   // Confidence too high for evidence
  UNDERCONFIDENCE: 'underconfidence', // Confidence too low for clear case
  THRESHOLD_EDGE: 'threshold_edge',   // Score near verdict threshold
  DIMENSION_SPARSE: 'dimension_sparse', // Too few dimensions scored
};

/**
 * Critique a judgment result
 *
 * @param {Object} judgment - Original judgment with Q-Score
 * @param {Object} [context] - Additional context about the item
 * @returns {Object} Critique with issues and recommendations
 */
export function critiqueJudgment(judgment, context = {}) {
  const critiques = [];
  const recommendations = [];

  // Get breakdown
  const breakdown = judgment.breakdown || judgment.axiomScores;
  const Q = judgment.Q || judgment.qScore || judgment.q_score;
  const verdict = judgment.verdict;

  if (!breakdown) {
    return {
      critiques: [{ type: CRITIQUE_TYPES.MISSING_CONTEXT, message: 'No axiom breakdown available' }],
      recommendations: ['Re-judge with explicit axiom scoring'],
      severity: 'high',
      refinable: false,
    };
  }

  // 1. Analyze axiom weaknesses
  const weakness = analyzeWeaknesses({ breakdown, Q });

  if (weakness.imbalance > 30) {
    critiques.push({
      type: CRITIQUE_TYPES.IMBALANCE,
      message: `Large imbalance: ${weakness.strongestAxiom}(${weakness.strongestScore}) vs ${weakness.weakestAxiom}(${weakness.weakestScore})`,
      data: weakness,
    });
    recommendations.push(`Re-evaluate ${weakness.weakestAxiom} - may be underscored due to missing information`);
  }

  if (weakness.weakestScore < THRESHOLDS.GROWL) {
    critiques.push({
      type: CRITIQUE_TYPES.WEAK_AXIOM,
      message: `${weakness.weakestAxiom} critically low at ${weakness.weakestScore}`,
      data: { axiom: weakness.weakestAxiom, score: weakness.weakestScore },
    });
    recommendations.push(`Consider if ${weakness.weakestAxiom} score reflects actual quality or measurement error`);
  }

  // 2. Check for threshold edge cases
  const thresholds = [THRESHOLDS.HOWL, THRESHOLDS.WAG, THRESHOLDS.GROWL];
  for (const threshold of thresholds) {
    const distance = Math.abs(Q - threshold);
    if (distance < 3) {
      critiques.push({
        type: CRITIQUE_TYPES.THRESHOLD_EDGE,
        message: `Q-Score ${Q} is within 3 points of ${threshold} threshold`,
        data: { Q, threshold, distance },
      });
      recommendations.push(`Edge case: small changes could flip verdict. Verify scores carefully.`);
      break;
    }
  }

  // 3. Check confidence vs evidence
  const confidence = judgment.confidence || verdict?.confidence || 0.5;

  if (confidence > PHI_INV) {
    critiques.push({
      type: CRITIQUE_TYPES.OVERCONFIDENCE,
      message: `Confidence ${(confidence * 100).toFixed(1)}% exceeds φ⁻¹ ceiling of 61.8%`,
      data: { confidence, ceiling: PHI_INV },
    });
    recommendations.push('Apply φ⁻¹ ceiling - max confidence should be 61.8%');
  }

  // 4. Check for potential biases (based on context)
  if (context.source) {
    // Known source bias check
    const knownBiasedSources = ['self', 'author', 'creator', 'affiliated'];
    const sourceLower = context.source.toLowerCase();
    if (knownBiasedSources.some(bias => sourceLower.includes(bias))) {
      critiques.push({
        type: CRITIQUE_TYPES.BIAS_DETECTED,
        message: `Source "${context.source}" may have inherent bias`,
        data: { source: context.source },
      });
      recommendations.push('Consider independent verification - source may be biased');
    }
  }

  // 5. Calculate overall severity
  const severity = calculateSeverity(critiques);

  // 6. Determine if refinement would help
  const refinable = critiques.length > 0 &&
    critiques.some(c =>
      c.type === CRITIQUE_TYPES.WEAK_AXIOM ||
      c.type === CRITIQUE_TYPES.IMBALANCE ||
      c.type === CRITIQUE_TYPES.THRESHOLD_EDGE
    );

  return {
    critiques,
    recommendations,
    severity,
    refinable,
    originalQ: Q,
    originalVerdict: typeof verdict === 'string' ? verdict : verdict?.verdict,
    meta: {
      critiqueCount: critiques.length,
      timestamp: new Date().toISOString(),
    },
  };
}

/**
 * Calculate severity from critique list
 */
function calculateSeverity(critiques) {
  if (critiques.length === 0) return 'none';

  const hasHigh = critiques.some(c =>
    c.type === CRITIQUE_TYPES.OVERCONFIDENCE ||
    c.type === CRITIQUE_TYPES.WEAK_AXIOM
  );

  const hasMedium = critiques.some(c =>
    c.type === CRITIQUE_TYPES.IMBALANCE ||
    c.type === CRITIQUE_TYPES.THRESHOLD_EDGE
  );

  if (hasHigh) return 'high';
  if (hasMedium) return 'medium';
  return 'low';
}

// =============================================================================
// REFINEMENT
// =============================================================================

/**
 * Generate refined scores based on critique
 *
 * @param {Object} originalJudgment - Original judgment
 * @param {Object} critique - Critique of the judgment
 * @param {Object} [options] - Refinement options
 * @returns {Object} Suggested refined scores
 */
export function suggestRefinement(originalJudgment, critique, options = {}) {
  const { adjustmentFactor = 0.1 } = options;

  const breakdown = originalJudgment.breakdown || originalJudgment.axiomScores;
  if (!breakdown) {
    return { error: 'No breakdown available for refinement' };
  }

  const refined = { ...breakdown };
  const adjustments = [];

  for (const crit of critique.critiques) {
    switch (crit.type) {
      case CRITIQUE_TYPES.WEAK_AXIOM: {
        const axiom = crit.data.axiom;
        const currentScore = crit.data.score;
        // Suggest modest increase if critically low (may be underscored)
        const suggested = Math.min(currentScore + 10, THRESHOLDS.WAG);
        adjustments.push({
          axiom,
          from: currentScore,
          to: suggested,
          reason: 'Potentially underscored - review for missing positive signals',
        });
        refined[axiom] = suggested;
        break;
      }

      case CRITIQUE_TYPES.IMBALANCE: {
        // Suggest reducing extreme scores slightly
        const { strongestAxiom, strongestScore, weakestAxiom, weakestScore } = crit.data;
        if (strongestScore > 85) {
          const suggested = strongestScore - 5;
          adjustments.push({
            axiom: strongestAxiom,
            from: strongestScore,
            to: suggested,
            reason: 'Very high score - verify not over-rated',
          });
          refined[strongestAxiom] = suggested;
        }
        break;
      }

      case CRITIQUE_TYPES.OVERCONFIDENCE: {
        // Note: confidence adjustment handled in final output
        adjustments.push({
          field: 'confidence',
          from: crit.data.confidence,
          to: PHI_INV,
          reason: 'Apply φ⁻¹ ceiling',
        });
        break;
      }
    }
  }

  // Calculate new Q-Score from refined scores
  const refinedResult = calculateQScoreFromAxioms(refined);

  return {
    originalBreakdown: breakdown,
    refinedBreakdown: refined,
    adjustments,
    originalQ: critique.originalQ,
    refinedQ: refinedResult.Q,
    improvement: refinedResult.Q - critique.originalQ,
    verdictChanged: refinedResult.verdict.verdict !== critique.originalVerdict,
    newVerdict: refinedResult.verdict,
  };
}

// =============================================================================
// SELF-REFINEMENT LOOP
// =============================================================================

/**
 * Run complete self-refinement loop
 *
 * @param {Object} judgment - Original judgment
 * @param {Object} [context] - Context about the judged item
 * @param {Object} [options] - Options
 * @returns {Object} Complete refinement result
 */
export function selfRefine(judgment, context = {}, options = {}) {
  const { maxIterations = 3 } = options;

  const iterations = [];
  let currentJudgment = judgment;
  let improved = false;

  for (let i = 0; i < maxIterations; i++) {
    // Critique current judgment
    const critique = critiqueJudgment(currentJudgment, context);

    // If no refinement needed, stop
    if (!critique.refinable) {
      iterations.push({
        iteration: i + 1,
        critique,
        refinement: null,
        status: 'stable',
      });
      break;
    }

    // Generate refinement
    const refinement = suggestRefinement(currentJudgment, critique);

    iterations.push({
      iteration: i + 1,
      critique,
      refinement,
      status: refinement.improvement > 0 ? 'improved' : 'no_improvement',
    });

    // Check if we improved
    if (refinement.improvement <= 0) {
      break; // No more improvement possible
    }

    improved = true;

    // Update for next iteration
    currentJudgment = {
      ...currentJudgment,
      breakdown: refinement.refinedBreakdown,
      Q: refinement.refinedQ,
      verdict: refinement.newVerdict,
    };
  }

  const finalIteration = iterations[iterations.length - 1];
  const finalQ = finalIteration.refinement?.refinedQ || judgment.Q || judgment.qScore;
  const originalQ = judgment.Q || judgment.qScore;

  return {
    original: {
      Q: originalQ,
      verdict: judgment.verdict,
      breakdown: judgment.breakdown || judgment.axiomScores,
    },
    final: {
      Q: finalQ,
      verdict: getVerdict(finalQ),
      breakdown: finalIteration.refinement?.refinedBreakdown || judgment.breakdown,
    },
    iterations,
    improved,
    totalImprovement: finalQ - originalQ,
    iterationCount: iterations.length,
    meta: {
      maxIterations,
      timestamp: new Date().toISOString(),
    },
  };
}

// =============================================================================
// LEARNING FROM REFINEMENT
// =============================================================================

/**
 * Extract learning from refinement for future judgments
 *
 * @param {Object} refinementResult - Result from selfRefine
 * @returns {Object} Learning to store
 */
export function extractLearning(refinementResult) {
  const learnings = [];

  for (const iter of refinementResult.iterations) {
    if (!iter.refinement) continue;

    for (const adj of iter.refinement.adjustments) {
      if (adj.axiom) {
        learnings.push({
          type: 'axiom_adjustment',
          axiom: adj.axiom,
          pattern: adj.reason,
          adjustment: adj.to - adj.from,
        });
      } else if (adj.field === 'confidence') {
        learnings.push({
          type: 'confidence_calibration',
          pattern: 'Overconfidence detected',
          correction: 'Apply φ⁻¹ ceiling',
        });
      }
    }
  }

  return {
    learnings,
    improved: refinementResult.improved,
    improvementMagnitude: refinementResult.totalImprovement,
    timestamp: new Date().toISOString(),
  };
}

// =============================================================================
// EXPORTS
// =============================================================================

export default {
  // Main functions
  critiqueJudgment,
  suggestRefinement,
  selfRefine,
  extractLearning,

  // Types
  CRITIQUE_TYPES,
};
