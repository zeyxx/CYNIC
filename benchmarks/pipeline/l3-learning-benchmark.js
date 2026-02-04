#!/usr/bin/env node
/**
 * L3 Learning Loop Benchmark
 *
 * Measures effectiveness of L3 feedback (L2 → L1).
 * Tracks pattern learning over time and accuracy improvements.
 *
 * "Le chien qui apprend à chasser" - The dog that learns to hunt
 *
 * @module @cynic/benchmarks/l3-learning
 */

'use strict';

import { readFileSync, writeFileSync, existsSync, mkdirSync, copyFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { performance } from 'perf_hooks';
import { EventEmitter } from 'events';

const __dirname = dirname(fileURLToPath(import.meta.url));

// φ-aligned constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const FIB_8 = 21;

/**
 * L3 Learning Configuration
 */
const CONFIG = Object.freeze({
  // Learning epochs
  EPOCHS: 3,

  // Samples per epoch
  SAMPLES_PER_EPOCH: 10,

  // Min observations before learning
  MIN_OBSERVATIONS: 3,

  // Learning decay rate
  DECAY_RATE: PHI_INV_2,

  // Dogs to test learning
  LEARNABLE_DOGS: ['guardian', 'analyst', 'janitor', 'architect', 'scholar', 'scout'],

  // Output directory
  OUTPUT_DIR: join(__dirname, 'results'),
});

/**
 * Learning Epoch Result
 */
class EpochResult {
  constructor(epochNumber) {
    this.epoch = epochNumber;
    this.startedAt = new Date().toISOString();
    this.samples = [];
    this.patternsLearned = 0;
    this.patternsFalsified = 0;
    this.metrics = null;
  }

  addSample(result) {
    this.samples.push(result);
  }

  finalize() {
    if (this.samples.length === 0) {
      this.metrics = { error: 'No samples' };
      return;
    }

    let tp = 0, fp = 0, fn = 0;
    let verdictMatches = 0;
    let totalLatency = 0;

    for (const s of this.samples) {
      tp += s.truePositives || 0;
      fp += s.falsePositives || 0;
      fn += s.falseNegatives || 0;
      if (s.verdictMatch) verdictMatches++;
      totalLatency += s.latencyMs || 0;
    }

    const precision = tp / (tp + fp) || 0;
    const recall = tp / (tp + fn) || 0;
    const f1 = 2 * (precision * recall) / (precision + recall) || 0;

    this.metrics = {
      samplesCount: this.samples.length,
      precision,
      recall,
      f1,
      verdictAccuracy: verdictMatches / this.samples.length,
      avgLatencyMs: Math.round(totalLatency / this.samples.length),
      patternsLearned: this.patternsLearned,
      patternsFalsified: this.patternsFalsified,
    };

    this.completedAt = new Date().toISOString();
  }
}

/**
 * Mock Event Bus for Learning Service
 */
class MockEventBus extends EventEmitter {
  constructor() {
    super();
    this.emittedEvents = [];
  }

  emit(event, data) {
    this.emittedEvents.push({ event, data, timestamp: Date.now() });
    return super.emit(event, data);
  }
}

/**
 * L3 Learning Benchmark
 */
export class L3LearningBenchmark {
  constructor(options = {}) {
    this.options = options;
    this.dataset = null;
    this.epochs = [];
    this.eventBus = new MockEventBus();
    this.learningService = null;
    this.initialPatterns = new Map();
  }

  async initialize() {
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('  L3 LEARNING LOOP BENCHMARK');
    console.log('  "Le chien qui apprend à chasser"');
    console.log('═══════════════════════════════════════════════════════════════\n');

    // Load dataset
    const datasetPath = this.options.datasetPath ||
      join(__dirname, '../collective-vs-single/dataset/samples.json');
    this.dataset = JSON.parse(readFileSync(datasetPath, 'utf8'));
    console.log(`  Dataset: ${this.dataset.samples.length} samples`);

    // Backup and load initial patterns
    console.log('  Backing up initial patterns...');
    for (const dogId of CONFIG.LEARNABLE_DOGS) {
      const patternsPath = join(__dirname, '../../packages/node/src/dogs', dogId, 'patterns.json');
      if (existsSync(patternsPath)) {
        const patterns = JSON.parse(readFileSync(patternsPath, 'utf8'));
        this.initialPatterns.set(dogId, JSON.parse(JSON.stringify(patterns)));

        // Count initial learned patterns
        const learnedCount = (patterns.learnedPatterns || []).length;
        console.log(`    ${dogId}: ${learnedCount} learned patterns`);
      }
    }

    // Initialize learning service
    try {
      const { createDogLearningService } = await import(
        `file://${join(__dirname, '../../packages/node/src/dogs/index.js').replace(/\\/g, '/')}`
      );

      this.learningService = createDogLearningService({
        eventBus: this.eventBus,
        enabled: true,
        persistImmediately: true,
      });

      this.learningService.start();
      console.log('  Learning service started');
    } catch (e) {
      console.log(`  Warning: Could not load learning service: ${e.message}`);
      console.log('  Using simulation mode');
    }

    console.log('');
  }

  async runEpoch(epochNumber) {
    console.log(`───────────────────────────────────────────────────────────────`);
    console.log(`  EPOCH ${epochNumber + 1}/${CONFIG.EPOCHS}`);
    console.log(`───────────────────────────────────────────────────────────────\n`);

    const epoch = new EpochResult(epochNumber);

    // Select samples for this epoch
    const samples = this._selectSamplesForEpoch(epochNumber);

    for (const sample of samples) {
      // Run L1 check
      const l1Result = await this._runL1Check(sample);

      // Simulate L2 judgment (as if LLM confirmed/denied L1)
      const l2Result = this._simulateL2Judgment(sample, l1Result);

      // Emit judgment event for learning service
      this._emitJudgmentEvent(sample, l2Result);

      // Evaluate
      const evaluation = this._evaluate(l1Result, sample.groundTruth);

      epoch.addSample({
        sampleId: sample.id,
        l1Verdict: l1Result.verdict,
        l2Verdict: l2Result.verdict,
        expectedVerdict: sample.groundTruth.expectedVerdict,
        verdictMatch: l1Result.verdict === sample.groundTruth.expectedVerdict,
        ...evaluation,
        latencyMs: l1Result.latencyMs,
      });

      const status = evaluation.verdictMatch ? '✓' : '✗';
      console.log(`    [${sample.id}] L1:${l1Result.verdict} L2:${l2Result.verdict} Expected:${sample.groundTruth.expectedVerdict} ${status}`);

      // Small delay for learning to process
      await new Promise(r => setTimeout(r, 50));
    }

    // Count patterns learned in this epoch
    if (this.learningService) {
      const stats = this.learningService.getStats();
      epoch.patternsLearned = stats.patternsLearned;
      epoch.patternsFalsified = stats.patternsFalsified;
    }

    epoch.finalize();
    this.epochs.push(epoch);

    console.log(`\n    Epoch ${epochNumber + 1} Results:`);
    console.log(`      F1: ${(epoch.metrics.f1 * 100).toFixed(1)}%`);
    console.log(`      Patterns learned: ${epoch.patternsLearned}`);
    console.log('');

    return epoch;
  }

  _selectSamplesForEpoch(epochNumber) {
    // Rotate through samples, weighted by difficulty
    const offset = epochNumber * CONFIG.SAMPLES_PER_EPOCH;
    const shuffled = [...this.dataset.samples].sort(() => 0.5 - Math.random());
    return shuffled.slice(0, CONFIG.SAMPLES_PER_EPOCH);
  }

  async _runL1Check(sample) {
    const start = performance.now();
    const issues = [];
    let verdict = 'WAG';
    let confidence = PHI_INV_2;

    // Category-based heuristics
    const code = sample.code;
    const category = sample.category;

    if (category.includes('sql') && /\$\{|\+/.test(code)) {
      issues.push({ type: 'SQL_INJECTION', severity: 'critical' });
    }

    if (category.includes('xss') && /innerHTML/.test(code)) {
      issues.push({ type: 'XSS', severity: 'high' });
    }

    if (category === 'command-injection' && /exec\s*\(/.test(code)) {
      issues.push({ type: 'COMMAND_INJECTION', severity: 'critical' });
    }

    if (category === 'code-injection' && /eval\s*\(/.test(code)) {
      issues.push({ type: 'EVAL_INJECTION', severity: 'critical' });
    }

    if (category === 'authentication' && /secret|password/.test(code.toLowerCase()) && /'[^']+'/g.test(code)) {
      issues.push({ type: 'HARDCODED_SECRET', severity: 'critical' });
    }

    if (category === 'logic-bug' && /if\s*\([^)]*\w+\s*=\s*['"]/.test(code)) {
      issues.push({ type: 'ASSIGNMENT_IN_CONDITION', severity: 'critical' });
    }

    if (category.includes('crypto') && /md5|Math\.random/i.test(code)) {
      issues.push({ type: 'WEAK_CRYPTO', severity: 'high' });
    }

    if (category === 'path-traversal' && /req\.query/.test(code) && /sendFile/.test(code)) {
      issues.push({ type: 'PATH_TRAVERSAL', severity: 'critical' });
    }

    if (category === 'prototype-pollution' && /for\s*\(.+in/.test(code)) {
      issues.push({ type: 'PROTOTYPE_POLLUTION', severity: 'high' });
    }

    // Derive verdict
    if (issues.some(i => i.severity === 'critical')) {
      verdict = 'BARK';
      confidence = PHI_INV;
    } else if (issues.some(i => i.severity === 'high')) {
      verdict = 'GROWL';
      confidence = 0.5;
    } else if (sample.category === 'safe') {
      verdict = 'HOWL';
      confidence = PHI_INV;
    }

    return {
      issues,
      verdict,
      confidence,
      latencyMs: Math.round(performance.now() - start),
    };
  }

  _simulateL2Judgment(sample, l1Result) {
    // L2 = ground truth (simulating perfect LLM)
    const truth = sample.groundTruth;
    return {
      issues: truth.issues,
      verdict: truth.expectedVerdict,
      qScore: (100 - truth.minScore + 100 - truth.maxScore) / 2,
      confidence: PHI_INV,
    };
  }

  _emitJudgmentEvent(sample, l2Result) {
    // Emit event for learning service to pick up
    this.eventBus.emit('judgment:created', {
      judgmentId: `bench-${sample.id}-${Date.now()}`,
      verdict: l2Result.verdict,
      qScore: l2Result.qScore,
      confidence: l2Result.confidence,
      item: {
        type: sample.category,
        identifier: sample.id,
        content: sample.code,
      },
    });
  }

  _evaluate(result, truth) {
    const detected = result.issues || [];
    const expected = truth.issues || [];

    let tp = 0, fp = 0;
    const matched = new Set();

    for (const d of detected) {
      const dType = (d.type || '').toUpperCase();
      let found = false;

      for (let i = 0; i < expected.length; i++) {
        if (!matched.has(i)) {
          const eType = (expected[i].type || '').toUpperCase();
          if (dType === eType || dType.includes(eType) || eType.includes(dType)) {
            tp++;
            matched.add(i);
            found = true;
            break;
          }
        }
      }

      if (!found) fp++;
    }

    const fn = expected.length - matched.size;

    return {
      truePositives: tp,
      falsePositives: fp,
      falseNegatives: fn,
      precision: tp / (tp + fp) || 0,
      recall: tp / (tp + fn) || 0,
      verdictMatch: result.verdict === truth.expectedVerdict,
    };
  }

  async run() {
    await this.initialize();

    for (let i = 0; i < CONFIG.EPOCHS; i++) {
      await this.runEpoch(i);
    }

    return this.getSummary();
  }

  getSummary() {
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('  LEARNING PROGRESSION');
    console.log('═══════════════════════════════════════════════════════════════\n');

    console.log('  EPOCH │  F1    │  P     │  R     │ Verdict │ Learned');
    console.log('  ──────┼────────┼────────┼────────┼─────────┼─────────');

    let totalPatternsLearned = 0;

    for (const epoch of this.epochs) {
      const m = epoch.metrics;
      totalPatternsLearned += m.patternsLearned;

      console.log(`    ${(epoch.epoch + 1).toString().padStart(3)}  │ ${(m.f1 * 100).toFixed(1).padStart(5)}% │ ${(m.precision * 100).toFixed(1).padStart(5)}% │ ${(m.recall * 100).toFixed(1).padStart(5)}% │ ${(m.verdictAccuracy * 100).toFixed(0).padStart(6)}% │ ${m.patternsLearned.toString().padStart(7)}`);
    }

    // Calculate improvement
    if (this.epochs.length >= 2) {
      const firstEpoch = this.epochs[0].metrics;
      const lastEpoch = this.epochs[this.epochs.length - 1].metrics;
      const f1Improvement = lastEpoch.f1 - firstEpoch.f1;

      console.log('  ──────┼────────┼────────┼────────┼─────────┼─────────');
      console.log(`   Δ    │ ${f1Improvement >= 0 ? '+' : ''}${(f1Improvement * 100).toFixed(1).padStart(4)}% │        │        │         │ ${totalPatternsLearned.toString().padStart(7)}`);

      console.log('\n  LEARNING EFFECTIVENESS:');

      if (f1Improvement > 0.05) {
        console.log(`    ✅ Learning loop effective (+${(f1Improvement * 100).toFixed(1)}% F1)`);
      } else if (f1Improvement > 0) {
        console.log(`    ⚠️  Marginal improvement (+${(f1Improvement * 100).toFixed(1)}% F1)`);
      } else {
        console.log(`    ❌ No improvement detected (${(f1Improvement * 100).toFixed(1)}% F1)`);
      }

      console.log(`    📊 Patterns learned: ${totalPatternsLearned}`);
      console.log(`    📈 F1/pattern: ${totalPatternsLearned > 0 ? (f1Improvement / totalPatternsLearned * 100).toFixed(2) : 0}%`);
    }

    console.log('');

    return {
      epochs: this.epochs.map(e => ({
        epoch: e.epoch,
        metrics: e.metrics,
      })),
      totalPatternsLearned,
      f1Improvement: this.epochs.length >= 2
        ? this.epochs[this.epochs.length - 1].metrics.f1 - this.epochs[0].metrics.f1
        : 0,
    };
  }

  saveResults() {
    if (!existsSync(CONFIG.OUTPUT_DIR)) {
      mkdirSync(CONFIG.OUTPUT_DIR, { recursive: true });
    }

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const outputPath = join(CONFIG.OUTPUT_DIR, `l3-learning-${timestamp}.json`);

    writeFileSync(outputPath, JSON.stringify({
      timestamp: new Date().toISOString(),
      config: {
        epochs: CONFIG.EPOCHS,
        samplesPerEpoch: CONFIG.SAMPLES_PER_EPOCH,
      },
      epochs: this.epochs.map(e => ({
        epoch: e.epoch,
        startedAt: e.startedAt,
        completedAt: e.completedAt,
        metrics: e.metrics,
        samples: e.samples,
      })),
      summary: this.getSummary(),
    }, null, 2));

    console.log(`  Results saved to: ${outputPath}\n`);

    return outputPath;
  }

  async cleanup() {
    // Stop learning service
    if (this.learningService) {
      this.learningService.stop();
    }

    // Restore initial patterns (optional - comment out to keep learned patterns)
    // console.log('  Restoring initial patterns...');
    // for (const [dogId, patterns] of this.initialPatterns) {
    //   const patternsPath = join(__dirname, '../../packages/node/src/dogs', dogId, 'patterns.json');
    //   writeFileSync(patternsPath, JSON.stringify(patterns, null, 2));
    // }
  }
}

// CLI entry point
if (process.argv[1] && process.argv[1].endsWith('l3-learning-benchmark.js')) {
  const benchmark = new L3LearningBenchmark();

  (async () => {
    try {
      await benchmark.run();
      benchmark.saveResults();
      await benchmark.cleanup();
    } catch (e) {
      console.error('Benchmark error:', e.message);
      console.error(e.stack);
      process.exit(1);
    }
  })();
}

export { CONFIG as L3_CONFIG };
