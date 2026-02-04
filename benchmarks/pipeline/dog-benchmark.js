#!/usr/bin/env node
/**
 * Per-Dog Benchmark Runner
 *
 * Measures individual Dog detection accuracy for their domain.
 * Each Dog is tested on samples relevant to their specialization.
 *
 * "Chaque chien chasse dans son territoire"
 *
 * @module @cynic/benchmarks/dog-benchmark
 */

'use strict';

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { performance } from 'perf_hooks';

const __dirname = dirname(fileURLToPath(import.meta.url));

// φ-aligned thresholds
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

/**
 * Dog Specialization Mapping
 */
const DOG_DOMAINS = Object.freeze({
  guardian: {
    name: 'Guardian',
    sefira: 'Gevurah',
    description: 'Security vulnerabilities',
    categories: [
      'sql-injection',
      'xss',
      'authentication',
      'path-traversal',
      'command-injection',
      'code-injection',
      'prototype-pollution',
      'cryptography',
    ],
    issueTypes: [
      'SQL_INJECTION',
      'XSS',
      'COMMAND_INJECTION',
      'EVAL_INJECTION',
      'PATH_TRAVERSAL',
      'HARDCODED_SECRET',
      'WEAK_CRYPTO',
      'WEAK_RANDOM',
      'PROTOTYPE_POLLUTION',
      'TIMING_ATTACK',
      'NO_SALT',
    ],
  },
  analyst: {
    name: 'Analyst',
    sefira: 'Binah',
    description: 'Logic bugs, edge cases',
    categories: [
      'logic-bug',
      'race-condition',
      'denial-of-service',
      'information-disclosure',
    ],
    issueTypes: [
      'ASSIGNMENT_IN_CONDITION',
      'MISSING_VALIDATION',
      'RACE_CONDITION',
      'UNHANDLED_EXCEPTION',
      'NO_SIZE_LIMIT',
      'STACK_TRACE_EXPOSURE',
      'WEAK_COMPARISON',
    ],
  },
  janitor: {
    name: 'Janitor',
    sefira: 'Yesod',
    description: 'Code quality, complexity',
    categories: ['quality', 'complexity', 'dead-code'],
    issueTypes: [
      'DEAD_CODE',
      'HIGH_COMPLEXITY',
      'DUPLICATE_CODE',
      'LONG_FUNCTION',
      'DEEP_NESTING',
    ],
  },
  architect: {
    name: 'Architect',
    sefira: 'Chesed',
    description: 'Design patterns, structure',
    categories: ['architecture', 'design', 'pattern'],
    issueTypes: [
      'ANTIPATTERN',
      'GOD_CLASS',
      'CIRCULAR_DEPENDENCY',
      'MISSING_ABSTRACTION',
    ],
  },
  scholar: {
    name: 'Scholar',
    sefira: 'Daat',
    description: 'Fact verification, knowledge',
    categories: ['verification', 'fact', 'claim'],
    issueTypes: [
      'HALLUCINATION',
      'INCORRECT_FACT',
      'UNVERIFIED_CLAIM',
    ],
  },
  scout: {
    name: 'Scout',
    sefira: 'Netzach',
    description: 'Exploration, search patterns',
    categories: ['search', 'exploration', 'navigation'],
    issueTypes: [
      'INEFFICIENT_SEARCH',
      'MISSING_INDEX',
      'REDUNDANT_QUERY',
    ],
  },
});

/**
 * Dog Benchmark Result
 */
class DogBenchmarkResult {
  constructor(dogId) {
    this.dogId = dogId;
    this.domain = DOG_DOMAINS[dogId];
    this.samples = [];
    this.metrics = null;
    this.timestamp = new Date().toISOString();
  }

  addSample(sampleId, result, groundTruth) {
    const detected = result.issues || [];
    const truth = groundTruth.issues || [];

    // Calculate per-sample metrics
    let tp = 0, fp = 0;
    const matched = new Set();

    for (const d of detected) {
      const dType = normalizeType(d.type);
      let found = false;

      for (let i = 0; i < truth.length; i++) {
        if (!matched.has(i) && typeMatches(dType, normalizeType(truth[i].type))) {
          tp++;
          matched.add(i);
          found = true;
          break;
        }
      }

      if (!found) fp++;
    }

    const fn = truth.length - matched.size;

    this.samples.push({
      sampleId,
      detected: detected.length,
      expected: truth.length,
      truePositives: tp,
      falsePositives: fp,
      falseNegatives: fn,
      verdictMatch: result.verdict === groundTruth.expectedVerdict,
      latencyMs: result.latencyMs,
    });
  }

  finalize() {
    if (this.samples.length === 0) {
      this.metrics = { error: 'No samples' };
      return;
    }

    let totalTP = 0, totalFP = 0, totalFN = 0;
    let verdictMatches = 0;
    let totalLatency = 0;

    for (const s of this.samples) {
      totalTP += s.truePositives;
      totalFP += s.falsePositives;
      totalFN += s.falseNegatives;
      if (s.verdictMatch) verdictMatches++;
      totalLatency += s.latencyMs;
    }

    const precision = totalTP / (totalTP + totalFP) || 0;
    const recall = totalTP / (totalTP + totalFN) || 0;
    const f1 = 2 * (precision * recall) / (precision + recall) || 0;

    this.metrics = {
      samplesCount: this.samples.length,
      precision,
      recall,
      f1,
      verdictAccuracy: verdictMatches / this.samples.length,
      avgLatencyMs: Math.round(totalLatency / this.samples.length),
      truePositives: totalTP,
      falsePositives: totalFP,
      falseNegatives: totalFN,
    };
  }
}

/**
 * Dog Benchmark Runner
 */
export class DogBenchmarkRunner {
  constructor(options = {}) {
    this.options = options;
    this.dogsToTest = options.dogs || Object.keys(DOG_DOMAINS);
    this.dataset = null;
    this.loadedDogs = new Map();
    this.results = new Map();
  }

  async initialize() {
    // Load dataset
    const datasetPath = this.options.datasetPath ||
      join(__dirname, '../collective-vs-single/dataset/samples.json');

    this.dataset = JSON.parse(readFileSync(datasetPath, 'utf8'));

    // Load Dog patterns and rules
    for (const dogId of this.dogsToTest) {
      const dogPath = join(__dirname, '../../packages/node/src/dogs', dogId);
      const patternsPath = join(dogPath, 'patterns.json');
      const rulesPath = join(dogPath, 'rules.js');

      const dogData = {
        patterns: null,
        rules: null,
      };

      if (existsSync(patternsPath)) {
        try {
          dogData.patterns = JSON.parse(readFileSync(patternsPath, 'utf8'));
        } catch (e) {}
      }

      if (existsSync(rulesPath)) {
        try {
          const rules = await import(`file://${rulesPath.replace(/\\/g, '/')}`);
          dogData.rules = rules.default || rules;
        } catch (e) {}
      }

      this.loadedDogs.set(dogId, dogData);
    }
  }

  async runDog(dogId) {
    const domain = DOG_DOMAINS[dogId];
    if (!domain) {
      throw new Error(`Unknown dog: ${dogId}`);
    }

    const result = new DogBenchmarkResult(dogId);
    const dogData = this.loadedDogs.get(dogId);

    // Filter samples relevant to this Dog's domain
    const relevantSamples = this.dataset.samples.filter(s =>
      domain.categories.includes(s.category) ||
      (s.groundTruth.issues || []).some(i =>
        domain.issueTypes.includes(normalizeType(i.type))
      )
    );

    console.log(`\n  ${domain.name} (${domain.sefira}) - ${relevantSamples.length} samples`);
    console.log(`    Domain: ${domain.description}`);
    console.log('');

    for (const sample of relevantSamples) {
      const start = performance.now();

      // Run L1 check
      const issues = [];
      let verdict = 'WAG';
      let confidence = PHI_INV_2;

      // Pattern-based detection
      if (dogData.patterns) {
        const patternIssues = this._checkPatterns(sample.code, dogData.patterns, domain);
        issues.push(...patternIssues);
      }

      // Rule-based detection
      if (dogData.rules?.l1Check) {
        try {
          const ruleResult = await dogData.rules.l1Check({
            content: sample.code,
            type: sample.category,
            language: sample.language,
          });

          if (ruleResult?.issues) issues.push(...ruleResult.issues);
          if (ruleResult?.verdict) verdict = ruleResult.verdict;
          if (ruleResult?.confidence) confidence = ruleResult.confidence;
        } catch (e) {}
      }

      // Derive verdict
      if (issues.length > 0) {
        const hasCritical = issues.some(i => i.severity === 'critical');
        const hasHigh = issues.some(i => i.severity === 'high');

        if (hasCritical) {
          verdict = 'BARK';
          confidence = PHI_INV;
        } else if (hasHigh) {
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

      result.addSample(sample.id, {
        issues,
        verdict,
        confidence,
        latencyMs: Math.round(elapsed),
      }, sample.groundTruth);

      const status = result.samples[result.samples.length - 1].verdictMatch ? '✓' : '✗';
      process.stdout.write(`    [${sample.id}] ${status}\n`);
    }

    result.finalize();
    this.results.set(dogId, result);

    console.log(`\n    Results: P=${(result.metrics.precision * 100).toFixed(1)}% R=${(result.metrics.recall * 100).toFixed(1)}% F1=${(result.metrics.f1 * 100).toFixed(1)}%`);

    return result;
  }

  _checkPatterns(code, patterns, domain) {
    const issues = [];

    // Domain-specific pattern checks
    if (domain.issueTypes.includes('SQL_INJECTION')) {
      if (/\$\{.*\}|'\s*\+\s*\w+|\w+\s*\+\s*'/.test(code) && /query|sql|select/i.test(code)) {
        issues.push({ type: 'SQL_INJECTION', severity: 'critical', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('XSS')) {
      if (/innerHTML|outerHTML|document\.write/.test(code)) {
        issues.push({ type: 'XSS', severity: 'high', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('COMMAND_INJECTION')) {
      if (/exec\s*\(|execSync|spawn/.test(code) && /\$\{|\+/.test(code)) {
        issues.push({ type: 'COMMAND_INJECTION', severity: 'critical', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('EVAL_INJECTION')) {
      if (/\beval\s*\(/.test(code)) {
        issues.push({ type: 'EVAL_INJECTION', severity: 'critical', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('HARDCODED_SECRET')) {
      if (/'[^']{6,}'|"[^"]{6,}"/.test(code) && /secret|password|key|token/i.test(code)) {
        issues.push({ type: 'HARDCODED_SECRET', severity: 'critical', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('WEAK_CRYPTO')) {
      if (/md5|sha1[^0-9]|Math\.random/i.test(code) && /hash|password|crypt|otp/i.test(code)) {
        issues.push({ type: 'WEAK_CRYPTO', severity: 'high', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('PATH_TRAVERSAL')) {
      if (/req\.(query|params|body)/.test(code) && /sendFile|readFile|createReadStream/.test(code)) {
        issues.push({ type: 'PATH_TRAVERSAL', severity: 'critical', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('ASSIGNMENT_IN_CONDITION')) {
      if (/if\s*\([^)]*\w+\s*=\s*['"][^'"]*['"]/.test(code)) {
        issues.push({ type: 'ASSIGNMENT_IN_CONDITION', severity: 'critical', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('RACE_CONDITION')) {
      if (/async.*\{[\s\S]*await[\s\S]*await/.test(code) && /balance|amount|transfer/i.test(code)) {
        issues.push({ type: 'RACE_CONDITION', severity: 'high', source: 'pattern' });
      }
    }

    if (domain.issueTypes.includes('PROTOTYPE_POLLUTION')) {
      if (/for\s*\(\s*(const|let|var)\s+\w+\s+in/.test(code) && !/hasOwnProperty|__proto__|constructor/.test(code)) {
        issues.push({ type: 'PROTOTYPE_POLLUTION', severity: 'high', source: 'pattern' });
      }
    }

    return issues;
  }

  async runAll() {
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('  PER-DOG BENCHMARK');
    console.log('  "Chaque chien chasse dans son territoire"');
    console.log('═══════════════════════════════════════════════════════════════');

    for (const dogId of this.dogsToTest) {
      if (DOG_DOMAINS[dogId]) {
        await this.runDog(dogId);
      }
    }

    return this.getSummary();
  }

  getSummary() {
    console.log('\n═══════════════════════════════════════════════════════════════');
    console.log('  COLLECTIVE SUMMARY');
    console.log('═══════════════════════════════════════════════════════════════\n');

    console.log('  DOG         │  P     │  R     │  F1    │ Verdict │ Samples');
    console.log('  ────────────┼────────┼────────┼────────┼─────────┼─────────');

    const summary = {};

    for (const [dogId, result] of this.results) {
      const m = result.metrics;
      const name = DOG_DOMAINS[dogId].name.padEnd(10);

      console.log(`  ${name}  │ ${(m.precision * 100).toFixed(1).padStart(5)}% │ ${(m.recall * 100).toFixed(1).padStart(5)}% │ ${(m.f1 * 100).toFixed(1).padStart(5)}% │ ${(m.verdictAccuracy * 100).toFixed(0).padStart(6)}% │ ${m.samplesCount.toString().padStart(7)}`);

      summary[dogId] = {
        precision: m.precision,
        recall: m.recall,
        f1: m.f1,
        verdictAccuracy: m.verdictAccuracy,
        samples: m.samplesCount,
      };
    }

    // Overall weighted average
    let totalF1 = 0, totalSamples = 0;
    for (const [_, result] of this.results) {
      totalF1 += result.metrics.f1 * result.metrics.samplesCount;
      totalSamples += result.metrics.samplesCount;
    }

    console.log('  ────────────┼────────┼────────┼────────┼─────────┼─────────');
    console.log(`  ${'COLLECTIVE'.padEnd(10)}  │        │        │ ${((totalF1 / totalSamples) * 100).toFixed(1).padStart(5)}% │         │ ${totalSamples.toString().padStart(7)}`);

    console.log('');

    return summary;
  }

  saveResults() {
    const outputDir = join(__dirname, 'results');
    if (!existsSync(outputDir)) {
      mkdirSync(outputDir, { recursive: true });
    }

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const outputPath = join(outputDir, `dog-benchmark-${timestamp}.json`);

    const data = {
      timestamp: new Date().toISOString(),
      dogs: {},
    };

    for (const [dogId, result] of this.results) {
      data.dogs[dogId] = {
        domain: result.domain,
        metrics: result.metrics,
        samples: result.samples,
      };
    }

    writeFileSync(outputPath, JSON.stringify(data, null, 2));
    console.log(`  Results saved to: ${outputPath}\n`);

    return outputPath;
  }
}

// Utility functions
function normalizeType(type) {
  return (type || '').toUpperCase().replace(/[-\s]/g, '_');
}

function typeMatches(a, b) {
  if (a === b) return true;

  const families = [
    ['SQL_INJECTION', 'SQLI'],
    ['XSS', 'CROSS_SITE_SCRIPTING'],
    ['WEAK_CRYPTO', 'WEAK_HASH', 'MD5'],
  ];

  for (const family of families) {
    if (family.includes(a) && family.includes(b)) return true;
  }

  return false;
}

// Export for module use
export { DOG_DOMAINS };

// CLI entry point
if (process.argv[1] && process.argv[1].endsWith('dog-benchmark.js')) {
  const runner = new DogBenchmarkRunner();

  (async () => {
    try {
      await runner.initialize();
      await runner.runAll();
      runner.saveResults();
    } catch (e) {
      console.error('Benchmark error:', e.message);
      process.exit(1);
    }
  })();
}
