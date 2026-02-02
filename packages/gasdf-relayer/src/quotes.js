/**
 * @cynic/gasdf-relayer - Quote Management
 *
 * Handles fee quotes for gasless transactions.
 * Quotes expire and include φ-aligned fee splits.
 *
 * @module @cynic/gasdf-relayer/quotes
 */

'use strict';

import { nanoid } from 'nanoid';
import { calculateFee, getRelayerAddress } from './solana.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

const config = {
  // Quote expiry time (2 minutes)
  quoteExpiry: 2 * 60 * 1000,

  // Max quotes in memory
  maxQuotes: 10000,

  // Cleanup interval
  cleanupInterval: 60 * 1000,

  // φ-aligned rates
  burnRate: 0.763932022500210,
  treasuryRate: 0.236067977499790,
};

// ═══════════════════════════════════════════════════════════════════════════════
// Quote Store
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * In-memory quote store
 * Production: Use Redis or PostgreSQL
 */
const quotes = new Map();

/**
 * E-Score discount tiers
 */
const DISCOUNT_TIERS = Object.freeze({
  NONE: { min: 0, max: 0, discount: 0 },
  BRONZE: { min: 1, max: 25, discount: 0.10 },
  SILVER: { min: 26, max: 50, discount: 0.25 },
  GOLD: { min: 51, max: 75, discount: 0.45 },
  PLATINUM: { min: 76, max: 90, discount: 0.65 },
  DIAMOND: { min: 91, max: 100, discount: 0.85 },
  LEGEND: { min: 101, max: Infinity, discount: 0.95 },
});

/**
 * Accepted payment tokens
 */
const ACCEPTED_TOKENS = Object.freeze([
  {
    mint: 'So11111111111111111111111111111111111111112',
    symbol: 'SOL',
    name: 'Wrapped SOL',
    decimals: 9,
    priceSource: 'native',
  },
  {
    mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
    symbol: 'USDC',
    name: 'USD Coin',
    decimals: 6,
    priceSource: 'jupiter',
  },
  {
    mint: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB',
    symbol: 'USDT',
    name: 'Tether USD',
    decimals: 6,
    priceSource: 'jupiter',
  },
]);

// ═══════════════════════════════════════════════════════════════════════════════
// Quote Operations
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Get discount tier for E-Score
 *
 * @param {number} eScore - User's E-Score
 * @returns {Object} Tier info
 */
function getDiscountTier(eScore = 0) {
  for (const [name, tier] of Object.entries(DISCOUNT_TIERS)) {
    if (eScore >= tier.min && eScore <= tier.max) {
      return { name, ...tier };
    }
  }
  return { name: 'NONE', ...DISCOUNT_TIERS.NONE };
}

/**
 * Create a fee quote
 *
 * @param {Object} params - Quote parameters
 * @param {string} params.paymentToken - Token mint for fee payment
 * @param {string} params.userPubkey - User's wallet address
 * @param {number} [params.estimatedComputeUnits=200000] - Estimated CUs
 * @param {number} [params.eScore=0] - User's E-Score for discount
 * @returns {Promise<Object>} Quote data
 */
export async function createQuote(params) {
  const {
    paymentToken,
    userPubkey,
    estimatedComputeUnits = 200000,
    eScore = 0,
  } = params;

  // Validate token
  const token = ACCEPTED_TOKENS.find((t) => t.mint === paymentToken);
  if (!token) {
    throw new Error(`Token ${paymentToken} not accepted`);
  }

  // Calculate base fee
  const fee = await calculateFee(estimatedComputeUnits);

  // Get discount
  const tier = getDiscountTier(eScore);
  const discount = tier.discount;

  // Apply discount
  const discountedFee = BigInt(Math.floor(Number(fee.totalFee) * (1 - discount)));

  // φ-aligned split
  const burnAmount = BigInt(Math.floor(Number(discountedFee) * config.burnRate));
  const treasuryAmount = discountedFee - burnAmount;

  // Create quote
  const quoteId = `q_${nanoid(16)}`;
  const now = Date.now();
  const expiresAt = now + config.quoteExpiry;

  const quote = {
    quoteId,
    createdAt: now,
    expiresAt,

    // User info
    userPubkey,
    eScore,

    // Fee info
    paymentToken,
    paymentSymbol: token.symbol,
    estimatedComputeUnits,
    baseFee: fee.totalFee.toString(),
    discount,
    tier: tier.name,
    feeAmount: discountedFee.toString(),
    feeToken: paymentToken,

    // Split
    burnAmount: burnAmount.toString(),
    treasuryAmount: treasuryAmount.toString(),

    // Relayer
    feePayer: getRelayerAddress(),

    // Status
    status: 'pending',
    usedAt: null,
    txSignature: null,
  };

  // Store quote
  quotes.set(quoteId, quote);

  // Cleanup if too many quotes
  if (quotes.size > config.maxQuotes) {
    cleanupExpiredQuotes();
  }

  return {
    quoteId: quote.quoteId,
    feePayer: quote.feePayer,
    feeAmount: quote.feeAmount,
    feeToken: quote.feeToken,
    discount: quote.discount,
    tier: quote.tier,
    expiresAt: quote.expiresAt,
    burnAmount: quote.burnAmount,
    treasuryAmount: quote.treasuryAmount,
    // Breakdown
    breakdown: {
      baseFee: quote.baseFee,
      discountRate: quote.discount,
      burnRate: config.burnRate,
      treasuryRate: config.treasuryRate,
    },
  };
}

/**
 * Get a quote by ID
 *
 * @param {string} quoteId - Quote ID
 * @returns {Object|null} Quote or null if not found/expired
 */
export function getQuote(quoteId) {
  const quote = quotes.get(quoteId);

  if (!quote) {
    return null;
  }

  // Check expiry
  if (Date.now() > quote.expiresAt) {
    quotes.delete(quoteId);
    return null;
  }

  return quote;
}

/**
 * Mark a quote as used
 *
 * @param {string} quoteId - Quote ID
 * @param {string} txSignature - Transaction signature
 * @returns {boolean} Success
 */
export function markQuoteUsed(quoteId, txSignature) {
  const quote = getQuote(quoteId);

  if (!quote) {
    return false;
  }

  if (quote.status !== 'pending') {
    return false;
  }

  quote.status = 'used';
  quote.usedAt = Date.now();
  quote.txSignature = txSignature;
  quotes.set(quoteId, quote);

  return true;
}

/**
 * Validate a quote for submission
 *
 * @param {string} quoteId - Quote ID
 * @returns {Object} Validation result
 */
export function validateQuote(quoteId) {
  const quote = quotes.get(quoteId);

  if (!quote) {
    return { valid: false, error: 'Quote not found' };
  }

  if (Date.now() > quote.expiresAt) {
    quotes.delete(quoteId);
    return { valid: false, error: 'Quote expired' };
  }

  if (quote.status !== 'pending') {
    return { valid: false, error: `Quote already ${quote.status}` };
  }

  return { valid: true, quote };
}

/**
 * Cleanup expired quotes
 */
export function cleanupExpiredQuotes() {
  const now = Date.now();
  let cleaned = 0;

  for (const [id, quote] of quotes) {
    if (now > quote.expiresAt || quote.status === 'used') {
      quotes.delete(id);
      cleaned++;
    }
  }

  return cleaned;
}

/**
 * Get accepted tokens list
 */
export function getAcceptedTokens() {
  return ACCEPTED_TOKENS.map((t) => ({
    mint: t.mint,
    symbol: t.symbol,
    name: t.name,
    decimals: t.decimals,
  }));
}

/**
 * Get discount tiers
 */
export function getDiscountTiers() {
  return Object.entries(DISCOUNT_TIERS).map(([name, tier]) => ({
    name,
    minEScore: tier.min,
    maxEScore: tier.max,
    discount: tier.discount,
    discountPercent: Math.round(tier.discount * 100),
  }));
}

/**
 * Get quote statistics
 */
export function getQuoteStats() {
  let pending = 0;
  let used = 0;
  let expired = 0;
  const now = Date.now();

  for (const quote of quotes.values()) {
    if (quote.status === 'used') {
      used++;
    } else if (now > quote.expiresAt) {
      expired++;
    } else {
      pending++;
    }
  }

  return {
    total: quotes.size,
    pending,
    used,
    expired,
    maxQuotes: config.maxQuotes,
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// Cleanup Timer
// ═══════════════════════════════════════════════════════════════════════════════

let cleanupTimer = null;

export function startCleanupTimer() {
  if (cleanupTimer) return;

  cleanupTimer = setInterval(() => {
    const cleaned = cleanupExpiredQuotes();
    if (cleaned > 0) {
      console.log(`[Quotes] Cleaned up ${cleaned} expired quotes`);
    }
  }, config.cleanupInterval);
}

export function stopCleanupTimer() {
  if (cleanupTimer) {
    clearInterval(cleanupTimer);
    cleanupTimer = null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Exports
// ═══════════════════════════════════════════════════════════════════════════════

export default {
  createQuote,
  getQuote,
  markQuoteUsed,
  validateQuote,
  cleanupExpiredQuotes,
  getAcceptedTokens,
  getDiscountTiers,
  getQuoteStats,
  startCleanupTimer,
  stopCleanupTimer,
  ACCEPTED_TOKENS,
  DISCOUNT_TIERS,
};
