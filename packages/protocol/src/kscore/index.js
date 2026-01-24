/**
 * K-Score Protocol Module
 *
 * Handles K-Score calculations, validations, and consensus integration
 *
 * K = 100 × ∛(D × O × L)
 *
 * Where:
 * - D (Diamond Hands): Conviction strength from holder behavior
 * - O (Organic Growth): Distribution quality (anti-sniper)
 * - L (Longevity): Survival factor over time
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/protocol/kscore
 */

'use strict';

import { PHI_INV, PHI_INV_2, secureNonce } from '@cynic/core';
import { sha256Prefixed, hashObject } from '../crypto/hash.js';

// K-Score thresholds (0-100 scale)
const KSCORE_HEALTHY_THRESHOLD = PHI_INV_2 * 100; // 38.2
const KSCORE_EXCEPTIONAL_THRESHOLD = PHI_INV * 100; // 61.8

/**
 * K-Score Transaction Types
 */
export const KScoreType = {
  REQUEST: 'KSCORE_REQUEST',
  RESULT: 'KSCORE_RESULT',
  DISPUTE: 'KSCORE_DISPUTE',
};

/**
 * K-Score Tiers
 */
export const KScoreTier = {
  DIAMOND: { min: 90, name: 'Diamond', verdict: 'HOWL' },
  PLATINUM: { min: 80, name: 'Platinum', verdict: 'HOWL' },
  GOLD: { min: 70, name: 'Gold', verdict: 'WAG' },
  SILVER: { min: 60, name: 'Silver', verdict: 'WAG' },
  BRONZE: { min: 50, name: 'Bronze', verdict: 'WAG' },
  IRON: { min: 38.2, name: 'Iron', verdict: 'GROWL' },
  STONE: { min: 0, name: 'Stone', verdict: 'BARK' },
};

/**
 * Calculate K-Score from components
 *
 * K = 100 × ∛(D × O × L)
 *
 * @param {Object} components - Score components
 * @param {number} components.D - Diamond Hands (0-1)
 * @param {number} components.O - Organic Growth (0-1)
 * @param {number} components.L - Longevity (0-1)
 * @returns {number} K-Score (0-100)
 */
export function calculateKScore(components) {
  const { D, O, L } = components;

  // Validate ranges
  if (D < 0 || D > 1 || O < 0 || O > 1 || L < 0 || L > 1) {
    throw new Error('Components must be in range [0, 1]');
  }

  // K = 100 × ∛(D × O × L)
  return 100 * Math.cbrt(D * O * L);
}

/**
 * Get K-Score tier
 * @param {number} score - K-Score value
 * @returns {Object} Tier info
 */
export function getKScoreTier(score) {
  for (const [key, tier] of Object.entries(KScoreTier)) {
    if (score >= tier.min) {
      return { key, ...tier };
    }
  }
  return { key: 'STONE', ...KScoreTier.STONE };
}

/**
 * Create K-Score request transaction
 * @param {Object} params - Request parameters
 * @param {string} params.mint - Token mint address
 * @param {Object} params.components - K-Score components {D, O, L}
 * @param {string} params.requestor - Requestor ID (HolDex worker, etc.)
 * @param {Object} [params.metadata] - Additional metadata
 * @returns {Object} K-Score request transaction
 */
export function createKScoreRequest({
  mint,
  components,
  requestor,
  metadata = {},
}) {
  if (!mint) throw new Error('Missing required field: mint');
  if (!components ||
      typeof components.D !== 'number' ||
      typeof components.O !== 'number' ||
      typeof components.L !== 'number') {
    throw new Error('Invalid components: {D, O, L} required as numbers');
  }
  if (!requestor) throw new Error('Missing required field: requestor');

  // Pre-calculate for validation
  const calculatedScore = calculateKScore(components);

  const request = {
    type: KScoreType.REQUEST,
    mint,
    components: {
      D: parseFloat(components.D.toFixed(6)),
      O: parseFloat(components.O.toFixed(6)),
      L: parseFloat(components.L.toFixed(6)),
    },
    calculatedScore: parseFloat(calculatedScore.toFixed(4)),
    requestor,
    metadata,
    timestamp: Date.now(),
    nonce: secureNonce(8),
  };

  // Add request ID based on hash
  request.requestId = `ks_${sha256Prefixed(hashObject(request)).slice(0, 16)}`;

  return request;
}

/**
 * Create K-Score result transaction
 * @param {Object} request - Original K-Score request
 * @param {Object} params - Result parameters
 * @param {number} params.consensusScore - Agreed K-Score from validators
 * @param {number} params.validatorCount - Number of validators that agreed
 * @param {number} params.height - Block height when finalized
 * @param {string} params.merkleProof - Merkle proof of inclusion
 * @returns {Object} K-Score result transaction
 */
export function createKScoreResult(request, {
  consensusScore,
  validatorCount,
  height,
  merkleProof,
}) {
  const tier = getKScoreTier(consensusScore);

  return {
    type: KScoreType.RESULT,
    requestId: request.requestId,
    mint: request.mint,
    components: request.components,
    requestedScore: request.calculatedScore,
    consensusScore: parseFloat(consensusScore.toFixed(4)),
    deviation: Math.abs(request.calculatedScore - consensusScore),
    tier: tier.key,
    tierName: tier.name,
    verdict: tier.verdict,
    consensus: {
      validatorCount,
      threshold: PHI_INV,
      reached: true,
    },
    block: {
      height,
      merkleProof,
    },
    timestamp: Date.now(),
  };
}

/**
 * Validate K-Score request
 * @param {Object} request - K-Score request to validate
 * @returns {{ valid: boolean, errors: string[] }} Validation result
 */
export function validateKScoreRequest(request) {
  const errors = [];

  // Required fields
  if (!request.type || request.type !== KScoreType.REQUEST) {
    errors.push('Invalid request type');
  }

  if (!request.mint || typeof request.mint !== 'string') {
    errors.push('Invalid or missing mint address');
  }

  // Validate components
  if (!request.components) {
    errors.push('Missing components');
  } else {
    const { D, O, L } = request.components;

    if (typeof D !== 'number' || D < 0 || D > 1) {
      errors.push('Invalid D component: must be number in [0, 1]');
    }
    if (typeof O !== 'number' || O < 0 || O > 1) {
      errors.push('Invalid O component: must be number in [0, 1]');
    }
    if (typeof L !== 'number' || L < 0 || L > 1) {
      errors.push('Invalid L component: must be number in [0, 1]');
    }

    // Verify calculated score
    if (errors.length === 0 && request.calculatedScore) {
      const expectedScore = calculateKScore(request.components);
      if (Math.abs(request.calculatedScore - expectedScore) > 0.01) {
        errors.push(`Calculated score mismatch: expected ${expectedScore.toFixed(4)}, got ${request.calculatedScore}`);
      }
    }
  }

  if (!request.requestor) {
    errors.push('Missing requestor');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Check if K-Score is healthy
 * @param {number} score - K-Score value (0-100)
 * @returns {boolean} True if score >= φ⁻² × 100 (38.2)
 */
export function isHealthyKScore(score) {
  return score >= KSCORE_HEALTHY_THRESHOLD;
}

/**
 * Check if K-Score is exceptional
 * @param {number} score - K-Score value (0-100)
 * @returns {boolean} True if score >= φ⁻¹ × 100 (61.8)
 */
export function isExceptionalKScore(score) {
  return score >= KSCORE_EXCEPTIONAL_THRESHOLD;
}

/**
 * Calculate K-Score deviation from validator submissions
 * @param {number[]} scores - Array of validator-submitted scores
 * @returns {{ mean: number, deviation: number, consensus: boolean }} Consensus result
 */
export function calculateKScoreConsensus(scores) {
  if (!scores || scores.length === 0) {
    return { mean: 0, deviation: 0, consensus: false };
  }

  const sum = scores.reduce((a, b) => a + b, 0);
  const mean = sum / scores.length;

  // Standard deviation
  const squaredDiffs = scores.map(s => Math.pow(s - mean, 2));
  const variance = squaredDiffs.reduce((a, b) => a + b, 0) / scores.length;
  const deviation = Math.sqrt(variance);

  // Consensus reached if deviation < φ⁻² of mean
  const consensus = deviation < (mean * PHI_INV_2);

  return {
    mean: parseFloat(mean.toFixed(4)),
    deviation: parseFloat(deviation.toFixed(4)),
    consensus,
    agreementRatio: deviation > 0 ? 1 - (deviation / mean) : 1,
  };
}

/**
 * Get K-Score confidence based on validator agreement
 * @param {number} agreementRatio - Agreement ratio from consensus
 * @returns {number} Confidence (capped at φ⁻¹)
 */
export function getKScoreConfidence(agreementRatio) {
  // CYNIC never exceeds φ⁻¹ confidence
  return Math.min(agreementRatio, PHI_INV);
}

export default {
  KScoreType,
  KScoreTier,
  calculateKScore,
  getKScoreTier,
  createKScoreRequest,
  createKScoreResult,
  validateKScoreRequest,
  isHealthyKScore,
  isExceptionalKScore,
  calculateKScoreConsensus,
  getKScoreConfidence,
};
