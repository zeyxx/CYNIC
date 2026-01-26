/**
 * @cynic/anchor - Solana Anchoring
 *
 * "Onchain is truth" - κυνικός
 *
 * This package provides Solana blockchain anchoring for CYNIC's memory.
 * Judgments, patterns, and knowledge are anchored as merkle roots,
 * making CYNIC's memory verifiable and immutable.
 *
 * ## Architecture
 *
 * ```
 * ┌──────────────────────────────────────────────────────────┐
 * │              LAYER 4: TRUTH (On-chain)                   │
 * │  Solana: The anchor of truth                             │
 * │  - PoJ Block merkle roots (periodic)                     │
 * │  - Burn verification                                     │
 * │  - E-Score snapshots                                     │
 * └──────────────────────────────────────────────────────────┘
 * ```
 *
 * ## Usage
 *
 * ```javascript
 * import { createAnchorer, createAnchorQueue } from '@cynic/anchor';
 *
 * // Create anchorer (simulation mode without wallet)
 * const anchorer = createAnchorer();
 *
 * // Create queue with batching
 * const queue = createAnchorQueue({
 *   anchorer,
 *   autoStart: true,
 * });
 *
 * // Enqueue judgment for anchoring
 * queue.enqueue('jdg_abc123', judgmentData);
 *
 * // Get proof after anchoring
 * const proof = queue.getProof('jdg_abc123');
 * ```
 *
 * @module @cynic/anchor
 */

'use strict';

// Constants
export {
  AnchorStatus,
  ANCHOR_CONSTANTS,
  SolanaCluster,
  DEFAULT_CONFIG,
  CYNIC_PROGRAM,
} from './constants.js';

// Anchorer
export { SolanaAnchorer, createAnchorer } from './anchorer.js';

// Program Client
export { CynicProgramClient, createProgramClient } from './program-client.js';

// Queue
export { AnchorQueue, createAnchorQueue } from './queue.js';

// Wallet
export {
  WalletType,
  CynicWallet,
  loadWalletFromFile,
  loadWalletFromEnv,
  generateWallet,
  saveWalletToFile,
  getDefaultWalletPath,
  base58Encode,
  base58Decode,
} from './wallet.js';

// PoJ Chain Integration
export {
  PoJAnchorIntegration,
  createPoJAnchorIntegration,
  connectPoJToSolana,
} from './poj-integration.js';
