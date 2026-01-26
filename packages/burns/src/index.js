/**
 * @cynic/burns - Burn Verification
 *
 * "Onchain is truth - burns must be verified" - κυνικός
 *
 * This package verifies burn transactions on Solana blockchain.
 * Burns are a core part of the CYNIC ecosystem - operators must burn to participate.
 *
 * ## Usage
 *
 * ```javascript
 * import { createBurnVerifier, SolanaCluster } from '@cynic/burns';
 *
 * // On-chain verification (preferred)
 * const verifier = createBurnVerifier({
 *   solanaCluster: SolanaCluster.MAINNET,
 * });
 *
 * // Verify a burn
 * const result = await verifier.verify('tx_signature_here');
 * if (result.verified) {
 *   console.log(`Burn verified: ${result.amount} by ${result.burner}`);
 * }
 *
 * // Check cache
 * if (verifier.isVerified('tx_signature')) {
 *   // Already verified
 * }
 * ```
 *
 * @module @cynic/burns
 */

'use strict';

// Verifier (main interface)
export {
  BurnVerifier,
  createBurnVerifier,
  BurnStatus,
  DEFAULT_CONFIG,
} from './verifier.js';

// Solana on-chain verifier
export {
  SolanaBurnVerifier,
  createSolanaBurnVerifier,
  SolanaCluster,
  BURN_ADDRESSES,
} from './solana-verifier.js';

// Burn enforcer (for requiring burns before operations)
export {
  BurnEnforcer,
  createBurnEnforcer,
  BurnRequiredError,
  DEFAULT_ENFORCER_CONFIG,
} from './enforcer.js';
