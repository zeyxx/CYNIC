/**
 * CYNIC Solana Accountant - C2.6 (SOLANA × ACCOUNT)
 *
 * Tracks Solana economics: fees spent, lamports moved, efficiency.
 * Part of the 7×7 Fractal Matrix accounting layer.
 *
 * "Le chien compte les lamports" - κυνικός
 *
 * Accounts:
 * - Transaction fees (total, avg, max)
 * - Lamports moved (in, out, net)
 * - Compute units consumed
 * - SOL efficiency metrics
 *
 * @module @cynic/node/solana/solana-accountant
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('SolanaAccountant');

/**
 * Transaction types for accounting
 */
export const SolanaTransactionType = {
  TRANSFER: 'transfer',
  SWAP: 'swap',
  MINT: 'mint',
  BURN: 'burn',
  STAKE: 'stake',
  UNSTAKE: 'unstake',
  VOTE: 'vote',
  OTHER: 'other',
};

/**
 * SolanaAccountant - Tracks Solana economics
 */
export class SolanaAccountant extends EventEmitter {
  /**
   * Create a new SolanaAccountant
   *
   * @param {Object} [options] - Configuration
   * @param {string} [options.userId] - User/wallet identifier
   */
  constructor(options = {}) {
    super();

    this._userId = options.userId || 'default';

    // Transaction tracking
    this._transactions = [];
    this._maxTransactions = 1000;

    // Aggregated metrics
    this._totals = {
      feesPaid: 0,           // Total lamports spent on fees
      lamportsSent: 0,       // Total lamports sent
      lamportsReceived: 0,   // Total lamports received
      computeUnitsUsed: 0,   // Total CU consumed
      transactionCount: 0,   // Total transactions
      successCount: 0,       // Successful transactions
      failCount: 0,          // Failed transactions
    };

    // Per-type tracking
    this._byType = new Map();
    for (const type of Object.values(SolanaTransactionType)) {
      this._byType.set(type, {
        count: 0,
        fees: 0,
        lamports: 0,
      });
    }

    // Daily aggregates
    this._dailyMetrics = new Map();

    // Session tracking
    this._currentSession = null;
  }

  /**
   * Record a transaction
   *
   * @param {Object} tx - Transaction data
   * @returns {Object} Recording result
   */
  recordTransaction(tx) {
    const timestamp = Date.now();
    const dateKey = new Date(timestamp).toISOString().split('T')[0];

    const record = {
      signature: tx.signature,
      type: tx.type || SolanaTransactionType.OTHER,
      fee: tx.fee || 0,
      lamportsSent: tx.lamportsSent || 0,
      lamportsReceived: tx.lamportsReceived || 0,
      computeUnits: tx.computeUnits || 0,
      success: tx.success !== false,
      timestamp,
    };

    // Add to history
    this._transactions.push(record);
    while (this._transactions.length > this._maxTransactions) {
      this._transactions.shift();
    }

    // Update totals
    this._totals.transactionCount++;
    this._totals.feesPaid += record.fee;
    this._totals.lamportsSent += record.lamportsSent;
    this._totals.lamportsReceived += record.lamportsReceived;
    this._totals.computeUnitsUsed += record.computeUnits;

    if (record.success) {
      this._totals.successCount++;
    } else {
      this._totals.failCount++;
    }

    // Update by-type
    const typeMetrics = this._byType.get(record.type);
    if (typeMetrics) {
      typeMetrics.count++;
      typeMetrics.fees += record.fee;
      typeMetrics.lamports += record.lamportsSent;
    }

    // Update daily metrics
    this._updateDailyMetrics(dateKey, record);

    // Emit event
    const result = {
      recorded: true,
      cell: 'C2.6',
      dimension: 'SOLANA',
      analysis: 'ACCOUNT',
      transaction: record,
      timestamp,
    };

    this.emit('transaction_recorded', result);
    globalEventBus.emit('solana:accounting', result);

    return result;
  }

  /**
   * Update daily metrics
   * @private
   */
  _updateDailyMetrics(dateKey, record) {
    let daily = this._dailyMetrics.get(dateKey);

    if (!daily) {
      daily = {
        date: dateKey,
        transactions: 0,
        feesPaid: 0,
        lamportsSent: 0,
        lamportsReceived: 0,
        successCount: 0,
        failCount: 0,
      };
      this._dailyMetrics.set(dateKey, daily);
    }

    daily.transactions++;
    daily.feesPaid += record.fee;
    daily.lamportsSent += record.lamportsSent;
    daily.lamportsReceived += record.lamportsReceived;

    if (record.success) {
      daily.successCount++;
    } else {
      daily.failCount++;
    }
  }

  /**
   * Get total metrics
   *
   * @returns {Object} Total metrics
   */
  getTotals() {
    const netLamports = this._totals.lamportsReceived - this._totals.lamportsSent;
    const successRate = this._totals.transactionCount > 0
      ? this._totals.successCount / this._totals.transactionCount
      : 0;
    const avgFee = this._totals.transactionCount > 0
      ? this._totals.feesPaid / this._totals.transactionCount
      : 0;

    return {
      ...this._totals,
      netLamports,
      netSOL: netLamports / 1e9,
      feesSOL: this._totals.feesPaid / 1e9,
      successRate,
      avgFee: Math.round(avgFee),
    };
  }

  /**
   * Get daily summary
   *
   * @param {string} [date] - Date (YYYY-MM-DD), defaults to today
   * @returns {Object} Daily metrics
   */
  getDailySummary(date = null) {
    const dateKey = date || new Date().toISOString().split('T')[0];
    const daily = this._dailyMetrics.get(dateKey);

    if (!daily) {
      return {
        date: dateKey,
        transactions: 0,
        feesPaid: 0,
        feesSOL: 0,
        netLamports: 0,
        netSOL: 0,
        successRate: 0,
      };
    }

    const netLamports = daily.lamportsReceived - daily.lamportsSent;

    return {
      date: dateKey,
      transactions: daily.transactions,
      feesPaid: daily.feesPaid,
      feesSOL: daily.feesPaid / 1e9,
      lamportsSent: daily.lamportsSent,
      lamportsReceived: daily.lamportsReceived,
      netLamports,
      netSOL: netLamports / 1e9,
      successRate: daily.transactions > 0
        ? daily.successCount / daily.transactions
        : 0,
    };
  }

  /**
   * Get breakdown by transaction type
   *
   * @returns {Array} Type breakdown
   */
  getTypeBreakdown() {
    const breakdown = [];

    for (const [type, metrics] of this._byType) {
      if (metrics.count > 0) {
        breakdown.push({
          type,
          count: metrics.count,
          fees: metrics.fees,
          feesSOL: metrics.fees / 1e9,
          lamports: metrics.lamports,
          avgFee: Math.round(metrics.fees / metrics.count),
        });
      }
    }

    return breakdown.sort((a, b) => b.count - a.count);
  }

  /**
   * Calculate fee efficiency
   *
   * @returns {Object} Efficiency metrics
   */
  getFeeEfficiency() {
    const totals = this.getTotals();

    // Efficiency = value moved per fee paid
    const valuePerFee = totals.feesPaid > 0
      ? totals.lamportsSent / totals.feesPaid
      : 0;

    // CU efficiency = work per fee
    const cuPerFee = totals.feesPaid > 0
      ? totals.computeUnitsUsed / totals.feesPaid
      : 0;

    // φ-aligned efficiency rating
    let rating;
    if (valuePerFee > 10000) rating = 'excellent';
    else if (valuePerFee > 1000) rating = 'good';
    else if (valuePerFee > 100) rating = 'acceptable';
    else rating = 'poor';

    return {
      valuePerFee,
      cuPerFee,
      rating,
      avgFee: totals.avgFee,
      totalFeesSOL: totals.feesSOL,
      successRate: totals.successRate,
    };
  }

  /**
   * Get statistics
   *
   * @returns {Object}
   */
  getStats() {
    return {
      transactionCount: this._totals.transactionCount,
      daysTracked: this._dailyMetrics.size,
      transactionsStored: this._transactions.length,
      lastTransaction: this._transactions.length > 0
        ? this._transactions[this._transactions.length - 1].timestamp
        : null,
    };
  }

  /**
   * Get health assessment
   *
   * @returns {Object}
   */
  getHealth() {
    const efficiency = this.getFeeEfficiency();
    const totals = this.getTotals();

    let status = 'healthy';
    let score = PHI_INV;

    // Check for high failure rate
    if (totals.successRate < PHI_INV_2) {
      status = 'degraded';
      score = PHI_INV_2;
    }

    // Check for poor efficiency
    if (efficiency.rating === 'poor') {
      status = 'inefficient';
      score = PHI_INV_2;
    }

    return {
      status,
      score,
      successRate: totals.successRate,
      efficiencyRating: efficiency.rating,
      totalFeesSOL: totals.feesSOL,
      transactionCount: totals.transactionCount,
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this._transactions = [];
    this._totals = {
      feesPaid: 0,
      lamportsSent: 0,
      lamportsReceived: 0,
      computeUnitsUsed: 0,
      transactionCount: 0,
      successCount: 0,
      failCount: 0,
    };
    for (const [, metrics] of this._byType) {
      metrics.count = 0;
      metrics.fees = 0;
      metrics.lamports = 0;
    }
    this._dailyMetrics.clear();
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create SolanaAccountant singleton
 *
 * @param {Object} [options] - Options
 * @returns {SolanaAccountant}
 */
export function getSolanaAccountant(options = {}) {
  if (!_instance) {
    _instance = new SolanaAccountant(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetSolanaAccountant() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  SolanaAccountant,
  SolanaTransactionType,
  getSolanaAccountant,
  resetSolanaAccountant,
};
