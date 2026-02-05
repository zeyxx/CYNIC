/**
 * CYNIC Judgment Entropy
 *
 * Shannon entropy for measuring judgment uncertainty.
 * "L'incertitude est information" - κυνικός
 *
 * H = -Σ p(x) · log₂ p(x)
 *
 * High entropy → uncertain judgment → trigger consensus
 * Low entropy → decisive judgment → higher confidence
 *
 * @module @cynic/node/judge/entropy
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger } from '@cynic/core';

const log = createLogger('Entropy');

// ═══════════════════════════════════════════════════════════════════════════
// φ-ALIGNED ENTROPY THRESHOLDS
// ═══════════════════════════════════════════════════════════════════════════

export const ENTROPY_THRESHOLDS = {
  // Normalized entropy [0, 1]
  DECISIVE: PHI_INV_3,      // < 23.6% = very decisive, clear signal
  MODERATE: PHI_INV_2,      // < 38.2% = moderately certain
  UNCERTAIN: PHI_INV,       // < 61.8% = acceptable uncertainty
  CHAOTIC: 1.0,             // > 61.8% = too uncertain, trigger consensus
};

// ═══════════════════════════════════════════════════════════════════════════
// CORE ENTROPY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate Shannon entropy of a probability distribution
 *
 * H = -Σ p(x) · log₂ p(x)
 *
 * @param {number[]} probabilities - Array of probabilities (must sum to 1)
 * @returns {number} Entropy in bits
 */
export function shannonEntropy(probabilities) {
  let H = 0;

  for (const p of probabilities) {
    if (p > 0 && p <= 1) {
      H -= p * Math.log2(p);
    }
  }

  return H;
}

/**
 * Calculate normalized entropy [0, 1]
 *
 * Normalized by maximum possible entropy (uniform distribution)
 * H_norm = H / log₂(n)
 *
 * @param {number[]} probabilities - Array of probabilities
 * @returns {number} Normalized entropy [0, 1]
 */
export function normalizedEntropy(probabilities) {
  const n = probabilities.length;
  if (n <= 1) return 0;

  const H = shannonEntropy(probabilities);
  const maxH = Math.log2(n);

  return maxH > 0 ? H / maxH : 0;
}

/**
 * Convert scores to probability distribution
 *
 * Uses softmax-like normalization to handle any score range
 *
 * @param {number[]} scores - Array of scores (any range)
 * @returns {number[]} Probability distribution (sums to 1)
 */
export function scoresToProbabilities(scores) {
  if (scores.length === 0) return [];

  // Handle all zeros
  const sum = scores.reduce((a, b) => a + Math.max(0, b), 0);
  if (sum === 0) {
    // Uniform distribution
    return scores.map(() => 1 / scores.length);
  }

  // Normalize to probabilities
  return scores.map(s => Math.max(0, s) / sum);
}

// ═══════════════════════════════════════════════════════════════════════════
// JUDGMENT ENTROPY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate entropy of axiom scores in a judgment
 *
 * Measures how "spread out" the axiom scores are.
 * - Uniform scores → high entropy → uncertain judgment
 * - Polarized scores → low entropy → decisive judgment
 *
 * @param {Object} axiomScores - Map of axiom → score (0-100)
 * @returns {Object} Entropy analysis
 */
export function judgmentEntropy(axiomScores) {
  const axioms = Object.keys(axiomScores);
  const scores = Object.values(axiomScores);

  if (scores.length === 0) {
    return {
      entropy: 0,
      normalizedEntropy: 0,
      category: 'DECISIVE',
      confidence: PHI_INV,
    };
  }

  // Convert to probabilities
  const probs = scoresToProbabilities(scores);

  // Calculate entropy
  const H = shannonEntropy(probs);
  const Hnorm = normalizedEntropy(probs);

  // Categorize
  let category;
  if (Hnorm < ENTROPY_THRESHOLDS.DECISIVE) {
    category = 'DECISIVE';
  } else if (Hnorm < ENTROPY_THRESHOLDS.MODERATE) {
    category = 'MODERATE';
  } else if (Hnorm < ENTROPY_THRESHOLDS.UNCERTAIN) {
    category = 'UNCERTAIN';
  } else {
    category = 'CHAOTIC';
  }

  // Adjust confidence based on entropy
  // High entropy → lower confidence
  // confidence = φ⁻¹ × (1 - H_norm × φ⁻²)
  const confidenceMultiplier = 1 - Hnorm * PHI_INV_2;
  const adjustedConfidence = PHI_INV * Math.max(PHI_INV_3, confidenceMultiplier);

  return {
    entropy: H,
    normalizedEntropy: Hnorm,
    maxEntropy: Math.log2(scores.length),
    category,
    confidence: adjustedConfidence,
    shouldTriggerConsensus: Hnorm > ENTROPY_THRESHOLDS.UNCERTAIN,
    axiomCount: axioms.length,
    probabilities: Object.fromEntries(axioms.map((a, i) => [a, probs[i]])),
  };
}

/**
 * Calculate information gain between two judgments
 *
 * Measures how much information was gained by refinement
 * IG = H(before) - H(after)
 *
 * @param {Object} beforeScores - Axiom scores before refinement
 * @param {Object} afterScores - Axiom scores after refinement
 * @returns {number} Information gain (positive = more certain)
 */
export function informationGain(beforeScores, afterScores) {
  const Hbefore = judgmentEntropy(beforeScores).normalizedEntropy;
  const Hafter = judgmentEntropy(afterScores).normalizedEntropy;

  return Hbefore - Hafter;
}

/**
 * Calculate cross-entropy between judgment and ground truth
 *
 * Used for learning: measures how well judgment predicted outcome
 * CE = -Σ q(x) · log₂ p(x)
 *
 * @param {Object} judgmentScores - Predicted axiom scores
 * @param {Object} truthScores - Actual/corrected scores
 * @returns {number} Cross-entropy (lower = better prediction)
 */
export function crossEntropy(judgmentScores, truthScores) {
  const axioms = Object.keys(judgmentScores);
  const pScores = axioms.map(a => judgmentScores[a] || 50);
  const qScores = axioms.map(a => truthScores[a] || 50);

  const p = scoresToProbabilities(pScores);
  const q = scoresToProbabilities(qScores);

  let CE = 0;
  for (let i = 0; i < p.length; i++) {
    if (q[i] > 0 && p[i] > 0) {
      CE -= q[i] * Math.log2(p[i]);
    }
  }

  return CE;
}

/**
 * Calculate KL divergence between judgment and truth
 *
 * D_KL(Q || P) = Σ q(x) · log₂(q(x) / p(x))
 *
 * @param {Object} judgmentScores - Predicted axiom scores
 * @param {Object} truthScores - Actual/corrected scores
 * @returns {number} KL divergence (0 = identical)
 */
export function klDivergence(judgmentScores, truthScores) {
  const axioms = Object.keys(judgmentScores);
  const pScores = axioms.map(a => judgmentScores[a] || 50);
  const qScores = axioms.map(a => truthScores[a] || 50);

  const p = scoresToProbabilities(pScores);
  const q = scoresToProbabilities(qScores);

  let DKL = 0;
  for (let i = 0; i < p.length; i++) {
    if (q[i] > 0 && p[i] > 0) {
      DKL += q[i] * Math.log2(q[i] / p[i]);
    }
  }

  return DKL;
}

// ═══════════════════════════════════════════════════════════════════════════
// ENTROPY-BASED CONFIDENCE ADJUSTMENT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Adjust confidence based on entropy
 *
 * @param {number} baseConfidence - Original confidence (0-1)
 * @param {number} normalizedEntropy - Normalized entropy (0-1)
 * @returns {number} Adjusted confidence (capped at φ⁻¹)
 */
export function adjustConfidenceByEntropy(baseConfidence, normalizedEntropy) {
  // Higher entropy → lower confidence
  // Factor: (1 - H × φ⁻²) ensures we never go below ~62% of base
  const factor = 1 - normalizedEntropy * PHI_INV_2;
  const adjusted = baseConfidence * Math.max(PHI_INV_3, factor);

  // Cap at φ⁻¹
  return Math.min(PHI_INV, adjusted);
}

/**
 * Calculate optimal confidence given entropy and base score
 *
 * Uses information-theoretic approach:
 * - More entropy = less information = lower confidence
 * - Confidence represents "bits of certainty"
 *
 * @param {number} qScore - Q-Score (0-100)
 * @param {Object} axiomScores - Axiom scores
 * @returns {number} Optimal confidence (0 to φ⁻¹)
 */
export function optimalConfidence(qScore, axiomScores) {
  const { normalizedEntropy, category } = judgmentEntropy(axiomScores);

  // Base confidence from Q-Score (normalized to 0-1)
  const baseConf = qScore / 100;

  // Entropy penalty
  const entropyPenalty = normalizedEntropy * PHI_INV_2;

  // Category bonus/penalty
  let categoryFactor = 1.0;
  switch (category) {
    case 'DECISIVE':
      categoryFactor = 1.0; // Full confidence
      break;
    case 'MODERATE':
      categoryFactor = PHI_INV; // 61.8%
      break;
    case 'UNCERTAIN':
      categoryFactor = PHI_INV_2; // 38.2%
      break;
    case 'CHAOTIC':
      categoryFactor = PHI_INV_3; // 23.6%
      break;
  }

  // Combine factors
  const optimal = baseConf * categoryFactor * (1 - entropyPenalty);

  // Cap at φ⁻¹
  return Math.min(PHI_INV, Math.max(0.01, optimal));
}

// ═══════════════════════════════════════════════════════════════════════════
// MUTUAL INFORMATION (for learning)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate mutual information between two judgment dimensions
 *
 * I(X;Y) = H(X) + H(Y) - H(X,Y)
 *
 * Used to find correlated axioms in judgments
 *
 * @param {number[]} scoresX - Scores for dimension X across judgments
 * @param {number[]} scoresY - Scores for dimension Y across judgments
 * @returns {number} Mutual information
 */
export function mutualInformation(scoresX, scoresY) {
  if (scoresX.length !== scoresY.length || scoresX.length === 0) {
    return 0;
  }

  const pX = scoresToProbabilities(scoresX);
  const pY = scoresToProbabilities(scoresY);

  const HX = shannonEntropy(pX);
  const HY = shannonEntropy(pY);

  // Joint distribution (simplified: assume independence for estimation)
  // In production, would compute actual joint distribution
  const pXY = [];
  for (let i = 0; i < pX.length; i++) {
    pXY.push(pX[i] * pY[i]);
  }
  const HXY = shannonEntropy(pXY);

  return Math.max(0, HX + HY - HXY);
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

export default {
  // Core
  shannonEntropy,
  normalizedEntropy,
  scoresToProbabilities,

  // Judgment
  judgmentEntropy,
  informationGain,
  crossEntropy,
  klDivergence,

  // Confidence
  adjustConfidenceByEntropy,
  optimalConfidence,

  // Learning
  mutualInformation,

  // Constants
  ENTROPY_THRESHOLDS,
};
