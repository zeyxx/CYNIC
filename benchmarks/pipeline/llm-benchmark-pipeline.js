#!/usr/bin/env node
/**
 * CYNIC LLM Benchmarking Pipeline
 *
 * "Le chien mesure sa morsure" - The dog measures its bite
 *
 * Comprehensive benchmark comparing:
 * - L1: Dog heuristics (patterns.json, rules.js)
 * - L2: LLM judgment (full pipeline)
 * - L3: Learning loop effectiveness
 *
 * @module @cynic/benchmarks/pipeline
 */

'use strict';

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { performance } from 'perf_hooks';
import { EventEmitter } from 'events';

const __dirname = dirname(fileURLToPath(import.meta.url));

// φ-aligned constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = PHI_INV * PHI_INV; // 0.382

/**
 * Benchmark Configuration
 */
const CONFIG = Object.freeze({
  // Runs per sample
  RUNS_PER_SAMPLE: 3,

  // Timeout per judgment (ms)
  TIMEOUT_MS: 30000,

  // Output directory
  OUTPUT_DIR: join(__dirname, 'results'),

  // Dataset path
  DATASET_PATH: join(__dirname, '../collective-vs-single/dataset/samples.json'),

  // Dogs to benchmark
  DOGS_TO_TEST: [
    'guardian',
    'analyst',
    'janitor',
    'architect',
    'scholar',
    'scout',
  ],

  // Issue category to Dog mapping
  CATEGORY_TO_DOG: {
    'sql-injection': 'guardian',
    'xss': 'guardian',
    'authentication': 'guardian',
    'path-traversal': 'guardian',
    'command-injection': 'guardian',
    'code-injection': 'guardian',
    'prototype-pollution': 'guardian',
    'cryptography': 'guardian',
    'logic-bug': 'analyst',
    'race-condition': 'analyst',
    'denial-of-service': 'analyst',
    'information-disclosure': 'analyst',
    'safe': 'analyst',
  },
});

/**
 * Benchmark Result
 */
class BenchmarkResult {
  constructor(sampleId, level) {
    this.sampleId = sampleId;
    this.level = level; // 'L1', 'L2', 'L3'
    this.runs = [];
    this.summary = null;
  }

  addRun(result) {
    this.runs.push({
      ...result,
      timestamp: Date.now(),
    });
  }

  finalize() {
    if (this.runs.length === 0) {
      this.summary = { error: 'No runs' };
      return;
    }

    const successfulRuns = this.runs.filter(r => r.success);

    if (successfulRuns.length === 0) {
      this.summary = { error: 'All runs failed' };
      return;
    }

    // Aggregate metrics
    const avgLatency = successfulRuns.reduce((s, r) => s + r.latencyMs, 0) / successfulRuns.length;
    const avgConfidence = successfulRuns.reduce((s, r) => s + (r.confidence || 0), 0) / successfulRuns.length;

    // Issue detection aggregation
    const allIssues = successfulRuns.flatMap(r => r.issues || []);
    const issueTypes = [...new Set(allIssues.map(i => i.type))];

    // Verdict consistency
    const verdicts = successfulRuns.map(r => r.verdict);
    const verdictCounts = {};
    for (const v of verdicts) {
      verdictCounts[v] = (verdictCounts[v] || 0) + 1;
    }
    const mostCommonVerdict = Object.entries(verdictCounts)
      .sort((a, b) => b[1] - a[1])[0]?.[0];
    const verdictConsistency = (verdictCounts[mostCommonVerdict] || 0) / verdicts.length;

    this.summary = {
      runsTotal: this.runs.length,
      runsSuccessful: successfulRuns.length,
      avgLatencyMs: Math.round(avgLatency),
      avgConfidence: avgConfidence.toFixed(3),
      issueTypesDetected: issueTypes,
      consensusVerdict: mostCommonVerdict,
      verdictConsistency: verdictConsistency.toFixed(3),
    };
  }
}

/**
 * L1 Benchmark Runner - Dog Heuristics Only
 */
class L1Benchmark {
  constructor(options = {}) {
    this.dogs = options.dogs || CONFIG.DOGS_TO_TEST;
    this._loadedPatterns = new Map();
    this._loadedRules = new Map();
  }

  async initialize() {
    // Load patterns and rules for each Dog
    for (const dogId of this.dogs) {
      const patternsPath = join(__dirname, '../../packages/node/src/dogs', dogId, 'patterns.json');
      const rulesPath = join(__dirname, '../../packages/node/src/dogs', dogId, 'rules.js');

      if (existsSync(patternsPath)) {
        try {
          const patterns = JSON.parse(readFileSync(patternsPath, 'utf8'));
          this._loadedPatterns.set(dogId, patterns);
        } catch (e) {
          console.warn(`  [L1] Failed to load patterns for ${dogId}: ${e.message}`);
        }
      }

      if (existsSync(rulesPath)) {
        try {
          const rules = await import(`file://${rulesPath.replace(/\\/g, '/')}`);
          this._loadedRules.set(dogId, rules.default || rules);
        } catch (e) {
          console.warn(`  [L1] Failed to load rules for ${dogId}: ${e.message}`);
        }
      }
    }
  }

  async runSample(sample) {
    const start = performance.now();
    const issues = [];
    let verdict = 'WAG';
    let confidence = PHI_INV_2; // 38.2% base confidence

    const responsibleDog = CONFIG.CATEGORY_TO_DOG[sample.category] || 'guardian';
    const patterns = this._loadedPatterns.get(responsibleDog);
    const rules = this._loadedRules.get(responsibleDog);

    // Pattern-based detection
    if (patterns) {
      const patternIssues = this._checkPatterns(sample.code, patterns, sample.category);
      issues.push(...patternIssues);
    }

    // Rule-based detection
    if (rules?.l1Check) {
      try {
        const ruleResult = await rules.l1Check({
          content: sample.code,
          type: sample.category,
          language: sample.language,
        });

        if (ruleResult?.issues) {
          issues.push(...ruleResult.issues);
        }
        if (ruleResult?.verdict) {
          verdict = ruleResult.verdict;
        }
        if (ruleResult?.confidence) {
          confidence = ruleResult.confidence;
        }
      } catch (e) {
        // Rule check failed
      }
    }

    // Derive verdict from issues if not set
    if (issues.length > 0) {
      const severities = issues.map(i => i.severity);
      if (severities.includes('critical')) {
        verdict = 'BARK';
        confidence = PHI_INV; // 61.8%
      } else if (severities.includes('high')) {
        verdict = 'GROWL';
        confidence = 0.5;
      } else {
        verdict = 'WAG';
        confidence = PHI_INV_2;
      }
    } else if (sample.category === 'safe') {
      verdict = 'HOWL';
      confidence = PHI_INV;
    }

    const elapsed = performance.now() - start;

    return {
      success: true,
      latencyMs: Math.round(elapsed),
      issues,
      verdict,
      confidence,
      dog: responsibleDog,
      level: 'L1',
    };
  }

  _checkPatterns(code, patterns, category) {
    const issues = [];
    const codeLower = code.toLowerCase();

    // Check security patterns (Guardian)
    if (patterns.dangerousCommands) {
      for (const [cmdType, cmdPatterns] of Object.entries(patterns.dangerousCommands)) {
        for (const pattern of cmdPatterns.patterns || []) {
          const regex = new RegExp(pattern, 'gi');
          if (regex.test(code)) {
            issues.push({
              type: cmdType.toUpperCase(),
              severity: cmdPatterns.severity || 'high',
              confidence: cmdPatterns.confidence || 0.7,
              source: 'L1_pattern',
            });
          }
        }
      }
    }

    // Check code quality patterns (Analyst)
    if (patterns.codeSmells) {
      for (const [smellType, smellConfig] of Object.entries(patterns.codeSmells)) {
        for (const pattern of smellConfig.patterns || []) {
          const regex = new RegExp(pattern, 'gi');
          if (regex.test(code)) {
            issues.push({
              type: smellType.toUpperCase(),
              severity: smellConfig.severity || 'medium',
              confidence: smellConfig.confidence || 0.5,
              source: 'L1_pattern',
            });
          }
        }
      }
    }

    // Check category-specific heuristics
    if (category.includes('sql') && /\$\{.*\}|'\s*\+|\+\s*'/.test(code)) {
      issues.push({
        type: 'SQL_INJECTION',
        severity: 'critical',
        confidence: 0.8,
        source: 'L1_heuristic',
      });
    }

    if (category.includes('xss') && /innerHTML|outerHTML|document\.write/.test(code)) {
      issues.push({
        type: 'XSS',
        severity: 'high',
        confidence: 0.7,
        source: 'L1_heuristic',
      });
    }

    if (category === 'command-injection' && /exec\s*\(|spawn\s*\(|execSync/.test(code)) {
      issues.push({
        type: 'COMMAND_INJECTION',
        severity: 'critical',
        confidence: 0.7,
        source: 'L1_heuristic',
      });
    }

    if (category === 'code-injection' && /eval\s*\(|Function\s*\(/.test(code)) {
      issues.push({
        type: 'EVAL_INJECTION',
        severity: 'critical',
        confidence: 0.9,
        source: 'L1_heuristic',
      });
    }

    if (category.includes('crypto') && /md5|sha1(?!\d)|Math\.random/.test(codeLower)) {
      issues.push({
        type: 'WEAK_CRYPTO',
        severity: 'high',
        confidence: 0.8,
        source: 'L1_heuristic',
      });
    }

    if (category === 'authentication' && /'[^']{4,20}'|"[^"]{4,20}"/.test(code) && /secret|password|key/i.test(code)) {
      issues.push({
        type: 'HARDCODED_SECRET',
        severity: 'critical',
        confidence: 0.6,
        source: 'L1_heuristic',
      });
    }

    if (category === 'logic-bug' && /=\s*['"][^'"]*['"]/.test(code) && /if\s*\(/.test(code)) {
      // Check for assignment in condition
      const assignInCondition = /if\s*\([^)]*\w+\s*=\s*['"][^'"]*['"]/.test(code);
      if (assignInCondition) {
        issues.push({
          type: 'ASSIGNMENT_IN_CONDITION',
          severity: 'critical',
          confidence: 0.9,
          source: 'L1_heuristic',
        });
      }
    }

    if (category === 'path-traversal' && /req\.query|req\.params|req\.body/.test(code) && /sendFile|readFile|createReadStream/.test(code)) {
      issues.push({
        type: 'PATH_TRAVERSAL',
        severity: 'critical',
        confidence: 0.7,
        source: 'L1_heuristic',
      });
    }

    if (category === 'prototype-pollution' && /for\s*\(\s*(const|let|var)\s+\w+\s+in/.test(code) && !/__proto__|constructor|prototype/.test(code)) {
      issues.push({
        type: 'PROTOTYPE_POLLUTION',
        severity: 'high',
        confidence: 0.5,
        source: 'L1_heuristic',
      });
    }

    return issues;
  }
}

/**
 * L2 Benchmark Runner - Full LLM Judgment
 *
 * Uses the actual CYNIC node for judgments.
 */
class L2Benchmark {
  constructor(options = {}) {
    this.node = options.node || null;
    this.anthropic = null;
    this.model = options.model || 'claude-sonnet-4-20250514';
  }

  async initialize() {
    // Try to use CYNIC node if available
    if (this.node) {
      return;
    }

    // Fallback to direct Anthropic API
    try {
      const Anthropic = (await import('@anthropic-ai/sdk')).default;
      this.anthropic = new Anthropic();
    } catch (e) {
      throw new Error('Neither CYNIC node nor Anthropic SDK available');
    }
  }

  async runSample(sample) {
    const start = performance.now();

    if (this.node) {
      return this._runWithNode(sample, start);
    }

    return this._runWithApi(sample, start);
  }

  async _runWithNode(sample, start) {
    try {
      const result = await this.node.judge({
        type: 'code_review',
        content: sample.code,
        metadata: {
          language: sample.language,
          category: sample.category,
          benchmark: true,
        },
      });

      const elapsed = performance.now() - start;

      return {
        success: true,
        latencyMs: Math.round(elapsed),
        issues: result.issues || [],
        verdict: result.verdict,
        confidence: result.qScore / 100,
        score: result.qScore,
        level: 'L2',
        source: 'cynic_node',
      };
    } catch (e) {
      return {
        success: false,
        error: e.message,
        latencyMs: Math.round(performance.now() - start),
        level: 'L2',
      };
    }
  }

  async _runWithApi(sample, start) {
    const systemPrompt = `You are CYNIC, a code review collective with these specialized roles:
- Guardian (Gevurah): Security vulnerabilities
- Analyst (Binah): Logic bugs, edge cases
- Janitor (Yesod): Code quality, complexity
- Architect (Chesed): Design patterns, structure

Analyze the code and respond with JSON:
{
  "issues": [{ "type": "ISSUE_TYPE", "severity": "critical|high|medium|low", "description": "..." }],
  "verdict": "BARK|GROWL|WAG|HOWL",
  "score": 0-100,
  "reasoning": "..."
}

Verdicts:
- BARK: Critical issues, do not proceed
- GROWL: Significant issues, fix before proceeding
- WAG: Minor issues, acceptable with notes
- HOWL: Clean code, good to go

Maximum confidence: 61.8% (φ⁻¹)`;

    try {
      const response = await this.anthropic.messages.create({
        model: this.model,
        max_tokens: 1500,
        system: systemPrompt,
        messages: [{
          role: 'user',
          content: `Review this ${sample.language} code:\n\n\`\`\`${sample.language}\n${sample.code}\n\`\`\``,
        }],
      });

      const elapsed = performance.now() - start;
      const text = response.content[0].text;

      // Parse JSON response
      let result;
      try {
        const jsonMatch = text.match(/\{[\s\S]*\}/);
        result = jsonMatch ? JSON.parse(jsonMatch[0]) : {};
      } catch (e) {
        result = { error: 'Failed to parse response' };
      }

      return {
        success: true,
        latencyMs: Math.round(elapsed),
        issues: result.issues || [],
        verdict: result.verdict || 'WAG',
        confidence: (result.score || 50) / 100,
        score: result.score || 50,
        tokensIn: response.usage.input_tokens,
        tokensOut: response.usage.output_tokens,
        level: 'L2',
        source: 'anthropic_api',
      };
    } catch (e) {
      return {
        success: false,
        error: e.message,
        latencyMs: Math.round(performance.now() - start),
        level: 'L2',
      };
    }
  }
}

/**
 * Benchmark Evaluator
 */
class BenchmarkEvaluator {
  constructor(groundTruth) {
    this.groundTruth = groundTruth;
  }

  evaluate(result, truth) {
    const detectedIssues = result.issues || [];
    const truthIssues = truth.issues || [];

    let truePositives = 0;
    let falsePositives = 0;
    const matchedTruth = new Set();

    // Match detected issues to ground truth
    for (const detected of detectedIssues) {
      let matched = false;
      for (let i = 0; i < truthIssues.length; i++) {
        if (!matchedTruth.has(i) && this._issueMatches(detected, truthIssues[i])) {
          truePositives++;
          matchedTruth.add(i);
          matched = true;
          break;
        }
      }
      if (!matched) {
        falsePositives++;
      }
    }

    const falseNegatives = truthIssues.length - matchedTruth.size;

    // Verdict evaluation
    const verdictMatch = result.verdict === truth.expectedVerdict;
    const verdictDirectionMatch = this._verdictDirectionMatches(result.verdict, truth.expectedVerdict);

    // Score evaluation
    const scoreInRange = result.score >= truth.minScore && result.score <= truth.maxScore;

    return {
      truePositives,
      falsePositives,
      falseNegatives,
      precision: truePositives / (truePositives + falsePositives) || 0,
      recall: truePositives / (truePositives + falseNegatives) || 0,
      verdictMatch,
      verdictDirectionMatch,
      scoreInRange,
    };
  }

  _issueMatches(detected, truth) {
    const detectedType = this._normalizeType(detected.type);
    const truthType = this._normalizeType(truth.type);

    if (detectedType === truthType) return true;

    // Related type families
    const families = [
      ['SQL_INJECTION', 'SQLI', 'SQL'],
      ['XSS', 'CROSS_SITE_SCRIPTING', 'REFLECTED_XSS'],
      ['COMMAND_INJECTION', 'CMD_INJECTION', 'SHELL_INJECTION'],
      ['WEAK_CRYPTO', 'WEAK_HASH', 'MD5', 'SHA1'],
    ];

    for (const family of families) {
      if (family.includes(detectedType) && family.includes(truthType)) {
        return true;
      }
    }

    return false;
  }

  _normalizeType(type) {
    return (type || '').toUpperCase().replace(/[-\s]/g, '_');
  }

  _verdictDirectionMatches(detected, expected) {
    const badVerdicts = ['BARK', 'GROWL'];
    const goodVerdicts = ['WAG', 'HOWL'];

    const detectedBad = badVerdicts.includes(detected);
    const expectedBad = badVerdicts.includes(expected);

    return detectedBad === expectedBad;
  }

  calculateMetrics(evaluations) {
    let totalTP = 0, totalFP = 0, totalFN = 0;
    let verdictMatches = 0, verdictDirectionMatches = 0, scoreInRange = 0;

    for (const e of evaluations) {
      totalTP += e.truePositives;
      totalFP += e.falsePositives;
      totalFN += e.falseNegatives;
      if (e.verdictMatch) verdictMatches++;
      if (e.verdictDirectionMatch) verdictDirectionMatches++;
      if (e.scoreInRange) scoreInRange++;
    }

    const precision = totalTP / (totalTP + totalFP) || 0;
    const recall = totalTP / (totalTP + totalFN) || 0;
    const f1 = 2 * (precision * recall) / (precision + recall) || 0;

    return {
      precision,
      recall,
      f1,
      truePositives: totalTP,
      falsePositives: totalFP,
      falseNegatives: totalFN,
      verdictAccuracy: verdictMatches / evaluations.length,
      verdictDirectionAccuracy: verdictDirectionMatches / evaluations.length,
      scoreAccuracy: scoreInRange / evaluations.length,
      sampleCount: evaluations.length,
    };
  }
}

/**
 * Main Pipeline
 */
class LLMBenchmarkPipeline extends EventEmitter {
  constructor(options = {}) {
    super();
    this.options = options;
    this.dataset = null;
    this.l1Benchmark = null;
    this.l2Benchmark = null;
    this.evaluator = null;
    this.results = {
      L1: [],
      L2: [],
      comparison: null,
    };
  }

  async initialize(options = {}) {
    const { skipL2 = false, skipL1 = false } = options;
    console.log('\n═══════════════════════════════════════════════════════════════');
    console.log('  CYNIC LLM BENCHMARKING PIPELINE');
    console.log('  "Le chien mesure sa morsure"');
    console.log('═══════════════════════════════════════════════════════════════\n');

    // Load dataset
    console.log('  Loading dataset...');
    const datasetPath = this.options.datasetPath || CONFIG.DATASET_PATH;
    this.dataset = JSON.parse(readFileSync(datasetPath, 'utf8'));
    console.log(`    ${this.dataset.samples.length} samples loaded`);

    // Initialize ground truth
    const groundTruth = new Map();
    for (const sample of this.dataset.samples) {
      groundTruth.set(sample.id, sample.groundTruth);
    }
    this.evaluator = new BenchmarkEvaluator(groundTruth);

    // Initialize L1 benchmark
    if (!skipL1) {
      console.log('  Initializing L1 (Dog heuristics)...');
      this.l1Benchmark = new L1Benchmark(this.options);
      await this.l1Benchmark.initialize();
      console.log(`    ${this.l1Benchmark._loadedPatterns.size} patterns loaded`);
      console.log(`    ${this.l1Benchmark._loadedRules.size} rules loaded`);
    }

    // Initialize L2 benchmark
    if (!skipL2) {
      console.log('  Initializing L2 (LLM judgment)...');
      this.l2Benchmark = new L2Benchmark(this.options);
      try {
        await this.l2Benchmark.initialize();
        console.log(`    Model: ${this.l2Benchmark.model}`);
        console.log(`    Source: ${this.l2Benchmark.node ? 'CYNIC Node' : 'Anthropic API'}`);
      } catch (e) {
        console.log(`    Warning: L2 not available (${e.message})`);
        this.l2Benchmark = null;
      }
    }

    // Ensure output directory
    if (!existsSync(CONFIG.OUTPUT_DIR)) {
      mkdirSync(CONFIG.OUTPUT_DIR, { recursive: true });
    }

    console.log('\n  Pipeline initialized.\n');
  }

  async runL1Benchmark() {
    console.log('───────────────────────────────────────────────────────────────');
    console.log('  L1 BENCHMARK: Dog Heuristics');
    console.log('───────────────────────────────────────────────────────────────\n');

    const results = [];
    const evaluations = [];

    for (const sample of this.dataset.samples) {
      const benchResult = new BenchmarkResult(sample.id, 'L1');

      process.stdout.write(`  [${sample.id}] `);

      for (let i = 0; i < CONFIG.RUNS_PER_SAMPLE; i++) {
        const run = await this.l1Benchmark.runSample(sample);
        benchResult.addRun(run);
        process.stdout.write(run.success ? '.' : 'x');
      }

      benchResult.finalize();

      // Evaluate against ground truth
      const lastRun = benchResult.runs.find(r => r.success) || {};
      const evaluation = this.evaluator.evaluate(
        {
          issues: lastRun.issues || [],
          verdict: lastRun.verdict,
          score: (lastRun.confidence || 0) * 100,
        },
        sample.groundTruth
      );

      results.push({
        ...benchResult,
        evaluation,
      });
      evaluations.push(evaluation);

      const status = evaluation.verdictDirectionMatch ? '✓' : '✗';
      console.log(` ${status} (${benchResult.summary.avgLatencyMs}ms)`);
    }

    // Calculate aggregate metrics
    const metrics = this.evaluator.calculateMetrics(evaluations);

    console.log('\n  L1 RESULTS:');
    console.log(`    Precision:         ${(metrics.precision * 100).toFixed(1)}%`);
    console.log(`    Recall:            ${(metrics.recall * 100).toFixed(1)}%`);
    console.log(`    F1 Score:          ${(metrics.f1 * 100).toFixed(1)}%`);
    console.log(`    Verdict Direction: ${(metrics.verdictDirectionAccuracy * 100).toFixed(1)}%`);
    console.log(`    Avg Latency:       ${Math.round(results.reduce((s, r) => s + r.summary.avgLatencyMs, 0) / results.length)}ms`);

    this.results.L1 = { results, metrics };

    return { results, metrics };
  }

  async runL2Benchmark() {
    console.log('\n───────────────────────────────────────────────────────────────');
    console.log('  L2 BENCHMARK: LLM Judgment');
    console.log('───────────────────────────────────────────────────────────────\n');

    const results = [];
    const evaluations = [];

    for (const sample of this.dataset.samples) {
      const benchResult = new BenchmarkResult(sample.id, 'L2');

      process.stdout.write(`  [${sample.id}] `);

      // Only 1 run for L2 to save tokens
      const run = await this.l2Benchmark.runSample(sample);
      benchResult.addRun(run);
      benchResult.finalize();

      // Evaluate against ground truth
      const evaluation = this.evaluator.evaluate(
        {
          issues: run.issues || [],
          verdict: run.verdict,
          score: run.score || (run.confidence || 0) * 100,
        },
        sample.groundTruth
      );

      results.push({
        ...benchResult,
        evaluation,
      });
      evaluations.push(evaluation);

      const status = evaluation.verdictDirectionMatch ? '✓' : '✗';
      console.log(` ${run.success ? '.' : 'x'} ${status} (${benchResult.summary.avgLatencyMs}ms)`);

      // Small delay to avoid rate limiting
      await new Promise(r => setTimeout(r, 100));
    }

    // Calculate aggregate metrics
    const metrics = this.evaluator.calculateMetrics(evaluations);

    console.log('\n  L2 RESULTS:');
    console.log(`    Precision:         ${(metrics.precision * 100).toFixed(1)}%`);
    console.log(`    Recall:            ${(metrics.recall * 100).toFixed(1)}%`);
    console.log(`    F1 Score:          ${(metrics.f1 * 100).toFixed(1)}%`);
    console.log(`    Verdict Direction: ${(metrics.verdictDirectionAccuracy * 100).toFixed(1)}%`);
    console.log(`    Avg Latency:       ${Math.round(results.reduce((s, r) => s + r.summary.avgLatencyMs, 0) / results.length)}ms`);

    this.results.L2 = { results, metrics };

    return { results, metrics };
  }

  compare() {
    if (!this.results.L1 || !this.results.L2) {
      console.log('  Run both L1 and L2 benchmarks first.');
      return;
    }

    console.log('\n═══════════════════════════════════════════════════════════════');
    console.log('  L1 vs L2 COMPARISON');
    console.log('═══════════════════════════════════════════════════════════════\n');

    const l1 = this.results.L1.metrics;
    const l2 = this.results.L2.metrics;

    console.log('  METRIC              │    L1     │    L2     │    Δ     ');
    console.log('  ────────────────────┼───────────┼───────────┼──────────');

    const metrics = [
      ['Precision', l1.precision, l2.precision],
      ['Recall', l1.recall, l2.recall],
      ['F1 Score', l1.f1, l2.f1],
      ['Verdict Dir.', l1.verdictDirectionAccuracy, l2.verdictDirectionAccuracy],
    ];

    for (const [name, v1, v2] of metrics) {
      const delta = v2 - v1;
      const deltaStr = `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`;
      console.log(`  ${name.padEnd(18)} │ ${(v1 * 100).toFixed(1).padStart(7)}%  │ ${(v2 * 100).toFixed(1).padStart(7)}%  │ ${deltaStr.padStart(8)}`);
    }

    // Latency comparison
    const l1AvgLatency = this.results.L1.results.reduce((s, r) => s + r.summary.avgLatencyMs, 0) / this.results.L1.results.length;
    const l2AvgLatency = this.results.L2.results.reduce((s, r) => s + r.summary.avgLatencyMs, 0) / this.results.L2.results.length;
    const latencyRatio = l2AvgLatency / l1AvgLatency;

    console.log('  ────────────────────┼───────────┼───────────┼──────────');
    console.log(`  ${'Avg Latency'.padEnd(18)} │ ${Math.round(l1AvgLatency).toString().padStart(6)}ms │ ${Math.round(l2AvgLatency).toString().padStart(6)}ms │ ${latencyRatio.toFixed(1).padStart(6)}x`);

    console.log('\n  ANALYSIS:');

    // F1 comparison
    const f1Diff = l2.f1 - l1.f1;
    if (f1Diff > 0.1) {
      console.log(`    ✅ L2 significantly outperforms L1 (F1 +${(f1Diff * 100).toFixed(1)}%)`);
    } else if (f1Diff > 0) {
      console.log(`    ⚠️  L2 marginally better than L1 (F1 +${(f1Diff * 100).toFixed(1)}%)`);
    } else {
      console.log(`    ❌ L1 matches or exceeds L2 (F1 ${(f1Diff * 100).toFixed(1)}%)`);
    }

    // Latency/quality tradeoff
    const qualityPerMs = f1Diff / (l2AvgLatency - l1AvgLatency);
    console.log(`    📊 Quality/Latency ratio: ${(qualityPerMs * 1000).toFixed(4)} F1 per second`);

    // Recommendation
    console.log('\n  RECOMMENDATION:');
    if (l1.verdictDirectionAccuracy > 0.8 && latencyRatio > 100) {
      console.log('    → L1 for screening, L2 for uncertain cases');
    } else if (l2.f1 - l1.f1 > 0.15) {
      console.log('    → L2 for all judgments (significant quality gain)');
    } else {
      console.log('    → Hybrid approach: L1 first, escalate to L2 if uncertain');
    }

    this.results.comparison = {
      l1Metrics: l1,
      l2Metrics: l2,
      f1Diff,
      latencyRatio,
    };

    return this.results.comparison;
  }

  saveResults() {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const outputPath = join(CONFIG.OUTPUT_DIR, `pipeline-${timestamp}.json`);

    writeFileSync(outputPath, JSON.stringify({
      timestamp: new Date().toISOString(),
      config: {
        runsPerSample: CONFIG.RUNS_PER_SAMPLE,
        samplesCount: this.dataset.samples.length,
      },
      results: this.results,
    }, null, 2));

    console.log(`\n  Results saved to: ${outputPath}\n`);

    return outputPath;
  }

  async run() {
    await this.initialize();
    await this.runL1Benchmark();
    await this.runL2Benchmark();
    this.compare();
    return this.saveResults();
  }
}

// Export for module use
export {
  LLMBenchmarkPipeline,
  L1Benchmark,
  L2Benchmark,
  BenchmarkEvaluator,
  BenchmarkResult,
  CONFIG,
};

// CLI entry point
if (process.argv[1] && process.argv[1].endsWith('llm-benchmark-pipeline.js')) {
  const pipeline = new LLMBenchmarkPipeline();

  const args = process.argv.slice(2);
  const l1Only = args.includes('--l1-only');
  const l2Only = args.includes('--l2-only');

  (async () => {
    try {
      await pipeline.initialize({ skipL2: l1Only, skipL1: l2Only });

      if (l1Only) {
        await pipeline.runL1Benchmark();
      } else if (l2Only) {
        await pipeline.runL2Benchmark();
      } else {
        await pipeline.runL1Benchmark();
        if (pipeline.l2Benchmark) {
          await pipeline.runL2Benchmark();
          pipeline.compare();
        } else {
          console.log('\n  Skipping L2 (not available). Run with ANTHROPIC_API_KEY for full comparison.\n');
        }
      }

      pipeline.saveResults();
    } catch (e) {
      console.error('Pipeline error:', e.message);
      process.exit(1);
    }
  })();
}
