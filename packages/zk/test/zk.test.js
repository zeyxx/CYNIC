/**
 * @cynic/zk Tests
 *
 * Tests for ZK proof generation and verification.
 *
 * @module @cynic/zk/test
 */

import { test, describe } from 'node:test';
import assert from 'node:assert';

import {
  ZK_CONSTANTS,
  ProofType,
  generateBlinding,
  blindingToField,
  computeCommitment,
  getVerdictForScore,
  getVerdictName,
  ZKProver,
  createZKProver,
  ZKVerifier,
  createZKVerifier,
  quickVerify,
  verifyJudgmentProof,
} from '../src/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('ZK_CONSTANTS', () => {
  test('should have φ-aligned thresholds', () => {
    assert.strictEqual(ZK_CONSTANTS.THRESHOLD_HOWL, 76);
    assert.strictEqual(ZK_CONSTANTS.THRESHOLD_WAG, 61);  // φ⁻¹ * 100
    assert.strictEqual(ZK_CONSTANTS.THRESHOLD_GROWL, 38); // φ⁻² * 100
  });

  test('should have all verdict values', () => {
    assert.strictEqual(ZK_CONSTANTS.VERDICT.HOWL, 0);
    assert.strictEqual(ZK_CONSTANTS.VERDICT.WAG, 1);
    assert.strictEqual(ZK_CONSTANTS.VERDICT.GROWL, 2);
    assert.strictEqual(ZK_CONSTANTS.VERDICT.BARK, 3);
  });

  test('should have φ⁻¹ lamports', () => {
    assert.ok(ZK_CONSTANTS.PHI_INV_LAMPORTS > 600000000n);
    assert.ok(ZK_CONSTANTS.PHI_INV_LAMPORTS < 700000000n);
  });
});

describe('ProofType', () => {
  test('should have all proof types', () => {
    assert.strictEqual(ProofType.SCORE_RANGE, 'score_range');
    assert.strictEqual(ProofType.VERDICT_VALID, 'verdict_valid');
    assert.strictEqual(ProofType.BURN_THRESHOLD, 'burn_threshold');
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('generateBlinding', () => {
  test('should generate 32-byte buffer', () => {
    const blinding = generateBlinding();
    assert.ok(Buffer.isBuffer(blinding));
    assert.strictEqual(blinding.length, 32);
  });

  test('should generate unique values', () => {
    const b1 = generateBlinding();
    const b2 = generateBlinding();
    assert.ok(!b1.equals(b2));
  });
});

describe('blindingToField', () => {
  test('should convert to hex with 0x prefix', () => {
    const blinding = Buffer.from('a'.repeat(64), 'hex');
    const field = blindingToField(blinding);
    assert.ok(field.startsWith('0x'));
    assert.strictEqual(field.length, 66); // 0x + 64 hex chars
  });
});

describe('computeCommitment', () => {
  test('should produce consistent commitments', () => {
    const blinding = generateBlinding();
    const c1 = computeCommitment(75, blinding);
    const c2 = computeCommitment(75, blinding);
    assert.strictEqual(c1, c2);
  });

  test('should produce different commitments for different values', () => {
    const blinding = generateBlinding();
    const c1 = computeCommitment(75, blinding);
    const c2 = computeCommitment(76, blinding);
    assert.notStrictEqual(c1, c2);
  });

  test('should produce different commitments for different blindings', () => {
    const b1 = generateBlinding();
    const b2 = generateBlinding();
    const c1 = computeCommitment(75, b1);
    const c2 = computeCommitment(75, b2);
    assert.notStrictEqual(c1, c2);
  });
});

describe('getVerdictForScore', () => {
  test('should return HOWL for score >= 76', () => {
    assert.strictEqual(getVerdictForScore(76), ZK_CONSTANTS.VERDICT.HOWL);
    assert.strictEqual(getVerdictForScore(100), ZK_CONSTANTS.VERDICT.HOWL);
  });

  test('should return WAG for score >= 61 and < 76', () => {
    assert.strictEqual(getVerdictForScore(61), ZK_CONSTANTS.VERDICT.WAG);
    assert.strictEqual(getVerdictForScore(75), ZK_CONSTANTS.VERDICT.WAG);
  });

  test('should return GROWL for score >= 38 and < 61', () => {
    assert.strictEqual(getVerdictForScore(38), ZK_CONSTANTS.VERDICT.GROWL);
    assert.strictEqual(getVerdictForScore(60), ZK_CONSTANTS.VERDICT.GROWL);
  });

  test('should return BARK for score < 38', () => {
    assert.strictEqual(getVerdictForScore(0), ZK_CONSTANTS.VERDICT.BARK);
    assert.strictEqual(getVerdictForScore(37), ZK_CONSTANTS.VERDICT.BARK);
  });
});

describe('getVerdictName', () => {
  test('should return correct names', () => {
    assert.strictEqual(getVerdictName(0), 'HOWL');
    assert.strictEqual(getVerdictName(1), 'WAG');
    assert.strictEqual(getVerdictName(2), 'GROWL');
    assert.strictEqual(getVerdictName(3), 'BARK');
  });

  test('should return UNKNOWN for invalid verdict', () => {
    assert.strictEqual(getVerdictName(4), 'UNKNOWN');
    assert.strictEqual(getVerdictName(-1), 'UNKNOWN');
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// PROVER TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('ZKProver', () => {
  describe('constructor', () => {
    test('should create prover with default config', () => {
      const prover = new ZKProver();
      assert.ok(prover);
      assert.deepStrictEqual(prover.circuits, {});
    });

    test('should accept circuit artifacts', () => {
      const circuits = { score_range: {} };
      const prover = new ZKProver({ circuits });
      assert.deepStrictEqual(prover.circuits, circuits);
    });
  });

  describe('proveScoreRange', () => {
    test('should generate score range proof', async () => {
      const prover = createZKProver();
      const proof = await prover.proveScoreRange(75, 61);

      assert.strictEqual(proof.type, ProofType.SCORE_RANGE);
      assert.ok(proof.publicInputs.commitment);
      assert.strictEqual(proof.publicInputs.minScore, 61);
      assert.ok(proof.proof);
      assert.ok(proof.metadata.proveTime >= 0);
    });

    test('should reject invalid score (> 100)', async () => {
      const prover = createZKProver();
      await assert.rejects(
        () => prover.proveScoreRange(101, 61),
        /Invalid score/
      );
    });

    test('should reject if score < minScore', async () => {
      const prover = createZKProver();
      await assert.rejects(
        () => prover.proveScoreRange(50, 61),
        /Cannot prove/
      );
    });

    test('should use provided blinding', async () => {
      const prover = createZKProver();
      const blinding = generateBlinding();
      const proof = await prover.proveScoreRange(75, 61, blinding);

      assert.ok(proof._private.blinding.equals(blinding));
    });
  });

  describe('proveVerdictValid', () => {
    test('should generate verdict proof for HOWL', async () => {
      const prover = createZKProver();
      const proof = await prover.proveVerdictValid(80, ZK_CONSTANTS.VERDICT.HOWL);

      assert.strictEqual(proof.type, ProofType.VERDICT_VALID);
      assert.strictEqual(proof.publicInputs.verdict, ZK_CONSTANTS.VERDICT.HOWL);
      assert.strictEqual(proof.publicInputs.verdictName, 'HOWL');
    });

    test('should generate verdict proof for WAG', async () => {
      const prover = createZKProver();
      const proof = await prover.proveVerdictValid(65, ZK_CONSTANTS.VERDICT.WAG);

      assert.strictEqual(proof.publicInputs.verdict, ZK_CONSTANTS.VERDICT.WAG);
      assert.strictEqual(proof.publicInputs.verdictName, 'WAG');
    });

    test('should generate verdict proof for GROWL', async () => {
      const prover = createZKProver();
      const proof = await prover.proveVerdictValid(45, ZK_CONSTANTS.VERDICT.GROWL);

      assert.strictEqual(proof.publicInputs.verdict, ZK_CONSTANTS.VERDICT.GROWL);
    });

    test('should generate verdict proof for BARK', async () => {
      const prover = createZKProver();
      const proof = await prover.proveVerdictValid(20, ZK_CONSTANTS.VERDICT.BARK);

      assert.strictEqual(proof.publicInputs.verdict, ZK_CONSTANTS.VERDICT.BARK);
    });

    test('should reject incorrect verdict', async () => {
      const prover = createZKProver();
      // Score 50 should be GROWL, not WAG
      await assert.rejects(
        () => prover.proveVerdictValid(50, ZK_CONSTANTS.VERDICT.WAG),
        /Cannot prove verdict/
      );
    });
  });

  describe('proveBurnThreshold', () => {
    test('should generate burn threshold proof', async () => {
      const prover = createZKProver();
      const amount = 1000000000n; // 1 SOL
      const minAmount = ZK_CONSTANTS.PHI_INV_LAMPORTS;
      const txHash = 'a'.repeat(64);

      const proof = await prover.proveBurnThreshold(amount, minAmount, txHash);

      assert.strictEqual(proof.type, ProofType.BURN_THRESHOLD);
      assert.ok(proof.publicInputs.commitment);
      assert.strictEqual(proof.publicInputs.txHash, txHash);
    });

    test('should reject if amount < minAmount', async () => {
      const prover = createZKProver();
      const amount = 100000000n; // 0.1 SOL
      const minAmount = ZK_CONSTANTS.PHI_INV_LAMPORTS;
      const txHash = 'a'.repeat(64);

      await assert.rejects(
        () => prover.proveBurnThreshold(amount, minAmount, txHash),
        /Cannot prove/
      );
    });
  });

  describe('getStats', () => {
    test('should track statistics', async () => {
      const prover = createZKProver();
      await prover.proveScoreRange(75, 61);
      await prover.proveScoreRange(80, 70);

      const stats = prover.getStats();
      assert.strictEqual(stats.proofsGenerated, 2);
      assert.strictEqual(stats.proofsFailed, 0);
      assert.ok(stats.avgProveTime >= 0);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// VERIFIER TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('ZKVerifier', () => {
  describe('verifyScoreRange', () => {
    test('should verify valid score range proof', async () => {
      const prover = createZKProver();
      const verifier = createZKVerifier();

      const proof = await prover.proveScoreRange(75, 61);
      const result = await verifier.verifyScoreRange(proof);

      assert.strictEqual(result.valid, true);
      assert.strictEqual(result.type, ProofType.SCORE_RANGE);
      assert.ok(result.confidence > 0);
    });

    test('should reject invalid proof type', async () => {
      const verifier = createZKVerifier();

      const result = await verifier.verifyScoreRange({
        type: 'wrong_type',
        publicInputs: { commitment: '0x123', minScore: 61 },
        proof: { mock: true },
      });

      assert.strictEqual(result.valid, false);
      assert.ok(result.error.includes('Invalid proof type'));
    });

    test('should reject missing public inputs', async () => {
      const verifier = createZKVerifier();

      const result = await verifier.verifyScoreRange({
        type: ProofType.SCORE_RANGE,
        publicInputs: {},
        proof: { mock: true },
      });

      assert.strictEqual(result.valid, false);
    });
  });

  describe('verifyVerdictValid', () => {
    test('should verify valid verdict proof', async () => {
      const prover = createZKProver();
      const verifier = createZKVerifier();

      const proof = await prover.proveVerdictValid(80, ZK_CONSTANTS.VERDICT.HOWL);
      const result = await verifier.verifyVerdictValid(proof);

      assert.strictEqual(result.valid, true);
      assert.strictEqual(result.type, ProofType.VERDICT_VALID);
    });

    test('should reject invalid verdict value', async () => {
      const verifier = createZKVerifier();

      const result = await verifier.verifyVerdictValid({
        type: ProofType.VERDICT_VALID,
        publicInputs: { commitment: '0x123', verdict: 5 },
        proof: { mock: true },
      });

      assert.strictEqual(result.valid, false);
    });
  });

  describe('verifyBurnThreshold', () => {
    test('should verify valid burn proof', async () => {
      const prover = createZKProver();
      const verifier = createZKVerifier();

      const proof = await prover.proveBurnThreshold(
        1000000000n,
        ZK_CONSTANTS.PHI_INV_LAMPORTS,
        'a'.repeat(64)
      );
      const result = await verifier.verifyBurnThreshold(proof);

      assert.strictEqual(result.valid, true);
      assert.strictEqual(result.type, ProofType.BURN_THRESHOLD);
    });
  });

  describe('verify (generic)', () => {
    test('should route to correct verifier', async () => {
      const prover = createZKProver();
      const verifier = createZKVerifier();

      const proof1 = await prover.proveScoreRange(75, 61);
      const proof2 = await prover.proveVerdictValid(80, ZK_CONSTANTS.VERDICT.HOWL);

      const result1 = await verifier.verify(proof1);
      const result2 = await verifier.verify(proof2);

      assert.strictEqual(result1.type, ProofType.SCORE_RANGE);
      assert.strictEqual(result2.type, ProofType.VERDICT_VALID);
    });

    test('should reject unknown proof type', async () => {
      const verifier = createZKVerifier();

      const result = await verifier.verify({
        type: 'unknown_type',
        publicInputs: {},
        proof: {},
      });

      assert.strictEqual(result.valid, false);
      assert.ok(result.error.includes('Unknown proof type'));
    });

    test('should handle missing type', async () => {
      const verifier = createZKVerifier();

      const result = await verifier.verify({});

      assert.strictEqual(result.valid, false);
    });
  });

  describe('getStats', () => {
    test('should track verification statistics', async () => {
      const prover = createZKProver();
      const verifier = createZKVerifier();

      const proof = await prover.proveScoreRange(75, 61);
      await verifier.verify(proof);
      await verifier.verify(proof);

      const stats = verifier.getStats();
      assert.strictEqual(stats.proofsVerified, 2);
      assert.ok(stats.avgVerifyTime >= 0);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// HELPER FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('quickVerify', () => {
  test('should verify proof without creating verifier', async () => {
    const prover = createZKProver();
    const proof = await prover.proveScoreRange(75, 61);

    const result = await quickVerify(proof);
    assert.strictEqual(result.valid, true);
  });
});

describe('verifyJudgmentProof', () => {
  test('should verify judgment with ZK proof', async () => {
    const prover = createZKProver();
    const zkProof = await prover.proveVerdictValid(80, ZK_CONSTANTS.VERDICT.HOWL);

    const judgment = {
      qScore: 80,
      verdict: 'HOWL',
      zkProof,
    };

    const result = await verifyJudgmentProof(judgment);
    assert.strictEqual(result.valid, true);
  });

  test('should reject judgment without ZK proof', async () => {
    const judgment = {
      qScore: 80,
      verdict: 'HOWL',
    };

    const result = await verifyJudgmentProof(judgment);
    assert.strictEqual(result.valid, false);
    assert.ok(result.error.includes('No ZK proof'));
  });

  test('should reject mismatched verdict', async () => {
    const prover = createZKProver();
    const zkProof = await prover.proveVerdictValid(80, ZK_CONSTANTS.VERDICT.HOWL);

    const judgment = {
      qScore: 80,
      verdict: 'WAG', // Mismatch!
      zkProof,
    };

    const result = await verifyJudgmentProof(judgment);
    assert.strictEqual(result.valid, false);
    assert.ok(result.error.includes('does not match'));
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// INTEGRATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Integration', () => {
  test('full proof lifecycle: generate, verify, stats', async () => {
    const prover = createZKProver();
    const verifier = createZKVerifier();

    // Generate multiple proofs
    const proofs = [
      await prover.proveScoreRange(85, 76),
      await prover.proveVerdictValid(65, ZK_CONSTANTS.VERDICT.WAG),
      await prover.proveBurnThreshold(1000000000n, 100000000n, 'b'.repeat(64)),
    ];

    // Verify all
    for (const proof of proofs) {
      const result = await verifier.verify(proof);
      assert.strictEqual(result.valid, true);
    }

    // Check stats
    const proverStats = prover.getStats();
    const verifierStats = verifier.getStats();

    assert.strictEqual(proverStats.proofsGenerated, 3);
    assert.strictEqual(verifierStats.proofsVerified, 3);
  });

  test('should handle concurrent proof generation', async () => {
    const prover = createZKProver();

    const proofs = await Promise.all([
      prover.proveScoreRange(70, 61),
      prover.proveScoreRange(80, 76),
      prover.proveScoreRange(90, 76),
    ]);

    assert.strictEqual(proofs.length, 3);
    assert.ok(proofs.every(p => p.type === ProofType.SCORE_RANGE));
  });
});
