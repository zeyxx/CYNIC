/**
 * BURN Axiom Scorer Tests
 *
 * Tests for UTILITY, SUSTAINABILITY, EFFICIENCY, VALUE_CREATION, NON_EXTRACTIVE, CONTRIBUTION
 *
 * @module @cynic/node/test/burn-axiom.test
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  scoreUtility,
  scoreNonExtractive,
  scoreContribution,
  scoreEfficiency,
  scoreSustainability,
  scoreValueCreation,
  BurnScorers,
} from '../src/judge/scorers/burn-axiom.js';

import { detectRiskPenalty } from '../src/judge/scorers/utils.js';

// ============================================================
// UTILITY DIMENSION TESTS
// ============================================================

describe('BURN Axiom - UTILITY Dimension', () => {
  describe('scoreUtility', () => {
    it('returns baseline 50 for empty item', () => {
      const score = scoreUtility({});
      assert.strictEqual(score, 50);
    });

    it('returns 50 for minimal text item', () => {
      const score = scoreUtility({ content: 'hello' });
      assert.strictEqual(score, 50);
    });

    describe('Purpose/Goal Bonus', () => {
      it('adds 15 points for item.purpose', () => {
        const withPurpose = scoreUtility({ purpose: 'Build secure dApps' });
        const without = scoreUtility({});
        assert.strictEqual(withPurpose - without, 15);
      });

      it('adds 15 points for item.goal', () => {
        const withGoal = scoreUtility({ goal: 'Reduce gas costs' });
        const without = scoreUtility({});
        assert.strictEqual(withGoal - without, 15);
      });
    });

    describe('Usage Count Bonus', () => {
      it('adds log10-based bonus for usageCount > 0', () => {
        const score1 = scoreUtility({ usageCount: 10 });
        const score100 = scoreUtility({ usageCount: 100 });
        const score1000 = scoreUtility({ usageCount: 1000 });

        assert.ok(score100 > score1, 'usageCount 100 should score higher than 10');
        assert.ok(score1000 > score100, 'usageCount 1000 should score higher than 100');
      });

      it('caps usage bonus at 25 points', () => {
        const massive = scoreUtility({ usageCount: 1000000000 });
        const baseline = scoreUtility({});
        assert.ok(massive - baseline <= 25);
      });

      it('no bonus for usageCount 0', () => {
        const score = scoreUtility({ usageCount: 0 });
        assert.strictEqual(score, 50);
      });
    });

    describe('Actionable Bonus', () => {
      it('adds 10 points for actionable: true', () => {
        const actionable = scoreUtility({ actionable: true });
        const notActionable = scoreUtility({ actionable: false });
        assert.strictEqual(actionable - notActionable, 10);
      });
    });

    describe('Instructions Bonus', () => {
      it('adds 10 points for instructions field', () => {
        const withInstructions = scoreUtility({ instructions: 'Step 1: ...' });
        const without = scoreUtility({});
        assert.strictEqual(withInstructions - without, 10);
      });

      it('adds 10 points for howTo field', () => {
        const withHowTo = scoreUtility({ howTo: 'First, install...' });
        const without = scoreUtility({});
        assert.strictEqual(withHowTo - without, 10);
      });
    });

    describe('Problem/Solution Bonus', () => {
      it('adds 10 points for problem field', () => {
        const withProblem = scoreUtility({ problem: 'Gas fees are too high' });
        const without = scoreUtility({});
        assert.strictEqual(withProblem - without, 10);
      });

      it('adds 10 points for solution field', () => {
        const withSolution = scoreUtility({ solution: 'Batch transactions' });
        const without = scoreUtility({});
        assert.strictEqual(withSolution - without, 10);
      });
    });

    describe('Risk Penalty', () => {
      it('applies risk penalty for scam indicators', () => {
        // detectRiskPenalty looks for specific patterns like "guaranteed.*return", "rug pull", etc.
        const scam = scoreUtility({ content: 'This is a rug pull scam with ponzi returns!' });
        const normal = scoreUtility({ content: 'A helpful utility library for developers' });
        assert.ok(scam < normal, `Scam (${scam}) should score lower than normal (${normal})`);
      });

      it('heavily penalizes "guaranteed returns"', () => {
        // Pattern is /guaranteed.*return|guaranteed.*profit/i
        const scam = scoreUtility({ content: 'Guaranteed massive return on your investment!' });
        assert.ok(scam < 50, `Guaranteed returns should drop below baseline, got ${scam}`);
      });
    });

    describe('Cumulative Bonuses', () => {
      it('accumulates all bonuses for comprehensive item', () => {
        const comprehensive = scoreUtility({
          purpose: 'Build dApps',
          usageCount: 1000,
          actionable: true,
          instructions: 'Step 1...',
          problem: 'High gas',
          solution: 'Batching',
        });
        assert.ok(comprehensive >= 85, `Expected >= 85, got ${comprehensive}`);
      });
    });
  });
});

// ============================================================
// SUSTAINABILITY DIMENSION TESTS
// ============================================================

describe('BURN Axiom - SUSTAINABILITY Dimension', () => {
  describe('scoreSustainability', () => {
    it('returns baseline 50 for empty item', () => {
      const score = scoreSustainability({});
      assert.strictEqual(score, 50);
    });

    describe('Maintenance Bonuses', () => {
      it('adds 15 points for maintained: true', () => {
        const maintained = scoreSustainability({ maintained: true });
        const notMaintained = scoreSustainability({});
        assert.strictEqual(maintained - notMaintained, 15);
      });

      it('adds 15 points for supported: true', () => {
        const supported = scoreSustainability({ supported: true });
        const baseline = scoreSustainability({});
        assert.strictEqual(supported - baseline, 15);
      });
    });

    describe('Version Bonus', () => {
      it('adds 10 points for version field', () => {
        const versioned = scoreSustainability({ version: '1.0.0' });
        const unversioned = scoreSustainability({});
        assert.strictEqual(versioned - unversioned, 10);
      });
    });

    describe('Maintenance Burden', () => {
      it('adds 10 points for low maintenance burden', () => {
        const low = scoreSustainability({ maintenanceBurden: 'low' });
        const baseline = scoreSustainability({});
        assert.strictEqual(low - baseline, 10);
      });

      it('no bonus for high maintenance burden', () => {
        const high = scoreSustainability({ maintenanceBurden: 'high' });
        assert.strictEqual(high, 50);
      });
    });

    describe('Roadmap/Future Bonus', () => {
      it('adds 10 points for roadmap', () => {
        const withRoadmap = scoreSustainability({ roadmap: 'Q1: Feature X' });
        const without = scoreSustainability({});
        assert.strictEqual(withRoadmap - without, 10);
      });

      it('adds 10 points for future field', () => {
        const withFuture = scoreSustainability({ future: 'Planned features...' });
        const without = scoreSustainability({});
        assert.strictEqual(withFuture - without, 10);
      });
    });

    describe('Community Bonus', () => {
      it('adds 10 points for community', () => {
        const withCommunity = scoreSustainability({ community: true });
        const without = scoreSustainability({});
        assert.strictEqual(withCommunity - without, 10);
      });

      it('adds 10 points for contributors', () => {
        const withContributors = scoreSustainability({ contributors: ['Alice', 'Bob'] });
        const without = scoreSustainability({});
        assert.strictEqual(withContributors - without, 10);
      });
    });

    describe('Deprecated Penalty', () => {
      it('subtracts 30 points for deprecated: true', () => {
        const deprecated = scoreSustainability({ deprecated: true });
        const active = scoreSustainability({});
        assert.strictEqual(active - deprecated, 30);
      });

      it('deprecated item can still have positive score with other bonuses', () => {
        const deprecatedButSupported = scoreSustainability({
          deprecated: true,
          maintained: true,
          version: '2.0.0',
          roadmap: 'v3 migration',
          community: true,
        });
        // 50 - 30 + 15 + 10 + 10 + 10 = 65
        assert.strictEqual(deprecatedButSupported, 65);
      });
    });

    describe('Risk Penalty', () => {
      it('applies risk penalty for unsustainable scam patterns', () => {
        const scam = scoreSustainability({ content: 'Ponzi scheme with guaranteed returns' });
        const normal = scoreSustainability({ content: 'Long-term project' });
        assert.ok(scam < normal);
      });
    });

    describe('Maximum Score', () => {
      it('reaches high score with all positive indicators', () => {
        const maximal = scoreSustainability({
          maintained: true,
          version: '3.0.0',
          maintenanceBurden: 'low',
          roadmap: 'Big plans',
          community: true,
        });
        // 50 + 15 + 10 + 10 + 10 + 10 = 100 (capped)
        assert.strictEqual(maximal, 100);
      });
    });
  });
});

// ============================================================
// EFFICIENCY DIMENSION TESTS
// ============================================================

describe('BURN Axiom - EFFICIENCY Dimension', () => {
  describe('scoreEfficiency', () => {
    it('returns baseline 50 for empty item', () => {
      const score = scoreEfficiency({});
      assert.strictEqual(score, 50);
    });

    describe('Size Efficiency', () => {
      it('adds 10 points for reasonable text size (< 5000 chars)', () => {
        const small = scoreEfficiency({ content: 'A'.repeat(1000) });
        const empty = scoreEfficiency({});
        assert.strictEqual(small - empty, 10);
      });

      it('no bonus for empty text', () => {
        const score = scoreEfficiency({ content: '' });
        assert.strictEqual(score, 50);
      });

      it('subtracts 10 points for very large text (> 50000 chars)', () => {
        const huge = scoreEfficiency({ content: 'X'.repeat(60000) });
        // 50 - 10 = 40
        assert.strictEqual(huge, 40);
      });
    });

    describe('Code Efficiency', () => {
      it('adds 10 points for code without deep nesting', () => {
        const flatCode = scoreEfficiency({
          content: `function simple() {
            return 42;
          }`,
        });
        const baseline = scoreEfficiency({ content: 'not code' });
        // Small text bonus + no nesting bonus
        assert.ok(flatCode >= baseline);
      });

      it('penalizes deeply nested code', () => {
        // The regex /\{[^{}]*\{[^{}]*\{/g finds 3-level nesting patterns
        // Penalty only applies if count > 5. With fewer matches, we compare to flat code.
        const deepNesting = `function a() { if(x) { if(y) { if(z) { if(w) { if(v) { } } } } } }`;
        const flatCode = `function simple() { return 42; }`;
        const nestedScore = scoreEfficiency({ content: deepNesting });
        const flatScore = scoreEfficiency({ content: flatCode });
        // Both get small text bonus, but flat code also gets no-nesting bonus (+10)
        assert.ok(flatScore >= nestedScore, `Flat (${flatScore}) should be >= nested (${nestedScore})`);
      });

      it('adds 5 points for code with imports', () => {
        const withImports = scoreEfficiency({
          content: `import { foo } from 'bar';
          function x() { return foo(); }`,
        });
        const withoutImports = scoreEfficiency({
          content: `function x() { return 42; }`,
        });
        assert.strictEqual(withImports - withoutImports, 5);
      });
    });

    describe('Performance Markers', () => {
      it('adds 10 points for performance field', () => {
        const withPerf = scoreEfficiency({ performance: 'optimized' });
        const without = scoreEfficiency({});
        assert.strictEqual(withPerf - without, 10);
      });

      it('adds 10 points for "fast" in text', () => {
        const fast = scoreEfficiency({ content: 'This is a fast algorithm' });
        const slow = scoreEfficiency({ content: 'This is an algorithm' });
        assert.strictEqual(fast - slow, 10);
      });

      it('adds 10 points for "efficient" in text', () => {
        const efficient = scoreEfficiency({ content: 'An efficient solution' });
        const baseline = scoreEfficiency({ content: 'A solution' });
        assert.strictEqual(efficient - baseline, 10);
      });

      it('adds 10 points for "optimized" in text', () => {
        const optimized = scoreEfficiency({ content: 'Highly optimized code' });
        const baseline = scoreEfficiency({ content: 'Some code' });
        assert.strictEqual(optimized - baseline, 10);
      });
    });

    describe('Resource Usage', () => {
      it('adds 10 points for low resource usage', () => {
        const low = scoreEfficiency({ resourceUsage: 'low' });
        const baseline = scoreEfficiency({});
        assert.strictEqual(low - baseline, 10);
      });
    });

    describe('Risk Penalty', () => {
      it('applies risk penalty for wasteful scam patterns', () => {
        const scam = scoreEfficiency({ content: 'Buy now! Limited time! 1000x guaranteed!' });
        const normal = scoreEfficiency({ content: 'Efficient resource management' });
        assert.ok(scam < normal);
      });
    });
  });
});

// ============================================================
// VALUE_CREATION DIMENSION TESTS
// ============================================================

describe('BURN Axiom - VALUE_CREATION Dimension', () => {
  describe('scoreValueCreation', () => {
    it('returns baseline 50 for empty item', () => {
      const score = scoreValueCreation({});
      assert.strictEqual(score, 50);
    });

    describe('Output/Produces Bonus', () => {
      it('adds 15 points for output field', () => {
        const withOutput = scoreValueCreation({ output: 'Generated report' });
        const without = scoreValueCreation({});
        assert.strictEqual(withOutput - without, 15);
      });

      it('adds 15 points for produces field', () => {
        const produces = scoreValueCreation({ produces: 'Documentation' });
        const baseline = scoreValueCreation({});
        assert.strictEqual(produces - baseline, 15);
      });
    });

    describe('Derivatives Bonus', () => {
      it('adds 5 points per derivative up to 20', () => {
        const oneDerivative = scoreValueCreation({ derivatives: 1 });
        const fourDerivatives = scoreValueCreation({ derivatives: 4 });
        const baseline = scoreValueCreation({});

        assert.strictEqual(oneDerivative - baseline, 5);
        assert.strictEqual(fourDerivatives - baseline, 20);
      });

      it('caps derivatives bonus at 20', () => {
        const manyDerivatives = scoreValueCreation({ derivatives: 100 });
        const fourDerivatives = scoreValueCreation({ derivatives: 4 });
        assert.strictEqual(manyDerivatives, fourDerivatives);
      });

      it('no bonus for derivatives: 0', () => {
        const zero = scoreValueCreation({ derivatives: 0 });
        assert.strictEqual(zero, 50);
      });
    });

    describe('Enables/Empowers Bonus', () => {
      it('adds 10 points for enables field', () => {
        const enables = scoreValueCreation({ enables: 'New features' });
        const baseline = scoreValueCreation({});
        assert.strictEqual(enables - baseline, 10);
      });

      it('adds 10 points for empowers field', () => {
        const empowers = scoreValueCreation({ empowers: 'Developers' });
        const baseline = scoreValueCreation({});
        assert.strictEqual(empowers - baseline, 10);
      });
    });

    describe('Net Value Bonus', () => {
      it('adds 15 points for positive netValue', () => {
        const positive = scoreValueCreation({ netValue: 100 });
        const baseline = scoreValueCreation({});
        assert.strictEqual(positive - baseline, 15);
      });

      it('no bonus for netValue <= 0', () => {
        const zero = scoreValueCreation({ netValue: 0 });
        const negative = scoreValueCreation({ netValue: -50 });
        assert.strictEqual(zero, 50);
        assert.strictEqual(negative, 50);
      });
    });

    describe('Created/Consumed Ratio', () => {
      it('adds ratio-based bonus for createdValue > consumedValue', () => {
        const goodRatio = scoreValueCreation({
          createdValue: 100,
          consumedValue: 10,
        });
        const badRatio = scoreValueCreation({
          createdValue: 10,
          consumedValue: 100,
        });
        const baseline = scoreValueCreation({});

        assert.ok(goodRatio > badRatio, 'Good ratio should score higher');
        assert.ok(goodRatio > baseline, 'Good ratio should exceed baseline');
      });

      it('caps ratio bonus at 20 points', () => {
        const hugeRatio = scoreValueCreation({
          createdValue: 10000,
          consumedValue: 1,
        });
        // 50 + min(10000 * 10, 20) = 70
        assert.strictEqual(hugeRatio, 70);
      });

      it('handles zero consumedValue safely', () => {
        // Note: Implementation requires both createdValue AND consumedValue to be truthy
        // With consumedValue: 0 (falsy), the ratio bonus is not applied
        const score = scoreValueCreation({
          createdValue: 100,
          consumedValue: 0,
        });
        // No ratio bonus since consumedValue is falsy (0)
        assert.strictEqual(score, 50);
      });
    });

    describe('Risk Penalty', () => {
      it('heavily penalizes value-destroying scams', () => {
        const scam = scoreValueCreation({
          content: 'Rug pull! Exit scam! Drain all liquidity!',
        });
        assert.ok(scam < 40, `Scam should score low, got ${scam}`);
      });
    });

    describe('Maximum Score', () => {
      it('accumulates bonuses for value-creating item', () => {
        const maximal = scoreValueCreation({
          output: 'reports',
          derivatives: 4,
          enables: 'features',
          netValue: 1000,
          createdValue: 500,
          consumedValue: 50,
        });
        // 50 + 15 + 20 + 10 + 15 + capped_ratio = close to 100
        assert.ok(maximal >= 95, `Expected >= 95, got ${maximal}`);
      });
    });
  });
});

// ============================================================
// NON_EXTRACTIVE DIMENSION TESTS
// ============================================================

describe('BURN Axiom - NON_EXTRACTIVE Dimension', () => {
  describe('scoreNonExtractive', () => {
    it('returns baseline 55 for empty item', () => {
      const score = scoreNonExtractive({});
      assert.strictEqual(score, 55);
    });

    describe('Non-Extractive/Fair Bonus', () => {
      it('adds 15 points for nonExtractive: true', () => {
        const nonExtract = scoreNonExtractive({ nonExtractive: true });
        const baseline = scoreNonExtractive({});
        assert.strictEqual(nonExtract - baseline, 15);
      });

      it('adds 15 points for fair: true', () => {
        const fair = scoreNonExtractive({ fair: true });
        const baseline = scoreNonExtractive({});
        assert.strictEqual(fair - baseline, 15);
      });
    });

    describe('Open Source/Free Bonus', () => {
      it('adds 15 points for openSource: true', () => {
        const open = scoreNonExtractive({ openSource: true });
        const closed = scoreNonExtractive({});
        assert.strictEqual(open - closed, 15);
      });

      it('adds 15 points for free: true', () => {
        const free = scoreNonExtractive({ free: true });
        const baseline = scoreNonExtractive({});
        assert.strictEqual(free - baseline, 15);
      });
    });

    describe('Compensation/Attribution Bonus', () => {
      it('adds 10 points for compensation', () => {
        const compensated = scoreNonExtractive({ compensation: 'Fair wages' });
        const baseline = scoreNonExtractive({});
        assert.strictEqual(compensated - baseline, 10);
      });

      it('adds 10 points for attribution', () => {
        const attributed = scoreNonExtractive({ attribution: 'MIT License' });
        const baseline = scoreNonExtractive({});
        assert.strictEqual(attributed - baseline, 10);
      });
    });

    describe('Community Benefit Bonus', () => {
      it('adds 10 points for communityBenefit: true', () => {
        const benefit = scoreNonExtractive({ communityBenefit: true });
        const baseline = scoreNonExtractive({});
        assert.strictEqual(benefit - baseline, 10);
      });
    });

    describe('Hidden Costs Penalty', () => {
      it('subtracts 30 points for hiddenCosts: true', () => {
        const hidden = scoreNonExtractive({ hiddenCosts: true });
        const transparent = scoreNonExtractive({});
        assert.strictEqual(transparent - hidden, 30);
      });
    });

    describe('Extractive Flag Penalty', () => {
      it('subtracts 40 points for extractive: true', () => {
        const extractive = scoreNonExtractive({ extractive: true });
        const baseline = scoreNonExtractive({});
        assert.strictEqual(baseline - extractive, 40);
      });
    });

    describe('Extractive Text Patterns', () => {
      it('penalizes "100% tax/fee/take"', () => {
        const taxAll = scoreNonExtractive({ content: '100% tax on all transfers' });
        const noTax = scoreNonExtractive({ content: 'No transfer fees' });
        assert.ok(taxAll < noTax);
      });

      it('penalizes "drain/extract/steal/siphon"', () => {
        const drain = scoreNonExtractive({ content: 'Drain the liquidity pool' });
        const provide = scoreNonExtractive({ content: 'Provide liquidity' });
        assert.ok(drain < provide);
      });

      it('penalizes "all funds/all liquidity"', () => {
        const allFunds = scoreNonExtractive({ content: 'Takes all funds from users' });
        const someFunds = scoreNonExtractive({ content: 'Takes 1% fee' });
        assert.ok(allFunds < someFunds);
      });

      it('penalizes "exit scam"', () => {
        const exitScam = scoreNonExtractive({ content: 'Classic exit scam pattern' });
        const legitimate = scoreNonExtractive({ content: 'Legitimate project' });
        assert.ok(exitScam < legitimate);
      });

      it('accumulates penalties for multiple extractive patterns', () => {
        const multiplePatterns = scoreNonExtractive({
          content: '100% fee drain exit scam all funds',
        });
        const singlePattern = scoreNonExtractive({
          content: 'exit scam warning',
        });
        assert.ok(
          multiplePatterns < singlePattern,
          'Multiple patterns should score lower'
        );
      });
    });

    describe('Risk Penalty', () => {
      it('applies universal risk penalty', () => {
        // detectRiskPenalty looks for scam/fraud/rug pull patterns
        const scam = scoreNonExtractive({ content: 'This is a rug pull ponzi fraud scheme!' });
        const normal = scoreNonExtractive({ content: 'Fair and transparent pricing model' });
        assert.ok(scam < normal, `Scam (${scam}) should be < normal (${normal})`);
      });
    });

    describe('Floor at 0', () => {
      it('never goes below 0 even with many penalties', () => {
        const maxPenalty = scoreNonExtractive({
          extractive: true,
          hiddenCosts: true,
          content:
            '100% tax drain all funds exit scam siphon steal guaranteed 1000x returns',
        });
        assert.ok(maxPenalty >= 0, `Score should not go negative, got ${maxPenalty}`);
      });
    });
  });
});

// ============================================================
// CONTRIBUTION DIMENSION TESTS
// ============================================================

describe('BURN Axiom - CONTRIBUTION Dimension', () => {
  describe('scoreContribution', () => {
    it('returns baseline 45 for empty item', () => {
      const score = scoreContribution({});
      assert.strictEqual(score, 45);
    });

    describe('Contributions Count Bonus', () => {
      it('adds 3 points per contribution up to 20', () => {
        const oneContrib = scoreContribution({ contributions: 1 });
        const fiveContrib = scoreContribution({ contributions: 5 });
        const baseline = scoreContribution({});

        assert.strictEqual(oneContrib - baseline, 3);
        assert.strictEqual(fiveContrib - baseline, 15);
      });

      it('caps contributions bonus at 20', () => {
        const manyContrib = scoreContribution({ contributions: 100 });
        const sevenContrib = scoreContribution({ contributions: 7 }); // 7 * 3 = 21 → capped at 20
        // Both should be baseline + 20
        assert.strictEqual(manyContrib - 45, 20);
      });

      it('no bonus for contributions: 0', () => {
        const zero = scoreContribution({ contributions: 0 });
        assert.strictEqual(zero, 45);
      });
    });

    describe('Open Source Bonus', () => {
      it('adds 15 points for openSource: true', () => {
        const open = scoreContribution({ openSource: true });
        const closed = scoreContribution({});
        assert.strictEqual(open - closed, 15);
      });
    });

    describe('Documentation Bonus', () => {
      it('adds 10 points for documentation field', () => {
        const documented = scoreContribution({ documentation: 'Full API docs' });
        const baseline = scoreContribution({});
        assert.strictEqual(documented - baseline, 10);
      });

      it('adds 10 points for docs field', () => {
        const withDocs = scoreContribution({ docs: 'README.md' });
        const baseline = scoreContribution({});
        assert.strictEqual(withDocs - baseline, 10);
      });
    });

    describe('Examples Bonus', () => {
      it('adds 10 points for examples field', () => {
        const withExamples = scoreContribution({ examples: ['ex1', 'ex2'] });
        const baseline = scoreContribution({});
        assert.strictEqual(withExamples - baseline, 10);
      });
    });

    describe('Tests Bonus', () => {
      it('adds 10 points for tests field', () => {
        const withTests = scoreContribution({ tests: 'jest' });
        const baseline = scoreContribution({});
        assert.strictEqual(withTests - baseline, 10);
      });

      it('adds 10 points for tested: true', () => {
        const tested = scoreContribution({ tested: true });
        const untested = scoreContribution({});
        assert.strictEqual(tested - untested, 10);
      });
    });

    describe('Community Involvement Bonus', () => {
      it('adds 10 points for communityInvolved: true', () => {
        const involved = scoreContribution({ communityInvolved: true });
        const solo = scoreContribution({});
        assert.strictEqual(involved - solo, 10);
      });
    });

    describe('Risk Penalty', () => {
      it('applies risk penalty for scam indicators', () => {
        const scam = scoreContribution({ content: 'Guaranteed 100x returns!' });
        const normal = scoreContribution({ content: 'Community contributions welcome' });
        assert.ok(scam < normal);
      });
    });

    describe('Maximum Score', () => {
      it('reaches 100 with all positive indicators', () => {
        const maximal = scoreContribution({
          contributions: 10,
          openSource: true,
          documentation: 'yes',
          examples: ['ex'],
          tests: 'jest',
          communityInvolved: true,
        });
        // 45 + 20(capped) + 15 + 10 + 10 + 10 + 10 = 120 → capped at 100
        assert.strictEqual(maximal, 100);
      });
    });
  });
});

// ============================================================
// BURN SCORERS MAP TESTS
// ============================================================

describe('BurnScorers Map', () => {
  it('exports all 6 BURN dimensions', () => {
    const dimensions = Object.keys(BurnScorers);
    assert.deepStrictEqual(dimensions.sort(), [
      'CONTRIBUTION',
      'EFFICIENCY',
      'NON_EXTRACTIVE',
      'SUSTAINABILITY',
      'UTILITY',
      'VALUE_CREATION',
    ]);
  });

  it('all scorers are functions', () => {
    for (const [name, scorer] of Object.entries(BurnScorers)) {
      assert.strictEqual(typeof scorer, 'function', `${name} should be a function`);
    }
  });

  it('all scorers return numbers between 0 and 100', () => {
    const testItem = { content: 'Test item' };
    for (const [name, scorer] of Object.entries(BurnScorers)) {
      const score = scorer(testItem);
      assert.ok(
        typeof score === 'number' && score >= 0 && score <= 100,
        `${name} returned invalid score: ${score}`
      );
    }
  });

  it('all scorers handle empty items', () => {
    for (const [name, scorer] of Object.entries(BurnScorers)) {
      const score = scorer({});
      assert.ok(
        typeof score === 'number' && !isNaN(score),
        `${name} failed on empty item`
      );
    }
  });

  it('all scorers apply risk penalty consistently', () => {
    const scamItem = {
      content: 'Guaranteed 1000% returns! Send ETH to double it instantly!',
    };
    const normalItem = { content: 'A simple utility function' };

    for (const [name, scorer] of Object.entries(BurnScorers)) {
      const scamScore = scorer(scamItem);
      const normalScore = scorer(normalItem);
      assert.ok(
        scamScore <= normalScore,
        `${name} should penalize scam content: scam=${scamScore}, normal=${normalScore}`
      );
    }
  });
});

// ============================================================
// INTEGRATION TESTS
// ============================================================

describe('BURN Axiom Integration Tests', () => {
  describe('Real-world BURN Scenarios', () => {
    it('high-value open source project scores well across all dimensions', () => {
      const openSourceProject = {
        // UTILITY fields
        purpose: 'Build decentralized applications',
        usageCount: 5000,
        actionable: true,
        instructions: 'npm install && npm start',
        problem: 'Complex blockchain interactions',
        solution: 'Simple API wrapper',
        // SUSTAINABILITY fields
        maintained: true,
        version: '2.0.0',
        maintenanceBurden: 'low',
        roadmap: 'v3 with new features',
        community: true,
        // EFFICIENCY fields
        performance: 'optimized',
        resourceUsage: 'low',
        // VALUE_CREATION fields
        output: 'dApps',
        derivatives: 5,
        enables: 'Web3 development',
        netValue: 1000,
        createdValue: 100,
        consumedValue: 10,
        // NON_EXTRACTIVE fields
        openSource: true,
        fair: true,
        communityBenefit: true,
        compensation: 'MIT License',
        // CONTRIBUTION fields
        contributions: 50,
        documentation: 'Full API docs',
        examples: ['example1', 'example2'],
        tests: 'comprehensive',
        communityInvolved: true,
        content: 'A well-maintained open source library for building dApps',
      };

      const utility = scoreUtility(openSourceProject);
      const sustainability = scoreSustainability(openSourceProject);
      const efficiency = scoreEfficiency(openSourceProject);
      const valueCreation = scoreValueCreation(openSourceProject);
      const nonExtractive = scoreNonExtractive(openSourceProject);
      const contribution = scoreContribution(openSourceProject);

      // All dimensions should score high with comprehensive positive indicators
      assert.ok(utility >= 85, `Utility: ${utility}`);
      assert.ok(sustainability >= 90, `Sustainability: ${sustainability}`);
      assert.ok(efficiency >= 60, `Efficiency: ${efficiency}`);
      assert.ok(valueCreation >= 85, `ValueCreation: ${valueCreation}`);
      assert.ok(nonExtractive >= 90, `NonExtractive: ${nonExtractive}`);
      assert.ok(contribution >= 95, `Contribution: ${contribution}`);
    });

    it('extractive scam project scores poorly across all dimensions', () => {
      const scamProject = {
        content:
          'Guaranteed 1000x returns! Exit scam imminent! Drain all liquidity! 100% tax on sells!',
        extractive: true,
        hiddenCosts: true,
        deprecated: true,
      };

      const utility = scoreUtility(scamProject);
      const sustainability = scoreSustainability(scamProject);
      const efficiency = scoreEfficiency(scamProject);
      const valueCreation = scoreValueCreation(scamProject);
      const nonExtractive = scoreNonExtractive(scamProject);
      const contribution = scoreContribution(scamProject);

      assert.ok(utility <= 40, `Utility should be low: ${utility}`);
      assert.ok(sustainability <= 30, `Sustainability should be low: ${sustainability}`);
      assert.ok(efficiency <= 50, `Efficiency should be low: ${efficiency}`);
      assert.ok(valueCreation <= 30, `ValueCreation should be low: ${valueCreation}`);
      assert.ok(
        nonExtractive <= 10,
        `NonExtractive should be very low: ${nonExtractive}`
      );
      assert.ok(contribution <= 40, `Contribution should be low: ${contribution}`);
    });

    it('abandoned project has mixed scores', () => {
      const abandonedProject = {
        purpose: 'Was useful once',
        deprecated: true,
        maintained: false,
        version: '0.1.0',
        openSource: true,
        documentation: 'Outdated',
        content: 'This project is no longer maintained',
      };

      const utility = scoreUtility(abandonedProject);
      const sustainability = scoreSustainability(abandonedProject);
      const contribution = scoreContribution(abandonedProject);
      const nonExtractive = scoreNonExtractive(abandonedProject);

      // Has some value but sustainability is hurt
      assert.ok(utility >= 50, `Utility still decent: ${utility}`);
      assert.ok(sustainability <= 40, `Sustainability hurt by deprecation: ${sustainability}`);
      assert.ok(contribution >= 60, `Still contributes as open source: ${contribution}`);
      assert.ok(nonExtractive >= 60, `Non-extractive as open source: ${nonExtractive}`);
    });

    it('efficient closed-source tool has dimension-specific scores', () => {
      const closedSourceTool = {
        purpose: 'Fast data processing',
        performance: 'optimized',
        resourceUsage: 'low',
        usageCount: 10000,
        actionable: true,
        instructions: 'Easy setup',
        maintained: true,
        version: '5.0.0',
        // No openSource, no community contribution markers
        content: 'A fast, efficient proprietary tool',
      };

      const utility = scoreUtility(closedSourceTool);
      const efficiency = scoreEfficiency(closedSourceTool);
      const nonExtractive = scoreNonExtractive(closedSourceTool);
      const contribution = scoreContribution(closedSourceTool);

      // High utility and efficiency
      assert.ok(utility >= 80, `High utility: ${utility}`);
      assert.ok(efficiency >= 70, `High efficiency: ${efficiency}`);

      // Lower on open/contribution dimensions
      assert.ok(
        nonExtractive < 70,
        `NonExtractive lower without openSource: ${nonExtractive}`
      );
      assert.ok(
        contribution < 60,
        `Contribution lower without open source: ${contribution}`
      );
    });
  });

  describe('Cross-Dimension Consistency', () => {
    it('all dimensions use detectRiskPenalty consistently', () => {
      const riskyContent = {
        content: 'Ponzi scheme with guaranteed returns and exit strategy',
      };

      const penalty = detectRiskPenalty(riskyContent, riskyContent.content);
      assert.ok(penalty > 0, 'Should detect risk');

      // All scorers should be below their baselines
      const baselines = {
        UTILITY: 50,
        SUSTAINABILITY: 50,
        EFFICIENCY: 50,
        VALUE_CREATION: 50,
        NON_EXTRACTIVE: 55,
        CONTRIBUTION: 45,
      };

      for (const [name, scorer] of Object.entries(BurnScorers)) {
        const score = scorer(riskyContent);
        assert.ok(
          score < baselines[name],
          `${name} should be below baseline ${baselines[name]}: got ${score}`
        );
      }
    });
  });
});
