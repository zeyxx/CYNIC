/**
 * @cynic/node - Privacy Aggregator
 *
 * Transforms local data → committed (hashes) → public (noised).
 * The key to collective learning without individual exposure.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/privacy/aggregator
 */

'use strict';

import { createHash } from 'crypto';
import { PHI, PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  commit,
  verify,
  ProfileCommitment,
  PatternCommitment,
} from './commitments.js';
import {
  DifferentialPrivacy,
  PrivatePatternAggregator,
  PRIVACY_CONSTANTS,
} from './differential.js';
import { LocalStore, LocalDataCategory } from './local-store.js';

/**
 * φ-aligned constants for aggregation
 */
export const AGGREGATOR_CONSTANTS = {
  /** Golden angle for bucket distribution (degrees) */
  GOLDEN_ANGLE: 137.5077640500378,

  /** Number of pattern buckets (Fib(9) = 34) */
  PATTERN_BUCKETS: 34,

  /** Minimum contributions for public release (Fib(5) = 5) */
  MIN_CONTRIBUTIONS_FOR_PUBLIC: 5,

  /** Batch size for aggregation (Fib(8) = 21) */
  AGGREGATION_BATCH_SIZE: 21,

  /** Contribution weight decay factor (φ⁻¹) */
  WEIGHT_DECAY: PHI_INV,

  /** Minimum bucket count for pattern to be included (Fib(3) = 2) */
  MIN_BUCKET_COUNT: 2,
};

/**
 * Data tiers for privacy classification
 */
export const DataTier = {
  /** Device-only, never shared */
  LOCAL: 'local',

  /** Hash/commitment only, no raw data */
  COMMITTED: 'committed',

  /** Noised aggregates, safe to share globally */
  PUBLIC: 'public',
};

/**
 * Hash content using golden angle distribution for even bucketing
 *
 * @param {string} content - Content to hash
 * @returns {{ hash: string, bucket: number }} Hash and bucket assignment
 */
export function goldenAngleHash(content) {
  const hash = createHash('sha256').update(content).digest();

  // Use first 4 bytes for angle calculation
  const angle = (hash.readUInt32BE(0) % 360000) / 1000; // 0-359.999

  // Apply golden angle for even distribution
  const rotated = (angle + AGGREGATOR_CONSTANTS.GOLDEN_ANGLE) % 360;
  const bucket = Math.floor((rotated / 360) * AGGREGATOR_CONSTANTS.PATTERN_BUCKETS);

  return {
    hash: hash.toString('hex').slice(0, 32),
    bucket,
  };
}

/**
 * Contribution metadata (never includes raw data)
 */
export class ContributionMeta {
  /**
   * @param {string} category - Pattern category
   * @param {number} bucket - Bucket assignment (0-33)
   * @param {number} [weight=1] - Contribution weight
   */
  constructor(category, bucket, weight = 1) {
    this.category = category;
    this.bucket = bucket;
    this.weight = Math.min(1, Math.max(0, weight));
    this.timestamp = Date.now();
  }
}

/**
 * Privacy-preserving aggregator
 *
 * Handles the transformation:
 *   LOCAL (raw) → COMMITTED (hash) → PUBLIC (noised aggregate)
 *
 * CRITICAL: Never reverses the flow. Public data cannot identify individuals.
 */
export class PrivacyAggregator {
  constructor(options = {}) {
    this.dp = new DifferentialPrivacy({
      epsilon: options.epsilon ?? PRIVACY_CONSTANTS.EPSILON,
    });

    this.patternAggregator = new PrivatePatternAggregator({
      epsilon: options.epsilon,
    });

    /** @type {Map<string, ProfileCommitment>} User ID → Profile commitment */
    this.profileCommitments = new Map();

    /** @type {Map<string, PatternCommitment[]>} Category → Pattern commitments */
    this.patternCommitments = new Map();

    /** @type {Map<number, ContributionMeta[]>} Bucket → Contributions */
    this.bucketContributions = new Map();

    /** Statistics (aggregated only, no individual data) */
    this.stats = {
      totalContributions: 0,
      categoryCounts: new Map(),
      bucketUtilization: new Array(AGGREGATOR_CONSTANTS.PATTERN_BUCKETS).fill(0),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TIER 1: LOCAL → COMMITTED (Create Commitments)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Commit a user's profile level
   * Returns commitment that can be shared; blinding factor stays local.
   *
   * @param {string} userId - User identifier (stays local after this call)
   * @param {number} level - Profile level (1, 2, 3, 5, 8)
   * @returns {{ commitment: string, blindingFactor: Buffer }} Commitment data
   */
  commitProfile(userId, level) {
    const profileCommitment = new ProfileCommitment(level);

    // Store for later proofs (the commitment, not the level!)
    this.profileCommitments.set(userId, profileCommitment);

    return {
      commitment: profileCommitment.commitment,
      blindingFactor: profileCommitment.blindingFactor,
      // WARNING: blindingFactor must stay LOCAL
    };
  }

  /**
   * Create a range proof for profile level
   * Proves "my level >= minLevel" without revealing actual level.
   *
   * @param {string} userId - User identifier
   * @param {number} minLevel - Minimum level to prove
   * @returns {object | null} Proof or null if level < minLevel
   */
  proveMinProfileLevel(userId, minLevel) {
    const commitment = this.profileCommitments.get(userId);
    if (!commitment) return null;

    return commitment.proveMinLevel(minLevel);
  }

  /**
   * Commit a pattern for collective learning
   * Pattern content stays local; only hash goes to collective.
   *
   * @param {string} pattern - Pattern content (STAYS LOCAL)
   * @param {string} category - Pattern category
   * @returns {{ commitment: string, patternHash: string, bucket: number }}
   */
  commitPattern(pattern, category) {
    const patternCommitment = new PatternCommitment(pattern, category);

    // Calculate bucket using golden angle distribution
    const { bucket } = goldenAngleHash(pattern);

    // Store commitment (not raw pattern!)
    if (!this.patternCommitments.has(category)) {
      this.patternCommitments.set(category, []);
    }
    this.patternCommitments.get(category).push(patternCommitment);

    // Add to bucket contributions
    if (!this.bucketContributions.has(bucket)) {
      this.bucketContributions.set(bucket, []);
    }
    this.bucketContributions.get(bucket).push(
      new ContributionMeta(category, bucket)
    );

    // Update stats (aggregate only)
    this.stats.totalContributions++;
    this.stats.bucketUtilization[bucket]++;
    const catCount = this.stats.categoryCounts.get(category) || 0;
    this.stats.categoryCounts.set(category, catCount + 1);

    // Also add to differential privacy aggregator
    this.patternAggregator.addPattern(patternCommitment.patternHash, category);

    return {
      commitment: patternCommitment.commitment,
      patternHash: patternCommitment.patternHash,
      bucket,
      // WARNING: raw pattern content is NOT returned
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TIER 2: COMMITTED → PUBLIC (Generate Noised Aggregates)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get public (noised) pattern statistics
   * Safe to share globally - individual contributions are protected.
   *
   * @returns {object} Privacy-preserved statistics
   */
  getPublicPatternStats() {
    // Check if we have enough contributions for anonymity
    if (this.stats.totalContributions < AGGREGATOR_CONSTANTS.MIN_CONTRIBUTIONS_FOR_PUBLIC) {
      return {
        status: 'insufficient_data',
        message: `Need ${AGGREGATOR_CONSTANTS.MIN_CONTRIBUTIONS_FOR_PUBLIC} contributions for public release`,
        currentContributions: this.stats.totalContributions,
      };
    }

    // Get private (noised) stats from differential privacy aggregator
    const privateStats = this.patternAggregator.getPrivateStats();

    // Add noised bucket distribution
    const noisedBuckets = this.stats.bucketUtilization.map(count =>
      this.dp.addNoiseToCount(count)
    );

    // Add noised category distribution
    const noisedCategories = {};
    for (const [category, count] of this.stats.categoryCounts) {
      noisedCategories[category] = this.dp.addNoiseToCount(count);
    }

    return {
      status: 'public',
      tier: DataTier.PUBLIC,
      totalPatterns: privateStats.totalPatterns,
      uniquePatterns: privateStats.uniquePatterns,
      bucketDistribution: noisedBuckets,
      categoryDistribution: noisedCategories,
      budgetStatus: privateStats.budgetStatus,
      phi: {
        epsilon: PRIVACY_CONSTANTS.EPSILON,
        maxConfidence: PHI_INV,
        note: 'All values include Laplacian noise for differential privacy',
      },
    };
  }

  /**
   * Get top patterns (by noised count)
   *
   * @param {number} [limit=8] - Max patterns (Fib(6))
   * @param {string} [category] - Filter by category
   * @returns {Array<{ hash: string, category: string, count: number }>}
   */
  getTopPatterns(limit = 8, category = null) {
    return this.patternAggregator.getTopPatterns(limit, category);
  }

  /**
   * Get category distribution with privacy
   *
   * @returns {Map<string, number>} Category → noised count
   */
  getCategoryDistribution() {
    return this.patternAggregator.getCategoryDistribution();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // VERIFICATION (Without Learning Individual Data)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Verify a profile range proof
   * Confirms "user has level >= N" without learning actual level.
   *
   * @param {string} commitment - Profile commitment
   * @param {object} proof - Range proof
   * @param {number} minLevel - Claimed minimum level
   * @returns {boolean} True if proof is valid
   */
  verifyProfileLevel(commitment, proof, minLevel) {
    return ProfileCommitment.verifyMinLevel(commitment, proof, minLevel);
  }

  /**
   * Verify a pattern category proof
   *
   * @param {string} commitment - Pattern commitment
   * @param {object} proof - Category proof
   * @param {string} category - Claimed category
   * @returns {boolean} True if proof is valid
   */
  verifyPatternCategory(commitment, proof, category) {
    // Basic structural verification
    if (!proof || proof.category !== category) {
      return false;
    }

    // Verify pattern hash format
    if (typeof proof.patternHash !== 'string' || proof.patternHash.length !== 16) {
      return false;
    }

    // Note: Full verification would require the blinding factor (which stays local)
    // This is intentional - we can only verify structure, not content
    return true;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // AGGREGATION UTILITIES
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get bucket distribution evenness score
   * Uses golden angle, so should approach 1.0 with enough data.
   *
   * @returns {number} Evenness score (0-1, higher is more even)
   */
  getBucketEvenness() {
    const buckets = this.stats.bucketUtilization;
    const total = buckets.reduce((a, b) => a + b, 0);

    if (total === 0) return 1;

    const expected = total / buckets.length;
    const variance = buckets.reduce((acc, b) => acc + Math.pow(b - expected, 2), 0) / buckets.length;
    const stdDev = Math.sqrt(variance);

    // Normalize: 0 stdDev = perfect evenness (1.0)
    const maxStdDev = expected; // Theoretical max for very uneven
    return Math.max(0, 1 - (stdDev / maxStdDev));
  }

  /**
   * Get aggregator status
   *
   * @returns {object}
   */
  getStatus() {
    return {
      totalContributions: this.stats.totalContributions,
      profileCommitments: this.profileCommitments.size,
      patternCategories: this.patternCommitments.size,
      bucketEvenness: this.getBucketEvenness(),
      privacyBudget: this.dp.getBudgetStatus(),
      canReleasePublic: this.stats.totalContributions >= AGGREGATOR_CONSTANTS.MIN_CONTRIBUTIONS_FOR_PUBLIC,
      phi: {
        goldenAngle: AGGREGATOR_CONSTANTS.GOLDEN_ANGLE,
        epsilon: PRIVACY_CONSTANTS.EPSILON,
        maxConfidence: PHI_INV,
      },
    };
  }

  /**
   * Reset privacy budget (for new period)
   * NOTE: Does not reset contributions, only the DP budget.
   */
  resetPrivacyBudget() {
    this.dp = new DifferentialPrivacy({
      epsilon: PRIVACY_CONSTANTS.EPSILON,
    });
  }
}

/**
 * Batch aggregator for processing multiple contributions
 */
export class BatchAggregator {
  /**
   * @param {PrivacyAggregator} aggregator - Main aggregator
   */
  constructor(aggregator) {
    this.aggregator = aggregator;
    this.pendingPatterns = [];
    this.batchSize = AGGREGATOR_CONSTANTS.AGGREGATION_BATCH_SIZE;
  }

  /**
   * Queue a pattern for batch processing
   *
   * @param {string} pattern - Pattern content
   * @param {string} category - Pattern category
   */
  queuePattern(pattern, category) {
    this.pendingPatterns.push({ pattern, category });

    if (this.pendingPatterns.length >= this.batchSize) {
      this.flush();
    }
  }

  /**
   * Process all pending patterns
   *
   * @returns {{ processed: number, results: Array }}
   */
  flush() {
    const results = [];

    for (const { pattern, category } of this.pendingPatterns) {
      try {
        const result = this.aggregator.commitPattern(pattern, category);
        results.push({ success: true, ...result });
      } catch (error) {
        results.push({ success: false, error: error.message });
      }
    }

    const processed = this.pendingPatterns.length;
    this.pendingPatterns = [];

    return { processed, results };
  }

  /**
   * Get pending count
   *
   * @returns {number}
   */
  getPendingCount() {
    return this.pendingPatterns.length;
  }
}

/**
 * Creates a complete privacy pipeline
 *
 * @param {LocalStore} localStore - Device-local storage
 * @param {object} [options] - Configuration options
 * @returns {{ local: LocalStore, aggregator: PrivacyAggregator, batch: BatchAggregator }}
 */
export function createPrivacyPipeline(localStore, options = {}) {
  const aggregator = new PrivacyAggregator(options);
  const batch = new BatchAggregator(aggregator);

  return {
    local: localStore,
    aggregator,
    batch,

    /**
     * Process local data into committed form
     * Extracts patterns from local store without exposing raw data.
     *
     * @param {string} category - Data category to process
     * @returns {{ committed: number, skipped: number }}
     */
    processLocalToCommitted(category) {
      const entries = localStore.getByCategory(category);
      let committed = 0;
      let skipped = 0;

      for (const entry of entries) {
        try {
          // Extract pattern from entry data
          const pattern = this._extractPattern(entry);
          if (pattern) {
            batch.queuePattern(pattern, category);
            committed++;
          } else {
            skipped++;
          }
        } catch {
          skipped++;
        }
      }

      // Flush remaining
      batch.flush();

      return { committed, skipped };
    },

    /**
     * Extract pattern from entry (override for custom logic)
     * @private
     */
    _extractPattern(entry) {
      // Default: use stringified data as pattern
      if (entry.data && typeof entry.data === 'object') {
        return JSON.stringify(entry.data);
      }
      return String(entry.data);
    },
  };
}

export default {
  AGGREGATOR_CONSTANTS,
  DataTier,
  goldenAngleHash,
  ContributionMeta,
  PrivacyAggregator,
  BatchAggregator,
  createPrivacyPipeline,
};
