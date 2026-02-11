/**
 * Experiment Runner — Capture, replay, ablation for prompt classification
 *
 * Pure code experimentation: no LLM, no database.
 * Feed prompts → run variants → ablate sections → compare statistically.
 *
 * Usage:
 *   const runner = createExperimentRunner();
 *   runner.addPrompts(prompts);
 *   runner.runBaseline();
 *   runner.runVariant('expert', { sessionCount: 100 });
 *   runner.runAblation(['code_status', 'solana_context', 'market_data']);
 *   const report = runner.compare('baseline', 'expert');
 *
 * "Le chien mesure avant de creuser" — κυνικός
 *
 * @module @cynic/core/intelligence/experiment-runner
 */

'use strict';

import { PHI_INV } from '../axioms/constants.js';
import { classifyPrompt, scoreContextRelevance, selectSections } from './prompt-classifier.js';
import { createPhiGovernor } from './phi-governor.js';

// =============================================================================
// CONSTANTS
// =============================================================================

const MAX_PROMPTS = 1000;
const MIN_SAMPLES_FOR_SIGNIFICANCE = 5;

// =============================================================================
// EXPERIMENT RUNNER
// =============================================================================

/**
 * Create an experiment runner for classification pipeline testing.
 *
 * @param {Object} [options]
 * @param {Object} [options.sectionSizes] - Map of section → token size estimate
 * @param {string[]} [options.availableSections] - Sections available for injection
 * @returns {Object} Runner with addPrompts, runBaseline, runVariant, runAblation, compare
 */
export function createExperimentRunner(options = {}) {
  const {
    sectionSizes = {},
    availableSections = [],
  } = options;

  const _prompts = [];       // { text, metadata }
  const _runs = new Map();   // name → { options, results[] }

  return {
    /**
     * Add a single prompt to the experiment.
     *
     * @param {string} text - Prompt text
     * @param {Object} [metadata] - Optional metadata (domain, intent, etc.)
     */
    addPrompt(text, metadata = {}) {
      if (_prompts.length >= MAX_PROMPTS) return;
      _prompts.push({ text, metadata });
    },

    /**
     * Add multiple prompts at once.
     *
     * @param {Array<string|{text: string, metadata?: Object}>} prompts
     */
    addPrompts(prompts) {
      for (const p of prompts) {
        if (typeof p === 'string') {
          this.addPrompt(p);
        } else {
          this.addPrompt(p.text, p.metadata || {});
        }
      }
    },

    /**
     * Run baseline classification (default parameters).
     *
     * @returns {Object[]} Classification results
     */
    runBaseline() {
      return this.runVariant('baseline', {});
    },

    /**
     * Run a named variant with custom classifier options.
     *
     * @param {string} name - Variant name
     * @param {Object} classifierOptions - Options passed to classifyPrompt
     * @returns {Object[]} Classification results
     */
    runVariant(name, classifierOptions = {}) {
      const results = [];
      const gov = createPhiGovernor();

      for (const prompt of _prompts) {
        const classification = classifyPrompt(prompt.text, classifierOptions);

        // Score context relevance if sections available
        let relevanceScores = {};
        let selectedSections = [];
        if (availableSections.length > 0) {
          relevanceScores = scoreContextRelevance(classification, availableSections);
          const budget = gov.applyToBudget(classification.tokenBudget);
          selectedSections = selectSections(relevanceScores, budget, sectionSizes);
        }

        // Simulate governor measurement (assume balanced injection for baseline)
        const totalTokens = classification.tokenBudget * 2 || 1000;
        const injectedTokens = Math.round(totalTokens * PHI_INV);
        gov.measure(injectedTokens, totalTokens);

        results.push({
          prompt: prompt.text,
          metadata: prompt.metadata,
          classification,
          relevanceScores,
          selectedSections,
          governorAdjustment: gov.getAdjustment(),
          tokenBudget: classification.tokenBudget,
        });
      }

      _runs.set(name, { options: classifierOptions, results });
      return results;
    },

    /**
     * Run ablation study: for each section, remove it and measure impact.
     *
     * @param {string[]} [sections] - Sections to ablate (defaults to availableSections)
     * @param {Object} [classifierOptions] - Base classifier options
     * @returns {Object} Ablation results: { baseline, ablations: { section → results } }
     */
    runAblation(sections, classifierOptions = {}) {
      const ablationSections = sections || availableSections;

      // Run baseline with all sections
      const baselineName = '_ablation_baseline';
      this.runVariant(baselineName, classifierOptions);
      const baseline = _runs.get(baselineName);

      const ablations = {};

      for (const section of ablationSections) {
        // Create reduced section set
        const reduced = availableSections.filter(s => s !== section);
        const results = [];
        const gov = createPhiGovernor();

        for (let i = 0; i < _prompts.length; i++) {
          const prompt = _prompts[i];
          const classification = classifyPrompt(prompt.text, classifierOptions);

          let relevanceScores = {};
          let selectedSections = [];
          if (reduced.length > 0) {
            relevanceScores = scoreContextRelevance(classification, reduced);
            const budget = gov.applyToBudget(classification.tokenBudget);
            selectedSections = selectSections(relevanceScores, budget, sectionSizes);
          }

          const totalTokens = classification.tokenBudget * 2 || 1000;
          const injectedTokens = Math.round(totalTokens * PHI_INV);
          gov.measure(injectedTokens, totalTokens);

          // Calculate impact: how many prompts lost this section?
          const baselineSections = baseline.results[i]?.selectedSections || [];
          const sectionWasSelected = baselineSections.includes(section);

          results.push({
            prompt: prompt.text,
            classification,
            selectedSections,
            sectionWasSelected,
            tokenBudget: classification.tokenBudget,
            budgetDelta: classification.tokenBudget - (baseline.results[i]?.tokenBudget || 0),
          });
        }

        ablations[section] = results;
      }

      return { baseline: baseline.results, ablations };
    },

    /**
     * Compare two named runs statistically.
     *
     * @param {string} nameA - First run name
     * @param {string} nameB - Second run name
     * @returns {Object} Comparison: { budgetDiff, intentAgreement, domainCorrelation, significant }
     */
    compare(nameA, nameB) {
      const runA = _runs.get(nameA);
      const runB = _runs.get(nameB);
      if (!runA || !runB) {
        return { error: `Run not found: ${!runA ? nameA : nameB}` };
      }

      const n = Math.min(runA.results.length, runB.results.length);
      if (n < MIN_SAMPLES_FOR_SIGNIFICANCE) {
        return { significant: false, reason: 'insufficient_samples', samples: n };
      }

      // Budget comparison
      const budgetDiffs = [];
      let intentMatches = 0;
      let skipMatchA = 0;
      let skipMatchB = 0;

      for (let i = 0; i < n; i++) {
        const a = runA.results[i];
        const b = runB.results[i];

        budgetDiffs.push(b.tokenBudget - a.tokenBudget);

        if (a.classification.intent === b.classification.intent) intentMatches++;
        if (a.classification.skip) skipMatchA++;
        if (b.classification.skip) skipMatchB++;
      }

      const budgetStats = computeStats(budgetDiffs);
      const intentAgreement = intentMatches / n;

      // Paired t-test for budget differences
      const tStat = pairedTTest(budgetDiffs);
      const significant = Math.abs(tStat) > 2.0 && n >= MIN_SAMPLES_FOR_SIGNIFICANCE;

      return {
        samples: n,
        budgetDiff: {
          mean: budgetStats.mean,
          std: budgetStats.std,
          min: budgetStats.min,
          max: budgetStats.max,
        },
        intentAgreement,
        skipRateA: skipMatchA / n,
        skipRateB: skipMatchB / n,
        tStatistic: tStat,
        significant,
        effectSize: budgetStats.std > 0 ? budgetStats.mean / budgetStats.std : 0,
      };
    },

    /**
     * Summarize ablation results into ranked impact scores.
     *
     * @param {Object} ablationResult - From runAblation()
     * @returns {Object[]} Sections ranked by impact (highest first)
     */
    summarizeAblation(ablationResult) {
      const { baseline, ablations } = ablationResult;
      const summary = [];

      for (const [section, results] of Object.entries(ablations)) {
        let selectionCount = 0;
        let budgetImpact = 0;

        for (let i = 0; i < results.length; i++) {
          if (results[i].sectionWasSelected) selectionCount++;
          budgetImpact += results[i].budgetDelta;
        }

        const selectionRate = results.length > 0 ? selectionCount / results.length : 0;

        summary.push({
          section,
          selectionRate,
          avgBudgetImpact: results.length > 0 ? budgetImpact / results.length : 0,
          essentiality: selectionRate, // How often was this section chosen?
        });
      }

      // Sort by essentiality (most essential first)
      summary.sort((a, b) => b.essentiality - a.essentiality);
      return summary;
    },

    /**
     * Get all results for a named run.
     *
     * @param {string} name - Run name
     * @returns {Object|null} Run data or null
     */
    getRun(name) {
      return _runs.get(name) || null;
    },

    /**
     * Get list of all run names.
     */
    getRunNames() {
      return [..._runs.keys()];
    },

    /**
     * Get compact summary across all runs.
     */
    getSummary() {
      const summary = {};

      for (const [name, run] of _runs) {
        const budgets = run.results.map(r => r.tokenBudget);
        const intents = {};
        const skipCount = run.results.filter(r => r.classification.skip).length;

        for (const r of run.results) {
          const intent = r.classification.intent;
          intents[intent] = (intents[intent] || 0) + 1;
        }

        summary[name] = {
          prompts: run.results.length,
          options: run.options,
          budget: computeStats(budgets),
          intentDistribution: intents,
          skipRate: run.results.length > 0 ? skipCount / run.results.length : 0,
        };
      }

      return summary;
    },

    /**
     * Get prompt count.
     */
    getPromptCount() {
      return _prompts.length;
    },

    /**
     * Reset all runs (keep prompts).
     */
    resetRuns() {
      _runs.clear();
    },

    /**
     * Reset everything.
     */
    reset() {
      _prompts.length = 0;
      _runs.clear();
    },
  };
}

// =============================================================================
// STATISTICAL HELPERS
// =============================================================================

/**
 * Compute basic descriptive statistics.
 *
 * @param {number[]} values
 * @returns {Object} { mean, std, min, max, median }
 */
export function computeStats(values) {
  if (values.length === 0) {
    return { mean: 0, std: 0, min: 0, max: 0, median: 0 };
  }

  const n = values.length;
  const mean = values.reduce((s, v) => s + v, 0) / n;
  const variance = values.reduce((s, v) => s + Math.pow(v - mean, 2), 0) / n;
  const std = Math.sqrt(variance);

  const sorted = [...values].sort((a, b) => a - b);
  const median = n % 2 === 0
    ? (sorted[n / 2 - 1] + sorted[n / 2]) / 2
    : sorted[Math.floor(n / 2)];

  return {
    mean,
    std,
    min: sorted[0],
    max: sorted[n - 1],
    median,
  };
}

/**
 * Paired t-test statistic for differences.
 * Tests H0: mean difference = 0.
 *
 * @param {number[]} diffs - Paired differences
 * @returns {number} t-statistic
 */
export function pairedTTest(diffs) {
  const n = diffs.length;
  if (n < 2) return 0;

  const mean = diffs.reduce((s, d) => s + d, 0) / n;
  const variance = diffs.reduce((s, d) => s + Math.pow(d - mean, 2), 0) / (n - 1);
  const se = Math.sqrt(variance / n);

  if (se === 0) return 0;
  return mean / se;
}

/**
 * Generate a standard set of test prompts covering all domains and intents.
 * Useful for quick benchmarking without captured data.
 *
 * @returns {Array<{text: string, metadata: Object}>}
 */
export function generateTestPrompts() {
  return [
    // Skip prompts
    { text: 'ok', metadata: { expectedIntent: 'skip', expectedDomain: null } },
    { text: 'yes', metadata: { expectedIntent: 'skip', expectedDomain: null } },
    { text: '/commit', metadata: { expectedIntent: 'skip', expectedDomain: null } },

    // CODE domain
    { text: 'fix the bug in the parser function', metadata: { expectedIntent: 'debug', expectedDomain: 'CODE' } },
    { text: 'refactor the event bus architecture', metadata: { expectedIntent: 'architecture', expectedDomain: 'CODE' } },
    { text: 'write unit tests for the validator module', metadata: { expectedIntent: 'test', expectedDomain: 'CODE' } },
    { text: 'build the webpack configuration', metadata: { expectedIntent: 'build', expectedDomain: 'CODE' } },

    // SOLANA domain
    { text: 'send a solana transaction to mint tokens', metadata: { expectedIntent: 'general', expectedDomain: 'SOLANA' } },
    { text: 'check the devnet validator status', metadata: { expectedIntent: 'general', expectedDomain: 'SOLANA' } },
    { text: 'debug the anchor program deployment', metadata: { expectedIntent: 'debug', expectedDomain: 'SOLANA' } },

    // MARKET domain
    { text: 'check the current price and liquidity', metadata: { expectedIntent: 'general', expectedDomain: 'MARKET' } },
    { text: 'analyze the trading volume on dex', metadata: { expectedIntent: 'general', expectedDomain: 'MARKET' } },

    // SOCIAL domain
    { text: 'post a tweet about the community update', metadata: { expectedIntent: 'general', expectedDomain: 'SOCIAL' } },
    { text: 'check discord engagement metrics', metadata: { expectedIntent: 'general', expectedDomain: 'SOCIAL' } },

    // HUMAN domain
    { text: 'detect user burnout patterns in the session', metadata: { expectedIntent: 'general', expectedDomain: 'HUMAN' } },
    { text: 'analyze cognitive bias in the feedback', metadata: { expectedIntent: 'general', expectedDomain: 'HUMAN' } },

    // CYNIC domain
    { text: 'check cynic consciousness and dog verdicts', metadata: { expectedIntent: 'general', expectedDomain: 'CYNIC' } },
    { text: 'review the kabbalistic router topology', metadata: { expectedIntent: 'general', expectedDomain: 'CYNIC' } },

    // COSMOS domain
    { text: 'check cross-project ecosystem coherence', metadata: { expectedIntent: 'general', expectedDomain: 'COSMOS' } },
    { text: 'synchronize the monorepo dependency graph', metadata: { expectedIntent: 'general', expectedDomain: 'COSMOS' } },

    // Multi-domain
    { text: 'fix the solana transaction error in the code and update the ecosystem', metadata: { expectedIntent: 'debug', expectedDomain: 'multi' } },

    // Security
    { text: 'check for sql injection vulnerabilities in the auth module', metadata: { expectedIntent: 'security', expectedDomain: 'CODE' } },

    // Decision
    { text: 'should we use Redis or PostgreSQL for caching', metadata: { expectedIntent: 'decision', expectedDomain: 'CODE' } },

    // Danger
    { text: 'rm -rf the entire database directory', metadata: { expectedIntent: 'danger', expectedDomain: 'CODE' } },

    // Deploy
    { text: 'deploy to render production', metadata: { expectedIntent: 'deploy', expectedDomain: 'CODE' } },

    // Git
    { text: 'commit and push the changes', metadata: { expectedIntent: 'git', expectedDomain: 'CODE' } },

    // Complex
    { text: 'I need to implement a new authentication system with OAuth2 and JWT. The architecture needs redesign for future providers. Should we use passport.js? Consider security vulnerabilities and compare trade-offs. Also refactor the existing auth middleware.', metadata: { expectedIntent: 'architecture', expectedDomain: 'CODE', expectedComplexity: 'complex' } },
  ];
}
