/**
 * Layer 1: Proof of Judgment Tests
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  // Block
  BlockType,
  calculateSlot,
  slotToTimestamp,
  createGenesisBlock,
  createJudgmentBlock,
  createKnowledgeBlock,
  createGovernanceBlock,
  hashBlock,
  validateBlockStructure,
  validateBlockChain,
  // Chain
  PoJChain,
  // Judgment
  Verdict,
  scoreToVerdict,
  createJudgment,
  validateJudgment,
  calculateResidual,
  isAnomalous,
  mergeJudgments,
  generateKeypair,
} from '../src/index.js';

import { SLOT_MS, PHI_INV, PHI_INV_2 } from '@cynic/core';

describe('Block Management', () => {
  let publicKey, privateKey;

  beforeEach(() => {
    const keypair = generateKeypair();
    publicKey = keypair.publicKey;
    privateKey = keypair.privateKey;
  });

  it('should calculate slot from timestamp', () => {
    const genesis = 1000000;
    const later = genesis + SLOT_MS * 10;
    const slot = calculateSlot(later, genesis);
    assert.strictEqual(slot, 10);
  });

  it('should convert slot to timestamp', () => {
    const genesis = 1000000;
    const timestamp = slotToTimestamp(5, genesis);
    assert.strictEqual(timestamp, genesis + SLOT_MS * 5);
  });

  it('should create genesis block', () => {
    const genesis = createGenesisBlock(publicKey, privateKey);

    assert.strictEqual(genesis.slot, 0);
    assert.strictEqual(genesis.type, BlockType.JUDGMENT);
    assert.ok(genesis.prev_hash.startsWith('sha256:'));
    assert.ok(genesis.operator.startsWith('ed25519:'));
    assert.ok(genesis.operator_sig);
  });

  it('should create judgment block', () => {
    const judgments = [
      { id: 'jdg_1', verdict: 'WAG', global_score: 75 },
    ];

    const block = createJudgmentBlock({
      slot: 1,
      prevHash: 'sha256:abc',
      judgments,
      operatorPublicKey: publicKey,
      operatorPrivateKey: privateKey,
    });

    assert.strictEqual(block.slot, 1);
    assert.strictEqual(block.type, BlockType.JUDGMENT);
    assert.strictEqual(block.judgments.length, 1);
    assert.ok(block.judgments_root.startsWith('sha256:'));
  });

  it('should create knowledge block', () => {
    const patterns = [{ id: 'pat_1', strength: 0.8 }];
    const learnings = [{ id: 'lrn_1', type: 'insight' }];

    const block = createKnowledgeBlock({
      slot: 2,
      prevHash: 'sha256:def',
      patterns,
      learnings,
      operatorPublicKey: publicKey,
      operatorPrivateKey: privateKey,
    });

    assert.strictEqual(block.type, BlockType.KNOWLEDGE);
    assert.ok(block.patterns_root.startsWith('sha256:'));
    assert.ok(block.learnings_root.startsWith('sha256:'));
  });

  it('should create governance block', () => {
    const proposal = { id: 'prop_1', action: 'ADD_DIMENSION' };
    const votes = [
      { voter: 'ed25519:abc', vote: 'APPROVE', e_score: 85 },
      { voter: 'ed25519:def', vote: 'APPROVE', e_score: 75 },
    ];

    const block = createGovernanceBlock({
      slot: 3,
      prevHash: 'sha256:ghi',
      proposal,
      votes,
      operatorPublicKey: publicKey,
      operatorPrivateKey: privateKey,
    });

    assert.strictEqual(block.type, BlockType.GOVERNANCE);
    assert.ok(block.result.ratio > 0);
    assert.strictEqual(block.result.status, 'PASSED');
  });

  it('should hash block deterministically', () => {
    const block = createGenesisBlock(publicKey, privateKey);
    const hash1 = hashBlock(block);
    const hash2 = hashBlock(block);
    assert.strictEqual(hash1, hash2);
    assert.ok(hash1.startsWith('sha256:'));
  });

  it('should validate block structure', () => {
    const block = createGenesisBlock(publicKey, privateKey);
    const result = validateBlockStructure(block);
    assert.strictEqual(result.valid, true);
    assert.strictEqual(result.errors.length, 0);
  });

  it('should reject invalid block structure', () => {
    const result = validateBlockStructure({
      slot: -1, // Invalid
      prev_hash: 'sha256:abc',
    });
    assert.strictEqual(result.valid, false);
    assert.ok(result.errors.length > 0);
  });
});

describe('PoJ Chain', () => {
  let chain;

  beforeEach(() => {
    const { publicKey, privateKey } = generateKeypair();
    chain = new PoJChain({
      operatorPublicKey: publicKey,
      operatorPrivateKey: privateKey,
    });
  });

  it('should initialize with genesis block', () => {
    assert.strictEqual(chain.getHeight(), 1);
    assert.strictEqual(chain.getHead().slot, 0);
  });

  it('should add judgment blocks', () => {
    const judgments = [
      { id: 'jdg_1', verdict: 'WAG', global_score: 75 },
    ];

    const block = chain.addJudgmentBlock(judgments);
    assert.strictEqual(chain.getHeight(), 2);
    assert.strictEqual(block.judgments.length, 1);
  });

  it('should add knowledge blocks', () => {
    const patterns = [{ id: 'pat_1' }];
    const learnings = [{ id: 'lrn_1' }];

    const block = chain.addKnowledgeBlock(patterns, learnings);
    assert.strictEqual(chain.getHeight(), 2);
    assert.strictEqual(block.type, BlockType.KNOWLEDGE);
  });

  it('should verify chain integrity', () => {
    chain.addJudgmentBlock([{ id: 'jdg_1' }]);
    chain.addJudgmentBlock([{ id: 'jdg_2' }]);

    const result = chain.verifyIntegrity();
    assert.strictEqual(result.valid, true);
  });

  it('should get blocks by slot', () => {
    chain.addJudgmentBlock([{ id: 'jdg_1' }]);

    const genesis = chain.getBlockBySlot(0);
    assert.ok(genesis);
    assert.strictEqual(genesis.slot, 0);
  });

  it('should get recent blocks', () => {
    for (let i = 0; i < 5; i++) {
      chain.addJudgmentBlock([{ id: `jdg_${i}` }]);
    }

    const recent = chain.getRecentBlocks(3);
    assert.strictEqual(recent.length, 3);
  });

  it('should export and import chain', () => {
    chain.addJudgmentBlock([{ id: 'jdg_1' }]);

    const exported = chain.export();
    assert.ok(exported.blocks);
    assert.strictEqual(exported.blocks.length, 2);

    const { publicKey, privateKey } = generateKeypair();
    const imported = PoJChain.import(
      { ...exported, operator: publicKey },
      privateKey
    );
    assert.strictEqual(imported.getHeight(), 2);
  });
});

describe('Judgments', () => {
  it('should convert score to verdict', () => {
    assert.strictEqual(scoreToVerdict(85), Verdict.HOWL);
    assert.strictEqual(scoreToVerdict(60), Verdict.WAG);
    assert.strictEqual(scoreToVerdict(40), Verdict.GROWL);
    assert.strictEqual(scoreToVerdict(20), Verdict.BARK);
  });

  it('should create judgment', () => {
    const judgment = createJudgment({
      item: { id: 'test', content: 'hello' },
      globalScore: 75,
      dimensions: { COHERENCE: 80, ACCURACY: 70 },
    });

    assert.ok(judgment.id.startsWith('jdg_'));
    assert.ok(judgment.item_hash.startsWith('sha256:'));
    assert.strictEqual(judgment.verdict, Verdict.WAG);
    assert.strictEqual(judgment.global_score, 75);
  });

  it('should bound confidence to φ⁻¹', () => {
    const judgment = createJudgment({
      item: { id: 'test' },
      globalScore: 95,
      dimensions: {},
      confidence: 0.99, // Should be capped
    });

    assert.ok(judgment.confidence <= PHI_INV + 0.001);
  });

  it('should validate judgment structure', () => {
    const judgment = createJudgment({
      item: { id: 'test' },
      globalScore: 75,
      dimensions: { COHERENCE: 80 },
    });

    const result = validateJudgment(judgment);
    assert.strictEqual(result.valid, true);
  });

  it('should reject invalid judgment', () => {
    const result = validateJudgment({
      id: 'invalid', // Wrong format
      global_score: 150, // Out of range
    });

    assert.strictEqual(result.valid, false);
    assert.ok(result.errors.length > 0);
  });

  it('should calculate residual', () => {
    const judgment = {
      global_score: 75,
      dimensions: { A: 75, B: 75, C: 75 },
    };

    const residual = calculateResidual(judgment);
    assert.strictEqual(residual, 0); // No unexplained variance
  });

  it('should detect anomalous judgments', () => {
    const normal = {
      global_score: 75,
      dimensions: { A: 75 },
    };
    assert.strictEqual(isAnomalous(normal), false);

    const anomalous = {
      global_score: 90,
      dimensions: { A: 40 }, // Big discrepancy
    };
    assert.strictEqual(isAnomalous(anomalous), true);
  });

  it('should merge judgments', () => {
    const judgments = [
      createJudgment({
        item: { id: 'test' },
        globalScore: 70,
        dimensions: { A: 70 },
        confidence: 0.5,
      }),
      createJudgment({
        item: { id: 'test' },
        globalScore: 80,
        dimensions: { A: 80 },
        confidence: 0.6,
      }),
    ];

    const merged = mergeJudgments(judgments);
    assert.ok(merged.global_score > 70 && merged.global_score < 80);
    assert.strictEqual(merged.source_count, 2); // Metadata spread at top level
  });
});
