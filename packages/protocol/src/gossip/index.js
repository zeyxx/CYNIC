/**
 * Layer 3: Gossip Propagation
 *
 * Fibonacci fanout (13 peers per hop)
 * O(log₁₃ n) scalability
 * Push-pull hybrid propagation
 *
 * @module @cynic/protocol/gossip
 */

export * from './message.js';
export * from './peer.js';
export * from './propagation.js';
