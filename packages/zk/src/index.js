/**
 * @cynic/zk - Zero-Knowledge Proofs for CYNIC
 *
 * Generates and verifies ZK proofs for judgments using Noir circuits.
 *
 * ## Features
 *
 * - **Score Range Proofs**: Prove score is within [0, 100] without revealing exact value
 * - **Verdict Proofs**: Prove verdict matches score thresholds (HOWL/WAG/GROWL/BARK)
 * - **Burn Threshold Proofs**: Prove burn amount >= minimum without revealing exact amount
 *
 * ## Usage
 *
 * ```javascript
 * import { createZKProver, createZKVerifier } from '@cynic/zk';
 *
 * // Generate a proof that score >= 61 (WAG threshold)
 * const prover = createZKProver();
 * const proof = await prover.proveScoreRange(75, 61);
 *
 * // Verify the proof (without knowing the actual score)
 * const verifier = createZKVerifier();
 * const result = await verifier.verify(proof);
 * console.log(result.valid); // true
 * ```
 *
 * ## Circuits
 *
 * Noir circuits are located in `circuits/` directory:
 * - `score_range/` - Score range proof
 * - `verdict_valid/` - Verdict validity proof
 * - `burn_threshold/` - Burn threshold proof
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/zk
 */

'use strict';

// Prover
export {
  ZK_CONSTANTS,
  ProofType,
  generateBlinding,
  blindingToField,
  computeCommitment,
  getVerdictForScore,
  getVerdictName,
  ZKProver,
  createZKProver,
} from './prover.js';

// Verifier
export {
  ZKVerifier,
  createZKVerifier,
  quickVerify,
  verifyJudgmentProof,
} from './verifier.js';
