/**
 * Experiment Runner Tests
 *
 * Tests the capture/replay/ablation pipeline for prompt classification.
 *
 * @module @cynic/core/test/experiment-runner
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  createExperimentRunner,
  computeStats,
  pairedTTest,
  generateTestPrompts,
} from '../src/intelligence/experiment-runner.js';

import { PHI_INV } from '../src/axioms/constants.js';

// =============================================================================
// Statistical helpers
// =============================================================================

describe('computeStats', () => {
  it('handles empty array', () => {
    const s = computeStats([]);
    assert.equal(s.mean, 0);
    assert.equal(s.std, 0);
  });

  it('single value', () => {
    const s = computeStats([42]);
    assert.equal(s.mean, 42);
    assert.equal(s.std, 0);
    assert.equal(s.median, 42);
  });

  it('correct mean and std', () => {
    const s = computeStats([2, 4, 4, 4, 5, 5, 7, 9]);
    assert.ok(Math.abs(s.mean - 5) < 0.001);
    assert.ok(s.std > 0);
  });

  it('correct median (odd count)', () => {
    const s = computeStats([1, 3, 5]);
    assert.equal(s.median, 3);
  });

  it('correct median (even count)', () => {
    const s = computeStats([1, 2, 3, 4]);
    assert.equal(s.median, 2.5);
  });

  it('min and max', () => {
    const s = computeStats([10, 20, 30]);
    assert.equal(s.min, 10);
    assert.equal(s.max, 30);
  });
});

describe('pairedTTest', () => {
  it('returns 0 for single value', () => {
    assert.equal(pairedTTest([5]), 0);
  });

  it('returns 0 for identical differences', () => {
    assert.equal(pairedTTest([3, 3, 3, 3]), 0);
  });

  it('detects large positive differences', () => {
    const t = pairedTTest([10, 12, 11, 13, 10]);
    assert.ok(t > 2, `expected t > 2, got ${t}`);
  });

  it('detects large negative differences', () => {
    const t = pairedTTest([-10, -12, -11, -13, -10]);
    assert.ok(t < -2, `expected t < -2, got ${t}`);
  });

  it('near-zero for mixed differences', () => {
    const t = pairedTTest([5, -5, 3, -3, 1, -1]);
    assert.ok(Math.abs(t) < 1, `expected |t| < 1, got ${t}`);
  });
});

// =============================================================================
// Test prompt generation
// =============================================================================

describe('generateTestPrompts', () => {
  it('returns non-empty array', () => {
    const prompts = generateTestPrompts();
    assert.ok(prompts.length > 20);
  });

  it('all prompts have text and metadata', () => {
    for (const p of generateTestPrompts()) {
      assert.equal(typeof p.text, 'string');
      assert.ok(p.text.length > 0);
      assert.equal(typeof p.metadata, 'object');
    }
  });

  it('covers all 7 domains', () => {
    const domains = new Set();
    for (const p of generateTestPrompts()) {
      if (p.metadata.expectedDomain) domains.add(p.metadata.expectedDomain);
    }
    for (const d of ['CODE', 'SOLANA', 'MARKET', 'SOCIAL', 'HUMAN', 'CYNIC', 'COSMOS']) {
      assert.ok(domains.has(d), `missing domain ${d}`);
    }
  });

  it('includes skip prompts', () => {
    const skips = generateTestPrompts().filter(p => p.metadata.expectedIntent === 'skip');
    assert.ok(skips.length >= 2);
  });
});

// =============================================================================
// Runner creation
// =============================================================================

describe('createExperimentRunner', () => {
  let runner;

  beforeEach(() => {
    runner = createExperimentRunner();
  });

  it('creates runner with all methods', () => {
    assert.equal(typeof runner.addPrompt, 'function');
    assert.equal(typeof runner.addPrompts, 'function');
    assert.equal(typeof runner.runBaseline, 'function');
    assert.equal(typeof runner.runVariant, 'function');
    assert.equal(typeof runner.runAblation, 'function');
    assert.equal(typeof runner.compare, 'function');
    assert.equal(typeof runner.summarizeAblation, 'function');
    assert.equal(typeof runner.getSummary, 'function');
    assert.equal(typeof runner.reset, 'function');
  });

  it('starts with zero prompts', () => {
    assert.equal(runner.getPromptCount(), 0);
  });

  it('starts with no runs', () => {
    assert.equal(runner.getRunNames().length, 0);
  });
});

// =============================================================================
// Adding prompts
// =============================================================================

describe('runner: adding prompts', () => {
  let runner;

  beforeEach(() => {
    runner = createExperimentRunner();
  });

  it('addPrompt increases count', () => {
    runner.addPrompt('fix the bug');
    assert.equal(runner.getPromptCount(), 1);
  });

  it('addPrompts with strings', () => {
    runner.addPrompts(['fix the bug', 'deploy to render']);
    assert.equal(runner.getPromptCount(), 2);
  });

  it('addPrompts with objects', () => {
    runner.addPrompts([
      { text: 'fix the bug', metadata: { domain: 'CODE' } },
      { text: 'check price', metadata: { domain: 'MARKET' } },
    ]);
    assert.equal(runner.getPromptCount(), 2);
  });

  it('addPrompts with mixed types', () => {
    runner.addPrompts([
      'simple string',
      { text: 'object form', metadata: { intent: 'debug' } },
    ]);
    assert.equal(runner.getPromptCount(), 2);
  });
});

// =============================================================================
// Running variants
// =============================================================================

describe('runner: variants', () => {
  let runner;

  beforeEach(() => {
    runner = createExperimentRunner();
    runner.addPrompts(generateTestPrompts());
  });

  it('runBaseline classifies all prompts', () => {
    const results = runner.runBaseline();
    assert.equal(results.length, runner.getPromptCount());
  });

  it('baseline results have classification', () => {
    const results = runner.runBaseline();
    for (const r of results) {
      assert.ok('classification' in r);
      assert.ok('tokenBudget' in r);
      assert.ok('prompt' in r);
    }
  });

  it('runVariant stores results by name', () => {
    runner.runVariant('test_v1', { sessionCount: 50 });
    assert.ok(runner.getRun('test_v1'));
    assert.ok(runner.getRunNames().includes('test_v1'));
  });

  it('expert variant has lower budgets', () => {
    runner.runBaseline();
    runner.runVariant('expert', { sessionCount: 100 });

    const baseline = runner.getRun('baseline');
    const expert = runner.getRun('expert');

    // Sum non-skip budgets
    const sumBudget = (results) => results
      .filter(r => !r.classification.skip)
      .reduce((s, r) => s + r.tokenBudget, 0);

    assert.ok(sumBudget(expert.results) < sumBudget(baseline.results),
      'expert should have lower total budget');
  });

  it('skip prompts have zero budget in all variants', () => {
    runner.runBaseline();
    const baseline = runner.getRun('baseline');
    const skipResults = baseline.results.filter(r => r.classification.skip);
    for (const r of skipResults) {
      assert.equal(r.tokenBudget, 0);
    }
  });
});

// =============================================================================
// Comparing runs
// =============================================================================

describe('runner: compare', () => {
  let runner;

  beforeEach(() => {
    runner = createExperimentRunner();
    runner.addPrompts(generateTestPrompts());
  });

  it('returns error for missing run', () => {
    const result = runner.compare('nonexistent', 'also_missing');
    assert.ok(result.error);
  });

  it('compares baseline vs expert', () => {
    runner.runBaseline();
    runner.runVariant('expert', { sessionCount: 100 });

    const cmp = runner.compare('baseline', 'expert');
    assert.ok('samples' in cmp);
    assert.ok('budgetDiff' in cmp);
    assert.ok('intentAgreement' in cmp);
    assert.ok('tStatistic' in cmp);
    assert.ok('significant' in cmp);
  });

  it('high intent agreement between same-parameter runs', () => {
    runner.runVariant('run_a', {});
    runner.runVariant('run_b', {});

    const cmp = runner.compare('run_a', 'run_b');
    assert.equal(cmp.intentAgreement, 1.0, 'same params should produce identical intents');
  });

  it('budget diff is negative for expert vs baseline', () => {
    runner.runBaseline();
    runner.runVariant('expert', { sessionCount: 100 });

    const cmp = runner.compare('baseline', 'expert');
    assert.ok(cmp.budgetDiff.mean < 0, 'expert should use less budget');
  });

  it('effect size is non-zero for different variants', () => {
    runner.runBaseline();
    runner.runVariant('expert', { sessionCount: 100 });

    const cmp = runner.compare('baseline', 'expert');
    assert.ok(cmp.effectSize !== 0);
  });
});

// =============================================================================
// Ablation
// =============================================================================

describe('runner: ablation', () => {
  const sections = ['code_status', 'solana_context', 'market_data', 'social_feed', 'ecosystem_overview'];
  const sectionSizes = {
    code_status: 200,
    solana_context: 300,
    market_data: 150,
    social_feed: 100,
    ecosystem_overview: 250,
  };

  let runner;

  beforeEach(() => {
    runner = createExperimentRunner({
      availableSections: sections,
      sectionSizes,
    });
    runner.addPrompts(generateTestPrompts());
  });

  it('returns baseline and ablations', () => {
    const result = runner.runAblation(sections);
    assert.ok(result.baseline);
    assert.ok(result.ablations);
    assert.equal(Object.keys(result.ablations).length, sections.length);
  });

  it('each ablation has results for all prompts', () => {
    const result = runner.runAblation(sections);
    for (const section of sections) {
      assert.equal(result.ablations[section].length, runner.getPromptCount());
    }
  });

  it('ablation results track sectionWasSelected', () => {
    const result = runner.runAblation(sections);
    for (const section of sections) {
      for (const r of result.ablations[section]) {
        assert.equal(typeof r.sectionWasSelected, 'boolean');
      }
    }
  });

  it('baseline selects sections for non-skip prompts', () => {
    const result = runner.runAblation(sections);
    const nonSkip = result.baseline.filter(r => !r.classification.skip);
    const hasSelection = nonSkip.some(r => r.selectedSections.length > 0);
    assert.ok(hasSelection, 'some non-skip prompts should select sections');
  });

  it('summarizeAblation ranks by essentiality', () => {
    const result = runner.runAblation(sections);
    const summary = runner.summarizeAblation(result);

    assert.equal(summary.length, sections.length);
    // Sorted descending by essentiality
    for (let i = 1; i < summary.length; i++) {
      assert.ok(summary[i - 1].essentiality >= summary[i].essentiality);
    }
  });

  it('summary includes section names and rates', () => {
    const result = runner.runAblation(sections);
    const summary = runner.summarizeAblation(result);

    for (const s of summary) {
      assert.ok(sections.includes(s.section));
      assert.equal(typeof s.selectionRate, 'number');
      assert.equal(typeof s.avgBudgetImpact, 'number');
      assert.equal(typeof s.essentiality, 'number');
    }
  });
});

// =============================================================================
// Summary
// =============================================================================

describe('runner: summary', () => {
  let runner;

  beforeEach(() => {
    runner = createExperimentRunner();
    runner.addPrompts(generateTestPrompts());
  });

  it('empty summary with no runs', () => {
    const summary = runner.getSummary();
    assert.equal(Object.keys(summary).length, 0);
  });

  it('summary includes all runs', () => {
    runner.runBaseline();
    runner.runVariant('expert', { sessionCount: 100 });

    const summary = runner.getSummary();
    assert.ok('baseline' in summary);
    assert.ok('expert' in summary);
  });

  it('summary has intent distribution', () => {
    runner.runBaseline();
    const summary = runner.getSummary();
    assert.ok(summary.baseline.intentDistribution);
    assert.ok('skip' in summary.baseline.intentDistribution);
  });

  it('summary has budget stats', () => {
    runner.runBaseline();
    const summary = runner.getSummary();
    assert.ok('mean' in summary.baseline.budget);
    assert.ok('std' in summary.baseline.budget);
  });

  it('summary has skip rate', () => {
    runner.runBaseline();
    const summary = runner.getSummary();
    assert.ok(summary.baseline.skipRate > 0, 'test prompts include skips');
    assert.ok(summary.baseline.skipRate < 1, 'not all prompts are skips');
  });
});

// =============================================================================
// Reset
// =============================================================================

describe('runner: reset', () => {
  it('resetRuns keeps prompts', () => {
    const runner = createExperimentRunner();
    runner.addPrompts(['a', 'b', 'c']);
    runner.runBaseline();
    runner.resetRuns();

    assert.equal(runner.getPromptCount(), 3);
    assert.equal(runner.getRunNames().length, 0);
  });

  it('reset clears everything', () => {
    const runner = createExperimentRunner();
    runner.addPrompts(['a', 'b', 'c']);
    runner.runBaseline();
    runner.reset();

    assert.equal(runner.getPromptCount(), 0);
    assert.equal(runner.getRunNames().length, 0);
  });
});

// =============================================================================
// Re-export
// =============================================================================

describe('experiment-runner re-export', () => {
  it('accessible from @cynic/core', async () => {
    const core = await import('../src/index.js');
    assert.equal(typeof core.createExperimentRunner, 'function');
    assert.equal(typeof core.computeStats, 'function');
    assert.equal(typeof core.pairedTTest, 'function');
    assert.equal(typeof core.generateTestPrompts, 'function');
  });
});
