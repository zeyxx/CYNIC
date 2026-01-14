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
export * from './engine.js';
export * from './finality.js';
export * from './slot.js';
export * from './messages.js';
export * from './gossip-bridge.js';
