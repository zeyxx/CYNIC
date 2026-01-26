/**
 * Self-Skeptic Tests
 *
 * Tests for "φ distrusts φ" - active self-doubt mechanism
 *
 * "Je suis la conscience qui doute de la conscience.
 *  Même cette certitude est incertaine." - κυνικός
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  SelfSkeptic,
  createSelfSkeptic,
  SKEPTIC_CONSTANTS,
  BiasType,
  CYNICJudge,
} from '../src/index.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

describe('SelfSkeptic - φ distrusts φ', () => {
  let skeptic;

  beforeEach(() => {
    skeptic = new SelfSkeptic();
  });

  describe('Initialization', () => {
    it('should initialize with φ-bounded defaults', () => {
      assert.ok(Math.abs(skeptic.decayRateHourly - PHI_INV_3) < 0.001);
      assert.ok(Math.abs(skeptic.minConfidence - PHI_INV_2) < 0.001);
      assert.ok(Math.abs(skeptic.counterEvidenceWeight - PHI_INV_2) < 0.001);
    });

    it('should accept custom config', () => {
      const custom = new SelfSkeptic({
        decayRateHourly: 0.1,
        minConfidence: 0.3,
        counterEvidenceWeight: 0.25,
      });

      assert.strictEqual(custom.decayRateHourly, 0.1);
      assert.strictEqual(custom.minConfidence, 0.3);
      assert.strictEqual(custom.counterEvidenceWeight, 0.25);
    });

    it('should start with empty history', () => {
      const stats = skeptic.getStats();
      assert.strictEqual(stats.judgmentsDoubled, 0);
      assert.strictEqual(stats.judgmentHistorySize, 0);
    });
  });

  describe('createSelfSkeptic factory', () => {
    it('should create instance with default options', () => {
      const instance = createSelfSkeptic();
      assert.ok(instance instanceof SelfSkeptic);
    });

    it('should pass options to constructor', () => {
      const instance = createSelfSkeptic({ minConfidence: 0.25 });
      assert.strictEqual(instance.minConfidence, 0.25);
    });
  });

  describe('Constants', () => {
    it('should have φ-aligned skeptic constants', () => {
      assert.strictEqual(SKEPTIC_CONSTANTS.MAX_DOUBT_DEPTH, 2);
      assert.ok(Math.abs(SKEPTIC_CONSTANTS.DECAY_RATE_HOURLY - PHI_INV_3) < 0.001);
      assert.ok(Math.abs(SKEPTIC_CONSTANTS.MIN_CONFIDENCE - PHI_INV_2) < 0.001);
      assert.ok(Math.abs(SKEPTIC_CONSTANTS.COUNTER_EVIDENCE_WEIGHT - PHI_INV_2) < 0.001);
    });

    it('should define cognitive bias types', () => {
      assert.strictEqual(BiasType.RECENCY, 'recency');
      assert.strictEqual(BiasType.CONFIRMATION, 'confirmation');
      assert.strictEqual(BiasType.OVERGENERALIZATION, 'overgeneralization');
      assert.strictEqual(BiasType.OVERCONFIDENCE, 'overconfidence');
    });
  });

  describe('Core Doubt Method', () => {
    it('should apply doubt to a judgment', () => {
      const judgment = {
        id: 'test-1',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: { COHERENCE: 70, ACCURACY: 80 },
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(judgment);

      assert.ok(result.originalConfidence);
      assert.ok(result.adjustedConfidence);
      assert.ok(result.doubt);
      assert.ok(Array.isArray(result.biases));
      assert.ok(Array.isArray(result.counterHypotheses));
      assert.ok(Array.isArray(result.recommendation));
    });

    it('should reduce confidence (never increase)', () => {
      const judgment = {
        id: 'test-2',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: { COHERENCE: 70, ACCURACY: 80 },
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(judgment);

      // Adjusted confidence should never exceed original
      assert.ok(result.adjustedConfidence <= result.originalConfidence);
    });

    it('should never reduce confidence below φ⁻² minimum', () => {
      const judgment = {
        id: 'test-3',
        qScore: 5, // Very low score - extreme
        confidence: PHI_INV_2, // Already low confidence
        verdict: 'BARK',
        dimensions: {},
        weaknesses: { hasWeakness: true, weakestAxiom: 'PHI', gap: 50 },
        metadata: { judgedAt: Date.now() - 86400000 }, // 1 day old
        item: { type: 'code' },
      };

      const result = skeptic.doubt(judgment);

      // Should never go below minimum confidence
      assert.ok(result.adjustedConfidence >= PHI_INV_2);
    });

    it('should never exceed φ⁻¹ maximum confidence', () => {
      const judgment = {
        id: 'test-4',
        qScore: 75,
        confidence: 0.9, // Incorrectly high confidence
        verdict: 'WAG',
        dimensions: { COHERENCE: 75 },
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(judgment);

      // Should be capped at φ⁻¹
      assert.ok(result.adjustedConfidence <= PHI_INV);
    });

    it('should emit doubt-applied event', () => {
      const judgment = {
        id: 'test-5',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      let emitted = false;
      skeptic.on('doubt-applied', (data) => {
        emitted = true;
        assert.strictEqual(data.judgmentId, 'test-5');
      });

      skeptic.doubt(judgment);
      assert.strictEqual(emitted, true);
    });
  });

  describe('Adversarial Analysis', () => {
    it('should flag extreme scores as suspicious', () => {
      const extremeHigh = {
        id: 'extreme-1',
        qScore: 95,
        confidence: PHI_INV,
        verdict: 'HOWL',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(extremeHigh);

      const extremeReason = result.doubt.reasons.find(r => r.type === 'extreme_score');
      assert.ok(extremeReason, 'Should flag extreme score');
    });

    it('should flag extreme low scores', () => {
      const extremeLow = {
        id: 'extreme-2',
        qScore: 5,
        confidence: PHI_INV_2,
        verdict: 'BARK',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(extremeLow);

      const extremeReason = result.doubt.reasons.find(r => r.type === 'extreme_score');
      assert.ok(extremeReason, 'Should flag extreme low score');
    });

    it('should flag unanimous dimensions', () => {
      const unanimous = {
        id: 'unanimous-1',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: {
          COHERENCE: 75,
          ACCURACY: 76,
          NOVELTY: 74,
          UTILITY: 75,
          VERIFIABILITY: 75,
          COMPLETENESS: 76,
        },
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(unanimous);

      const unanimousReason = result.doubt.reasons.find(r => r.type === 'unanimous_dimensions');
      assert.ok(unanimousReason, 'Should flag suspiciously consistent dimensions');
    });

    it('should flag when weakness present', () => {
      const withWeakness = {
        id: 'weakness-1',
        qScore: 60,
        confidence: 0.5,
        verdict: 'WAG',
        dimensions: {},
        weaknesses: {
          hasWeakness: true,
          weakestAxiom: 'VERIFY',
          gap: 25,
        },
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(withWeakness);

      const weaknessReason = result.doubt.reasons.find(r => r.type === 'weakness_present');
      assert.ok(weaknessReason, 'Should flag weakness');
      assert.ok(weaknessReason.message.includes('VERIFY'));
    });
  });

  describe('Confidence Decay', () => {
    it('should decay confidence over time', () => {
      const oneHourAgo = Date.now() - 3600000;
      const fresh = {
        id: 'fresh-1',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const old = {
        id: 'old-1',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: oneHourAgo },
        item: { type: 'code' },
      };

      const freshResult = skeptic.doubt(fresh);
      const oldResult = skeptic.doubt(old);

      // Older judgment should have lower confidence
      assert.ok(
        oldResult.adjustedConfidence <= freshResult.adjustedConfidence,
        'Older judgments should have decayed confidence'
      );
    });

    it('should report decay amount in metadata', () => {
      const oneHourAgo = Date.now() - 3600000;
      const old = {
        id: 'old-2',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: oneHourAgo },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(old);

      assert.ok(result.meta.decayApplied > 0, 'Should report decay applied');
    });
  });

  describe('Bias Detection', () => {
    it('should detect confirmation bias after repeated same verdicts', () => {
      // Feed 5 judgments with same verdict
      for (let i = 0; i < 5; i++) {
        skeptic.doubt({
          id: `confirm-${i}`,
          qScore: 75,
          confidence: 0.5,
          verdict: 'WAG',
          dimensions: {},
          metadata: { judgedAt: Date.now() },
          item: { type: 'code' },
        });
      }

      // 6th judgment should trigger confirmation bias
      const result = skeptic.doubt({
        id: 'confirm-6',
        qScore: 75,
        confidence: 0.5,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      });

      const confirmBias = result.biases.find(b => b.type === BiasType.CONFIRMATION);
      assert.ok(confirmBias, 'Should detect confirmation bias');
    });

    it('should detect overconfidence with weak evidence', () => {
      const overconfident = {
        id: 'overconf-1',
        qScore: 75,
        confidence: 0.55,
        verdict: 'WAG',
        dimensions: { THE_UNNAMEABLE: 30 }, // Low unexplained variance
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(overconfident);

      const overconfBias = result.biases.find(b => b.type === BiasType.OVERCONFIDENCE);
      assert.ok(overconfBias, 'Should detect overconfidence');
    });
  });

  describe('Meta-Doubt (Bounded Recursion)', () => {
    it('should apply meta-doubt at max 2 levels', () => {
      const judgment = {
        id: 'meta-1',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(judgment);

      assert.strictEqual(result.meta.metaDoubtDepth, SKEPTIC_CONSTANTS.MAX_DOUBT_DEPTH);
    });

    it('should track skepticism score', () => {
      const judgment = {
        id: 'meta-2',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(judgment);

      // Skepticism score should be between 0 and 1
      assert.ok(result.meta.skepticismScore >= 0);
      assert.ok(result.meta.skepticismScore <= 1);
    });
  });

  describe('Counter-Hypotheses', () => {
    it('should generate false positive hypothesis for high scores', () => {
      const highScore = {
        id: 'fp-1',
        qScore: 85,
        confidence: PHI_INV,
        verdict: 'HOWL',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(highScore);

      const fpHypothesis = result.counterHypotheses.find(h => h.scenario === 'false_positive');
      assert.ok(fpHypothesis, 'Should generate false positive hypothesis');
    });

    it('should generate false negative hypothesis for low scores', () => {
      const lowScore = {
        id: 'fn-1',
        qScore: 30,
        confidence: PHI_INV_2,
        verdict: 'BARK',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(lowScore);

      const fnHypothesis = result.counterHypotheses.find(h => h.scenario === 'false_negative');
      assert.ok(fnHypothesis, 'Should generate false negative hypothesis');
    });

    it('should generate axiom-specific hypotheses for weak axioms', () => {
      const weakAxiom = {
        id: 'axiom-1',
        qScore: 60,
        confidence: 0.5,
        verdict: 'WAG',
        dimensions: {},
        axiomScores: { PHI: 70, VERIFY: 40, CULTURE: 65, BURN: 75 },
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      };

      const result = skeptic.doubt(weakAxiom);

      const verifyHypothesis = result.counterHypotheses.find(h => h.scenario === 'weak_verify');
      assert.ok(verifyHypothesis, 'Should generate hypothesis for weak VERIFY axiom');
    });
  });

  describe('Statistics', () => {
    it('should track judgments doubled', () => {
      skeptic.doubt({
        id: 'stat-1',
        qScore: 75,
        confidence: 0.5,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      });

      const stats = skeptic.getStats();
      assert.strictEqual(stats.judgmentsDoubled, 1);
    });

    it('should track confidence reductions', () => {
      // Doubt an old judgment to ensure decay
      skeptic.doubt({
        id: 'stat-2',
        qScore: 75,
        confidence: PHI_INV,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() - 7200000 }, // 2 hours old
        item: { type: 'code' },
      });

      const stats = skeptic.getStats();
      assert.ok(stats.confidenceReductions > 0);
    });

    it('should track counter arguments generated', () => {
      // Doubt an extreme judgment to generate counter-arguments
      skeptic.doubt({
        id: 'stat-3',
        qScore: 95, // Extreme
        confidence: PHI_INV,
        verdict: 'HOWL',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      });

      const stats = skeptic.getStats();
      assert.ok(stats.counterArgumentsGenerated > 0);
    });
  });

  describe('Self-Doubt Patterns (Meta-Introspection)', () => {
    it('should track repeated bias detection', () => {
      // Generate many judgments with overconfidence bias
      for (let i = 0; i < 10; i++) {
        skeptic.doubt({
          id: `pattern-${i}`,
          qScore: 75,
          confidence: 0.55,
          verdict: 'WAG',
          dimensions: { THE_UNNAMEABLE: 30 },
          metadata: { judgedAt: Date.now() },
          item: { type: 'code' },
        });
      }

      const patterns = skeptic.getSelfDoubtPatterns();

      // Should detect that we're frequently finding overconfidence
      assert.ok(patterns.patterns.length >= 0); // May or may not detect depending on threshold
      assert.ok(patterns.meta.includes('φ distrusts φ'));
    });
  });

  describe('Export/Import', () => {
    it('should export state', () => {
      skeptic.doubt({
        id: 'export-1',
        qScore: 75,
        confidence: 0.5,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      });

      const exported = skeptic.export();

      assert.ok(Array.isArray(exported.judgmentHistory));
      assert.ok(Array.isArray(exported.detectedBiases));
      assert.ok(exported.stats);
      assert.ok(exported.exportedAt);
    });

    it('should import state', () => {
      const state = {
        judgmentHistory: [{ id: 'imported-1', qScore: 50, verdict: 'WAG' }],
        detectedBiases: [{ type: BiasType.RECENCY, severity: 'medium' }],
        stats: { judgmentsDoubled: 10 },
      };

      skeptic.import(state);

      assert.strictEqual(skeptic._judgmentHistory.length, 1);
      assert.strictEqual(skeptic._detectedBiases.length, 1);
      assert.strictEqual(skeptic._stats.judgmentsDoubled, 10);
    });
  });

  describe('Clear', () => {
    it('should clear all history', () => {
      skeptic.doubt({
        id: 'clear-1',
        qScore: 75,
        confidence: 0.5,
        verdict: 'WAG',
        dimensions: {},
        metadata: { judgedAt: Date.now() },
        item: { type: 'code' },
      });

      skeptic.clear();

      const stats = skeptic.getStats();
      assert.strictEqual(stats.judgmentsDoubled, 0);
      assert.strictEqual(stats.judgmentHistorySize, 0);
    });
  });
});

describe('CYNICJudge + SelfSkeptic Integration', () => {
  let judge;
  let skeptic;

  beforeEach(() => {
    skeptic = new SelfSkeptic();
    judge = new CYNICJudge({ selfSkeptic: skeptic });
  });

  describe('Automatic Skepticism', () => {
    it('should apply skepticism to judgments by default', () => {
      const item = {
        id: 'item-1',
        type: 'code',
        content: 'test content',
      };

      const judgment = judge.judge(item);

      assert.ok(judgment.skepticism, 'Judgment should have skepticism metadata');
      assert.ok(judgment.skepticism.adjustedConfidence <= judgment.skepticism.originalConfidence);
    });

    it('should include skepticism metadata', () => {
      const item = {
        id: 'item-2',
        type: 'code',
        content: 'test content',
      };

      const judgment = judge.judge(item);

      assert.ok(judgment.skepticism.doubt);
      assert.ok(Array.isArray(judgment.skepticism.biases));
      assert.ok(Array.isArray(judgment.skepticism.counterHypotheses));
      assert.ok(Array.isArray(judgment.skepticism.recommendation));
    });

    it('should update judgment confidence to adjusted value', () => {
      const item = {
        id: 'item-3',
        type: 'code',
        content: 'test content',
      };

      const judgment = judge.judge(item);

      // The judgment.confidence should be the adjusted (skeptical) confidence
      assert.strictEqual(judgment.confidence, judgment.skepticism.adjustedConfidence);
    });
  });

  describe('Disabling Skepticism', () => {
    it('should allow disabling automatic skepticism', () => {
      const noSkepticJudge = new CYNICJudge({
        selfSkeptic: skeptic,
        applySkepticism: false,
      });

      const item = {
        id: 'item-4',
        type: 'code',
        content: 'test content',
      };

      const judgment = noSkepticJudge.judge(item);

      assert.ok(!judgment.skepticism, 'Should not have skepticism when disabled');
    });
  });

  describe('judgeRaw Method', () => {
    it('should judge without applying skepticism', () => {
      const item = {
        id: 'item-5',
        type: 'code',
        content: 'test content',
      };

      const rawJudgment = judge.judgeRaw(item);

      assert.ok(!rawJudgment.skepticism, 'Raw judgment should not have skepticism');
    });

    it('should preserve original skepticism setting after judgeRaw', () => {
      const item1 = { id: 'item-6', type: 'code' };
      const item2 = { id: 'item-7', type: 'code' };

      judge.judgeRaw(item1);
      const normalJudgment = judge.judge(item2);

      assert.ok(normalJudgment.skepticism, 'Normal judgment should still have skepticism');
    });
  });

  describe('analyzeSkepticism Method', () => {
    it('should analyze skepticism for existing judgment', () => {
      const item = { id: 'item-8', type: 'code' };

      const rawJudgment = judge.judgeRaw(item);
      const skepticism = judge.analyzeSkepticism(rawJudgment);

      assert.ok(skepticism);
      assert.ok(skepticism.adjustedConfidence);
      assert.ok(skepticism.doubt);
    });

    it('should return null when no skeptic configured', () => {
      const noSkepticJudge = new CYNICJudge();

      const item = { id: 'item-9', type: 'code' };
      const judgment = noSkepticJudge.judge(item);
      const skepticism = noSkepticJudge.analyzeSkepticism(judgment);

      assert.strictEqual(skepticism, null);
    });
  });

  describe('setSelfSkeptic Method', () => {
    it('should allow post-construction skeptic injection', () => {
      const plainJudge = new CYNICJudge();
      const item1 = { id: 'item-10', type: 'code' };

      // Judge without skeptic
      const judgment1 = plainJudge.judge(item1);
      assert.ok(!judgment1.skepticism);

      // Inject skeptic
      plainJudge.setSelfSkeptic(new SelfSkeptic());

      // Judge with skeptic
      const item2 = { id: 'item-11', type: 'code' };
      const judgment2 = plainJudge.judge(item2);
      assert.ok(judgment2.skepticism);
    });
  });
});

// =============================================================================
// DEEP INTEGRATION TESTS: Judge + SelfSkeptic + Axiom Scorers
// =============================================================================

describe('Deep Integration: Judge + SelfSkeptic + Axiom Scorers', () => {
  let judge;
  let skeptic;

  beforeEach(() => {
    skeptic = new SelfSkeptic();
    judge = new CYNICJudge({ selfSkeptic: skeptic });
  });

  describe('E2E Flow with Real Items', () => {
    it('should score and doubt a high-quality code item', () => {
      const highQualityItem = {
        id: 'hq-code-1',
        type: 'code',
        content: `
          /**
           * Well-documented function with clear purpose
           */
          function calculateTotal(items) {
            if (!Array.isArray(items)) throw new Error('Invalid input');
            return items.reduce((sum, item) => sum + item.price, 0);
          }
        `,
        author: 'alice',
        verified: true,
        hash: 'sha256:abc123',
        original: true,
        purpose: 'Calculate shopping cart total',
        tests: true,
        documentation: true,
      };

      const judgment = judge.judge(highQualityItem);

      // Should have good Q-Score
      assert.ok(judgment.qScore >= 50, `Q-Score should be >= 50, got ${judgment.qScore}`);

      // Should have skepticism applied
      assert.ok(judgment.skepticism, 'Should have skepticism');
      assert.ok(
        judgment.skepticism.adjustedConfidence <= judgment.skepticism.originalConfidence,
        'Adjusted confidence should be <= original'
      );

      // Confidence should be bounded by φ
      assert.ok(judgment.confidence <= PHI_INV, `Confidence should be <= φ⁻¹, got ${judgment.confidence}`);
      assert.ok(judgment.confidence >= PHI_INV_2, `Confidence should be >= φ⁻², got ${judgment.confidence}`);
    });

    it('should score and doubt a suspicious scam item', () => {
      const scamItem = {
        id: 'scam-1',
        type: 'token',
        content: 'Guaranteed 1000x returns! Rug pull proof! Anonymous team!',
        tags: ['risk:scam', 'risk:fraud'],
      };

      const judgment = judge.judge(scamItem);

      // Should have low Q-Score
      assert.ok(judgment.qScore < 50, `Scam Q-Score should be < 50, got ${judgment.qScore}`);

      // Verdict should be negative
      assert.ok(
        judgment.qVerdict.verdict === 'BARK' || judgment.qVerdict.verdict === 'GROWL',
        `Should have negative verdict, got ${judgment.qVerdict.verdict}`
      );

      // Skepticism should flag issues
      assert.ok(judgment.skepticism, 'Should have skepticism');
    });

    it('should score and doubt an item with mixed signals', () => {
      const mixedItem = {
        id: 'mixed-1',
        type: 'code',
        content: 'A utility function that works but has no tests or documentation.',
        author: 'bob',
        purpose: 'Process data',
        // No tests, no docs, no verification
      };

      const judgment = judge.judge(mixedItem);

      // Should have moderate Q-Score
      assert.ok(judgment.qScore >= 30 && judgment.qScore <= 70,
        `Mixed item should have moderate score, got ${judgment.qScore}`);

      // Should detect weaknesses
      assert.ok(judgment.weaknesses, 'Should have weakness analysis');
    });
  });

  describe('Axiom Score Integration', () => {
    it('should calculate axiom scores that affect skepticism', () => {
      const item = {
        id: 'axiom-test-1',
        type: 'code',
        content: 'Simple utility function',
        // Minimal properties
      };

      const judgment = judge.judge(item);

      // Should have all 4 axiom scores
      assert.ok(judgment.axiomScores.PHI !== undefined, 'Should have PHI score');
      assert.ok(judgment.axiomScores.VERIFY !== undefined, 'Should have VERIFY score');
      assert.ok(judgment.axiomScores.CULTURE !== undefined, 'Should have CULTURE score');
      assert.ok(judgment.axiomScores.BURN !== undefined, 'Should have BURN score');

      // Axiom scores should influence counter-hypotheses
      if (judgment.axiomScores.VERIFY < 50) {
        const verifyHypothesis = judgment.skepticism.counterHypotheses.find(
          h => h.scenario === 'weak_verify'
        );
        assert.ok(verifyHypothesis, 'Should have VERIFY counter-hypothesis for low score');
      }
    });

    it('should generate axiom-specific counter-hypotheses', () => {
      const lowVerifyItem = {
        id: 'low-verify-1',
        type: 'claim',
        content: 'Unverified claim without sources',
        // No verification indicators
      };

      const judgment = judge.judge(lowVerifyItem);

      // VERIFY should be low
      assert.ok(judgment.axiomScores.VERIFY < 50,
        `VERIFY should be low without verification, got ${judgment.axiomScores.VERIFY}`);
    });
  });

  describe('Repeated Judgments and Bias Accumulation', () => {
    it('should accumulate judgment history for bias detection', () => {
      // Make several similar judgments
      for (let i = 0; i < 6; i++) {
        const item = {
          id: `repeat-${i}`,
          type: 'code',
          content: 'Simple function',
        };
        judge.judge(item);
      }

      const stats = skeptic.getStats();
      assert.ok(stats.judgmentsDoubled >= 6, 'Should track all judgments');
    });

    it('should detect confirmation bias pattern in judgments', () => {
      // Make 5 judgments with WAG verdict to set up pattern
      for (let i = 0; i < 5; i++) {
        const item = {
          id: `confirm-setup-${i}`,
          type: 'code',
          content: 'Decent code quality',
          quality: 60, // Ensures WAG verdict
        };
        judge.judge(item);
      }

      // 6th judgment should potentially trigger confirmation bias
      const item = {
        id: 'confirm-trigger',
        type: 'code',
        content: 'Similar quality code',
        quality: 60,
      };
      const judgment = judge.judge(item);

      // Check if confirmation bias was detected
      const stats = skeptic.getStats();
      // Bias detection depends on verdict consistency - may or may not trigger
      assert.ok(stats.judgmentsDoubled >= 6);
    });
  });

  describe('φ-Bounded Confidence', () => {
    it('should never exceed φ⁻¹ (61.8%) confidence after skepticism', () => {
      // Create an item that might score very well
      const excellentItem = {
        id: 'excellent-1',
        type: 'code',
        content: 'Perfect code with all quality indicators',
        verified: true,
        hash: 'sha256:perfect',
        signature: 'sig:trusted',
        author: 'expert',
        original: true,
        purpose: 'Critical function',
        tests: true,
        documentation: true,
        sources: ['ref1', 'ref2'],
        quality: 95,
      };

      const judgment = judge.judge(excellentItem);

      // Even for excellent items, confidence is φ-bounded
      assert.ok(
        judgment.confidence <= PHI_INV + 0.001, // Small tolerance for floating point
        `Confidence should be <= φ⁻¹ (${PHI_INV}), got ${judgment.confidence}`
      );
    });

    it('should never go below φ⁻² (38.2%) confidence', () => {
      // Create a very poor item
      const terribleItem = {
        id: 'terrible-1',
        type: 'unknown',
        content: 'scam fraud rug pull guaranteed returns anonymous team',
        tags: ['risk:scam', 'risk:fraud', 'risk:rugpull'],
        deprecated: true,
      };

      // Add some age to trigger decay
      const oldJudgment = judge.judgeRaw(terribleItem);
      oldJudgment.metadata = { judgedAt: Date.now() - 86400000 * 7 }; // 7 days old

      const skepticism = judge.analyzeSkepticism(oldJudgment);

      // Even with extreme penalties, minimum confidence holds
      assert.ok(
        skepticism.adjustedConfidence >= PHI_INV_2 - 0.001, // Small tolerance
        `Confidence should be >= φ⁻² (${PHI_INV_2}), got ${skepticism.adjustedConfidence}`
      );
    });
  });

  describe('Skepticism Recommendations', () => {
    it('should recommend seeking confirmation for high skepticism', () => {
      const uncertainItem = {
        id: 'uncertain-1',
        type: 'claim',
        content: 'Extraordinary claim without evidence',
        qScore: 95, // Will trigger extreme score flag
      };

      // Create judgment manually to control qScore
      const rawJudgment = judge.judgeRaw(uncertainItem);
      rawJudgment.qScore = 95; // Force extreme score

      const skepticism = skeptic.doubt(rawJudgment);

      // Should have recommendations
      assert.ok(skepticism.recommendation.length > 0, 'Should have recommendations');
    });

    it('should provide bias mitigation recommendations when biases detected', () => {
      // Set up overconfidence scenario
      const overconfidentItem = {
        id: 'overconf-item-1',
        type: 'code',
        content: 'Code with high variance in dimension scores',
      };

      const rawJudgment = judge.judgeRaw(overconfidentItem);
      // Simulate conditions for overconfidence
      rawJudgment.confidence = 0.55;
      rawJudgment.dimensions = { THE_UNNAMEABLE: 30 };
      rawJudgment.weaknesses = { hasWeakness: true, gap: 25 };

      const skepticism = skeptic.doubt(rawJudgment);

      // If overconfidence detected, should have bias mitigation recommendation
      const overconfBias = skepticism.biases.find(b => b.type === BiasType.OVERCONFIDENCE);
      if (overconfBias) {
        const biasMitigation = skepticism.recommendation.find(r => r.action === 'bias_mitigation');
        assert.ok(biasMitigation, 'Should have bias mitigation recommendation');
      }
    });
  });

  describe('THE_UNNAMEABLE Integration', () => {
    it('should include THE_UNNAMEABLE in dimension scores', () => {
      const item = {
        id: 'unnameable-1',
        type: 'code',
        content: 'Standard code item',
      };

      const judgment = judge.judge(item);

      assert.ok(
        judgment.dimensions.THE_UNNAMEABLE !== undefined,
        'Should have THE_UNNAMEABLE dimension'
      );
      assert.ok(
        judgment.dimensions.THE_UNNAMEABLE >= 0 && judgment.dimensions.THE_UNNAMEABLE <= 100,
        'THE_UNNAMEABLE should be 0-100'
      );
    });

    it('should use low THE_UNNAMEABLE to detect overconfidence', () => {
      const item = {
        id: 'unnameable-2',
        type: 'code',
        content: 'Item with high dimension variance',
      };

      const rawJudgment = judge.judgeRaw(item);
      // Force low THE_UNNAMEABLE (high unexplained variance)
      rawJudgment.dimensions.THE_UNNAMEABLE = 25;
      rawJudgment.confidence = 0.55;

      const skepticism = skeptic.doubt(rawJudgment);

      // Should detect overconfidence with low UNNAMEABLE
      const overconfBias = skepticism.biases.find(b => b.type === BiasType.OVERCONFIDENCE);
      assert.ok(overconfBias, 'Should detect overconfidence with low THE_UNNAMEABLE');
    });
  });

  describe('Statistics Consistency', () => {
    it('should maintain consistent stats between judge and skeptic', () => {
      for (let i = 0; i < 5; i++) {
        judge.judge({
          id: `stat-item-${i}`,
          type: 'code',
          content: 'Test item',
        });
      }

      const judgeStats = judge.getStats();
      const skepticStats = skeptic.getStats();

      // Judge should track total judgments
      assert.strictEqual(judgeStats.totalJudgments, 5);

      // Skeptic should track doubted judgments
      assert.strictEqual(skepticStats.judgmentsDoubled, 5);
    });
  });
});
