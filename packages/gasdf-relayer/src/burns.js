/**
 * @cynic/gasdf-relayer - Burn Tracking
 *
 * Tracks all burns for transparency and CYNIC integration.
 * "Don't Extract, Burn" - κυνικός
 *
 * @module @cynic/gasdf-relayer/burns
 */

'use strict';

import { createBurnTransaction, createTreasuryTransfer, confirmTransaction } from './solana.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

const config = {
  // φ-aligned rates
  burnRate: 0.763932022500210,
  treasuryRate: 0.236067977499790,

  // Batch burns (accumulate small burns)
  batchThreshold: 10_000_000n, // 0.01 SOL minimum to execute burn
  batchInterval: 60_000, // Check every minute

  // Max records to keep in memory
  maxRecords: 10000,
};

// ═══════════════════════════════════════════════════════════════════════════════
// Burn Store
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * In-memory burn records
 * Production: PostgreSQL with burn_records table
 */
const burnRecords = [];

/**
 * Pending burns (to be batched)
 */
let pendingBurnAmount = 0n;
let pendingTreasuryAmount = 0n;

/**
 * Aggregate statistics
 */
const stats = {
  totalBurned: 0n,
  totalTreasury: 0n,
  burnCount: 0,
  lastBurn: null,
  last24hBurned: 0n,
  last24hCount: 0,
};

// ═══════════════════════════════════════════════════════════════════════════════
// Burn Operations
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Record a fee for burning
 *
 * Accumulates fees until threshold is reached, then executes burn.
 *
 * @param {Object} params - Fee parameters
 * @param {string|bigint} params.feeAmount - Total fee collected
 * @param {string} params.quoteId - Quote ID
 * @param {string} params.userPubkey - User who paid
 * @param {string} params.txSignature - User's transaction signature
 * @returns {Promise<Object>} Burn record
 */
export async function recordFee(params) {
  const { feeAmount, quoteId, userPubkey, txSignature } = params;

  const fee = BigInt(feeAmount);

  // Calculate φ-aligned split
  const burnAmount = BigInt(Math.floor(Number(fee) * config.burnRate));
  const treasuryAmount = fee - burnAmount;

  // Add to pending
  pendingBurnAmount += burnAmount;
  pendingTreasuryAmount += treasuryAmount;

  // Create record
  const record = {
    id: `burn_${Date.now()}_${burnRecords.length}`,
    timestamp: Date.now(),
    quoteId,
    userPubkey,
    txSignature,
    feeAmount: fee.toString(),
    burnAmount: burnAmount.toString(),
    treasuryAmount: treasuryAmount.toString(),
    // Will be filled when burn executes
    burnTxSignature: null,
    treasuryTxSignature: null,
    status: 'pending',
  };

  burnRecords.push(record);

  // Trim records if too many
  while (burnRecords.length > config.maxRecords) {
    burnRecords.shift();
  }

  // Check if we should execute burn
  let burnResult = null;
  if (pendingBurnAmount >= config.batchThreshold) {
    burnResult = await executePendingBurns();
  }

  return {
    record,
    burnExecuted: !!burnResult,
    burnResult,
    pendingBurnAmount: pendingBurnAmount.toString(),
    pendingTreasuryAmount: pendingTreasuryAmount.toString(),
  };
}

/**
 * Execute pending burns
 *
 * Sends accumulated burns to null address and treasury.
 */
export async function executePendingBurns() {
  if (pendingBurnAmount === 0n && pendingTreasuryAmount === 0n) {
    return null;
  }

  const burnAmount = pendingBurnAmount;
  const treasuryAmount = pendingTreasuryAmount;

  // Reset pending
  pendingBurnAmount = 0n;
  pendingTreasuryAmount = 0n;

  const result = {
    timestamp: Date.now(),
    burnAmount: burnAmount.toString(),
    treasuryAmount: treasuryAmount.toString(),
    burnTxSignature: null,
    treasuryTxSignature: null,
    errors: [],
  };

  // Execute burn transaction
  if (burnAmount > 0n) {
    try {
      result.burnTxSignature = await createBurnTransaction(burnAmount);

      // Update stats
      stats.totalBurned += burnAmount;
      stats.burnCount++;
      stats.lastBurn = Date.now();
      stats.last24hBurned += burnAmount;
      stats.last24hCount++;

      console.log(`[Burns] Burned ${Number(burnAmount) / 1e9} SOL - tx: ${result.burnTxSignature}`);
    } catch (err) {
      result.errors.push(`Burn failed: ${err.message}`);
      // Return amount to pending for retry
      pendingBurnAmount += burnAmount;
    }
  }

  // Execute treasury transfer
  if (treasuryAmount > 0n) {
    try {
      result.treasuryTxSignature = await createTreasuryTransfer(treasuryAmount);

      // Update stats
      stats.totalTreasury += treasuryAmount;

      console.log(`[Burns] Treasury ${Number(treasuryAmount) / 1e9} SOL - tx: ${result.treasuryTxSignature}`);
    } catch (err) {
      result.errors.push(`Treasury failed: ${err.message}`);
      // Return amount to pending for retry
      pendingTreasuryAmount += treasuryAmount;
    }
  }

  // Update pending burn records
  for (const record of burnRecords) {
    if (record.status === 'pending') {
      record.status = result.errors.length === 0 ? 'completed' : 'partial';
      record.burnTxSignature = result.burnTxSignature;
      record.treasuryTxSignature = result.treasuryTxSignature;
    }
  }

  return result;
}

/**
 * Force execute all pending burns
 *
 * Used for shutdown or manual trigger.
 */
export async function forceExecuteBurns() {
  if (pendingBurnAmount > 0n || pendingTreasuryAmount > 0n) {
    console.log(`[Burns] Force executing pending burns: ${Number(pendingBurnAmount) / 1e9} SOL burn, ${Number(pendingTreasuryAmount) / 1e9} SOL treasury`);
    return executePendingBurns();
  }
  return null;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Statistics
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Get burn statistics
 */
export function getStats() {
  return {
    totalBurned: stats.totalBurned.toString(),
    totalBurnedSol: Number(stats.totalBurned) / 1e9,
    totalTreasury: stats.totalTreasury.toString(),
    totalTreasurySol: Number(stats.totalTreasury) / 1e9,
    burnCount: stats.burnCount,
    lastBurn: stats.lastBurn,
    last24h: {
      burned: stats.last24hBurned.toString(),
      burnedSol: Number(stats.last24hBurned) / 1e9,
      count: stats.last24hCount,
    },
    pending: {
      burnAmount: pendingBurnAmount.toString(),
      burnAmountSol: Number(pendingBurnAmount) / 1e9,
      treasuryAmount: pendingTreasuryAmount.toString(),
      treasuryAmountSol: Number(pendingTreasuryAmount) / 1e9,
    },
    averageBurn: stats.burnCount > 0
      ? (Number(stats.totalBurned) / stats.burnCount / 1e9).toFixed(6)
      : '0',
    phiRatios: {
      burn: config.burnRate,
      treasury: config.treasuryRate,
    },
  };
}

/**
 * Get recent burn records
 *
 * @param {number} [limit=50] - Max records to return
 */
export function getRecentBurns(limit = 50) {
  return burnRecords
    .slice(-limit)
    .reverse()
    .map((r) => ({
      id: r.id,
      timestamp: r.timestamp,
      userPubkey: r.userPubkey,
      feeAmount: r.feeAmount,
      burnAmount: r.burnAmount,
      treasuryAmount: r.treasuryAmount,
      status: r.status,
      burnTxSignature: r.burnTxSignature,
    }));
}

/**
 * Verify a burn transaction on-chain
 *
 * @param {string} signature - Burn transaction signature
 */
export async function verifyBurn(signature) {
  try {
    const result = await confirmTransaction(signature);

    return {
      verified: result.confirmed && !result.err,
      confirmationStatus: result.confirmationStatus,
      slot: result.slot,
      error: result.err,
    };
  } catch (err) {
    return {
      verified: false,
      error: err.message,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 24h Stats Rotation
// ═══════════════════════════════════════════════════════════════════════════════

let statsRotationTimer = null;

/**
 * Start 24h stats rotation
 */
export function startStatsRotation() {
  if (statsRotationTimer) return;

  // Reset 24h stats every 24 hours
  statsRotationTimer = setInterval(
    () => {
      console.log(`[Burns] Rotating 24h stats - burned: ${Number(stats.last24hBurned) / 1e9} SOL, count: ${stats.last24hCount}`);
      stats.last24hBurned = 0n;
      stats.last24hCount = 0;
    },
    24 * 60 * 60 * 1000
  );
}

/**
 * Stop stats rotation
 */
export function stopStatsRotation() {
  if (statsRotationTimer) {
    clearInterval(statsRotationTimer);
    statsRotationTimer = null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Batch Timer
// ═══════════════════════════════════════════════════════════════════════════════

let batchTimer = null;

/**
 * Start batch burn timer
 */
export function startBatchTimer() {
  if (batchTimer) return;

  batchTimer = setInterval(async () => {
    // Execute burns even if under threshold after interval
    if (pendingBurnAmount > 0n || pendingTreasuryAmount > 0n) {
      await executePendingBurns();
    }
  }, config.batchInterval);
}

/**
 * Stop batch timer
 */
export function stopBatchTimer() {
  if (batchTimer) {
    clearInterval(batchTimer);
    batchTimer = null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Exports
// ═══════════════════════════════════════════════════════════════════════════════

export default {
  recordFee,
  executePendingBurns,
  forceExecuteBurns,
  getStats,
  getRecentBurns,
  verifyBurn,
  startStatsRotation,
  stopStatsRotation,
  startBatchTimer,
  stopBatchTimer,
  config: Object.freeze({ ...config }),
};
