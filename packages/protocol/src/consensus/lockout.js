/**
 * Exponential Lockout
 *
 * φ-based lockout for vote finality
 *
 * If you vote for block X at slot S:
 * - You cannot vote against X for φⁿ slots
 * - n = number of successive confirmations
 *
 * @module @cynic/protocol/consensus/lockout
 */

'use strict';

import { PHI } from '@cynic/core';

/**
 * Lockout state for a voter
 */
export class VoterLockout {
  /**
   * @param {string} voterId - Voter ID/public key
   */
  constructor(voterId) {
    this.voterId = voterId;
    this.votes = new Map(); // blockHash -> { slot, confirmations }
  }

  /**
   * Record a vote
   * @param {string} blockHash - Block voted for
   * @param {number} slot - Current slot
   * @returns {Object} Vote record
   */
  recordVote(blockHash, slot) {
    const existing = this.votes.get(blockHash);

    if (existing) {
      // Increment confirmations
      existing.confirmations++;
      existing.lastSlot = slot;
      return existing;
    }

    // New vote
    const record = {
      blockHash,
      initialSlot: slot,
      lastSlot: slot,
      confirmations: 1,
    };
    this.votes.set(blockHash, record);
    return record;
  }

  /**
   * Calculate lockout duration for block
   * @param {string} blockHash - Block hash
   * @returns {number} Lockout slots (φⁿ where n = confirmations)
   */
  getLockoutSlots(blockHash) {
    const record = this.votes.get(blockHash);
    if (!record) return 0;

    return Math.pow(PHI, record.confirmations);
  }

  /**
   * Check if voter is locked out from voting against block
   * @param {string} blockHash - Block hash
   * @param {number} currentSlot - Current slot
   * @returns {boolean} True if still locked out
   */
  isLockedOut(blockHash, currentSlot) {
    const record = this.votes.get(blockHash);
    if (!record) return false;

    const lockoutSlots = this.getLockoutSlots(blockHash);
    const slotsSinceVote = currentSlot - record.lastSlot;

    return slotsSinceVote < lockoutSlots;
  }

  /**
   * Check if voter can vote against all existing votes
   * @param {number} currentSlot - Current slot
   * @returns {Object} Lockout status
   */
  canSwitchVote(currentSlot) {
    const locked = [];
    const unlocked = [];

    for (const [blockHash, record] of this.votes) {
      if (this.isLockedOut(blockHash, currentSlot)) {
        locked.push({
          blockHash,
          confirmations: record.confirmations,
          lockoutRemaining: Math.ceil(
            this.getLockoutSlots(blockHash) - (currentSlot - record.lastSlot)
          ),
        });
      } else {
        unlocked.push({ blockHash, confirmations: record.confirmations });
      }
    }

    return {
      canSwitch: locked.length === 0,
      locked,
      unlocked,
    };
  }

  /**
   * Get all active votes
   * @returns {Object[]} Vote records
   */
  getActiveVotes() {
    return Array.from(this.votes.values());
  }

  /**
   * Clear old votes (pruning)
   * @param {number} currentSlot - Current slot
   * @param {number} maxAge - Maximum age in slots
   */
  pruneOldVotes(currentSlot, maxAge = 1000) {
    for (const [blockHash, record] of this.votes) {
      if (currentSlot - record.lastSlot > maxAge) {
        this.votes.delete(blockHash);
      }
    }
  }
}

/**
 * Lockout Manager - Tracks lockout for all voters
 */
export class LockoutManager {
  constructor() {
    this.voters = new Map();
  }

  /**
   * Get or create voter lockout
   * @param {string} voterId - Voter ID
   * @returns {VoterLockout} Voter lockout state
   */
  getVoter(voterId) {
    if (!this.voters.has(voterId)) {
      this.voters.set(voterId, new VoterLockout(voterId));
    }
    return this.voters.get(voterId);
  }

  /**
   * Record a vote
   * @param {string} voterId - Voter ID
   * @param {string} blockHash - Block voted for
   * @param {number} slot - Current slot
   * @returns {Object} Vote record
   */
  recordVote(voterId, blockHash, slot) {
    return this.getVoter(voterId).recordVote(blockHash, slot);
  }

  /**
   * Check if voter is locked out
   * @param {string} voterId - Voter ID
   * @param {string} blockHash - Block to check
   * @param {number} currentSlot - Current slot
   * @returns {boolean} True if locked out
   */
  isLockedOut(voterId, blockHash, currentSlot) {
    const voter = this.voters.get(voterId);
    if (!voter) return false;
    return voter.isLockedOut(blockHash, currentSlot);
  }

  /**
   * Check if voter can switch their vote
   * @param {string} voterId - Voter ID
   * @param {number} currentSlot - Current slot
   * @returns {Object} Switch status
   */
  canSwitchVote(voterId, currentSlot) {
    const voter = this.voters.get(voterId);
    if (!voter) return { canSwitch: true, locked: [], unlocked: [] };
    return voter.canSwitchVote(currentSlot);
  }

  /**
   * Get lockout statistics
   * @param {number} currentSlot - Current slot
   * @returns {Object} Lockout statistics
   */
  getStats(currentSlot) {
    let totalVoters = 0;
    let lockedVoters = 0;
    let totalConfirmations = 0;

    for (const voter of this.voters.values()) {
      totalVoters++;
      const status = voter.canSwitchVote(currentSlot);
      if (!status.canSwitch) {
        lockedVoters++;
      }
      totalConfirmations += voter.getActiveVotes().reduce(
        (sum, v) => sum + v.confirmations,
        0
      );
    }

    return {
      totalVoters,
      lockedVoters,
      freeVoters: totalVoters - lockedVoters,
      avgConfirmations: totalVoters > 0 ? totalConfirmations / totalVoters : 0,
    };
  }

  /**
   * Prune old data
   * @param {number} currentSlot - Current slot
   */
  prune(currentSlot) {
    for (const voter of this.voters.values()) {
      voter.pruneOldVotes(currentSlot);
    }
  }
}

/**
 * Calculate total lockout for given confirmations
 * Useful for estimating finality
 * @param {number} confirmations - Number of confirmations
 * @returns {number} Total lockout slots
 */
export function calculateTotalLockout(confirmations) {
  let total = 0;
  for (let i = 1; i <= confirmations; i++) {
    total += Math.pow(PHI, i);
  }
  return total;
}

/**
 * Estimate confirmations needed for target lockout
 * @param {number} targetSlots - Target lockout slots
 * @returns {number} Confirmations needed
 */
export function confirmationsForLockout(targetSlots) {
  let confirmations = 0;
  let total = 0;

  while (total < targetSlots) {
    confirmations++;
    total += Math.pow(PHI, confirmations);
  }

  return confirmations;
}

export default {
  VoterLockout,
  LockoutManager,
  calculateTotalLockout,
  confirmationsForLockout,
};
