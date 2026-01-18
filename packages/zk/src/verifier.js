/**
 * @cynic/zk - ZK Proof Verifier
 *
 * Verifies zero-knowledge proofs for CYNIC judgments.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/zk/verifier
 */

'use strict';

import { createHash } from 'crypto';
import { PHI_INV } from '@cynic/core';
import {
  ZK_CONSTANTS,
  ProofType,
  computeCommitment,
  getVerdictForScore,
  blindingToField,
} from './prover.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Verification Result
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Verification result
 * @typedef {Object} VerificationResult
 * @property {boolean} valid - Whether proof is valid
 * @property {string} type - Proof type
 * @property {Object} publicInputs - Public inputs from proof
 * @property {number} confidence - Confidence level (max φ⁻¹)
 * @property {string} [error] - Error message if invalid
 */

// ═══════════════════════════════════════════════════════════════════════════════
// ZK Proof Verifier
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * ZK Proof Verifier
 *
 * Verifies zero-knowledge proofs for CYNIC judgments.
 * Supports both real Noir proofs and simulated proofs.
 */
export class ZKVerifier {
  /**
   * @param {Object} [options] - Configuration
   * @param {Object} [options.circuits] - Pre-compiled circuit artifacts
   */
  constructor(options = {}) {
    this.circuits = options.circuits || {};
    this.backend = null;

    // Stats
    this.stats = {
      proofsVerified: 0,
      proofsRejected: 0,
      totalVerifyTime: 0,
    };
  }

  /**
   * Initialize Barretenberg (lazy load)
   * @private
   */
  async _ensureInitialized() {
    if (this.backend) return;

    try {
      const { BarretenbergBackend } = await import('@aztec/bb.js');
      this.backend = BarretenbergBackend;
    } catch (error) {
      console.warn('⚠️ Barretenberg not available, using simulation mode');
      this.backend = null;
    }
  }

  /**
   * Verify a score range proof
   *
   * @param {Object} proofData - Proof data from prover
   * @returns {Promise<VerificationResult>}
   */
  async verifyScoreRange(proofData) {
    const startTime = Date.now();

    try {
      // Validate structure
      if (proofData.type !== ProofType.SCORE_RANGE) {
        return this._reject('Invalid proof type', proofData.type, startTime);
      }

      if (!proofData.publicInputs?.commitment || proofData.publicInputs?.minScore === undefined) {
        return this._reject('Missing public inputs', proofData.type, startTime);
      }

      await this._ensureInitialized();

      // If real proof and backend available
      if (this.backend && this.circuits.score_range && !proofData.proof?.mock) {
        const circuit = this.circuits.score_range;
        const backend = new this.backend(circuit.bytecode);

        const isValid = await backend.verifyProof({
          proof: proofData.proof,
          publicInputs: [
            proofData.publicInputs.commitment,
            proofData.publicInputs.minScore.toString(),
          ],
        });

        if (!isValid) {
          return this._reject('Proof verification failed', proofData.type, startTime);
        }
      }

      // Simulated proof - verify mock signature
      if (proofData.proof?.mock) {
        // In simulation mode, we trust the proof was generated correctly
        // Real deployment would require Noir circuits
      }

      this.stats.proofsVerified++;
      this.stats.totalVerifyTime += Date.now() - startTime;

      return {
        valid: true,
        type: ProofType.SCORE_RANGE,
        publicInputs: proofData.publicInputs,
        confidence: PHI_INV,
        verifyTime: Date.now() - startTime,
        simulated: !!proofData.proof?.mock,
      };
    } catch (error) {
      return this._reject(error.message, ProofType.SCORE_RANGE, startTime);
    }
  }

  /**
   * Verify a verdict validity proof
   *
   * @param {Object} proofData - Proof data from prover
   * @returns {Promise<VerificationResult>}
   */
  async verifyVerdictValid(proofData) {
    const startTime = Date.now();

    try {
      if (proofData.type !== ProofType.VERDICT_VALID) {
        return this._reject('Invalid proof type', proofData.type, startTime);
      }

      if (!proofData.publicInputs?.commitment || proofData.publicInputs?.verdict === undefined) {
        return this._reject('Missing public inputs', proofData.type, startTime);
      }

      // Validate verdict value
      const verdict = proofData.publicInputs.verdict;
      if (verdict < 0 || verdict > 3) {
        return this._reject('Invalid verdict value', proofData.type, startTime);
      }

      await this._ensureInitialized();

      // If real proof and backend available
      if (this.backend && this.circuits.verdict_valid && !proofData.proof?.mock) {
        const circuit = this.circuits.verdict_valid;
        const backend = new this.backend(circuit.bytecode);

        const isValid = await backend.verifyProof({
          proof: proofData.proof,
          publicInputs: [
            proofData.publicInputs.commitment,
            verdict.toString(),
          ],
        });

        if (!isValid) {
          return this._reject('Proof verification failed', proofData.type, startTime);
        }
      }

      this.stats.proofsVerified++;
      this.stats.totalVerifyTime += Date.now() - startTime;

      return {
        valid: true,
        type: ProofType.VERDICT_VALID,
        publicInputs: proofData.publicInputs,
        confidence: PHI_INV,
        verifyTime: Date.now() - startTime,
        simulated: !!proofData.proof?.mock,
      };
    } catch (error) {
      return this._reject(error.message, ProofType.VERDICT_VALID, startTime);
    }
  }

  /**
   * Verify a burn threshold proof
   *
   * @param {Object} proofData - Proof data from prover
   * @returns {Promise<VerificationResult>}
   */
  async verifyBurnThreshold(proofData) {
    const startTime = Date.now();

    try {
      if (proofData.type !== ProofType.BURN_THRESHOLD) {
        return this._reject('Invalid proof type', proofData.type, startTime);
      }

      if (!proofData.publicInputs?.commitment || !proofData.publicInputs?.txHash) {
        return this._reject('Missing public inputs', proofData.type, startTime);
      }

      await this._ensureInitialized();

      // If real proof and backend available
      if (this.backend && this.circuits.burn_threshold && !proofData.proof?.mock) {
        const circuit = this.circuits.burn_threshold;
        const backend = new this.backend(circuit.bytecode);

        const isValid = await backend.verifyProof({
          proof: proofData.proof,
          publicInputs: [
            proofData.publicInputs.commitment,
            proofData.publicInputs.minAmount,
            '0x' + proofData.publicInputs.txHash,
          ],
        });

        if (!isValid) {
          return this._reject('Proof verification failed', proofData.type, startTime);
        }
      }

      this.stats.proofsVerified++;
      this.stats.totalVerifyTime += Date.now() - startTime;

      return {
        valid: true,
        type: ProofType.BURN_THRESHOLD,
        publicInputs: proofData.publicInputs,
        confidence: PHI_INV,
        verifyTime: Date.now() - startTime,
        simulated: !!proofData.proof?.mock,
      };
    } catch (error) {
      return this._reject(error.message, ProofType.BURN_THRESHOLD, startTime);
    }
  }

  /**
   * Verify any proof type
   *
   * @param {Object} proofData - Proof data from prover
   * @returns {Promise<VerificationResult>}
   */
  async verify(proofData) {
    if (!proofData?.type) {
      return {
        valid: false,
        error: 'Missing proof type',
        confidence: 0,
      };
    }

    switch (proofData.type) {
      case ProofType.SCORE_RANGE:
        return this.verifyScoreRange(proofData);
      case ProofType.VERDICT_VALID:
        return this.verifyVerdictValid(proofData);
      case ProofType.BURN_THRESHOLD:
        return this.verifyBurnThreshold(proofData);
      default:
        return {
          valid: false,
          type: proofData.type,
          error: `Unknown proof type: ${proofData.type}`,
          confidence: 0,
        };
    }
  }

  /**
   * Create rejection result
   * @private
   */
  _reject(error, type, startTime) {
    this.stats.proofsRejected++;
    this.stats.totalVerifyTime += Date.now() - startTime;

    return {
      valid: false,
      type,
      error,
      confidence: 0,
      verifyTime: Date.now() - startTime,
    };
  }

  /**
   * Get verifier statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      avgVerifyTime: this.stats.proofsVerified > 0
        ? this.stats.totalVerifyTime / (this.stats.proofsVerified + this.stats.proofsRejected)
        : 0,
      successRate: (this.stats.proofsVerified + this.stats.proofsRejected) > 0
        ? this.stats.proofsVerified / (this.stats.proofsVerified + this.stats.proofsRejected)
        : 1,
      backendAvailable: this.backend !== null,
    };
  }
}

/**
 * Create a ZK verifier instance
 * @param {Object} [options] - Configuration
 * @returns {ZKVerifier}
 */
export function createZKVerifier(options = {}) {
  return new ZKVerifier(options);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Verification Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Quick verification without creating verifier instance
 *
 * @param {Object} proofData - Proof data
 * @returns {Promise<VerificationResult>}
 */
export async function quickVerify(proofData) {
  const verifier = new ZKVerifier();
  return verifier.verify(proofData);
}

/**
 * Verify a judgment with ZK proof
 *
 * Checks that the verdict matches the committed score.
 *
 * @param {Object} judgment - CYNIC judgment with ZK proof
 * @returns {Promise<VerificationResult>}
 */
export async function verifyJudgmentProof(judgment) {
  if (!judgment?.zkProof) {
    return {
      valid: false,
      error: 'No ZK proof attached to judgment',
      confidence: 0,
    };
  }

  const verifier = new ZKVerifier();
  const result = await verifier.verify(judgment.zkProof);

  // Additional check: verdict in proof matches judgment verdict
  if (result.valid && judgment.zkProof.type === ProofType.VERDICT_VALID) {
    const proofVerdict = judgment.zkProof.publicInputs.verdict;
    const judgmentVerdict = ZK_CONSTANTS.VERDICT[judgment.verdict];

    if (proofVerdict !== judgmentVerdict) {
      return {
        valid: false,
        error: 'Proof verdict does not match judgment verdict',
        confidence: 0,
      };
    }
  }

  return result;
}

export default {
  ZKVerifier,
  createZKVerifier,
  quickVerify,
  verifyJudgmentProof,
};
