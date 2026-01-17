/**
 * Sandbox Formulas Tests
 *
 * Tests for the pure calculation functions from sandbox/formulas.js
 * Note: Browser-dependent rendering functions are not tested here.
 *
 * @module @cynic/mcp/test/sandbox-formulas
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

// Re-implement the formulas for testing (since original is browser-only)
const CYNICFormulas = {
  // Golden Ratio constants
  PHI: (1 + Math.sqrt(5)) / 2,
  PHI_INV: 2 / (1 + Math.sqrt(5)),
  PHI_INV_SQ: Math.pow(2 / (1 + Math.sqrt(5)), 2),

  AXIOMS: {
    PHI: { name: 'PHI', weight: 0.382 },
    VERIFY: { name: 'VERIFY', weight: 0.236 },
    CULTURE: { name: 'CULTURE', weight: 0.236 },
    BURN: { name: 'BURN', weight: 0.146 },
  },

  DIMENSIONS: {
    PHI: [
      { id: 'symmetry', name: 'Symmetry' },
      { id: 'proportion', name: 'Proportion' },
      { id: 'coherence', name: 'Coherence' },
      { id: 'resonance', name: 'Resonance' },
      { id: 'elegance', name: 'Elegance' },
      { id: 'balance', name: 'Balance' },
    ],
    VERIFY: [
      { id: 'source_quality', name: 'Source Quality' },
      { id: 'data_integrity', name: 'Data Integrity' },
      { id: 'cross_reference', name: 'Cross Reference' },
      { id: 'temporal_validity', name: 'Temporal Validity' },
      { id: 'methodology', name: 'Methodology' },
      { id: 'transparency', name: 'Transparency' },
      { id: 'reproducibility', name: 'Reproducibility' },
    ],
    CULTURE: [
      { id: 'community_alignment', name: 'Community Alignment' },
      { id: 'historical_pattern', name: 'Historical Pattern' },
      { id: 'network_position', name: 'Network Position' },
      { id: 'contribution_history', name: 'Contribution History' },
      { id: 'reputation_signals', name: 'Reputation Signals' },
      { id: 'cultural_fit', name: 'Cultural Fit' },
    ],
    BURN: [
      { id: 'resource_efficiency', name: 'Resource Efficiency' },
      { id: 'value_distribution', name: 'Value Distribution' },
      { id: 'extraction_resistance', name: 'Extraction Resistance' },
      { id: 'sustainability', name: 'Sustainability' },
      { id: 'burn_commitment', name: 'Burn Commitment' },
      { id: 'simplicity', name: 'Simplicity' },
    ],
  },

  E_SCORE_COMPONENTS: {
    HOLD: { weight: 0.20 },
    BURN: { weight: 0.20 },
    USE: { weight: 0.15 },
    BUILD: { weight: 0.15 },
    RUN: { weight: 0.10 },
    REFER: { weight: 0.10 },
    TIME: { weight: 0.10 },
  },

  calculateQScore(dimensionScores) {
    let totalScore = 0;

    for (const [axiom, dimensions] of Object.entries(this.DIMENSIONS)) {
      const axiomWeight = this.AXIOMS[axiom].weight;
      let axiomSum = 0;
      let count = 0;

      for (const dim of dimensions) {
        const score = dimensionScores[dim.id] ?? 0.5;
        axiomSum += score;
        count++;
      }

      const axiomAvg = count > 0 ? axiomSum / count : 0;
      totalScore += axiomWeight * axiomAvg;
    }

    const rawScore = totalScore * 100;
    const constrainedScore = Math.min(rawScore, 100 * this.PHI_INV);

    return {
      raw: rawScore,
      constrained: constrainedScore,
      confidence: Math.min(constrainedScore / 100, this.PHI_INV),
    };
  },

  calculateEScore(componentScores) {
    let totalScore = 0;

    for (const [component, config] of Object.entries(this.E_SCORE_COMPONENTS)) {
      const score = componentScores[component] ?? 0;
      totalScore += config.weight * score;
    }

    return {
      raw: totalScore * 100,
      components: componentScores,
    };
  },

  calculateFinalScore(kScore, qScore) {
    return Math.min(kScore, qScore);
  },

  getVerdict(qScore) {
    if (qScore >= 75) return { verdict: 'HOWL', emoji: '🐺' };
    if (qScore >= 50) return { verdict: 'WAG', emoji: '🐕' };
    if (qScore >= 25) return { verdict: 'GROWL', emoji: '😾' };
    return { verdict: 'BARK', emoji: '🚨' };
  },

  getFormulasLatex() {
    return [
      { name: 'Golden Ratio', latex: '\\varphi = \\frac{1 + \\sqrt{5}}{2}' },
      { name: 'Max Confidence', latex: '\\varphi^{-1}' },
      { name: 'Q-Score', latex: 'Q = \\sum_{i} (a_i \\cdot w_i)' },
    ];
  },

  getMatrixData(dimensionScores = {}) {
    const matrixData = [];

    for (const [axiom, dimensions] of Object.entries(this.DIMENSIONS)) {
      for (const dim of dimensions) {
        matrixData.push({
          axiom,
          dimension: dim.id,
          name: dim.name,
          score: dimensionScores[dim.id] ?? 0.5,
          weight: this.AXIOMS[axiom].weight / dimensions.length,
        });
      }
    }

    return matrixData;
  },

  simulateJudgment() {
    const dimensionScores = {};

    for (const dimensions of Object.values(this.DIMENSIONS)) {
      for (const dim of dimensions) {
        dimensionScores[dim.id] = Math.random() * this.PHI_INV + (1 - this.PHI_INV) * Math.random();
      }
    }

    const qResult = this.calculateQScore(dimensionScores);
    const verdict = this.getVerdict(qResult.constrained);

    return {
      dimensions: dimensionScores,
      qScore: qResult,
      verdict,
      timestamp: new Date().toISOString(),
    };
  },
};

describe('CYNICFormulas Constants', () => {
  it('has correct PHI value', () => {
    assert.ok(Math.abs(CYNICFormulas.PHI - 1.618033988749895) < 0.0001);
  });

  it('has correct PHI_INV value', () => {
    assert.ok(Math.abs(CYNICFormulas.PHI_INV - 0.618033988749895) < 0.0001);
  });

  it('PHI * PHI_INV = 1', () => {
    assert.ok(Math.abs(CYNICFormulas.PHI * CYNICFormulas.PHI_INV - 1) < 0.0001);
  });

  it('axiom weights sum to 1', () => {
    const sum = Object.values(CYNICFormulas.AXIOMS).reduce((s, a) => s + a.weight, 0);
    assert.ok(Math.abs(sum - 1) < 0.001);
  });

  it('has 25 dimensions', () => {
    let count = 0;
    for (const dims of Object.values(CYNICFormulas.DIMENSIONS)) {
      count += dims.length;
    }
    assert.equal(count, 25);
  });

  it('has 4 axioms', () => {
    assert.equal(Object.keys(CYNICFormulas.AXIOMS).length, 4);
  });
});

describe('CYNICFormulas.calculateQScore', () => {
  it('calculates from dimension scores', () => {
    const scores = {};
    for (const dims of Object.values(CYNICFormulas.DIMENSIONS)) {
      for (const dim of dims) {
        scores[dim.id] = 0.8;
      }
    }

    const result = CYNICFormulas.calculateQScore(scores);

    assert.ok(result.raw >= 0);
    assert.ok(result.constrained <= 100 * CYNICFormulas.PHI_INV);
    assert.ok(result.confidence <= CYNICFormulas.PHI_INV);
  });

  it('uses default 0.5 for missing dimensions', () => {
    const result = CYNICFormulas.calculateQScore({});

    assert.ok(result.raw > 0);
    assert.ok(Math.abs(result.raw - 50) < 1);
  });

  it('constrains to PHI_INV maximum', () => {
    const scores = {};
    for (const dims of Object.values(CYNICFormulas.DIMENSIONS)) {
      for (const dim of dims) {
        scores[dim.id] = 1.0;
      }
    }

    const result = CYNICFormulas.calculateQScore(scores);

    assert.ok(result.constrained <= 100 * CYNICFormulas.PHI_INV + 0.01);
  });

  it('returns zero for all zeros', () => {
    const scores = {};
    for (const dims of Object.values(CYNICFormulas.DIMENSIONS)) {
      for (const dim of dims) {
        scores[dim.id] = 0;
      }
    }

    const result = CYNICFormulas.calculateQScore(scores);

    assert.equal(result.raw, 0);
  });
});

describe('CYNICFormulas.calculateEScore', () => {
  it('calculates from component scores', () => {
    const scores = {
      HOLD: 0.8,
      BURN: 0.9,
      USE: 0.7,
      BUILD: 0.6,
      RUN: 0.5,
      REFER: 0.4,
      TIME: 0.3,
    };

    const result = CYNICFormulas.calculateEScore(scores);

    assert.ok(result.raw > 0);
    assert.deepEqual(result.components, scores);
  });

  it('handles missing components as 0', () => {
    const result = CYNICFormulas.calculateEScore({});

    assert.equal(result.raw, 0);
  });

  it('weights sum correctly', () => {
    const sum = Object.values(CYNICFormulas.E_SCORE_COMPONENTS).reduce(
      (s, c) => s + c.weight,
      0
    );
    assert.equal(sum, 1.0);
  });

  it('max score is 100', () => {
    const scores = {
      HOLD: 1.0,
      BURN: 1.0,
      USE: 1.0,
      BUILD: 1.0,
      RUN: 1.0,
      REFER: 1.0,
      TIME: 1.0,
    };

    const result = CYNICFormulas.calculateEScore(scores);

    assert.equal(result.raw, 100);
  });
});

describe('CYNICFormulas.calculateFinalScore', () => {
  it('returns minimum of K and Q', () => {
    assert.equal(CYNICFormulas.calculateFinalScore(80, 70), 70);
    assert.equal(CYNICFormulas.calculateFinalScore(60, 90), 60);
    assert.equal(CYNICFormulas.calculateFinalScore(50, 50), 50);
  });
});

describe('CYNICFormulas.getVerdict', () => {
  it('returns HOWL for >= 75', () => {
    const result = CYNICFormulas.getVerdict(75);
    assert.equal(result.verdict, 'HOWL');
  });

  it('returns WAG for >= 50 and < 75', () => {
    const result = CYNICFormulas.getVerdict(60);
    assert.equal(result.verdict, 'WAG');
  });

  it('returns GROWL for >= 25 and < 50', () => {
    const result = CYNICFormulas.getVerdict(30);
    assert.equal(result.verdict, 'GROWL');
  });

  it('returns BARK for < 25', () => {
    const result = CYNICFormulas.getVerdict(10);
    assert.equal(result.verdict, 'BARK');
  });

  it('includes emoji', () => {
    const result = CYNICFormulas.getVerdict(80);
    assert.ok(result.emoji);
  });
});

describe('CYNICFormulas.getFormulasLatex', () => {
  it('returns array of formulas', () => {
    const formulas = CYNICFormulas.getFormulasLatex();

    assert.ok(Array.isArray(formulas));
    assert.ok(formulas.length >= 3);
  });

  it('formulas have name and latex', () => {
    const formulas = CYNICFormulas.getFormulasLatex();

    for (const f of formulas) {
      assert.ok(f.name);
      assert.ok(f.latex);
    }
  });
});

describe('CYNICFormulas.getMatrixData', () => {
  it('returns 25 dimension entries', () => {
    const data = CYNICFormulas.getMatrixData({});

    assert.equal(data.length, 25);
  });

  it('includes all required fields', () => {
    const data = CYNICFormulas.getMatrixData({});

    for (const entry of data) {
      assert.ok(entry.axiom);
      assert.ok(entry.dimension);
      assert.ok(entry.name);
      assert.ok(typeof entry.score === 'number');
      assert.ok(typeof entry.weight === 'number');
    }
  });

  it('uses provided scores', () => {
    const data = CYNICFormulas.getMatrixData({ symmetry: 0.9 });
    const symmetry = data.find((d) => d.dimension === 'symmetry');

    assert.equal(symmetry.score, 0.9);
  });

  it('uses default 0.5 for missing scores', () => {
    const data = CYNICFormulas.getMatrixData({});

    for (const entry of data) {
      assert.equal(entry.score, 0.5);
    }
  });
});

describe('CYNICFormulas.simulateJudgment', () => {
  it('returns complete judgment object', () => {
    const judgment = CYNICFormulas.simulateJudgment();

    assert.ok(judgment.dimensions);
    assert.ok(judgment.qScore);
    assert.ok(judgment.verdict);
    assert.ok(judgment.timestamp);
  });

  it('generates all 25 dimension scores', () => {
    const judgment = CYNICFormulas.simulateJudgment();
    const dimCount = Object.keys(judgment.dimensions).length;

    assert.equal(dimCount, 25);
  });

  it('scores are bounded by PHI constraints', () => {
    const judgment = CYNICFormulas.simulateJudgment();

    assert.ok(judgment.qScore.constrained <= 100 * CYNICFormulas.PHI_INV + 0.01);
    assert.ok(judgment.qScore.confidence <= CYNICFormulas.PHI_INV + 0.01);
  });

  it('verdict matches qScore', () => {
    const judgment = CYNICFormulas.simulateJudgment();
    const expectedVerdict = CYNICFormulas.getVerdict(judgment.qScore.constrained);

    assert.equal(judgment.verdict.verdict, expectedVerdict.verdict);
  });
});
