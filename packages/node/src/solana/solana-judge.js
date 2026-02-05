/**
 * CYNIC Solana Judge - C2.2 (SOLANA × JUDGE)
 *
 * Judges Solana blockchain data using CYNIC's 25-dimension system.
 * Part of the 7×7 Fractal Matrix judgment layer.
 *
 * "On-chain truth, φ-capped confidence" - κυνικός
 *
 * Judges:
 * - Transaction quality (signature, fee efficiency)
 * - Program security (verified, audit status)
 * - Account health (balance, rent, activity)
 * - Network state (TPS, slot lag)
 *
 * @module @cynic/node/solana/solana-judge
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('SolanaJudge');

/**
 * Solana judgment types
 */
export const SolanaJudgmentType = {
  TRANSACTION: 'transaction',
  ACCOUNT: 'account',
  PROGRAM: 'program',
  NETWORK: 'network',
  TOKEN: 'token',
};

/**
 * Solana-specific dimensions (subset aligned with CYNIC 25D)
 */
const SOLANA_DIMENSIONS = {
  // VERIFY axiom
  signature_valid: { weight: 1.0, axiom: 'verify' },
  rent_exempt: { weight: 0.8, axiom: 'verify' },
  program_verified: { weight: 0.9, axiom: 'verify' },

  // PHI axiom (efficiency, ratios)
  fee_efficiency: { weight: 0.7, axiom: 'phi' },
  compute_efficiency: { weight: 0.6, axiom: 'phi' },
  balance_health: { weight: 0.8, axiom: 'phi' },

  // CULTURE axiom (patterns, history)
  account_age: { weight: 0.5, axiom: 'culture' },
  transaction_history: { weight: 0.6, axiom: 'culture' },
  program_popularity: { weight: 0.4, axiom: 'culture' },

  // BURN axiom (value, contribution)
  burn_potential: { weight: 0.7, axiom: 'burn' },
  stake_amount: { weight: 0.6, axiom: 'burn' },
  fees_contributed: { weight: 0.5, axiom: 'burn' },
};

/**
 * φ-aligned thresholds for Solana judgments
 */
const JUDGMENT_THRESHOLDS = {
  excellent: 80,      // HOWL
  good: 50,           // WAG
  acceptable: PHI_INV_2 * 100, // ~38.2% - GROWL
  critical: 0,        // BARK
};

/**
 * Verdict mapping
 */
const getVerdict = (score) => {
  if (score >= JUDGMENT_THRESHOLDS.excellent) return 'HOWL';
  if (score >= JUDGMENT_THRESHOLDS.good) return 'WAG';
  if (score >= JUDGMENT_THRESHOLDS.acceptable) return 'GROWL';
  return 'BARK';
};

/**
 * SolanaJudge - Judges Solana blockchain data
 *
 * Implements the judgment layer for on-chain data (C2.2).
 */
export class SolanaJudge extends EventEmitter {
  /**
   * Create a new SolanaJudge
   *
   * @param {Object} [options] - Configuration options
   * @param {Object} [options.customDimensions] - Additional dimensions
   * @param {number} [options.maxConfidence=0.618] - Max confidence (φ⁻¹)
   */
  constructor(options = {}) {
    super();

    this.customDimensions = options.customDimensions || {};
    this.maxConfidence = options.maxConfidence || PHI_INV;

    // Merge dimensions
    this._dimensions = { ...SOLANA_DIMENSIONS, ...this.customDimensions };

    // Stats tracking
    this._stats = {
      totalJudgments: 0,
      byType: {},
      verdicts: { HOWL: 0, WAG: 0, GROWL: 0, BARK: 0 },
      avgScore: 0,
      lastJudgment: null,
    };

    // Initialize type counters
    for (const type of Object.values(SolanaJudgmentType)) {
      this._stats.byType[type] = 0;
    }
  }

  /**
   * Judge a transaction
   *
   * @param {Object} tx - Transaction data
   * @returns {Object} Judgment result
   */
  judgeTransaction(tx) {
    const scores = {};

    // Signature valid (always true if we have the tx)
    scores.signature_valid = tx.signature ? 100 : 0;

    // Fee efficiency (lower is better, normalize to 100)
    const maxFee = 5000; // lamports
    scores.fee_efficiency = tx.fee
      ? Math.min(100, ((maxFee - tx.fee) / maxFee) * 100)
      : 50;

    // Compute efficiency
    const maxCU = 200000;
    scores.compute_efficiency = tx.computeUnits
      ? Math.min(100, ((maxCU - tx.computeUnits) / maxCU) * 100)
      : 50;

    // Transaction success
    scores.transaction_history = tx.success ? 100 : 0;

    return this._createJudgment(SolanaJudgmentType.TRANSACTION, tx, scores);
  }

  /**
   * Judge an account
   *
   * @param {Object} account - Account data
   * @returns {Object} Judgment result
   */
  judgeAccount(account) {
    const scores = {};

    // Rent exempt
    scores.rent_exempt = account.rentEpoch === 0 || account.lamports > 890880 ? 100 : 0;

    // Balance health (φ-based thresholds)
    const solBalance = account.lamports / 1e9;
    if (solBalance >= 1) scores.balance_health = 100;
    else if (solBalance >= 0.1) scores.balance_health = 80;
    else if (solBalance >= 0.01) scores.balance_health = 50;
    else scores.balance_health = 20;

    // Account age (if available)
    scores.account_age = account.age
      ? Math.min(100, account.age / 365 * 100) // 1 year = 100
      : 50;

    // Stake amount (if applicable)
    scores.stake_amount = account.stakeAmount
      ? Math.min(100, (account.stakeAmount / 1e9) * 10) // 10 SOL = 100
      : null;

    return this._createJudgment(SolanaJudgmentType.ACCOUNT, account, scores);
  }

  /**
   * Judge a program
   *
   * @param {Object} program - Program data
   * @returns {Object} Judgment result
   */
  judgeProgram(program) {
    const scores = {};

    // Program verified (has verified build)
    scores.program_verified = program.verified ? 100 : 20;

    // Program popularity (by usage)
    if (program.txCount) {
      if (program.txCount > 1000000) scores.program_popularity = 100;
      else if (program.txCount > 100000) scores.program_popularity = 80;
      else if (program.txCount > 10000) scores.program_popularity = 60;
      else scores.program_popularity = 40;
    } else {
      scores.program_popularity = 50;
    }

    // Burn potential (fees generated)
    scores.burn_potential = program.feesGenerated
      ? Math.min(100, (program.feesGenerated / 1e9) * 100) // 1 SOL fees = 100
      : 50;

    return this._createJudgment(SolanaJudgmentType.PROGRAM, program, scores);
  }

  /**
   * Judge network state
   *
   * @param {Object} network - Network state data
   * @returns {Object} Judgment result
   */
  judgeNetwork(network) {
    const scores = {};

    // TPS health
    if (network.tps) {
      if (network.tps >= 2000) scores.compute_efficiency = 100;
      else if (network.tps >= 1000) scores.compute_efficiency = 80;
      else if (network.tps >= 500) scores.compute_efficiency = 60;
      else scores.compute_efficiency = 40;
    }

    // Slot lag
    if (network.slotLag !== undefined) {
      if (network.slotLag <= 1) scores.fee_efficiency = 100;
      else if (network.slotLag <= 5) scores.fee_efficiency = 80;
      else if (network.slotLag <= 20) scores.fee_efficiency = 60;
      else scores.fee_efficiency = 30;
    }

    // Validator health
    scores.stake_amount = network.activeValidators
      ? Math.min(100, (network.activeValidators / 2000) * 100)
      : 50;

    return this._createJudgment(SolanaJudgmentType.NETWORK, network, scores);
  }

  /**
   * Create judgment from scores
   * @private
   */
  _createJudgment(type, subject, scores) {
    // Filter null scores and calculate weighted average
    const validScores = Object.entries(scores)
      .filter(([k, v]) => v !== null && this._dimensions[k]);

    let totalWeight = 0;
    let weightedSum = 0;

    for (const [dim, score] of validScores) {
      const weight = this._dimensions[dim]?.weight || 1;
      weightedSum += score * weight;
      totalWeight += weight;
    }

    const rawScore = totalWeight > 0 ? weightedSum / totalWeight : 50;

    // Apply φ confidence cap
    const confidence = Math.min(this.maxConfidence, rawScore / 100);
    const finalScore = rawScore * confidence;

    const verdict = getVerdict(finalScore);

    const judgment = {
      type,
      cell: 'C2.2',
      dimension: 'SOLANA',
      analysis: 'JUDGE',
      subject: this._summarizeSubject(type, subject),
      scores,
      rawScore,
      confidence,
      finalScore,
      verdict,
      timestamp: Date.now(),
    };

    // Update stats
    this._updateStats(type, finalScore, verdict);

    // Emit events
    this.emit('judgment', judgment);
    globalEventBus.emit('solana:judgment', judgment);

    log.debug('Solana judgment', { type, verdict, finalScore: finalScore.toFixed(1) });

    return judgment;
  }

  /**
   * Summarize subject for logging
   * @private
   */
  _summarizeSubject(type, subject) {
    switch (type) {
      case SolanaJudgmentType.TRANSACTION:
        return { signature: subject.signature?.slice(0, 12) + '...' };
      case SolanaJudgmentType.ACCOUNT:
        return { pubkey: subject.pubkey?.slice(0, 12) + '...' };
      case SolanaJudgmentType.PROGRAM:
        return { programId: subject.programId?.slice(0, 12) + '...' };
      case SolanaJudgmentType.NETWORK:
        return { cluster: subject.cluster || 'unknown' };
      default:
        return {};
    }
  }

  /**
   * Update statistics
   * @private
   */
  _updateStats(type, score, verdict) {
    this._stats.totalJudgments++;
    this._stats.byType[type] = (this._stats.byType[type] || 0) + 1;
    this._stats.verdicts[verdict]++;
    this._stats.lastJudgment = Date.now();

    // Update rolling average
    const n = this._stats.totalJudgments;
    this._stats.avgScore = ((n - 1) * this._stats.avgScore + score) / n;
  }

  /**
   * Get judgment statistics
   *
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      ...this._stats,
      dimensions: Object.keys(this._dimensions).length,
    };
  }

  /**
   * Get judgment health
   *
   * @returns {Object} Health assessment
   */
  getHealth() {
    const now = Date.now();
    const timeSinceLastJudgment = this._stats.lastJudgment
      ? now - this._stats.lastJudgment
      : null;

    let status = 'healthy';
    let score = PHI_INV;

    if (this._stats.totalJudgments === 0) {
      status = 'idle';
      score = PHI_INV_2;
    } else if (this._stats.verdicts.BARK > this._stats.totalJudgments * 0.5) {
      status = 'alarming';
      score = PHI_INV_3;
    }

    return {
      status,
      score,
      totalJudgments: this._stats.totalJudgments,
      avgScore: this._stats.avgScore,
      verdictDistribution: this._stats.verdicts,
      timeSinceLastJudgment,
    };
  }

  /**
   * Clear statistics
   */
  clear() {
    this._stats = {
      totalJudgments: 0,
      byType: {},
      verdicts: { HOWL: 0, WAG: 0, GROWL: 0, BARK: 0 },
      avgScore: 0,
      lastJudgment: null,
    };
    for (const type of Object.values(SolanaJudgmentType)) {
      this._stats.byType[type] = 0;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create SolanaJudge singleton
 *
 * @param {Object} [options] - Options (only used on first call)
 * @returns {SolanaJudge}
 */
export function getSolanaJudge(options = {}) {
  if (!_instance) {
    _instance = new SolanaJudge(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetSolanaJudge() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  SolanaJudge,
  SolanaJudgmentType,
  getSolanaJudge,
  resetSolanaJudge,
};
