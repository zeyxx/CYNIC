/**
 * Residual Governance - Automatic dimension promotion governance
 *
 * "THE_UNNAMEABLE becomes named through collective wisdom"
 *
 * Uses CollectivePack voting to decide when candidates should become
 * real dimensions. Tracks outcomes for DPO learning.
 *
 * @module @cynic/node/judge/residual-governance
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { getPool } from '@cynic/persistence';

// Simple logger
const log = {
  debug: (mod, msg, data) => process.env.CYNIC_DEBUG && console.debug(`[${mod}]`, msg, data || ''),
  info: (mod, msg, data) => console.log(`[${mod}]`, msg, data || ''),
  warn: (mod, msg, data) => console.warn(`[${mod}]`, msg, data || ''),
  error: (mod, msg, data) => console.error(`[${mod}]`, msg, data || ''),
};

// ═══════════════════════════════════════════════════════════════════════════
// φ GOVERNANCE THRESHOLDS
// ═══════════════════════════════════════════════════════════════════════════

const GOVERNANCE_CONFIG = {
  // Minimum confidence for candidate to be considered
  minCandidateConfidence: PHI_INV_2,  // 38.2%

  // Minimum Dogs vote approval for promotion
  minVoteApproval: PHI_INV,  // 61.8%

  // Minimum sample count before governance review
  minSamplesForReview: 5,

  // Cooldown between reviews of same candidate (ms)
  reviewCooldownMs: 30 * 60 * 1000,  // 30 min

  // Max promotions per day (prevent runaway)
  maxDailyPromotions: 3,

  // φ-aligned age bonus (older candidates get slight boost)
  ageBoostFactor: PHI_INV_3,  // 23.6% max boost
  ageBoostMaxDays: 7,
};

/**
 * Residual Governance Service
 *
 * Manages automatic dimension promotion through collective voting.
 */
export class ResidualGovernance {
  constructor(options = {}) {
    this.pool = options.pool || getPool();
    this.residualDetector = options.residualDetector || null;
    this.collectivePack = options.collectivePack || null;
    this.dpoProcessor = options.dpoProcessor || null;
    this.residualStorage = options.residualStorage || null;

    // Governance state
    this._reviewHistory = new Map();  // candidateKey -> lastReviewAt
    this._dailyPromotions = 0;
    this._lastResetDay = new Date().toDateString();

    // Stats
    this.stats = {
      reviewsRun: 0,
      promotionsApproved: 0,
      promotionsRejected: 0,
      votingResults: [],
    };
  }

  /**
   * Set dependencies (for late binding)
   */
  setDependencies({ residualDetector, collectivePack, dpoProcessor, residualStorage }) {
    if (residualDetector) this.residualDetector = residualDetector;
    if (collectivePack) this.collectivePack = collectivePack;
    if (dpoProcessor) this.dpoProcessor = dpoProcessor;
    if (residualStorage) this.residualStorage = residualStorage;
  }

  /**
   * Run governance review on all candidates
   *
   * @returns {Promise<Object>} Review result
   */
  async reviewCandidates() {
    if (!this.residualDetector) {
      log.warn('ResidualGovernance', 'No ResidualDetector configured');
      return { reviewed: 0, promoted: 0, rejected: 0 };
    }

    // Reset daily counter if new day
    this._checkDailyReset();

    const candidates = this.residualDetector.getCandidates();
    const result = {
      reviewed: 0,
      promoted: 0,
      rejected: 0,
      skipped: 0,
      decisions: [],
    };

    log.info('ResidualGovernance', `Reviewing ${candidates.length} candidates`);

    for (const candidate of candidates) {
      // Check if we've hit daily limit
      if (this._dailyPromotions >= GOVERNANCE_CONFIG.maxDailyPromotions) {
        log.info('ResidualGovernance', 'Daily promotion limit reached');
        result.skipped++;
        continue;
      }

      // Check cooldown
      if (this._isOnCooldown(candidate.key)) {
        result.skipped++;
        continue;
      }

      // Check minimum requirements
      if (!this._meetsMinimumRequirements(candidate)) {
        result.skipped++;
        continue;
      }

      // Run governance decision
      const decision = await this._governCandidate(candidate);
      result.decisions.push(decision);
      result.reviewed++;

      if (decision.approved) {
        result.promoted++;
        this._dailyPromotions++;
      } else {
        result.rejected++;
      }

      // Record review timestamp
      this._reviewHistory.set(candidate.key, Date.now());
    }

    this.stats.reviewsRun++;
    log.info('ResidualGovernance', `Review complete`, result);

    return result;
  }

  /**
   * Govern a single candidate through voting
   *
   * @private
   * @param {Object} candidate - Candidate to govern
   * @returns {Promise<Object>} Decision result
   */
  async _governCandidate(candidate) {
    const decision = {
      candidateKey: candidate.key,
      suggestedName: candidate.suggestedName,
      suggestedAxiom: candidate.suggestedAxiom,
      confidence: candidate.confidence,
      sampleCount: candidate.sampleCount,
      avgResidual: candidate.avgResidual,
      approved: false,
      voteResult: null,
      reason: null,
      promotedAs: null,
    };

    // Calculate governance score with age boost
    const ageBoost = this._calculateAgeBoost(candidate);
    const governanceScore = Math.min(PHI_INV, candidate.confidence + ageBoost);

    log.debug('ResidualGovernance', `Governing ${candidate.key}`, {
      confidence: candidate.confidence,
      ageBoost,
      governanceScore,
    });

    // If CollectivePack available, use Dogs voting
    if (this.collectivePack) {
      try {
        decision.voteResult = await this._collectiveVote(candidate, governanceScore);
        decision.approved = decision.voteResult.approval >= GOVERNANCE_CONFIG.minVoteApproval;
        decision.reason = decision.voteResult.consensus;
      } catch (err) {
        log.error('ResidualGovernance', 'Voting failed', { error: err.message });
        decision.reason = `Voting failed: ${err.message}`;
      }
    } else {
      // Fallback: auto-approve if governance score > threshold
      decision.approved = governanceScore >= GOVERNANCE_CONFIG.minVoteApproval;
      decision.reason = decision.approved
        ? `Auto-approved: score ${(governanceScore * 100).toFixed(1)}% >= ${(GOVERNANCE_CONFIG.minVoteApproval * 100).toFixed(1)}%`
        : `Auto-rejected: score ${(governanceScore * 100).toFixed(1)}% < ${(GOVERNANCE_CONFIG.minVoteApproval * 100).toFixed(1)}%`;
    }

    // Execute decision
    if (decision.approved) {
      decision.promotedAs = await this._promoteCandidate(candidate);
      this.stats.promotionsApproved++;
    } else {
      // Don't reject immediately - just mark as reviewed
      // Candidates can be re-reviewed after cooldown with more samples
      this.stats.promotionsRejected++;
    }

    // Track voting result for learning
    this.stats.votingResults.push({
      timestamp: Date.now(),
      candidateKey: candidate.key,
      approved: decision.approved,
      voteApproval: decision.voteResult?.approval,
    });

    // Keep only last 100 voting results
    if (this.stats.votingResults.length > 100) {
      this.stats.votingResults.shift();
    }

    return decision;
  }

  /**
   * Run collective vote on candidate
   *
   * @private
   * @param {Object} candidate - Candidate to vote on
   * @param {number} governanceScore - Pre-calculated score
   * @returns {Promise<Object>} Vote result
   */
  async _collectiveVote(candidate, governanceScore) {
    // Format question for Dogs
    const question = {
      type: 'dimension_promotion',
      candidateKey: candidate.key,
      suggestedName: candidate.suggestedName,
      suggestedAxiom: candidate.suggestedAxiom,
      evidence: {
        sampleCount: candidate.sampleCount,
        avgResidual: candidate.avgResidual,
        weakDimensions: candidate.weakDimensions,
        governanceScore,
      },
    };

    // Ask collective for vote
    // Note: Using decide() which returns { decision, confidence, votes }
    const packDecision = await this.collectivePack.decide(
      `Promote dimension candidate '${candidate.suggestedName}' under ${candidate.suggestedAxiom}? ` +
      `Evidence: ${candidate.sampleCount} samples, ${(candidate.avgResidual * 100).toFixed(1)}% avg residual, ` +
      `weak in: ${candidate.weakDimensions.join(', ') || 'general'}`,
      {
        context: 'dimension_governance',
        metadata: question,
      }
    );

    // Calculate approval from votes
    const totalVotes = Object.values(packDecision.votes || {}).length;
    const approveVotes = Object.values(packDecision.votes || {})
      .filter(v => v.decision === 'approve' || v.score > 50).length;

    const approval = totalVotes > 0 ? approveVotes / totalVotes : 0;

    return {
      approval,
      totalVotes,
      approveVotes,
      consensus: packDecision.decision,
      confidence: packDecision.confidence,
      dogVotes: packDecision.votes,
    };
  }

  /**
   * Promote candidate to real dimension
   *
   * @private
   * @param {Object} candidate - Candidate to promote
   * @returns {Promise<Object>} Promoted dimension info
   */
  async _promoteCandidate(candidate) {
    // Generate proper name from suggested name or pattern
    const name = this._generateDimensionName(candidate);
    const axiom = candidate.suggestedAxiom;

    try {
      // Use ResidualDetector's acceptCandidate
      const discovery = this.residualDetector.acceptCandidate(candidate.key, {
        name,
        axiom,
        weight: 1.0,
        threshold: 50,
      });

      log.info('ResidualGovernance', `Promoted candidate to dimension: ${name}`, {
        axiom,
        fromCandidate: candidate.key,
      });

      // THE_UNNAMEABLE: Persist discovered dimension to PostgreSQL
      // Without this, discoveries die on restart
      if (this.residualStorage?.saveDiscoveredDimension) {
        await this.residualStorage.saveDiscoveredDimension({
          name,
          axiom,
          weight: 1.0,
          threshold: 50,
          description: `Discovered dimension (was ${candidate.suggestedName})`,
          promotedBy: this.collectivePack ? 'governance' : 'auto',
          fromCandidate: candidate.key,
          evidenceCount: candidate.sampleCount,
          avgResidual: candidate.avgResidual,
          weakDimensions: candidate.weakDimensions,
        });
        await this.residualStorage.markCandidatePromoted?.(candidate.key);
        await this.residualStorage.logGovernanceDecision?.({
          candidateKey: candidate.key,
          approved: true,
          promotedAs: { name, axiom },
          confidence: candidate.confidence,
        });
      }

      // R-GAP-1 FIX: Register dimension LIVE in globalDimensionRegistry
      // Without this, discoveries only take effect on next boot
      try {
        const { globalDimensionRegistry } = await import('./dimension-registry.js');
        globalDimensionRegistry.register(axiom, name, {
          weight: 1.0,
          threshold: 50,
          description: `Discovered dimension (was ${candidate.suggestedName})`,
        });
        log.info('ResidualGovernance', `Live-registered dimension ${axiom}.${name} in DimensionRegistry`);
      } catch (regErr) {
        log.debug('ResidualGovernance', 'Live registration failed (non-blocking)', { error: regErr.message });
      }

      // Record for DPO learning (this is a "chosen" outcome)
      await this._recordForDPO(candidate, discovery, true);

      return discovery;

    } catch (err) {
      log.error('ResidualGovernance', `Failed to promote ${candidate.key}`, { error: err.message });
      throw err;
    }
  }

  /**
   * Generate proper dimension name
   *
   * @private
   * @param {Object} candidate - Candidate
   * @returns {string} Dimension name
   */
  _generateDimensionName(candidate) {
    // If weak dimensions suggest a pattern, use that
    if (candidate.weakDimensions && candidate.weakDimensions.length > 0) {
      // Combine weak dimension hints into a name
      const hint = candidate.weakDimensions[0];
      if (hint && hint !== 'general') {
        return `${hint.toUpperCase()}_EXTENDED`;
      }
    }

    // Fallback: use suggested name with timestamp
    const timestamp = Date.now().toString(36).slice(-6);
    return `DISCOVERED_${timestamp}`.toUpperCase();
  }

  /**
   * Record governance decision for DPO learning
   *
   * @private
   * @param {Object} candidate - Candidate
   * @param {Object} discovery - Discovery result (if promoted)
   * @param {boolean} approved - Whether approved
   */
  async _recordForDPO(candidate, discovery, approved) {
    if (!this.dpoProcessor) return;

    try {
      // Create feedback-like record for DPO
      // The governance decision itself becomes training signal
      await this.pool.query(`
        INSERT INTO feedback (
          item_type, item_id, judgment_id,
          outcome, q_score, actual_score,
          source_type, reason
        ) VALUES (
          'dimension_governance',
          $1,
          NULL,
          $2,
          $3,
          $4,
          'governance',
          $5
        )
      `, [
        candidate.key,
        approved ? 'correct' : 'incorrect',
        Math.round(candidate.confidence * 100),
        approved ? 80 : 20,  // Actual score based on approval
        approved
          ? `Promoted as ${discovery?.name}`
          : 'Rejected by governance',
      ]);
    } catch (err) {
      log.warn('ResidualGovernance', 'Failed to record DPO feedback', { error: err.message });
    }
  }

  /**
   * Calculate age boost for candidate
   *
   * @private
   * @param {Object} candidate - Candidate
   * @returns {number} Age boost (0 to ageBoostFactor)
   */
  _calculateAgeBoost(candidate) {
    const ageMs = Date.now() - (candidate.detectedAt || Date.now());
    const ageDays = ageMs / (24 * 60 * 60 * 1000);

    // Linear boost up to max days
    const boost = Math.min(
      GOVERNANCE_CONFIG.ageBoostFactor,
      (ageDays / GOVERNANCE_CONFIG.ageBoostMaxDays) * GOVERNANCE_CONFIG.ageBoostFactor
    );

    return Math.max(0, boost);
  }

  /**
   * Check if candidate is on cooldown
   *
   * @private
   * @param {string} key - Candidate key
   * @returns {boolean} True if on cooldown
   */
  _isOnCooldown(key) {
    const lastReview = this._reviewHistory.get(key);
    if (!lastReview) return false;
    return Date.now() - lastReview < GOVERNANCE_CONFIG.reviewCooldownMs;
  }

  /**
   * Check if candidate meets minimum requirements
   *
   * @private
   * @param {Object} candidate - Candidate
   * @returns {boolean} True if meets requirements
   */
  _meetsMinimumRequirements(candidate) {
    return (
      candidate.confidence >= GOVERNANCE_CONFIG.minCandidateConfidence &&
      candidate.sampleCount >= GOVERNANCE_CONFIG.minSamplesForReview
    );
  }

  /**
   * Reset daily counter if new day
   *
   * @private
   */
  _checkDailyReset() {
    const today = new Date().toDateString();
    if (today !== this._lastResetDay) {
      this._dailyPromotions = 0;
      this._lastResetDay = today;
    }
  }

  /**
   * Provide feedback on a governance decision
   *
   * Called when human overrides or validates a decision
   *
   * @param {string} candidateKey - Candidate key
   * @param {boolean} wasCorrect - Whether the decision was correct
   * @param {string} [reason] - Reason for feedback
   */
  async provideFeedback(candidateKey, wasCorrect, reason = '') {
    try {
      await this.pool.query(`
        INSERT INTO feedback (
          item_type, item_id,
          outcome, source_type, reason
        ) VALUES (
          'dimension_governance',
          $1,
          $2,
          'human',
          $3
        )
      `, [
        candidateKey,
        wasCorrect ? 'correct' : 'incorrect',
        reason || (wasCorrect ? 'Human validated' : 'Human corrected'),
      ]);

      log.info('ResidualGovernance', `Feedback recorded for ${candidateKey}`, { wasCorrect });
    } catch (err) {
      log.error('ResidualGovernance', 'Failed to record feedback', { error: err.message });
    }
  }

  /**
   * Get governance statistics
   *
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      dailyPromotions: this._dailyPromotions,
      maxDailyPromotions: GOVERNANCE_CONFIG.maxDailyPromotions,
      reviewHistorySize: this._reviewHistory.size,
      config: GOVERNANCE_CONFIG,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create the ResidualGovernance singleton
 *
 * @param {Object} options - Governance options
 * @returns {ResidualGovernance} Singleton instance
 */
export function getResidualGovernance(options = {}) {
  if (!_instance) {
    _instance = new ResidualGovernance(options);
  }
  return _instance;
}

/**
 * Reset the singleton (for testing)
 */
export function resetResidualGovernance() {
  _instance = null;
}

export default {
  ResidualGovernance,
  getResidualGovernance,
  resetResidualGovernance,
  GOVERNANCE_CONFIG,
};
