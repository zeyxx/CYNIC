/**
 * @cynic/zk - ZK Proof Generator
 *
 * Generates zero-knowledge proofs for CYNIC judgments using Noir + Barretenberg.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/zk/prover
 */

'use strict';

import { createHash, randomBytes } from 'crypto';
import { PHI_INV } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * φ-aligned constants for ZK proofs
 */
export const ZK_CONSTANTS = {
  /** φ⁻¹ in lamports for burn threshold */
  PHI_INV_LAMPORTS: 618033988n,

  /** Verdict thresholds */
  THRESHOLD_HOWL: 76,
  THRESHOLD_WAG: 61,   // φ⁻¹ * 100
  THRESHOLD_GROWL: 38, // φ⁻² * 100

  /** Verdict values */
  VERDICT: {
    HOWL: 0,
    WAG: 1,
    GROWL: 2,
    BARK: 3,
  },

  /** Blinding factor size (bytes) */
  BLINDING_SIZE: 32,
};

/**
 * Proof types
 */
export const ProofType = {
  /** Prove score is within valid range */
  SCORE_RANGE: 'score_range',
  /** Prove verdict matches score */
  VERDICT_VALID: 'verdict_valid',
  /** Prove burn amount meets threshold */
  BURN_THRESHOLD: 'burn_threshold',
};

// ═══════════════════════════════════════════════════════════════════════════════
// Utility Functions
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Generate a cryptographically secure blinding factor
 * @returns {Buffer} Random 32-byte blinding factor
 */
export function generateBlinding() {
  return randomBytes(ZK_CONSTANTS.BLINDING_SIZE);
}

/**
 * Convert blinding to Field representation (hex string)
 * @param {Buffer} blinding - Blinding factor
 * @returns {string} Field representation
 */
export function blindingToField(blinding) {
  return '0x' + blinding.toString('hex');
}

/**
 * Compute Pedersen-style commitment (simulated without Noir)
 *
 * Note: This is a placeholder that uses SHA256.
 * Real implementation would use Noir's Pedersen hash.
 *
 * @param {bigint|number} value - Value to commit
 * @param {Buffer} blinding - Blinding factor
 * @returns {string} Commitment hash
 */
export function computeCommitment(value, blinding) {
  const hash = createHash('sha256');
  hash.update(Buffer.from(BigInt(value).toString(16).padStart(16, '0'), 'hex'));
  hash.update(blinding);
  return '0x' + hash.digest('hex');
}

/**
 * Determine correct verdict for a score
 * @param {number} score - Q-Score (0-100)
 * @returns {number} Verdict value
 */
export function getVerdictForScore(score) {
  if (score >= ZK_CONSTANTS.THRESHOLD_HOWL) return ZK_CONSTANTS.VERDICT.HOWL;
  if (score >= ZK_CONSTANTS.THRESHOLD_WAG) return ZK_CONSTANTS.VERDICT.WAG;
  if (score >= ZK_CONSTANTS.THRESHOLD_GROWL) return ZK_CONSTANTS.VERDICT.GROWL;
  return ZK_CONSTANTS.VERDICT.BARK;
}

/**
 * Get verdict name from value
 * @param {number} verdict - Verdict value
 * @returns {string} Verdict name
 */
export function getVerdictName(verdict) {
  const names = ['HOWL', 'WAG', 'GROWL', 'BARK'];
  return names[verdict] || 'UNKNOWN';
}

// ═══════════════════════════════════════════════════════════════════════════════
// ZK Proof Generator
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * ZK Proof Generator
 *
 * Generates zero-knowledge proofs for CYNIC judgments.
 * Uses Noir circuits compiled to ACIR and Barretenberg for proving.
 */
export class ZKProver {
  /**
   * @param {Object} [options] - Configuration
   * @param {Object} [options.circuits] - Pre-compiled circuit artifacts
   */
  constructor(options = {}) {
    this.circuits = options.circuits || {};
    this.noir = null;
    this.backend = null;

    // Stats
    this.stats = {
      proofsGenerated: 0,
      proofsFailed: 0,
      totalProveTime: 0,
    };
  }

  /**
   * Initialize Noir and Barretenberg (lazy load)
   * @private
   */
  async _ensureInitialized() {
    if (this.noir && this.backend) return;

    try {
      // Dynamic import to avoid bundling issues
      const { Noir } = await import('@noir-lang/noir_js');
      const { BarretenbergBackend } = await import('@aztec/bb.js');

      this.noir = Noir;
      this.backend = BarretenbergBackend;
    } catch (error) {
      // NoirJS not installed - use simulation mode
      console.warn('⚠️ NoirJS not available, using simulation mode');
      this.noir = null;
      this.backend = null;
    }
  }

  /**
   * Generate a score range proof
   *
   * Proves: score >= minScore AND score <= 100
   *
   * @param {number} score - The actual Q-Score (private)
   * @param {number} minScore - Minimum threshold to prove (public)
   * @param {Buffer} [blinding] - Optional blinding factor
   * @returns {Promise<Object>} Proof and public inputs
   */
  async proveScoreRange(score, minScore, blinding = null) {
    const blind = blinding || generateBlinding();
    const commitment = computeCommitment(score, blind);

    // Validate inputs
    if (score < 0 || score > 100) {
      throw new Error(`Invalid score: ${score}, must be 0-100`);
    }
    if (score < minScore) {
      throw new Error(`Cannot prove score >= ${minScore} when score is ${score}`);
    }

    const startTime = Date.now();

    try {
      await this._ensureInitialized();

      // If NoirJS is available, generate real proof
      if (this.noir && this.circuits.score_range) {
        const circuit = this.circuits.score_range;
        const noir = new this.noir(circuit);

        const witness = await noir.execute({
          score: score.toString(),
          commitment,
          min_score: minScore.toString(),
          blinding: blindingToField(blind),
        });

        const backend = new this.backend(circuit.bytecode);
        const proof = await backend.generateProof(witness);

        this.stats.proofsGenerated++;
        this.stats.totalProveTime += Date.now() - startTime;

        return {
          type: ProofType.SCORE_RANGE,
          proof: proof.proof,
          publicInputs: {
            commitment,
            minScore,
          },
          metadata: {
            proveTime: Date.now() - startTime,
            circuitSize: circuit.bytecode?.length || 0,
          },
        };
      }

      // Simulation mode - return mock proof
      this.stats.proofsGenerated++;
      this.stats.totalProveTime += Date.now() - startTime;

      return {
        type: ProofType.SCORE_RANGE,
        proof: this._generateMockProof('score_range', { score, minScore }),
        publicInputs: {
          commitment,
          minScore,
        },
        metadata: {
          proveTime: Date.now() - startTime,
          simulated: true,
        },
        _private: { blinding: blind }, // Keep for verification
      };
    } catch (error) {
      this.stats.proofsFailed++;
      throw error;
    }
  }

  /**
   * Generate a verdict validity proof
   *
   * Proves: verdict is correct for the committed score
   *
   * @param {number} score - The actual Q-Score (private)
   * @param {number} verdict - The claimed verdict (public)
   * @param {Buffer} [blinding] - Optional blinding factor
   * @returns {Promise<Object>} Proof and public inputs
   */
  async proveVerdictValid(score, verdict, blinding = null) {
    const blind = blinding || generateBlinding();
    const commitment = computeCommitment(score, blind);

    // Validate inputs
    if (score < 0 || score > 100) {
      throw new Error(`Invalid score: ${score}, must be 0-100`);
    }

    const expectedVerdict = getVerdictForScore(score);
    if (verdict !== expectedVerdict) {
      throw new Error(
        `Cannot prove verdict ${getVerdictName(verdict)} for score ${score}. ` +
        `Expected verdict: ${getVerdictName(expectedVerdict)}`
      );
    }

    const startTime = Date.now();

    try {
      await this._ensureInitialized();

      // If NoirJS is available, generate real proof
      if (this.noir && this.circuits.verdict_valid) {
        const circuit = this.circuits.verdict_valid;
        const noir = new this.noir(circuit);

        const witness = await noir.execute({
          score: score.toString(),
          blinding: blindingToField(blind),
          commitment,
          verdict: verdict.toString(),
        });

        const backend = new this.backend(circuit.bytecode);
        const proof = await backend.generateProof(witness);

        this.stats.proofsGenerated++;
        this.stats.totalProveTime += Date.now() - startTime;

        return {
          type: ProofType.VERDICT_VALID,
          proof: proof.proof,
          publicInputs: {
            commitment,
            verdict,
            verdictName: getVerdictName(verdict),
          },
          metadata: {
            proveTime: Date.now() - startTime,
          },
        };
      }

      // Simulation mode
      this.stats.proofsGenerated++;
      this.stats.totalProveTime += Date.now() - startTime;

      return {
        type: ProofType.VERDICT_VALID,
        proof: this._generateMockProof('verdict_valid', { score, verdict }),
        publicInputs: {
          commitment,
          verdict,
          verdictName: getVerdictName(verdict),
        },
        metadata: {
          proveTime: Date.now() - startTime,
          simulated: true,
        },
        _private: { blinding: blind },
      };
    } catch (error) {
      this.stats.proofsFailed++;
      throw error;
    }
  }

  /**
   * Generate a burn threshold proof
   *
   * Proves: burnAmount >= minAmount
   *
   * @param {bigint|number} amount - Actual burn amount in lamports (private)
   * @param {bigint|number} minAmount - Minimum threshold (public)
   * @param {string} txHash - Transaction hash (public, for binding)
   * @param {Buffer} [blinding] - Optional blinding factor
   * @returns {Promise<Object>} Proof and public inputs
   */
  async proveBurnThreshold(amount, minAmount, txHash, blinding = null) {
    const amountBig = BigInt(amount);
    const minBig = BigInt(minAmount);
    const blind = blinding || generateBlinding();

    // Include txHash in commitment
    const hash = createHash('sha256');
    hash.update(Buffer.from(amountBig.toString(16).padStart(16, '0'), 'hex'));
    hash.update(blind);
    hash.update(Buffer.from(txHash, 'hex'));
    const commitment = '0x' + hash.digest('hex');

    // Validate
    if (amountBig < minBig) {
      throw new Error(`Cannot prove amount >= ${minBig} when amount is ${amountBig}`);
    }

    const startTime = Date.now();

    try {
      await this._ensureInitialized();

      // If NoirJS is available, generate real proof
      if (this.noir && this.circuits.burn_threshold) {
        const circuit = this.circuits.burn_threshold;
        const noir = new this.noir(circuit);

        const witness = await noir.execute({
          amount: amountBig.toString(),
          blinding: blindingToField(blind),
          commitment,
          min_amount: minBig.toString(),
          tx_hash: '0x' + txHash,
        });

        const backend = new this.backend(circuit.bytecode);
        const proof = await backend.generateProof(witness);

        this.stats.proofsGenerated++;
        this.stats.totalProveTime += Date.now() - startTime;

        return {
          type: ProofType.BURN_THRESHOLD,
          proof: proof.proof,
          publicInputs: {
            commitment,
            minAmount: minBig.toString(),
            txHash,
          },
          metadata: {
            proveTime: Date.now() - startTime,
          },
        };
      }

      // Simulation mode
      this.stats.proofsGenerated++;
      this.stats.totalProveTime += Date.now() - startTime;

      return {
        type: ProofType.BURN_THRESHOLD,
        proof: this._generateMockProof('burn_threshold', { amount: amountBig, minAmount: minBig }),
        publicInputs: {
          commitment,
          minAmount: minBig.toString(),
          txHash,
        },
        metadata: {
          proveTime: Date.now() - startTime,
          simulated: true,
        },
        _private: { blinding: blind },
      };
    } catch (error) {
      this.stats.proofsFailed++;
      throw error;
    }
  }

  /**
   * Generate a mock proof for simulation mode
   * @private
   */
  _generateMockProof(circuit, inputs) {
    const hash = createHash('sha256');
    hash.update(circuit);
    // Convert BigInt to string for JSON serialization
    hash.update(JSON.stringify(inputs, (_, v) =>
      typeof v === 'bigint' ? v.toString() : v
    ));
    hash.update(Date.now().toString());
    return {
      mock: true,
      circuit,
      hash: hash.digest('hex'),
      confidence: PHI_INV, // Max confidence
    };
  }

  /**
   * Get prover statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      avgProveTime: this.stats.proofsGenerated > 0
        ? this.stats.totalProveTime / this.stats.proofsGenerated
        : 0,
      successRate: this.stats.proofsGenerated > 0
        ? this.stats.proofsGenerated / (this.stats.proofsGenerated + this.stats.proofsFailed)
        : 1,
      noirAvailable: this.noir !== null,
    };
  }
}

/**
 * Create a ZK prover instance
 * @param {Object} [options] - Configuration
 * @returns {ZKProver}
 */
export function createZKProver(options = {}) {
  return new ZKProver(options);
}

export default {
  ZK_CONSTANTS,
  ProofType,
  generateBlinding,
  blindingToField,
  computeCommitment,
  getVerdictForScore,
  getVerdictName,
  ZKProver,
  createZKProver,
};
