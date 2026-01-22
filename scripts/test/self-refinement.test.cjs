#!/usr/bin/env node
/**
 * Tests for CYNIC Self-Refinement System
 *
 * "φ distrusts φ" - Testing the self-critique mechanism
 *
 * @module scripts/test/self-refinement.test
 */

'use strict';

const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert');
const path = require('path');

// Load module under test
const refinement = require(path.join(__dirname, '..', 'lib', 'self-refinement.cjs'));

describe('Self-Refinement Module', () => {

  describe('critiqueJudgment', () => {

    it('should detect weak axioms', () => {
      const judgment = {
        Q: 45,
        breakdown: { PHI: 60, VERIFY: 20, CULTURE: 55, BURN: 50 },
        verdict: 'GROWL',
      };

      const critique = refinement.critiqueJudgment(judgment);

      assert.ok(critique.critiques.length > 0, 'Should have critiques');
      assert.ok(critique.critiques.some(c => c.type === 'weak_axiom'), 'Should detect weak axiom');
      assert.strictEqual(critique.refinable, true, 'Should be refinable');
    });

    it('should detect imbalance between axioms', () => {
      const judgment = {
        Q: 55,
        breakdown: { PHI: 90, VERIFY: 30, CULTURE: 55, BURN: 50 },
        verdict: 'WAG',
      };

      const critique = refinement.critiqueJudgment(judgment);

      assert.ok(critique.critiques.some(c => c.type === 'imbalance'), 'Should detect imbalance');
    });

    it('should detect overconfidence', () => {
      const judgment = {
        Q: 70,
        breakdown: { PHI: 70, VERIFY: 70, CULTURE: 70, BURN: 70 },
        verdict: 'WAG',
        confidence: 0.85, // Above φ⁻¹ (0.618)
      };

      const critique = refinement.critiqueJudgment(judgment);

      assert.ok(critique.critiques.some(c => c.type === 'overconfidence'), 'Should detect overconfidence');
    });

    it('should detect threshold edge cases', () => {
      const judgment = {
        Q: 49, // Near 50 (WAG threshold)
        breakdown: { PHI: 50, VERIFY: 48, CULTURE: 50, BURN: 48 },
        verdict: 'GROWL',
      };

      const critique = refinement.critiqueJudgment(judgment);

      assert.ok(critique.critiques.some(c => c.type === 'threshold_edge'), 'Should detect threshold edge');
    });

    it('should return not refinable for clean judgments', () => {
      const judgment = {
        Q: 75,
        breakdown: { PHI: 78, VERIFY: 72, CULTURE: 76, BURN: 74 },
        verdict: 'WAG',
        confidence: 0.6,
      };

      const critique = refinement.critiqueJudgment(judgment);

      // May have low severity issues but not refinable
      assert.ok(critique.severity === 'none' || critique.severity === 'low');
    });

  });

  describe('suggestRefinement', () => {

    it('should suggest increasing weak axiom scores', () => {
      const judgment = {
        breakdown: { PHI: 60, VERIFY: 20, CULTURE: 55, BURN: 50 },
      };

      const critique = {
        originalQ: 45,
        critiques: [{
          type: 'weak_axiom',
          data: { axiom: 'VERIFY', score: 20 },
        }],
      };

      const refinedResult = refinement.suggestRefinement(judgment, critique);

      assert.ok(refinedResult.adjustments.length > 0, 'Should have adjustments');
      assert.ok(refinedResult.refinedBreakdown.VERIFY > 20, 'VERIFY should be increased');
    });

    it('should cap confidence at φ⁻¹', () => {
      const judgment = {
        breakdown: { PHI: 70, VERIFY: 70, CULTURE: 70, BURN: 70 },
      };

      const critique = {
        originalQ: 70,
        critiques: [{
          type: 'overconfidence',
          data: { confidence: 0.85, ceiling: refinement.PHI_INV },
        }],
      };

      const refinedResult = refinement.suggestRefinement(judgment, critique);

      const confAdj = refinedResult.adjustments.find(a => a.field === 'confidence');
      assert.ok(confAdj, 'Should have confidence adjustment');
      assert.strictEqual(confAdj.to, refinement.PHI_INV, 'Should cap at φ⁻¹');
    });

  });

  describe('selfRefine', () => {

    it('should improve weak judgments', () => {
      const judgment = {
        Q: 35,
        qScore: 35,
        verdict: 'GROWL',
        confidence: 0.75,
        breakdown: { PHI: 50, VERIFY: 15, CULTURE: 40, BURN: 35 },
      };

      const result = refinement.selfRefine(judgment, {}, { maxIterations: 3 });

      assert.ok(result.improved, 'Should be improved');
      assert.ok(result.final.Q > result.original.Q, 'Final Q should be higher');
      assert.ok(result.totalImprovement > 0, 'Should have positive improvement');
    });

    it('should stop when no more improvement possible', () => {
      const judgment = {
        Q: 75,
        qScore: 75,
        verdict: 'WAG',
        confidence: 0.6,
        breakdown: { PHI: 78, VERIFY: 72, CULTURE: 76, BURN: 74 },
      };

      const result = refinement.selfRefine(judgment, {}, { maxIterations: 5 });

      // Should stop early since judgment is already good
      assert.ok(result.iterationCount <= 2, 'Should stop early for good judgments');
    });

    it('should respect maxIterations', () => {
      const judgment = {
        Q: 25,
        qScore: 25,
        verdict: 'BARK',
        breakdown: { PHI: 30, VERIFY: 10, CULTURE: 25, BURN: 30 },
      };

      const result = refinement.selfRefine(judgment, {}, { maxIterations: 1 });

      assert.strictEqual(result.iterationCount, 1, 'Should respect maxIterations');
    });

  });

  describe('extractLearning', () => {

    it('should extract learnings from refinement', () => {
      const refinementResult = {
        iterations: [
          {
            refinement: {
              adjustments: [
                { axiom: 'VERIFY', from: 20, to: 30, reason: 'Underscored' },
                { field: 'confidence', from: 0.8, to: 0.618, reason: 'φ⁻¹ ceiling' },
              ],
            },
          },
        ],
        improved: true,
        totalImprovement: 5,
      };

      const learning = refinement.extractLearning(refinementResult);

      assert.ok(learning.learnings.length === 2, 'Should have 2 learnings');
      assert.ok(learning.learnings.some(l => l.type === 'axiom_adjustment'));
      assert.ok(learning.learnings.some(l => l.type === 'confidence_calibration'));
      assert.strictEqual(learning.improved, true);
    });

  });

  describe('getStats', () => {

    it('should return refinement statistics', () => {
      const stats = refinement.getStats();

      assert.ok('total' in stats, 'Should have total');
      assert.ok('improved' in stats, 'Should have improved');
      assert.ok('improvementRate' in stats, 'Should have improvementRate');
      assert.ok('avgImprovement' in stats, 'Should have avgImprovement');
    });

  });

});

describe('Constants', () => {

  it('should export PHI_INV as golden ratio inverse', () => {
    assert.ok(Math.abs(refinement.PHI_INV - 0.618033988749895) < 0.0001);
  });

  it('should have correct thresholds', () => {
    assert.strictEqual(refinement.THRESHOLDS.HOWL, 80);
    assert.strictEqual(refinement.THRESHOLDS.WAG, 50);
    assert.strictEqual(refinement.THRESHOLDS.GROWL, 30);
  });

});
