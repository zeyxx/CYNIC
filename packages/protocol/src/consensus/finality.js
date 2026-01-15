/**
 * Finality Gadget
 *
 * Provides deterministic finality for φ-BFT consensus
 *
 * A block is final when:
 * 1. It has φ⁻¹ (61.8%) supermajority approval
 * 2. Sufficient confirmations have passed (lockout accumulated)
 * 3. No conflicting block can possibly achieve supermajority
 *
 * @module @cynic/protocol/consensus/finality
 */

'use strict';

import { PHI, PHI_INV, CONSENSUS_THRESHOLD } from '@cynic/core';
import { calculateTotalLockout, confirmationsForLockout } from './lockout.js';

/**
 * Finality status
 */
export const FinalityStatus = {
  PENDING: 'PENDING', // Not enough votes yet
  OPTIMISTIC: 'OPTIMISTIC', // Has supermajority but not enough confirmations
  PROBABILISTIC: 'PROBABILISTIC', // Very likely final (>99%)
  DETERMINISTIC: 'DETERMINISTIC', // Mathematically impossible to revert
};

/**
 * Calculate finality probability
 *
 * Based on:
 * - Current approve ratio
 * - Number of confirmations
 * - Lockout slots accumulated
 *
 * @param {Object} params - Finality parameters
 * @param {number} params.approveRatio - Approval ratio (0-1)
 * @param {number} params.confirmations - Number of confirmations
 * @param {number} params.totalValidators - Total validator count
 * @param {number} params.votedValidators - Validators who voted
 * @returns {Object} Finality assessment
 */
export function calculateFinalityProbability({
  approveRatio,
  confirmations,
  totalValidators,
  votedValidators,
}) {
  // Base probability from approval ratio
  // Supermajority (61.8%) gives ~95% base probability
  let probability = 0;

  if (approveRatio >= CONSENSUS_THRESHOLD) {
    // Excess over threshold contributes to probability
    const excess = approveRatio - CONSENSUS_THRESHOLD;
    probability = 0.95 + excess * 0.125; // Max ~100% at full approval
  } else {
    // Linear scale below threshold
    probability = approveRatio * 0.95 / CONSENSUS_THRESHOLD;
  }

  // Confirmation boost
  // Each confirmation increases probability using diminishing returns
  const confirmationBoost = 1 - Math.pow(PHI_INV, confirmations);
  probability = probability * (1 + confirmationBoost * 0.05);

  // Participation penalty
  // Lower participation = lower confidence
  const participationRatio = totalValidators > 0 ? votedValidators / totalValidators : 0;
  probability *= participationRatio;

  // Clamp to valid range
  probability = Math.min(Math.max(probability, 0), 1);

  // Determine status
  let status = FinalityStatus.PENDING;
  if (approveRatio >= CONSENSUS_THRESHOLD) {
    if (probability >= 0.9999) {
      status = FinalityStatus.DETERMINISTIC;
    } else if (probability >= 0.99) {
      status = FinalityStatus.PROBABILISTIC;
    } else {
      status = FinalityStatus.OPTIMISTIC;
    }
  }

  return {
    probability: Math.round(probability * 10000) / 10000,
    status,
    confirmations,
    lockoutSlots: calculateTotalLockout(confirmations),
    revertProbability: Math.round((1 - probability) * 10000) / 10000,
  };
}

/**
 * Estimate time to finality
 *
 * @param {Object} params - Parameters
 * @param {number} params.currentConfirmations - Current confirmations
 * @param {number} params.targetProbability - Target probability (e.g., 0.99)
 * @param {number} params.slotDurationMs - Slot duration in ms
 * @returns {Object} Time estimate
 */
export function estimateTimeToFinality({
  currentConfirmations,
  targetProbability = 0.99,
  slotDurationMs = 400,
}) {
  // Estimate confirmations needed
  // Higher probability requires more confirmations
  let targetConfirmations;

  if (targetProbability >= 0.9999) {
    targetConfirmations = 32; // Deterministic
  } else if (targetProbability >= 0.99) {
    targetConfirmations = 13; // Probabilistic (Fibonacci)
  } else if (targetProbability >= 0.95) {
    targetConfirmations = 8;
  } else {
    targetConfirmations = 5;
  }

  const remainingConfirmations = Math.max(0, targetConfirmations - currentConfirmations);
  const estimatedSlots = remainingConfirmations;
  const estimatedMs = estimatedSlots * slotDurationMs;

  return {
    targetConfirmations,
    currentConfirmations,
    remainingConfirmations,
    estimatedSlots,
    estimatedMs,
    estimatedSeconds: Math.round(estimatedMs / 1000),
  };
}

/**
 * Check if a fork is still possible
 *
 * A fork is possible if:
 * - There exists a validator set that could achieve supermajority for alternative
 * - Not all validators are locked into the current block
 *
 * @param {Object} params - Fork parameters
 * @param {number} params.lockedWeight - Weight locked into current block
 * @param {number} params.totalWeight - Total validator weight
 * @param {number} params.currentSlot - Current slot
 * @param {number} params.blockSlot - Block slot
 * @param {number} params.confirmations - Block confirmations
 * @returns {Object} Fork possibility assessment
 */
export function checkForkPossibility({
  lockedWeight,
  totalWeight,
  currentSlot,
  blockSlot,
  confirmations,
}) {
  // Calculate minimum weight needed for alternative supermajority
  const minForSupermajority = totalWeight * CONSENSUS_THRESHOLD;

  // Available weight = total - locked
  const availableWeight = totalWeight - lockedWeight;

  // Fork is possible if available weight >= minimum for supermajority
  const forkPossible = availableWeight >= minForSupermajority;

  // Calculate how much more lockout is needed to make fork impossible
  const weightGap = availableWeight - minForSupermajority;
  const additionalLockoutNeeded = weightGap > 0 ? weightGap : 0;

  // Estimate slots until fork is impossible
  const lockoutSlots = calculateTotalLockout(confirmations);
  const slotsSinceBlock = currentSlot - blockSlot;
  const remainingLockout = Math.max(0, lockoutSlots - slotsSinceBlock);

  return {
    forkPossible,
    lockedWeight,
    availableWeight,
    minForSupermajority,
    weightGap,
    additionalLockoutNeeded,
    remainingLockoutSlots: Math.ceil(remainingLockout),
    confirmationsNeeded: forkPossible
      ? confirmationsForLockout(lockoutSlots + additionalLockoutNeeded)
      : 0,
  };
}

/**
 * Finality Tracker
 *
 * Tracks finality status for multiple blocks
 */
export class FinalityTracker {
  constructor() {
    this.blocks = new Map(); // blockHash -> finality info
    this.finalizedBlocks = new Set();
  }

  /**
   * Update block finality status
   * @param {string} blockHash - Block hash
   * @param {Object} params - Update parameters
   */
  update(blockHash, params) {
    const existing = this.blocks.get(blockHash) || {
      blockHash,
      firstSeen: Date.now(),
      history: [],
    };

    const finality = calculateFinalityProbability(params);

    existing.current = finality;
    existing.lastUpdated = Date.now();

    // Enforce bounds before pushing (FIFO eviction)
    while (existing.history.length >= 100) {
      existing.history.shift();
    }
    existing.history.push({
      timestamp: Date.now(),
      ...finality,
    });

    this.blocks.set(blockHash, existing);

    // Track deterministic finality
    if (finality.status === FinalityStatus.DETERMINISTIC) {
      this.finalizedBlocks.add(blockHash);
    }

    return finality;
  }

  /**
   * Get block finality status
   * @param {string} blockHash - Block hash
   * @returns {Object|null} Finality info
   */
  get(blockHash) {
    return this.blocks.get(blockHash)?.current || null;
  }

  /**
   * Check if block is deterministically final
   * @param {string} blockHash - Block hash
   * @returns {boolean} True if final
   */
  isFinal(blockHash) {
    return this.finalizedBlocks.has(blockHash);
  }

  /**
   * Get finality statistics
   * @returns {Object} Statistics
   */
  getStats() {
    const blocks = Array.from(this.blocks.values());

    const byStatus = {
      [FinalityStatus.PENDING]: 0,
      [FinalityStatus.OPTIMISTIC]: 0,
      [FinalityStatus.PROBABILISTIC]: 0,
      [FinalityStatus.DETERMINISTIC]: 0,
    };

    for (const block of blocks) {
      if (block.current) {
        byStatus[block.current.status]++;
      }
    }

    return {
      total: blocks.length,
      finalized: this.finalizedBlocks.size,
      byStatus,
      avgProbability:
        blocks.length > 0
          ? blocks.reduce((sum, b) => sum + (b.current?.probability || 0), 0) / blocks.length
          : 0,
    };
  }

  /**
   * Prune old entries
   * @param {number} maxAge - Maximum age in ms
   */
  prune(maxAge = 3600000) {
    const now = Date.now();

    for (const [hash, info] of this.blocks) {
      if (now - info.lastUpdated > maxAge && !this.finalizedBlocks.has(hash)) {
        this.blocks.delete(hash);
      }
    }
  }
}

export default {
  FinalityStatus,
  calculateFinalityProbability,
  estimateTimeToFinality,
  checkForkPossibility,
  FinalityTracker,
};
