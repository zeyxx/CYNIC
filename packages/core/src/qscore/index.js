/**
 * Q-SCORE - Hierarchical Quality Score
 *
 * "Quality > Quantity, mathematically proven"
 *
 * Q = 100 × ∜(φ × V × C × B)
 *
 * Like K-Score but for knowledge:
 * - K-Score: K = 100 × ∛(D × O × L) - 3 pillars, balanced
 * - Q-Score: Q = 100 × ∜(φ × V × C × B) - 4 axioms, balanced
 *
 * Key insight: One weak pillar = low score (cannot game by excelling elsewhere)
 *
 * @module @cynic/core/qscore
 * @philosophy Geometric mean forces balance across all axioms
 */

'use strict';

import {
  PHI,
  PHI_INV,
  PHI_INV_2,
  THRESHOLDS,
  AXIOMS,
} from '../axioms/constants.js';

// =============================================================================
// GEOMETRIC MEAN HELPERS
// =============================================================================

/**
 * Calculate geometric mean of an array of values
 * Uses log-sum-exp for numerical stability
 *
 * @param {number[]} values - Array of positive numbers
 * @returns {number} Geometric mean
 */
export function geometricMean(values) {
  if (!values || values.length === 0) return 0;

  // Filter out zeros/negatives to avoid NaN
  const positives = values.filter((v) => v > 0);
  if (positives.length === 0) return 0;

  // Use log-sum-exp for numerical stability
  const logSum = positives.reduce((sum, v) => sum + Math.log(v), 0);
  return Math.exp(logSum / positives.length);
}

/**
 * Calculate nth root
 *
 * @param {number} value - Value to take root of
 * @param {number} n - Root degree
 * @returns {number} nth root
 */
export function nthRoot(value, n) {
  if (value <= 0) return 0;
  return Math.pow(value, 1 / n);
}

// =============================================================================
// AXIOM SCORE CALCULATION
// =============================================================================

/**
 * Default dimension definitions per axiom
 * Used when no explicit dimensions are provided
 */
const DEFAULT_AXIOM_DIMENSIONS = {
  PHI: ['COHERENCE', 'HARMONY', 'STRUCTURE', 'ELEGANCE', 'COMPLETENESS', 'PRECISION'],
  VERIFY: ['ACCURACY', 'VERIFIABILITY', 'TRANSPARENCY', 'REPRODUCIBILITY', 'PROVENANCE', 'INTEGRITY'],
  CULTURE: ['AUTHENTICITY', 'RELEVANCE', 'NOVELTY', 'ALIGNMENT', 'IMPACT', 'RESONANCE'],
  BURN: ['UTILITY', 'SUSTAINABILITY', 'EFFICIENCY', 'VALUE_CREATION', 'NON_EXTRACTIVE', 'CONTRIBUTION'],
};

/**
 * Neutral score for missing dimensions
 */
const NEUTRAL_SCORE = 50;

/**
 * Calculate score for a single axiom from dimension scores
 *
 * Uses geometric mean: √^n(d1 × d2 × ... × dn)
 *
 * @param {string} axiomName - Axiom name (PHI, VERIFY, CULTURE, BURN)
 * @param {Object} dimensionScores - { HARMONY: 75, TRUTH: 80, ... }
 * @param {Object} [options] - Options
 * @param {string[]} [options.dimensions] - Override default dimensions for axiom
 * @returns {Object} { score, dimensions, missing }
 */
export function calculateAxiomScore(axiomName, dimensionScores, options = {}) {
  const axiom = AXIOMS[axiomName];
  if (!axiom) {
    return { score: 0, error: `Unknown axiom: ${axiomName}` };
  }

  const dimensions = options.dimensions || DEFAULT_AXIOM_DIMENSIONS[axiomName] || [];
  const scores = [];
  const present = [];
  const missing = [];

  for (const dim of dimensions) {
    const score = dimensionScores[dim];
    if (score !== undefined && score !== null) {
      // Normalize to 0-100 if needed
      const normalized = score > 1 ? score : score * 100;
      scores.push(normalized);
      present.push({ dimension: dim, score: normalized });
    } else {
      missing.push(dim);
      // Use neutral score (50) for missing dimensions
      scores.push(NEUTRAL_SCORE);
    }
  }

  const score = geometricMean(scores);

  return {
    axiom: axiomName,
    score: Math.round(score * 10) / 10, // 1 decimal
    dimensions: present,
    missing,
    count: scores.length,
    world: axiom.world,
  };
}

// =============================================================================
// Q-SCORE CALCULATION
// =============================================================================

/**
 * Calculate hierarchical Q-Score
 *
 * Q = 100 × ∜(φ_score × V_score × C_score × B_score)
 *
 * @param {Object} dimensionScores - { HARMONY: 75, TRUTH: 80, ... }
 * @param {Object} [options] - { weighted: boolean }
 * @returns {Object} Q-Score result
 */
export function calculateQScore(dimensionScores, options = {}) {
  const { weighted = false } = options;

  // Calculate score for each axiom
  const axiomScores = {};
  for (const axiomName of Object.keys(AXIOMS)) {
    axiomScores[axiomName] = calculateAxiomScore(axiomName, dimensionScores);
  }

  // Extract raw scores
  const phi_score = axiomScores.PHI.score;
  const v_score = axiomScores.VERIFY.score;
  const c_score = axiomScores.CULTURE.score;
  const b_score = axiomScores.BURN.score;

  let Q;

  if (weighted) {
    // Weighted version: apply axiom weights
    // Each axiom has equal weight in balanced mode
    const weights = {
      PHI: PHI_INV,    // 0.618
      VERIFY: PHI_INV, // 0.618
      CULTURE: 1.0,    // 1.0
      BURN: PHI_INV,   // 0.618
    };

    const weightedProduct =
      Math.pow(phi_score, weights.PHI) *
      Math.pow(v_score, weights.VERIFY) *
      Math.pow(c_score, weights.CULTURE) *
      Math.pow(b_score, weights.BURN);

    const totalWeight =
      weights.PHI + weights.VERIFY + weights.CULTURE + weights.BURN;

    Q = nthRoot(weightedProduct, totalWeight);
  } else {
    // Simple 4th root (unweighted, like K-Score's cube root)
    // Q = 100 × ∜(φ × V × C × B / 100^4)
    Q = 100 * nthRoot((phi_score * v_score * c_score * b_score) / Math.pow(100, 4), 4);
  }

  // Round to 1 decimal
  Q = Math.round(Q * 10) / 10;

  // Determine verdict based on Q-Score
  const verdict = getVerdict(Q);

  return {
    Q,
    verdict,
    axiomScores,
    breakdown: {
      PHI: phi_score,
      VERIFY: v_score,
      CULTURE: c_score,
      BURN: b_score,
    },
    formula: weighted
      ? 'Q = ∜(φ^w × V^w × C^w × B^w)'
      : 'Q = 100 × ∜(φ × V × C × B / 100^4)',
    meta: {
      dimensionCount: Object.keys(dimensionScores).length,
      weighted,
      timestamp: new Date().toISOString(),
    },
  };
}

/**
 * Calculate Q-Score from pre-computed axiom scores
 *
 * Useful when axiom scores are already available (e.g., from Judge)
 *
 * @param {Object} axiomScores - { PHI: 75, VERIFY: 80, CULTURE: 60, BURN: 70 }
 * @returns {Object} Q-Score result
 */
export function calculateQScoreFromAxioms(axiomScores) {
  const { PHI: phi_score = 50, VERIFY: v_score = 50, CULTURE: c_score = 50, BURN: b_score = 50 } = axiomScores;

  // Q = 100 × ∜(φ × V × C × B / 100^4)
  const Q = Math.round(
    100 * nthRoot((phi_score * v_score * c_score * b_score) / Math.pow(100, 4), 4) * 10
  ) / 10;

  const verdict = getVerdict(Q);

  return {
    Q,
    verdict,
    breakdown: { PHI: phi_score, VERIFY: v_score, CULTURE: c_score, BURN: b_score },
    formula: 'Q = 100 × ∜(φ × V × C × B / 100^4)',
  };
}

// =============================================================================
// VERDICT DETERMINATION
// =============================================================================

/**
 * Determine verdict from Q-Score
 *
 * Uses phi-based thresholds:
 * - BARK: Q < 38.2 (phi^-2)
 * - GROWL: 38.2 <= Q < 50
 * - WAG: 50 <= Q < 80
 * - HOWL: Q >= 80
 *
 * @param {number} Q - Q-Score value
 * @returns {Object} Verdict with confidence
 */
export function getVerdict(Q) {
  // Calculate confidence based on distance from threshold
  const distanceFromThreshold = Math.abs(Q - 50) / 100;
  const rawConfidence = 0.5 + distanceFromThreshold * 0.5;

  // Apply phi ceiling - never exceed 61.8%
  const confidence = Math.min(rawConfidence, PHI_INV);
  const doubt = 1 - confidence;

  if (Q >= THRESHOLDS.HOWL) {
    return {
      verdict: 'HOWL',
      emoji: '🐺',
      reaction: '*howls approvingly*',
      confidence: Math.round(confidence * 1000) / 1000,
      doubt: Math.round(doubt * 1000) / 1000,
      reason: `Q=${Q} >= ${THRESHOLDS.HOWL} (exceptional threshold)`,
      action: 'Accept with high confidence',
    };
  }

  if (Q >= THRESHOLDS.WAG) {
    return {
      verdict: 'WAG',
      emoji: '🐕',
      reaction: '*wags steadily*',
      confidence: Math.round(confidence * 1000) / 1000,
      doubt: Math.round(doubt * 1000) / 1000,
      reason: `${THRESHOLDS.WAG} <= Q=${Q} < ${THRESHOLDS.HOWL}`,
      action: 'Accept with verification',
    };
  }

  if (Q >= THRESHOLDS.GROWL) {
    return {
      verdict: 'GROWL',
      emoji: '🐕‍🦺',
      reaction: '*low growl*',
      confidence: Math.round(confidence * 1000) / 1000,
      doubt: Math.round(doubt * 1000) / 1000,
      reason: `${THRESHOLDS.GROWL} <= Q=${Q} < ${THRESHOLDS.WAG}`,
      action: 'Transform before accepting - improve weak axioms',
    };
  }

  return {
    verdict: 'BARK',
    emoji: '🐶',
    reaction: '*barks warning*',
    confidence: Math.round(confidence * 1000) / 1000,
    doubt: Math.round(doubt * 1000) / 1000,
    reason: `Q=${Q} < ${THRESHOLDS.GROWL} (phi^-2 threshold)`,
    action: 'Do not accept - quality too low',
  };
}

// =============================================================================
// WEAKNESS ANALYSIS
// =============================================================================

/**
 * Identify weakest axiom(s) pulling Q-Score down
 *
 * @param {Object} qScoreResult - Result from calculateQScore
 * @returns {Object} Weakness analysis
 */
export function analyzeWeaknesses(qScoreResult) {
  const { breakdown, Q } = qScoreResult;

  // Sort axioms by score (ascending = weakest first)
  const sorted = Object.entries(breakdown).sort(([, a], [, b]) => a - b);

  const weakest = sorted[0];
  const strongest = sorted[sorted.length - 1];

  // Calculate how much the weakest axiom is pulling down the score
  const avgWithoutWeakest =
    (breakdown.PHI + breakdown.VERIFY + breakdown.CULTURE + breakdown.BURN - weakest[1]) / 3;

  const drag = avgWithoutWeakest - Q;

  return {
    weakestAxiom: weakest[0],
    weakestScore: weakest[1],
    strongestAxiom: strongest[0],
    strongestScore: strongest[1],
    imbalance: strongest[1] - weakest[1],
    drag: Math.round(drag * 10) / 10,
    recommendation:
      weakest[1] < THRESHOLDS.GROWL
        ? `Critical: ${weakest[0]} at ${weakest[1]} - focus improvement here`
        : weakest[1] < THRESHOLDS.WAG
          ? `Improve ${weakest[0]} (${weakest[1]}) to reach φ⁻¹ threshold`
          : 'All axioms above threshold - maintain balance',
  };
}

// =============================================================================
// COMBINED SCORE (K × Q)
// =============================================================================

/**
 * Calculate Final Score from K-Score and Q-Score
 *
 * Final = √(K × Q) - Geometric mean punishes imbalance
 *
 * @param {number} kScore - K-Score (0-100, token health)
 * @param {number} qScore - Q-Score (0-100, knowledge quality)
 * @returns {Object} Final score with breakdown
 */
export function calculateFinalScore(kScore, qScore) {
  // Ensure valid inputs
  const K = Math.max(0, Math.min(100, kScore || 0));
  const Q = Math.max(0, Math.min(100, qScore || 0));

  // Final = √(K × Q)
  const Final = Math.round(Math.sqrt(K * Q) * 10) / 10;

  // Determine which score is limiting
  const limiting = K < Q ? 'K-Score' : Q < K ? 'Q-Score' : 'balanced';

  return {
    Final,
    K,
    Q,
    formula: 'Final = √(K × Q)',
    limiting,
    verdict: getVerdict(Final),
    analysis: {
      balanced: Math.abs(K - Q) < 10,
      imbalance: Math.abs(K - Q),
      comment: K === Q
        ? 'Perfect balance between token health and knowledge quality'
        : K > Q
          ? `Knowledge quality (Q=${Q}) is limiting final score`
          : `Token health (K=${K}) is limiting final score`,
    },
  };
}

// =============================================================================
// COMPARISON DOCUMENTATION
// =============================================================================

/**
 * Show how Q-Score relates to K-Score philosophy
 */
export const COMPARISON = {
  'K-Score': {
    formula: 'K = 100 × ∛(D × O × L)',
    pillars: ['Diamond Hands (D)', 'Organic Growth (O)', 'Longevity (L)'],
    root: 3,
    purpose: 'Token quality measurement',
  },
  'Q-Score': {
    formula: 'Q = 100 × ∜(φ × V × C × B)',
    pillars: ['PHI (φ)', 'VERIFY (V)', 'CULTURE (C)', 'BURN (B)'],
    root: 4,
    purpose: 'Knowledge quality measurement',
  },
  'Final': {
    formula: 'Final = √(K × Q)',
    pillars: ['K-Score', 'Q-Score'],
    root: 2,
    purpose: 'Combined token + knowledge quality',
  },
  principle: 'Geometric mean prevents gaming - cannot compensate weak pillar with strong ones',
};

// =============================================================================
// DEFAULT EXPORT
// =============================================================================

export default {
  // Main functions
  calculateQScore,
  calculateQScoreFromAxioms,
  calculateAxiomScore,
  calculateFinalScore,
  getVerdict,
  analyzeWeaknesses,

  // Helpers
  geometricMean,
  nthRoot,

  // Documentation
  COMPARISON,

  // Constants
  DEFAULT_AXIOM_DIMENSIONS,
  NEUTRAL_SCORE,
};
