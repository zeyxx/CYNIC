/**
 * Layer 4: φ-BFT Consensus
 *
 * Votes weighted by E-Score × BURN × Uptime
 * φ⁻¹ (61.8%) supermajority threshold
 * Exponential lockout (φⁿ slots)
 * Soft vs Hard consensus modes
 *
 * @module @cynic/protocol/consensus
 */

export * from './voting.js';
export * from './lockout.js';
export * from './proposal.js';
