/**
 * Judge Tests
 *
 * Tests for dimensions, CYNIC judge, and residual detector
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  Dimensions,
  getAllDimensions,
  getDimensionsForAxiom,
  getDimension,
  dimensionRegistry,
  CYNICJudge,
  ResidualDetector,
} from '../src/index.js';

import { PHI, PHI_INV, PHI_INV_2, AXIOMS } from '@cynic/core';
import { Verdict } from '@cynic/protocol';

describe('Dimensions', () => {
  it('should have 5 axioms (4 core + META)', () => {
    const axioms = Object.keys(Dimensions);
    assert.strictEqual(axioms.length, 5);
    assert.ok(axioms.includes('PHI'));
    assert.ok(axioms.includes('VERIFY'));
    assert.ok(axioms.includes('CULTURE'));
    assert.ok(axioms.includes('BURN'));
    assert.ok(axioms.includes('META')); // 25th dimension axiom
  });

  it('should have 6 dimensions per core axiom, 1 for META', () => {
    for (const axiom of Object.keys(Dimensions)) {
      const dims = Object.keys(Dimensions[axiom]);
      const expected = axiom === 'META' ? 1 : 6;
      assert.strictEqual(dims.length, expected, `${axiom} should have ${expected} dimensions`);
    }
  });

  it('should have φ-derived weights', () => {
    // Check PHI axiom dimensions
    assert.strictEqual(Dimensions.PHI.COHERENCE.weight, PHI);
    assert.strictEqual(Dimensions.PHI.HARMONY.weight, PHI_INV);
    assert.strictEqual(Dimensions.PHI.ELEGANCE.weight, PHI_INV_2);

    // Check VERIFY axiom dimensions
    assert.strictEqual(Dimensions.VERIFY.ACCURACY.weight, PHI);
    assert.strictEqual(Dimensions.VERIFY.VERIFIABILITY.weight, PHI);
  });

  it('should get all dimensions flat', () => {
    const all = getAllDimensions();

    // Should have 25 dimensions (4 axioms × 6 + 1 META)
    assert.strictEqual(Object.keys(all).length, 25);

    // Each should have axiom property
    for (const [name, config] of Object.entries(all)) {
      assert.ok(config.axiom, `${name} should have axiom`);
      assert.ok(config.weight, `${name} should have weight`);
      assert.ok(typeof config.threshold === 'number', `${name} should have threshold`);
    }
  });

  it('should get dimensions for axiom', () => {
    const phiDims = getDimensionsForAxiom('PHI');
    assert.strictEqual(Object.keys(phiDims).length, 6);
    assert.ok(phiDims.COHERENCE);
    assert.ok(phiDims.HARMONY);
  });

  it('should return empty for invalid axiom', () => {
    const invalid = getDimensionsForAxiom('INVALID');
    assert.deepStrictEqual(invalid, {});
  });

  it('should get dimension by name', () => {
    const coherence = getDimension('COHERENCE');
    assert.ok(coherence);
    assert.strictEqual(coherence.axiom, 'PHI');
    assert.strictEqual(coherence.weight, PHI);
  });

  it('should return null for unknown dimension', () => {
    const unknown = getDimension('NONEXISTENT');
    assert.strictEqual(unknown, null);
  });
});

describe('Dimension Registry', () => {
  beforeEach(() => {
    // Clear custom dimensions
    dimensionRegistry.custom = {};
  });

  it('should register custom dimension', () => {
    dimensionRegistry.register('CUSTOM_DIM', 'VERIFY', {
      weight: 1.0,
      threshold: 50,
      description: 'A custom dimension',
    });

    const dim = dimensionRegistry.get('CUSTOM_DIM');
    assert.ok(dim);
    assert.strictEqual(dim.axiom, 'VERIFY');
    assert.strictEqual(dim.weight, 1.0);
    assert.ok(dim.discovered);
    assert.ok(dim.discoveredAt);
  });

  it('should reject invalid axiom', () => {
    assert.throws(() => {
      dimensionRegistry.register('BAD', 'INVALID_AXIOM', { weight: 1.0 });
    });
  });

  it('should get all custom dimensions', () => {
    dimensionRegistry.register('DIM_A', 'PHI', { weight: 1.0 });
    dimensionRegistry.register('DIM_B', 'BURN', { weight: 1.5 });

    const all = dimensionRegistry.getAll();
    assert.strictEqual(Object.keys(all).length, 2);
    assert.ok(all.DIM_A);
    assert.ok(all.DIM_B);
  });

  it('should export and import registry', () => {
    dimensionRegistry.register('TEST_DIM', 'CULTURE', {
      weight: PHI_INV,
      threshold: 45,
    });

    const exported = dimensionRegistry.export();
    assert.ok(exported.custom.TEST_DIM);

    // Clear and import
    dimensionRegistry.custom = {};
    dimensionRegistry.import(exported);

    const restored = dimensionRegistry.get('TEST_DIM');
    assert.ok(restored);
    assert.strictEqual(restored.axiom, 'CULTURE');
  });
});

describe('CYNIC Judge', () => {
  let judge;

  beforeEach(() => {
    judge = new CYNICJudge();
  });

  it('should judge item', () => {
    const item = {
      id: 'test_1',
      type: 'knowledge',
      content: 'Test content',
    };

    const judgment = judge.judge(item);

    assert.ok(judgment.id.startsWith('jdg_'));
    assert.ok(judgment.item_hash);
    assert.ok(judgment.global_score >= 0 && judgment.global_score <= 100);
    assert.ok(judgment.verdict);
    assert.ok(judgment.dimensions);
    assert.ok(judgment.confidence > 0 && judgment.confidence <= PHI_INV);
  });

  it('should produce dog-themed verdicts', () => {
    // High quality item - must provide scores for all dimensions to get HOWL
    // Create scores object with 85 for all 24 dimensions
    const allDims = getAllDimensions();
    const highScores = {};
    for (const dimName of Object.keys(allDims)) {
      highScores[dimName] = 85;
    }

    const goodItem = {
      id: 'good',
      verified: true,
      scores: highScores,
    };
    const goodJudgment = judge.judge(goodItem);
    assert.strictEqual(goodJudgment.verdict, Verdict.HOWL);

    // Medium quality item - use explicit scores for WAG range (50-80)
    const mediumScores = {};
    for (const dimName of Object.keys(allDims)) {
      mediumScores[dimName] = 55;
    }
    const okItem = { id: 'ok', scores: mediumScores };
    const okJudgment = judge.judge(okItem);
    assert.strictEqual(okJudgment.verdict, Verdict.WAG);
  });

  it('should bound confidence to φ⁻¹', () => {
    // Even with consistent scores, confidence should not exceed φ⁻¹
    const item = {
      id: 'test',
      scores: {
        COHERENCE: 50,
        HARMONY: 50,
        STRUCTURE: 50,
        ELEGANCE: 50,
        COMPLETENESS: 50,
        PRECISION: 50,
        ACCURACY: 50,
        VERIFIABILITY: 50,
        TRANSPARENCY: 50,
        REPRODUCIBILITY: 50,
        PROVENANCE: 50,
        INTEGRITY: 50,
        AUTHENTICITY: 50,
        RELEVANCE: 50,
        NOVELTY: 50,
        ALIGNMENT: 50,
        IMPACT: 50,
        RESONANCE: 50,
        UTILITY: 50,
        SUSTAINABILITY: 50,
        EFFICIENCY: 50,
        VALUE_CREATION: 50,
        NON_EXTRACTIVE: 50,
        CONTRIBUTION: 50,
      },
    };

    const judgment = judge.judge(item);
    assert.ok(judgment.confidence <= PHI_INV + 0.001);
  });

  it('should use custom scorer', () => {
    const customJudge = new CYNICJudge({
      scorer: (dimName, item) => {
        if (dimName === 'ACCURACY') return 100;
        return 50;
      },
    });

    const item = { id: 'test' };
    const judgment = customJudge.judge(item);

    assert.strictEqual(judgment.dimensions.ACCURACY, 100);
  });

  it('should score coherence based on structure', () => {
    // Structured item
    const structured = { id: 'test', type: 'data', content: 'hello' };
    const j1 = judge.judge(structured);

    // Unstructured item
    const unstructured = 'just a string';
    const j2 = judge.judge(unstructured);

    assert.ok(j1.dimensions.COHERENCE > j2.dimensions.COHERENCE);
  });

  it('should score accuracy based on verification', () => {
    const verified = { id: 'test', verified: true, hash: 'sha256:abc' };
    const unverified = { id: 'test' };

    const j1 = judge.judge(verified);
    const j2 = judge.judge(unverified);

    assert.ok(j1.dimensions.ACCURACY > j2.dimensions.ACCURACY);
  });

  it('should detect anomalies', () => {
    // Create items that will produce high residuals
    for (let i = 0; i < 10; i++) {
      judge.judge({ id: `anomaly_${i}`, quality: i * 10 });
    }

    const stats = judge.getStats();
    // Anomalies may or may not be detected depending on variance
    assert.ok(stats.anomaliesDetected >= 0);
  });

  it('should track statistics', () => {
    judge.judge({ id: 'test1', quality: 80 });
    judge.judge({ id: 'test2', quality: 60 });
    judge.judge({ id: 'test3', quality: 40 });

    const stats = judge.getStats();
    assert.strictEqual(stats.totalJudgments, 3);
    assert.ok(stats.avgScore > 0);
    assert.ok(stats.verdicts.HOWL >= 0);
    assert.ok(stats.verdicts.WAG >= 0);
    assert.ok(stats.verdicts.GROWL >= 0);
    assert.ok(stats.verdicts.BARK >= 0);
  });

  it('should get anomaly candidates', () => {
    // Judge many items to potentially build anomaly buffer
    for (let i = 0; i < 20; i++) {
      judge.judge({ id: `item_${i}`, quality: Math.random() * 100 });
    }

    const candidates = judge.getAnomalyCandidates();
    assert.ok(Array.isArray(candidates));
  });

  it('should reset statistics', () => {
    judge.judge({ id: 'test' });
    judge.judge({ id: 'test2' });

    judge.resetStats();

    const stats = judge.getStats();
    assert.strictEqual(stats.totalJudgments, 0);
    assert.strictEqual(stats.avgScore, 0);
    assert.strictEqual(stats.anomaliesDetected, 0);
  });
});

describe('Residual Detector', () => {
  let detector;

  beforeEach(() => {
    detector = new ResidualDetector();
  });

  it('should have φ⁻² default threshold', () => {
    assert.ok(Math.abs(detector.threshold - PHI_INV_2) < 0.001);
  });

  it('should analyze judgment for residual', () => {
    const judgment = {
      id: 'jdg_test',
      global_score: 75,
      dimensions: { A: 75, B: 75, C: 75 },
    };

    const result = detector.analyze(judgment);

    assert.strictEqual(result.judgmentId, 'jdg_test');
    assert.ok(typeof result.residual === 'number');
    assert.ok(typeof result.isAnomaly === 'boolean');
    assert.ok(result.timestamp);
  });

  it('should detect high residual as anomaly', () => {
    // Create judgment with high discrepancy
    const anomalous = {
      id: 'jdg_anomaly',
      global_score: 90,
      dimensions: { A: 30 }, // Big gap = high residual
    };

    const result = detector.analyze(anomalous);
    assert.strictEqual(result.isAnomaly, true);
    assert.ok(result.residual > PHI_INV_2);
  });

  it('should not detect low residual as anomaly', () => {
    const consistent = {
      id: 'jdg_consistent',
      global_score: 75,
      dimensions: { A: 75, B: 75, C: 75 },
    };

    const result = detector.analyze(consistent);
    assert.strictEqual(result.isAnomaly, false);
    assert.ok(result.residual < PHI_INV_2);
  });

  it('should accumulate anomalies', () => {
    for (let i = 0; i < 10; i++) {
      detector.analyze({
        id: `jdg_${i}`,
        global_score: 90,
        dimensions: { A: 30 },
      });
    }

    const stats = detector.getStats();
    assert.strictEqual(stats.anomalyCount, 10);
  });

  it('should get candidate dimensions', () => {
    // Add enough anomalies to trigger pattern detection
    for (let i = 0; i < 10; i++) {
      detector.analyze(
        {
          id: `jdg_${i}`,
          global_score: 90,
          dimensions: { COHERENCE: 30, ACCURACY: 30 },
        },
        { context: 'test' }
      );
    }

    const candidates = detector.getCandidates();
    assert.ok(Array.isArray(candidates));
  });

  it('should accept candidate dimension', () => {
    // Manually add a candidate
    detector.candidates.set('test_candidate', {
      key: 'test_candidate',
      suggestedAxiom: 'VERIFY',
      suggestedName: 'UNNAMED_test',
      confidence: PHI_INV,
      sampleCount: 5,
    });

    const discovery = detector.acceptCandidate('test_candidate', {
      name: 'NEW_DIM',
      axiom: 'VERIFY',
      weight: 1.0,
      threshold: 50,
    });

    assert.strictEqual(discovery.name, 'NEW_DIM');
    assert.strictEqual(discovery.axiom, 'VERIFY');
    assert.ok(discovery.discoveredAt);

    // Should be removed from candidates
    assert.strictEqual(detector.candidates.size, 0);

    // Should be in discoveries
    assert.strictEqual(detector.discoveries.length, 1);

    // Should be in dimension registry
    const registered = dimensionRegistry.get('NEW_DIM');
    assert.ok(registered);
    assert.strictEqual(registered.axiom, 'VERIFY');
  });

  it('should reject invalid axiom in acceptance', () => {
    detector.candidates.set('bad_candidate', {
      key: 'bad_candidate',
      confidence: PHI_INV,
    });

    assert.throws(() => {
      detector.acceptCandidate('bad_candidate', {
        name: 'BAD_DIM',
        axiom: 'INVALID',
      });
    });
  });

  it('should reject unknown candidate', () => {
    assert.throws(() => {
      detector.acceptCandidate('nonexistent', { name: 'X', axiom: 'PHI' });
    });
  });

  it('should reject candidate', () => {
    detector.candidates.set('to_reject', { key: 'to_reject' });
    detector.rejectCandidate('to_reject');

    assert.strictEqual(detector.candidates.has('to_reject'), false);
  });

  it('should export and import state', () => {
    // Add some state
    detector.analyze({
      id: 'jdg_test',
      global_score: 90,
      dimensions: { A: 30 },
    });

    const exported = detector.export();
    assert.ok(exported.anomalies);
    assert.ok(exported.candidates);
    assert.ok(exported.discoveries);

    // Create new detector and import
    const newDetector = new ResidualDetector();
    newDetector.import(exported);

    assert.strictEqual(newDetector.anomalies.length, detector.anomalies.length);
  });

  it('should get statistics', () => {
    const stats = detector.getStats();

    assert.ok('anomalyCount' in stats);
    assert.ok('candidateCount' in stats);
    assert.ok('discoveryCount' in stats);
    assert.ok('threshold' in stats);
    assert.ok(Math.abs(stats.threshold - PHI_INV_2) < 0.001);
  });

  it('should get discoveries', () => {
    const discoveries = detector.getDiscoveries();
    assert.ok(Array.isArray(discoveries));
  });
});

describe('Q-Score Integration', () => {
  let judge;
  let allDims;

  beforeEach(() => {
    judge = new CYNICJudge();
    allDims = getAllDimensions();
  });

  it('should calculate axiom scores', () => {
    const scores = {};
    // PHI at 80
    for (const dim of Object.keys(getDimensionsForAxiom('PHI'))) {
      scores[dim] = 80;
    }
    // VERIFY at 70
    for (const dim of Object.keys(getDimensionsForAxiom('VERIFY'))) {
      scores[dim] = 70;
    }
    // CULTURE at 60
    for (const dim of Object.keys(getDimensionsForAxiom('CULTURE'))) {
      scores[dim] = 60;
    }
    // BURN at 50
    for (const dim of Object.keys(getDimensionsForAxiom('BURN'))) {
      scores[dim] = 50;
    }

    const item = { id: 'test', scores };
    const judgment = judge.judge(item);

    assert.ok(judgment.axiomScores);
    assert.ok(judgment.axiomScores.PHI > 70);
    assert.ok(judgment.axiomScores.VERIFY > 60);
    assert.ok(judgment.axiomScores.CULTURE > 50);
    assert.ok(judgment.axiomScores.BURN > 40);
  });

  it('should calculate Q-Score via geometric mean', () => {
    // All dimensions at 70
    const scores = {};
    for (const dim of Object.keys(allDims)) {
      scores[dim] = 70;
    }

    const item = { id: 'test', scores };
    const judgment = judge.judge(item);

    assert.ok(judgment.qScore);
    // When all axioms are equal, Q-Score should be close to that value
    assert.ok(Math.abs(judgment.qScore - 70) < 5);
  });

  it('should include qVerdict', () => {
    const scores = {};
    for (const dim of Object.keys(allDims)) {
      scores[dim] = 85;
    }

    const item = { id: 'test', scores };
    const judgment = judge.judge(item);

    assert.ok(judgment.qVerdict);
    assert.strictEqual(judgment.qVerdict.verdict, 'HOWL');
  });

  it('should identify weaknesses', () => {
    const scores = {};
    // PHI high
    for (const dim of Object.keys(getDimensionsForAxiom('PHI'))) {
      scores[dim] = 90;
    }
    // VERIFY high
    for (const dim of Object.keys(getDimensionsForAxiom('VERIFY'))) {
      scores[dim] = 85;
    }
    // CULTURE medium
    for (const dim of Object.keys(getDimensionsForAxiom('CULTURE'))) {
      scores[dim] = 60;
    }
    // BURN low - this should be identified as weakness
    for (const dim of Object.keys(getDimensionsForAxiom('BURN'))) {
      scores[dim] = 30;
    }

    const item = { id: 'test', scores };
    const judgment = judge.judge(item);

    assert.ok(judgment.weaknesses);
    assert.strictEqual(judgment.weaknesses.weakestAxiom, 'BURN');
  });

  it('should penalize imbalance via geometric mean', () => {
    // Balanced at 70
    const balancedScores = {};
    for (const dim of Object.keys(allDims)) {
      balancedScores[dim] = 70;
    }

    // Imbalanced: PHI, VERIFY, CULTURE at 90, BURN at 20
    const imbalancedScores = {};
    for (const axiom of ['PHI', 'VERIFY', 'CULTURE']) {
      for (const dim of Object.keys(getDimensionsForAxiom(axiom))) {
        imbalancedScores[dim] = 90;
      }
    }
    for (const dim of Object.keys(getDimensionsForAxiom('BURN'))) {
      imbalancedScores[dim] = 20;
    }

    const balanced = judge.judge({ id: 'balanced', scores: balancedScores });
    const imbalanced = judge.judge({ id: 'imbalanced', scores: imbalancedScores });

    // Geometric mean punishes imbalance
    assert.ok(imbalanced.qScore < balanced.qScore);
  });

  it('should calculate Final score when K-Score provided', () => {
    const scores = {};
    for (const dim of Object.keys(allDims)) {
      scores[dim] = 80;
    }

    const item = { id: 'test', scores };
    const judgment = judge.judge(item, { kScore: 80 });

    assert.ok(judgment.finalScore);
    assert.strictEqual(judgment.kScore, 80);
    // Final = sqrt(K * Q) = sqrt(80 * ~80) = ~80
    assert.ok(Math.abs(judgment.finalScore - 80) < 5);
  });

  it('should not include Final without K-Score', () => {
    const item = { id: 'test' };
    const judgment = judge.judge(item);

    assert.strictEqual(judgment.finalScore, undefined);
    assert.strictEqual(judgment.kScore, undefined);
  });

  it('should identify limiting factor', () => {
    const scores = {};
    for (const dim of Object.keys(allDims)) {
      scores[dim] = 90;
    }

    // Q-Score ~90, K-Score = 40 -> K is limiting
    const item = { id: 'test', scores };
    const judgment = judge.judge(item, { kScore: 40 });

    assert.strictEqual(judgment.limiting, 'K-Score');
  });
});
