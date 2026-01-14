/**
 * Proof of Judgment - Judgment Creation
 *
 * Create and validate judgments for the chain
 *
 * @module @cynic/protocol/poj/judgment
 */

'use strict';

import { THRESHOLDS, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { sha256Prefixed, hashObject, randomHex } from '../crypto/hash.js';

/**
 * Verdict types (dog metaphors)
 */
export const Verdict = {
  HOWL: 'HOWL', // Exceptional (≥80)
  WAG: 'WAG', // Passes (≥50)
  GROWL: 'GROWL', // Needs work (≥38.2)
  BARK: 'BARK', // Critical (<38.2)
};

/**
 * Determine verdict from score
 * @param {number} score - Global score (0-100)
 * @returns {string} Verdict
 */
export function scoreToVerdict(score) {
  if (score >= THRESHOLDS.HOWL) return Verdict.HOWL;
  if (score >= THRESHOLDS.WAG) return Verdict.WAG;
  if (score >= THRESHOLDS.GROWL) return Verdict.GROWL;
  return Verdict.BARK;
}

/**
 * Generate judgment ID
 * @returns {string} Unique judgment ID
 */
export function generateJudgmentId() {
  return `jdg_${randomHex(16)}`;
}

/**
 * Create a judgment
 * @param {Object} params - Judgment parameters
 * @param {Object} params.item - Item being judged
 * @param {number} params.globalScore - Global score (0-100)
 * @param {Object} params.dimensions - Dimension scores
 * @param {number} [params.confidence] - Confidence level (max 61.8%)
 * @param {Object} [params.metadata] - Additional metadata
 * @returns {Object} Judgment object
 */
export function createJudgment({ item, globalScore, dimensions, confidence, metadata = {} }) {
  // Hash the item (never store raw content in chain)
  const itemHash = sha256Prefixed(hashObject(item));

  // Enforce φ confidence bounds
  const boundedConfidence = Math.min(confidence || globalScore / 100, PHI_INV);

  // Determine verdict
  const verdict = scoreToVerdict(globalScore);

  return {
    id: generateJudgmentId(),
    item_hash: itemHash,
    verdict,
    global_score: Math.round(globalScore * 10) / 10,
    confidence: Math.round(boundedConfidence * 1000) / 1000,
    dimensions,
    timestamp: Date.now(),
    ...metadata,
  };
}

/**
 * Validate judgment structure
 * @param {Object} judgment - Judgment to validate
 * @returns {{ valid: boolean, errors: string[] }} Validation result
 */
export function validateJudgment(judgment) {
  const errors = [];

  // Required fields
  const required = ['id', 'item_hash', 'verdict', 'global_score', 'confidence', 'dimensions'];
  for (const field of required) {
    if (judgment[field] === undefined) {
      errors.push(`Missing required field: ${field}`);
    }
  }

  // ID format
  if (judgment.id && !judgment.id.startsWith('jdg_')) {
    errors.push('Invalid judgment ID format');
  }

  // Item hash format
  if (judgment.item_hash && !judgment.item_hash.startsWith('sha256:')) {
    errors.push('Invalid item_hash format');
  }

  // Valid verdict
  if (judgment.verdict && !Object.values(Verdict).includes(judgment.verdict)) {
    errors.push(`Invalid verdict: ${judgment.verdict}`);
  }

  // Score bounds
  if (typeof judgment.global_score === 'number') {
    if (judgment.global_score < 0 || judgment.global_score > 100) {
      errors.push('global_score must be between 0 and 100');
    }
  }

  // Confidence bounds (φ-constrained)
  if (typeof judgment.confidence === 'number') {
    if (judgment.confidence < 0 || judgment.confidence > PHI_INV + 0.001) {
      errors.push(`confidence must be between 0 and ${PHI_INV} (φ⁻¹)`);
    }
  }

  // Dimensions must be object
  if (judgment.dimensions && typeof judgment.dimensions !== 'object') {
    errors.push('dimensions must be an object');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Calculate residual (unexplained variance)
 * @param {Object} judgment - Judgment with dimensions
 * @returns {number} Residual (0-1)
 */
export function calculateResidual(judgment) {
  const { dimensions, global_score } = judgment;

  if (!dimensions || Object.keys(dimensions).length === 0) {
    return 1; // 100% unexplained
  }

  // Simple average of dimensions
  const dimValues = Object.values(dimensions);
  const avgDim = dimValues.reduce((a, b) => a + b, 0) / dimValues.length;

  // Residual = how much the global score differs from dimension average
  const residual = Math.abs(global_score - avgDim) / 100;
  return Math.min(residual, 1);
}

/**
 * Check if judgment is anomalous (high residual)
 * @param {Object} judgment - Judgment to check
 * @returns {boolean} True if anomalous
 */
export function isAnomalous(judgment) {
  return calculateResidual(judgment) > PHI_INV_2; // > 38.2%
}

/**
 * Merge judgments from multiple sources
 * @param {Object[]} judgments - Judgments for same item
 * @returns {Object} Merged judgment
 */
export function mergeJudgments(judgments) {
  if (judgments.length === 0) {
    throw new Error('Cannot merge empty judgments');
  }

  if (judgments.length === 1) {
    return judgments[0];
  }

  // Weighted average by confidence
  let totalWeight = 0;
  let weightedScore = 0;
  const mergedDimensions = {};

  for (const j of judgments) {
    const weight = j.confidence || 0.5;
    totalWeight += weight;
    weightedScore += j.global_score * weight;

    for (const [dim, score] of Object.entries(j.dimensions || {})) {
      if (!mergedDimensions[dim]) {
        mergedDimensions[dim] = { sum: 0, weight: 0 };
      }
      mergedDimensions[dim].sum += score * weight;
      mergedDimensions[dim].weight += weight;
    }
  }

  const avgScore = weightedScore / totalWeight;
  const finalDimensions = {};

  for (const [dim, data] of Object.entries(mergedDimensions)) {
    finalDimensions[dim] = Math.round((data.sum / data.weight) * 10) / 10;
  }

  return createJudgment({
    item: { merged_from: judgments.map((j) => j.id) },
    globalScore: avgScore,
    dimensions: finalDimensions,
    confidence: Math.min(totalWeight / judgments.length, PHI_INV),
    metadata: {
      source_count: judgments.length,
      merged_at: Date.now(),
    },
  });
}

export default {
  Verdict,
  scoreToVerdict,
  generateJudgmentId,
  createJudgment,
  validateJudgment,
  calculateResidual,
  isAnomalous,
  mergeJudgments,
};
