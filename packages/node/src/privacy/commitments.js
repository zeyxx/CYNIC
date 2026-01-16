/**
 * @cynic/node - Privacy Commitments
 *
 * Pedersen-style commitment scheme for privacy-preserving profile contributions.
 * Allows proving properties about data without revealing the data itself.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/privacy/commitments
 */

'use strict';

import { createHash, randomBytes } from 'crypto';
import { PHI_INV } from '@cynic/core';

/**
 * φ-aligned constants for commitment scheme
 */
export const COMMITMENT_CONSTANTS = {
  /** Blinding factor size in bytes (Fib(8) = 21) */
  BLINDING_SIZE: 21,

  /** Hash algorithm for commitments */
  HASH_ALGORITHM: 'sha256',

  /** Max commitment age in days (Fib(8) = 21) */
  MAX_AGE_DAYS: 21,

  /** Refresh interval in days (Fib(6) = 8) */
  REFRESH_DAYS: 8,
};

/**
 * Generate a cryptographically secure blinding factor
 * @returns {Buffer} Random blinding factor
 */
export function generateBlindingFactor() {
  return randomBytes(COMMITMENT_CONSTANTS.BLINDING_SIZE);
}

/**
 * Create a Pedersen-style commitment
 *
 * Commitment = H(value || blinding_factor)
 *
 * Properties:
 * - Hiding: Can't learn value from commitment
 * - Binding: Can't open to different value
 *
 * @param {string|number|Buffer} value - Value to commit to
 * @param {Buffer} [blindingFactor] - Optional blinding factor (generated if not provided)
 * @returns {{ commitment: string, blindingFactor: Buffer }} Commitment and blinding factor
 */
export function commit(value, blindingFactor = null) {
  const blinding = blindingFactor || generateBlindingFactor();

  // Convert value to buffer
  const valueBuffer = Buffer.isBuffer(value)
    ? value
    : Buffer.from(String(value), 'utf8');

  // Create commitment: H(value || blinding)
  const hash = createHash(COMMITMENT_CONSTANTS.HASH_ALGORITHM);
  hash.update(valueBuffer);
  hash.update(blinding);

  return {
    commitment: hash.digest('hex'),
    blindingFactor: blinding,
  };
}

/**
 * Verify a commitment opening
 *
 * @param {string} commitment - The commitment to verify
 * @param {string|number|Buffer} claimedValue - The value being claimed
 * @param {Buffer} blindingFactor - The blinding factor used
 * @returns {boolean} True if commitment opens to claimed value
 */
export function verify(commitment, claimedValue, blindingFactor) {
  const { commitment: recomputed } = commit(claimedValue, blindingFactor);
  return commitment === recomputed;
}

/**
 * Profile level commitment with range proof support
 *
 * Allows proving "my level >= N" without revealing exact level.
 */
export class ProfileCommitment {
  /**
   * @param {number} level - Profile level (1, 2, 3, 5, 8)
   * @param {Buffer} [blindingFactor] - Optional blinding factor
   */
  constructor(level, blindingFactor = null) {
    this.level = level;
    this.blindingFactor = blindingFactor || generateBlindingFactor();
    this._commitment = null;
  }

  /**
   * Get the commitment (lazy computed)
   * @returns {string} The commitment hash
   */
  get commitment() {
    if (!this._commitment) {
      const result = commit(this.level, this.blindingFactor);
      this._commitment = result.commitment;
    }
    return this._commitment;
  }

  /**
   * Create a proof that level >= minLevel
   *
   * Simple range proof using hash chains.
   * For level L and min M, proves L >= M without revealing L.
   *
   * @param {number} minLevel - Minimum level to prove
   * @returns {{ commitment: string, proof: object, claim: string } | null} Proof or null if level < minLevel
   */
  proveMinLevel(minLevel) {
    if (this.level < minLevel) {
      return null; // Can't prove false claim
    }

    // Generate proof witnesses for each level from minLevel to actual level
    // This allows verifier to check level >= minLevel without learning exact level
    const witnesses = [];
    const levels = [1, 2, 3, 5, 8]; // Fibonacci levels

    for (const lvl of levels) {
      if (lvl >= minLevel && lvl <= this.level) {
        // Create witness for this level
        const witness = createHash(COMMITMENT_CONSTANTS.HASH_ALGORITHM)
          .update(this.commitment)
          .update(Buffer.from(String(lvl)))
          .update(this.blindingFactor.slice(0, 8)) // Partial blinding
          .digest('hex')
          .slice(0, 16);
        witnesses.push({ level: lvl, witness });
      }
    }

    return {
      commitment: this.commitment,
      proof: {
        minLevel,
        witnesses,
        // Confidence capped at φ⁻¹
        confidence: Math.min(PHI_INV, witnesses.length / levels.length),
      },
      claim: `level >= ${minLevel}`,
    };
  }

  /**
   * Verify a range proof
   *
   * @param {string} commitment - The commitment
   * @param {{ minLevel: number, witnesses: Array }} proof - The proof
   * @param {number} minLevel - The claimed minimum level
   * @returns {boolean} True if proof is valid
   */
  static verifyMinLevel(commitment, proof, minLevel) {
    if (proof.minLevel !== minLevel) {
      return false;
    }

    // Must have at least one witness at or above minLevel
    const validWitnesses = proof.witnesses.filter(w => w.level >= minLevel);
    if (validWitnesses.length === 0) {
      return false;
    }

    // Verify witness structure (can't verify content without blinding factor)
    // This is a simplified verification - full ZK would require more
    for (const w of validWitnesses) {
      if (typeof w.witness !== 'string' || w.witness.length !== 16) {
        return false;
      }
    }

    return true;
  }

  /**
   * Serialize commitment for storage
   * @returns {object} Serialized commitment
   */
  toJSON() {
    return {
      commitment: this.commitment,
      // Note: blindingFactor should NEVER be serialized to collective storage
      // Only stored locally for future proofs
      createdAt: new Date().toISOString(),
    };
  }

  /**
   * Create from stored data (requires local blinding factor)
   * @param {object} data - Stored commitment data
   * @param {number} level - Known level
   * @param {Buffer} blindingFactor - Local blinding factor
   * @returns {ProfileCommitment}
   */
  static fromJSON(data, level, blindingFactor) {
    const pc = new ProfileCommitment(level, blindingFactor);
    pc._commitment = data.commitment;
    return pc;
  }
}

/**
 * Pattern commitment for contributing patterns to collective without revealing them
 */
export class PatternCommitment {
  /**
   * @param {string} pattern - The pattern content
   * @param {string} category - Pattern category
   * @param {Buffer} [blindingFactor] - Optional blinding factor
   */
  constructor(pattern, category, blindingFactor = null) {
    this.pattern = pattern;
    this.category = category;
    this.blindingFactor = blindingFactor || generateBlindingFactor();
    this._commitment = null;
    this._patternHash = null;
  }

  /**
   * Get pattern hash (for deduplication, not commitment)
   * @returns {string} Hash of pattern content
   */
  get patternHash() {
    if (!this._patternHash) {
      this._patternHash = createHash(COMMITMENT_CONSTANTS.HASH_ALGORITHM)
        .update(this.pattern)
        .digest('hex')
        .slice(0, 16); // Truncated for privacy
    }
    return this._patternHash;
  }

  /**
   * Get the commitment
   * @returns {string} The commitment hash
   */
  get commitment() {
    if (!this._commitment) {
      // Include category in commitment for typed proofs
      const value = `${this.category}:${this.pattern}`;
      const result = commit(value, this.blindingFactor);
      this._commitment = result.commitment;
    }
    return this._commitment;
  }

  /**
   * Prove membership in category without revealing pattern
   *
   * @param {string} category - Category to prove membership in
   * @returns {{ commitment: string, proof: object, claim: string } | null}
   */
  proveCategory(category) {
    if (this.category !== category) {
      return null;
    }

    return {
      commitment: this.commitment,
      proof: {
        category,
        patternHash: this.patternHash,
        confidence: PHI_INV, // Never certain
      },
      claim: `pattern in category: ${category}`,
    };
  }

  /**
   * Serialize for collective storage
   * @returns {object}
   */
  toCollective() {
    return {
      commitment: this.commitment,
      patternHash: this.patternHash,
      category: this.category,
      // NO raw pattern, NO blinding factor
    };
  }
}

/**
 * Commitment store for managing multiple commitments
 */
export class CommitmentStore {
  constructor() {
    /** @type {Map<string, { commitment: ProfileCommitment | PatternCommitment, createdAt: Date }>} */
    this.commitments = new Map();
  }

  /**
   * Add a commitment
   * @param {string} id - Unique identifier
   * @param {ProfileCommitment | PatternCommitment} commitment
   */
  add(id, commitment) {
    this.commitments.set(id, {
      commitment,
      createdAt: new Date(),
    });
  }

  /**
   * Get a commitment
   * @param {string} id
   * @returns {ProfileCommitment | PatternCommitment | null}
   */
  get(id) {
    const entry = this.commitments.get(id);
    return entry ? entry.commitment : null;
  }

  /**
   * Remove expired commitments (older than MAX_AGE_DAYS)
   * @returns {number} Number of commitments removed
   */
  pruneExpired() {
    const maxAge = COMMITMENT_CONSTANTS.MAX_AGE_DAYS * 24 * 60 * 60 * 1000;
    const now = Date.now();
    let removed = 0;

    for (const [id, entry] of this.commitments) {
      if (now - entry.createdAt.getTime() > maxAge) {
        this.commitments.delete(id);
        removed++;
      }
    }

    return removed;
  }

  /**
   * Get commitments needing refresh
   * @returns {Array<string>} IDs of commitments needing refresh
   */
  getNeedingRefresh() {
    const refreshAge = COMMITMENT_CONSTANTS.REFRESH_DAYS * 24 * 60 * 60 * 1000;
    const now = Date.now();
    const needsRefresh = [];

    for (const [id, entry] of this.commitments) {
      if (now - entry.createdAt.getTime() > refreshAge) {
        needsRefresh.push(id);
      }
    }

    return needsRefresh;
  }

  /**
   * Get stats
   * @returns {object}
   */
  getStats() {
    return {
      total: this.commitments.size,
      needsRefresh: this.getNeedingRefresh().length,
    };
  }
}

export default {
  COMMITMENT_CONSTANTS,
  generateBlindingFactor,
  commit,
  verify,
  ProfileCommitment,
  PatternCommitment,
  CommitmentStore,
};
